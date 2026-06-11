# EXP-006 — the calibration gate (baseline before recalibration)

- **Date:** 2026-06-06
- **Purpose:** turn EXP-005's finding into a single acceptance number, and capture the current
  (broken) baseline so the recalibration (`PITCH-eval-recalibration.md`) has a pass/fail target.

## The gate
For each fixture, measure `|eval_4vec(after move) − eval_4vec(before move)|` for the **mover's own
component**, split by move type:
- **Quiet move** (no capture): should swing the eval by ~**tens** of centipawns (positional only —
  no material changed).
- **Capture move**: legitimately swings, but bounded by ~the **piece value** (≤ ~900); not thousands.

A sane eval cannot move by thousands on a move that changes no material. Printed by
`examples/gate_ablation.rs` as the `CALIBRATION` line.

## Baseline (current eval, the bug)
```
quiet-move swing    avg=1294   max=3506   (n=20)
capture-move swing  avg=5189   max=13691  (n=6)
```
- **Quiet moves swing the eval by ~1294 on average (max 3506)** with zero material change — pure
  miscalibration.
- **Capture moves swing by ~5189 avg / 13691 max** — far beyond any piece value; the crossfire
  `value × count` term amplifies the attacker/defender-set change into thousands.

## Pass target (after recalibration)
```
quiet-move swing    avg/max ~tens
capture-move swing  bounded by piece value (<= ~900)
```
When these hold, the eval is stable; only then are the move-match rate, the depth sweep (EXP-001), and
the `HORNET_SEE` threats comparison (EXP-002) trustworthy — re-run all three on the recalibrated eval.

## Status
Gate harness built (claude). Recalibration is `queries.rs`/`eval.rs` (Kimi's lane, confirmed) — fix
crossfire (`value × count` → SEE material-at-risk), scale safety into centipawns (fold `attack_value`),
lift `ffa_points` bounty out of Oᵢ, re-derive weights. Then this gate is the first acceptance check.
