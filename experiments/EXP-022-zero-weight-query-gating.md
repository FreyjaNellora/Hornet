# EXP-022 — gate the zero-weight queries (C1): +41% search throughput, output-identical

- **Date:** 2026-06-10
- **Hypothesis:** `run_all_queries` computes positional control, SEE threats, PST, and the
  king-safety scan at every leaf and then multiplies them by `W_POSITIONAL = W_SAFETY = 0`;
  skipping what the weights zero out is a pure-perf win with provably identical eval output.
- **Lever / change:** `queries::run_queries_gated(lines, board, need_positional, need_safety)` —
  gated-off components return zeros. `run_all_queries` is now `run_queries_gated(.., true, true)`
  (the **full** vector — `texel_tune` and weight-exploration tooling are unchanged). `eval_4vec`
  calls the gated variant with `W_POSITIONAL != 0, W_SAFETY != 0` — const flags, so the gating is
  compile-time, and if a weight is ever un-zeroed the corresponding query switches back on by
  construction. Not a strength lever: output-identical (below), so exempt from default-off.

## Conditions (before)

- Post-EXP-020/021 tree (ordering flags default-off; `count_defenders` fixed). Weights `(6,0,0,1)`.
- The search-side king-danger term calls `query_king_safety` **directly** (search.rs
  `eval_with_win`), never through `run_all_queries` — unaffected by the gating. Verified.
- Baseline perf: EXP-021's freecap-OFF arm — `bench_beam 0 0 1 8 10 5` (five seeded mid-game
  positions, depth 8, beam 10, fwd+adaptive, idle box): **median 64,337 nodes/sec**, node counts
  1,395,635 / 2,101,082 / 3,101,463 / 6,077,347 / 4,282,640.

## Method

- **Equality:** structural argument (a zero-weight component contributes exactly 0 through
  mean-relative × weight, whether its value is real or zero) pinned by the new
  `gated_queries_match_full_eval` test: 8 seeded random-walk positions (0–28 plies from start),
  gated `eval_4vec` == full-path `compute_utility(run_all_queries(..))`, exact equality.
- **Perf:** identical command to the baseline, same idle box, after the change.

## Results

| Position | nodes (before = after) | nodes/sec before | nodes/sec after | Δ |
|----------|------------------------|------------------|-----------------|---|
| 1 | 1,395,635 | 69,456 | 95,658 | +37.7% |
| 2 | 2,101,082 | 64,337 | 89,986 | +39.9% |
| 3 | 3,101,463 | 63,782 | 90,356 | +41.7% |
| 4 | 6,077,347 | 66,641 | 92,051 | +38.1% |
| 5 | 4,282,640 | 64,318 | 90,777 | +41.1% |
| **median** | — | **64,337** | **90,777** | **+41.1%** |

Node counts and best moves **identical** on all five positions — the trees did not change, only
the per-leaf cost. Suite green (115 lib + 3 integration) including the equality sweep.

## Conditions (after)

- `eval_4vec` skips positional (control + threats + PST) and the king-safety scan while their
  weights are 0; `run_all_queries` still computes everything for tooling. **~41% more nodes/sec**
  on the always-recompute leaf path — this also makes every future self-play/bench run ~1.4×
  cheaper at no measurement cost.
- When the safety rebuild or a positional feature lands with a non-zero weight, its query
  automatically resumes (the gate reads the weight consts) — re-record the perf number then.

## Conclusion

Confirmed, larger than expected: **+41% search throughput for zero behavioral change.** The
zero-weight queries were costing ~30% of every leaf evaluation. Per-leaf eval cost remains the
dominant per-node cost (the always-recompute line projection is the floor); the next perf frontier
is the projection itself, but that is architecture (Hard Rule #5), not waste.
