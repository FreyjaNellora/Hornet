# EXP-010 — self-play harness (the gold-standard "vs")

- **Date:** 2026-06-06
- **Built:** `examples/selfplay.rs` — drives four `Searcher`s through a 4PC game from the start
  position, each seat playing its own config, scoring by accumulated FFA points (`board.points`). To
  compare config A vs B, **A is rotated through all four seats** (B fills the rest) to cancel seat
  bias. Unblocked by the protocol work (P8).
- **Hypothesis under test (first run):** does quiescence (TRS) win games on the recalibrated eval?

## Conditions
- A = `q-on`, B = `q-off`; both depth 4, beam 16, forward-pruning + adaptive on, 300k node budget.
- 100-ply cap (a 32-ply cap ended in the opening with `[0,0,0,0]` — no captures, no signal).
- Release build, `CARGO_TARGET_DIR=C:\rust-target\hornet-p6test`.

## Results
Game 1 (A=q-on in seat 0): points `[3, 25, 3, 12]` → A (q-on) = 3, the q-off seats = 25 / 3 / 12.
(Full 4-seat rotation runs in the background; **one game is pure noise — do not conclude from it.**)

Two things this already shows:
1. **The harness works** — points accumulate, the rotation runs, the comparison mechanism is sound.
2. **Seat bias is real and large** (seat 1 scored 25 vs others' 3–12 in one game) — which is exactly
   why A is rotated through every seat. It also means *many* games are needed to see through it.

## Speed tuning — depth-12 made manageable (SUPERSEDED — see "Depth cost" below)
**Superseded 2026-06-07.** The node budget below was wrong: it cuts *mid-rotation* (some branches at
depth d, others dropped to a static eval), mixing depths and violating Hard Rule #1. Replaced by a
**hard per-node branch cap** (the fixed flat/adaptive beam — `search.rs`) so every branch reaches the
same depth, no budget. The original (flawed) budget notes:

The node budget caps per-move work, so nominal depth becomes a *ceiling* (the search reaches whatever
fits in the budget, never runs away). Tuning it down:

| config | per-game |
|---|---|
| depth 4, beam 16, budget 300k | ~6 min |
| **depth-12 ceiling, beam 10, budget 150k** | **~3.7 min (220s)** |

So a **depth-12 game is now faster than the depth-4 baseline** and below the ~4-min target — deeper
ceiling, less wall-clock. The per-node cost is fixed by design (line projection is always recomputed
from scratch — no incremental indices), so the only levers are **node count** (budget + beam), not
per-node speed. Lower the budget further for even faster games at the cost of effective depth.

## Depth cost — full rotations, hard beam cap (current)
Per-move cost from a midgame, **full depth, no budget** (every branch a clean rotation):

| depth | flat beam 2 | adapt base 4 |
|---|---|---|
| 8  | 0.14s | 0.87s |
| 12 | 2.4s  | 16.6s |
| 16 | 38s   | 244s  |

**Each +4 depth ≈ ×16** — the four new plies sit at the deep beam floor of 2 (2⁴). Per ~100-ply game
(openings instant): d8 ~15s; d12 ~4 min (flat 2) / ~28 min (adapt 4); **d16 ~1 hr (flat 2) / ~7 hr
(adapt 4)**. FLAT 2 is the deep config — adapt 4's wider root costs ~7× for little gain at depth. d16
~38s/move is near the floor: beam 2 is the minimum width that still gives a real Max^n vector (beam 1
= a single PV line), and per-node cost is fixed by always-recompute line projection (Hard Rule #5), so
d16 won't speed up without shallow pruning (deferred) or a faster eval. **Bootstrap: d8 bulk, d12
flat-2 quality, d16 occasional**; d20 is another ×16 (out of reach). Bench: `examples/depth_bench.rs`.

## Cost & cadence (tractable)
~6 min/game at depth 4 + 100-ply — **good for a 4PC engine** (full games are 100+ plies × a search per
ply; this beats prior attempts at the same job). A multi-rotation comparison (dozens of games) is a few
unattended hours in the background, so **self-play is a practical strength metric for Hornet**, not a
someday-ideal. Workflow: **Texel MSE (EXP-009) as the fast pre-filter** for eval variants, **self-play
to confirm** a real strength change. 4PC games are high-variance + seat-biased (game 1 alone: 3 vs 25),
so run enough rotations to see through it.

## Conclusion
- **Infrastructure delivered.** Self-play A-vs-B exists; it's the venue the dead match-rate metric
  couldn't be — the place to finally re-test **depth** (EXP-001) and **the speed levers**
  (quiescence/LMR/adaptive) in *real play* rather than move-matching.
- **Worth running for real now** (it's tractable): many rotations of `q-on vs q-off`, then
  `depth8 vs depth4`, before trusting a verdict — a single rotation is still noise.
- **Scope caveats:** plays to a ply-cap or a stuck player (king-capture elimination is handled by the
  engine's rotation; a *true* game-to-the-end needs DKW move-gen, next item). Compares **search**
  configs (runtime-settable); **eval-weight** comparison needs runtime weights (an `eval.rs` refactor),
  deferred — for eval variants, use `texel_tune` (EXP-009).
