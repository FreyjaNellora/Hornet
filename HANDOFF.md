# Hornet — Shift Handoff (HANDOFF.md)

**Shift store — overwritten each handoff.** Detail: `sessions/`, `STATUS.md`, `ENGINE-HANDOFF.md`.

## State (2026-06-10 — review-cycle cleanup shift)

**Engine:** full pipeline (board → move-gen → lines → queries → eval → Max^n) end-to-end.
Suite **fully green: 112 lib + 3 integration**, from a clean tree, against the **32-game** corpus.

Landed this shift (the review-consolidation A-bucket; see `PLAN-three-agent-worksplit-2026-06-10.md`):

- **Suite gate repaired.** `tests/pgn4_replay.rs` and `tests/pgn4_roundtrip.rs` both hardcoded the
  old 16-game corpus count → the suite had been failing since the corpus doubled. Counts now 32;
  replay floors recalibrated against observed **5058/7477 plies, 15/32 games fully** (floors:
  ≥5000 / ≥15). Divergence profile unchanged (DKW boundary).
- **Env experiment flags fixed.** `HORNET_SEE` / `HORNET_SELECTIVE_INTENT` required only *presence*
  (so `HORNET_SEE=0` enabled SEE). Both now require the exact value `1` (`queries.rs`).
- **Protocol zobrist desync fixed.** `apply_ply`'s side-to-move self-sync bypasses the incremental
  hash; `build_position` / `load_pgn4` now `recompute_zobrist()` before handing the board to a
  TT-keyed search.
- **Doc-comments corrected.** `with_win_term` / `with_king_danger` are **flashlight-only** (the
  `maxn`/`search()` path never sees the objective layer); `eval_scalar` computes the full vector.
- **State docs reconciled.** STATUS / ENGINE-HANDOFF updated (test counts, replay numbers,
  protocol-wired contradiction, contamination note). Spec staleness drafted as **CO-004 / CO-005**
  (await user approval — Tier 2).

## Known open defect (owned, do not silently fix)

`move_order::count_defenders` polarity is **inverted** (counts non-victim pieces as defenders;
adjacency-only) and gates `FREE_CAPTURE_BONUS`, which ships **default-ON** with
`FFA_BOUNTY_MOVE_ORDER` against Hard Rule #6. Outcome-affecting at narrow beams (ordering = beam
selection). Fix in flight as **B1/B2** of the plan: a **3-arm measured flag flip**
(both-on / free-capture-off / both-off; move_match + seeded self-play A/B at beam 30 and beam 4),
landing both flags off, then a perf-gated fix-or-delete of the function.

**Corpus contamination split (verified — don't re-prove):** `selfplay_games/` (133-game bootstrap,
maxn path at beam 4) was generated under the bug → regenerate (B5) before tuning on it.
**EXP-017/018 flashlight results are clean** — `search_flashlight` never calls `move_order`
(`search.rs`: only `root_move_values`:412 and `search_depth`:432 do). Wide-beam maxn runs: mildly
affected at most (no value cutoffs in Max^n; within-beam order only breaks ties / picks LMR set).

## ⏭ Next (per `PLAN-three-agent-worksplit-2026-06-10.md`)

- **B1** (3-arm flag measurement) → new move_match baseline → **B2** (`count_defenders`
  fix-or-delete, perf-gated), **B3** (protocol `go` config off the deprecated node budget).
- **C1** (gate the `W=0` queries — pure perf, gateless, can land any time) → **C2** (safety
  rebuild + objective-layer A/B, read against the *new* baseline) → **C3** (relational terms, on
  the regenerated corpus).
- **CO-004 / CO-005** await user approval; **Hard-Rule amendment** (move-changing levers under the
  ablation gate) drafted in B4, also user-tier.

## Next agent: start here

`STATUS.md` → `PLAN-three-agent-worksplit-2026-06-10.md` → your lane's section.
