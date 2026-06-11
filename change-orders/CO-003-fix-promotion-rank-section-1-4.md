# Change Order CO-003

**Date:** 2026-06-02
**Requested By:** Phase 2 — Move Generation (Session 001, agent claude)
**Target Phase:** Phase 0 — Spec / Reference (owner: Kimi)
**Status:** resolved (landed 2026-06-06; header reconciled 2026-06-10 — user authorized closing the open CO backlog)

## What Needs Changing

Spec **§1.4 gives the wrong promotion rank**. It says pawns promote at the **board edge**:

> "Promotion: On reaching the player's promotion rank (Red→rank 13, Blue→file 13, Yellow→rank 0,
> Green→file 0), the pawn promotes."

The real chess.com 4PC rule is that pawns promote at the **central crossing** — the far edge of the
player's own 8-square span, one step past the centre — **not** the board edge:

| Player | Direction | Promotes at (internal) | e.g. (display) |
|--------|-----------|------------------------|----------------|
| Red | +rank (N) | **rank 7** | …→`g8=D` |
| Blue | +file (E) | **file 7** | …→`h8=D` |
| Yellow | −rank (S) | **rank 6** | …→`e7=D` |
| Green | −file (W) | **file 6** | …→`g7=D` |

## Why

Caught by **replaying the 16-game corpus**: with the spec's board-edge rule, 11/16 games diverged
at the first `=D` token (e.g. Red's g-pawn marches `g2…g6-g7` then `g7-g8=D` — promoting at g8,
internal rank 7, not rank 13). With the corrected ranks, corpus replay jumps from 1198 → **2532
plies, with 4 games replaying completely**. The four cases above are mutually consistent (each player
promotes one step past centre) and match every promotion token in the corpus (incl. capture-promotions
like Green's `h12xNg13=D` onto file 6).

## Impact Assessment

- [ ] Cosmetic
- [x] Structural (a real movement rule — affects move generation, and downstream eval/search via the
      set of legal moves)
- [ ] Architectural

The **engine is already corrected** (`move_gen::on_promotion_edge`); this CO is to fix the **spec
text** so it matches reality and the implementation.

## Affected Phases

| Phase | Impact |
|-------|--------|
| P0 (target) | Fix §1.4 promotion ranks. |
| P2 (requester) | Already fixed in `on_promotion_edge`; corpus replay now validates it. |
| P2/P5 | Pawn promotion happens far earlier than the edge — relevant to move-gen, eval, and any
          tablebase/endgame reasoning. |

## Recommended Fix

Replace the §1.4 promotion ranks with: Red→rank 7, Blue→file 7, Yellow→rank 6, Green→file 6 (internal
indices; one step past the centre in each player's forward direction).

## Resolution

**Landed 2026-06-06** — `HORNET-BUILD-SPEC.md` §1.4 updated. Promotion ranks corrected from board-edge (13/0) to central-crossing (7/6), matching the engine's `move_gen::on_promotion_edge` and corpus replay validation.
