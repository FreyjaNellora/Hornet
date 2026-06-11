# EXP-001 — depth × quiescence diagnostic

- **Date:** 2026-06-06
- **Hypothesis:** if the strength gate (0/13) is a *search-depth* problem, deeper search and/or
  tactical-resolution (quiescence) should lift the match rate; if it doesn't, the eval is the cause.
- **Lever / change:** search depth {4, 8} × quiescence {off, on}. No eval change.

## Conditions (before)
- Eval v0: `Uᵢ = w₁ΔMᵢ + w₂ΔPᵢ + w₃ΔSᵢ − w₄ΔOᵢ`, weights `4/2/1/1`, mean-relative (zero-sum),
  bounty folded into Oᵢ. `intent.rs`/`zones.rs` dormant.
- Search: beam 10 + forward pruning (LMR) + adaptive beam, all on in both arms (tractability).
- Node budget: 800k per search (added this session so capture-dense fixtures can't hang).
- Fixtures: `baselines/tactical_samples.json`, 13 testable.

## Method
`examples/gate_ablation.rs`, 2×2 over depth × quiescence, release, isolated target dir.
Each arm = the gate match rate (engine move == human move).

## Results

| | quiescence OFF | quiescence ON |
|---|---|---|
| **depth 4** | 0/13 | 1/13 |
| **depth 8** | 0/13 | 1/13 |

- Doubling depth (4→8): no change (0/13 both at qOFF; 1/13 both at qON).
- Quiescence: +1/13, the same at both depths.
- Process finding: depth-8 on tactical (capture-dense) fixtures *explodes* without the node budget —
  the adaptive-beam "tactical completeness" guard never prunes captures.

## Conditions (after)
- No eval/search lever changed by this experiment (diagnostic only). Quiescence + node budget remain
  default-off / configurable.

## Conclusion
The eval is the binding constraint, **but the depth result is a confound, not a verdict**: a faulty
eval makes depth useless at any depth (shit in → shit out). "Depth didn't help" only means "depth
can't help *through* a broken eval." → Fix the eval first (EXP-002), then **re-run this sweep** to
measure depth's real contribution. Quiescence's +1 is weak evidence that exchange resolution is the
gap — consistent with EXP-002's hypothesis.

**Re-test status (2026-06-06):** the eval was recalibrated (EXP-008). Depth is re-tested in *real
play* via **self-play (EXP-010)**, not the move-match sweep — EXP-004/005 showed exact-move match is
noise, so the clean depth re-test is self-play, not the gate.
