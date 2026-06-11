# EXP-013 — self-play bootstrap (growing a game corpus)

- **Date:** 2026-06-07 · **Status:** running (overnight)
- **Goal:** generate a large self-play game corpus, because the eval-feature wall (EXP-009 / Kimi) and
  the search-shape question (EXP-012) both turned out to be **data-bound**: 16 human games / 855
  positions can't detect <0.1% MSE feature deltas (noise floor ≈ ±0.005), and "which search shape
  plays better" needs self-play win-rate (SPSA). More games is the shared prerequisite.

## Tool — `examples/bootstrap.rs`
Plays N full games from **seeded random openings** (12 plies) then **depth-8 laser** (adaptive base 4,
deep floor 1, forward pruning) to a survivor or a 150-ply cap. Records the move stream and writes each
game as PGN4 to `selfplay_games/sp_game_NNNN.pgn4` — the exact format `texel_tune` consumes (auto-loads
`baselines/*.pgn4`; `[Result "R: p - B: p - Y: p - G: p"]` parsed by position; moves replay via
`decode_ply`). Games replay (like the human corpus) up to the first elimination; positions are labelled
by the final placement points.

Run: `cargo run --release --example bootstrap [N]` (this run: N=300).

## Observations (early)
- **~134 s/game** at depth-8 laser (300 games ≈ 11 h → ~200 games by morning; ~13× the human corpus).
- **Games are drawish:** they hit the 150-ply cap with **no eliminations** (e.g. points `[0,17,10,13]`,
  `[2,10,4,10]`). The engine accumulates a few captures but doesn't convert to eliminations in 150
  plies. This is consistent with the eval not valuing eliminations (king-capture ≈ 0 in search; the
  game-flow sweep is play-only) — so self-play is balanced/cautious. **Implication:** outcome labels
  are "points-ahead at the cap", a weaker gradient than "who survived." Future bootstraps should use a
  deeper config (stronger, more decisive) and/or a higher ply cap once throughput allows.

## Uses (downstream)
1. **Eval features (Kimi):** re-test pawn-structure / zone / intent on the enlarged corpus — the
   current "ablate" verdicts are below the 16-game noise floor, so they're not trustworthy yet.
2. **Search-shape SPSA:** A-vs-B (laser vs flashlight vs current) by self-play win-rate.
3. **NNUE (P7):** eventual training data (needs ≫ this; this is a start, and the loop compounds).

## Caveats
- Self-play games reflect the **v0 eval** (weak), so they're a bootstrap, not ground truth — the
  standard self-play tuning loop (play → tune → stronger → play). Mixing with the human corpus dilutes
  quality but adds quantity; keep them in `selfplay_games/` (separate) so tuning can choose the mix.
- To tune on them, point `texel_tune` at `selfplay_games/` (one-line change) or merge dirs.
