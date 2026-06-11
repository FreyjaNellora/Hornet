# How we try again — the call, grounded in the overnight data

Your four levers were **wider / different eval-order / more-or-fewer terms / attunement.** Here's what
the data says about each, and the recommendation. (Evidence: EXP-016 depth-pathology, ENGINE-MATH,
the `tools/fit_weights.py` bootstrap, the self-play A/B.)

## What depth-worse actually exposed (two faults, in sequence)
Depth is a probe; it exposed two faults, and **two of your levers are the fixes:**
- **A pruning fault (search).** The d8 deficit shrank monotonically as breadth rose — laser **−47%** →
  flashlight cap-400 **−24%** → cap-1200 **−7%** (even). Most of the *hurt* was the beam dropping the
  best line. **Wider breadth fixes it — you were right.**
- **A wrong-objective fault (eval).** With pruning fixed, depth goes **neutral, not positive** — deeper
  doesn't *win more*, because the eval doesn't reward depth (EXP-012) and optimizes the wrong thing
  (points-blind, zero-sum material vs the non-zero-sum FFA objective; ENGINE-MATH §5). **Only attunement
  makes depth pay.**
So it's not one lever — it's **wider** (to stop depth hurting) **then attunement** (to make depth help and
to play for the win).

## The levers, weighed

**1. Wider (more breadth / cap).** *A real fix — your instinct was right.* The d8 deficit shrinks to ~0
as the cap rises (−47% → −24% → −7%): the beam was dropping the best line, and breadth recovers it. **Use
the flashlight with a generous cap (≥~1000); never the laser.** This stops depth from *hurting* — but it
only gets depth to *neutral*. For depth to *help*, you need lever 4.

**2. Different eval-order / reweighting.** *Exhausted.* The proper scipy fit + bootstrap (32 human
games, corrected labels) on the four components:
| weight | fit | 95% CI | verdict |
|---|---|---|---|
| material | 4.34 | [4.06, 4.65] | **real** |
| positional | 1.24 | [−0.17, 2.49] | **noise** (CI includes 0) |
| safety | −1.11 | [−2.02, −0.06] | **real, but NEGATIVE — current safety *hurts*** |
| crossfire | 1.03 | [0.59, 1.55] | **real** |
Reweighting the existing components is done: the optimum is ≈ material + crossfire (the deployed
`6,0,0,1`). Re-ordering won't help.

**3. More / fewer terms.** *Fewer is wrong* (crossfire is significant; material-only is weaker). *More*
is the eventual answer — but the per-square/scalar class is **dead** (positional = noise here, after 8
variants), so "more" must mean the **relational** class (pawn structure, outpost, rook-open-line) **and
rebuilding the broken safety term** (it's significantly *negative* — its construction is inverted/wrong,
not just unweighted). Caveat: at 32 games the gates can't prove these (positional already reads as
noise), so this is **data-gated** and belongs *after* lever 4.

**4. Attunement (aim for the win).** *This is the lever.* The depth-pathology, the points-blind eval
(Hard Rule #8), the zero-sum-by-construction eval vs the non-zero-sum FFA reality, and the drawish
self-play all indict the **objective**: the eval optimizes *relative material*, but the game is won on
*FFA points / eliminations / placement.* Optimizing material harder (more depth) makes you the material
leader → the target → you score worse. Fixing the objective is the only thing that turns "more search"
from harmful to helpful, **and** it de-drawishes self-play, which is what lets us tune everything else
without a human corpus.

## Recommendation (order matters)
0. **Now (trivial): flashlight + a generous cap (≥~1000), never the laser.** It took depth from −47% to
   ~even. Stops depth hurting. (The A/B harness already runs the flashlight.)
1. **Attunement (the lever): the bounded win/objective term (Phase 0).** It fixes the root the
   depth-pathology points to (wrong objective), is what makes depth actually *help*, and de-drawishes
   self-play so everything else can be tuned without human games. Gate on the self-play A/B.
2. **Stop reweighting the four components** — optimum found: material + crossfire (bootstrap: positional
   is noise, safety significantly negative).
3. **Then relational terms + rebuild safety** — on a *correct* objective, gated on the now-decisive
   self-play (+ a larger corpus). Safety tests **negative**, so it's not missing, it's *wrong* — rebuild
   it (king-shelter / attack-units table from the pitch), don't just leave it off.

## The through-line
Depth-worse exposed **two** faults: an unsound beam (fixed by **wider** — confirmed by the cap-spectrum,
−47%→−7%) and a wrong objective (fixed by **attunement**). So: **flashlight + generous cap, then the win
term**, with relational features + a rebuilt safety after. Everything is still rooted in the array
line-projection (`LineMap`); the win term and the relational terms both read from it. The new `tools/`
(fit_weights + bootstrap CIs, ab_stats significance, perf_breakdown) are how we keep each step honest
instead of eyeballing it.
