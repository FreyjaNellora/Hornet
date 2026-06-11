# Phase 6: Search

## Commander's Intent

Search the 4PC tree with **Max^n** — each node maximizes the moving player's own component of the
per-player value vector `V`, backed up whole, never collapsed to a scalar (Hard Rule #3). Find the
best move for the side to move within a depth budget. Owner: **claude**.

## Reading List

1. `STATUS.md` · 2. this file · 3. `sessions/phase-6/session-001.md`
4. `HORNET-BUILD-SPEC.md` §6 (search), §6.3 (eval interface), §1.1 (depth ≡ 0 mod 4).
5. `hornet-engine/src/{search,tt,move_order}.rs`, `board/zobrist.rs`, `src/eval.rs` (Kimi's `eval_4vec`).

## Write Scope

**Owns:** `hornet-engine/src/search.rs`, `tt.rs`, `move_order.rs`, `board/zobrist.rs`.
**Read-only:** `eval.rs`, `queries.rs` (Kimi), everything else.

## Current State

| Field | Value |
|-------|-------|
| Status | **core complete + hardened + ordering levers measured/landed-off** (2026-06-10, EXP-020/021) |
| Last Session | 2026-06-10 — `sessions/phase-6/session-003.md` |
| Blocking Issues | none |
| Next Action | B5 corpus regen (config with Kimi); free-capture strength gate if ever wanted on; shallow pruning stays deferred |

## Move-ordering levers (2026-06-10, EXP-020/021)

- `OrderState.ffa_bounty` / `.free_capture` — **default off** (Hard Rule #6; the old `const = true`
  was the violation the blind review caught). Builders: `with_ffa_bounty_order` /
  `with_free_capture_order`. Maxn path only — the flashlight never calls `move_order`.
- The buggy (inverted) free-capture heuristic changed **11.6%** of played moves at beam 4
  (0.9%/0.6% at 10/30) → bootstrap corpus tainted, regenerate (B5). Landing off cost nothing
  (self-play noise). New move_match baseline (flags off): 13.5%/13.6%/13.6% at beams 4/10/30.
- `count_defenders` → `is_defended` (`board::attacks::is_attacked_by`): polarity fixed, real
  geometry, measured cost ≈ 0 (EXP-021). `order()` uses `sort_by_cached_key` (score once per move).
- Protocol `go` (B3) plays `search_flashlight` cap 1200 (SYNTHESIS); the maxn `search()` remains
  the exact-path reference and harness default.

## Hardening pass (2026-06-03)

Reviewed the rushed core and fixed three correctness issues + added tests:
- **TT demoted to move-ordering only** — the value cutoff was unsound under beam (beam-approximate
  values stored as `Exact`); removed it. Real value reuse + bounds come with shallow pruning.
- **Root now full-width** — the root iterated `take(beam_width)`, which could drop a strong move
  ordered past the beam. Root considers all legal moves; beam stays at internal nodes.
- **Depth ≡ 0 mod 4 enforced** — `search` rounds the requested depth up to a full rotation
  (`round_to_rotation`, Hard Rule #1).
- Tests added: Max^n backup (each node maximizes its *own* component — via an injectable synthetic
  eval), root completeness, fresh-search determinism, depth rounding. Confirmed `points` excluded
  from the Zobrist hash is TT-safe (eval is points-independent).

## Done (acceptance — core)

- [x] **Zobrist** (`board/zobrist.rs`): incremental hash on make/unmake, verified vs recompute.
- [x] **Transposition table** (`tt.rs`): power-of-two, depth-preferred replace, probe/store.
- [x] **Max^n search** (`search.rs`): `Searcher::search(board, depth) -> Option<(Move, [i16;4])>`;
      beam (top-`beam_width`, default 30), TT exact-value reuse + best-move hint, leaves = `eval_4vec`.
- [x] **Move ordering** (`move_order.rs`): TT move first, then MVV-LVA captures, then quiets.
- [x] Tests: free-queen grab, beam keeps best capture, legal-move/node-count. 59 unit + 3 integ green.

## Remaining (refinements)

- [ ] **Max^n shallow pruning** (Korf/Sturtevant bounded-sum) — provable cutoffs still **deferred**:
      the bounds strategy is an open design fork (see `PITCH-maxn-shallow-pruning.md`); the eval is
      non-constant-sum so naïve provable bounds never fire.
- [x] **Forward pruning (LMR)** (2026-06-04): the practical speed lever — late *quiet* moves searched
      at reduced depth, re-searched only if they beat the current best; captures/promotions/first-N
      never reduced (tactical completeness). **Default-off** with an ablation arm
      (`Searcher::with_forward_pruning`). `search.rs`.
- [x] **Adaptive beam scheduling** (2026-06-04): the beam narrows with ply toward `BEAM_MIN`
      (captures/promotions never dropped); branching budget spent near the root. Bounds
      lines-per-node only — the multiple-of-4 depth rule is untouched. **Default-off**
      (`Searcher::with_adaptive_beam`). The breadth-side complement to LMR.
- [x] **Proper terminal scoring** (§1.8, 2026-06-04): a no-legal-moves node eliminates the mover with
      a mate-distance score `-(MATE - ply)`, kept in **centipawns** (§1.7 — the search value never
      uses FFA points; the +20/+10 awards are game-scoring on `board.points`, applied at play time).
      Checkmate vs stalemate are the same elimination for the search value; DKW-king stalemate is
      deferred with DKW move-gen. `search.rs::maxn`.
- [x] **Iterative deepening** (4, 8, 12, …, 2026-06-04): `search` drives `search_depth` shallow→deep,
      carrying the TT best-move + killer/history forward.
- [x] **Killers + history** (2026-06-04): `move_order::OrderState` (2 killers/ply + `[from][to]`
      history); order is TT → captures (MVV-LVA + FFA bounty) → killers → history-scored quiets.
- [~] **Perf:** the two speed levers **stack** (`examples/search_bench.rs`, release, start position).
      Depth 4 / beam 20 vs flat (160k nodes): LMR −79.7% (4.9×), adaptive −68.7% (3.3×),
      **both −91.7% (12.2×)**. Depth 8 / beam 6: flat 6.4M nodes / 89 s → **both 228k / 3.1 s (~28×)**
      — deep search now runs in seconds. `nodes/s` ~constant → pure node reduction. Heuristic: the
      pruned configs change move selection (more so as they get aggressive), so both are
      **default-off**; strength is for Kimi's gate to validate before either ships on. Provable
      shallow-pruning value cutoffs remain the orthogonal (sound) win, still deferred on bounds.

- [x] **Quiescence (= "TRS", 2026-06-06):** tactical-only (captures/promotions) leaf extension,
      returning a value **only at a rotation boundary** (`qply % 4 == 0`, Hard Rule #1) so the
      per-player vector stays fair; mid-rotation it advances via `make_null`/`unmake_null`. Bounded by
      `QUIESCENCE_MAX_PLY` (one rotation). Reduces — does not eliminate — the capture-exchange horizon
      effect. **Default-off** (`Searcher::with_quiescence`). Needs a null-move primitive
      (`board::make_null`/`unmake_null`, round-trip tested in `zobrist.rs`). This is Freyja's "TRS"
      (Tactical Resolution Search) landed.
- [x] **Node budget (2026-06-06):** `Searcher::with_node_budget(n)` (0 = unlimited). On a capture-dense
      position the unbounded recursion (deep main search **or** quiescence) could run for hours; the
      budget cuts (returns the static eval at the current node) and stops expanding further root moves /
      deeper ID iterations, returning the best move so far. Real engine gap closed, not just a
      diagnostic aid. Test: `node_budget_bounds_the_search`.

## Strength-gate diagnostic (2026-06-06) — depth is **not** the current bottleneck; the eval is

Ran Kimi's tactical fixtures (`examples/gate_ablation.rs`, 13 testable) over a depth × quiescence
matrix, all with the speed levers (beam 10 + LMR + adaptive) and an 800k-node budget for tractability:

| | quiescence OFF | quiescence ON |
|---|---|---|
| **depth 4** | 0/13 | 1/13 |
| **depth 8** | 0/13 | 1/13 |

- **Doubling depth (4→8) changed nothing** (0/13 either way). The only gain was quiescence (+1/13),
  same at both depths. → the eval is broken, and **that makes the depth result a confound, not a
  verdict**: a faulty eval makes depth useless (shit in → shit out), so "depth didn't help" only
  means "depth can't help *through* a broken eval." The depth question is **deferred** — re-test the
  sweep *after* the eval is fixed. Fix the eval first (it's the binding constraint either way).
- **Process finding:** depth-8 on these tactical (capture-dense) fixtures *explodes* without the node
  budget — the adaptive-beam "tactical completeness" guard never prunes captures, so a capture-dense
  node fans out fully for 8 plies. "Depth 8 is tractable" held for the quiet start position (the
  `search_bench` numbers), **not** for tactical positions. Real depth needs sound pruning + a cheaper
  eval (the always-recompute line cost dominates per-node), not brute force.
- **Direction:** the pruning/ordering work (narrow-and-deep, → master-plan MCTS-for-midgame) is the
  right architecture but is *downstream* of the eval; the immediate leverage is the eval (Kimi's lane:
  wire the dormant `intent`/`bounty`/`zones` substrate + strategy layer in affordably).

## Active Watch Items

- **DKW** (deferred): if search ever reaches an eliminated-player position it needs DKW move-gen.
  Shallow search from normal positions doesn't hit it.
- TT value reuse assumes exact full-beam values; revisit when shallow pruning adds true bounds.

## Downstream Notes

P8 protocol wires `position`/`go` to `Searcher::search`. P7 NNUE (Kimi) eventually replaces/augments
`eval_4vec` behind the same interface — search consumes whatever `eval_4vec` returns.
