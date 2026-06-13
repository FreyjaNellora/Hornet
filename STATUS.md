# Hornet — Production Board (STATUS.md)

**Updated:** 2026-06-12 end-of-cycle (**Gates cycle CLOSED — nothing earned a flip; the
discipline held three times.** **EXP-031 objective layer (win term + king danger) CLOSED at
both weights:** w50 = 34 unique pairs (after seed-collision dedup; harness gained a seed-offset
arg) +8.7% pts, p≈0.09; w100 = 24 pairs +9.5% pts, p≈0.11 (its first-12 p≈0.044 was the lucky
half — the fresh 12 were dead even). A remarkably consistent ~+9% lean across 58 pairs that
never clears the bar; post-hoc family pooling t(57)=1.86 p≈0.034 recorded as suggestive only.
**Defaults stay plain flashlight cap 1200.** Next objective-layer move = the mining-nominated
shape (target selection by MATERIAL weakness — elimination forensics: 67% of kill victims rank
last in material) + the human tester gate, not more pairs at the generic win term. **EXP-032
pawn-adv CLOSED: gate NULL** (6-6-0, 809-800) after its run-1 was VOID (selfplay_ab eval_id
silent fallthrough → both arms deployed, 12/12 exact ties; fixed, unknown ids panic). Fourth
prediction≠play instance — +1.5pp winners-only agreement bought zero strength. **Corpus:
`human_games/` = 143 standard-array games (188 unique total after the user's 48-game batch:
ingest script renumber bug fixed + post-ingest GameNr validation added; one prior-batch dup
removed; 1 corrupt header repaired). DISCOVERED: 45 of the batch are `StartFen4 "4PCo"` = the
RETIRED OLD ARRAY (Blue/Green royals exchanged; proven by replay trial ladder; GameNr timeline
shows it dying ~50M) → dispatch ruled it a VARIANT, quarantined to `human_games_4pco/`, ingest
auto-routes by StartFen4. Engine canonical start = current chess.com array, confirmed.** Lib
reads both arrays (`START_FEN4_4PCO`, pinned test). New tool `examples/corpus_check.rs` (per
batch: gate/replay health + failure classification). Mining program (NOTE-behavior-mining.md)
passes 1–3b done: winners farm middle/kill weakest-by-material, promotions 2.6× (biggest
differential), king-raid gap every phase, denial NOT differential, trades not statically better
— just unpunished (tempo). Tester loop live: `examples/play.rs` → emailable PGN4 reports →
`versus_games/`. Texel re-baseline 143 games/10.7k positions: ISO only robust term, WIN null.
Suite **117 lib + 1 variant + 3 integration green**. Queued: 4PCo castle tail; repetition/
50-move/claim rules (needed before wide tester play); material-weakness targeting term. Prior:
DKW rules EXP-026/CO-007; shared replayer; C1 +41% nps; B1–B5; recalibration EXP-005→009.)
**State store — replaced, not appended.** History lives in `sessions/` and `dispatch_comms.jsonl`.

Architecture/reference = `HORNET-BUILD-SPEC.md` (§9 file structure defines the module tree).
Per-phase acceptance criteria live in each `phases/{phase}.md`. This board shows only current state.

## Phases (stations)

| Phase | Name | Owner | Status | Next action |
|-------|------|-------|--------|-------------|
| P0 | Spec / Reference | Kimi (landed by claude, CO-001) | **v0.2 LANDED; spec synced to as-built** (CO-002…006 all resolved 2026-06-10) | — |
| **P1** | **Board I/O** (FEN4/PGN4 + types) | **claude** | **complete** ✅ (FEN4 byte-identical; PGN4 round-trip + `decode_ply`; corpus replay 5058/7477 over 32 games) | — |
| **P2** | **Move generation** | **claude** | **complete** ✅ (perft `20/395/7800/152050`; castling, EP, promotion fix; **DKW** ✅ 2026-06-07) | — |
| **P3** | **Line projection** (`lines.rs`) | **claude** | **complete** ✅ (§7.2, X-ray, inverse index) | — |
| **P4** | **Query engine** (`queries.rs`) | **Kimi** | **complete** ✅ (material/positional/safety/crossfire; 7 tests) | — |
| **P5** | **Evaluation** (`eval.rs`) | **Kimi** | **complete** ✅ (`eval_4vec → [i16;4]`; 4 tests) | — |
| **P6** | **Search** (`search.rs`, Max^n) | **claude** | **core + refinements** ✅ (Zobrist + TT + beam Max^n; terminal §1.8, ID, killers+history; **quiescence/TRS + node budget** 2026-06-06) | shallow pruning **deferred** (low-ROI: `COMP_LB` loose → cutoffs ~never; see `experiments/NOTE-shallow-pruning-deferred`) |
| P7 | NNUE (`nnue/`) | Kimi | not-started | After the strength gate (Hard Rule #7) |
| P8 | Protocol (`protocol/`, UCI-like) | **claude** | **wired** ✅ (2026-06-06: `position startpos/fen4/pgn4 [moves]` + `go [depth]` → `bestmove`; **B3 2026-06-10:** `go` plays the flashlight at cap 1200 per SYNTHESIS — deprecated node-budget maxn config removed) | — |

## Critical path

`P0 ✅ → P1 ✅ → P2 ✅ → P3 ✅ → P4 ✅ → P5 ✅ → P6 ✅ (core)`. The full board → move pipeline runs.
⏭ **Eval recalibrated — scale bug FIXED (EXP-005→008).** Crossfire `value×count` → SEE material-at-risk
(Kimi), safety → clamped centipawn danger (Kimi), `ffa_points` bounty lifted out of Oᵢ (#8), weights
`4/2/1/1`→`4/1/1/1` (claude). Calibration gate: quiet-move eval swing **1294→276**, captures track
material, suite green (101), **0 blunders**, engine takes free material. The thousands-swings that made
depth (EXP-001) and SEE (EXP-002) useless are gone. **The match-rate gate is now exhausted as a tuning
signal (0–2/13 = noise); productive further tuning needs a strength metric (self-play or blunder-rate
over many positions), not weight-twiddling.** Shallow pruning unblocked (zero-sum) = orthogonal search
win — **but DEFERRED as low-ROI:** zero-sum fixes `SUM_UB`, but `COMP_LB` (per-player lower bound) is
deeply negative so provable cutoffs fire ~never (known Max^n weakness); the speed is already banked by
forward pruning (LMR+adaptive, 12–28×). Revisit only via clamped bounds if a speed wall hits. Strength
gate (Hard Rule #7) gates P7 NNUE.

**Eval tuning infra landed (EXP-009):** `examples/texel_tune.rs` fits eval weights to corpus game
outcomes (PGN4 `[Result]` points → placement; sigmoid + MSE; queries cached → runs in seconds) — the
classical hand-eval method (`REFERENCE-eval-tuning.md`). Finding: the eval **predicts outcomes better
than chance (MSE 0.1146 vs 0.14)** and the **4 weights are already optimal** — further eval gains are
in the *features* (queries), not the linear weights, now tunable against outcomes via Texel. The
outcome-MSE is the config-comparison metric replacing the noisy move-match rate; self-play A-vs-B
(true Elo) is the deferred gold standard (expensive: full games × per-ply search). Blunder-rate
metric (`gate_ablation.rs`) on the recalibrated eval: ~1% capture-into-loss, avg 12cp newly-hung.

## Active blockers / watch items

- ✅ **Move-order bug RESOLVED (B1/B2, Fable, 2026-06-10, EXP-020/021):** both ordering flags are
  now default-off `OrderState` fields (builders `with_ffa_bounty_order`/`with_free_capture_order`;
  guard test pins the defaults); `count_defenders` replaced by a real attack scan (cost ≈ 0,
  polarity regression test). **Re-baseline anything comparing to pre-flip maxn numbers** — new
  move_match baseline (arm iii): 13.5%/13.6%/13.6% at beams 4/10/30 (32 games, d4, S2).
- ✅ **Corpus REGENERATED (B5/EXP-023 complete, 2026-06-11):** `selfplay_games/` = 150 clean
  games (flashlight d8 cap 1200 + objective layer win 50/danger 100, 200-ply cap, seeded).
  **Decisiveness 55/150 = 37% ≥1 completed elimination** (old corpus ~0); wide point spreads.
  First clean-data Texel fit confirms the deployed weight shape (P=S=0, O=1, M dominant; combined
  corpus 241 games / 13,924 positions, MSE 0.1295). **Tune-freeze lifted — C3 unblocked.** Old
  tainted corpus in git history (≤ 6a2b6a9). EXP-020 context: the old corpus's inverted heuristic
  changed 11.6% of moves at beam 4 (0.9%/0.6% at 10/30); EXP-017/018 flashlight results were
  always clean.
- ✅ **Protocol `go` config RESOLVED (B3, Fable, 2026-06-10):** `go` plays
  `search_flashlight` at cap 1200 (SYNTHESIS: "flashlight + a generous cap (≥~1000), never the
  laser"); the deprecated maxn + 2M node-budget config is gone. Objective-layer knobs stay off
  until C2 passes its gate.
- ✅ **All open change orders RESOLVED (2026-06-10, user-approved, landed by Fable):**
  **CO-004** — spec §4.5 rewritten to SEE material-at-risk (mirrors `query_crossfire`; history
  line warns against `value × count`). **CO-005** — §2.5 Board rewritten to as-built (+ explicit
  "piece lists / cached kings deliberately not maintained"); §4.7 mean-relative formula + deployed
  weights `(6,0,0,1)` with do-not-hand-retune warning; appendix constants synced. **CO-006** —
  Hard Rule #6 amended to "anything that changes the played move" in `PITCH-for-new-agents.md` +
  `agent-conduct.md` §1.2 (basis: EXP-020's 11.6%). **CO-002/CO-003** — spec text had landed
  2026-06-06; stale "open" headers reconciled. The spec is now a faithful as-built reference.
- ✅ **P2 perft gate RESOLVED (2026-06-02):** Hornet computes `20/395/7800/152050`, matching Freyja.
  `perft(2)=395` is correct — the gap vs 400 is a discovered pin (vacating f2 opens the g1-queen's
  diagonal onto Blue's pinned b6 pawn). Now a regression test. See `COMMS_CLAUDE_PERFT_RESULT.md`.
- **CO-001** (✅ resolved 2026-06-01): v0.2 landed by claude (dispatch-authorized; no Kimi). PGN4
  now unblocked. PGN4 content lives in the new spec **§10**.
- **Two FEN4 dialects:** `tactical_samples.json` uses a non-native (`xxx`-corner) dialect → the
  **strength gate (P7)** will need a converter or re-export. Not blocking now.
- **DKW** ✅ (claude, 2026-06-07): dead-king-walking fully implemented (EXP-011). DKW pieces fully
  frozen (un-capturable); eliminated players' pieces removed. Corpus replay 5058/7477, 15/32 games full
  (the frozen rule diverges from the *takeable* corpus; the `DKW_PIECES_REMOVABLE` toggle restores
  ≈2846/10 — geometry confirmed).
- **Lanes (sync):** claude owns `board`/`move_gen`/`lines`/`zobrist`/`tt`/`search`/`move_order`; Kimi
  owns `queries`/`eval`/`nnue`. Interface changes to `board` route through claude. See `COMMS_CLAUDE_*`.

## Bootstrap state (after sessions 001–002)

Playbook runtime stood up: `agent-conduct.md`, this board, `phases/`, `sessions/`,
`change-orders/`, `dispatch_comms.jsonl`. **Spec v0.2 landed** (CO-001). Engine: `hornet-engine/`
crate with the spec-§9 module skeleton; `board/types.rs`, `board/mod.rs` (`Board`), `board/fen4.rs`
(byte-identical round-trip), `board/pgn4.rs` (structural parser, all 16 corpus games round-trip).
`cargo test` = **19 unit + 1 integration green**, clippy clean, fmt applied. Detail in
`sessions/phase-1/session-001.md` and `…/session-002.md`.
