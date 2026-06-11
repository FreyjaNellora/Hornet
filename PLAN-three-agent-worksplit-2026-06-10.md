# Plan — three-agent work split (2026-06-10)

**Inputs:** Fable's blind code review + doc-pass follow-up · Kimi's verification + commentary ·
Opus's review (`REVIEW-claude-on-fable-review-2026-06-10.md`). All three converge on the same
finding list; this plan assigns the work by lane and by demonstrated strength, with the
dependencies made explicit so nobody re-baselines on sand.

**Assignment logic:** Opus takes the search/move-order lane (claude's lane; the outcome-affecting
fixes that need measurement). Kimi takes the eval lane (her files; the feature program that
consumes the re-baselined metrics). Fable takes cross-cutting code hygiene, test infrastructure,
and doc reconciliation (the defect class the blind pass exists to catch).

---

## Sequencing (the only hard ordering)

```
Phase A (parallel):  Fable A1-A5    Opus B1 (3-arm flag measurement)    Kimi C1 (gateless perf)
Phase B:             Opus B2-B3     Kimi C2 (after B1 lands: new move_match baseline)
Phase C:             Opus B4 → corpus regen (B5)    Kimi C3 (gated on the new corpus)
```

*(Corrected 2026-06-10 after Opus's plan review: C1 is gateless perf work and belongs in Phase A;
C2 is the item that waits on B1's baseline — the first draft swapped them.)*

Rule for everything below: **anything that changes the played move ships default-off with a
measured arm** (proposed Hard Rule amendment, see B4). Doc-only and test-only changes are exempt.

---

## Fable — code hygiene, test infra, doc reconciliation

Strengths used: cold-pass code reading, spotting code/doc divergence, byte-level verification.
No strength-affecting changes in this bucket; everything is Tier 0–1 except the spec COs.

- **A1. Fix the suite gate.** `tests/pgn4_replay.rs:199` — corpus is 32 games, not 16. Update the
  count, then recalibrate the `>= 2500 plies` / `>= 8 fully` floors by running the replay against
  all 32 and recording actuals (floor slightly below observed, same convention as before).
  Acceptance: `cargo test` fully green from a clean tree.
- **A2. Env-flag parsing.** `queries.rs:363,367` — `is_ok()` means `HORNET_SEE=0` enables the
  flag. Change both to parse the value (`== "1"` or equivalent). Doc the convention once.
  (Kimi's files — one-line behavioral fix to flag *parsing*, not eval behavior; route past her.)
- **A3. Doc-comment corrections.** `search.rs` — `with_win_term`/`with_king_danger` say they
  affect "the search value"; they only reach `search_flashlight`, never `maxn`/`search()`. Say so
  explicitly. Also fix the `eval_scalar` comment in `eval.rs` (it does not avoid computing the
  vector) and the `protocol/mod.rs` `apply_ply` note (direct `side_to_move` writes desync the
  running zobrist on out-of-rotation replays — document or recompute).
- **A4. State-doc reconciliation.** STATUS.md ("suite green" is false until A1; node budget
  "deprecated" but live in protocol until B3), ENGINE-HANDOFF.md (internal contradiction:
  "protocol not yet wired" vs "protocol DONE"), HANDOFF.md (stale at 2026-06-02). Add the
  **corpus contamination note**: bootstrap corpus (maxn, beam 4) = generated under the inverted
  ordering heuristic, regenerate before use (B5); `selfplay_ab` / EXP-017/018 results = clean
  (flashlight never calls `move_order`); wide-beam maxn runs = mildly affected at most.
- **A5. Spec change orders (drafts only — spec edits are Tier 2 + CO).** Draft CO-004: §4.5 still
  specifies the pre-EXP-005 `enemy_value × enemy_count` crossfire; replace with the SEE
  material-at-risk definition. Draft CO-005: appendix weights `(1,2,1,1)` → deployed `(6,0,0,1)`
  with pointer to EXP-015; §2.5 Board struct (piece lists / cached king squares / packed castling
  byte) → as-built layout. User approves before landing.

## Opus — search lane: ordering fixes, measurement, protocol config

Strengths used: sequencing discipline, verification rigor, the search lane is claude's lane.
B1–B2 are the outcome-affecting items; both ship with numbers, not assertions.

- **B1. Flag flip + 3-arm measurement (first, everything downstream re-baselines on it).** The
  two flags are independent levers and only one carries the bug: `count_defenders` is called
  solely inside the `FREE_CAPTURE_BONUS` block (`move_order.rs:160-164`); `FFA_BOUNTY_MOVE_ORDER`
  (line 153) has no identified defect. A single combined flip cannot separate "how much did the
  inverted heuristic contaminate existing data" from "how much does the (un-bugged) bounty
  ordering merely change play" — and the first question is B1's whole purpose. So measure three
  arms, fixed seeds, maxn path at both beam 30 and beam 4 (the bug's effect concentrates at
  narrow beams):
  1. both ON — the de-facto baseline every recorded number was measured under (11.7% move-match,
     EXP-015's 13.5%, blunder rates, the bootstrap corpus);
  2. `FREE_CAPTURE_BONUS` off, bounty on — isolates the bug; this delta is the contamination
     estimate;
  3. both off — the Hard-Rule-#6 landing state and the new recorded baseline.
  Run `move_match` and the short seeded self-play A/B per arm. Land with both flags `false`;
  record all deltas.
- **B2. `count_defenders` — fix or delete, decided by a measured cost.** The polarity is inverted
  (`p.player != victim_player` counts the capturer's pieces and bystanders as "defenders") and the
  geometry is adjacency-only. The shortcut existed for a reason the first draft of this plan
  glossed: ordering runs at interior nodes, where **no LineMap exists** (lines are computed at
  leaf-eval time), so "use the inverse index" would mean projecting lines per node — exactly the
  cost the author avoided (`move_order.rs:82`). The cheap correct candidate is
  `is_attacked_by(board, m.to, victim.player)` — a single attack scan answering "does the victim's
  side defend this square," the same machinery check/castling legality already pay per move.
  Caveats to note: it ignores the discovered-defense case (capturer vacating `m.from` can open a
  defender's line) — acceptable for ordering — and it costs one scan per scored capture; measure
  that before committing. If the measured cost is unacceptable, **delete the function and the
  bonus path** — that is the honest call, not a fallback. If fixed: reintroduce free-capture as a
  genuine default-off lever that must earn its place in the B1 harness. Remove the dead
  `match`/`pawn_deltas` scaffolding either way.
- **B3. Protocol play config.** `protocol/mod.rs:23-26` still ships the maxn path with the
  node budget STATUS deprecated (cut mid-rotation, unsound). Align `go` with the post-EXP-012/016
  recommendation (flashlight + generous cap, or maxn with the hard per-node cap — pick per the
  current state docs and say which). Acceptance: `go` config matches what STATUS says the engine
  plays.
- **B4. Hard Rule amendment (draft; Tier 2, user approves).** "Anything that changes the played
  move — move ordering, beam width/shape, LMR, TT-hint usage, killer/history — ships default-off
  with a measured self-play arm, same gate as eval changes." Include the audit corollary: killers,
  history, and the TT best-move hint are outcome-affecting in narrow-beam configs and belong under
  the same gate going forward (no current bug found; the rule is preventative).
- **B5. Bootstrap corpus regeneration (after B1-B2, with Kimi on config).** Regenerate
  `selfplay_games/` under the fixed ordering and the current objective-layer defaults. Joint
  decision with Kimi on search shape (flashlight per SYNTHESIS) and on the aggression/decisiveness
  config, since drawish labels were the corpus's other defect.

## Kimi — eval lane: perf gating, feature program on the clean baseline

Strengths used: eval design (the independent plan that improved on the in-house build),
tuning methodology, her files. C2-C3 are her existing merged plan — unchanged, just re-anchored
to the post-B1 baseline so feature deltas aren't measured against a moving target.

- **C1. Gate the zero-weight queries.** `run_all_queries` computes positional control, threats,
  PST, and the king-safety scan at every leaf and multiplies by `W_POSITIONAL = W_SAFETY = 0`.
  Skip what the weights zero out (keep the king-safety path available to the search-side danger
  term, which reads it independently). This is a pure-perf change to the hot, always-recompute
  path; acceptance is eval-output equality on a position sweep + a perf number. Independent of
  B1 — can land any time.
- **C2. Safety rebuild + objective layer A/B (the merged plan).** Per
  `REVIEW-claude-on-kimi-independent-plan.md`: non-linear attack-units danger table (her shape),
  mean-relative vs absolute placement A/B'd, both win signals (banked points + elimination
  proximity) in all combinations. Gate on self-play. **Wait for B1's new move_match baseline
  before reading any move-agreement numbers** — the old 11.7%/18.3%/13.5% figures are
  pre-flag-flip.
- **C3. Relational terms, on the regenerated corpus.** Her unbundled plan (iso/doubled pawns
  first, rook-open, outpost, targeted mobility), Texel/move-agreement gated — but against the B5
  corpus, not the tainted 133-game bootstrap set. The 16→32-game human corpus from A1's
  recalibration also feeds this. Existing rule stands: a feature is kept only on a measured gate.

---

## What this plan deliberately does not do

- No NNUE work (Hard Rule #7 — strength gate first; unchanged).
- No shallow-pruning revival (still low-ROI per the deferral note; B4's rule change doesn't
  reopen it).
- No re-run of EXP-017/018 — the flashlight path never touches `move_order`, so the
  objective-layer results are clean. **Verified, not assumed** (Opus, 2026-06-10): the only
  `move_order::order` calls in `search.rs` are at lines 412 (`root_move_values`) and 432
  (`search_depth`), both maxn drivers; `search_flashlight` (lines 293–397) prunes directly off
  `eval_with_win` (line ~361). Recorded here and in A4 so nobody burns compute re-proving it.
