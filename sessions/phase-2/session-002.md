# Session 002 — Phase 2 (Move Generation) — DKW/dead-army rules corrected by corpus arbitration

**Date:** 2026-06-12
**Agent:** Fable (user-initiated rule challenge)

## Summary

The user challenged EXP-011's fully-frozen DKW rule ("pieces should be capturable while the king
walks, locked after it falls"). Authority check: the chess.com Help Center settles capturability
directly — dead pieces remain, **can be captured, and award no points** (also exposing that
`VERIFICATION-*.md` item #5's forum-sourced "for points" claim was wrong, and that the engine had
a latent in-search scoring bug awarding points for dead-owned captures). The post-death question
is officially unspecified, so the corpus arbitrated it.

## Method + verdict

Three rule variants behind `board::dkw_rule()` (`HORNET_DKW_RULE=0|1|2`), replayed over the
deduped 140-game human corpus (33,771 plies) with `examples/replay_rules.rs`:

| Variant | Plies | Full games |
|---|---|---|
| 0 frozen/swept (pre-EXP-026) | 24,945 | 72/140 |
| 1 capturable-then-locked (user hypothesis) | 23,648 | 61/140 |
| 2 capturable-always, never swept | 24,945 | 72/140 |

**Locked refuted** (real games capture dead pieces after the king falls); 0/2 are
replay-equivalent by construction and the Help Center text decides for 2. **Landed: rule 2.**

## Landed

`make_move` zero-points gate for DKW/dead-owned victims; `is_wall` per variant;
DKW king captures anything adjacent including its own army (corpus-observed);
`Board::retire_king`; game flow no longer sweeps (search/game-flow inconsistency from EXP-011
dissolves); two move_gen tests rewritten to the landed rule; spec §1.7 rewritten + history note
(CO-007); VERIFICATION #5 dated correction; EXP-026. Suite 115 lib + 3 integration green; replay
floors unchanged (replay-equivalence).

## Notes for later shifts

- Eval still counts dead/DKW armies (deliberate: zeroing them re-creates the EXP-011 king-hunt
  pathology). Terrain-valuation = a future measured eval arm.
- Remaining replay gap (68/140 games diverge) is NOT DKW-related — a separate move-gen/notation
  fidelity frontier worth its own diagnostic pass.
- EXP-023 corpus was generated under rule 0; fine for (human-only) tuning, but future bootstraps
  generate under rule 2.
- Same session, eval lane: texel default flipped to human-only (user data-separation principle;
  `HORNET_INCLUDE_SELFPLAY=1` opts in), after human-only fits showed ~8× stronger structure
  signal (EXP-024/025 addenda).
