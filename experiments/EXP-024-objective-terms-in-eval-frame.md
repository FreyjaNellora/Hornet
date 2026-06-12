# EXP-024 — C2 continuation: objective terms in the eval frame (defect → rework → fit)

- **Date:** 2026-06-11 · (continuation of Kimi's C2 shift after her session cutout; her work
  preserved and attributed)
- **Hypothesis (Kimi's C2):** the search-side objective-layer terms — elimination-proximity (win
  signal) and the non-linear king-danger table — carry outcome signal as *eval-side* terms, and
  should be tunable by move_tune/texel_tune.
- **Lever / change:** none shipped to the eval. The tuner (`texel_tune`) was extended instead;
  the eval hot path is byte-identical to EXP-022.

## The defect (audit finding, recorded so the pattern isn't repeated)

Kimi's in-flight implementation folded the two terms into the **values** of P and S, compute-gated
on new consts `W_WIN_PROXIMITY`/`W_KING_DANGER` — but the utility step multiplies P and S by
`W_POSITIONAL = W_SAFETY = 0`, and `compute_utility_n` never applied the new weights. Net effect:
any A/B arm would have paid the king-safety scan at every leaf and changed the eval output by
exactly **nothing**. General rule extracted: **never fold a candidate term into a zero-weighted
component** — it needs its own weight path end-to-end, or it is dead code that looks like an
experiment. The fold-ins were reverted (suite green before and after; the
`gated_queries_match_full_eval` equality gate held throughout, which is itself the proof the
folds were inert: they were dead code at weight 0).

## Method

`texel_tune` extended to a 9-weight fit `[M, P, S, O | WIN, DGR, ISO, DBL, CONN]` — the five
candidate terms computed independently per cached position (pub query fns; no eval changes),
mean-relative frame, fixed signs (WIN+ DGR− ISO− DBL− CONN+; candidate weights may go negative =
opposite-direction signal). Corpus: the clean EXP-023 set (241 games / 13,924 positions).
**Self-check passed:** with candidates at 0 the fit reproduces EXP-023 exactly (K=0.0001,
baseline MSE 0.12956). Joint fit + **single-term marginal fits** (each candidate alone over the
tuned deployed four — the interpretable read; the pawn triple is collinear, see EXP-025).
Noise floor estimated from the null terms' drops: ≈ 0.00005.

## Results (this experiment's terms; pawn terms in EXP-025)

| Term | single-term weight | MSE drop | Read |
|------|--------------------|----------|------|
| **DGR** (king-danger table) | **4** (classical direction; stable joint=single) | **0.00063 (~12× floor)** | **Real outcome signal.** Kimi's non-linear shape validated on clean data. |
| WIN (elimination-proximity) | 7 | 0.00005 (= floor) | **Null in the eval frame** — consistent with its design as a search-side *finishing gradient* (it fires late, on collapsing opponents; a static frame sees few such positions). |

Self-play arms (search-side knobs, flashlight cap 1200 d8, 6 games each — re-run of Kimi's lost
runs, incl. her null-control design):

| Arm | Pairing | Points | Win-rate | Decisive |
|-----|---------|--------|----------|----------|
| (i) | win 50 vs 0 | A 304 – B 222 (+37%) | 5/6 = 83% | 1/6 |
| (ii) | danger 100 vs 0 (both win 50) | A 169 – B 192 (−12%) | 3/6 = 50% | 3/6 |
| (iii) | **null control A≡B** | **A 251 – B 152 (+65%)** | **5/6 = 83%** | 2/6 |

**The headline is the null: identical configs produced an 83% win-rate and a +65% points edge
from pure seat-pair/game variance** — larger than either real arm's effect. Conclusions:
1. **6-game self-play resolves nothing.** Both real arms are inside the measured null artifact.
2. **EXP-017's 6-game win-rate claims (67%, 83%) sit at the null level and are hereby
   re-graded to "unresolved"** — the elimination *counts* in those runs remain objective facts
   (the decisiveness basis for the EXP-023 corpus config stands), but the win-rate inferences do
   not. The win/danger knobs stay default-off on unchanged evidence.
3. **Harness fix for the future powered gate:** paired seat-swap design — play each (seed, split)
   twice with A/B exchanged; differencing the pair cancels seat/game variance exactly, instead
   of hoping it averages out. Then power up the game count on top.

### Addendum (same day): +49 human games and the base-sensitivity finding

The user collected 49 new chess.com games (all standard-rules — see the RuleVariants audit in the
session note); corpus → **290 games / 17,003 positions** (human fraction 48%). Marginal fits
turned out **base-sensitive** — candidate signal partially overlaps material's (a collapsing
player is also down material), so a heavier `W_MATERIAL` absorbs danger/structure signal. The
tuner now reports marginals on both canonical bases:

| Term | drop on deployed (6,0,0,1) | drop on texel-shape (4,0,0,1) |
|------|---------------------------|-------------------------------|
| DGR | 0.00015 (w 2) | **0.00051 (w 3)** |
| WIN | 0.00018 (w **−12**, sign-flipped) | 0.00002 |

**Revised reads:** DGR's signal is real but **material-entangled** — strong on softer-material
bases, mostly absorbed at M=6. The S′ rebuild recommendation stands but its measured arm should
expect the eval-frame gain to be modest; the search-side runtime knob remains the primary
vehicle. WIN flips sign across bases at floor-level magnitudes — **null confirmed, now with the
instability documented**.

## Conditions (after)

- Eval hot path = EXP-022 exactly (reverted fold-ins; no new consts). Suite 115 lib + 3 green.
- `texel_tune` permanently carries the 9-weight + single-term-marginal machinery for future
  candidate terms.
- Deployment vehicles unchanged: search-side `with_win_term`/`with_king_danger` (runtime,
  EXP-017/018, the play path).

## Conclusion

The C2 question splits cleanly: **win-proximity is a search-layer term, not an eval term** (null
statically, sign-unstable across bases); **the danger table carries real but material-entangled
eval-frame signal** (strong at M=4, mostly absorbed at the deployed M=6). Recommended next move
(Kimi's lane): **rebuild the S component around the danger table** — S′ := danger-table scalar
(sign-flipped), refit `W_SAFETY` — with tempered expectations, behind a *properly powered*
self-play arm. On that last point this experiment's most durable contribution is methodological:
the null control measured the 6-game instrument's false-signal level at **83% win-rate / +65%
points**, re-grading all prior 6-game win-rate reads and defining what "powered" must mean
(paired seat-swap design + many more games).
