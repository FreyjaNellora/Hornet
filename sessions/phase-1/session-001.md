# Session 001 — Phase 1 (Board I/O) + project bootstrap

**Date:** 2026-06-01
**Duration:** one shift (autonomous; user asleep, authorized "carry on until a real
engine-design decision or blocker")
**Agent:** claude (Opus 4.8)

## Summary

Stood up the Playbook runtime from scratch (it didn't exist), then implemented the first
engine code: the board types and a complete, round-tripping FEN4 parser/serializer for the
canonical chess.com 4PC dialect. 15 unit tests green, clippy clean, fmt applied. One spec
change (landing v0.2) was raised as CO-001 but deliberately **deferred** for a dispatch
decision rather than executed autonomously.

## What Was Done

**Stage 0 — Playbook runtime (project setup, README steps 3–8):**
- `agent-conduct.md` (Mythos conduct + Regular change-orders; 8 Hard Rules transcribed; tiers,
  paths, Rust standards, info-hierarchy).
- `STATUS.md` (production board: phases P0 Spec … P8 Protocol, critical path).
- `phases/phase-1-board-io.md` (full) and `phases/phase-0-spec.md` (thin, Kimi-owned).
- `HANDOFF.md`; `change-orders/`, `sessions/phase-1/` dirs; `dispatch_comms.jsonl` (append-only,
  SBAR entries throughout this shift).

**Stage 1 — CO-001 (land spec v0.2):** raised `change-orders/CO-001-land-spec-v0.2.md`,
logged tier-2. **Execution deferred** (see Decisions).

**Stage 2 — Board I/O / FEN4 (the code):**
- `hornet-engine/` cargo crate + `src/lib.rs` + `src/main.rs` + the full spec-§9 module tree as
  stubs (board/, nnue/, protocol/ + lines/queries/eval/search/move_gen/move_order/tt). Builds clean.
- `board/types.rs`: `Square` (index/rank/file/`is_valid` partitioning 160 valid / 36 invalid;
  algebraic `to`/`from`), `Player` (turn order, opponents, char/prefix codecs), `PieceType` with
  the **split value systems** `eval_value` (centipawns) + `ffa_points` (FFA) per Hard Rule #8,
  plus `to_char`/`from_char`, `Piece` token codec. 7 tests.
- `board/mod.rs`: `Board` struct (squares[196] + FEN4 header state: side_to_move, dead[4],
  castle_kingside/queenside[4], points[4], `extra` raw, en_passant) + accessors
  (`piece_at`/`set_piece`/`piece_count`/`king_square`).
- `board/fen4.rs`: `parse` + `serialize` for the chess.com dash-header dialect, `START_FEN4`
  constant, `Fen4Error` enum (Display + Error). 8 tests incl. **byte-identical** round-trip of the
  canonical start string, empty-board round-trip, a hand-built mixed position, and malformed-input
  rejection.
- Toolchain gates: `cargo build`/`cargo test` green (15 passed), `cargo clippy --all-targets`
  clean, `cargo fmt` applied (it alphabetized `mod` declarations — cosmetic), `cargo run` boots.

## What Was Tried But Failed / Course-Corrected

- **Initially intended to land v0.2 autonomously** (CO-001 first written `approved`). On reading the
  full 792-line spec I reversed: the delta routes PGN4 to a non-existent "§6.5" (spec §6 is Search;
  there is **no** protocol/I-O section), so landing it requires *creating and placing* a new section
  — a structural call the spec owner should make. Combined with the pitch's statement that Kimi is
  mid-release of v0.2 (collision risk), I judged this a "real decision" the user reserved, and
  deferred. CO-001 status set to `open` with the decision written up.
- `assert_eq!` on `Board` failed to compile until `Board` derived `Debug` (added).

## Decisions Made

1. **Targeted the chess.com dash-header FEN4 dialect** as Hornet's native format (per Hard Rule #2 +
   spec §1.3), NOT the `xxx`-corner dialect in `tactical_samples.json`. See Watch Items.
2. **Implemented the eval/FFA value split now** even though v0.2 isn't landed: it's a *settled* Hard
   Rule (#8) with fully-fixed values, not a pending design choice.
3. **Stored FEN4 field 6 (`extra`) raw** rather than guessing its grammar — guarantees byte-exact
   round-trips and avoids silently misparsing a possible draw-clock/EP field. Flagged.
4. **Deferred landing v0.2** (CO-001) — see above.
5. **Lean `Board`** holding only what FEN4 needs; derived structures (piece lists, cached king
   square, zobrist, line maps) deferred to P2/P3 when they're actually needed.

## What's Next (priority order)

1. **USER/DISPATCH DECISION on CO-001:** who lands v0.2 — Kimi (owns spec, resolves §6.5 placement
   natively) or me (with confirmation Kimi is idle)? This gates P1's PGN4 work.
2. After v0.2 lands: **Stage 3 — PGN4 parser** (`board/pgn4.rs`) + round-trip the 16 corpus games
   (`tests/pgn4_roundtrip.rs`). Notation seen in corpus: `h2-h3`, `Ne1-f3`, `Qm8xh13+`, `g7-g8=D`,
   `O-O`/`O-O-O`, mate `#`. StartFen4 shorthand `"4PC"` = canonical start.
3. Then **P2 — move generation** (needs the castling tables from v0.2 §1.5 + EP §1.6).
4. Lock the board-representation decision (mailbox vs bitboards) before P2 — currently mailbox.

## Watch Items

- **Two FEN4 dialects.** `tactical_samples.json` uses a different (`xxx`-corner, space-trailer)
  format. The 25 tactical samples are **not** directly loadable by Hornet's FEN4 parser → strength
  gate (P4/P5) needs a converter or a re-export. Raise a CO when that phase starts.
- **FEN4 field 6 semantics unknown.** Stored raw. If a real mid-game chess.com FEN4 shows it encodes
  the draw clock and/or en passant, extend the parser to populate `Board.en_passant`. May warrant a
  CO to the spec to pin the grammar.
- **PromotedQueen not representable in FEN4** (serializes as `Q`). Fine for placement; if FFA
  scoring needs the distinction from a loaded position, that information is lost on FEN4 import.
- **Nested git repo:** `cargo new` created `hornet-engine/.git`. Harmless (parent isn't a repo) but
  note it if the project later wants a single top-level repo.

## Open Questions

- Does chess.com FEN4 ever place explicit en-passant state, and where (field 6, or an extra field)?
  No mid-game sample of this dialect is in the corpus to confirm.
- Is the second (`xxx`) dialect worth supporting natively, or should the tactical fixtures be
  re-exported to canonical FEN4 during the strength-gate phase?
