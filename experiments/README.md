# Experiments log

Tracking for engine experiments — what was tried, under what conditions, and the result.
One file per experiment: `EXP-NNN-<slug>.md`. Newest entries at the top of the index.

Each experiment records **conditions before**, **method**, **results**, **conditions after**,
and a **conclusion**, so a later reader can tell what changed and whether it helped.

**Metric evolution (important — read before trusting old numbers).** This log opened with the
**strength-gate match rate** (engine's move vs the human's move over `baselines/tactical_samples.json`,
13 fixtures) as the signal. EXP-004/005 showed that is **noise** — exact-move-vs-one-human, 0–2/13
with no real signal. The metrics that replaced it:
- **Texel outcome-MSE** — does the eval predict game placement? (EXP-009; 0.1146 vs chance 0.14)
- **Calibration gate** — quiet-move eval swing (EXP-006; should be ~hundreds, not thousands)
- **Blunder rate** — engine moves that lose material over corpus replay (EXP-008)
- **Self-play** — true Elo via A-vs-B games (EXP-010)

Match rate is kept in the early entries for history only. Notes/decisions that aren't single
experiments live in `NOTE-*.md`.

## Template

```
# EXP-NNN — <title>

- **Date:** YYYY-MM-DD
- **Hypothesis:** one sentence — what we expect and why.
- **Lever / change:** the exact toggle or code under test (default-off flag, weight, etc.).

## Conditions (before)
Eval/search config, weights, flags, build profile, fixture set.

## Method
Exact command(s) and config. How on-vs-off is isolated.

## Results
The numbers (match rate, node counts, timing). Per-fixture if relevant.

## Conditions (after)
What is now true in the tree (flag default, what's wired, what's still off).

## Conclusion
What the result means; falsified or confirmed; next step.
```

## Index

**Notes (decisions/analyses, not single runs):**
- [NOTE — shallow pruning deferred (low-ROI)](NOTE-shallow-pruning-deferred.md) — zero-sum fixes `SUM_UB`, but `COMP_LB` is deeply negative so provable Max^n cutoffs fire ~never; speed already banked by forward pruning. Revisit only on a speed wall (via clamped bounds).
- [NOTE — eval-feature direction](NOTE-eval-feature-direction.md) — Pᵢ is the only V component without a piece-level base. Of its substrates, **pawn structure is the only likely Texel-mover** (mobility = cleanup, SEE-threats already null). `intent.rs` is a *threat* substrate, not mobility. Build pawn structure first, Texel-gate it.

**Experiments:**

*(Index gap: EXP-012…019 exist as files but were never indexed — see the experiment files
directly; backfilling the hooks is an open hygiene item.)*

- [EXP-025 — pawn structure unbundled (C3)](EXP-025-pawn-structure-unbundled.md) — Kimi's
  isolated/doubled/connected count queries fitted on the clean corpus. **First pawn signal above
  the noise floor: ISO drop 0.00054 (~11× floor)** (CONN borderline, DBL null; joint fit
  collinear — single-term marginals are the read). Fitted ISO scale ≈ −290 cp/pawn is
  symptom-vs-cause suspect — deployment = the recorded P′-rebuild arm, nothing wired (wiring
  into zero-weighted P would be dead code).
- [EXP-024 — objective terms in the eval frame (C2 continuation)](EXP-024-objective-terms-in-eval-frame.md) —
  Kimi's C2 fold-ins were **inert as built** (terms folded into components multiplied by weight
  0) → reverted; tuner extended to 9 weights + dual-base marginals instead (self-check
  reproduces EXP-023 exactly). **Danger-table: real but material-entangled signal** (0.00051 at
  M=4, 0.00015 at deployed M=6) → S′ rebuild with tempered expectations; **win-proximity: null**
  statically. **Methodological headline: the null-control self-play arm (A≡B) read 83% win-rate
  / +65% points from pure variance** → 6-game arms resolve nothing, EXP-017's win-rate reads
  re-graded, powered gate = paired seat-swap + many games.
- [EXP-023 — bootstrap corpus regeneration (B5)](EXP-023-corpus-regeneration.md) — **complete:**
  150 clean games replacing the tainted corpus (EXP-020 11.6%): flashlight d8 cap 1200 +
  objective layer (win 50, danger 100), 200-ply cap, 5 parallel range instances.
  **Decisiveness 37%** (old ~0), wide point spreads; first clean-data Texel fit confirms the
  deployed weight shape (MSE 0.1295, 241 games / 13,924 positions). Tune-freeze lifted. Old
  corpus in git history (≤ 6a2b6a9).
- [EXP-022 — zero-weight query gating (C1)](EXP-022-zero-weight-query-gating.md) — skip the
  queries `W_POSITIONAL = W_SAFETY = 0` zero out (positional control, SEE threats, PST, the
  king-safety scan) at every leaf. **+41% search throughput (median 64,337 → 90,777 nodes/sec),
  node counts and best moves bit-identical** — pure perf, equality pinned by test;
  `run_all_queries` stays full for texel_tune; the search-side king-danger term is independent
  and unaffected. Queries auto-resume if a weight is un-zeroed.
- [EXP-021 — count_defenders fix: polarity + measured cost](EXP-021-count-defenders-cost.md) —
  the inverted adjacency-only `count_defenders` replaced by a real attack scan
  (`board::attacks::is_attacked_by`); polarity regression test added. **Cost ≈ 0** (median
  nodes/sec 64,337 off vs 65,813 on — noise; venue re-based beam 30→10 with justification: per-node
  ordering cost is beam-independent). Pre-ratified >10%-drop delete rule not triggered → **lever
  kept, default off**; its *strength* case still needs a powered self-play gate before it ever
  ships on.
- [EXP-020 — move-ordering flag ablation (3 arms)](EXP-020-move-order-flag-ablation.md) — the two
  ordering flags (`FFA bounty`, `free-capture`) moved const→`OrderState` fields **default-off**
  (Hard Rule #6 restored), behavior-preservation proven by exact golden-ref equivalence. **The
  inverted free-capture heuristic changed the played move on 11.6% of positions at beam 4**
  (0.9%/0.6% at beams 10/30) → the beam-4 bootstrap corpus is tainted ~1-in-9 moves (B5
  regenerates); landing flags-off costs nothing measurable (self-play noise; human-agreement flat
  13.4–13.7% everywhere). Arm (iii) = the new recorded move_match baseline. New instruments:
  `selfplay_ab_maxn` (maxn-path A/B), `move_diverge` (per-position behavior-change frequency).
- [EXP-011 — Dead-King-Walking (full implementation)](EXP-011-dead-king-walking.md) — the complete DKW lifecycle (§1.7/1.8) across board/move-gen/search(expectimax)/game-flow. DKW pieces are **fully frozen** (immovable + un-capturable by anyone, even the dead king); a fully eliminated player's pieces are **removed**. King-capture does **not** sweep in search (avoids over-valuing king-hunts; game-flow sweeps). **106 lib green** + DKW unit tests; corpus replay 2532/3770, 8/16 full (frozen diverges from the takeable corpus; `DKW_PIECES_REMOVABLE` toggle restores ≈2846/10).
- [EXP-010 — self-play harness (the gold-standard "vs")](EXP-010-selfplay-harness.md) — built `examples/selfplay.rs` (4 Searchers play a 4PC game from start, scored by FFA points; A rotated through all seats to cancel seat bias). **Works + tractable (~6 min/game).** The venue to re-test depth (EXP-001) + the speed levers in real play. First run (q-on vs q-off) in progress; one game is noise. Compares *search* configs; eval-weights use Texel (EXP-009).
- [EXP-009 — Texel tuning + outcome-prediction "vs"](EXP-009-texel-tuning.md) — built `texel_tune` (fits weights to corpus outcomes, runs in seconds). **Eval predicts outcomes better than chance (0.1146 vs 0.14); the 4 weights are already optimal.** Further gains are in eval *features*, not weights. MSE is the new config-comparison metric. (`REFERENCE-eval-tuning.md` = the Stockfish/Texel philosophy.)
- [EXP-008 — recalibration pass 2 (scale bug FIXED)](EXP-008-recalibration-pass2.md) — bounty lifted from O (#8) + weights `4/1/1/1`: **quiet swing 1294→276, captures track material, suite green, 0 blunders, takes free material.** Match-rate exhausted as a tuning metric (0–2/13 noise) → next needs a strength metric.
- [EXP-007 — recalibration pass 1](EXP-007-recalibration-pass1.md) — **gate not yet passed**: crossfire/safety fixed (captures halved, 2/13 match, 0 blunders) but eval.rs untouched (weights/bounty) + positional still unbounded + mean-relative amplifies. Quiet swing 1294→1172.
- [EXP-006 — the calibration gate (baseline)](EXP-006-calibration-gate.md) — single acceptance number: **quiet-move eval swing avg=1294 / max=3506** (should be ~tens). The pass/fail target for the recalibration.
- [EXP-005 — validate the harness (and find the real bug)](EXP-005-harness-validation.md) — **ROOT CAUSE: the eval is miscalibrated.** Data/replay are correct; a single move swings the static eval by thousands (crossfire `value × count`, queries.rs:227). Fix = eval recalibration, not features/search.
- [EXP-004 — quality metric (blunder vs different)](EXP-004-quality-metric.md) — **the gate itself is suspect**: engine rates human moves as −thousands cp, SEE flags human captures as material-losing → validate the harness (replay/eval/SEE) before more eval work. S02 = a trade, not a bug.
- [EXP-003 — inspect the 13 misses](EXP-003-inspect-the-misses.md) — **8/13 human moves are quiet**; misses are positional + metric-harshness, not tactical. Engine isn't blind (wins a rook in S18).
- [EXP-002 — exchange-aware (SEE) threat scoring](EXP-002-exchange-aware-threats.md) — **null** (0/13 → 0/13); threats-only SEE doesn't move the gate → inspect the misses next
- [EXP-001 — depth × quiescence diagnostic](EXP-001-depth-quiescence-diagnostic.md) — eval is the bottleneck (depth confounded)
