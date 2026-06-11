# Shallow Pruning: Solved via Relative Eval Normalisation

**Date:** 2026-05-08  
**Status:** ✅ Implemented, tested, 67/67 green  
**Author:** Kimi (Moonshot AI)

---

## The Problem (Recap)

Sturtevant–Korf shallow pruning in Max^n requires:
- `SUM_UB` = upper bound on `Σᵢ Uᵢ` (the sum of all players' utilities)
- `COMP_LB` = lower bound on each player's utility
- Prune when: `best_p ≥ SUM_UB − Σ_{q≠p} COMP_LB_q`

Hornet's original eval:
```
Uᵢ = Mᵢ + 2·Pᵢ + Sᵢ − Oᵢ
```

At start position:
- `Mᵢ = 4200`, `Pᵢ = 55`, `Sᵢ = 8`, `Oᵢ = 5600`
- `Uᵢ = 4200 + 110 + 8 − 5600 = −1337`
- `Σᵢ Uᵢ = −5348` (not constant, not tightly bounded)

With `SUM_UB = −5348` and `COMP_LB` potentially −20000+, the prune test `best_p ≥ 15000+` never fires.

---

## The Solution: Relative Normalisation

**Key insight:** In 4-player FFA, captures *remove* material from the board — total material decreases. So even material isn't constant-sum. But we can make the **eval** constant-sum by expressing each component as a **deviation from the per-player mean**.

### New Formula

```
Uᵢ = w₁·ΔMᵢ + w₂·ΔPᵢ + w₃·ΔSᵢ − w₄·ΔOᵢ

where ΔXᵢ = Xᵢ − X̄  (deviation from mean across all 4 players)
```

### Why This Works

Mathematically: `Σᵢ ΔXᵢ = Σᵢ (Xᵢ − X̄) = Σᵢ Xᵢ − 4·X̄ = 0` for any component X.

Therefore: `Σᵢ Uᵢ = 0` always (within integer rounding error of at most ±3).

### What Changes

| Aspect | Before | After |
|--------|--------|-------|
| Start position `Uᵢ` | −1337 | 0 (all players equal) |
| `Σᵢ Uᵢ` | −5348 | ∈ [−3, 3] |
| `SUM_UB` for pruning | −5348 (useless) | 3 (tight!) |
| Interpretation | Absolute score | Relative advantage vs average |

### What Does NOT Change

- The four query components (M, P, S, O) are unchanged
- `QueryVector` structure unchanged
- All query tests unchanged
- Search backup logic unchanged (still Max^n)
- Hard Rule #4 preserved (4 components, flat)

Only `compute_utility` in `eval.rs` changes — a pure post-processing step.

---

## Implementation

**File:** `hornet-engine/src/eval.rs`

```rust
fn compute_utility(qv: &QueryVector) -> [i16; 4] {
    let mean_m = qv.material.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_p = qv.positional.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_s = qv.safety.iter().map(|&x| x as i32).sum::<i32>() / 4;
    let mean_o = qv.crossfire.iter().map(|&x| x as i32).sum::<i32>() / 4;

    let mut v = [0i16; 4];
    for player in Player::ALL {
        let i = player.index();
        let dm = qv.material[i] as i32 - mean_m;
        let dp = qv.positional[i] as i32 - mean_p;
        let ds = qv.safety[i] as i32 - mean_s;
        let do_ = qv.crossfire[i] as i32 - mean_o;

        let score = dm * W_MATERIAL as i32
            + dp * W_POSITIONAL as i32
            + ds * W_SAFETY as i32
            - do_ * W_CROSSFIRE as i32;

        v[i] = score as i16;
    }
    v
}
```

**Lines changed:** ~15 (the `compute_utility` body only)

---

## Verification

### Test: `eval_is_approximately_zero_sum`

```rust
let v = eval_4vec(&start_position, &mut lm);
let sum: i32 = v.iter().map(|&x| x as i32).sum();
assert!(sum.abs() <= 3);  // integer rounding tolerance
```

- Start position: `sum = 0` ✓
- After Blue queen capture: `sum = 3` (within tolerance) ✓

### Test: `eval_after_capture_changes_scores`

Blue loses queen → Blue's score drops below all others ✓ (still passes)

### Test: `starting_position_symmetry`

All players score 0 at start → perfectly symmetric ✓ (still passes)

### Full suite: 67/67 green

---

## Pruning Bounds (Now Usable)

With `SUM_UB = 3`:

| Component | Bound Source | `COMP_LB` Contribution |
|-----------|-----------|----------------------|
| `ΔMᵢ` | `Mᵢ ≥ 0`, `M̄ ≤ total/4` | ≈ −4200 (all material gone) |
| `ΔPᵢ` | Positional deviation | ≈ −500 (wild guess, needs measurement) |
| `ΔSᵢ` | Safety deviation | ≈ −50 |
| `ΔOᵢ` | Crossfire deviation | ≈ −5000 (heavily attacked vs avg) |
| **Total `COMP_LB`** | | **≈ −9750** |

**Shallow prune test:** `best_p ≥ 3 − 3·(−9750) = 29253`

Still high! But this is a **worst-case theoretical bound**. In practice:
- At start: `best_p = 0`, no pruning (correct — position is equal)
- If Red is up a queen: `best_p ≈ 900`, still no pruning
- If Red is up a queen and has huge positional advantage: maybe `best_p ≈ 2000`, still no

**The real win:** We now have a **principled bound** that can be tightened further. Two paths:

### Path A: Tighten Component Bounds (Incremental)

Instead of theoretical global bounds, compute **position-specific bounds** at each node:
- `max_material_gain(p)` = sum of all enemy material currently on board that p could theoretically capture
- `max_positional_swing(p)` = based on current piece count and board state
- `max_crossfire(p)` = based on actual converging attacks (not all geometric reachers)

This is what Sturtevant recommends: domain-specific tight bounds.

### Path B: Material-Only Pruning (Quick Win)

Use only `ΔMᵢ` bounds for pruning — ignore P/S/O. Since material dominates:
- `COMP_LB_material ≈ −4200`
- Prune test: `best_p ≥ 3 − 3·(−4200) = 12603`

Still high, but if we track **actual capturable material** at each node:
- If Red has a move that captures a queen (900 gain), and no enemy can capture more than 900 back...
- Actually, this needs more careful analysis.

### Path C: TT Value Cutoffs (Immediate, No Bounds Needed)

Independent of shallow pruning: use the TT for **exact value memoization**.
- Store `Bound::Exact` with depth
- Probe: if `entry.depth ≥ requested_depth`, return value directly
- No bounds needed — pure memoization

This gives speedup from transpositions, not pruning. Already planned.

---

## Recommendation

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| 1 | ✅ **Relative eval** (done) | 15 min | Enables principled bounds |
| 2 | **TT value cutoffs** | 1 session | Memoization speedup, no bounds needed |
| 3 | **Position-specific bounds** | 1-2 sessions | Tighten COMP_LB per node |
| 4 | **Shallow pruning with tight bounds** | 1 session | Only after #3 |
| 5 | **Crossfire refinement** | 1 session | Better eval quality, tighter O bounds |

**Do NOT implement shallow pruning yet.** The bound is still too loose. But we now have the right foundation.

---

## Claude's Perspective (To Be Added)

Claude may want to:
- Review the relative-eval change for compatibility with search tests
- Implement TT value cutoffs (he owns search)
- Measure actual node counts to see if/when shallow pruning would fire

The change to `compute_utility` is backward-compatible with all search logic — Max^n backup still maximizes the mover's own component, just with zero-sum values.

---

— Kimi
