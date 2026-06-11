# Session 001 — Phase 6 (Search) + P2 follow-ups

**Date:** 2026-06-02
**Agent:** claude (Opus 4.8) · autonomous overnight

## Summary

Built the **P6 search core** — Zobrist → transposition table → beam Max^n — wired to Kimi's P5
`eval_4vec`. The full pipeline (board → move-gen → lines → queries → eval → Max^n) now runs
end-to-end. Also (earlier this session) built the PGN4 decoder + corpus replay, which caught the
§1.4 promotion-rank spec bug. 59 unit + 3 integration tests; pushed `eb58b5f`.

## What Was Done

- **`board/zobrist.rs`** — incremental Zobrist hash (SplitMix64 keys; piece/side/castle/EP/dead).
  `set_piece` auto-XORs placement; `make_move` XORs the rest; `unmake` restores. Verified vs
  recompute over move sequences incl. capture/EP/elimination. `Board` gained a `zobrist` field
  (excluded from `PartialEq` — it's a cache) and a `recompute_zobrist`. Added `pub use` re-exports
  of the core board types (`Piece`/`PieceType`/`Player`/`Square`) — Kimi's P4 needed them.
- **`tt.rs`** — power-of-two transposition table, depth-preferred replacement, probe/store.
- **`move_order.rs`** — TT move first, then MVV-LVA captures, then quiets.
- **`search.rs`** — `Searcher` with beam Max^n: each node maximizes the mover's own `V` component;
  beam width (default 30); TT exact-value reuse + best-move ordering hint; leaves call `eval_4vec`.
- **Earlier:** `pgn4::decode_ply` + `tests/pgn4_replay.rs` (self-syncing replay); fixed §1.4
  promotion rank (CO-003); fixed EP §1.6 orthogonality; filed CO-002.

## Decisions Made

1. **Beam Max^n** (top-`beam_width` per node, default 30) as the baseline — full-width is intractable.
   Approximate but the MVV-LVA ordering keeps tactically-relevant moves; refine with pruning.
2. **TT exact-value reuse** is valid for full-beam Max^n (same position searched ≥ depth → same vector).
   Revisit when shallow pruning introduces real bounds.
3. **Terminal nodes** (no legal moves) return the static eval for now — proper §1.8 scoring (mate /
   stalemate / DKW) is deferred.
4. **Zobrist incremental** (not always-recompute) — Hard Rule #5 governs *line projection*, not the hash.

## Coordination (the sync event)

Kimi built P4/P5 in the same crate concurrently. We raced on the `board::Piece` interface (both
fixed it). Kimi's "zobrist.rs fails (6 errors)" was a snapshot of my mid-edit state — resolved.
Established lanes (see STATUS + `COMMS_CLAUDE_*`). One cosmetic warning remains in Kimi's `queries.rs`
(unused `Piece` import) — left it (their lane; pre-acknowledged).

## What's Next

P6 refinements: Max^n shallow pruning, proper terminal scoring (§1.8), iterative deepening, killers +
history. Then **P8 protocol** (wire `position`/`go` to `Searcher`) to make the engine usable
end-to-end. **Strength gate** (Hard Rule #7) needs the FEN4-dialect converter for `tactical_samples`.
**DKW** still deferred (needs a spec clarification on the `T`/`S`/`R` markers).

## Watch Items

- Search perf at depth ≥ 8 needs pruning (eval at every beam leaf is the cost).
- Kimi to land CO-002 (cosmetic) + CO-003 (promotion rank) in the spec.
- Lanes: keep `board` interface changes routed through claude.
