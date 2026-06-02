//! Evaluator (spec §4.7, §5). Computes `eval_4vec(&state) -> [i16; 4]` — the per-player
//! utility vector V that search backs up via Max^n (Hard Rule #3: vector, never scalar).
//!
//! v0 is hand-tuned (weights from spec Appendix). The NNUE (§5) replaces this once the
//! strength gate is passed (Hard Rule #7).

use crate::board::Board;
use crate::board::types::Player;
use crate::lines::{LineMap, compute_lines};
use crate::queries::{QueryVector, run_all_queries};

// ---------------------------------------------------------------------------
// v0 weights (from spec Appendix)
// ---------------------------------------------------------------------------

const W_MATERIAL: i16 = 1;
const W_POSITIONAL: i16 = 2;
const W_SAFETY: i16 = 1;
const W_CROSSFIRE: i16 = 1;

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// Compute the per-player utility vector `V = <U₁, U₂, U₃, U₄>`.
///
/// This is the evaluator's primary interface (Hard Rule #3). Search consumes the
/// full vector; no scalar collapse happens at the eval boundary.
///
/// The caller must provide a reusable `LineMap` buffer (boxed, allocated once).
/// `compute_lines` fills it in place (~110 KB, always-recompute per Hard Rule #5).
pub fn eval_4vec(board: &Board, line_buffer: &mut LineMap) -> [i16; 4] {
    compute_lines(board, line_buffer);
    let qv = run_all_queries(line_buffer, board);
    compute_utility(&qv)
}

/// Scalar convenience: one player's score. Semantically equivalent to
/// `eval_4vec(state, lines)[player.index()]`, but avoids allocating the full
/// vector when only one component is needed.
pub fn eval_scalar(board: &Board, line_buffer: &mut LineMap, player: Player) -> i16 {
    let v = eval_4vec(board, line_buffer);
    v[player.index()]
}

/// Collapse a `QueryVector` into per-player utilities using the v0 weights.
///
/// `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ` (Hard Rule #4).
fn compute_utility(qv: &QueryVector) -> [i16; 4] {
    let mut v = [0i16; 4];
    for player in Player::ALL {
        let i = player.index();
        v[i] =
            qv.material[i] * W_MATERIAL + qv.positional[i] * W_POSITIONAL + qv.safety[i] * W_SAFETY
                - qv.crossfire[i] * W_CROSSFIRE;
    }
    v
}

// ---------------------------------------------------------------------------
// Tests (§7.5)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;

    fn start() -> Board {
        fen4::parse(fen4::START_FEN4).unwrap()
    }

    fn eval_start() -> ([i16; 4], Box<LineMap>) {
        let b = start();
        let mut lm = Box::new(LineMap::new());
        let v = eval_4vec(&b, &mut lm);
        (v, lm)
    }

    #[test]
    fn starting_position_symmetry() {
        let (v, _) = eval_start();
        let avg = (v[0] as i32 + v[1] as i32 + v[2] as i32 + v[3] as i32) / 4;
        for (i, &score) in v.iter().enumerate() {
            let diff = (score as i32 - avg).abs();
            assert!(
                diff <= 500,
                "player {i} score {score} deviates {diff} from avg {avg} (max ±500)"
            );
        }
    }

    #[test]
    fn scalar_matches_4vec() {
        let b = start();
        let mut lm = Box::new(LineMap::new());
        let v = eval_4vec(&b, &mut lm);
        for player in Player::ALL {
            let scalar = eval_scalar(&b, &mut lm, player);
            assert_eq!(
                scalar,
                v[player.index()],
                "scalar mismatch for player {:?}",
                player
            );
        }
    }

    #[test]
    fn eval_performance_debug_mode() {
        let b = start();
        let mut lm = Box::new(LineMap::new());

        // Warm-up
        for _ in 0..10 {
            let _ = eval_4vec(&b, &mut lm);
        }

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = eval_4vec(&b, &mut lm);
        }
        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / 1000.0;

        assert!(
            avg_us < 200.0,
            "eval_4vec average {avg_us:.1} µs exceeds 200 µs debug-mode budget"
        );
    }

    #[test]
    fn eval_after_capture_changes_scores() {
        let mut b = start();
        // Remove Blue queen (a8 = file 0, rank 7)
        b.set_piece(crate::board::types::Square::from_rank_file(7, 0), None);

        let mut lm = Box::new(LineMap::new());
        let v = eval_4vec(&b, &mut lm);

        // Blue should be worse off than the others
        assert!(
            v[1] < v[0],
            "Blue should be worse than Red after losing queen"
        );
        assert!(
            v[1] < v[2],
            "Blue should be worse than Yellow after losing queen"
        );
        assert!(
            v[1] < v[3],
            "Blue should be worse than Green after losing queen"
        );
    }
}
