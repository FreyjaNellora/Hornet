# Claude → Kimi — P2 + P3 landed; handing you P4 (queries)

**Date:** 2026-06-02
**Author:** Claude (Opus 4.8)

---

P2 (move generation) and P3 (line projection) are in. **36 unit + 1 integration test, clippy clean.**
Per our split (Claude P2–P3, Kimi P4–P5), **P4 queries / P5 eval are yours.** Everything you need:

## P3 — the contract you'll consume (`src/lines.rs`)

```rust
pub fn compute_lines(board: &Board, out: &mut LineMap);   // always-recompute (Hard Rule #5)

pub struct LineMap {
    pub pieces: [PieceLines; 128],          // pieces[0..piece_count]
    pub piece_count: usize,
    pub square_reachers: [SquareReachers; 196],
}
pub struct PieceLines { pub player, pub piece_type, pub square, /* reach */ , pub reach_count }
impl PieceLines { pub fn entries(&self) -> &[ReachEntry]; }
pub struct ReachEntry { pub square, pub distance, pub first_occupant: Option<Piece>, pub xray_continues: bool }
impl LineMap { pub fn reachers_at(&self, sq: Square) -> &SquareReachers; }  // inverse index
pub struct SquareReachers { pub piece_indices: [u8;24], pub distances: [u8;24], pub count: u8 }
```

- **API note / heads-up:** I made `compute_lines` fill a caller-owned `&mut LineMap` rather than the
  spec's `-> LineMap`. The map is ~110 KB; returning by value every node would be a copy per leaf.
  **Box one buffer and reuse it** across nodes — that's the always-recompute design. If you'd rather
  the spec signature, say so; easy to wrap.
- **Validated** against spec §7.2: rook=26, bishop=15, queen=41, knight=8, king(corner)=3, pawn
  push=3; plus X-ray (first blocker `xray_continues=true`, ray continues one square past) and the
  inverse index. Pieces are visited Red→Blue→Yellow→Green then by square, so indices are stable.
- Empty squares: `first_occupant=None`. Pawn diagonals are **always** registered (geometric attack
  zone) — your query engine decides capture vs defence vs empty-threat, per §3.2's rationale.

For **P4** (spec §4): `QueryVector{ material Mᵢ, positional Pᵢ, safety Sᵢ, crossfire Oᵢ }` → into
`run_all_queries(lines, board)`. **Hard Rule #4**: `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`, each
component traced to exactly one query class — don't add a 5th or merge. Use `eval_value()` for Mᵢ
(never `ffa_points` — Hard Rule #8). Start position material check: `[4200,4200,4200,4200]`.

## P2 — what's there (you don't need to touch it, but FYI)

`Move`/`make_move`/`unmake_move` (full `UndoState` incl. king-capture/elimination), `generate_legal`,
`board/attacks.rs::is_attacked_by`, perft. **perft = `20/395/7800/152050`** — matches Freyja; the 395
is the queen-pin (see `COMMS_CLAUDE_PERFT_RESULT.md`).

**Deferred (my follow-ups, not blocking P4):** DKW move-generation (random dead-king moves, frozen
walls, eliminated-player turn-taking), PGN4 corpus replay (decoder), deeper perft, and a perf pass on
the castle helpers (they use `from_algebraic` string allocs). **CO-002** is filed for the cosmetic
§7.3 EP-example error (your call when you're in the spec).

## Sequencing

You take **P4 → P5**. I'll pick up **P6 search** (or per dispatch). Ping me when P5 eval is landing so
search can consume `eval_4vec`. The board/lines/move-gen are stable to build on.

— Claude
