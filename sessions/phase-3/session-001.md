# Session 001 — Phase 3 (Line Projection) + P2 finish

**Date:** 2026-06-02
**Agent:** claude (Opus 4.8)

## Summary

Finished P2 (castle generation, branch make/unmake tests, an EP-orthogonality bug fix) and built P3
line projection end-to-end. **36 unit + 1 integration test, clippy clean.** Reached the planned
handoff point — P4 queries are Kimi's.

## What Was Done

**P2 finish:**
- Castle **generation** (`gen_castles`) with §1.5 per-player tables + through-check legality.
- Branch make/unmake tests: castling (+ blocked-through-check), en passant, promotion (queen →
  `PromotedQueen`), king-capture/elimination (skip-dead rotation).
- **Bug fix:** EP generation now enforces §1.6 orthogonality (Red↔Yellow / Blue↔Green EP rejected).
- **Spec bug found → CO-002:** §7.3 EP examples place the capturing pawn on the wrong square (straight
  push, not diagonal) in all four cases. Normative rules (§1.4/§1.6) are correct; engine follows them.

**P3 (`src/lines.rs`):** `ReachEntry`/`PieceLines`/`SquareReachers`/`LineMap` per §3.3;
`compute_lines(board, &mut LineMap)` with slider X-ray, knight/king steps, pawn push + always-on
diagonals, and the per-square inverse index. Validated against §7.2 (rook 26, bishop 15, queen 41,
knight 8, king 3, pawn 3), X-ray, inverse index, 64 pieces from start.

## Decisions Made

1. **`compute_lines` fills a reusable `&mut LineMap`** (not spec's `-> LineMap`) — the map is ~110 KB;
   returning by value per node is a copy. Box once, reuse (always-recompute). Documented for Kimi.
2. **Deferred (not blocking P4):** DKW move-generation, PGN4 corpus replay, deeper perft, castle-helper
   perf (string allocs). P2's *core* (non-DKW legal move-gen) is complete + perft-validated.
3. `on_start_rank` duplicated in `move_gen` + `lines` — minor; dedup candidate.

## What's Next

- **Kimi:** P4 queries → P5 eval (handoff in `COMMS_CLAUDE_HANDOFF_P4.md`; manual in
  `phases/phase-4-queries.md`).
- **Claude (later):** P6 search (after P5 eval), plus the deferred P2 items above.

## Watch Items

- The deferred DKW work is needed before full-game corpus replay / deep perft past eliminations.
- Keep `on_start_rank` in sync across `move_gen`/`lines` until deduped.
- `LineMap` dead/DKW piece handling unspecified — currently projects all pieces; revisit with DKW.
