//! Max^n search (spec §6). Each node maximizes the **moving player's own component** of the
//! per-player value vector `V = ⟨U1, U2, U3, U4⟩` — the vector is backed up whole, never
//! collapsed to a scalar (Hard Rule #3). Leaves are scored by P5's [`eval_4vec`].
//!
//! Beam Max^n — at internal nodes expand the top-`beam_width` ordered moves (§6.1), at the
//! **root** consider all legal moves (so a good move ordered past the beam is never dropped).
//! The transposition table is used for **move ordering only** (best-move hint); the beam makes
//! node values approximate, so there is no value cutoff until Max^n shallow pruning adds real
//! bounds.
//!
//! Implemented refinements (P6): **terminal scoring** (§1.8 — a no-legal-moves node eliminates the
//! mover with a mate-distance score, kept in centipawns; §1.7: the search value never uses FFA
//! points), **iterative deepening** (4, 8, 12, … carrying the TT best-move forward), and
//! **killers + a history heuristic** in move ordering. Shallow pruning is still pending (its bounds
//! decision is open).
//!
//! Depth should be a multiple of 4 (Hard Rule #1) so the perspective chain ends on a full
//! 4-player rotation; the recursion itself accepts any depth.

use crate::board::{Board, Move};
use crate::eval::eval_4vec;
use crate::lines::LineMap;
use crate::move_gen::{generate_legal, in_check};
use crate::move_order;
use crate::queries::{
    elimination_proximity, king_danger_scalar, king_danger_table_scalar, query_king_safety,
    query_material, query_target_pressure,
};
use crate::tt::{Bound, TranspositionTable};

/// Default beam width (spec appendix `DEFAULT_BEAM_WIDTH`).
const DEFAULT_BEAM_WIDTH: usize = 30;

/// Terminal/mate score magnitude (centipawns, well inside `i16`). A mated player's own component
/// is set to `-(MATE - ply)`, so a *sooner* elimination is more extreme — the mated side delays it
/// as long as it can and avoids walking into it. (§1.7: the search value never uses FFA points; the
/// §1.8 point awards are game-scoring on `board.points`, applied at play time.)
const MATE: i16 = 30_000;

/// Forward-pruning (late move reduction) knobs — default-off lever (see `with_forward_pruning`).
/// The first `LMR_LATE_MOVES` moves and all captures/promotions are searched at full depth; later
/// quiet moves are searched reduced and re-searched only if they beat the current best.
const LMR_LATE_MOVES: usize = 3;
const LMR_MIN_DEPTH: u32 = 3;

/// Adaptive-beam schedule (see `with_adaptive_beam` and `beam_at`): a **hard** per-node branch cap
/// that tapers by rotation. Width is concentrated in the first rotation (the responses to the move we
/// actually play) and tightens to a floor of 2 deeper — we only ever play the root move, so deep
/// sub-branches need just enough width to *value* it. Scales off the configured beam width and steps
/// only at rotation boundaries (multiples of 4), so the multiple-of-4 rotation rule is untouched.
/// This is what bounds the tree — limiting *branches per node*, never search time or depth.
const BEAM_DEEP_FLOOR: usize = 2; // third rotation onward (ply 8+): minimal per-node branch cap

/// Quiescence cap (default-off lever, see `with_quiescence`). At a leaf the search continues along
/// tactical moves until quiet, but only ever returns at a rotation boundary (Hard Rule #1) and never
/// extends past this many plies (one full rotation). Keep it a multiple of 4.
const QUIESCENCE_MAX_PLY: u32 = 4;

pub struct Searcher {
    tt: TranspositionTable,
    /// Reusable line buffer handed to the evaluator (always-recompute; one boxed buffer).
    lines: Box<LineMap>,
    /// Leaf evaluator. Defaults to [`eval_4vec`]; injectable so the Max^n backup can be tested
    /// with a controllable synthetic eval.
    eval: fn(&Board, &mut LineMap) -> [i16; 4],
    /// Per-node beam width: expand only the top-N ordered moves (§6.1).
    beam_width: usize,
    /// Nodes visited in the last `search` call (summed across iterative-deepening iterations).
    pub nodes: u64,
    /// Move-ordering state (killers + history), carried across iterative-deepening iterations.
    order_state: move_order::OrderState,
    /// Forward pruning (late move reductions). Default-off strength/speed lever with an ablation arm.
    forward_pruning: bool,
    /// Adaptive beam scheduling (narrow the beam with depth). Default-off lever with an ablation arm.
    adaptive_beam: bool,
    /// Quiescence search at leaves (tactical-only, rotation-complete). Default-off lever with an
    /// ablation arm.
    quiescence: bool,
    /// Hard node budget per `search` call (0 = unlimited). When exceeded the search cuts (returns the
    /// static eval at the current node) and stops expanding new root moves / deeper iterations,
    /// returning the best move found so far. Bounds cost on pathological capture-dense positions.
    node_budget: u64,
    /// Deep beam floor — the per-node branch cap at the third rotation onward (`beam_at`, adaptive
    /// only). **1 = "laser"** (Shannon Type B: follow the single best line deep, no deep branching →
    /// depth becomes ~free); **2 = a minimal Max^n vector** (the default). Experiment knob.
    deep_floor: usize,
    /// Noise-adaptive beam (experiment): per node, **narrow** when there's a real tactic (laser down
    /// the forcing line) and **broad** when quiet (flashlight to compare near-equal moves). When on it
    /// overrides the rotation taper: `beam_cap = noisy ? deep_floor : beam_width`. Default off.
    noise_adaptive: bool,
    /// **Win term (search-side objective).** Weight on each player's mean-relative FFA `points` added to
    /// the *search* value — not the static eval, which stays points-blind (Hard Rule #8). FFA points are
    /// the **goal** (who to target: captures + the +20 for eliminating a player); cp is the **means**
    /// (the tactics to get there). `0` = off (default). Tune by self-play A/B.
    win_weight: i16,
    /// Win-term SIGNAL: `false` = banked FFA points (fires throughout — a *scoring* driver); `true` =
    /// Elimination-proximity (fires late on a collapsing opponent — a *finishing* gradient).
    /// A/B knob for the win-signal comparison. Default banked-points.
    win_proxy: bool,
    /// **King-danger weight (the points-aware safety rebuild).** Subtracts `weight × king_danger / 100`
    /// from each player's *search* value — king-safety valued as the points-risk of elimination, in the
    /// same objective layer as `win_weight`. Uses [`king_danger_scalar`] (pure incoming attack, no
    /// huddle bonus). `0` = off (default). The eval's old cp huddle-safety stays off.
    danger_weight: i16,
    /// **Material-weakness targeting weight (EXP-033).** Adds `weight × target_pressure / 100` to
    /// each player's search value, where target_pressure is the SEE-winning threat value against the
    /// player's materially-weakest opponent. `0` = off (default). Tune by A/B.
    target_weight: i16,
    /// Target-pressure SHAPE: `false` = flat threat sum; `true` = turn-proximity-weighted (threats
    /// against the next-to-move weak opponent are more urgent). Default flat.
    target_proximity: bool,
    /// King-danger SHAPE: `false` = linear scalar (defender-mitigated); `true` = a non-linear
    /// attack-units table (compounding). A/B knob for the safety-shape comparison. Default linear.
    danger_table: bool,
}

/// One node of the flashlight search tree (index 0 = root). `best_child` records the child that
/// gave this node its backed-up value (0 = none/leaf) — used to reconstruct the principal variation.
struct FNode {
    parent: usize,
    mover: usize,
    mv: Option<Move>,
    value: [i16; 4],
    leaf: bool,
    best_child: usize,
}

/// Post-search telemetry from the flashlight (the play path) — for the protocol's `info` output and
/// any UI. Read-only: producing it does not change the move `search_flashlight` would pick.
pub struct SearchInfo {
    /// The move the engine plays (root child maximizing the root mover's component).
    pub best: Option<Move>,
    /// Backed-up `[R,B,Y,G]` value of the best line.
    pub value: [i16; 4],
    /// Rounded search depth actually reached (a multiple of 4 — full rotations).
    pub depth: u32,
    /// Nodes visited.
    pub nodes: u64,
    /// Every root move with its backed-up value, ranked best-first by the root mover's component.
    pub candidates: Vec<(Move, [i16; 4])>,
    /// Principal variation (the engine's expected line), best move first.
    pub pv: Vec<Move>,
}

impl Searcher {
    pub fn new(tt_mb: usize) -> Self {
        Searcher {
            tt: TranspositionTable::new(tt_mb),
            lines: Box::new(LineMap::new()),
            eval: eval_4vec,
            beam_width: DEFAULT_BEAM_WIDTH,
            nodes: 0,
            order_state: move_order::OrderState::new(),
            forward_pruning: false,
            adaptive_beam: false,
            quiescence: false,
            node_budget: 0,
            deep_floor: BEAM_DEEP_FLOOR,
            noise_adaptive: false,
            win_weight: 0,
            win_proxy: false,
            danger_weight: 0,
            danger_table: false,
            target_weight: 0,
            target_proximity: false,
        }
    }

    /// Override the per-node beam width.
    pub fn with_beam_width(mut self, width: usize) -> Self {
        self.beam_width = width.max(1);
        self
    }

    /// Override the deep beam floor (the adaptive per-node cap at rotation 3+). **1 = laser** (drive
    /// one line deep, no deep branching); **2 = minimal vector**. Experiment knob for the
    /// depth-vs-breadth trade-off (Shannon Type B vs a thin Type A).
    pub fn with_deep_floor(mut self, floor: usize) -> Self {
        self.deep_floor = floor.max(1);
        self
    }

    /// Enable the noise-adaptive beam (experiment): narrow on tactics, broad when quiet. Pairs with
    /// `with_deep_floor` (narrow width) and `with_beam_width` (broad width).
    pub fn with_noise_adaptive(mut self, on: bool) -> Self {
        self.noise_adaptive = on;
        self
    }

    /// Enable the **win term** (search-side): add `weight × mean-relative FFA points` to the search
    /// value, so the engine plays for the actual objective (points / eliminations) with cp as the
    /// tactical layer underneath. The static eval stays points-blind. `0` = off. Tune by self-play A/B.
    ///
    /// **Scope: [`Self::search_flashlight`] only.** The objective layer lives in `eval_with_win`,
    /// which only the flashlight calls at its leaves; the default [`Self::search`] / `maxn` /
    /// `qsearch` path evaluates leaves with the plain static eval and is unaffected by this knob.
    pub fn with_win_term(mut self, weight: i16) -> Self {
        self.win_weight = weight;
        self
    }

    /// Use the elimination-proximity win signal (finishing gradient) instead of banked FFA points
    /// (scoring driver). A/B knob for the win-signal comparison.
    pub fn with_win_proxy(mut self, on: bool) -> Self {
        self.win_proxy = on;
        self
    }

    /// Enable the **points-aware king-danger** term (the safety rebuild): subtract
    /// `weight × king_danger / 100` from each player's search value, valuing king-safety as the
    /// points-risk of elimination in the same objective layer as the win term. `0` = off. Tune by A/B.
    ///
    /// **Scope: [`Self::search_flashlight`] only** — same as [`Self::with_win_term`]; the default
    /// [`Self::search`] path is unaffected.
    pub fn with_king_danger(mut self, weight: i16) -> Self {
        self.danger_weight = weight;
        self
    }

    /// Use the non-linear attack-units danger **table** instead of the linear scalar.
    pub fn with_danger_table(mut self, on: bool) -> Self {
        self.danger_table = on;
        self
    }

    /// Enable the **material-weakness targeting** term (EXP-033): add `weight × target_pressure / 100`
    /// to each player's search value, where target_pressure is the SEE-winning threat value against the
    /// player's materially-weakest opponent. `0` = off (default). Tune by A/B.
    ///
    /// **Scope: [`Self::search_flashlight`] only** — same as [`Self::with_win_term`].
    pub fn with_target_weight(mut self, weight: i16) -> Self {
        self.target_weight = weight;
        self
    }

    /// Use turn-proximity weighting for the target-pressure signal: threats against the next-to-move
    /// weak opponent are more urgent than threats against a player 2-3 turns away. Default flat.
    pub fn with_target_proximity(mut self, on: bool) -> Self {
        self.target_proximity = on;
        self
    }

    /// Leaf value = static eval (cp, the *means*) + the **search-side objective layer**: the win term
    /// (`win_weight ×` mean-relative FFA points, the *goal*), the king-danger term (`danger_weight ×`
    /// the points-risk of elimination), and the material-weakness targeting term (`target_weight ×`
    /// pressure on the weakest opponent). With all weights `0` this is exactly the static eval (the
    /// validated `flashlight == Max^n` default path).
    fn eval_with_win(&mut self, board: &Board) -> [i16; 4] {
        let mut v = (self.eval)(board, &mut self.lines);
        if self.win_weight != 0 {
            if self.win_proxy {
                // Elimination-proximity: win_i = Σ_{j≠i} prox_j − 3·prox_i = total − 4·prox_i.
                let material = query_material(board);
                let ks = query_king_safety(&self.lines, board);
                let prox = elimination_proximity(&material, &ks);
                let total: i32 = prox.iter().map(|&p| p as i32).sum();
                for i in 0..4 {
                    let adj = self.win_weight as i32 * (total - 4 * prox[i] as i32) / 100;
                    v[i] = (v[i] as i32 + adj).clamp(-30_000, 30_000) as i16;
                }
            } else {
                // Banked FFA points, mean-relative (scoring driver).
                let p = board.points;
                let mean = (p[0] as i32 + p[1] as i32 + p[2] as i32 + p[3] as i32) / 4;
                for i in 0..4 {
                    let adj = self.win_weight as i32 * (p[i] as i32 - mean);
                    v[i] = (v[i] as i32 + adj).clamp(-30_000, 30_000) as i16;
                }
            }
        }
        if self.danger_weight != 0 {
            // King-danger (the points-aware safety rebuild): subtract the incoming attack on each king,
            // valued as the points-risk of elimination. `self.lines` is already populated for `board`
            // by the eval call above (the same array-line projection the eval reads).
            let ks = query_king_safety(&self.lines, board);
            for i in 0..4 {
                let raw = if self.danger_table {
                    king_danger_table_scalar(&ks[i])
                } else {
                    king_danger_scalar(&ks[i])
                };
                let d = self.danger_weight as i32 * raw as i32 / 100;
                v[i] = (v[i] as i32 - d).clamp(-30_000, 30_000) as i16;
            }
        }
        if self.target_weight != 0 {
            // EXP-033: material-weakness targeting — reward pressure on the weakest opponent.
            let pressure = query_target_pressure(&self.lines, board, self.target_proximity);
            let mean = (pressure[0] as i32 + pressure[1] as i32 + pressure[2] as i32 + pressure[3] as i32) / 4;
            for i in 0..4 {
                let adj = self.target_weight as i32 * (pressure[i] as i32 - mean) / 100;
                v[i] = (v[i] as i32 + adj).clamp(-30_000, 30_000) as i16;
            }
        }
        v
    }

    /// Enable forward pruning (late move reductions). Default off — a strength/speed lever shipped
    /// with an ablation arm (new levers ship default-off; toggle to measure on vs off).
    pub fn with_forward_pruning(mut self, on: bool) -> Self {
        self.forward_pruning = on;
        self
    }

    /// Enable the **FFA-bounty move-ordering term** (captures scored up by `victim ffa_points ×
    /// 500`). Default off — ordering is selection under a beam, so this is a strength-affecting
    /// lever (Hard Rule #6), measured in EXP-020.
    ///
    /// **Scope: the maxn path only** ([`Self::search`] and its drivers) — `search_flashlight`
    /// never calls `move_order`.
    pub fn with_ffa_bounty_order(mut self, on: bool) -> Self {
        self.order_state.ffa_bounty = on;
        self
    }

    /// Enable the **free-capture move-ordering bonus** (a capture whose victim is undefended gets
    /// a large bonus). Default off — same gate and scope as [`Self::with_ffa_bounty_order`]
    /// (Hard Rule #6, EXP-020; maxn path only).
    pub fn with_free_capture_order(mut self, on: bool) -> Self {
        self.order_state.free_capture = on;
        self
    }

    /// Enable adaptive beam scheduling: a **hard** per-node branch cap that keeps more lines near the
    /// root and tapers to a floor of 2 deeper (the root itself stays full-width; interior nodes follow
    /// `beam_at`). MVV-LVA ordering keeps the best captures inside the cap — there is no capture
    /// exemption. Only affects how many lines per node, never the search depth — the multiple-of-4
    /// rotation rule is untouched. Default off — strength/speed lever with an ablation arm.
    pub fn with_adaptive_beam(mut self, on: bool) -> Self {
        self.adaptive_beam = on;
        self
    }

    /// Enable quiescence search at leaves: continue along tactical moves (captures/promotions) until
    /// the position is quiet, returning only at a rotation boundary (Hard Rule #1) so the value
    /// vector stays fair. Reduces — does not eliminate — the horizon effect on capture exchanges
    /// (quiet/positional horizon effects are untouched). Default off — strength lever with an
    /// ablation arm.
    pub fn with_quiescence(mut self, on: bool) -> Self {
        self.quiescence = on;
        self
    }

    /// Set a hard node budget per `search` call (0 = unlimited). The search returns its best move so
    /// far once the budget is hit — a realistic time/cost bound that keeps pathological capture-dense
    /// positions from running unbounded.
    pub fn with_node_budget(mut self, nodes: u64) -> Self {
        self.node_budget = nodes;
        self
    }

    /// Inject a leaf evaluator. **Experiment/test machinery only** — used by unit tests (a
    /// controllable synthetic eval for the Max^n backup) and by eval-arm A/B harnesses
    /// (EXP-029: candidate evals like [`crate::eval::eval_4vec_pprime`] fighting the deployed
    /// [`crate::eval::eval_4vec`] inside one process on the paired gate). The deployed default
    /// never changes here; a candidate becoming the default is a Tier-2 ship.
    pub fn with_eval(mut self, eval: fn(&Board, &mut LineMap) -> [i16; 4]) -> Self {
        self.eval = eval;
        self
    }

    /// Iterative deepening: search depths 4, 8, …, up to the requested (rounded) depth, reusing the
    /// transposition table's best-move hint (and the killer/history state) for ordering between
    /// iterations. Returns the deepest result, or `None` if the side to move has no legal moves.
    pub fn search(&mut self, board: &mut Board, depth: u32) -> Option<(Move, [i16; 4])> {
        let target = round_to_rotation(depth); // Hard Rule #1: full 4-player rotations.
        self.nodes = 0;
        let mut best = None;
        let mut d = 4;
        while d <= target {
            best = self.search_depth(board, d);
            if self.over_budget() {
                break; // budget spent → don't start a deeper iteration; keep this best
            }
            d += 4;
        }
        best
    }

    /// **Flashlight (Type A) search** — a level/frontier beam. Expand the tree level by level, keep
    /// only the top `cap_at(level)` nodes per level (ranked by the moving player's own eval gain),
    /// then back up Max^n values over the *kept* tree. `cap_at` lets the cap vary by level (a fixed
    /// width, or grow per rotation). Width is bounded per level, so cost is ~linear in depth instead
    /// of exponential. Heuristic — a pruned line is lost; with a huge cap (no pruning) it reduces to
    /// exact Max^n (see `flashlight_matches_maxn_without_pruning`). **This is the play path**: the
    /// protocol's `go` drives it with a generous cap (SYNTHESIS recommendation, B3 2026-06-10);
    /// [`Self::search`] (beam Max^n) remains the exact-path reference and harness default.
    pub fn search_flashlight(
        &mut self,
        board: &Board,
        depth: u32,
        cap_at: impl Fn(u32) -> usize,
    ) -> Option<(Move, [i16; 4])> {
        let target = round_to_rotation(depth);
        let (nodes, root_mover) = self.flashlight_build(board, target, cap_at);
        // Best root move = the root child maximizing the root player's component (first max).
        let mut best: Option<(Move, [i16; 4])> = None;
        for n in nodes.iter().skip(1) {
            if n.parent == 0 {
                let take = best.is_none_or(|(_, bv)| n.value[root_mover] > bv[root_mover]);
                if take {
                    best = n.mv.map(|m| (m, n.value));
                }
            }
        }
        best
    }

    /// Build the flashlight search tree for `board` to `target` plies and back up Max^n values.
    /// Returns the node arena (index 0 = root) and the root mover's seat index. **The shared core**
    /// of [`Self::search_flashlight`] (best move) and [`Self::search_flashlight_info`] (telemetry),
    /// so both see the exact same tree — the played move is identical with or without telemetry.
    fn flashlight_build(
        &mut self,
        board: &Board,
        target: u32,
        cap_at: impl Fn(u32) -> usize,
    ) -> (Vec<FNode>, usize) {
        self.nodes = 0;
        let root_mover = board.side_to_move.index();
        let mut nodes: Vec<FNode> = vec![FNode {
            parent: 0,
            mover: root_mover,
            mv: None,
            value: [i16::MIN; 4],
            leaf: false,
            best_child: 0,
        }];
        let mut frontier: Vec<(usize, Board)> = vec![(0, board.clone())];

        for level in 0..target {
            let mut cands: Vec<(usize, Board, i32)> = Vec::new();
            for (idx, b) in &frontier {
                let mut bb = b.clone();
                let mover = bb.side_to_move.index();
                let moves = generate_legal(&mut bb);
                if moves.is_empty() {
                    // Terminal: the mover is eliminated → mate-distance value (kept in centipawns).
                    self.nodes += 1;
                    let mut v = self.eval_with_win(&bb);
                    let dist = MATE - (level as i16).min(MATE - 1);
                    v[mover] = -dist;
                    nodes[*idx].value = v;
                    nodes[*idx].leaf = true;
                    continue;
                }
                for mv in moves {
                    let undo = bb.make_move(mv);
                    self.nodes += 1;
                    // Pruning score: how good this move is for the player who just made it (goal + means).
                    let score = i32::from(self.eval_with_win(&bb)[mover]);
                    let child_board = bb.clone();
                    bb.unmake_move(undo);
                    let cidx = nodes.len();
                    nodes.push(FNode {
                        parent: *idx,
                        mover: child_board.side_to_move.index(),
                        mv: Some(mv),
                        value: [i16::MIN; 4],
                        leaf: false,
                        best_child: 0,
                    });
                    cands.push((cidx, child_board, score));
                }
            }
            if cands.is_empty() {
                break;
            }
            // Prune the level to the top `cap_at(level)` by score; pruned nodes become leaves. The
            // cap can vary by level (e.g. grow per rotation) — that's the schedule knob.
            let cap = cap_at(level);
            if cands.len() > cap {
                cands.sort_by(|a, b| b.2.cmp(&a.2));
                for (cidx, cb, _) in cands.drain(cap..) {
                    nodes[cidx].value = self.eval_with_win(&cb);
                    nodes[cidx].leaf = true;
                }
            }
            frontier = cands.into_iter().map(|(c, b, _)| (c, b)).collect();
        }
        // The surviving frontier (at max depth) are leaves.
        for (idx, b) in &frontier {
            if !nodes[*idx].leaf {
                nodes[*idx].value = self.eval_with_win(b);
                nodes[*idx].leaf = true;
            }
        }
        // Max^n backup: children (always a higher index than their parent) propagate up; each parent
        // keeps the child that maximizes the parent's own component (and records it for the PV).
        for idx in (1..nodes.len()).rev() {
            let cv = nodes[idx].value;
            let p = nodes[idx].parent;
            let pm = nodes[p].mover;
            if !nodes[p].leaf && cv[pm] > nodes[p].value[pm] {
                nodes[p].value = cv;
                nodes[p].best_child = idx;
            }
        }
        (nodes, root_mover)
    }

    /// Flashlight search **with telemetry** (candidates, PV, nodes) — same tree and same best move as
    /// [`Self::search_flashlight`], plus the data a UI/protocol needs to show what the engine thought.
    pub fn search_flashlight_info(
        &mut self,
        board: &Board,
        depth: u32,
        cap_at: impl Fn(u32) -> usize,
    ) -> SearchInfo {
        let target = round_to_rotation(depth);
        let (nodes, root_mover) = self.flashlight_build(board, target, cap_at);
        // Root children, with the best (first max by the root mover's component) — identical
        // selection to `search_flashlight`, so `best` is the move actually played.
        let mut best_idx: Option<usize> = None;
        let mut cands: Vec<(usize, Move, [i16; 4])> = Vec::new();
        for (i, n) in nodes.iter().enumerate().skip(1) {
            if n.parent == 0
                && let Some(mv) = n.mv
            {
                cands.push((i, mv, n.value));
                if best_idx.is_none_or(|bi| n.value[root_mover] > nodes[bi].value[root_mover]) {
                    best_idx = Some(i);
                }
            }
        }
        cands.sort_by(|a, b| b.2[root_mover].cmp(&a.2[root_mover]));
        let candidates: Vec<(Move, [i16; 4])> = cands.iter().map(|&(_, m, v)| (m, v)).collect();
        let (best, value) = match best_idx {
            Some(bi) => (nodes[bi].mv, nodes[bi].value),
            None => (None, [0i16; 4]),
        };
        // PV: from the best root child, follow `best_child` until a leaf.
        let mut pv = Vec::new();
        let mut cur = best_idx;
        while let Some(c) = cur {
            if let Some(mv) = nodes[c].mv {
                pv.push(mv);
            }
            let nc = nodes[c].best_child;
            cur = (nc != 0).then_some(nc);
        }
        SearchInfo {
            best,
            value,
            depth: target,
            nodes: self.nodes,
            candidates,
            pv,
        }
    }

    #[inline]
    fn over_budget(&self) -> bool {
        self.node_budget != 0 && self.nodes >= self.node_budget
    }

    /// Every legal root move with its backed-up `[i16;4]` value (single full-width pass at the
    /// rounded `depth`). Lets a caller rank a specific move against the engine's choice. Late moves
    /// may be cut to a static value once the node budget is spent (they still appear).
    pub fn root_move_values(&mut self, board: &mut Board, depth: u32) -> Vec<(Move, [i16; 4])> {
        let target = round_to_rotation(depth);
        self.nodes = 0;
        let mut moves = generate_legal(board);
        let tt_move = self.tt.probe(board.zobrist).and_then(|e| e.best_move);
        move_order::order(board, &mut moves, tt_move, 0, &self.order_state);
        let mut out = Vec::with_capacity(moves.len());
        for mv in moves {
            let undo = board.make_move(mv);
            let child = self.maxn(board, target.saturating_sub(1), 1);
            board.unmake_move(undo);
            out.push((mv, child));
        }
        out
    }

    /// One full-width root search at a fixed depth (a multiple of 4). The root considers ALL legal
    /// moves (no beam) so a strong move ordered past the beam is never dropped.
    fn search_depth(&mut self, board: &mut Board, depth: u32) -> Option<(Move, [i16; 4])> {
        let mover = board.side_to_move.index();
        let mut moves = generate_legal(board);
        if moves.is_empty() {
            return None;
        }
        let tt_move = self.tt.probe(board.zobrist).and_then(|e| e.best_move);
        move_order::order(board, &mut moves, tt_move, 0, &self.order_state);

        let mut best: Option<(Move, [i16; 4])> = None;
        for mv in moves {
            if self.over_budget() {
                break; // budget spent → keep best-so-far, stop expanding further root moves
            }
            let undo = board.make_move(mv);
            let child = self.maxn(board, depth.saturating_sub(1), 1);
            board.unmake_move(undo);
            let take = best.is_none_or(|(_, bv)| child[mover] > bv[mover]);
            if take {
                best = Some((mv, child));
            }
        }
        if let Some((mv, v)) = best {
            self.tt
                .store(board.zobrist, clamp_depth(depth), v, Bound::Exact, Some(mv));
            if !mv.flags.capture {
                self.order_state.add_killer(0, mv);
                self.order_state
                    .bump_history(mv.from, mv.to, (depth * depth) as i32);
            }
        }
        best
    }

    fn maxn(&mut self, board: &mut Board, depth: u32, ply: u32) -> [i16; 4] {
        self.nodes += 1;
        if self.over_budget() {
            return (self.eval)(board, &mut self.lines); // budget spent → cut, treat as a leaf
        }

        if depth == 0 {
            if self.quiescence {
                return self.qsearch(board, 0);
            }
            return (self.eval)(board, &mut self.lines);
        }

        let mover = board.side_to_move.index();

        // DKW node: the side to move is a dead king walking *randomly* — it does not maximize its own
        // component, so back up the *expected* value (uniform average over its king moves). This is an
        // expectimax node embedded in the Max^n tree; live players above still maximize against it.
        if board.is_dkw(board.side_to_move) {
            let dkw_moves = generate_legal(board); // king-only, no check filter (DKW ignores check)
            if dkw_moves.is_empty() {
                // DKW-king stalemate (§1.8): the king is removed, no further contribution from it.
                return (self.eval)(board, &mut self.lines);
            }
            let n = dkw_moves.len() as i32;
            let mut sum = [0i32; 4];
            for mv in dkw_moves {
                let undo = board.make_move(mv);
                let child = self.maxn(board, depth - 1, ply + 1);
                board.unmake_move(undo);
                for k in 0..4 {
                    sum[k] += i32::from(child[k]);
                }
            }
            return [
                (sum[0] / n) as i16,
                (sum[1] / n) as i16,
                (sum[2] / n) as i16,
                (sum[3] / n) as i16,
            ];
        }

        let mut moves = generate_legal(board);
        if moves.is_empty() {
            // Terminal: the mover has no legal moves → eliminated. §1.8 makes both checkmate (in
            // check) and stalemate (not in check) an elimination for the mover, so the search value
            // is identical for the two; the distinction (and the +20/+10 point awards) is game
            // scoring on `board.points`, handled at play time, not in this centipawn backup (§1.7).
            // The mover's own component becomes a mate-distance score; everyone else keeps their
            // positional standing.
            let mut v = (self.eval)(board, &mut self.lines);
            let dist = MATE - (ply.min((MATE - 1) as u32) as i16);
            v[mover] = -dist;
            return v;
        }
        let tt_move = self.tt.probe(board.zobrist).and_then(|e| e.best_move);
        move_order::order(board, &mut moves, tt_move, ply as usize, &self.order_state);

        let mut best = [i16::MIN; 4];
        let mut best_mover = i32::MIN;
        let mut best_move = None;
        let beam_cap = if self.noise_adaptive {
            // Narrow on a real tactic (laser the forcing line), broad when quiet (compare options).
            if position_is_noisy(board, &moves) {
                self.deep_floor
            } else {
                self.beam_width
            }
        } else if self.adaptive_beam {
            beam_at(self.beam_width, self.deep_floor, ply)
        } else {
            self.beam_width
        };
        for (i, mv) in moves.into_iter().enumerate() {
            // Hard per-node branch cap: expand only the top `beam_cap` MVV-LVA-ordered moves. The best
            // captures are ordered first, so the cap keeps them — there is **no** capture exemption
            // (an unbounded capture exemption is what let capture-dense nodes explode and forced the
            // node-budget band-aid). This bounds the whole tree to `root_moves × Π beam_cap`, so full
            // depth-8/12 stay tractable with no time/depth cutoff.
            if i >= beam_cap {
                break;
            }
            let undo = board.make_move(mv);
            // Late move reductions (forward pruning, default-off): search a late *quiet* move at
            // reduced depth; only if it beats the current best do we confirm it at full depth.
            // Captures, promotions, and the first `LMR_LATE_MOVES` moves are never reduced
            // (tactical completeness).
            let reduce = self.forward_pruning
                && depth >= LMR_MIN_DEPTH
                && i >= LMR_LATE_MOVES
                && !mv.flags.capture
                && mv.promotion.is_none();
            let mut child = self.maxn(board, depth - if reduce { 2 } else { 1 }, ply + 1);
            if reduce && i32::from(child[mover]) > best_mover {
                child = self.maxn(board, depth - 1, ply + 1);
            }
            board.unmake_move(undo);
            if i32::from(child[mover]) > best_mover {
                best_mover = i32::from(child[mover]);
                best = child;
                best_move = Some(mv);
            }
        }
        if let Some(mv) = best_move {
            self.tt.store(
                board.zobrist,
                clamp_depth(depth),
                best,
                Bound::Exact,
                Some(mv),
            );
            if !mv.flags.capture {
                self.order_state.add_killer(ply as usize, mv);
                self.order_state
                    .bump_history(mv.from, mv.to, (depth * depth) as i32);
            }
        }
        best
    }

    /// Tactical quiescence at a leaf: continue along captures/promotions until quiet, returning a
    /// value only at a **rotation boundary** (`qply % 4 == 0`, Hard Rule #1) so the per-player vector
    /// stays fair. At a boundary the mover may "stand pat" (stop); mid-rotation it advances via a
    /// null `pass` (`make_null`) when it has no tactical move or declines one — the standard
    /// stand-pat assumption, sound enough for a default-off lever. Bounded by `QUIESCENCE_MAX_PLY`
    /// (one rotation), so cost stays small (tactical branching only).
    fn qsearch(&mut self, board: &mut Board, qply: u32) -> [i16; 4] {
        self.nodes += 1;
        let mover = board.side_to_move.index();
        let stand_pat = (self.eval)(board, &mut self.lines);
        if self.over_budget() {
            return stand_pat; // budget spent → stop extending the tactical line
        }

        let at_boundary = qply % 4 == 0;
        if at_boundary && qply >= QUIESCENCE_MAX_PLY {
            return stand_pat; // capped, at a boundary
        }

        let tacticals: Vec<Move> = generate_legal(board)
            .into_iter()
            .filter(|m| m.flags.capture || m.promotion.is_some())
            .collect();
        if at_boundary && tacticals.is_empty() {
            return stand_pat; // quiet at a boundary → leaf here
        }

        // At a boundary the mover's baseline is to stop (stand pat). Mid-rotation it must advance to
        // the next boundary, so its baseline is a null pass rather than an immediate return.
        let mut best = stand_pat;
        let mut best_mover = if at_boundary {
            i32::from(stand_pat[mover])
        } else {
            let u = board.make_null();
            let child = self.qsearch(board, qply + 1);
            board.unmake_null(u);
            best = child;
            i32::from(child[mover])
        };

        for mv in tacticals {
            let undo = board.make_move(mv);
            let child = self.qsearch(board, qply + 1);
            board.unmake_move(undo);
            if i32::from(child[mover]) > best_mover {
                best_mover = i32::from(child[mover]);
                best = child;
            }
        }
        best
    }
}

fn clamp_depth(depth: u32) -> u8 {
    depth.min(u8::MAX as u32) as u8
}

/// Round a requested depth up to the next positive multiple of 4, so the Max^n perspective
/// chain ends on a full 4-player rotation (Hard Rule #1: valid depths are 4, 8, 12, …).
fn round_to_rotation(depth: u32) -> u32 {
    depth.div_ceil(4).max(1) * 4
}

/// Rotation-aware stepwise beam schedule.
/// Drops sharply at each rotation boundary (ply 4, 8, 12...).
/// Ply 1 = just below root (after root move is made).
/// "Noisy" = a real tactic is available: the mover is in check, or a *favorable* capture exists
/// (victim worth more than the attacker — a cheap MVV>LVA proxy for "a winning capture"). Quiet
/// otherwise. Used by the noise-adaptive beam (narrow on noisy, broad on quiet).
fn position_is_noisy(board: &Board, moves: &[Move]) -> bool {
    if in_check(board, board.side_to_move) {
        return true;
    }
    moves.iter().any(|m| {
        m.flags.capture
            && match (board.piece_at(m.from), board.piece_at(m.to)) {
                (Some(a), Some(v)) => v.piece_type.eval_value() > a.piece_type.eval_value(),
                _ => false,
            }
    })
}

fn beam_at(base: usize, floor: usize, ply: u32) -> usize {
    // ply 0 = root (full width, handled in `search_depth`); ply 1 = first response below the root.
    // Hard per-node branch cap, tapering by rotation, floored at `floor` (1 = laser).
    match ply {
        0..=3 => base.max(floor), // first rotation: responses to our root move
        4..=7 => (base / 2).max(floor), // second rotation
        _ => floor,               // third rotation onward
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::board::types::{Piece, PieceType, Player, Square};

    fn at(s: &str) -> Square {
        Square::from_algebraic(s).unwrap()
    }

    /// A sparse board with the four kings on their start squares + extra pieces.
    fn with_kings(extra: &[(&str, Player, PieceType)]) -> Board {
        let mut b = Board::empty();
        for (sq, pl) in [
            ("h1", Player::Red),
            ("a7", Player::Blue),
            ("g14", Player::Yellow),
            ("n8", Player::Green),
        ] {
            b.set_piece(at(sq), Some(Piece::new(pl, PieceType::King)));
        }
        for (sq, pl, pt) in extra {
            b.set_piece(at(sq), Some(Piece::new(*pl, *pt)));
        }
        b.recompute_zobrist();
        b
    }

    #[test]
    fn search_returns_a_legal_move_and_counts_nodes() {
        let mut b = with_kings(&[("g7", Player::Red, PieceType::Rook)]);
        let mut s = Searcher::new(8);
        let (mv, _v) = s.search(&mut b, 4).expect("has moves");
        assert!(s.nodes > 0);
        // The returned move must be one of the legal moves.
        assert!(generate_legal(&mut b).contains(&mv));
    }

    #[test]
    fn search_grabs_a_free_queen() {
        // Red rook on g7 can capture an undefended Blue queen on g10 up the file.
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("g10", Player::Blue, PieceType::Queen),
        ]);
        let mut s = Searcher::new(8);
        let (mv, v) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(mv.from, at("g7"));
        assert_eq!(mv.to, at("g10"), "Max^n should take the free queen");
        assert!(mv.flags.capture);
        // Red ends materially ahead of Blue.
        assert!(v[Player::Red.index()] > v[Player::Blue.index()]);
    }

    #[test]
    fn beam_keeps_the_best_capture() {
        // Even with a narrow beam, the MVV-LVA-ordered free-queen capture is expanded.
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("g10", Player::Blue, PieceType::Queen),
        ]);
        let mut s = Searcher::new(8).with_beam_width(3);
        let (mv, _) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(mv.to, at("g10"));
    }

    // --- Review/hardening: Max^n backup, root completeness, determinism, depth rounding ---

    fn find(board: &Board, player: Player, pt: PieceType) -> Option<Square> {
        (0..196u8)
            .map(Square::new)
            .find(|&sq| board.piece_at(sq) == Some(Piece::new(player, pt)))
    }

    /// Synthetic eval: Red's component = (Red king's file) × 100; others 0.
    fn red_king_file(board: &Board, _l: &mut LineMap) -> [i16; 4] {
        let f = find(board, Player::Red, PieceType::King).map_or(0, |s| s.file() as i16 * 100);
        [f, 0, 0, 0]
    }

    /// Synthetic eval: Blue's component = (Blue king's file) × 100; others 0.
    fn blue_king_file(board: &Board, _l: &mut LineMap) -> [i16; 4] {
        let f = find(board, Player::Blue, PieceType::King).map_or(0, |s| s.file() as i16 * 100);
        [0, f, 0, 0]
    }

    #[test]
    fn maxn_node_maximizes_the_movers_own_component() {
        // Four kings only. A Red node maximizes RED's own component (king file): from h1
        // (file 7) the best reachable file is 8 → 800.
        let mut b = with_kings(&[]);
        let mut s = Searcher::new(1).with_eval(red_king_file);
        assert_eq!(s.maxn(&mut b, 1, 0), [800, 0, 0, 0]);

        // A Blue node maximizes BLUE's own component (not minimizing Red — the Max^n property,
        // vs paranoid minimax). Blue king a7 (file 0) reaches file 1 → 100.
        let mut bb = with_kings(&[]);
        bb.side_to_move = Player::Blue;
        bb.recompute_zobrist();
        let mut sb = Searcher::new(1).with_eval(blue_king_file);
        assert_eq!(sb.maxn(&mut bb, 1, 0), [0, 100, 0, 0]);
    }

    #[test]
    fn root_considers_all_moves_not_just_the_beam() {
        // Red rook g7 can capture a Blue pawn on h7 (sorts first under MVV-LVA). With beam 1, a
        // beamed root would only try the capture; root-full-width still finds the king move that
        // maximizes Red's (synthetic) score — king h1 → file 8 (800) beats keeping it (700).
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("h7", Player::Blue, PieceType::Pawn),
        ]);
        let mut s = Searcher::new(1).with_eval(red_king_file).with_beam_width(1);
        let (mv, v) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(
            mv.from,
            at("h1"),
            "found the king move, not just the top-ordered capture"
        );
        assert_eq!(v[Player::Red.index()], 800);
    }

    #[test]
    fn fresh_searches_are_deterministic() {
        let mk = || {
            with_kings(&[
                ("g7", Player::Red, PieceType::Rook),
                ("g10", Player::Blue, PieceType::Queen),
            ])
        };
        let r1 = Searcher::new(8).with_beam_width(4).search(&mut mk(), 4);
        let r2 = Searcher::new(8).with_beam_width(4).search(&mut mk(), 4);
        assert_eq!(r1, r2, "two fresh searchers give identical results");
    }

    #[test]
    fn depth_rounds_up_to_a_full_rotation() {
        assert_eq!(round_to_rotation(0), 4);
        assert_eq!(round_to_rotation(1), 4);
        assert_eq!(round_to_rotation(4), 4);
        assert_eq!(round_to_rotation(5), 8);
        assert_eq!(round_to_rotation(8), 8);
    }

    // --- P6 refinements: terminal scoring, mate distance, iterative deepening ---

    /// Board where Red has no pieces → Red (to move) has no legal moves.
    fn red_has_no_moves() -> Board {
        let mut b = Board::empty();
        b.set_piece(at("a7"), Some(Piece::new(Player::Blue, PieceType::King)));
        b.set_piece(at("g14"), Some(Piece::new(Player::Yellow, PieceType::King)));
        b.set_piece(at("n8"), Some(Piece::new(Player::Green, PieceType::King)));
        b.side_to_move = Player::Red;
        b.recompute_zobrist();
        b
    }

    #[test]
    fn no_legal_moves_is_terminal_mate_score() {
        // No legal moves for the mover → eliminated; the mover's own component is the mate score.
        let mut b = red_has_no_moves();
        let mut s = Searcher::new(1);
        let v = s.maxn(&mut b, 4, 0);
        assert_eq!(
            v[Player::Red.index()],
            -MATE,
            "mated mover gets -(MATE - ply), ply = 0"
        );
    }

    #[test]
    fn mate_distance_favors_delay_for_the_mated_side() {
        let mut s = Searcher::new(1);
        let near = s.maxn(&mut red_has_no_moves(), 4, 0)[Player::Red.index()];
        let far = s.maxn(&mut red_has_no_moves(), 4, 5)[Player::Red.index()];
        assert_eq!(near, -MATE);
        assert!(
            far > near,
            "a later (deeper-ply) mate is less bad for the mated side"
        );
    }

    #[test]
    fn iterative_deepening_returns_a_legal_move() {
        // depth 8 exercises the ID loop (d = 4 then 8). A narrow beam keeps it tractable in debug —
        // unbounded depth-8 Max^n needs shallow pruning (deferred) to be fast.
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("g10", Player::Blue, PieceType::Queen),
        ]);
        let mut s = Searcher::new(8).with_beam_width(2);
        let (mv, _) = s.search(&mut b, 8).expect("has moves");
        assert!(generate_legal(&mut b).contains(&mv));
    }

    #[test]
    fn forward_pruning_visits_fewer_nodes() {
        // Enough branching for late-move reductions to bite. Default-off vs on (ablation arm).
        let mk = || {
            with_kings(&[
                ("g7", Player::Red, PieceType::Rook),
                ("g10", Player::Blue, PieceType::Queen),
                ("c7", Player::Yellow, PieceType::Bishop),
            ])
        };
        let mut off = Searcher::new(8).with_beam_width(8).with_eval(red_king_file);
        let _ = off.search(&mut mk(), 4);
        let nodes_off = off.nodes;

        let mut on = Searcher::new(8)
            .with_beam_width(8)
            .with_forward_pruning(true)
            .with_eval(red_king_file);
        let (mv, _) = on.search(&mut mk(), 4).expect("has moves");
        let nodes_on = on.nodes;

        assert!(
            nodes_on < nodes_off,
            "forward pruning should visit fewer nodes ({nodes_on} on vs {nodes_off} off)"
        );
        assert!(
            generate_legal(&mut mk()).contains(&mv),
            "forward pruning still returns a legal move"
        );
    }

    #[test]
    fn adaptive_beam_schedule_narrows_with_ply() {
        // Base-scaled, stepwise at rotation boundaries: base -> base/2 -> floor. (floor = 2 here.)
        assert_eq!(beam_at(8, 2, 1), 8, "ply 1: base (within R1)");
        assert_eq!(beam_at(8, 2, 3), 8, "ply 3: still base");
        assert_eq!(beam_at(8, 2, 4), 4, "ply 4: base/2 after R1");
        assert_eq!(beam_at(8, 2, 7), 4, "ply 7: still base/2 (within R2)");
        assert_eq!(beam_at(8, 2, 8), 2, "ply 8: floor after R2");
        assert_eq!(beam_at(8, 2, 12), 2, "ply 12: floor");
        // Laser floor (1): single line deep, no deep branching.
        assert_eq!(beam_at(8, 1, 8), 1, "laser: floor 1 deep");
        assert_eq!(beam_at(8, 1, 1), 8, "laser: still wide near root");
    }

    #[test]
    fn flashlight_matches_maxn_without_pruning() {
        // Four kings only → tiny branching → the full tree is small; a huge level cap = no pruning,
        // so the flashlight must reproduce exact Max^n. This validates the tree build + backup.
        let mut b = with_kings(&[]);
        let exact = Searcher::new(1).with_eval(red_king_file).maxn(&mut b, 4, 0);
        let mut fl = Searcher::new(1).with_eval(red_king_file);
        let (mv, v) = fl
            .search_flashlight(&b, 4, |_| 1_000_000)
            .expect("flashlight returns a move");
        assert_eq!(
            v, exact,
            "flashlight without pruning must equal exact Max^n"
        );
        assert!(
            generate_legal(&mut b).contains(&mv),
            "flashlight returns a legal root move"
        );
    }

    #[test]
    fn flashlight_info_best_matches_played_move() {
        // The telemetry path must pick the SAME move as the bare flashlight (it reads the same tree)
        // and rank its candidates best-first — so emitting telemetry never changes the played move.
        use crate::board::fen4;
        let mut s = Searcher::new(8);
        let mut b = fen4::parse(fen4::START_FEN4).unwrap();
        for _ in 0..3 {
            let plain = s.search_flashlight(&b, 8, |_| 200);
            let info = s.search_flashlight_info(&b, 8, |_| 200);
            assert_eq!(
                plain.map(|(m, _)| m),
                info.best,
                "telemetry best == played move"
            );
            assert_eq!(
                info.candidates.first().map(|&(m, _)| m),
                info.best,
                "candidates are ranked best-first"
            );
            let Some((m, _)) = plain else { break };
            let _ = b.make_move(m);
        }
    }

    #[test]
    fn adaptive_beam_visits_fewer_nodes_at_depth() {
        // Stepwise adaptive: ply 1-3 beam 30, ply 4-7 beam 12, ply 8+ beam 6 — needs depth 8 to
        // cross the ply-4 boundary. Uses a CHEAP synthetic eval and a low-branching position (one
        // mobile Red rook + three corner kings with ~3 moves each) so the depth-8 tree stays ~10^5
        // nodes. With the real eval at this depth the search is ~10^7+ leaves and runs for hours —
        // that is a property of the position/eval, not the lever under test (node count).
        let mk = || {
            let mut b = Board::empty();
            b.set_piece(at("d1"), Some(Piece::new(Player::Red, PieceType::King)));
            b.set_piece(at("g7"), Some(Piece::new(Player::Red, PieceType::Rook)));
            b.set_piece(at("a4"), Some(Piece::new(Player::Blue, PieceType::King)));
            b.set_piece(at("k1"), Some(Piece::new(Player::Yellow, PieceType::King)));
            b.set_piece(at("n4"), Some(Piece::new(Player::Green, PieceType::King)));
            b.recompute_zobrist();
            b
        };
        let mut flat = Searcher::new(8)
            .with_beam_width(30)
            .with_eval(red_king_file);
        let _ = flat.search(&mut mk(), 8);
        let nodes_flat = flat.nodes;

        let mut adaptive = Searcher::new(8)
            .with_beam_width(30)
            .with_adaptive_beam(true)
            .with_eval(red_king_file);
        let (mv, _) = adaptive.search(&mut mk(), 8).expect("has moves");
        let nodes_adaptive = adaptive.nodes;

        assert!(
            nodes_adaptive < nodes_flat,
            "adaptive beam should visit fewer nodes at depth 8 ({nodes_adaptive} vs {nodes_flat} flat)"
        );
        assert!(
            generate_legal(&mut mk()).contains(&mv),
            "adaptive beam still returns a legal move"
        );
    }

    #[test]
    fn quiescence_is_a_noop_in_quiet_positions() {
        // Four kings only → no tactical moves anywhere → qsearch returns the static eval unchanged.
        let mut b1 = with_kings(&[]);
        let v_off = Searcher::new(1).maxn(&mut b1, 0, 0);
        let mut b2 = with_kings(&[]);
        let v_on = Searcher::new(1).with_quiescence(true).maxn(&mut b2, 0, 0);
        assert_eq!(
            v_off, v_on,
            "quiescence changes nothing when there are no captures"
        );
    }

    #[test]
    fn quiescence_extends_pending_captures() {
        // Red rook can capture a Blue queen up the g-file → quiescence searches past the leaf.
        let mk = || {
            with_kings(&[
                ("g7", Player::Red, PieceType::Rook),
                ("g10", Player::Blue, PieceType::Queen),
            ])
        };
        let mut off = Searcher::new(1);
        let _ = off.maxn(&mut mk(), 0, 0);
        let nodes_off = off.nodes;

        let mut on = Searcher::new(1).with_quiescence(true);
        let _ = on.maxn(&mut mk(), 0, 0);
        let nodes_on = on.nodes;

        assert!(
            nodes_on > nodes_off,
            "quiescence searches the pending capture ({nodes_on} vs {nodes_off})"
        );
    }

    #[test]
    fn node_budget_bounds_the_search() {
        // A tiny budget cuts an otherwise-unbounded depth-8 search; it must still return a legal
        // best-so-far move and stop close to the budget (small overshoot for stack unwinding).
        let mk = || {
            with_kings(&[
                ("g7", Player::Red, PieceType::Rook),
                ("g10", Player::Blue, PieceType::Queen),
                ("c7", Player::Yellow, PieceType::Bishop),
            ])
        };
        let mut s = Searcher::new(8).with_beam_width(30).with_node_budget(500);
        let (mv, _) = s.search(&mut mk(), 8).expect("has moves");
        assert!(
            generate_legal(&mut mk()).contains(&mv),
            "budgeted search still returns a legal move"
        );
        assert!(
            s.nodes < 5_000,
            "node budget bounds the work (got {} for a budget of 500)",
            s.nodes
        );
    }
}
