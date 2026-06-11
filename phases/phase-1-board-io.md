# Phase 1: Board I/O

## Commander's Intent

Give Hornet a correct, native in/out boundary: parse and emit FEN4 positions and PGN4 games
directly (Hard Rule #2), backed by a board representation the rest of the engine builds on. Success
is round-tripping real chess.com data losslessly — the 16 PGN4 corpus games and the canonical FEN4
start string — so every downstream phase can trust the board it receives.

## Reading List (Start Here)

1. `STATUS.md` — where the project stands; what's blocked.
2. `phases/phase-1-board-io.md` — this file.
3. `sessions/phase-1/session-{latest}.md` — last shift's handoff.
4. `HORNET-BUILD-SPEC.md` §1 (rules, geometry, starting position, FEN4) and §9 (file structure);
   v0.2 additions for PGN4 in `HORNET-BUILD-SPEC-v0.2-DELTA.md` §3 (or the landed §6.5/§9).
5. `baselines/README.md` + `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md` (canonical FEN4
   start string, castling/placement ground truth).

## Write Scope

**Owns (create/modify/delete):**
- `hornet-engine/` scaffold: `Cargo.toml`, `src/lib.rs`, `src/main.rs`, module stub tree (§9)
- `hornet-engine/src/board/**` (`mod.rs`, `types.rs`, `fen4.rs`, `pgn4.rs`, `zobrist.rs`, `attacks.rs`)
- `hornet-engine/tests/fen4*.rs`, `hornet-engine/tests/pgn4_roundtrip.rs`
- `hornet-engine/baselines/` symlink-or-copy decision for test access (document choice)

**Shared (modify with care, log):**
- `hornet-engine/src/protocol/parse.rs` — only the `position fen4` / `position pgn4` command wiring
  (coordinate with P8 Protocol via change order if it conflicts).

**Read-only:** everything else.

## Current State

| Field | Value |
|-------|-------|
| Status | in-progress — FEN4 + PGN4 (structural) done; move-semantics need P2 |
| Last Session | 2026-06-01 — `sessions/phase-1/session-002.md` (v0.2 landed; PGN4 structural) |
| Blocking Issues | none — PGN4 *move-semantics* await P2 move-gen (not a P1-I/O blocker) |
| Next Action | Hand to P2 (move generation); decode PGN4 ply tokens once P2 exists |

## Acceptance Checklist

- [x] `board/types.rs`: `Player`, `PieceType` (incl. `PromotedQueen`), `Piece`, square index +
      `is_valid` (exactly 160 valid / 36 invalid), `eval_value()` + `ffa_points()` (Hard Rule #8).
- [x] FEN4 **parse**: canonical start string → correct placement (kings at h1/a7/g14/n8), correct
      side-to-move and the four flag groups.
- [x] FEN4 **serialize**: board → FEN4; canonical start round-trips **byte-identical**.
- [x] PGN4 **parse (structural)**: headers + move stream tokenized into rounds/plies; `StartFen4
      "4PC"` resolved. *Semantic decode of plies (from/to, SAN, promotion, +/#) deferred to P2.*
- [x] PGN4 **round-trip**: all 16 `baselines/*.pgn4` — structural (`parse∘serialize` stable), not
      byte-identical. *Move-stream replay (applying moves) awaits P2.*
- [x] `cargo build` + `cargo test` green (19 unit + 1 integration); `cargo fmt` clean; `clippy` triaged.

## Active Watch Items

- CO-001: PGN4 round-trip blocked until v0.2 lands. FEN4 proceeds regardless.
- Board representation choice (mailbox `[Option<Piece>; 196]` vs bitboards) is a downstream-visible
  decision — see Downstream Notes; lock it before P2 starts.

## Rework Log

| Date | Requested By | What Changed | Why | Impact |
|------|-------------|-------------|-----|--------|
| | | | | |

## Downstream Notes

P2 (Move-gen) consumes `Board`: square indexing `sq = rank*14 + file` (0..195, 160 valid), the
invalid-corner mask, side-to-move, castling/EP flags, and per-player piece lists. **Decision pending
this phase:** board storage layout — recommend a 196-cell mailbox for clarity first (Hard Rule #5
forbids incremental line indices, so a simple array is fine), revisit if perft is slow. Document the
chosen FEN4 flag-group semantics (the four `0/1` groups + EP field) so P2/P8 read them identically.
