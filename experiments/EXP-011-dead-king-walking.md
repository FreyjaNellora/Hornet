# EXP-011 — Dead-King-Walking (full implementation)

- **Date:** 2026-06-07
- **Built:** the complete DKW lifecycle (§1.7/1.8) across board / move-gen / search / game-flow.
  Was the gnarliest roadmap item (#2); rules were in the spec but unmodeled. Two rule clarifications
  from the user shaped the final model (see below).

## The model (3 player states)
- **LIVE** (`!dead && !dkw`) — normal play.
- **DKW** (`board.dkw[p]`, king still on board) — the player was checkmated/stalemated. Its king
  **walks randomly**, ignoring check; its non-king pieces are **fully-frozen walls** — immovable and
  **un-capturable by anyone** (not by live players, not by other dead kings, not even by its own
  walking king). They only block. The DKW king itself is capturable, and earns **no points** for any
  capture it makes.
- **DEAD** (fully eliminated — king captured or DKW-king stalemated) — **all its pieces are removed
  from the board** and it is skipped in rotation.

Transitions: checkmate/stalemate (no legal moves, king alive) → DKW (+20 consolation if stalemate).
DKW-king capture, or DKW-king stalemate (+10 to each survivor), → DEAD → board swept of its pieces.

**Two user rule clarifications (vs an earlier draft):**
1. A DKW player's pieces are *fully frozen* — **un-capturable**, not takeable. (The takeable variant
   is deferred behind `move_gen::DKW_PIECES_REMOVABLE`, default `false`.)
2. A *fully eliminated* player's pieces are **removed** from the board (not left as permanent walls).

## What was implemented
- **Board** (`board/mod.rs`): `dkw[4]` flag + `enter_dkw`; `eliminate_player` (sweeps all a player's
  pieces, sets `dead`, clears `dkw`); `make_move`/`unmake` track `dkw`; a DKW capturer earns no
  points; Zobrist `key_dkw` for cross-turn TT correctness.
- **Move-gen** (`move_gen.rs`): a DKW mover generates **king-only** moves (`gen_steps`, no check
  filter). `is_wall` (a DKW player's non-king piece) makes frozen pieces **block but be un-capturable**
  in `gen_steps`/`gen_rays`/`gen_pawn`; `in_check` is frozen-aware (a DKW opponent threatens only with
  its king). `DKW_PIECES_REMOVABLE = false` is the one-line toggle for the removable variant.
- **Search** (`search.rs`): a DKW node is **expectimax** — it averages over the king's moves (the king
  is *random*, not a maximizer). **King-capture does NOT sweep pieces inside the search** — doing so
  made Max^n wildly over-value king-captures (the victim's whole material vanishes from the eval →
  pathological king-hunting; it broke the free-queen sanity test). The sweep is applied at game-flow.
- **Game-flow** (`game.rs`, new): a `Game` driver runs the lifecycle (checkmate/stalemate → DKW, the
  random king walk via a seeded PRNG, DKW-king-stalemate scoring) and **sweeps any player a move
  eliminates** (the game board obeys rule #2). Self-play plays full games through it.

## Results
- **106 lib tests green** + integration tests. New unit tests: a DKW king walks but can't take frozen
  pieces (own or others'); frozen DKW pieces are un-capturable and block a live slider; a DKW capture
  awards no points; `eliminate_player` sweeps every piece; a full game runs to completion with the
  Zobrist hash staying in sync across the whole DKW lifecycle.
- **Corpus replay: 2532 / 3770 plies, 8 / 16 games fully replayed** (was 2532, 4/16). The replay
  validates move *geometry* against the chess.com corpus, whose DKW is *takeable*; it never sets the
  DKW flag, so our fully-frozen rule intentionally diverges wherever the corpus captures a DKW piece
  (incl. a dead king taking its own). The plies stay at the pre-DKW baseline for that reason; the
  R/S-marker skip is what lifted fully-replayed games 4→8. The *removable* toggle would restore corpus
  fidelity (≈2846, 10/16) — confirming the geometry is right; the difference is purely the frozen rule.

## Conclusion
DKW is built properly end-to-end under the user's rules (frozen pieces; eliminated pieces removed).
The engine plays and searches post-elimination positions, and self-play runs full games to a survivor
with elimination scoring. The corpus replay is a chess.com-fidelity check of move geometry, validated
by the removable-toggle equivalence; the frozen rule is validated by the unit + lifecycle tests.
