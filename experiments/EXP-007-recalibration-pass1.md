# EXP-007 — recalibration pass 1 (gate not yet passed)

- **Date:** 2026-06-06
- **What changed:** `queries.rs` — `query_crossfire` rewritten to SEE-resolved material-at-risk
  (bounded by victim value), `safety_scalar` rewritten to centipawn danger (`attack_value` folded,
  clamped ±600). `eval.rs` **unchanged** (weights still `4/2/1/1`; bounty still folded into Oᵢ).

## Calibration gate (EXP-006 metric)
```
              quiet swing        capture swing
baseline      avg 1294 max 3506  avg 5189 max 13691
pass 1        avg 1172 max 3917  avg 2816 max 8511
target        avg ~tens          ≤ ~900
```
**Not passed.** Captures roughly halved; quiet barely moved.

## What improved (real)
Crossfire (SEE-bounded ≤ victim) and safety (clamped) are sound. Match 0/13 → **2/13** (S05, S21,
gap 0); **blunders 1 → 0** (the SEE-bounded crossfire stopped the engine capturing into a loss);
missed-wins → 0.

## Why it still swings (per-component diagnostic)
`gate_ablation.rs` now prints the raw per-query delta (M/P/S/O) for the mover on each human move:
```
[S02] xQ  swing=8511 | rawΔ M=0 P=368 S=-45 O=300   (queen TRADE; self-Δ ~+400)
[S01] xN  swing=4116 | rawΔ M=0 P=192 S=-80 O=900
[S03] qt  swing=596  | rawΔ M=0 P=350 S=-98 O=0     (≈ P×2 — positional alone)
[S17] qt  swing=2910 | rawΔ M=0 P=362 S=-27 O=0     (self-Δ ~700; swing 2910)
[S10] qt  swing=27   | rawΔ small                   (fine)
```
1. **eval.rs untouched.** Weights `4/2/1/1` + bounty (`ffa_points`) still in Oᵢ — two of the pitch's
   four items not done. The new centipawn components are combined with old-scale weights.
2. **Positional (P) is the remaining unbounded term.** Centrality reach + the *flat* `query_threats`
   (`target/4`, uncapped) swing P by ~350 per quiet move; at `W_POSITIONAL=2` that's ~700 (S03 is
   almost entirely this).
3. **Mean-relative coupling amplifies.** Swings exceed the mover's *own* weighted deltas (S17: 2910 vs
   ~700; S02: 8511 vs ~400) — centering on the per-component mean means a move that changes several
   players' components (mover's P/threats + every attacked player's O) compounds through the shared
   means. Captures swing worst (two players change at once).

## Next (still Kimi's lane — `eval.rs` + `query_threats`/positional)
1. **Finish eval.rs:** re-derive weights for the new centipawn scales (drop M from 4, P from 2 — the
   components are now comparable, so ~1/1/1/1 is the starting point); lift the `ffa_points` bounty out
   of Oᵢ (it's still there).
2. **Bound the positional term:** the flat `query_threats` (`target/4`) is uncapped and the biggest
   quiet-move swing source — cap it, or replace it with the SEE threat (already implemented behind
   `HORNET_SEE`), or scale it down.
3. The mean-relative amplification is inherent to the zero-sum coupling; the lever is keeping each
   component's per-move delta small (bounded) — P is the last unbounded one.

Re-run this gate after each step; quiet swing should fall toward ~tens (positionally-neutral) with
captures bounded by piece value.
