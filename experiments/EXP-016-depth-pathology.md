# EXP-016 — why does more depth play WORSE? (the depth-pathology)

- **Date:** 2026-06-08 · **Status:** in progress (overnight)
- **Trigger:** in the self-play A/B gate, depth-8 lost to depth-4 (0/6). The user's principle: *a sound
  engine can never get worse with more search* — if it does, the search is unsound or the eval is
  wrong, and **depth is the instrument exposing it.** This is a diagnostic, not a quirk.

## The Gödel lens (why it's the right frame)
The eval `V` is a formal model of the game. It is **incomplete**: it cannot express the real FFA
objective — FFA points, the +20 for eliminating a player, final placement, alliance/kingmaker dynamics.
Search operates strictly *inside* this model; it can only find what `V` can express. So **more search
cannot reach truths `V` does not encode — it just commits harder to `V`'s blind spots.** Depth-worse is
the model's incompleteness made measurable. The fix for incompleteness is to *extend the model* (a
better eval), not to *prove harder inside it* (more search).

## Reduce to the math: when can Max^n + this eval make depth hurt?

**Notation.** State `s`. True value `V*_i(s)` = player i's expected final FFA points under best play.
Eval `V(s) = ⟨U_1,…,U_4⟩` with `U_i = w·(c_i − mean(c))` (mean-relative), so `Σ_i U_i ≈ 0` (zero-sum *by
construction*). Today `c_i ≈ material_i` and `V` is points-blind. Max^n: at a node where p moves, p
picks the child maximizing that child's `U_p`; the node inherits the chosen child's whole vector;
leaves get `V(leaf)`.

**Why 2-player intuition fails.** In 2-player minimax with a fixed, consistent eval, deeper search is no
worse in expectation (it's a better estimate of the true minimax value; pathology needs a contrived
eval). Max^n has **no such guarantee**, for three independent reasons:

1. **Beam unsoundness (search-side).** The flashlight keeps the top-`W` children per level by the
   mover's own eval-gain and discards the rest. If the truly-best line *looks* bad to `V` near the root
   but pays off deep, it is pruned and **never searched** — and deeper search amplifies the exclusion.
   At `W = ∞` this vanishes (the flashlight provably equals exact Max^n — already validated). So this
   cause is a **monotone function of pruning**: it should *shrink as the cap rises*.

2. **Opponent-model mismatch (Max^n-side).** Max^n's backup is only correct if every opponent actually
   maximizes its *own* `U`-component w.r.t. the same eval. In the A/B the other seats run a *different*
   config; even in pure self-play the random openings + a wrong `V` break the assumption. Deeper Max^n
   commits *harder* to a wrong opponent model → can be strictly worse. This is **not** removed by more
   breadth or depth; it's intrinsic to Max^n under model mismatch.

3. **Eval-incompleteness (eval-side — the Gödel core).** Even with perfect pruning *and* a correct
   opponent model, `U_i ≈ relative material ≠ V*_i`. `V` is (a) **points-blind** (ignores the +20 /
   placement) and (b) **zero-sum by construction** while FFA is **not** zero-sum (alliances, kingmaking,
   being-targeted are non-zero-sum / non-transitive). So Max^n optimizes the **wrong objective**, and
   the depth-`d` optimum of `U` diverges *further* from the optimum of `V*` as `d` grows — deeper search
   commits harder to the wrong objective. **Not fixable by search; needs a more complete eval.**

The three compound. (1) is search-side and pruning-monotone; (2)+(3) are model/eval-side and pruning-
*independent*. That gives a clean experiment.

## The experiment that separates them — cap spectrum
Run d8 vs d4 with the **flashlight** at rising caps (more cap = less pruning). Hold everything else
fixed (same eval, same opening protocol, seat-balanced).
- **If the d8 deficit shrinks toward 0 as the cap rises** → cause **(1) beam pruning** dominates →
  search-side, fixable by a wider cap / sounder pruning.
- **If the deficit persists at high cap** (minimal pruning) → causes **(2)/(3)** dominate → the eval /
  Max^n model is wrong → *depth is correctly reporting that the eval is the bottleneck*, which is the
  objective-alignment direction. Not a search fix.

A second control: **engine vs random-mover.** If the searched engine can't decisively beat random play,
something is broken at a more basic level than depth.

## Results
- **Laser d8 vs d4:** A win-rate **0/6**, points **117 vs 219**. (Laser is unsound — pruned to one
  line; depth amplified it. This is the wrong search; discarded.)
- **Flashlight d8 vs d4, cap 400** (the *sound* search): A win-rate **2/6 = 33%**, points **214 vs 282**
  (per seat 17.8 vs 23.5). → **The flashlight recovered ~half the gap** (0%→33%, 117/219→214/282), so a
  real chunk of the pathology *was* the laser's unsoundness. But d8 **still underperforms d4** — a
  residual remains. (n=6 is small; the points gap is the clearer signal.)
- **Flashlight d8 vs d4, cap 1200** (less pruning): win-rate still 2/6, but points **189 vs 202** — the
  gap **collapsed** vs cap-400 (68→13). Statistically even (ab_stats: p=0.69, 7% points gap).
- **The cap spectrum (the decisive trend):**
  | breadth | d8 vs d4 points | d8 relative |
  |---|---|---|
  | laser (~cap 1) | 117 vs 219 | −47% |
  | flashlight cap 400 | 214 vs 282 | −24% |
  | flashlight cap 1200 | 189 vs 202 | −7% (even) |
  The deficit shrinks **monotonically toward zero as breadth rises** → the depth-HURT is **cause (1),
  beam pruning** (search-side). Wider fixes it.
- Tooling: numpy / scipy / scikit-learn / pandas installed; `tools/fit_weights.py` (bootstrap CIs:
  positional = noise, safety significantly negative, material+crossfire real); `tools/ab_stats.py`.

## Verdict — two real effects, in order
1. **Depth was HURTING because of beam pruning (search-side, cause 1).** The d8 deficit shrinks to ~0 as
   the cap rises (−47% → −24% → −7%). **Wider breadth fixes it** — use the flashlight with a generous
   cap; never the laser. (The user's "a bit wider" instinct was correct.)
2. **With the pruning fixed, depth is NEUTRAL, not helpful** (d8 ≈ d4 at cap 1200). Deeper search doesn't
   *win more* because the **eval doesn't reward depth** (EXP-012: depth doesn't change the move) — the
   incomplete, points-blind, zero-sum-modeled objective (causes 2/3). **Making depth actually *help*
   needs a correct objective — attunement** (the win term), not more search.

So depth-worse was the *probe* doing its job: it first exposed an unsound search (fixed by breadth), and
underneath it exposes the wrong objective (fixed by attunement). The order is **flashlight + generous cap
→ then the win/objective term** (which also de-drawishes self-play so the rest can be tuned without human
games). See SYNTHESIS-next-attempt.md.
