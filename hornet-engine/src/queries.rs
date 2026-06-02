//! Query engine (spec §4). Turns a [`LineMap`] into the four scalar-per-player
//! query outputs: material, positional control, king safety, and crossfire.
//!
//! Hard Rule #4: `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`. Each query traces to exactly
//! one component — no 5th component, no merging.

use crate::board::Piece;
use crate::board::types::{PieceType, Player, Square};
use crate::board::{Board, KING_DELTAS, KNIGHT_DELTAS, offset};
use crate::lines::{LineMap, SquareReachers};

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

/// Collapse KingSafety into a scalar: defenders − attackers + escape_squares.
/// Positive = safer king.
pub fn safety_scalar(ks: &KingSafety) -> i16 {
    ks.defenders as i16 - ks.attackers as i16 + ks.escape_squares as i16
}

// ---------------------------------------------------------------------------
// Crossfire Query (§4.5)
// ---------------------------------------------------------------------------

/// For each player's pieces, penalise squares where multiple enemies converge.
pub fn query_crossfire(lines: &LineMap) -> [i16; 4] {
    let mut o = [0i16; 4];

    for pl in lines.pieces[..lines.piece_count].iter() {
        let pi = pl.player.index();
        let sr = lines.reachers_at(pl.square);

        let mut enemy_count = 0u8;
        let mut enemy_value = 0i16;
        for i in 0..sr.count {
            let pidx = sr.piece_indices[i as usize] as usize;
            let other = &lines.pieces[pidx];
            if other.player != pl.player {
                enemy_count += 1;
                enemy_value += other.piece_type.eval_value();
            }
        }

        if enemy_count >= 2 {
            let penalty = enemy_value * enemy_count as i16 + pl.piece_type.eval_value();
            o[pi] += penalty;
        }
    }

    o
}

// ---------------------------------------------------------------------------
// Master Query (§4.6)
// ---------------------------------------------------------------------------

/// Run all queries. This is the only function the evaluator calls.
pub fn run_all_queries(lines: &LineMap, board: &Board) -> QueryVector {
    let material = query_material(board);
    let positional = query_positional_control(lines);
    let safety_raw = query_king_safety(lines, board);
    let safety = [
        safety_scalar(&safety_raw[0]),
        safety_scalar(&safety_raw[1]),
        safety_scalar(&safety_raw[2]),
        safety_scalar(&safety_raw[3]),
    ];
    let crossfire = query_crossfire(lines);

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
        // Positional, safety, crossfire are non-zero at start
        for (i, &pos) in qv.positional.iter().enumerate() {
            assert!(
                pos > 0,
                "player {i} positional control should be > 0 at start"
            );
        }
    }
}
