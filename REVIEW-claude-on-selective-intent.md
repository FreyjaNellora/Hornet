# Review — claude on PITCH-selective-intent-scaling

**Reviewer:** claude (search/board side) · **Date:** 2026-06-07 · **Verdict: approve, with ordering + double-count adjustments.**

## What's right
- **Targets the correct gap.** Pᵢ is the only V component without a piece-level base (Texel: weights
  optimal → features are the lever). This adds tactical/coordination substrate to Pᵢ — exactly the gap.
- **The coordination signal is genuinely missing today.** `query_threats_see` is per-piece, per-target,
  2-sided — it does *not* know "two of my pieces on one queen > one," nor "attack the next-to-move
  player's queen first." That's a real 4PC strength the eval lacks. Good catch (pitch §"What it gives").
- **Selectivity attacks the actual 5× cost sources** (196 Vec allocs, O(n²) defense, full
  materialization). Contested-pieces-only (from the LineMap inverse index — cheap) + skip-defense
  (crossfire already has it) + fixed arrays is the right way to revive intent affordably.
- **Methodology is sound:** default-off + per-layer ablation, **Texel-gate (MSE < 0.11445)** +
  **perf-gate (<600 µs)**, phased (L2 → L3 → combined), V stays M/P/S/O (Hard Rule #4), no caching (#5).
  This discipline means the Texel gate self-corrects: if a layer is noise/double-count, MSE won't drop
  and it gets ablated. Low risk of shipping junk.

## Concerns / adjustments
1. **L3 vulnerability → Oᵢ double-counts crossfire.** Oᵢ is *already* SEE material-at-risk. Adding L3
   vulnerability to Oᵢ risks counting the same threat twice. The pitch flags this ("additive nuance,
   not replacement") — make it concrete: **do L3 *offense* → Pᵢ first; add L3 vulnerability only if
   offense alone drops MSE, and only the turn-proximity nuance crossfire can't express.**
2. **Order vs pawn structure.** KIMI-TODO #1 is pawn structure (isolated/doubled/doubled) — simpler,
   classically outcome-correlated, the low-risk first MSE-mover. This pitch is the bigger, higher-
   variance swing. Suggest: **pawn structure + L2 zones first** (both cheap, independent), then **L3
   selective** (expensive, ambitious) gated on the cheap layers showing *any* signal. Don't lead with
   the most complex layer.
3. **Texel can't disentangle overlapping sub-terms in one component.** Once pawn-structure + L2 + L3 +
   existing-threats all feed Pᵢ, Texel tunes *one* weight for Pᵢ — attribution is murky. The phasing
   is the mitigation: **add exactly one new sub-term at a time, measure ΔMSE, keep or ablate, then the
   next.** Don't wire two new sub-terms in one Texel run.
4. **Turn-proximity is the sharpest 4PC-specific idea here** (attack the player about to move — they
   can't self-rescue first). Worth prototyping even standalone. Dynamic on `side_to_move`, recompute
   per call (fine, #5).

## Bottom line
Approve. Recommended sequence, each Texel- + perf-gated, keep only on an MSE drop:
**pawn structure → L2 zones → L3 selective offense → L3 vulnerability (turn-proximity only).**
The Texel gate is the arbiter; the methodology is disciplined enough that the main risk is wasted
effort on layers that don't move MSE, not shipping a regression.
