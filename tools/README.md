# tools/ — Python diagnostics for the engine

Shared analysis tools (numpy / scipy / scikit-learn / pandas) for both claude and kimi. They read CSVs
the engine exports — they don't reach into the Rust. Use **Python 3.11** (the interpreter the packages
are installed in): `py tools/<script>.py`, or `py -3.11 tools/<script>.py` if `py` picks the wrong one.

## fit_weights.py — proper eval-weight fit **with confidence intervals**
The Rust `texel_tune` hill-climb gives a point estimate but can't tell a real weight from noise. This
fits the same sigmoid-MSE objective with a real optimizer (scipy) and **bootstraps a 95% CI per weight**.
A CI that excludes 0 is a genuine signal; one that straddles 0 is noise — the question we keep hitting.

1. Export data (one row per position×player):
   `HORNET_HUMAN_ONLY=1 HORNET_DUMP_CSV=1 cargo run --release --example texel_tune`
   → writes `tools/texel_positions.csv`. Drop `HORNET_HUMAN_ONLY` to include self-play games.
2. Fit + bootstrap: `py tools/fit_weights.py [n_bootstrap]` (default 200)

**Latest (32 human games, corrected labels):** `M=4.34 [4.06,4.65]*`, `P=1.24 [-0.17,2.49]` (noise),
`S=-1.11 [-2.02,-0.06]*` (significantly **negative** — current safety hurts), `O=1.03 [0.59,1.55]*`.
Caveat: row-bootstrap (block-by-game would be stricter), so treat borderline as suggestive until the
corpus grows.

## ab_stats.py — self-play A/B significance
`py tools/ab_stats.py WINS GAMES [A_POINTS B_POINTS]` → win-rate 95% CI, p-value vs 50%, and the number
of games needed for 80% power. Keeps small-sample A/B honest.
**Example:** `py tools/ab_stats.py 2 6 214 282` → 33%, CI [8%,71%], p=0.69 (not conclusive; ~69 games
needed). So "depth lost" is directional, not proven, at n=6.

## perf_breakdown (Rust example, in hornet-engine/examples)
`cargo run --release --example perf_breakdown` → where search time actually goes. Run on an idle machine.

**Latest:** `eval_4vec` = **11.0 µs/node**, of which the **line projection (`compute_lines`) is 7.3 µs
(66%)** — and it's recomputed from scratch every node (Hard Rule #5). Move-gen 5.1 µs. Eval is ~**35% of
total search time**; the other ~65% is move-gen + the flashlight's board-clones + pruning-sort + backup.
→ **biggest latency lever = an incremental line projection** (update on make/unmake instead of a full
recompute), then fewer nodes (cap/pruning). For a flame graph: `cargo install flamegraph` (Windows needs
a sampler — `blondie`/WSL).

## conventions
Keep each tool small and single-purpose; consume an exported CSV; never embed engine logic. The Rust
side gates data dumps behind env flags (`HORNET_DUMP_CSV`, `HORNET_HUMAN_ONLY`).
