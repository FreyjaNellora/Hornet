# EXP-028 — replay-gap forensics: DKW own-capture inference (+32 fully-replayed games)

- **Date:** 2026-06-12
- **Hypothesis:** the post-EXP-026 replay gap (68/140 games diverging for non-DKW-rule reasons)
  has classifiable causes; the largest single cause was suspected to be corpus DKW kings
  capturing their **own** pieces, which the replayer refuses because it has no game flow and
  never learns the mover is DKW.
- **Lever / change:** `replay_rules` gained `HORNET_REPLAY_VERBOSE=1` failure classification and
  a **DKW inference**: a live king can never capture its own piece, so observing one in the
  corpus *proves* the mover is DKW — `enter_dkw(mover)` and retry. Instrument-only change.

## Results

Failure classification (140 deduped human games), before inference:

| Cause | Games |
|-------|-------|
| king-own-capture (DKW walk, unflagged) | **32** |
| empty-from (earlier silent state divergence) | 25 |
| castle | 5 |
| queen/pawn/knight/promotion tail | 6 |

Coverage with the inference: **27,155/33,771 plies; 104/140 games fully** (from 24,945 / 72 —
all 32 own-capture games recovered, +2,210 plies).

## Pass 2 (same day): castle misattribution — the empty-from cause

The last-applied-tokens trace showed the empty-from failures clustering after `O-O`/`O-O-O`
tokens: **a castle token names no player**, and the replayer tried players in fixed RBYG order —
whenever two players could castle the same side, the castle was applied to the wrong one
(usually Red, tried first), silently rearranging their back rank; every later reference to that
player's king/rook then failed. Fix: track the rotation (`last_mover`) and resolve castle tokens
in rotation order from the expected next mover.

**Coverage: 32,736/33,771 plies (96.9%); 133/140 games fully** (72 → 104 → 133 across the two
passes).

Engine fix found en route: `gen_castles`' through-check test counted DKW armies as attackers
(raw `is_attacked_by`), phantom-blocking castles chess.com allows — now mirrors `in_check`
(dead → nothing; DKW → king-adjacency only; live → everything). Correct for play; doesn't move
the *replay* number because the replayer only learns DKW status from own-capture inference.

## Conditions (after)

- Replay coverage: **133/140 fully (95%), 96.9% of plies**. The 7-game tail, fully classified:
  4 castle failures where the relevant player went DKW via *checkmate* (the replayer can infer
  DKW only from own-captures; a checkmate-detection inference would close these), 2 pawn-capture
  quirks, 1 empty-from. Diminishing returns; recorded for a future pass.
- **Propagation pending:** the two replayer fixes (DKW inference, rotation-aware castles) live in
  `replay_rules` only. `tests/pgn4_replay.rs`, `texel_tune`, `move_match`, and `move_diverge`
  carry the old replay logic — propagating raises the test floors substantially and **grows
  every instrument's position sample (re-baseline move_match when that lands)**.

## Conclusion

The replay gap was almost entirely the instrument, not the engine: DKW flag inference (+32
games) and rotation-aware castle resolution (+29 games) took coverage from 51% to 95% of games.
One real engine fidelity bug surfaced (DKW phantom-attacks blocking castles — fixed). The corpus
is now nearly fully harvested as validated move-geometry ground truth.
