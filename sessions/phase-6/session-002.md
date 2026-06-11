# Session 002 — Phase 6 (Search) — review & harden

**Date:** 2026-06-03
**Agent:** claude (Opus 4.8)

## Summary

Deliberate correctness review of the P6 search core (built fast last session during the concurrent
churn), per the approved plan. Found and fixed three issues, added tests pinning down the Max^n
semantics. 63 unit + 3 integration tests green, my files clippy-clean.

## Findings → fixes (all in `search.rs` unless noted)

1. **TT exact-value reuse was unsound under beam** — `maxn` returned `e.value` when `e.depth >= depth`,
   but beam-computed values are approximate (and ordering-dependent). **Removed the value cutoff;
   the TT is now move-ordering only.** True reuse + bounds belong with shallow pruning.
2. **Root applied the beam** — `take(beam_width)` at the root could drop a strong move ordered past
   the beam. **Root now iterates all legal moves;** beam stays at internal nodes.
3. **Hard Rule #1 not enforced** — added `round_to_rotation` so `search` rounds the requested depth
   up to the next positive multiple of 4.

## Added for testability + verification

- **Injectable leaf eval** on `Searcher` (`#[cfg(test)] with_eval`) so the Max^n backup can be tested
  with a controllable synthetic eval (default stays `eval_4vec`).
- Tests: `maxn_node_maximizes_the_movers_own_component` (Red node → max Red, Blue node → max Blue —
  the Max^n-vs-paranoid distinction), `root_considers_all_moves_not_just_the_beam`,
  `fresh_searches_are_deterministic`, `depth_rounds_up_to_a_full_rotation`.
- `zobrist.rs`: documented that excluding `points` from the hash is TT-safe (eval is points-independent).

## Notes

- Determinism is **per-fresh-searcher**: a TT persisting across calls legitimately changes move
  ordering (and thus, under beam, can change results) — that's expected TT behaviour, not a bug.
- `tt.rs` replacement scheme (different-key-always-replace + `key==0` sentinel) kept for the baseline;
  a two-tier/aged scheme is a later upgrade. The `Bound` enum stays (used when shallow pruning lands).

## What's next (P6 refinements)

Max^n shallow pruning (then re-enable TT value reuse with real bounds), proper §1.8 terminal scoring
(checkmate/stalemate/DKW), iterative deepening, killers + history. Then P8 protocol.
