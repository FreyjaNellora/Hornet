# EXP-029 — P′/S′ candidate-eval arms on the paired gate: terms real, fitted scales unplayable

- **Date:** 2026-06-12
- **Hypothesis:** the two Texel-nominated terms (EXP-024/025: isolated pawns, danger table) at
  their human-fitted scales improve play.
- **Lever / change:** injectable-eval machinery — `Searcher::with_eval` made pub
  (experiment-only); `eval_4vec_pprime` (deployed + ISO at 3305) and `eval_4vec_sprime`
  (deployed + danger-table at 5) as cold candidate fns; eval selectors in `selfplay_ab` and
  `move_match`. **Deployed eval untouched.**

## Method

Three instruments per candidate, vs the deployed eval:
1. Move-agreement (beam 10, d4, S2, 32 games) vs the 13.6% baseline.
2. **Winners-only move-agreement** (new this experiment: agreement counted only on moves by
   players who finished 1st/2nd — blunder-prone losing play removed from the target).
3. **Paired seat-swap gate** (EXP-027): 6 pairs / 12 games, flashlight d8 cap 1200 — seat/game
   variance cancels by construction, so pair records are config effects.

## Results

| Instrument | deployed | P′ (ISO@3305) | S′ (DGR@5) |
|------------|----------|----------------|-------------|
| move-agreement (all) | 13.6% | 12.7% | 10.5% |
| paired gate (points) | — | **315 – 492** | **394 – 522** |
| paired gate (pair record) | — | **1–5** | **1–5** |

Both candidates **rejected at their fitted scales** — consistently across both instruments, with
the paired design guaranteeing the gate deficits are real config effects.

**New baseline metric:** winners-only agreement (deployed eval) = **181/1455 = 12.4%**, *below*
the all-moves 13.6% — **the engine currently agrees more with losing players than with
winners**. The deployed material+crossfire style is the short-horizon grabby style that loses
4PC games; the improvement target is specifically the winners gap.

## The lesson (now measured twice at two layers)

Outcome-**prediction** weights are not move-**choice** weights. A term can carry real
placement-predictive signal (ISO ~15× the Texel noise floor on human games) while its fitted
magnitude — which includes all the symptom-of-losing correlation — is far too loud to *play*:
~300cp per isolated pawn and ~5 pawns-equivalent of danger swing drown the tactical layer that
actually wins material. EXP-015 learned this for the original P/S components; EXP-029 confirms
it generalizes to any Texel-fitted candidate. **Recipe going forward:** Texel nominates terms;
play scales come from a separate sweep (e.g. ISO ∈ {400, 800, 1600}, DGR ∈ {1, 2}) gated on
winners-only agreement first, the paired gate second.

## Horizon check (the d8 winners-gap question) — answered: it's priorities, not horizon

Flashlight cap 1200 (the depth-pays shape, EXP-017), same sampled positions (S=4, 1,270
positions / 668 winner-moves):

| Depth | all | winners-only |
|-------|-----|--------------|
| 4 | 14.4% | 13.0% |
| 8 | 14.3% | 12.3% |

**Doubling the horizon moved nothing** (all-moves flat; winners-only if anything down — noise).
Two conclusions:
1. **The winners gap is a priorities (eval-features) problem, not a horizon problem.** Winners'
   moves aren't deep moves the engine could find with more lookahead — they're moves the current
   eval doesn't value at any reachable depth. Feature work leads; depth executes. (Strength is a
   different axis: EXP-017's depth-pays results were *placement* with the objective layer on —
   depth can still win games without increasing human-agreement.)
2. **The d4 instrument depth is vindicated on the modern engine** (closing the 2026-06-11 open
   dispatch item): move choice is eval-dominated, so d4 comparisons transfer. Side finding:
   *breadth* alone helps even at d4 — flashlight cap 1200 reads 14.4/13.0 vs the maxn-beam-10
   instrument's 13.6/12.4, consistent with EXP-016's beam-drops-good-moves diagnosis.

## Conditions (after)

- Deployed eval byte-identical throughout; candidates remain as cold fns for the scale sweeps.
- Winners-only agreement is now a standing move_match readout; `fcap` arg gives move_match a
  flashlight mode for horizon studies.
- Rotation-equivariance test added (passed first try) — the eval's 4-fold symmetry is pinned,
  not assumed.

## Conclusion

The candidate terms survive; their volumes don't. Nothing ships (Hard Rule #6 honored by
measurement, not paperwork). The instruments did exactly what the last week built them to do:
three independent reads, one coherent verdict, no possibility of seat-luck false positives.
