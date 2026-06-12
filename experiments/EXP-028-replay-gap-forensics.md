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

## Conditions (after)

- Replay coverage instrument: 104/140 fully (74%), 80.4% of plies. Remaining 36 games:
  25 empty-from (the board state silently diverged earlier — suspects: EP victim-square or
  castle rook-placement side-effects differing from chess.com), 5 castle-matching failures,
  6 tail. **Next forensics pass:** for empty-from games, log the last applied token before
  divergence and diff the board against expectation; for castle, dump the castling state at the
  failing ply.
- The `pgn4_replay` integration-test floors (baselines corpus) remain valid; raising them to
  the new coverage level is deferred until the inference lands in the test replayer too.

## Conclusion

The biggest single replay-fidelity gap was the instrument, not the engine — the corpus-observed
DKW own-capture rule (landed in EXP-026) plus flag inference recovers a third of all failing
games. The remaining empty-from class is the next move-gen/notation fidelity frontier and likely
worth another +10–20 games of usable corpus.
