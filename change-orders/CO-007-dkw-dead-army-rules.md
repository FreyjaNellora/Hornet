# Change Order CO-007

**Date:** 2026-06-12
**Requested By:** User (rule challenge, 2026-06-11) + EXP-026 corpus arbitration
**Target Phase:** Phase 0 — Spec / Reference (§1.7/§1.8) + P2 move generation
**Status:** resolved (user initiated and approved the shift; landed by Fable)

## What Needs Changing

The DKW/dead-army rules in spec §1.7 and the engine (EXP-011's "fully-frozen, swept-on-death"
model) do not match chess.com:

1. **Capturability:** chess.com Help Center — dead pieces "remain on the board and can be
   captured". The engine made them un-capturable walls.
2. **Points:** Help Center — "Capturing dead pieces does not earn points." The engine awarded
   full `ffa_points` for any capture by a live player (including dead-owned pieces inside search
   trees — a latent scoring bug).
3. **Persistence:** the engine swept a fully-eliminated player's army at game flow (and not in
   search — the EXP-011 inconsistency). The user hypothesized "locked after the king falls";
   chess.com is silent → measured by corpus replay.

## Evidence (EXP-026)

Three rule variants replayed over the deduped 140-game human corpus (33,771 plies):

| Variant | Plies | Full games |
|---------|-------|-----------|
| 0 — frozen walls, swept on death (pre-EXP-026) | 24,945 | 72/140 |
| 1 — capturable while walking, locked after death | 23,648 | 61/140 |
| 2 — capturable always, never swept | 24,945 | 72/140 |

Variant 1 **refuted** (real games capture dead pieces after the king falls). Variants 0/2 tie in
replay (replay never exercises game-flow sweep or DKW freezing); the Help Center text decides:
rule 0's sweep contradicts "remain on the board", its freeze contradicts "can be captured".
**Landed: variant 2** (env-overridable `HORNET_DKW_RULE` keeps 0/1 as diagnostics).

## Impact Assessment

- [ ] Cosmetic
- [x] Structural (game-rules fidelity: move-gen capturability, capture scoring, no sweep;
      spec §1.7 normative text)
- [ ] Architectural

This is **rules-correctness** (like CO-003's promotion rank), not an optional strength lever —
Hard Rule #6's default-off discipline does not apply to making the engine play the actual game.

## What Landed

- `board::dkw_rule()` variant switch (default 2); `make_move` awards no points for DKW/dead-owned
  victims; `Board::retire_king` (DKW-stalemate without sweeping); game flow no longer sweeps —
  search and game flow now agree (the EXP-011 inconsistency dissolves).
- `move_gen::is_wall` per variant; the DKW king may capture any adjacent piece including its own
  army (corpus-observed).
- Spec §1.7 rewritten (with history note); VERIFICATION item #5 corrected (dated note — the
  forum-sourced "for points" claim was wrong).
- Eval treatment of persisting dead armies **deliberately unchanged** (they remain counted in
  queries): zeroing them re-creates the EXP-011 king-hunt pathology. Terrain-valuation is a
  future measured arm (noted in EXP-026).

## Resolution

**Landed 2026-06-12.** Suite 115 lib + 3 integration green under the new default; replay floors
unchanged (replay-equivalent); `replay_rules` example added as the standing arbitration
instrument.
