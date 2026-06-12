//! Query engine (spec §4). Turns a [`LineMap`] into the four scalar-per-player
//! query outputs: material, positional control, king safety, and crossfire.
//!
//! Hard Rule #4: `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`. Each query traces to exactly
//! one component — no 5th component, no merging.

use crate::board::Piece;
use crate::board::types::{PieceType, Player, Square};
use crate::board::{Board, KING_DELTAS, KNIGHT_DELTAS, offset};
use crate::lines::{LineMap, MAX_REACHERS_PER_SQUARE, PieceLines, SquareReachers, compute_lines};
use crate::zones::aggregate_zone_control;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// QueryVector
// ---------------------------------------------------------------------------

/// The four query outputs, each a per-player vector.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryVector {
    /// Mᵢ: sum of piece values per player (centipawns).
    pub material: [i16; 4],
    /// Pᵢ: centrality-weighted empty-square control.
    pub positional: [i16; 4],
    /// Sᵢ: king safety composite (defenders − attackers + escapes).
    pub safety: [i16; 4],
    /// Oᵢ: converging-enemy penalty (crossfire).
    pub crossfire: [i16; 4],
}

impl QueryVector {
    pub fn zeros() -> Self {
        QueryVector {
            material: [0; 4],
            positional: [0; 4],
            safety: [0; 4],
            crossfire: [0; 4],
        }
    }
}

// ---------------------------------------------------------------------------
// KingSafety
// ---------------------------------------------------------------------------

/// Per-player king-safety breakdown. The evaluator collapses this into a scalar.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct KingSafety {
    pub defenders: u8,
    pub attackers: u8,
    pub attack_value: i16,
    pub escape_squares: u8,
}

// ---------------------------------------------------------------------------
// Material Query (§4.2)
// ---------------------------------------------------------------------------

/// Sum `eval_value()` for all active pieces per player. Hard Rule #8: never use
/// `ffa_points()` here.
pub fn query_material(board: &Board) -> [i16; 4] {
    let mut m = [0i16; 4];
    for i in 0..crate::board::types::TOTAL_SQUARES {
        let sq = Square::new(i as u8);
        if let Some(p) = board.piece_at(sq) {
            m[p.player.index()] += p.piece_type.eval_value();
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Piece-Square Tables (PST) — v2 zone-aware per-piece
// ---------------------------------------------------------------------------

/// Which zone family does this square belong to? Returns (is_center, is_gate, is_quad).
/// Based on the 9 secondary zones from zones.rs / PITCH-secondary-zones.md.
const fn zone_family(r: u8, f: u8) -> (bool, bool, bool) {
    // Center: g7,h7,g8,h8 → ranks 6-7, files 6-7
    let center = r >= 6 && r <= 7 && f >= 6 && f <= 7;
    // Gates: cardinal 2×2 blocks adjacent to center
    let gate_w = r >= 6 && r <= 7 && f >= 2 && f <= 3;
    let gate_e = r >= 6 && r <= 7 && f >= 10 && f <= 11;
    let gate_s = r >= 2 && r <= 3 && f >= 6 && f <= 7;
    let gate_n = r >= 10 && r <= 11 && f >= 6 && f <= 7;
    let gate = gate_w || gate_e || gate_s || gate_n;
    // Quadrants: diagonal 2×2 blocks
    let quad_sw = r >= 4 && r <= 5 && f >= 4 && f <= 5;
    let quad_se = r >= 4 && r <= 5 && f >= 8 && f <= 9;
    let quad_nw = r >= 8 && r <= 9 && f >= 4 && f <= 5;
    let quad_ne = r >= 8 && r <= 9 && f >= 8 && f <= 9;
    let quad = quad_sw || quad_se || quad_nw || quad_ne;
    (center, gate, quad)
}

/// Forward bonus for pawns (Red moves +rank).
const fn pst_forward(r: u8) -> i16 {
    if r < 3 || r > 10 {
        0
    } else {
        (r as i16 - 1) * 4
    }
}

/// PST v3: zone-aware per-piece tables.
///
/// Derived from zone_stats + visit_freq + rook_files + rook_deep data:
/// - Gates are anchors (23.9% occ) — all pieces benefit
/// - Quadrants are diagonal transit (7.5% occ) — bishops love them
/// - Center is contested/deadly (7.7% occ, 32 caps/101 entries) — knights/queens only
/// - ROOKS PREFER EDGES: files a/n (23 each), ranks 1/14 (28, 21). They live on the periphery.
///   Rook avg move: 2.8 squares (short adjustments). Endgame: corners/edges.
///
/// Zone values per piece (centipawns):
///               Center  Gate  Quad  Edge(rook only)
///   Pawn        +3      +2    +1    —
///   Knight      +4      +2    +2    —
///   Bishop      -4      +1    +4    —
///   Rook        -2      +1    +0    —        (edge bonus dropped — start-square confound)
///   Queen       +3      +2    +2    —
///   King        -6      +2    -1    —
///
/// Indexed by [piece_type_index][square_index]. Invalid corners are 0.
static PST: [[i16; 196]; 6] = {
    let mut tables = [[0i16; 196]; 6];
    let mut sq = 0usize;
    while sq < 196 {
        let r = (sq / 14) as u8;
        let f = (sq % 14) as u8;

        // Skip invalid corners
        if (r < 3 || r > 10) && (f < 3 || f > 10) {
            sq += 1;
            continue;
        }

        let (center, gate, quad) = zone_family(r, f);
        let fwd = pst_forward(r);

        // Zone base value (0 if not in any zone family)
        let mut zv = [0i16; 6];
        if center {
            zv[0] = 3; // Pawn
            zv[1] = 4; // Knight
            zv[2] = -4; // Bishop
            zv[3] = -2; // Rook: slight center penalty (periphery piece)
            zv[4] = 3; // Queen
            zv[5] = -6; // King
        } else if gate {
            zv[0] = 2;
            zv[1] = 2;
            zv[2] = 1;
            zv[3] = 1; // Rook: mild gate (open file entry point)
            zv[4] = 2;
            zv[5] = 2;
        } else if quad {
            zv[0] = 1;
            zv[1] = 2;
            zv[2] = 4; // Bishop: quadrants are diagonal = best squares
            zv[3] = 0; // Rook: neutral on quadrants
            zv[4] = 2;
            zv[5] = -1;
        }

        // 0=Pawn, 1=Knight, 2=Bishop, 3=Rook, 4=Queen, 5=King
        tables[0][sq] = zv[0] + fwd;
        tables[1][sq] = zv[1];
        tables[2][sq] = zv[2];
        tables[3][sq] = zv[3]; // Rook: zone value only (edge bonus dropped — start-square confound)
        tables[4][sq] = zv[4];
        tables[5][sq] = zv[5];

        sq += 1;
    }
    tables
};

/// Get PST value for a piece on a square, from the player's perspective.
/// For Red: direct lookup. For others: transform square to Red's perspective.
fn pst_value(piece_type: PieceType, sq: Square, player: Player) -> i16 {
    let pt_idx = match piece_type {
        PieceType::Pawn => 0,
        PieceType::Knight => 1,
        PieceType::Bishop => 2,
        PieceType::Rook => 3,
        PieceType::Queen => 4,
        PieceType::King => 5,
        PieceType::PromotedQueen => 4, // same as queen
    };

    // Transform square to Red's perspective
    let red_sq = match player {
        Player::Red => sq,
        Player::Blue => {
            // Blue's +file is Red's +rank — rotate 90° clockwise
            // Blue file = Red rank, Blue rank = 13 - Red file
            Square::from_rank_file(13 - sq.file(), sq.rank())
        }
        Player::Yellow => {
            // Yellow's -rank is Red's +rank — reflect across rank 6.5
            // Yellow rank = 13 - Red rank, Yellow file = 13 - Red file
            Square::from_rank_file(13 - sq.rank(), 13 - sq.file())
        }
        Player::Green => {
            // Green's -file is Red's +rank — rotate 90° counter-clockwise
            // Green file = Red rank, Green rank = sq.file()
            Square::from_rank_file(sq.file(), sq.rank())
        }
    };

    PST[pt_idx][red_sq.index() as usize]
}

/// Sum PST values per player. Feeds into Pᵢ as positional nudge.
pub fn query_pst(board: &Board) -> [i16; 4] {
    let mut p = [0i16; 4];
    for i in 0..crate::board::types::TOTAL_SQUARES {
        let sq = Square::new(i as u8);
        if !sq.is_valid() {
            continue;
        }
        if let Some(piece) = board.piece_at(sq) {
            p[piece.player.index()] += pst_value(piece.piece_type, sq, piece.player);
        }
    }
    p
}

// ---------------------------------------------------------------------------
// Mobility Query — per-player sum of piece mobilities (empty or enemy squares)
// ---------------------------------------------------------------------------

/// Sum of legal-move-like reach per player: count of squares each piece can reach
/// that are empty or enemy-occupied. This is a proxy for development/activity.
/// Knights/queens that are developed have high mobility; trapped pieces have low.
pub fn query_mobility(lines: &LineMap) -> [i16; 4] {
    let mut m = [0i16; 4];
    for pl in lines.pieces[..lines.piece_count].iter() {
        let mut piece_mob = 0i16;
        for e in pl.entries() {
            // Count if square is empty OR occupied by enemy
            let counts = match e.first_occupant {
                None => true,
                Some(occ) => occ.player != pl.player,
            };
            if counts {
                piece_mob += 1;
            }
        }
        m[pl.player.index()] += piece_mob;
    }
    m
}

// ---------------------------------------------------------------------------
// Development Tempo Query — count of non-pawn pieces off their back rank/file
// ---------------------------------------------------------------------------

/// Development tempo: weighted count of developed non-pawn pieces per player.
///
/// From opening_dev data (16 human games):
///   Knight avg first move: ply 8.4  → weight 3
///   Queen avg first move:  ply 9.3  → weight 3
///   Bishop avg first move: ply 19.2 → weight 2
///   Rook avg first move:   ply 32.2 → weight 1
///
/// A piece is "developed" if it's not on its starting back rank/file:
///   Red: rank > 1,   Blue: file > 0,   Yellow: rank < 12,   Green: file < 13
///
/// This is a position-level dynamic signal — static PSTs cannot capture it.
pub fn query_tempo(board: &Board) -> [i16; 4] {
    let mut tempo = [0i16; 4];
    for i in 0..crate::board::types::TOTAL_SQUARES {
        let sq = Square::new(i as u8);
        if !sq.is_valid() {
            continue;
        }
        if let Some(piece) = board.piece_at(sq) {
            let pt = piece.piece_type;
            // Skip pawns and kings (kings "develop" by castling, not relevant here)
            if pt == PieceType::Pawn || pt == PieceType::King {
                continue;
            }
            let pi = piece.player.index();
            let r = sq.rank();
            let f = sq.file();
            // Check if piece is off its back rank/file (developed)
            let developed = match piece.player {
                Player::Red => r > 1,     // Red starts on rank 1
                Player::Blue => f > 0,    // Blue starts on file 0
                Player::Yellow => r < 12, // Yellow starts on rank 12
                Player::Green => f < 13,  // Green starts on file 13
            };
            if developed {
                let weight = match pt {
                    PieceType::Knight => 3,
                    PieceType::Queen => 3,
                    PieceType::Bishop => 2,
                    PieceType::Rook => 1,
                    _ => 0,
                };
                tempo[pi] += weight;
            }
        }
    }
    tempo
}

// ---------------------------------------------------------------------------
// Positional Control Query (§4.3)
// ---------------------------------------------------------------------------

/// Centrality weight: centre squares (ranks 5-8, files 5-8) score highest.
#[inline]
fn centrality_weight(sq: Square) -> i16 {
    let dr = (sq.rank() as f32 - 6.5).abs();
    let df = (sq.file() as f32 - 6.5).abs();
    let dist = dr.max(df);
    if dist > 5.0 { 0 } else { (5.0 - dist) as i16 }
}

/// Sum centrality-weighted empty-square control per player.
pub fn query_positional_control(lines: &LineMap) -> [i16; 4] {
    let mut p = [0i16; 4];
    for pl in lines.pieces[..lines.piece_count].iter() {
        for e in pl.entries() {
            if e.first_occupant.is_none() {
                p[pl.player.index()] += centrality_weight(e.square);
            }
        }
    }
    p
}

/// Tactical threat value: sum of (attacked enemy piece values) per player,
/// scaled by 1/4 so threats don't equal captures. A threatened queen is worth
/// ~225, not 900 — this ensures capturing it is still clearly better.
///
/// Filter: attacker value ≤ target value (cheap SEE proxy). A pawn threatening a queen counts;
/// a queen threatening a defended pawn doesn't. Prevents Pᵢ inflation from meaningless threats.
pub fn query_threats(lines: &LineMap) -> [i16; 4] {
    let mut t = [0i16; 4];
    for pl in lines.pieces[..lines.piece_count].iter() {
        let attacker_val = pl.piece_type.eval_value();
        for e in pl.entries() {
            if let Some(target) = e.first_occupant {
                if target.player != pl.player {
                    let target_val = target.piece_type.eval_value();
                    // Cheap SEE proxy: only count threats where attacker is cheaper than target
                    if attacker_val <= target_val {
                        t[pl.player.index()] += target_val / 4;
                    }
                }
            }
        }
    }
    t
}

// ---------------------------------------------------------------------------
// Exchange-aware (SEE) threats — experiment EXP-002, default-off (HORNET_SEE=1)
// ---------------------------------------------------------------------------

/// Experiment flag: replace the flat target-value threat term with exchange-resolved (SEE)
/// threats. Toggle with env `HORNET_SEE=1` (read once; only the exact value `1` enables — any
/// other value, including `0`, leaves it off). Default off — measured on the strength gate
/// before it becomes a real config. Runs in centipawns (`eval_value`), Hard Rule #8-clean.
static SEE_THREATS: LazyLock<bool> =
    LazyLock::new(|| std::env::var("HORNET_SEE").is_ok_and(|v| v == "1"));

/// Experiment flag: enable L3 selective offense intent (contested pieces, turn-proximity weighted).
/// Toggle with env `HORNET_SELECTIVE_INTENT=1` (only the exact value `1` enables). Default off —
/// Texel-gated.
static SELECTIVE_INTENT: LazyLock<bool> =
    LazyLock::new(|| std::env::var("HORNET_SELECTIVE_INTENT").is_ok_and(|v| v == "1"));

/// A SEE-winning capture is *available*, not yet taken, so it scores a fraction of the material it
/// would win (keeps "threat < capture", matching the old `/4` convention).
const THREAT_DISCOUNT: i16 = 4;

/// Is `pl` a *direct* attacker/defender of `target` — the first occupant on its ray (sliders) or a
/// stepper/pawn? Filters X-ray reachers out of the inverse index: a slider reaching `target` only
/// past a prior blocker is X-ray and cannot capture until that blocker is gone.
fn reaches_directly(pl: &PieceLines, target: Square) -> bool {
    for e in pl.entries() {
        if e.square == target && e.first_occupant.is_some() {
            return match pl.piece_type {
                PieceType::Bishop
                | PieceType::Rook
                | PieceType::Queen
                | PieceType::PromotedQueen => e.xray_continues, // direct only if `target` is the first blocker
                _ => true, // knight / king / pawn capture: always direct
            };
        }
    }
    false
}

/// Insertion sort, ascending, for the small fixed attacker/defender slices.
fn sort_asc(a: &mut [i16]) {
    for i in 1..a.len() {
        let mut j = i;
        while j > 0 && a[j - 1] > a[j] {
            a.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// Static exchange evaluation on one square (classic swap algorithm), 2-sided: the attacking side
/// initiates with its least-valuable attacker, the owner recaptures with its least-valuable
/// defender, alternating; each side may stop. `attackers`/`defenders` are ascending centipawn
/// values. Returns the net centipawns the attacking side comes out with (≤ 0 if the capture loses).
fn see_swap(target_value: i16, attackers: &[i16], defenders: &[i16]) -> i16 {
    if attackers.is_empty() {
        return 0;
    }
    const CAP: usize = 2 * MAX_REACHERS_PER_SQUARE + 2;
    let mut g = [0i32; CAP];
    g[0] = target_value as i32; // attacker captures the target
    let mut n = 1usize;
    let mut on_square = attackers[0] as i32; // attacker A0 now occupies the square
    let (mut ai, mut di) = (1usize, 0usize);
    let mut owner_to_move = true; // owner recaptures first
    while n < CAP {
        if owner_to_move {
            if di >= defenders.len() {
                break;
            }
            g[n] = on_square; // owner captures the attacker sitting on the square
            n += 1;
            on_square = defenders[di] as i32;
            di += 1;
        } else {
            if ai >= attackers.len() {
                break;
            }
            g[n] = on_square; // attacker recaptures the defender
            n += 1;
            on_square = attackers[ai] as i32;
            ai += 1;
        }
        owner_to_move = !owner_to_move;
    }
    // g[k] = value captured at step k. Fold to running net (in place), then minimax back: each
    // side keeps the better of stopping vs continuing.
    for k in 1..n {
        g[k] -= g[k - 1];
    }
    for k in (1..n).rev() {
        g[k - 1] = -std::cmp::max(-g[k - 1], g[k]);
    }
    g[0] as i16
}

/// SEE-based threats: every opponent's *winning* capture of a (non-king) piece scores a discounted
/// threat for that opponent. Per-attacking-player 2-sided SEE (attacker's pieces vs the owner's
/// defenders); third parties don't enter the swap (4PC: no one recaptures to save another player's
/// piece — that is search's job). Centipawns, fixed-array (no heap allocation in the hot path).
pub fn query_threats_see(lines: &LineMap) -> [i16; 4] {
    let mut t = [0i16; 4];
    for ti in 0..lines.piece_count {
        let target = &lines.pieces[ti];
        let target_val = target.piece_type.eval_value();
        if target_val == 0 {
            continue; // king: capture is terminal (search handles it), not a material threat
        }
        let owner = target.player.index();
        let sr = lines.reachers_at(target.square);

        let mut atk = [[0i16; MAX_REACHERS_PER_SQUARE]; 4];
        let mut atk_n = [0usize; 4];
        let mut def = [0i16; MAX_REACHERS_PER_SQUARE];
        let mut def_n = 0usize;

        for r in 0..sr.count as usize {
            let pi = sr.piece_indices[r] as usize;
            if pi >= lines.piece_count {
                continue;
            }
            let pl = &lines.pieces[pi];
            if pl.square == target.square || !reaches_directly(pl, target.square) {
                continue;
            }
            let v = pl.piece_type.eval_value();
            let p = pl.player.index();
            if p == owner {
                if def_n < MAX_REACHERS_PER_SQUARE {
                    def[def_n] = v;
                    def_n += 1;
                }
            } else if atk_n[p] < MAX_REACHERS_PER_SQUARE {
                atk[p][atk_n[p]] = v;
                atk_n[p] += 1;
            }
        }

        sort_asc(&mut def[..def_n]);
        for p in 0..4 {
            if p == owner || atk_n[p] == 0 {
                continue;
            }
            sort_asc(&mut atk[p][..atk_n[p]]);
            let see = see_swap(target_val, &atk[p][..atk_n[p]], &def[..def_n]);
            if see > 0 {
                t[p] = t[p].saturating_add(see / THREAT_DISCOUNT);
            }
        }
    }
    t
}

/// Turn-proximity-weighted SEE threats: attack the next-to-move player = more valuable.
///
/// In 4PC, a threat from the player who moves next is more dangerous than a threat from
/// a player 2-3 turns away, because the target gets fewer chances to escape.
///
/// Weight: next = 1.0x, 2-away = 0.6x, 3-away = 0.3x.
/// This is the standalone Phase E prototype — test even if L3 selective didn't pass.
pub fn query_threats_see_proximity(lines: &LineMap, side_to_move: Player) -> [i16; 4] {
    let base = query_threats_see(lines);
    let mut t = [0i16; 4];
    for (p, &player) in Player::ALL.iter().enumerate() {
        let proximity = turn_proximity_weight(side_to_move, player);
        // proximity is 10, 6, or 3 — scale to multiplier
        t[p] =
            (base[p] as i32 * proximity as i32 / 10).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    t
}

/// SEE for `attacker_player` capturing the piece on `target_sq` (value `target_value`, owned by
/// `owner`): the attacker's direct attackers vs the owner's direct defenders, swapped off. Positive
/// = the capture wins material. Best-case for the attacking side (uses the least-valuable attacker).
pub fn see_capture(
    lines: &LineMap,
    target_sq: Square,
    target_value: i16,
    owner: Player,
    attacker_player: Player,
) -> i16 {
    let sr = lines.reachers_at(target_sq);
    let mut atk = [0i16; MAX_REACHERS_PER_SQUARE];
    let mut atk_n = 0usize;
    let mut def = [0i16; MAX_REACHERS_PER_SQUARE];
    let mut def_n = 0usize;
    for r in 0..sr.count as usize {
        let pi = sr.piece_indices[r] as usize;
        if pi >= lines.piece_count {
            continue;
        }
        let pl = &lines.pieces[pi];
        if pl.square == target_sq || !reaches_directly(pl, target_sq) {
            continue;
        }
        let v = pl.piece_type.eval_value();
        if pl.player == attacker_player {
            if atk_n < MAX_REACHERS_PER_SQUARE {
                atk[atk_n] = v;
                atk_n += 1;
            }
        } else if pl.player == owner && def_n < MAX_REACHERS_PER_SQUARE {
            def[def_n] = v;
            def_n += 1;
        }
    }
    sort_asc(&mut atk[..atk_n]);
    sort_asc(&mut def[..def_n]);
    see_swap(target_value, &atk[..atk_n], &def[..def_n])
}

// ---------------------------------------------------------------------------
// King Safety Query (§4.4)
// ---------------------------------------------------------------------------

/// Radius-1 vicinity of a square: the 8 adjacent squares that are valid.
fn vicinity(sq: Square) -> impl Iterator<Item = Square> {
    KING_DELTAS
        .iter()
        .filter_map(move |&(dr, df)| offset(sq, dr, df).filter(|s| s.is_valid()))
}

/// Radius-2 knight-jump squares around a square.
fn knight_vicinity(sq: Square) -> impl Iterator<Item = Square> {
    KNIGHT_DELTAS
        .iter()
        .filter_map(move |&(dr, df)| offset(sq, dr, df).filter(|s| s.is_valid()))
}

/// For a given reachers record at a square, count how many are friendly vs enemy
/// and sum enemy piece values.
fn classify_reachers(sr: &SquareReachers, lines: &LineMap, player: Player) -> (u8, u8, i16) {
    let mut defenders = 0u8;
    let mut attackers = 0u8;
    let mut attack_value = 0i16;
    for i in 0..sr.count {
        let pi = sr.piece_indices[i as usize] as usize;
        let pl = &lines.pieces[pi];
        if pl.player == player {
            defenders += 1;
        } else {
            attackers += 1;
            attack_value += pl.piece_type.eval_value();
        }
    }
    (defenders, attackers, attack_value)
}

/// King safety for all four players.
pub fn query_king_safety(lines: &LineMap, board: &Board) -> [KingSafety; 4] {
    let mut ks = [
        KingSafety::default(),
        KingSafety::default(),
        KingSafety::default(),
        KingSafety::default(),
    ];

    for player in Player::ALL {
        let pi = player.index();
        let Some(king_sq) = board.king_square(player) else {
            continue; // king already captured
        };

        // Radius-1 vicinity
        for adj in vicinity(king_sq) {
            let sr = lines.reachers_at(adj);
            let (def, att, att_val) = classify_reachers(sr, lines, player);
            ks[pi].defenders += def;
            ks[pi].attackers += att;
            ks[pi].attack_value += att_val;

            // Escape square: empty and not enemy-attacked
            if board.piece_at(adj).is_none() && att == 0 {
                ks[pi].escape_squares += 1;
            }
        }

        // Radius-2 knight threats
        for jump in knight_vicinity(king_sq) {
            let sr = lines.reachers_at(jump);
            for i in 0..sr.count {
                let pi2 = sr.piece_indices[i as usize] as usize;
                let pl = &lines.pieces[pi2];
                if pl.player != player && pl.piece_type == PieceType::Knight {
                    ks[pi].attackers += 1;
                    ks[pi].attack_value += PieceType::Knight.eval_value();
                }
            }
        }
    }

    ks
}

/// Collapse KingSafety into a scalar: centipawn-scale safety score.
///
/// v0 was `defenders − attackers + escapes` (single-digit scale, invisible next to material).
/// Recalibrated: `attack_value` (sum of attacker piece values, already computed) is folded in,
/// scaled by defender coverage, with escapes as a bonus. Puts safety on the centipawn scale
/// (hundreds) so a king under heavy attack costs a meaningful fraction of a piece.
pub fn safety_scalar(ks: &KingSafety) -> i16 {
    // Attack danger: sum of attacker values, mitigated by defenders.
    // Each defender absorbs ~150 cp of attack; danger is capped so extreme cases
    // don't blow up the vector.
    let attack_danger = (ks.attack_value as i32 * 10)
        .saturating_div((ks.defenders as i32 * 15 + 10).max(1))
        .clamp(0, 600) as i16;

    // Defense bonus: each defender contributes directly
    let defense_bonus = (ks.defenders as i16).saturating_mul(40);

    // Escape bonus: each safe escape square is worth ~25 cp
    let escape_bonus = (ks.escape_squares as i16).saturating_mul(25);

    defense_bonus + escape_bonus - attack_danger
}

/// Pure king-**danger** for the search-side objective layer (the points-aware safety rebuild).
///
/// `0` = safe, positive = the king is under attack. UNLIKE [`safety_scalar`], there is **no
/// standalone defender/escape bonus** — that bonus rewarded huddling pieces around the king, which in
/// 4PC is passive, undeveloped play and correlates with *losing* (it's why Texel had to give the whole
/// safety term a negative weight; see EXP-018). Here defenders and escapes only **mitigate** the
/// incoming attack. The search subtracts `weight × this` so a threatened king is valued as the
/// points-risk of elimination, not a cp huddle reward.
pub fn king_danger_scalar(ks: &KingSafety) -> i16 {
    (ks.attack_value as i32 * 10)
        .saturating_div((ks.defenders as i32 * 15 + ks.escape_squares as i32 * 10 + 10).max(1))
        .clamp(0, 600) as i16
}

/// King-danger via a **non-linear attack-units table** (Kimi's independent design, Glaurung-style):
/// multiple attackers compound super-linearly. `attack_units ≈ attack_value / 150` (pawn 100, knight
/// 300, …); the table maps units → danger. Clamped to 600 to match [`king_danger_scalar`]'s range so an
/// A/B isolates the *shape* (compounding) rather than raw magnitude. The contrast with the linear
/// scalar: this is pure incoming attack (no defender term at all); the scalar mitigates by defenders.
pub fn king_danger_table_scalar(ks: &KingSafety) -> i16 {
    const TABLE: [i16; 21] = [
        0, 5, 15, 30, 50, 75, 105, 140, 180, 225, 275, 330, 390, 455, 525, 600, 680, 765, 855, 950,
        1000,
    ];
    let units = (ks.attack_value as i32 / 150).clamp(0, 20) as usize;
    TABLE[units].min(600)
}

/// Elimination-proximity (Kimi's independent win-term design): how close each player is to being
/// eliminated — **low material AND an attacked king** (multiplicative, so both must hold). `0` = healthy,
/// ~`100` = collapsing. Points-blind (material + king-danger, never FFA points), so it keeps Hard Rule
/// #8 while still being win-aware. The search-side win term then uses `win_i = Σ_{j≠i} prox_j − 3·prox_i`
/// ("I want opponents weak and myself not"), which is mean-relative (Σ=0).
pub fn elimination_proximity(material: &[i16; 4], ks: &[KingSafety; 4]) -> [i16; 4] {
    let mut prox = [0i16; 4];
    for i in 0..4 {
        let mat_weak = (2500 - material[i] as i32).clamp(0, 1500) / 15; // 0..100
        let danger = king_danger_scalar(&ks[i]).min(600) as i32 / 6; // 0..100
        prox[i] = (mat_weak * danger / 100).min(100) as i16;
    }
    prox
}

// ---------------------------------------------------------------------------
// Crossfire Query (§4.5)
// ---------------------------------------------------------------------------

/// For each player's pieces, penalise material actually at risk from enemy attacks.
/// Replaces the old `enemy_value * enemy_count` (dimensionally wrong, scale-explosive) with
/// SEE-resolved exchange value: the net centipawns the owner would lose if enemies capture.
/// Per-attacking-player 2-sided SEE; third parties don't enter (4PC: no one recaptures to save
/// another player's piece). Sum of positive SEE threats, bounded by the victim's value.
pub fn query_crossfire(lines: &LineMap) -> [i16; 4] {
    let mut o = [0i16; 4];

    for ti in 0..lines.piece_count {
        let target = &lines.pieces[ti];
        let target_val = target.piece_type.eval_value();
        if target_val == 0 {
            continue; // king: terminal capture, search handles it
        }
        let owner = target.player.index();
        let sr = lines.reachers_at(target.square);

        let mut atk = [[0i16; MAX_REACHERS_PER_SQUARE]; 4];
        let mut atk_n = [0usize; 4];
        let mut def = [0i16; MAX_REACHERS_PER_SQUARE];
        let mut def_n = 0usize;

        for r in 0..sr.count as usize {
            let pi = sr.piece_indices[r] as usize;
            if pi >= lines.piece_count {
                continue;
            }
            let pl = &lines.pieces[pi];
            if pl.square == target.square || !reaches_directly(pl, target.square) {
                continue;
            }
            let v = pl.piece_type.eval_value();
            let p = pl.player.index();
            if p == owner {
                if def_n < MAX_REACHERS_PER_SQUARE {
                    def[def_n] = v;
                    def_n += 1;
                }
            } else if atk_n[p] < MAX_REACHERS_PER_SQUARE {
                atk[p][atk_n[p]] = v;
                atk_n[p] += 1;
            }
        }

        sort_asc(&mut def[..def_n]);
        let mut total_risk = 0i16;
        for p in 0..4 {
            if p == owner || atk_n[p] == 0 {
                continue;
            }
            sort_asc(&mut atk[p][..atk_n[p]]);
            let see = see_swap(target_val, &atk[p][..atk_n[p]], &def[..def_n]);
            if see > 0 {
                total_risk = total_risk.saturating_add(see);
            }
        }
        // Bound risk by victim value (can't lose more than the piece is worth)
        let penalty = total_risk.min(target_val);
        o[owner] = o[owner].saturating_add(penalty);
    }

    o
}

// ---------------------------------------------------------------------------
// Pawn Structure Query (§4.3 sub-readout — feeds into Pᵢ)
// ---------------------------------------------------------------------------

/// Per-player pawn-structure penalty: isolated + doubled pawns.
///
/// **Lane geometry is player-parameterized** (perpendicular to forward direction):
/// - Red/Yellow (forward ±rank): lane = **file**
/// - Blue/Green (forward ±file): lane = **rank**
///
/// - **Doubled** = ≥2 friendly pawns on the same lane (stacked on advance lane).
/// - **Isolated** = no friendly pawn on lane−1 or lane+1.
///
/// Penalty is in centipawns, bounded and small — this is a positional nudge, not a
/// tactical signal. Texel-gated: must drop MSE to be kept.
pub fn query_pawn_structure(board: &Board) -> [i16; 4] {
    let mut penalty = [0i16; 4];

    // Collect pawn lanes per player
    // lanes[p][i] = count of player p's pawns on lane i
    // For Red/Yellow: lane = file (0..13)
    // For Blue/Green: lane = rank (0..13)
    let mut red_lanes = [0u8; 14];
    let mut blue_lanes = [0u8; 14];
    let mut yellow_lanes = [0u8; 14];
    let mut green_lanes = [0u8; 14];

    for i in 0..crate::board::types::TOTAL_SQUARES {
        let sq = Square::new(i as u8);
        if let Some(p) = board.piece_at(sq) {
            if p.piece_type == PieceType::Pawn {
                match p.player {
                    Player::Red => red_lanes[sq.file() as usize] += 1,
                    Player::Blue => blue_lanes[sq.rank() as usize] += 1,
                    Player::Yellow => yellow_lanes[sq.file() as usize] += 1,
                    Player::Green => green_lanes[sq.rank() as usize] += 1,
                }
            }
        }
    }

    // Score each player's pawn structure
    for (player, lanes) in [
        (Player::Red, &red_lanes),
        (Player::Blue, &blue_lanes),
        (Player::Yellow, &yellow_lanes),
        (Player::Green, &green_lanes),
    ] {
        let pi = player.index();
        let mut isolated_count = 0u8;
        let mut doubled_count = 0u8;

        for lane in 0..14 {
            if lanes[lane] == 0 {
                continue;
            }
            // Doubled: >1 pawn on same lane
            if lanes[lane] >= 2 {
                doubled_count += lanes[lane] - 1;
            }
            // Isolated: no pawn on adjacent lane
            let left = lane.saturating_sub(1);
            let right = (lane + 1).min(13);
            let has_neighbor = if left == lane {
                lanes[right] > 0
            } else if right == lane {
                lanes[left] > 0
            } else {
                lanes[left] > 0 || lanes[right] > 0
            };
            if !has_neighbor {
                isolated_count += 1;
            }
        }

        // Penalty: 20 cp per isolated pawn, 15 cp per doubled pawn (beyond the first)
        // These are small positional nudges — the exact values are Texel-tuned later.
        let p =
            (isolated_count as i16).saturating_mul(20) + (doubled_count as i16).saturating_mul(15);
        penalty[pi] = p;
    }

    penalty
}

// ---------------------------------------------------------------------------
// C3 / EXP-025: Unbundled pawn-structure queries (isolated / doubled / connected)
// ---------------------------------------------------------------------------
// Each returns a raw COUNT per player (not pre-scaled cp). The tuner assigns
// independent weights. Hard Rule #4: all fold into P (positional).

/// Per-player pawn counts by **lane** — the file for Red/Yellow (who advance along ranks) and
/// the rank for Blue/Green (who advance along files), so "adjacent lane" means the same thing
/// for every player's structure. Shared prologue of the three pawn queries; also used by the
/// EXP-032 rook-open candidate eval.
pub(crate) fn pawn_lanes(board: &Board) -> [[u8; 14]; 4] {
    let mut lanes: [[u8; 14]; 4] = [[0; 14]; 4];
    for i in 0..crate::board::types::TOTAL_SQUARES {
        let sq = Square::new(i as u8);
        if let Some(p) = board.piece_at(sq)
            && p.piece_type == PieceType::Pawn
        {
            let lane = if p.player == Player::Red || p.player == Player::Yellow {
                sq.file() as usize
            } else {
                sq.rank() as usize
            };
            lanes[p.player.index()][lane] += 1;
        }
    }
    lanes
}

/// True if `lanes` has any pawn on a lane adjacent to `lane`.
fn lane_has_neighbor(lanes: &[u8; 14], lane: usize) -> bool {
    (lane > 0 && lanes[lane - 1] > 0) || (lane < 13 && lanes[lane + 1] > 0)
}

/// Count isolated pawns per player: a pawn with no friendly pawn on an adjacent lane.
/// Returns unit counts (0..N); the consumer scales them (EXP-025: fitted in texel_tune).
pub fn query_pawn_isolated(board: &Board) -> [i16; 4] {
    let lanes = pawn_lanes(board);
    let mut iso = [0i16; 4];
    for pi in 0..4 {
        for lane in 0..14 {
            if lanes[pi][lane] > 0 && !lane_has_neighbor(&lanes[pi], lane) {
                iso[pi] += 1;
            }
        }
    }
    iso
}

/// Count doubled pawns per player: extra pawns beyond the first per lane.
/// Returns unit counts (0..N); the consumer scales them (EXP-025: fitted in texel_tune).
pub fn query_pawn_doubled(board: &Board) -> [i16; 4] {
    let lanes = pawn_lanes(board);
    let mut dbl = [0i16; 4];
    for pi in 0..4 {
        for lane in 0..14 {
            if lanes[pi][lane] >= 2 {
                dbl[pi] += (lanes[pi][lane] - 1) as i16;
            }
        }
    }
    dbl
}

/// Count connected pawns per player: pawns with a friendly pawn on an adjacent lane.
/// Returns unit counts (0..N); the consumer scales them (EXP-025: fitted in texel_tune).
pub fn query_pawn_connected(board: &Board) -> [i16; 4] {
    let lanes = pawn_lanes(board);
    let mut conn = [0i16; 4];
    for pi in 0..4 {
        for lane in 0..14 {
            if lanes[pi][lane] > 0 && lane_has_neighbor(&lanes[pi], lane) {
                // Each pawn on this lane is connected (has support on an adjacent lane).
                conn[pi] += lanes[pi][lane] as i16;
            }
        }
    }
    conn
}

// ---------------------------------------------------------------------------
// L3 Selective Intent Query (§4.8 selective — offense only, feeds into Pᵢ)
// ---------------------------------------------------------------------------

/// Turn-order proximity: how soon can this threatener act?
/// Weight: 1.0 = next to move, 0.6 = 2 turns away, 0.3 = 3 turns away.
fn turn_proximity_weight(owner: Player, threatener: Player) -> i16 {
    let o = owner.index() as i8;
    let t = threatener.index() as i8;
    let dist = ((t - o + 4) % 4) as i16;
    match dist {
        1 => 10, // next: 1.0x
        2 => 6,  // 2 away: 0.6x
        3 => 3,  // 3 away: 0.3x
        _ => 5,  // self (shouldn't happen): 0.5x
    }
}

/// Selective offense intent: for "contested" pieces (attacking ≥2 enemy targets,
/// or attacking a target that is also attacked by ≥1 friendly piece), compute
/// per-opponent offense with turn-proximity weighting.
///
/// This gives the eval the coordination signal: "I attack the next-to-move player's
/// queen with my knight AND my bishop" is more valuable than either attack alone.
pub fn query_selective_intent(lines: &LineMap, board: &Board) -> [i16; 4] {
    let mut intent = [0i16; 4];

    // Track which (attacker, target) pairs are "contested" — the target is attacked
    // by multiple friendlies, or the attacker attacks multiple enemies.
    // We use a simple heuristic: for each piece, if it attacks ≥2 enemies OR
    // attacks a target that has ≥2 friendly attackers, it's contested.

    // First pass: count friendly attackers per target square
    let mut friendly_attackers_on_target: [u8; 196] = [0; 196];
    for pl in lines.pieces[..lines.piece_count].iter() {
        for e in pl.entries() {
            if let Some(target) = e.first_occupant {
                if target.player != pl.player {
                    let sq_idx = e.square.index() as usize;
                    if sq_idx < 196 {
                        friendly_attackers_on_target[sq_idx] += 1;
                    }
                }
            }
        }
    }

    // Second pass: for each contested attacker, compute weighted offense
    for pl in lines.pieces[..lines.piece_count].iter() {
        let attacker_player = pl.player;
        let attacker_val = pl.piece_type.eval_value();
        let mut contested_targets = 0u8;
        let mut total_offense = 0i16;

        for e in pl.entries() {
            if let Some(target) = e.first_occupant {
                if target.player == attacker_player {
                    continue; // can't attack own piece
                }
                let sq_idx = e.square.index() as usize;
                let multi_attacker = sq_idx < 196 && friendly_attackers_on_target[sq_idx] >= 2;
                if multi_attacker {
                    contested_targets += 1;
                    let target_val = target.piece_type.eval_value();
                    let proximity = turn_proximity_weight(board.side_to_move, attacker_player);
                    // Offense score: target value × proximity × attacker quality bonus
                    let quality_bonus = if attacker_val <= target_val { 12 } else { 8 };
                    let score = (target_val as i32 * proximity as i32 * quality_bonus as i32 / 100)
                        .clamp(i16::MIN as i32, i16::MAX as i32)
                        as i16;
                    total_offense = total_offense.saturating_add(score);
                }
            }
        }

        // Coordination bonus: if this piece attacks multiple contested targets, multiply
        if contested_targets >= 2 {
            total_offense =
                (total_offense as i32 * 15 / 10).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }

        intent[attacker_player.index()] =
            intent[attacker_player.index()].saturating_add(total_offense);
    }

    intent
}

// ---------------------------------------------------------------------------
// Master Query (§4.6)
// ---------------------------------------------------------------------------

/// Run all queries — the **full** vector, every component computed regardless of the deployed
/// eval weights. `texel_tune` and any weight-exploration tooling depend on this; the evaluator's
/// hot path uses [`run_queries_gated`] instead (C1 / EXP-022).
pub fn run_all_queries(lines: &LineMap, board: &Board) -> QueryVector {
    run_queries_gated(lines, board, true, true)
}

/// Run the queries the caller actually consumes (C1 / EXP-022 — pure perf, output-identical).
/// A component gated off returns zeros; under the eval's mean-relative × weight combination a
/// zero-weight component contributes exactly 0 either way, so skipping it cannot change the eval
/// output — it only skips the work (positional control + threats + PST, and the king-safety
/// scan, at every leaf). The search-side king-danger term is unaffected: it calls
/// [`query_king_safety`] directly, never through this function.
///
pub fn run_queries_gated(
    lines: &LineMap,
    board: &Board,
    need_positional: bool,
    need_safety: bool,
) -> QueryVector {
    let material = query_material(board);
    let positional = if need_positional {
        let control = query_positional_control(lines);
        // EXP-002: exchange-aware (SEE) threats when HORNET_SEE=1, else the flat target-value term.
        let threats = if *SEE_THREATS {
            query_threats_see(lines)
        } else {
            query_threats(lines)
        };
        // Positional = empty-square control + tactical threats (both reward activity)
        [
            control[0] + threats[0],
            control[1] + threats[1],
            control[2] + threats[2],
            control[3] + threats[3],
        ]
    } else {
        [0; 4]
    };
    let safety = if need_safety {
        let safety_raw = query_king_safety(lines, board);
        [
            safety_scalar(&safety_raw[0]),
            safety_scalar(&safety_raw[1]),
            safety_scalar(&safety_raw[2]),
            safety_scalar(&safety_raw[3]),
        ]
    } else {
        [0; 4]
    };
    let crossfire = query_crossfire(lines);
    // Pawn structure ablated — Texel MSE drop was marginal (0.11445→0.11441, ~0.03%).
    // Code kept in query_pawn_structure for re-test with better features, but not wired.

    // L2 zone intent ablated — Texel MSE drop was marginal (0.11443→0.11439, ~0.03%).
    // aggregate_zone_control exists in zones.rs for re-test, but not wired.

    // L3 selective offense intent ablated — Texel MSE didn't improve.
    // With selective: baseline 0.11556, tuned 0.11450. Without: baseline 0.11453, tuned 0.11452.
    // The feature adds noise; tuning barely compensates. Not wired into eval.

    // Piece-square tables: v3 zone-aware per-piece (rook edge bonus dropped). Part of the
    // positional component, so gated with it.
    let positional = if need_positional {
        let pst = query_pst(board);
        [
            positional[0] + pst[0],
            positional[1] + pst[1],
            positional[2] + pst[2],
            positional[3] + pst[3],
        ]
    } else {
        positional
    };

    QueryVector {
        material,
        positional,
        safety,
        crossfire,
    }
}

// ---------------------------------------------------------------------------
// Tests (§7.4)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;

    fn start() -> Board {
        fen4::parse(fen4::START_FEN4).unwrap()
    }

    fn lines_for(board: &Board) -> Box<LineMap> {
        let mut lm = Box::new(LineMap::new());
        crate::lines::compute_lines(board, &mut lm);
        lm
    }

    #[test]
    fn material_starting_position() {
        let b = start();
        let m = query_material(&b);
        // 8×100 + 2×300 + 2×450 + 2×500 + 900 = 800 + 600 + 900 + 1000 + 900 = 4200
        assert_eq!(m, [4200, 4200, 4200, 4200]);
    }

    #[test]
    fn material_after_capture() {
        let mut b = start();
        // Remove Blue queen (a8 = file 0, rank 7)
        b.set_piece(Square::from_rank_file(7, 0), None);
        let m = query_material(&b);
        assert_eq!(m[0], 4200); // Red unchanged
        assert_eq!(m[1], 3300); // Blue lost 900
        assert_eq!(m[2], 4200); // Yellow unchanged
        assert_eq!(m[3], 4200); // Green unchanged
    }

    #[test]
    fn positional_control_symmetric() {
        let b = start();
        let lm = lines_for(&b);
        let p = query_positional_control(&lm);
        // All four players should have similar control in the starting position.
        let avg = (p[0] + p[1] + p[2] + p[3]) as f32 / 4.0;
        for (i, &pos) in p.iter().enumerate() {
            let diff = (pos as f32 - avg).abs();
            assert!(
                diff / avg < 0.25,
                "player {i} positional control {v} deviates >25% from avg {avg}",
                v = pos
            );
        }
    }

    #[test]
    fn king_safety_defenders_at_start() {
        let b = start();
        let lm = lines_for(&b);
        let ks = query_king_safety(&lm, &b);
        for (i, k) in ks.iter().enumerate() {
            assert!(k.defenders > 0, "player {i} king has no defenders at start");
        }
    }

    #[test]
    fn crossfire_empty_board() {
        let b = Board::empty();
        let lm = lines_for(&b);
        let o = query_crossfire(&lm);
        assert_eq!(o, [0, 0, 0, 0]);
    }

    #[test]
    fn crossfire_with_convergence() {
        // Place 2 enemy rooks attacking the same friendly knight.
        // Red knight at g7, Blue rook at g1 (same file), Yellow rook at a7 (same rank).
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("g7").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("g1").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Rook)),
        );
        b.set_piece(
            Square::from_algebraic("a7").unwrap(),
            Some(Piece::new(Player::Yellow, PieceType::Rook)),
        );

        let lm = lines_for(&b);
        let o = query_crossfire(&lm);

        // Red's knight is attacked by 2 enemies → Red gets a crossfire penalty
        assert!(
            o[0] > 0,
            "Red should have crossfire penalty (knight attacked by 2 rooks)"
        );
        // Blue and Yellow each have only 1 enemy attacking their pieces → no crossfire
        assert_eq!(o[1], 0, "Blue has no crossfire");
        assert_eq!(o[2], 0, "Yellow has no crossfire");
    }

    #[test]
    fn run_all_queries_produces_query_vector() {
        let b = start();
        let lm = lines_for(&b);
        let qv = run_all_queries(&lm, &b);

        assert_eq!(qv.material, [4200, 4200, 4200, 4200]);
        // Positional can be negative at start (zone control: pieces are far from zones).
        // The test is that the QueryVector is well-formed and symmetric.
        for (i, &pos) in qv.positional.iter().enumerate() {
            assert_eq!(
                pos, qv.positional[0],
                "player {i} positional {pos} should match player 0 {} at start",
                qv.positional[0]
            );
        }
    }

    // --- EXP-002: exchange-aware (SEE) threats ---

    #[test]
    fn see_swap_resolves_exchanges() {
        // Hanging queen: take it free.
        assert_eq!(see_swap(900, &[100], &[]), 900);
        // Pawn takes a DEFENDED queen: +800 (win queen, lose pawn to the recapture). "Defended"
        // does not negate the threat when the attacker is cheaper than the target.
        assert_eq!(see_swap(900, &[100], &[500]), 800);
        // Queen takes a DEFENDED pawn: −800 (LVA ≥ target) — not a threat.
        assert_eq!(see_swap(100, &[900], &[100]), -800);
        // No attacker → nothing.
        assert_eq!(see_swap(500, &[], &[100]), 0);
    }

    #[test]
    fn see_threats_credit_only_winning_captures() {
        let at = |s: &str| Square::from_algebraic(s).unwrap();

        // Red knight b5 attacks an undefended Blue queen a7 (no mutual attack) → Red threat > 0.
        let mut b = Board::empty();
        b.set_piece(at("b5"), Some(Piece::new(Player::Red, PieceType::Knight)));
        b.set_piece(at("a7"), Some(Piece::new(Player::Blue, PieceType::Queen)));
        let lm = lines_for(&b);
        let t = query_threats_see(&lm);
        assert!(
            t[Player::Red.index()] > 0,
            "Red threatens the hanging queen"
        );
        assert_eq!(t[Player::Blue.index()], 0, "Blue threatens nothing");

        // Red queen a1 'attacks' a Blue pawn a7 defended by a Blue knight c8 → not winnable → 0.
        let mut b2 = Board::empty();
        b2.set_piece(at("a1"), Some(Piece::new(Player::Red, PieceType::Queen)));
        b2.set_piece(at("a7"), Some(Piece::new(Player::Blue, PieceType::Pawn)));
        b2.set_piece(at("c8"), Some(Piece::new(Player::Blue, PieceType::Knight)));
        let lm2 = lines_for(&b2);
        let t2 = query_threats_see(&lm2);
        assert_eq!(
            t2[Player::Red.index()],
            0,
            "queen can't profitably take the defended pawn"
        );
    }

    // --- Pawn structure tests ---

    #[test]
    fn pawn_structure_starting_position_no_penalty() {
        let b = start();
        let p = query_pawn_structure(&b);
        // Starting position: pawns are not isolated or doubled
        for (i, &pen) in p.iter().enumerate() {
            assert_eq!(
                pen, 0,
                "player {i} should have no pawn structure penalty at start"
            );
        }
    }

    #[test]
    fn pawn_structure_isolated_pawn() {
        // Red pawn at d4, no other Red pawns on c-file or e-file
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("d4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        let p = query_pawn_structure(&b);
        assert_eq!(p[0], 20, "isolated Red pawn = 20 cp penalty");
        assert_eq!(p[1], 0, "Blue has no pawns");
    }

    #[test]
    fn pawn_structure_doubled_pawns() {
        // Two Red pawns on d-file: d4 and d5, plus neighbors on c and e to avoid isolation
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("c4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("d4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("d5").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("e4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        let p = query_pawn_structure(&b);
        // d-file has 2 pawns (1 extra = 15), c/d/e are not isolated (have neighbors)
        assert_eq!(p[0], 15, "doubled Red pawns = 15 cp penalty (1 extra)");
    }

    #[test]
    fn pawn_structure_isolated_and_doubled() {
        // Three Red pawns on d-file: d4, d5, d6 — isolated and 2 extra = 20 + 30 = 50
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("d4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("d5").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("d6").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        let p = query_pawn_structure(&b);
        // Isolated (20) + doubled×2 (30) = 50
        assert_eq!(p[0], 50, "isolated + 2 extra = 50 cp penalty");
    }
}
