# Kimi's Analysis: Shallow Pruning Alternatives for Hornet

**Date:** 2026-06-02  
**Re:** `PITCH-maxn-shallow-pruning.md` + handwritten notes on sum-constant constraint  
**Author:** Kimi (Moonshot AI)

---

## The Core Problem

Sturtevant & Korf shallow pruning requires:
1. **Σᵢ Uᵢ = C** (constant-sum), OR
2. **Tight hand-provided bounds** on every component

Hornet's eval satisfies **neither**:
- `Uᵢ = Mᵢ + 2·Pᵢ + Sᵢ − Oᵢ` is not constant-sum (measured: ΣUᵢ = −5348 at start, not bounded)
- `Pᵢ` (positional), `Sᵢ` (safety), `Oᵢ` (crossfire) have no natural tight bounds

The pitch correctly identifies this: "a naïve provable SUM_UB/COMP_LB for this eval is enormous... UB_p stays far above any realistic alpha and you get zero cutoffs."

---

## Measured Data (Starting Position)

| Component | Per-Player Value | Notes |
|-----------|-----------------|-------|
| Mᵢ (material) | 4200 | Bounded: [0, ~16800] |
| Pᵢ (positional) | 55 | Unbounded-ish; scales with piece count × reach |
| Sᵢ (safety) | 8 | Can go negative (attackers > defenders + escapes) |
| Oᵢ (crossfire) | 5600 | ≥ 0; large at start due to many mutual attacks |
| **Uᵢ (v0)** | **−1337** | = 4200 + 110 + 8 − 5600 |

**Key insight:** Crossfire `Oᵢ` = 5600 at start because every piece is "attacked" by multiple enemies in the LineMap sense (reachers_at includes all geometric attacks, not just meaningful threats). This is **not a bug** — it's how the query is defined — but it means `Oᵢ` dominates the utility.

---

## Three Alternatives (Beyond the Pitch's Three)

### Alternative 0: Fix Crossfire First (Recommended)

**Problem:** Crossfire is massively overcounting. At the start position, every pawn is "attacked" by 2-3 enemy pawns diagonally, and every piece on the back rank is attacked by enemy pawns' forward pushes. The raw `reachers_at` count includes geometric attacks that aren't real threats.

**Fix:** Crossfire should only count **converging attacks on pieces that are actually threatened** — i.e., the attacker has a path to capture, not just a ray that passes through. Or: crossfire should only apply to pieces with **no defender** or **more attackers than defenders**.

**Impact on pruning:** If `Oᵢ` drops from 5600 to something proportional to actual tactical vulnerability (say, ~500), then:
- `Uᵢ` at start becomes ~4200 + 110 + 8 − 500 = **3818**
- `SUM_UB` for 4 players ≈ 15,272
- `COMP_LB` ≈ 0 (material can't go below 0, safety can go negative but bounded by −attackers)

Still not constant-sum, but **much tighter bounds**.

**Effort:** 1 session to refine crossfire. Then re-measure.

---

### Alternative 1: Beam Pruning (Already Implemented)

**What we have:** The search already uses beam width (top 30 moves at interior nodes). This is a **form of pruning** — it cuts branches based on move ordering, not value bounds.

**Why it's enough for now:**
- Beam pruning is already giving us ~10-30× node reduction (from full-width to beam)
- Shallow pruning would give additional reduction, but only if bounds are tight
- The pitch admits: "With loose bounds the prune test never fires"

**Verdict:** Beam pruning is the pragmatic win. Don't chase shallow pruning until the eval is tightened.

---

### Alternative 2: Material-Only Pruning (Sound, Weak)

**Idea:** Use **only material bounds** for pruning, ignore positional/safety/crossfire.

At any node:
- `Mᵢ` is bounded: `[0, total_material_on_board]`
- The maximum material a player can gain = sum of all enemy material
- If even the **best-case material gain** can't beat alpha, prune

**Formula:**
```
UB_material(p) = current_M[p] + sum_of_all_enemy_material
If UB_material(p) <= alpha: prune
```

**Why it's sound:** Material is the dominant term (4200 vs 55 for positional). If you can't even theoretically gain enough material, no amount of positional/safety swing will change the decision.

**Why it's weak:** It only prunes in positions where one player is already decisively ahead. In close positions, `UB_material` >> alpha, so no pruning.

**Effort:** 30 minutes to implement. Test: does it ever fire in real positions?

---

### Alternative 3: TT Value Cutoffs (Without Shallow Pruning)

**Idea:** Even without shallow pruning, the TT can do **value-based cutoffs** if we store exact values and probe with depth gating.

Current TT usage:
- Store: always `Bound::Exact`
- Probe: read `best_move` only (ordering)

**Enhancement:**
- Store with actual bound type
- Probe: if entry.depth >= requested_depth and bound is `Exact`, return the value directly
- This is **not pruning** — it's memoization. But it gives the same speedup.

**Why it works without tight bounds:** We don't need bounds for memoization. We just need the stored value to be exact (which it is, for the beam-limited tree).

**Effort:** 1 session. Lower risk than shallow pruning.

---

### Alternative 4: Replace Eval with Constant-Sum Design (Hard)

**Idea:** Redesign the eval so that `Σᵢ Uᵢ = 0` (zero-sum) or `Σᵢ Uᵢ = constant`.

**How:**
- Material: zero-sum by definition (one player's gain = another's loss)
- Positional: make it relative (control share, not absolute control)
- Safety: make it relative (safety differential)
- Crossfire: make it a transfer (one player's crossfire = another's attack)

**Example:**
```
Uᵢ = Mᵢ + 2·(Pᵢ − P̄) + (Sᵢ − S̄) − (Oᵢ − Ō)
```
where `P̄` = average positional across all players.

Then `Σᵢ Uᵢ = Σᵢ Mᵢ` (material is constant-sum, other terms cancel).

**Effort:** Major eval redesign. 2-3 sessions. Changes every test.

**Verdict:** Too invasive for v0. Consider for v1.

---

## My Recommendation

| Priority | Action | Effort | Speedup |
|----------|--------|--------|---------|
| 1 | **TT value cutoffs** (memoization) | 1 session | Moderate (skips repeated subtrees) |
| 2 | **Fix crossfire overcounting** | 1 session | Makes bounds tighter for future pruning |
| 3 | **Material-only pruning** | 30 min | Weak but sound; test if it ever fires |
| 4 | **Shallow pruning with measured bounds** | 2 sessions | Only after crossfire is fixed |
| 5 | **Constant-sum eval redesign** | 3 sessions | v1, not v0 |

**Do NOT implement shallow pruning now.** The pitch is correct: with current eval, bounds are too loose for cutoffs. Fix the eval first (crossfire), then measure, then prune.

---

## What I Can Do Now

1. **TT value cutoffs** — implement depth-gated TT probe that returns stored exact values. Low risk, measurable speedup.
2. **Crossfire refinement** — make crossfire count only genuine threats (not geometric rays). This improves eval quality AND tightens future pruning bounds.
3. **Material-only pruning** — quick experiment to see if it ever fires.

Pick one, or tell me to wait.

---

— Kimi
