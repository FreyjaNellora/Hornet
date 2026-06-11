# Phase 3: Line Projection

## Commander's Intent

Compute, for every piece, its geometric reach (the foundational primitive) into a `LineMap` that the
query engine consumes. Always-recompute, no incremental state (Hard Rule #5). Owner: **claude**.

## Reading List

1. `STATUS.md` · 2. this file · 3. `sessions/phase-3/session-001.md` ·
4. `HORNET-BUILD-SPEC.md` §3 (algorithm + data layout) and §7.2 (reach-count tests).

## Write Scope

**Owns:** `hornet-engine/src/lines.rs`. **Read-only:** everything else.

## Current State

| Field | Value |
|-------|-------|
| Status | **complete** (2026-06-02) |
| Last Session | 2026-06-02 — `sessions/phase-3/session-001.md` |
| Blocking Issues | none |
| Next Action | — (consumed by P4 queries, owner Kimi) |

## Acceptance Checklist

- [x] `ReachEntry` / `PieceLines` / `SquareReachers` / `LineMap` per §3.3.
- [x] `compute_lines(board, &mut LineMap)` — slider X-ray, knight/king steps, pawn push + always-on
      diagonals; inverse index built.
- [x] §7.2 reach counts verified: rook 26, bishop 15, queen 41, knight 8, king(corner) 3, pawn 3.
- [x] X-ray (first blocker `xray_continues`, continues one past) + inverse index tested. 64 pieces
      from the start. `cargo test`/`clippy`/`fmt` green.

## Downstream Notes

P4 consumes the `LineMap` (see `COMMS_CLAUDE_HANDOFF_P4.md`). **API deviation from spec §3.1:**
`compute_lines` fills a caller-owned `&mut LineMap` (reusable ~110 KB buffer) instead of returning by
value — a perf choice for always-recompute. `on_start_rank` is duplicated from `move_gen` (minor;
dedup candidate). Dead/DKW pieces are currently projected like any other piece (revisit when DKW lands).
