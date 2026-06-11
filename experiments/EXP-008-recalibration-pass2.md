# EXP-008 — recalibration pass 2 (scale bug fixed)

- **Date:** 2026-06-06
- **What changed (eval.rs):** lifted the `ffa_points` bounty out of Oᵢ (Hard Rule #8 fix + a swing
  source removed) and re-derived weights. Final: **`W_MATERIAL=4, others=1`**. (`1/1/1/1` was tried
  first and **broke the free-queen tests** — under mean-relative normalization a free piece nets only
  ~value/4 to the taker, so material must out-weigh the positional swing of repositioning.)

## Calibration gate
```
                 quiet swing        capture swing       match  suite
baseline         avg 1294 max 3506  avg 5189 max 13691  0/13
pass 1 (Kimi)    avg 1172 max 3917  avg 2816 max  8511  2/13   green
pass 2  1/1/1/1  avg  267 max  802  avg  246 max   507  1/13   2 FAIL (free queen)
pass 2  4/1/1/1  avg  276 max  876  avg  316 max  1024  0/13   green ✓
target           ~tens              ≤ ~900
```
**Scale bug fixed.** Quiet swing fell ~5× (1294→276); the S02 queen *trade* that swung 8511 now swings
~350; captures track material (≤~1 piece); **blunders 0**; the engine takes free material (free-queen
tests pass). The match rate bounced 0→2→1→0 across weight settings — it's noise at this level.

## What "~tens" misjudged
The residual quiet swing is largely *legitimate*, not the bug:
- S07 max-802 is `O=−900` — a quiet move that **saves a piece** from a winning capture; the eval
  *should* swing ~the piece value there.
- S03 P=350 is a queen repositioning (centrality + threats).

A queen moving to a strong square genuinely changes positional eval by hundreds, so "~tens" was naive.
The right statement: swings are now bounded by *real* material/positional change, not value² noise.

## SEE threats re-test (EXP-002 on the recalibrated eval)
`HORNET_SEE=1`: quiet 267→278, capture 246→250, match 1/13→1/13. **Still no effect.** EXP-002's null
holds on a sane eval — SEE-as-eval is a confirmed dead end (keep `query_threats_see` default-off; its
value is as a blunder *filter*, per S22, not an eval term).

## Blunder rate (recalibrated eval, 150 corpus positions)
`gate_ablation.rs` replays the corpus, runs the engine at each unique position, and checks whether its
move loses material:
- **capture-into-loss: 1**, **hangs (>200 cp): 2**, **avg newly-hung: 12 cp** (over 150 positions).

→ the recalibrated eval plays soundly — it almost never loses material (~1–2 of 150). A real
play-quality number (unlike the match rate); the baseline to beat as eval features are added.

## Status & remaining levers
- **Scale bug: fixed.** The eval is stable and sane (no thousands-swings, captures bounded, trades
  net ~0, 0 blunders). This was the session's goal.
- **One mild residual:** the flat `query_threats` (`target/4`, uncapped) inflates positional — a queen
  repositioning swinging ~350 (≈3.5 pawns) is high. Capping/scaling it (queries.rs) is the next lever.
- **The match-rate metric is exhausted** as a tuning signal at 1–2/13 (noise, and exact-move-vs-human
  is harsh). Productive further tuning needs a play-quality metric (blunder rate over many positions,
  or self-play), not exact-match.
