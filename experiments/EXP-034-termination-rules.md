# EXP-034 — FFA termination rules: threefold repetition + 50-move (the tester-readiness fix)

- **Date:** 2026-06-13 · **Status:** LANDED (repetition + 50-move; insufficient-material deferred)
- **Why:** the engine had **no draw/termination rules** — games only ended by elimination or an
  arbitrary ply cap (`play.rs` 400, self-play 140). A human tester could shuffle or perpetually
  check forever and the engine would never claim the draw, and long tester games ended by cap
  instead of by rule. Needed before distributing `play.exe` to testers.

## Rules verified (chess.com Help Center — the project's trusted source, same as the DKW rules)

| Condition | Outcome |
|---|---|
| Threefold repetition | game ends, **+10 to each remaining (alive or zombie) player** |
| 50-move rule (no capture / no pawn push) | same: **+10 each remaining** |
| Insufficient material | same: **+10 each** *(deferred — see below)* |
| Stalemate | +20 to the stalemated player *(already modeled as DKW entry; see flag)* |
| Checkmate | +20 to the checkmater, then DKW *(already modeled)* |

Sources: [4PC Help Center](https://support.chess.com/en/articles/8614233-4-player-chess-4pc),
[4PC FFA Basics](https://www.chess.com/blog/TheCheeseDuck/4-player-chess-ffa-the-basics).

## Implementation (engine-only, `game.rs`; query-based so existing consumers are untouched)

- `Game` gains `history: Vec<u64>` (Zobrist key after every turn, incl. the start) and
  `halfmove_clock: u32` (plies since the last capture or pawn move). Both updated by a private
  `record(reset_clock)` called at **every** `step` return path (live move, DKW walk, DKW-entry,
  DKW-stalemate removal, pass).
- `Game::draw_status() -> Option<DrawReason>` — pure query: `Repetition` when the current Zobrist
  key appears ≥3× in `history`; `FiftyMove` when the clock reaches the threshold.
- `Game::claim_draw()` — awards **+10 to each non-`dead` player** (alive or zombie, per the Help
  Center wording) and returns the reason. Mirrors the existing DKW-stalemate +10 logic.
- **Design choice — opt-in via query, not baked into `step`.** `step`'s return values and game
  flow are unchanged, so self-play (`selfplay_ab`), mining, and the tuner see **zero behavior
  change**. Only `play.rs` (the tester loop) calls `draw_status`/`claim_draw` and ends the game,
  writing the reason into the report's `[Termination]` header. (Wiring draw termination into
  self-play is a future opt-in, deliberately not done here while a gate was running on the old
  binary.)
- Repetition uses the **Zobrist key**, which includes squares, side-to-move, castle/EP rights,
  and dead/DKW flags but **excludes points** — correct: repetition is a board-configuration rule,
  and points differing doesn't make it a different position.

## Open / deferred / flagged

- **50-move counting unit is UNCONFIRMED.** The Help Center states the rule and its +10 outcome
  but not whether "50 moves" means 50 plies or 50 full rounds. `Board::extra` (FEN4 field 6) is
  "the lone counter" and may hold chess.com's clock, but we have no mid-game FEN4 to confirm its
  grammar. Chose `FIFTY_MOVE_PLIES = 200` (= 50 rounds) — **conservative**: it never draws a
  still-progressing game early, and threefold repetition catches real shuffles far sooner.
  Single named constant; tune once verified against live play.
- **Insufficient material deferred** — genuinely complex in a 4-army, non-zero-sum, points-scoring
  game (when are 4 partial armes "insufficient"?). Repetition + 50-move already stop infinite
  tester games; insufficient-material is a rare tail.
- **Stalemate scoring flag:** the Help Center awards a stalemated player **+20**, and the engine
  already does this for a LIVE player stalemated (`game.rs` step, `!in_check` → +20). Consistent.
  No change. (Noting it here for the record; not a defect.)

## Verification

- Suite green: **119 lib** (+2: `threefold_repetition_detected_and_scored`,
  `halfmove_clock_resets_on_pawn_and_capture`) **+ 1 variant + integration**.
- `threefold_repetition_detected_and_scored`: 4 kings toggling between two squares each; the start
  position recurs every 8 plies, and the test asserts the draw fires at **exactly ply 16** (third
  occurrence) and that `claim_draw` adds +10 to all four.
- `halfmove_clock_resets_on_pawn_and_capture`: clock advances on quiet king moves, resets to 0 on
  a pawn push and on a knight capture.
- Deployed eval / self-play unchanged (no `step` behavior change; `Game` only gained tracking +
  query methods). `play.rs` ends on a draw and records it in the debug report.

## Conclusion

The two unambiguous, high-value FFA draw rules now end tester games correctly with the right
scoring. The engine is no longer shufflable-to-infinity, and `versus_games/` reports carry the
real termination reason. The one soft spot (the 50-move ply/round unit) is isolated to a single
documented constant and flagged for live confirmation.
