# EXP-014: Piece-Square Tables v0

## Hypothesis
Classical PSTs (centrality + forward bonus for pawns) will provide discriminating signal
between sibling moves, improving Texel MSE on the human corpus where positional play matters.

## Implementation
- Added `PST` static table in `queries.rs`: 6 piece types × 196 squares
- Const-fn helpers: `pst_centrality`, `pst_forward`  
- Player-relative lookup via `pst_value()`: transforms square to Red's perspective
- Wired into `run_all_queries()` as `positional = control + threats + pst`
- King PST = `cent/2` (slight center preference, endgame-oriented)
- No edge penalty (caused negative start position, zeroed out by test)

## Results

### Human corpus (16 games, 855 positions)
| Config | Baseline MSE | Tuned MSE | Weights (M,P,S,O) |
|--------|-------------|-----------|-------------------|
| Without PST (prev best) | 0.11453 | 0.11452 | 4,1,1,1 |
| **With PST v0** | **0.11417** | **0.11354** | **4,5,2,1** |

**Δ = -0.00063 (~0.55% relative improvement)** — best result on human corpus to date.

### Merged corpus (149 games, 6901 positions)
| Config | Baseline MSE | Tuned MSE | Weights |
|--------|-------------|-----------|---------|
| With PST v0 | 0.13252 | 0.13213 | 5,0,0,0 |

Tuner zeroes P,S,O — material dominates in drawish self-play data.

## Analysis
1. **PSTs work on human games.** The positional weight jumped from 1→5, meaning the
   centrality/forward signal is predictive of human outcomes.
2. **Self-play data drowns the signal.** 133 drawish games (150-ply cap, no elimination)
   have no positional gradient — material is the only reliable predictor.
3. **The MSE wall is data-quality, not feature-capacity.** With clean human labels,
   PSTs move the needle. With noisy self-play labels, nothing does.

## Move-match cross-check (claude, 2026-06-07) — CONTRADICTS the MSE read
Ran the MSE-tuned config through the dense move-match instrument (`examples/move_match.rs`). Deployed
the weights temporarily, measured against the 16 human games, reverted `eval.rs` afterward:

| config | move-match |
|---|---|
| default (4,1,1,1), PST weight ≈ off | 149/1270 = **11.7%** |
| (4,1,1,1) + PST at P=1 | 149/1270 = **11.7%** (identical — PST swamped by material at P=1) |
| **tuned (4,5,2,1) + PST** | 123/1270 = **9.7%** |

**The MSE-optimal tune is a move-match REGRESSION** (−2.0%, beyond the ±0.9% noise; ~26 fewer
human-matching moves). MSE and move-match **disagree**, and move-match measures the thing EXP-012 said
matters — the move choice. So the 0.00063 MSE drop is most likely outcome-label overfit on 16 games,
not a play gain; optimizing it made the engine *less* human-like.

**Implications:**
- **Do NOT deploy the tuned weights on the MSE result.** (`eval.rs` left at the default 4,1,1,1.)
- **Gate eval changes on move-match, not MSE.** This is the cross-check working: it caught a regression
  MSE scored as a win. (Confirm any deploy candidate with self-play before shipping.)
- **Hypothesis (not yet isolated): centrality is anti-aligned with 4PC.** A center-seeking PST pulls
  pieces toward the middle — which in 4PC is surrounded by THREE opponents, so centralizing is often
  bad. Up-weighting it chose worse moves. Caveat: P, S, and the PST all moved together here, so this
  isn't isolated to the PST yet — next, test PST-only, and an *anti*-centrality / corner-safety table.

## Next Steps
- **Re-gate PST on move-match** (not MSE): isolate PST-only at a weight that actually changes moves,
  and test whether *anti*-centrality (corner/edge safety) beats centrality for 4PC.
- **PST v1**: Tune individual PST entries via gradient descent (not just weights)
- **Aggression-biased self-play**: Generate decisive games for better training data
- **Strength gate check**: 0.11354 < 0.10? No — still 13% above gate. More needed.

## Files Modified
- `hornet-engine/src/queries.rs`: PST table, `query_pst()`, wiring in `run_all_queries()`
