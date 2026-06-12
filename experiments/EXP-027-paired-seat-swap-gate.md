# EXP-027 — paired seat-swap self-play: the powered gate instrument

- **Date:** 2026-06-12
- **Hypothesis:** EXP-024's null finding (A≡B reads 83% win-rate / +65% points from seat/game
  variance at 6 games) can be eliminated by design rather than overpowered by game count: play
  each (seed, split) **twice with A/B exchanged** — under identical configs the two games are
  move-for-move identical, so the pair difference is exactly zero by construction.
- **Lever / change:** `examples/selfplay_ab.rs` now plays antithetic pairs and reports the
  per-pair record (A/B/tie) instead of a per-game win-rate. Instrument-only change.

## Method + validation

Null run (A≡B, d4 cap 100, 6 pairs / 12 games): every pair must tie **exactly**.

## Results

**Exact ties on every pair** — e.g. pair 1: A 23 – B 23; pair 2: A 83 – B 83, with the swapped
game's points array identical to the original's ([41,4,25,13] both), confirming the
identical-games mechanism. The unpaired design's 83% false signal is structurally eliminated.

## Conditions (after)

- `selfplay_ab` is paired by default; `games_per_split` now means pairs per split (2 games each).
- A real A≠B effect now shows as a non-tie pair record + paired point differences; residual
  variance comes only from genuine config×position interaction, not seat luck.
- Future gates: power up pairs (not raw games) on top of this design. The EXP-017-era unpaired
  win-rates remain re-graded as unresolved (EXP-024).

## Conclusion

Confirmed — variance cancelled by construction, validated empirically. This is the standing
self-play gate instrument for every future strength claim (eval arms, search levers,
objective-layer defaults).
