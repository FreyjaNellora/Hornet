# EXP-009 — Texel tuning (the real fine-tune metric) + outcome-prediction "vs"

- **Date:** 2026-06-06
- **Built:** `examples/texel_tune.rs` — Texel tuning (Österlund 2014) adapted to 4PC. Labels every
  corpus position with the game's eventual **per-player placement** (from the PGN4 `[Result "R: pts -
  B: pts - Y: pts - G: pts"]` tag → rank-based target in [0,1]), maps each player's eval component
  through a sigmoid, and minimizes MSE vs the actual outcome. Queries are cached per position, so the
  fit loop is pure arithmetic — **the whole tune runs in seconds** (no search). See
  `REFERENCE-eval-tuning.md` for why this is the classical method.

## Result
```
dataset: 855 positions, 16 corpus games
fitted K = 0.0002
baseline weights 4/1/1/1   MSE = 0.11456
tuned    weights 4/1/2/1   MSE = 0.11448   (improvement −0.00007 = noise)
chance (predict 0.5 for all) MSE ≈ 0.14
```

## Findings
1. **The eval has real predictive power** — MSE 0.1146 vs chance 0.14. The recalibrated v0 eval does
   rank 4PC outcomes better than a coin flip.
2. **The 4 top weights are already optimal.** Texel can't improve them (only nudges safety 1→2 for
   −0.00007). Weight-tuning is exhausted — this is the principled confirmation of EXP-008's "match
   rate bounced because weights are near-optimal."
3. **Positional stays low / safety slightly up** — consistent with EXP-007 (the flat `query_threats`
   positional term carries little outcome signal). Texel agrees the structure, not the weights, is
   the lever now.

## The "vs" (comparing results)
`texel_tune` reports **outcome-prediction MSE for any weight config** — that IS the A-vs-B comparison
at the eval level (lower MSE = better predictor), and it replaces the exhausted/ noisy move-match
rate. To compare a new eval *variant*, run it through the tuner and read the MSE. (Gold-standard
self-play A-vs-B is the true strength test but expensive — a full 4PC game is ~100-176 plies × a
search per ply, so dozens of games = hours; deferred until a game loop exists.)

## What this means for fine-tuning
The eval weights are done. **Further gains are in the eval's *features* (what the queries measure),
not the linear weights** — and the Texel pipeline now exists to tune them against outcomes the moment
they're exposed as parameters. The next productive eval work:
- richer/better query features (the flat positional term is weak; better king-safety, pawn structure,
  zone control), each validated by Texel MSE;
- eventually a linear feature eval with many tunable terms (the NNUE precursor), tuned by Texel.

Not deploying 4/1/2/1 (the gain is noise; keeping 4/1/1/1). Suite green (96); lib unchanged this pass.
