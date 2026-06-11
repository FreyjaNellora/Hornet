# EXP-012 — going deeper via pruning shape (flashlight vs laser)

- **Date:** 2026-06-07 · **Status:** in progress
- **Goal:** push search depth (toward d16/d20+) inside a ≤10-min/game budget by changing *how* the
  tree is pruned, not by capping time/depth. Test the two strategies separately, then combined, and
  log every variant.

## The two strategies (Shannon 1950)
- **Flashlight = Type A** (wide, bounded *breadth*): keep many lines per level — a level/frontier beam
  of width W, possibly **growing per rotation** to respect the genuinely larger position count deeper.
  Broad coverage; the bigger build (needs a frontier/best-first restructure).
- **Laser = Type B** (selective *depth*): commit to the single best line and drive it deep, cutting
  the rest. Implemented cheaply as **deep beam floor = 1** (`Searcher::with_deep_floor(1)`): rotations
  1–2 keep breadth (base, base/2), rotation 3+ keeps only the top-ordered move → **no deep branching →
  depth ≈ linear, not exponential.** Approximation: trusts move ordering deep (single PV line).

Natural hybrid to test later: flashlight near the root (pick the move on broad evidence), laser deep
(confirm the line cheaply).

## Baseline (full depth, no budget, adaptive base 4, floor 2) — from EXP-010
| depth | per-move | nodes |
|---|---|---|
| 12 | 16.6s | 1.07M |
| 16 | 244s | 17.1M |
| each +4 | ×16 (the four new plies at floor 2 = 2⁴) | |

## Laser test (deep floor 1) — `examples/depth_bench.rs` ✅ 2026-06-07
Midgame (start + 16 random plies), adaptive base 4, full depth, no budget:

| depth | current floor 2 | LASER floor 1 | speedup | laser move |
|---|---|---|---|---|
| 12 | 14.2s / 1.06M | **3.7s / 296k** | 3.8× | h1-h2 |
| 16 | 225.6s / 17.0M | **9.2s / 753k** | 24.6× | h1-h2 |
| 20 | ≈1 hr (est) | **17.1s / 1.42M** | — | j1-i3 |
| 24 | ≈16 hr (est) | **26.9s / 2.22M** | — | j3-j4 |

**Finding 1 — laser makes depth LINEAR.** Current = ×16 per rotation (the four new plies branch at
floor 2). Laser = **+~8s / +4 plies** (additive — the deep part is a single non-branching line). So
d24 is reachable (27s/move) where current would be ~16 hr. d16 laser is ~25× cheaper than current.

**Finding 2 — accuracy isn't obviously lost.** At d16 the wide search and the laser **agree (h1-h2)**,
and the laser already had h1-h2 at d12 — it reached the wide d16 answer for ~25× less work.

**Finding 3 — the move oscillates with depth** (h1-h2 → j1-i3 → j3-j4 at d16/20/24): **search
instability**. Could be genuine deep refinement or the narrow line wandering — only self-play (or a
tactical suite) can adjudicate. This is the signal a self-adjusting controller would react to.

**Per-game budget (≤10 min):** at base 4, d12 laser ≈ 6 min/game (fits), d16 ≈ ~13–15 min (just over),
d20+ over. To push deeper inside 10 min, narrow the R1/R2 base (fewer shallow lines) — a tuning knob.

## Noise-adaptive test (narrow on noisy, broad on quiet) — 2026-06-07
Idea: laser the forcing line on a real tactic (mover in check OR a *favorable* capture, victim > attacker),
flashlight (broad) when quiet. Config: narrow = floor 1, broad = width 6 (`with_noise_adaptive`).

| depth | result | move |
|---|---|---|
| 12 NOISE n1/b6 | **122.8s / 8.98M nodes** (vs laser 4s, current 15s) | f4-j8 |
| 16 NOISE | exploding (killed) | — |

**Finding — "broad on quiet" backfires in 4PC.** It's ~30× slower than laser. Cause: most nodes are
*quiet* by a strict tactic test (4PC has heavy contact but few *favorable* captures), so the broad-6
branch fires constantly → exponential again. The narrow-on-noisy half is fine; broad-on-quiet is the
problem because quiet is the **common** case, not the rare one. To make the idea viable: **bound the
broad** (e.g. 3, not 6), or apply broad **only in rotation 1** (near the root), or loosen the "noisy"
test so fewer nodes go broad — i.e. exactly the SPSA/empirical tuning. Pure laser remains the
cheap-deep winner; broad must be bounded/shallow to be affordable.

## Setting the knobs — approaches to test (user)
1. **Fixed numbers, benched per depth** (mathematically fixed + tested). Above is the start.
2. **Per-rotation growth** of the cap (deeper rotations get a larger budget — they have more positions).
3. **Self-adjusting balance** = a control loop on top: **time management + search-instability detection**
   (widen when the best move is unstable / position sharp; narrow when quiet+decided). The *last* layer.
4. **Empirical tuning ("Texel-style")** = the principled way to set the fixed knobs, but the proper tool
   is **SPSA tuned by self-play win-rate**, NOT Texel-MSE: search shape doesn't *predict outcomes*, it
   changes *which move you play*, judged by *winning*. Same game-data dependency as eval tuning, but
   search-param effects are large → detectable with far fewer games. Proxy until bootstrap: depth +
   tactical test-suite. **Stack: mechanism → SPSA/self-play tuning → self-adjusting control.**

## Flashlight test (level/frontier beam) ✅ 2026-06-07
Built `Searcher::search_flashlight(board, depth, level_cap)`: expand the tree level by level, keep the
top `level_cap` nodes per level (ranked by the mover's own eval gain), then Max^n-back-up over the
kept tree. **Validated**: with a huge cap (no pruning) it reproduces exact Max^n
(`flashlight_matches_maxn_without_pruning` test). Default-off (the engine uses `search`).

| depth | LASER f1 | FLASH cap 1000 | FLASH cap 3000 |
|---|---|---|---|
| 12 | 4.1s / 296k — h1-h2 | 7.9s / 252k — h1-h2 | 24.5s / 776k — h1-h2 |
| 16 | 9.8s / 753k — h1-h2 | 11.4s / 378k — h1-h2 | 33.8s / 1.10M — h1-h2 |
| 20 | 18.2s / 1.42M — **j1-i3** | 15.0s / 495k — h1-h2 | 41.3s / 1.39M — h1-h2 |

**Finding 1 — flashlight is linear in depth too** (cap-1000: +~3.5s/+4 plies, flatter than laser), and
at d20 it's *faster* than laser (15s vs 18s). The level cap bounds cost; depth just adds capped levels.
**Finding 2 — flashlight is MORE STABLE than laser.** Cap-1000 holds h1-h2 at d12/16/20; the laser
wandered to j1-i3 at d20. The wide search (floor 2) also picked h1-h2 at d16, so the flashlight's broad
coverage tracks the trustworthy move while the laser's single narrow line drifts deep. Cost note:
flashlight evals *every* candidate (`cap × branching × depth`), so per-move it's pricier than laser
(d16: 11s vs 10s; d12: 8s vs 4s) — laser is cheapest for bulk, flashlight is steadier for move quality.
**Verdict:** both viable. Laser for max throughput (bootstrap); flashlight for stable deep evaluation.
Which actually *plays better* is the SPSA/self-play question (needs the bootstrap games). **Decision:
flashlight is the default deep mechanism** (linear + move-stable + validated; the laser wanders deep).

### Per-rotation GROWING cap — tested, refuted (2026-06-07)
`search_flashlight` now takes a per-level cap schedule (`cap_at: Fn(u32)->usize`). Tested a growing cap
(`base 500 << rotation`, ceiling 8000 — "respect more positions deeper") vs the fixed cap 1000:

| depth | FLASH fixed 1000 | FLASH grow 500→8000 |
|---|---|---|
| 12 | 7.8s — h1-h2 | 9.8s — h1-h2 |
| 16 | 11.6s — h1-h2 | 19.6s — h1-h2 |
| 20 | 14.9s — h1-h2 | 36.9s — h1-h2 |

**Growing the cap loses strictly:** 1.3–2.5× more cost (the gap widens with depth — growth piles width
into the deep rotations, which dominate the node count) for the **same move** at every depth. The deep
extra breadth buys nothing; the fixed cap already has enough breadth to find the move, and deep lines
are speculative. **Decision: fixed cap, not growing.**

### Cap-sufficiency study — is cap a function of depth? (`examples/cap_sufficiency.rs`, 2026-06-07)
For 4 random-opening positions × depths {8,12,16}, the **min level-cap whose move == the cap-1600 move**:

| position | d8 | d12 | d16 | move-vs-cap [50,100,200,400,800,1600] |
|---|---|---|---|---|
| seed 1 | 400 | 400 | 400 | h2-h3, d3-c5, d3-c5, **k1-k2**, k1-k2, k1-k2 |
| seed 2 | 1600 | 1600 | 1600 | d3-e1, d3-e1, g2-g3, g2-g3, g2-g3, **g2-g4** (not converged) |
| seed 3 | 1600 | 1600 | 1600 | (same as seed 2) |
| seed 4 | 800 | 800 | 800 | h1-h2, h1-h2, g2-g4, g2-g4, **g2-g3**, g2-g3 |

**Finding 1 — cap need is DEPTH-INDEPENDENT.** `min_cap` is identical at d8/d12/d16 for every position
(and the whole move-vs-cap sequence is identical across depths). So there is no cap(depth) to math out
— a deeper search does **not** need a wider cap. The cap is set by the **position's sharpness**, not
the depth (400 / 800 / 1600+ across positions). → the right model is **position-adaptive** (widen only
when the root choice is close/unstable — the self-adjusting idea), or a fixed cap ≥ ~1600 to cover
sharp positions (cap 1000 picks a different, less-converged move in seeds 2–4).

**Finding 2 — with the current eval, DEPTH doesn't change the move; BREADTH does.** At a fixed cap the
move is the same at d8/d12/d16; it only changes as the *cap* grows. So searching deeper buys nothing
for the decision here — re-confirming EXP-001: **the eval is the bottleneck, not depth.** The
laser/flashlight depth machinery is ready, but won't pay off until the eval rewards depth. Strategic
read: priority is the **eval** (data-gated → bootstrap corpus) + enough near-root breadth, not deeper
search. (Caveat: 4 positions, move-match metric, one eval — a self-play win-rate is the harder test.)

### Recommended broadening method — CONFIRMED (`examples/beam_shape.rs`, 2026-06-07)
On the sharp positions (seeds 2,3 @ d12), widening **only rotation-1** (floor-2 deep) vs a uniform
wide cap:

| shape | time | nodes | move |
|---|---|---|---|
| uniform 1600 | 12.3s | 390k | g2-g4 |
| **root-wide 1600 / floor-2 deep** | **3.3s** | 102k | g2-g4 |

Same move, **3.7× cheaper.** So near-root breadth is the entire lever; deep breadth is waste. **Final
method for broadening the beam:**
- **Where:** widen **rotation-1 only; floor the deep beam at 2.** (Confirmed: same move as uniform-wide.)
- **How much / how fast:** **iterative widening** of the rotation-1 cap (×2: 100→200→…→1600) until the
  root move is stable two steps running — the breadth analogue of iterative deepening. Auto-sizes per
  position (cheap when robust, wide only when sharp).
- **Per depth / rotation:** **flat — do not grow with depth** (cap need is depth-independent; growth
  refuted).
- **Limit:** (1) **convergence** — widen until the move stops changing = you've matched exact Max^n for
  that position (full width is the ceiling, but convergence arrives well before it); (2) **cost** —
  rotation-1 cap ≤ ~1600–3000 keeps a ≤10-min game; sharp positions can hit this before converging, so
  accept best-so-far there.

## Open questions to test (pragmatic, one knob at a time)
- Does laser (floor 1) reach much deeper at equal cost? Does it pick a *different* (better?) move than
  the wider current search — i.e. does the extra depth buy anything, or does the narrowness hurt?
- Per-rotation **growth** of the cap (deeper rotations get a bigger budget) vs flat cap — which holds
  up better?
- Hybrid (flashlight R1, laser deep) vs pure laser vs pure flashlight.
- Quiescence interaction: left alone for now; account for its cost in the per-depth timing.

## Notes / changelog
- 2026-06-07: added `Searcher::deep_floor` (config; 1 = laser, 2 = default) + benched. Beam is a hard
  per-node cap (EXP from the budget-removal work); this knob sets only the deep floor.
