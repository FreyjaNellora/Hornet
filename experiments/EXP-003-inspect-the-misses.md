# EXP-003 — inspect the 13 gate misses

- **Date:** 2026-06-06
- **Hypothesis:** before adding more eval features, look at what the 13 fixtures actually require —
  are the misses capture/threat-shaped (→ exchange-aware eval) or quiet/positional (→ something else)?
- **Lever / change:** none — instrumentation only. `gate_ablation.rs` now prints, per fixture, the
  human move and the engine move, each classified (capture-of-what / quiet / promotion).

## Conditions (before)
Engine: depth 4, beam 10 + LMR + adaptive, 800k node budget, **baseline eval (SEE off)**. 13 fixtures.

## Method
`cargo run --release --example gate_ablation` (now the inspector). Per fixture: replay → decode human
move → search → print both moves + capture classification + a capture/quiet tally.

## Results

```
[S01] HUMAN Qf6-c6 xN(300)   | ENGINE Pe2-e3 quiet     miss
[S02] HUMAN Qd7-g4 xQ(900)   | ENGINE Qd7-d5 quiet     miss   (declines a queen capture)
[S03] HUMAN Qj4-j9 quiet     | ENGINE Ne1-d3 quiet     miss
[S04] HUMAN Nd3-e5 quiet     | ENGINE Pj2-j3 quiet     miss
[S05] HUMAN Qj4-g4 quiet     | ENGINE Ni3-h1 quiet     miss
[S06] HUMAN Bm8-h3 xP(100)   | ENGINE Bm8-l9 quiet     miss   (same bishop, retreats vs grabs pawn)
[S07] HUMAN Qk7-f2 quiet     | ENGINE Qk7-g3 quiet     miss   (same queen, diff square)
[S10] HUMAN Ka7-a5 quiet     | ENGINE Qd7-d6 quiet     miss
[S17] HUMAN Qg1-j4 quiet     | ENGINE Qg1-i3 quiet     miss   (same queen, diff square)
[S18] HUMAN Qg9-m9 xP(100)   | ENGINE Qg9-d12 xR(500)  miss   (engine wins MORE material)
[S21] HUMAN Qi5-m5 xP(100)   | ENGINE Pg2-g4 quiet     miss
[S22] HUMAN Qh14-f12 quiet   | ENGINE Qh14-c9 xP(100)  miss   (engine grabs a pawn)
[S23] HUMAN Rd14-d12 quiet   | ENGINE Qf6-f8 quiet     miss
--- 0/13 | human: 5 capture / 8 quiet | engine: 2 capture / 11 quiet ---
```

## Conditions (after)
No engine change (instrumentation only). `gate_ablation.rs` is now the per-fixture inspector; the
match-rate matrix (`run_config`) is retained `#[allow(dead_code)]`.

## Conclusion
The 0/13 is **mostly a positional problem and a metric problem, not a tactical one**:
- **8/13 human moves are quiet** → exchange-awareness can't touch the majority. This is *why* EXP-002
  (SEE threats) was null; SEE was the wrong bet for this fixture set.
- **The engine is not tactically blind** — S18 wins a rook, S22 grabs a pawn. It captures when material
  is on offer.
- **Dominant miss shape: "same piece, different reasonable square"** (S02/S06/S07/S17/S18) — a
  positional judgment gap (where to put the piece), not a hung-material blunder.
- **Exact-move match is too harsh.** S18's engine move looks *materially better* than the human's
  (rook vs pawn) yet is a "miss." 0/13 conflates "blundered" with "played a different good move," so it
  cannot tell us the engine is weak.
- **One real check: S02** — engine declines a queen capture (`Qd7-g4` vs quiet `Qd7-d5`). Defended-trade
  decline (fine) or genuine miss (bug)? Resolve directly.

## Next
- **EXP-004 — a blunder/quality metric, not exact match.** For each fixture compute: does the engine's
  move *lose* material vs the human's (or vs best), and is the human move a *winning* tactic the engine
  ranked below its choice? That separates "engine is weak" from "metric is harsh," and tells us whether
  a strength problem even exists here.
- The high-leverage eval work is **positional** (control / king-safety / piece placement — the
  intent/zones substrate), not SEE. SEE stays default-off, ready if a tactical metric later shows misses.
- Resolve S02 specifically (is the queen capture winning?).
