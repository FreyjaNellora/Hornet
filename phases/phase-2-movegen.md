# Phase 2: Move Generation

## Commander's Intent

Generate all legal 4PC moves with a fast make/unmake, so search and line projection can explore
positions correctly. Success = perft matches the locked correctness gate **and** the 16 PGN4 corpus
move-streams replay cleanly once moves can be applied. Owner: **claude**.

> **STATUS: in-progress.** Perft gate **RESOLVED** — Hornet independently computes
> `20 / 395 / 7800 / 152050`, matching Freyja exactly. `perft(2)=395` is correct: the 5 "missing"
> vs 400 are `d2-d4` (occupancy block of `b4-d4`) + `f2-f3`/`f2-f4` (each vacates f2, opening the
> g1-queen's diagonal to **pin Blue's b6 pawn** against its king on a7 → both b6 pushes illegal).
> Now a regression test. See `COMMS_CLAUDE_PERFT_RESULT.md`.

## Reading List (Start Here)

1. `STATUS.md`
2. `phases/phase-2-movegen.md` — this file.
3. `COMMS_KIMI_REPLY.md` + `COMMS_CLAUDE_REPLY.md` — the agreed P2 design (Move, make/unmake, perft).
4. `HORNET-BUILD-SPEC.md` §1.4 (movement), §1.5 (castling tables), §1.6 (en passant), §1.7
   (elimination / DKW), §2.5 (Board), §10.3 (PGN4 for replay validation).
5. `hornet-engine/src/board/{types,mod,fen4,pgn4}.rs` — existing board + I/O.

## Write Scope

**Owns:** `hornet-engine/src/move_gen.rs`, `src/board/attacks.rs`, `tests/perft.rs`, and the move
machinery added to `src/board/mod.rs` (`Move`, `MoveFlags`, `UndoState`, `make_move`/`unmake_move`,
plus any derived `Board` state move-gen needs).
**Shared:** `src/board/pgn4.rs` (only to add ply-token → `Move` decoding once moves apply).
**Read-only:** everything else.

## Agreed design (from the Kimi/Claude exchange)

- **Board representation:** keep the 196-cell mailbox (accepted by both). Bitboards are a v2
  optimization if perft is slow.
- **`Move`:** `{ from: Square, to: Square, promotion: Option<PieceType>, flags: MoveFlags }`.
  `promotion` holds the chosen target (`N/B/R/Q`); a queen promotion lands as `PromotedQueen` on the
  board. Castle rook-hop and EP captured-square are derived (not stored on `Move`).
- **make/unmake:** `make_move(&mut self, Move) -> UndoState` / `unmake_move(&mut self, UndoState)`.
  **`UndoState` must capture (Claude's required addition):** captured piece **and its square** (EP ≠
  `to`), all-8 castling bits, EP target + pushing-player, `side_to_move`, **`dead`/DKW flags + wall
  conversion + `points`** (king-capture elimination), and the `extra`/halfmove field.
- **Hard Rule #5:** no incremental line/inverse index in make/unmake — lines recompute every node.
  (This is why Hornet needs no `piece_id`: the piece-identity ambiguity from Freyja's `lines.rs`
  simply doesn't arise here.)

## Acceptance Checklist

- [x] `Move` + `MoveFlags` + `UndoState` per the agreed design (incl. elimination state).
- [~] `make_move`/`unmake_move` round-trip — validated for all opening moves + a capture; **EP,
      castle, promotion, and king-capture branches still need targeted tests**.
- [~] Move generation: pawns (push/double/capture/EP/promotion), knight, sliders (blocking), king,
      + own-king-in-check filtering done. **Castling generation not yet emitted** (make/unmake
      already handles castle moves; castling can't fire in the opening).
- [x] **perft matches the invariants** `20/395/7800/152050` — regression test `perft_matches_known_values`.
- [x] Corpus replay: `pgn4::decode_ply` + self-syncing replay (`tests/pgn4_replay.rs`). **2532/3770
      plies, 8/16 games fully** (R/S-marker skip lifted full games from 4). Validates move *geometry*
      against the *takeable*-DKW corpus; the fully-frozen rule diverges where the corpus captures a DKW
      piece (the removable toggle restores ≈2846/10). Caught + fixed the §1.4 promotion bug (CO-003).
- [x] No incremental line index (Hard Rule #5). `cargo test`/`clippy`/`fmt` green (106 unit + integ).

## Active Watch Items

- ✅ perft gate resolved (queen-pin); ✅ castle generation done; ✅ EP/castle/promotion/king-capture
  make/unmake branch tests added; ✅ EP §1.6 orthogonality fixed; ✅ §1.4 promotion-rank bug fixed (CO-003).
- ✅ **DKW implemented** (2026-06-07, EXP-011). On checkmate/stalemate a player becomes a **Dead King
  Walking** — its king walks randomly each turn ignoring check (earns no points); its non-king pieces
  are **fully-frozen walls: immovable AND un-capturable by anyone**, even its own king. A **fully
  eliminated** player (king captured/stalemated) has **all its pieces removed**. King-only DKW
  move-gen + `is_wall` (block-but-uncapturable, toggle `DKW_PIECES_REMOVABLE`) + frozen-aware
  `in_check` + expectimax search node (no king-capture sweep in search) + the `game.rs` lifecycle
  driver (sweeps eliminated players). (`=D` is promotion, not a DKW marker; trailing `R`/`S` are
  non-move result tokens.)
- Deeper perft (≥5) has no reference values yet; only a smoke test until cross-checked.

## Downstream Notes

P3 (line projection) consumes the post-move `Board` and recomputes lines from scratch. make/unmake
must therefore leave `Board` in a fully self-consistent state (placement + flags) after every call —
there is no separate line cache to keep in sync.
