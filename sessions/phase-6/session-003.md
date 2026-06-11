# Session 003 — Phase 6 (Search) — move-ordering flags: measure, land off, fix the scan

**Date:** 2026-06-10
**Agent:** Fable (Fable 5)

## Summary

Executed worksplit items **B1 + B2** (user-assigned; Opus reviewed the plan pre-execution and
folded in one fix), then **B3 + B4** on an explicit user greenlight. The two move-ordering flags
that shipped `const = true` against Hard Rule #6 are now default-off `OrderState` fields, with the
flip's effect **measured, not asserted** (EXP-020); the polarity-inverted `count_defenders` is
replaced by a real attack scan at measured ≈zero cost (EXP-021); protocol `go` now plays the
SYNTHESIS-recommended flashlight config; CO-006 drafts the Hard-Rule amendment. Suite **114 lib +
3 integration green**, fmt applied, no new clippy warnings (my files clean), REPL smoke-tested
end-to-end.

## What landed (code)

- `move_order.rs`: consts → `OrderState.ffa_bounty`/`free_capture` (default **false**; guard test
  `ordering_levers_default_off`); `order()` → `sort_by_cached_key` (key runs once per move —
  prerequisite so EXP-021 measured scan cost, not sort re-evaluation; Opus review);
  `count_defenders` (inverted `p.player != victim_player`, radius-1, dead `pawn_deltas` match) →
  `is_defended` = `board::attacks::is_attacked_by`; polarity regression test
  `free_capture_bonus_prefers_undefended_victim` (distant-rook defender + adjacent-bystander
  construction — the old code fails it both ways); bounty tests now enable the lever explicitly.
- `search.rs`: builders `with_ffa_bounty_order` / `with_free_capture_order`; flashlight doc-comment
  now names it the play path.
- `protocol/mod.rs` (B3): `go` → `search_flashlight` at `GO_FLASHLIGHT_CAP = 1200` (SYNTHESIS;
  cap-spectrum ~even point); deprecated maxn + 2M node-budget config removed; test mirrors the
  shipped path. Side effect of default-off flags: protocol needed no flag wiring.
- Harnesses (test-only): `move_match`/`bench_beam` parameterized (beam/depth/sample/flags;
  bench_beam gained a seeded **mid-game mode** = the EXP-021 cost instrument); new
  `selfplay_ab_maxn` (maxn-path A/B — `selfplay_ab` drives flashlight, which never orders); new
  `move_diverge` (added mid-experiment: runs both configs per corpus position, counts differing
  choices — match-rate deltas can't show behavior change).

## What was measured (EXP-020 / EXP-021)

- **Equivalence gate (pass, exact):** flags-on refactored binary reproduced the golden refs
  bit-for-bit (move_match 347/2530; all six bench_beam node counts + best moves).
- **Contamination:** the buggy lever changed the played move on **11.6%** of corpus positions at
  beam 4; 0.9% at beam 10; 0.6% at beam 30. Bounty lever: 1.8% at beam 4. Human-agreement flat
  everywhere (13.4–13.7%).
- **Strength of landing off:** self-play (12 games/pairing, d8 beam 4) — freecap pairing noise
  (A 228 – B 223, 5/12); bounty mildly positive (282–240, 6/12) but unpowered → stays off.
- **New baseline (arm iii):** move_match 13.5% / 13.6% / 13.6% at beams 4/10/30.
- **Scan cost:** median nodes/sec 64,337 (off) vs 65,813 (on) — noise; >10% delete rule not
  triggered → fix kept. **Venue deviation recorded:** cost run re-based beam 30→10 after the
  first beam-30 positions ran 30–47 min each with already-stable nodes/sec; per-node ordering
  cost is beam-independent (sort precedes beam truncation); beam-30 OFF numbers cross-check.

## Decisions / notes for the next shift

- **B5 (corpus regen) is unblocked** but needs the search-shape + aggression/decisiveness config
  decided with Kimi before burning compute.
- **Kimi C2:** read move-agreement against the arm-(iii) baseline, not EXP-015's figures.
- The corrected free-capture lever has **never been strength-measured** — only cost-measured. If
  anyone wants it on, it must earn it through a powered self-play arm.
- `experiments/README.md` index had a gap (EXP-012…019 unindexed) — noted in the README; backfill
  is an open hygiene item.
- Pre-existing clippy/build warnings in `queries.rs`/`bounty.rs`/`intent.rs`/`zones.rs`/
  `parse.rs`/old examples remain (other lane / pre-dating this shift; triaged as not-mine).
- Plan-mode artifacts: golden refs and the full run matrix are reproducible from the recorded
  commands in EXP-020/021.

## Open after this shift

B5 (corpus regen, joint config), C1 (Kimi, zero-weight gating), C2/C3 (Kimi, re-anchored),
CO-004/CO-005/CO-006 (user, Tier 2).
