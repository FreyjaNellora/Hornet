//! Evaluator (spec §4.7, §5). Computes `eval_4vec(&state) -> [i16; 4]` — the per-player
//! utility vector V that search backs up via Max^n (Hard Rule #3: vector, never scalar).
//!
//! v0 is hand-tuned (weights from spec Appendix). The NNUE (§5) replaces this once the
//! strength gate is passed (Hard Rule #7).

use crate::board::Board;
use crate::board::types::Player;
use crate::lines::{LineMap, compute_lines};
use crate::queries::{QueryVector, run_queries_gated};

// ---------------------------------------------------------------------------
// v0 weights (from spec Appendix)
// ---------------------------------------------------------------------------

// Recalibrated weights. Crossfire/safety are now centipawn-bounded (SEE material-at-risk; clamped
// danger), so the heuristic components sit at 1. Material stays high: under the mean-relative
// normalization a free piece only nets ~value/4 to the taker, so material must out-weigh the
// positional swing of repositioning or the engine won't take free material (free-queen test).
// Weights validated by move-agreement tuning (EXP-015).
// Baseline (4,1,1,1): 11.7% move-match. Tuned (6,0,0,1): 13.5% (+1.8pp).
// Positional and safety are net-harmful for move choice as currently built;
// material+crossfire is the validated stopgap. Re-enable P/S when a fixed
// positional component lifts move-agreement above 18.3%.
const W_MATERIAL: i16 = 6;
const W_POSITIONAL: i16 = 0;
const W_SAFETY: i16 = 0;
const W_CROSSFIRE: i16 = 1;

// C2 / EXP-024 note: an eval-side mirror of the search-side objective layer (win-proximity into
// P, king-danger-table into S, gated on their own consts) was drafted here and REMOVED — as
// built it was inert: the terms folded into component *values* that the utility step multiplies
// by W_POSITIONAL = W_SAFETY = 0, so no weight setting could make them reach the output. Tuning
// for these terms lives in `texel_tune` (which computes them independently per position);
// deployment lives in the search-side runtime knobs (`with_win_term` / `with_king_danger`,
// EXP-017/018 — the flashlight play path). Do not re-fold objective terms into zero-weighted
// components.

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
    // C1 / EXP-022: skip the query components the deployed weights zero out (pure perf —
    // a zero-weight component contributes exactly 0 through mean-relative × weight, so the
    // output is identical; pinned by `gated_queries_match_full_eval`). The flags are consts,
    // so the gating itself is compile-time.
    let qv = run_queries_gated(line_buffer, board, W_POSITIONAL != 0, W_SAFETY != 0);
    compute_utility(&qv)
}

/// Scalar convenience: one player's score. Exactly `eval_4vec(state, lines)[player.index()]` —
/// the full vector is still computed (lines + all queries); this only saves the caller the
/// indexing, not any work.
pub fn eval_scalar(board: &Board, line_buffer: &mut LineMap, player: Player) -> i16 {
    let v = eval_4vec(board, line_buffer);
    v[player.index()]
}

/// Collapse a `QueryVector` into per-player utilities using the v0 weights.
///
/// `Uᵢ = w₁·ΔMᵢ + w₂·ΔPᵢ + w₃·ΔSᵢ − w₄·ΔOᵢ` where `ΔXᵢ = Xᵢ − X̄` (deviation from mean).
///
/// **Why relative?** In 4-player FFA, captures remove material from the board — total
/// material is not conserved. Making each component relative to its per-player mean
/// ensures `Σᵢ Uᵢ = 0` (zero-sum), which enables Sturtevant–Korf shallow pruning
/// bounds (`SUM_UB = 0` exactly). This is a post-processing step; the four query
/// components remain independent and inspectable (Hard Rule #4).
///
/// Oᵢ is the crossfire query alone (centipawns). The `ffa_points` bounty term was lifted out of the
/// eval (Hard Rule #8 / §1.7: the evaluator is points-blind; the FFA-hunt preference lives in move
/// ordering, not V).
fn compute_utility(qv: &QueryVector) -> [i16; 4] {
    // Per-component means (i32 to avoid overflow). Mean-relative keeps Σ Uᵢ ≈ 0 (zero-sum) for
    // Sturtevant–Korf shallow-pruning bounds.
    let mean_m = qv.material.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_p = qv.positional.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_s = qv.safety.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_o = qv.crossfire.iter().map(|&x| x as i32).sum::<i32>() / 4;

    let mut v = [0i16; 4];
    for player in Player::ALL {
        let i = player.index();
        let dm = qv.material[i] as i32 - mean_m;
        let dp = qv.positional[i] as i32 - mean_p;
        let ds = qv.safety[i] as i32 - mean_s;
        let do_ = qv.crossfire[i] as i32 - mean_o;

        let score = dm * W_MATERIAL as i32 + dp * W_POSITIONAL as i32 + ds * W_SAFETY as i32
            - do_ * W_CROSSFIRE as i32;

        v[i] = score.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    v
}

// ---------------------------------------------------------------------------
// EXP-029: candidate evals (experiment-only — the deployed eval is `eval_4vec`)
// ---------------------------------------------------------------------------
// Each candidate = the deployed eval + exactly ONE term at its human-only fitted scale
// (EXP-024/025 addenda, fitted on the deployed (6,0,0,1) base over 140 human games), so the
// paired-gate verdict attributes cleanly. Injected via `Searcher::with_eval`; never the default
// (a candidate becoming the default is a Tier-2 ship gated on EXP-027 paired self-play).
// Both terms are mean-relative (Σ ≈ 0 preserved); values clamped inside mate bounds (±29_000).

/// Isolated-pawn scale (util units; human-only fit: w=3305, MSE drop 0.00326 — EXP-025).
/// Caveat recorded there: predictive weight, possibly symptom-not-cause; this arm tests exactly
/// that by *playing as if* isolation matters.
const PPRIME_ISO_SCALE: i32 = 3305;

/// King-danger-table scale (human-only fit: w=5, MSE drop 0.00094 — EXP-024).
const SPRIME_DGR_SCALE: i32 = 5;

/// P′ candidate: deployed eval − ISO_SCALE·Δ(isolated-pawn count).
pub fn eval_4vec_pprime(board: &Board, line_buffer: &mut LineMap) -> [i16; 4] {
    let mut v = eval_4vec(board, line_buffer);
    let iso = crate::queries::query_pawn_isolated(board);
    let sum: i32 = iso.iter().map(|&x| x as i32).sum();
    for i in 0..4 {
        // Mean-relative in quarter units to avoid truncating small counts: Δ = (4·x − Σ)/4.
        let adj = PPRIME_ISO_SCALE * (4 * iso[i] as i32 - sum) / 4;
        v[i] = (v[i] as i32 - adj).clamp(-29_000, 29_000) as i16;
    }
    v
}

/// S′ candidate: deployed eval − DGR_SCALE·Δ(king-danger table scalar).
pub fn eval_4vec_sprime(board: &Board, line_buffer: &mut LineMap) -> [i16; 4] {
    let mut v = eval_4vec(board, line_buffer); // fills `line_buffer` for the safety scan below
    let ks = crate::queries::query_king_safety(line_buffer, board);
    let dgr: [i32; 4] =
        std::array::from_fn(|i| crate::queries::king_danger_table_scalar(&ks[i]) as i32);
    let sum: i32 = dgr.iter().sum();
    for i in 0..4 {
        let adj = SPRIME_DGR_SCALE * (4 * dgr[i] - sum) / 4;
        v[i] = (v[i] as i32 - adj).clamp(-29_000, 29_000) as i16;
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
        println!("eval_4vec debug-mode average: {avg_us:.1} µs (working budget 600 µs)");

        // The 600 µs working budget (the eval-feature gate, see KIMI-TODO) is machine-dependent in
        // debug mode, so the strict assert is opt-in: HORNET_PERF_ASSERT=1. The always-on bound is
        // a generous catastrophic-regression backstop only.
        if std::env::var("HORNET_PERF_ASSERT").is_ok_and(|v| v == "1") {
            assert!(
                avg_us < 600.0,
                "eval_4vec average {avg_us:.1} µs exceeds the 600 µs debug-mode working budget"
            );
        }
        assert!(
            avg_us < 3000.0,
            "eval_4vec average {avg_us:.1} µs — catastrophic regression (backstop 3000 µs; working budget 600 µs)"
        );
    }

    /// C1 / EXP-022 equality gate: the gated query path (skipping zero-weight components) must
    /// produce the identical eval vector to the full path, across a seeded random-walk position
    /// sweep — not just the start position. If a weight is ever un-zeroed, the gating flags in
    /// `eval_4vec` flip with it (they are `W_* != 0`), so this holds by construction; the test
    /// pins it against refactor drift.
    #[test]
    fn gated_queries_match_full_eval() {
        use crate::move_gen::generate_legal;
        use crate::queries::run_all_queries;

        let mut xs = 0x9E37_79B9_7F4A_7C15u64;
        let mut rng = || {
            xs ^= xs << 13;
            xs ^= xs >> 7;
            xs ^= xs << 17;
            xs
        };

        let mut lm = Box::new(LineMap::new());
        for walk in 0..8 {
            let mut b = start();
            // Walk 0 checks the start position itself; others take 4..32 random plies.
            for _ in 0..(walk * 4) {
                let legal = generate_legal(&mut b);
                if legal.is_empty() {
                    break;
                }
                let mv = legal[rng() as usize % legal.len()];
                b.make_move(mv);
            }
            let gated = eval_4vec(&b, &mut lm);
            compute_lines(&b, &mut lm);
            let full = compute_utility(&run_all_queries(&lm, &b));
            assert_eq!(gated, full, "gated vs full eval diverged on walk {walk}");
        }
    }

    /// Zero-sum invariant: Σᵢ Uᵢ ≈ 0 for any position (off by at most 3 from integer-mean
    /// rounding). This enables Sturtevant–Korf shallow pruning with SUM_UB = 3, which is
    /// vastly tighter than the pre-normalisation bound of −5348.
    #[test]
    fn eval_is_approximately_zero_sum() {
        let b = start();
        let mut lm = Box::new(LineMap::new());
        let v = eval_4vec(&b, &mut lm);
        let sum: i32 = v.iter().map(|&x| x as i32).sum();
        assert!(
            sum.abs() <= 10,
            "eval sum should be within ±10 of zero, got {sum}: {:?}",
            v
        );

        // After a capture
        let mut b2 = start();
        b2.set_piece(crate::board::types::Square::from_rank_file(7, 0), None);
        let v2 = eval_4vec(&b2, &mut lm);
        let sum2: i32 = v2.iter().map(|&x| x as i32).sum();
        assert!(
            sum2.abs() <= 10,
            "eval sum should be within ±10 of zero after capture, got {sum2}: {:?}",
            v2
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

    /// Calibration gate (EXP-006): a single quiet move should swing the mover's static eval by
    /// ~tens of centipawns (positional delta), not thousands. Thousands = the scale bug.
    /// This test asserts the bound on a known quiet move from the starting position.
    #[test]
    fn quiet_move_eval_stability() {
        use crate::board::Move;
        use crate::board::types::Square;
        use crate::move_gen::generate_pseudo_legal;

        let mut b = start();
        let mut lm = Box::new(LineMap::new());

        // Red's quiet move: g2-g3 (pawn push, no capture)
        let quiet_mv = Move {
            from: Square::from_algebraic("g2").unwrap(),
            to: Square::from_algebraic("g3").unwrap(),
            promotion: None,
            flags: crate::board::MoveFlags::default(),
        };

        let before = eval_4vec(&b, &mut lm)[0]; // Red's score
        b.make_move(quiet_mv);
        let after = eval_4vec(&b, &mut lm)[0];
        let swing = (before as i32 - after as i32).abs();

        // After recalibration, quiet moves should swing by ~tens, not thousands.
        // Allow some slack for the hand-tuned v0: bound at 200 cp.
        assert!(
            swing <= 200,
            "quiet move g2-g3 swings Red's eval by {swing} cp (before={before} after={after}); \
             target is ~tens, max 200 during recalibration"
        );
    }
}
