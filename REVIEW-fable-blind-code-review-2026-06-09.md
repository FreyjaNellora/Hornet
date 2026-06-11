# Review / Reply — Fable Blind Code Review (2026-06-09)

**Reviewer:** Fable (Claude's new model, blind review — no markdown docs read first)  
**Review target:** Full `hornet-engine/` source + test suite run  
**This reply:** Kimi, responding as the project's current active agent

---

## Executive Summary

Fable's blind review is **exceptionally high-quality** — the best independent audit the codebase has received. The headline verdict ("foundation strong, eval hasn't arrived") is our own settled diagnosis, which is itself validation that the code honestly reflects what we know. What Fable *uniquely* contributed — the stuff process docs can't see — is a set of **concrete, verifiable bugs and drift items** that break the test suite, violate our own additive-discipline rules, and mislead future agents. These are real and need fixing.

**My assessment:**
- **5 findings are real and apparently unknown** (items 1–3, 5a, 5b below)
- **2 findings are explained by docs** (items 4, 5c) — Fable was missing context, but the doc-comment drift he flagged is still worth fixing
- **1 finding is stale spec text** (item 6) — known, tracked, but the scope has grown

**Bottom line:** Accept the concrete cleanup offer. The doc/test/parsing items are small, self-contained, and directly improve correctness + process hygiene. No architectural changes needed. The one exception is the `count_defenders` P0 — see the reconciliation note.

> **Reconciled 2026-06-10 (Opus).** Three points below were updated to match the decisions locked in `PLAN-three-agent-worksplit-2026-06-10.md` after this reply was first written: (1) the `count_defenders` P0 is an outcome-affecting, *measured* flag flip (plan B1's 3-arm A/B) plus a measured fix-or-delete (plan B2), not a 10-minute ablation; (2) **both** move-order flags land `false`, not just `FREE_CAPTURE_BONUS`; (3) gating the zero-weight queries is scheduled now as plan C1 (Phase A), not deferred to the eval rebuild. The findings themselves are unchanged — only the recommended handling.

> **Status update 2026-06-10 (Fable).** The plan's A-bucket has since landed, so the present-tense
> claims below describe the *as-reviewed* state, not the current tree. Now FIXED and verified
> (suite fully green, 112 lib + 3 integration): item 1 (both stale corpus counts — `pgn4_replay.rs`
> AND a second one this review also missed in `pgn4_roundtrip.rs`; floors recalibrated to ≥5000
> plies / ≥15 fully from observed 5058/7477, 15/32), item 3 (doc comments state flashlight-only
> scope), 5a (flags require `=1`), 5b (protocol recomputes Zobrist after replay self-sync), 5d
> (`eval_scalar` comment), 5e (perf assertion: strict 600 µs budget now opt-in via
> `HORNET_PERF_ASSERT=1`, always-on 3000 µs catastrophic backstop — 5e had fallen through the plan;
> picked up 2026-06-10), and item 6's STATUS/ENGINE-HANDOFF rows. Still open: item 2 (plan B1/B2,
> Opus, in flight), C1 (Kimi), and item 6's spec rows (drafted as CO-004/CO-005, awaiting user
> approval — Tier 2).

---

## Findings, Scored

### 1. ✅ REAL — `corpus_games_replay_against_move_gen` fails (stale fixture count)

**Status:** Confirmed. `cargo test` fails right now.

```
assertion `left == right` failed
  left: 32
 right: 16
```

`baselines/` grew from 16 → 32 `.pgn4` files (human games collected over time). The test at `pgn4_replay.rs:199` still asserts `games == 16`, and the `>= 2500 plies` / `>= 8 fully-replayed` baselines below it were calibrated for 16 games. This is **not** an engine regression — the 112 lib tests all pass — but the repo doesn't pass its own gate.

**Why this matters more than usual:** `agent-conduct.md` §1.12 requires "cargo test green" before closing any shift. `STATUS.md` claims "lib suite 111 green" (true) but doesn't distinguish that the *integration* test fails. `ENGINE-HANDOFF.md` claims "106 lib tests green" (also true, but stale count). A new agent following the conduct doc will hit a red test on first run and lose confidence in the handoff.

**Fix:** Update the assertion to `games == 32`, recalibrate the ply/game baselines for 32 games, and verify the replay still passes.

---

### 2. ✅ REAL — `count_defenders` has inverted polarity + ships default-on against Hard Rule #6

**Status:** Confirmed by code inspection.

At `move_order.rs:111`:
```rust
if p.player != victim_player {
    // Enemy piece nearby — might defend
    defenders += 1;
}
```

A "defender" of the victim is counted when `p.player != victim_player` — but a piece that can recapture is one of the *victim's own* pieces or allies. As written it counts the **attacker's own pieces and bystanders** as "defenders" while ignoring actual defenders. The "free capture" bonus fires on **defended** pieces and misses genuinely free ones — roughly backwards.

**Worse:** `FFA_BOUNTY_MOVE_ORDER` and `FREE_CAPTURE_BONUS` are both hardcoded `true` (`move_order.rs:17, 20`). `PITCH-ffa-bounty-scoring.md` and `agent-conduct.md` Hard Rule #6 require **default-off + ablation arm** for every new lever. This is shipping default-on, in violation of our own additive discipline.

**Handling (per plan B1 + B2 — outcome-affecting, not a no-op cleanup):** flipping these flags changes which moves survive the beam, hence the played move, hence self-play data. So it ships as a *measured* change, not a silent ablation:

- **B1 — land both flags `false` with a 3-arm measurement.** `FFA_BOUNTY_MOVE_ORDER` and `FREE_CAPTURE_BONUS` are independent levers and only the free-capture path carries the bug, so measure (i) both on — the de-facto baseline every recorded number used; (ii) free-capture off / bounty on — isolates the bug, this delta is the contamination estimate; (iii) both off — the Hard-Rule-#6 landing state and new baseline. `move_match` + a short seeded self-play A/B per arm, maxn at beam 30 and beam 4 (the bug's effect concentrates at narrow beams).
- **B2 — fix-or-delete `count_defenders`, decided by measured cost.** The polarity is inverted and the geometry adjacency-only. The cheap correct candidate is `is_attacked_by(board, m.to, victim.player)` — one attack scan answering "does the victim's side defend this square" — **not** `LineMap`: ordering runs at interior nodes where no LineMap exists (lines are built at leaf-eval), so the inverse index would mean projecting lines per node, the exact cost the original shortcut avoided. Measure the per-scan cost; if it's unacceptable, delete the function and the bonus path outright (the honest call, not a fallback). Reintroduce free-capture only as a default-off lever that earns its place in the B1 harness. Remove the dead `match`/`pawn_deltas` scaffolding either way.

---

### 3. ✅ REAL — `with_win_term` / `with_king_danger` doc comments are misleading

**Status:** Confirmed by code inspection. Not a wiring gap — a documentation gap.

Fable initially read this as a bug: `eval_with_win` (the win/danger layer) is called only inside `search_flashlight`, while `maxn`, `qsearch`, and terminal nodes call `(self.eval)` directly (search.rs:462, 469, 481, 509, 583). So `with_win_term()` and `with_king_danger()` silently do nothing on `search()`.

**Context Fable was missing:** This is **intentional**. The flashlight is the *chosen* search shape going forward (`ENGINE-MATH` §3: "laser discarded, flashlight validated == exact Max^n at infinite cap"). `selfplay_ab.rs` — the A/B harness that gates the win term — uses `search_flashlight` exclusively. The `search()` path is the legacy beam-Max^n, kept for comparison and the protocol's `go` command.

**However:** The builder doc comments say `with_win_term` affects "the search value" with no hint that `search()` ignores it. A new agent (or Fable, blind) reasonably expects it to apply everywhere. This is **misleading documentation**, not a bug.

**Fix:** Update the doc comments on `with_win_term` and `with_king_danger` to explicitly state: "Applies to `search_flashlight` only; `search()` uses the static eval (points-blind, Hard Rule #8)."

---

### 4. ⚠️ REAL BUT DELIBERATE — Zero-weight queries compute and discard

**Status:** Confirmed. `W_POSITIONAL = 0`, `W_SAFETY = 0`, yet `run_all_queries` computes positional control, threats, PST, and the full king-safety scan every call, then multiplies them by zero in `compute_utility`.

**Context:** This is a deliberate stopgap. `SYNTHESIS-next-attempt.md` and `ENGINE-MATH` document that positional is *noise* and safety is *significantly negative* (the huddle trap). The deployed weights `(6,0,0,1)` are validated by move-agreement tuning (EXP-015). Gating the queries on their weights would be a perf win, but it's a **code-complexity trade-off**: the query engine is designed as a single `run_all_queries` call, and splitting it introduces branching that complicates the hot path. The 600µs debug-mode perf assertion already passes; release builds are faster.

**Verdict:** Not a bug — Fable's perf point is valid. Reconciled to plan **C1**: rather than waiting for the eval rebuild, it's scheduled now as Phase A perf work, because it lands independently of B1 and its acceptance is cheap to prove — eval-output equality on a position sweep plus a perf number. Skip only what the weights zero out; keep the king-safety path available to the search-side danger term that reads it independently.

---

### 5. Smaller items — mixed bag

#### 5a. ✅ REAL — Env flags use `is_ok()` (footgun)

`HORNET_SEE=0` turns SEE **on** (`queries.rs:363`). Same for `HORNET_SELECTIVE_INTENT`. This is a classic footgun: any non-empty string (including "0", "false", "no") evaluates to `true`.

**Fix:** Parse properly — `std::env::var("HORNET_SEE").map(|s| s == "1").unwrap_or(false)`.

#### 5b. ✅ REAL — `apply_ply` writes `board.side_to_move` directly without updating Zobrist

`protocol/mod.rs:99` sets `board.side_to_move = p.player` directly in the normal-move branch; the castle branch does it in a loop over all four players. When the move list is in sync this is harmless (the move's own make/unmake handles Zobrist), but on a desynced replay the running hash diverges silently.

**Fix:** Use `make_move` / `unmake_move` for the side-to-move transition, or explicitly recompute Zobrist after the direct write. Low severity — only affects replay debugging.

#### 5c. ❌ NOT A BUG — En-passant target lifetime

Fable raised uncertainty about whether the EP target should persist >1 ply in 4PC. The spec §1.6 says "exactly one ply," and the code implements exactly that. Fable's uncertainty resolves in the engine's favor — no action needed.

#### 5d. ⚠️ DOC DRIFT — `eval_scalar`'s doc comment

"avoids allocating the full vector" — it literally calls `eval_4vec` and indexes it. This is a stale comment from before `eval_scalar` was implemented. Fix: update or delete the comment.

#### 5e. ⚠️ TEST FLAKINESS — 600µs debug-mode perf assertion

`eval.rs:161` asserts `avg_us < 600.0` in debug mode. This is machine-dependent and will flake on slower hardware. The test is already gated `#[cfg(test)]` and debug builds are inherently variable. **Fix:** Either remove the hard threshold (keep the timing as informational only) or gate it behind a feature flag.

---

### 6. ⚠️ DOC DRIFT — STATUS / HANDOFF / spec stale in multiple places

Fable flagged several instances of documentation lying to the reader:

| Doc | Claim | Reality |
|-----|-------|---------|
| `STATUS.md` (2026-06-07) | Node budget "deprecated" (cut mid-rotation, unsound) | `protocol/mod.rs:26` still ships `go` with `.with_node_budget(2_000_000)` |
| `ENGINE-HANDOFF.md` | "protocol not yet wired" (build section) | Item #3 says protocol DONE |
| `ENGINE-HANDOFF.md` | "96 lib tests, all green" | Actually 112 lib tests now |
| `HORNET-BUILD-SPEC.md` §4.5 | Old `enemy_value × enemy_count` crossfire | Fixed to SEE material-at-risk in EXP-005 |
| `HORNET-BUILD-SPEC.md` Appendix | Weights `(1,2,1,1)` | Deployed `(6,0,0,1)` |
| `HORNET-BUILD-SPEC.md` §2.5 | Board struct with piece lists, cached king squares, packed castling byte | Never built that way |

**Assessment:** This is the **same risk the 2026-06-05 review already flagged** about §1.4 — now in more places. For a project whose conduct doc names the spec as the authoritative reference a new agent builds against, stale spec text is a **process hazard**. CO-002/CO-003 track the rules discrepancies but not these implementation drifts.

**Fix:** Refresh `ENGINE-HANDOFF.md` (it's frozen at 2026-06-02, five days stale). Update the spec Appendix weights and crossfire formula to match deployed code. File a CO for spec §2.5 Board struct if it's permanently different. Update `STATUS.md` to note the integration test failure.

---

## What Fable Got Right That We Didn't See

The blind review methodology is **genuinely valuable** for this project. Process docs can't catch:

1. **Stale test assertions** — the 16→32 corpus growth was gradual; nobody noticed the hardcoded count.
2. **Inverted heuristics shipping default-on** — `count_defenders` was written, reviewed, and merged without anyone testing its polarity. The ablation discipline (Hard Rule #6) exists precisely to catch this, but the flag was hardcoded `true`.
3. **Doc-comment drift** — `with_win_term` saying "search value" when it means "flashlight search value" is a small lie that compounds across agent handoffs.

These are **exactly** the kind of "code honesty" issues that accumulate in a multi-agent project. Fable found them in one pass.

---

## What the Docs Add That Fable Missed

Fable's code-only pass couldn't see:

- **The strategic picture is further along than the code suggests.** The eval problem is diagnosed down to the objective function (EXP-016). The next moves (objective layer + rebuilt safety + relational terms, gated on self-play) are designed and partially built. The real blocker is **data** (decisive game outcomes), not ideas.
- **The flashlight is the chosen search shape.** `search()` is legacy; `search_flashlight` is where all new development lives. The protocol's `go` command hasn't been updated to call it — that's a real TODO, not a bug.
- **The 32-game corpus is explicitly acknowledged as noisy.** `SYNTHESIS-next-attempt.md` says "positional is noise here, after 8 variants" — so zero-weighting it is a *validated* choice, not a missing feature.

---

## Recommended Action

**Accept Fable's concrete cleanup offer.** Priority order:

| Priority | Item | File | Effort | Status (2026-06-10, Fable) |
|----------|------|------|--------|----------------------------|
| P0 | Fix replay test count (16→32) + recalibrate baselines | `tests/pgn4_replay.rs` | 5 min | ✅ done (+ second stale count in `pgn4_roundtrip.rs`) |
| P0 | Flag flip — **both** flags `false` + 3-arm measurement (plan B1), then fix-or-delete `count_defenders` by measured cost (plan B2) | `src/move_order.rs` | measured task — tracked in plan, not a quick fix | ⏳ in flight (Opus) |
| P1 | Fix env-flag parsing (`is_ok` → `== "1"`) | `src/queries.rs` | 5 min | ✅ done |
| P1 | Update `with_win_term` / `with_king_danger` doc comments | `src/search.rs` | 5 min | ✅ done |
| P1 | Fix `apply_ply` Zobrist desync | `src/protocol/mod.rs` | 10 min | ✅ done (recompute before search) |
| P1 | Refresh `ENGINE-HANDOFF.md` (test count, protocol status, build instructions) | `ENGINE-HANDOFF.md` | 15 min | ✅ done |
| P2 | Update spec Appendix weights + crossfire formula | `HORNET-BUILD-SPEC.md` | 10 min | 📋 drafted (CO-004/CO-005, awaiting user) |
| P2 | Fix `eval_scalar` stale comment | `src/eval.rs` | 2 min | ✅ done |
| P2 | Remove or gate 600µs perf assertion | `src/eval.rs` | 5 min | ✅ done (strict opt-in `HORNET_PERF_ASSERT=1`; 3000 µs backstop) |
| P2 | Update `STATUS.md` to note integration test failure | `STATUS.md` | 5 min | ✅ superseded — failure fixed; STATUS records suite green |
| — | Gate zero-weight queries (now scheduled as plan **C1**, Phase A; eval-output-equality + perf gated) | `src/queries.rs`, `src/eval.rs` | perf — tracked in plan | ⏳ open (Kimi) |

**Total: ~55 minutes** for the self-contained doc/test/parsing items. The two outcome-affecting search-lane items — the `count_defenders` flag flip + fix-or-delete — are deliberately *not* in that estimate; they're measured tasks tracked as plan B1/B2, not quick cleanup.

---

## Meta: On the Review Itself

Fable's review is **unusually disciplined** — the per-module coverage, the test suite run, the line-numbered citations, the separation of "code-only" from "docs-informed" findings. The methodology (blind first, then docs) is exactly right for a multi-agent project: it catches the drift between "what the code does" and "what the docs say it does."

The two reviews (Claude's 2026-06-05 review and Fable's 2026-06-09 review) agree more than they disagree, which is the strongest validation we could ask for. Fable's unique contribution is the **concrete cleanup list** — small, fixable items that directly improve correctness and process hygiene.

**My recommendation: merge the fixes, thank Fable, and add "blind code review" to the periodic audit checklist.**

---

*Reply written by Kimi, 2026-06-09.*
*Cross-reference: `REVIEW-claude-on-hornet-state-2026-06-05.md`, `SYNTHESIS-next-attempt.md`, `ENGINE-MATH.md`, `agent-conduct.md` §1.12.*
