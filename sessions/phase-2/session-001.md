# Session 001 — Phase 2 (Move Generation)

**Date:** 2026-06-01 → 2026-06-02
**Agent:** claude (Opus 4.8)

## Summary

Built P2's core — `Move`/make-unmake/attacks/move-gen/legality/perft — and **resolved the perft
discrepancy empirically: Hornet computes `20/395/7800/152050`, matching Freyja exactly.** 395 is
correct; the gap vs 400 is a discovered pin neither hand-analysis caught.

## What Was Done

- **`board/mod.rs`:** added `en_passant_pushing_player`; `Move{from,to,promotion,flags}`, `MoveFlags`,
  `UndoState`; `make_move`/`unmake_move` (capture incl. EP square ≠ `to`, promotion → `PromotedQueen`,
  castling rook-hop, double-push EP arming, castle-right updates, **king-capture elimination** state,
  turn rotation skipping dead players); castle-table helpers; shared geometry helpers
  (`KNIGHT/KING/BISHOP/ROOK` deltas, `pawn_forward`, `pawn_capture_deltas`, `offset`).
- **`board/attacks.rs`:** `is_attacked_by(board, sq, by)` — pawn/knight/king/slider, rays stop at
  pieces and invalid corners.
- **`move_gen.rs`:** `generate_pseudo_legal` (pawn push/double/capture/EP/promotion, knight, sliders,
  king), `generate_legal` (own-king-in-check filter), `perft`. Castle generation stubbed (can't fire
  in the opening).
- **Tests (25 unit + 1 integration, clippy clean):** Red opens with 20; make/unmake restores the
  board for all opening moves; double-push arms/clears EP; capture make/unmake; `perft_matches_known_values`
  (`20/395/7800/152050`); `opening_pin_explains_perft2` (documents the pin).

## The Perft Resolution (key finding)

perft-divide at depth 2 → three Red openings reduce Blue's 20 replies: `d2-d4`→19 (occupancy blocks
`b4-d4`), `f2-f3`→18 and `f2-f4`→18. The −2s are a **pin**: vacating f2 opens the Red queen's
g1-diagonal `g1-f2-e3-d4-c5-b6-a7`, pinning Blue's b6 pawn against its king on a7 (both b6 pushes
illegal). `1+2+2 = 5`, `400−5 = 395`. Freyja's "open lines" comment was right; we both modeled only
occupancy. Full write-up: `COMMS_CLAUDE_PERFT_RESULT.md`.

## What's Next (P2 remainder, then P3)

1. Castle **generation** (with through-check legality) — make/unmake already supports castle moves.
2. Targeted make/unmake round-trip tests for **EP, castle, promotion, king-capture** branches
   (perft 1–4 from start exercises none of them).
3. DKW/elimination handling in move-gen + deeper perft once the above land.
4. **PGN4 corpus replay** (decode ply tokens → `Move`, apply) — closes the P1 PGN4-semantics gap.
5. Then **P3 line projection**; hand **P4 queries / P5 eval** to Kimi.

## Watch Items

- The untested make/unmake branches (above) are the main correctness risk — test before deeper perft.
- `next_live_player` (dead-skip) and king-capture elimination are implemented but unexercised at
  shallow depth.
- Castle helpers use `from_algebraic` (string alloc) — fine for now, optimize if hot.
