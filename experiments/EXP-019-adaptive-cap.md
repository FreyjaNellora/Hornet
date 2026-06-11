# EXP-019 — what sets the flashlight cap? (adaptive-beam scheduling)

- **Date:** 2026-06-08 · **Status:** in progress
- **Question:** the d12 result (EXP-017) showed breadth must scale with depth. *What* should the cap
  scale with — pieces, branching, depth, or position noise? The answer determines the adaptive-beam rule.

## Diagnostic 1 — cap vs branching/pieces (`examples/cap_vs_branching.rs`)
For 40 random-walk positions spanning a game, the **minimum cap whose move == the widest cap's move**
(move-stability convergence) at d8, vs branching and piece count. Fit with `tools/fit_cap.py`.

**Result — neither structural feature predicts the cap:**
| predictor | slope | R² | p |
|---|---|---|---|
| branching | −0.3 · branching + 612 | **0.00** | 0.98 |
| pieces | 12 · pieces − 95 | **0.01** | 0.67 |

Cap-need was **bimodal at fixed branching** (e.g. branching 22 → 50 *or* 3200; branching 30 → 50;
branching 37 → 1600). So pieces/branching **bound** the cap (can't keep more lines than exist) but do
**not set** it. → **The driver is noise / separability** (how clearly the best move stands out), which
the user predicted.

## Cross-check with search theory (RESEARCH-search-theory.md)
- Separability is the **singular margin** (singular-extensions): a move is "singular" if it beats its
  siblings by a margin — large margin = sharp = small cap; bunched = quiet = large cap. A ready detector.
- The widen-with-effort form is **progressive widening** (`width = c·N^α`).
- Our hard top-W cut ≈ **late-move pruning**; **LMR** (reduce-then-re-search) is the safer version.

## Caveats (fix in diagnostic 2)
1. **Metric flaw:** move-*identity* stability overstates quiet positions ("still moving at 3200" really
   means the moves are interchangeable, not that 3200 is insufficient). Use **value**-stability instead.
2. **Seed bug:** `seed|1` collapsed seeds {2,3} and {4,5} → only ~3 distinct positions/ply (fix the seeding).
3. Random-walk positions are low-diversity (max branching only 50, max pieces 64).

## Next — Diagnostic 2
`cap_vs_noise`: per position, record the **singular margin / top-candidate eval-gain gap** (noise),
measure **value**-convergence (not move-identity), sweep **depth ∈ {4,8,12}**, fixed independent seeds,
positions from real self-play. Fit `cap ≈ f(margin, depth)` (expect margin to carry it, depth to
compound). Then the rule: `cap = clamp(c · margin^β · depth^γ, floor, k·max_branching)` — coefficients
from the fit.
