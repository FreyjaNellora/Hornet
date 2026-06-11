# NOTE — Max^n shallow pruning: deferred (low-ROI)

- **Date:** 2026-06-06
- **Decision:** defer shallow pruning. Not a run-experiment — a scoping analysis + decision.

## The analysis
Shallow-pruning cutoff (Korf/Sturtevant): `UB_p = SUM_UB − best_q − 2·COMP_LB`. Prune a subtree when
`UB_p ≤ alpha` (what the parent already secured).
- `SUM_UB` (upper bound on Σ Uᵢ): the recalibrated eval is ~zero-sum (`Σ Uᵢ ≈ 0`), so `SUM_UB ≈ 18`
  — **tight** (this was the old −5348 blocker, now fixed).
- `COMP_LB` (provable lower bound on one player's `Uᵢ`): a player can be down most of their material,
  so `Uᵢ ≈ −20,000`. Thus `−2·COMP_LB ≈ +40,000` **swamps** the bound → `UB_p` stays huge → **provable
  cutoffs fire ~never.** This is the known weakness of Max^n pruning (nothing like 2-player α-β).

## Why deferred (not built)
- The speed is **already banked** by forward pruning (LMR + adaptive beam): 12–28× measured
  (`search.rs`, `examples/search_bench.rs`). Shallow pruning would add ~nothing on top.
- Cutoffs would only fire with **clamped bounds** (PITCH-maxn-shallow-pruning option 2), which *change
  the eval at extremes* to make `COMP_LB` tight — trading eval fidelity for speed we already have.
- The one sound residual benefit (real TT-bounded values; TT is ordering-only today) is modest and
  intricate.

## Concern / revisit trigger
Revisit only if a **speed wall** appears (e.g. we need much deeper search and forward pruning isn't
enough), via clamped/position-derived bounds. Until then it's a no-op. Logged in ENGINE-HANDOFF
"What's left" #1.
