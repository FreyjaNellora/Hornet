# EXP-025 — C3: unbundled pawn structure (isolated / doubled / connected) on the clean corpus

- **Date:** 2026-06-11 · (C3.1 queries authored by Kimi; fit + analysis by Fable after her cutout)
- **Hypothesis:** pawn-structure terms, unbundled into independent readouts, carry outcome signal
  the bundled `query_pawn_structure` (ablated at the 16-game noise floor, marginal ~0.00004) could
  not show — now testable on the 241-game clean corpus (EXP-023).
- **Lever / change:** `queries.rs`: `query_pawn_isolated/doubled/connected` (Kimi) — raw counts
  per player, lane-aware per 4PC orientation (Red/Yellow by file, Blue/Green by rank). **Nothing
  wired into the eval** (see Conditions/Conclusion). Fitted via the EXP-024 tuner extension.

## Method

As EXP-024: 9-weight joint fit + single-term marginal fits on 241 games / 13,924 positions,
sigmoid-MSE vs placement outcome, K=0.0001, noise floor ≈ 0.00005 (null-term drops).

## Results

**Joint fit is collinear garbage for the pawn triple** — ISO=1470 / DBL=−555 / CONN=−240 with a
combined −0.00121: a pawn is either isolated or connected, so the optimizer rides a direction in
the anti-correlated subspace with huge opposing weights. Recorded as a methods lesson; the
single-term marginals are the real read:

| Term | weight (util units; ÷4 ≈ cp) | MSE drop | Verdict |
|------|------------------------------|----------|---------|
| **ISO** | 1175 (≈ −290 cp per isolated pawn) | **0.00054 (~11× floor)** | **Passes Texel.** Caveat below. |
| CONN | 280 (≈ +70 cp per connected pawn) | 0.00018 (~3.5× floor) | Borderline pass. |
| DBL | −165 | 0.00000 | **Null.** Doubling carries no signal here (plausibly rare/benign under 4PC central promotion). |

**The ISO caveat (don't deploy the magnitude):** ~290 cp per isolated pawn is implausible as a
causal value (classical chess: tens of cp). On an outcome-labelled corpus, isolated-pawn count is
partly a *symptom of already-losing positions* (structures shatter as a player collapses), and
Texel cannot distinguish symptom from cause. The fitted scale is a prediction weight, not a play
weight — the self-play arm decides whether playing *as if* isolation matters helps.

## Addendum (same day): +49 human games, dual-base marginals

On the enlarged corpus (290 games / 17,003 positions; human fraction 48%) with the
base-sensitivity correction (marginals reported on both canonical bases — see EXP-024):

| Term | drop on deployed (6,0,0,1) | drop on texel-shape (4,0,0,1) |
|------|---------------------------|-------------------------------|
| **ISO** | **0.00040 (w 1105)** | **0.00066 (w 1330)** |
| CONN | 0.00000 | 0.00026 (w 335) |
| DBL | 0.00006 | 0.00007 |

**ISO is the one robust term — it passes on every base and corpus tested** (0.00040–0.00081,
6–13× floor). CONN's earlier borderline pass was base-dependent (vanishes at M=6) — downgraded to
fragile; include it in the P′ arm only as a secondary. DBL null everywhere. The P′-rebuild recipe
in the Conclusion is unchanged, with ISO as its core and the symptom-vs-cause caveat still the
reason it must pass self-play before shipping.

## Addendum 2: human-only fit — the signal is a human-game behavior

`HORNET_HUMAN_ONLY=1` (140 games / 8,205 positions; floor ≈ 0.0002 from WIN's null level):

| Term | deployed (6,0,0,1) | texel-shape (4,0,0,1) |
|------|--------------------|------------------------|
| **ISO** | **0.00326 (w 3305)** | **0.00379 (w 3355)** |
| **CONN** | **0.00130 (w 860)** | **0.00244 (w 1110)** |
| DGR | 0.00094 (w 5) | 0.00177 (w 6) |
| DBL | 0.00046 (w **−2500**) | 0.00044 (w −2335) |
| WIN | 0.00001 | 0.00022 |

The structural/safety signals are **concentrated in human games** (~8× ISO, CONN resurrected,
DGR passing even at M=6) — the self-play half of the mixed corpus dilutes them, as expected: a
structure-blind engine produces games where structure varies ~randomly and cannot correlate with
outcomes. This is the cleanest evidence yet that these are real behaviors-over-many-games, while
also sharpening the deployment caveat: part of the human signal is "good players build good
structure," which playing-as-if can't fully capture — the P′ self-play arm remains the gate.
**DBL sign-flip:** on human data doubled pawns predict *winning* — in 4PC doubling arises from
capturing toward one's promotion lane, so it's a capture-activity symptom, opposite of 2-player
folklore. Do not penalize doubling.

- The three pawn queries live in `queries.rs`, exercised by `texel_tune`, **not wired into the
  eval**. Deliberate deviation from the original step plan ("wire passing terms at default-0"):
  wiring into P at any default-0 scale is dead code while `W_POSITIONAL = 0` zeroes the whole
  component — the exact inert pattern EXP-024 just reverted. No dead experiment-looking code.

## Conclusion

First pawn-structure signal above the noise floor in project history (the bundled version never
cleared it), with isolation carrying most of it. **Deployment recipe (the measured arm a future
shift runs):** rebuild P as a live component — start minimal, `P′ := −scale·iso (+ scale·conn)`
with the control/threats/PST base left out (it's the part EXP-015 measured net-harmful), set
`W_POSITIONAL = 1`, gate on move-agreement vs the arm-(iii) baselines (13.5/13.6/13.6%) and a
self-play arm. Pair naturally with the S′ danger-table rebuild (EXP-024) — together they are the
"fixed positional/safety components" the eval comments have been waiting on since EXP-015.
