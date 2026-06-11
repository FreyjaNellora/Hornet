# Hornet — engine catch-up (send this to onboard any agent)

**This is the single doc to bring any agent up to speed on Hornet's current state.** Read it
top-to-bottom; it is self-contained, with pointers to detail where needed. Hornet is a four-player
chess (chess.com FFA) engine in Rust (`hornet-engine/`).

**Current focus (2026-06-10).** P0–P6 complete: the full pipeline (board → move-gen → line projection
→ query engine → evaluation → Max^n search) runs end-to-end; **suite fully green (112 lib + 3
integration)** against the now-**32-game** human corpus (replay floors recalibrated: observed
5058/7477 plies, 15/32 games full — same DKW-divergence profile as before). The evaluator
was recalibrated (a crossfire `value × count` scale bug; fixed + Texel-confirmed weights are optimal →
**eval gains are now in the *features*, not the weights**). The **protocol is wired** (plays
end-to-end), **self-play** runs full games (`examples/selfplay.rs`, depth-12 ~3.7 min/game), and
**Dead-King-Walking is fully implemented** (board/move-gen/search/game-flow; DKW pieces fully frozen,
eliminated players' pieces removed). Shallow pruning is deferred (low-ROI — "What's left" #1).
**Known open defect:** `move_order::count_defenders` has inverted polarity and gates a default-ON
free-capture bonus — outcome-affecting at narrow beams; being fixed via a 3-arm measured flag flip
(see `PLAN-three-agent-worksplit-2026-06-10.md`). The `selfplay_games/` bootstrap corpus was
generated under that bug (maxn path, beam 4) — regenerate before tuning on it; flashlight-path
experiments (EXP-017/018) are unaffected (`search_flashlight` never calls `move_order`; verified).
Open next: the eval features.

**Where to read more:** `HORNET-BUILD-SPEC.md` (full spec) · `STATUS.md` (live production board) ·
`experiments/` (eval diagnostic→tune EXP-001…009; self-play EXP-010; DKW EXP-011) · `REFERENCE-eval-tuning.md`
(the Texel/Fishtest tuning method) · `PITCH-*.md` (design pitches).

## Architecture

One primitive drives everything. Per-piece BFS **line projection** (every piece's reach, with X-ray
past the first blocker and a per-square inverse index) feeds a **query engine** that produces a
per-player utility vector **V = ⟨U₁, U₂, U₃, U₄⟩**. A **Max^n** search backs V up, each node maximizing
the moving player's own component. The evaluation is a vector, never a scalar. A dense-MLP NNUE over
the query outputs eventually replaces the hand-tuned evaluator (once it passes the strength gate).

## Build & run

```
cd hornet-engine
cargo test     # 112 lib + 3 integration tests, all green (as of 2026-06-10)
cargo run      # UCI-like protocol REPL on stdin/stdout (position / go / bestmove)
```

Repo: `github.com/FreyjaNellora/Hornet` (current).

## What's implemented

- **Board & I/O** (`board/`): 14×14 board, 160 valid squares (four 3×3 corners removed); piece /
  player / square types with the two value systems (`eval_value` in centipawns vs `ffa_points`).
  Native **FEN4** parse/serialize (canonical start round-trips byte-identically) and **PGN4** parse +
  `decode_ply` (move-stream decoder; replays 5058/7477 real-corpus plies over 32 games as last
  observed — the regression test floors this at ≥5000 and ≥15 games fully, the unreplayed remainder
  bounding at Dead-King-Walking).
- **Move generation** (`move_gen.rs`, `board/attacks.rs`): legal moves for all pieces — castling
  (per-player tables), en passant (with the orthogonal-pairs rule), promotion at the central crossing
  (rank 7 / file 7 / rank 6 / file 6); make/unmake with full undo incl. king-capture elimination.
  **perft from the start = 20 / 395 / 7800 / 152050** (regression-tested).
- **Line projection** (`lines.rs`): `compute_lines(board, &mut LineMap)` — slider X-ray, knight/king
  steps, pawn push + always-on diagonals, per-square inverse index. Matches the spec §7.2 reach counts.
- **Query engine** (`queries.rs`): material, positional control, king safety, crossfire → `QueryVector`.
- **Evaluation** (`eval.rs`): `eval_4vec(board, &mut LineMap) -> [i16; 4]` via the fixed V
  decomposition (recalibrated — see "Eval state & tuning" below).
- **Search** (`search.rs`, `tt.rs`, `move_order.rs`, `board/zobrist.rs`): incremental Zobrist hash,
  transposition table (move ordering), beam Max^n (root full-width, internal beam), MVV-LVA ordering;
  search depth rounds up to a multiple of 4.

## What's left to build (priority)

1. **Max^n shallow pruning** — **DEFERRED (low-ROI, 2026-06-06).** The cutoff `UB_p = SUM_UB − best_q
   − 2·COMP_LB`: zero-sum fixes `SUM_UB` (~18), but `COMP_LB` (provable lower bound on one player's
   `Uᵢ`) is deeply negative (a player can be down most of their material, `Uᵢ ≈ −20k`), so
   `−2·COMP_LB ≈ +40k` swamps the bound and **provable cutoffs fire ~never** — the known weakness of
   Max^n pruning. The speed is already banked by **forward pruning (LMR + adaptive beam, 12–28×)**.
   Cutoffs would only fire with **clamped bounds** (pitch option 2), which change the eval at extremes
   for speed we already have. Revisit only if a speed wall hits; the one sound residual is TT-bounded
   values. See `PITCH-maxn-shallow-pruning.md`, `search.rs`.
2. **Dead-King-Walking** — **DONE (2026-06-07; EXP-011).** Full lifecycle (§1.7/1.8) across
   board/move-gen/search/game-flow. 3 player states: LIVE / **DKW** (`board.dkw[p]`, king walks
   randomly ignoring check, earns no points; its non-king pieces are **fully-frozen walls — immovable
   AND un-capturable by anyone**, even its own king) / **DEAD** (fully eliminated → **all its pieces
   removed** from the board). checkmate/stalemate → DKW (+20 stalemate); DKW-king capture or stalemate
   (+10 each survivor) → DEAD. `move_gen` emits king-only DKW moves; `is_wall` makes frozen pieces
   block-but-uncapturable (toggle `DKW_PIECES_REMOVABLE` for the removable variant); `in_check` is
   frozen-aware; `search` treats a DKW node as **expectimax** (king is random) and **does not** sweep
   on king-capture (that would over-value king-hunts — the sweep is game-flow only, in `game.rs`).
   Corpus replay **5058/7477, 15/32 games full** as of the 32-game corpus (frozen rule diverges from
   the *takeable* corpus where it captures DKW pieces; on the old 16-game corpus the removable toggle
   restored ≈2846/10 — geometry confirmed).
3. **UCI-style protocol** — **DONE (2026-06-06).** `position startpos | fen4 <fen> | pgn4 <path>
   [moves <ply>...]` + `go [depth N]` wired to the searcher; emits `bestmove <from-to>`; also
   `uci`/`isready`/`d`/`quit`. Strips a leading BOM; `bestmove` output round-trips back through
   `position … moves`, so it's drivable for external self-play. (`protocol/{mod,parse,output}.rs`;
   the playing config uses the forward-pruning + adaptive-beam levers + a 2M node budget so `go` is
   responsive.)
4. **Eval features → strength gate → NNUE** (`nnue/`): the v0 eval is recalibrated and predicts
   outcomes better than chance; its 4 weights are optimal, so **gains are in the features.** The
   concrete next one: **Pᵢ (positional) is the only V component without a piece-level base** (Mᵢ
   material, Sᵢ king intent, Oᵢ SEE attacker/defender all got tactical bases in the recalibration; Pᵢ
   is flat centrality-mobility), which is why Texel finds it signal-thin. Pᵢ's candidate substrates:
   - **Mobility** (restructure `query_positional_control`) — *cleanup only*; re-skinning the same
     reach×centrality geometry won't move Texel (mobility-Pᵢ is already thin → ~0 MSE delta).
   - **Threats** — already folded into Pᵢ (`query_threats`/`query_threats_se`); EXP-002 showed the SEE
     threat null. Not the adder.
   - **Pawn structure** (isolated / doubled / passed — new, bounded) — *the genuine signal-adder
     candidate*; classically predictive (caveat: 4PC central-promotion geometry may dampen it — the
     Texel delta judges). **Build this first.**
   Note: `intent.rs` is a **threat** substrate (per-piece per-opponent offense/defense/vulnerability —
   no mobility/directional-reach), so it can supplement Oᵢ/threats (distance/value-weighted), **not**
   Pᵢ-mobility. Every feature ships **default-off as an ablation arm** and is accepted only on a
   `texel_tune` MSE drop. The strength gate then gates NNUE. (Fixtures:
   `baselines/tactical_samples.json`, 13 testable via `moves_to_replay`; the `xxx`-corner FEN4 dialect
   still isn't parsed. `nnue/` dir is Phase-7 stubs.)

**Done since the prior handoff:** terminal scoring (§1.8, centipawn mate-distance), iterative
deepening, killers + history, quiescence (= "TRS", default-off), forward pruning (LMR) + adaptive
beam (default-off speed levers), per-search node budget, and the eval recalibration + tuning infra
below. (The §1.8 *point* awards — +20 stalemate, +10 DKW — remain game-scoring on `board.points`,
applied at play time, not in the centipawn search backup.)

## Eval state & tuning infrastructure (2026-06-06)

The v0 eval was scale-miscalibrated — one query (crossfire) used `enemy_value × enemy_count` (≈ value²
scale), swinging the eval by *thousands* per move and drowning material, while king-safety was scaled
to near-invisibility. A single quiet move swung the eval ~1300+ (now ~hundreds). Fixed:
- **Crossfire (Oᵢ)** = SEE-resolved material-at-risk, bounded by victim value (not value×count).
- **Safety (Sᵢ)** = clamped centipawn danger (attacker value net of defenders), no longer single-digit.
- **Bounty** (`ffa_points`) **lifted out of Oᵢ** — the evaluator is points-blind again (Hard Rule #8);
  the FFA-hunt preference lives in move ordering. (`bounty.rs` is now dormant w.r.t. the eval.)
- **Weights** `4/1/1/1` — material high to offset the mean-relative dilution of a free piece; the
  now-bounded heuristics at 1.

Result: captures track material, the engine takes free material and ~never captures into a loss
(blunder-rate ≈ 1% capture-into-loss, avg 12 cp newly-hung over 150 corpus positions).

**Tuning is outcome-based, not move-match.** The exact-move match rate is dead as a metric (0–2/13
noise; matching one human's move was never the target — classical engines tuned to *outcomes*).
- `examples/texel_tune.rs` — Texel tuning: fits eval weights to corpus game outcomes (PGN4 `[Result]`
  points → per-player placement; sigmoid + MSE; queries cached → runs in seconds). The v0 eval scores
  **MSE 0.1146 vs chance 0.14** (real predictive power); the 4 weights are already optimal. **Use the
  outcome-MSE to validate any new eval feature.**
- `examples/gate_ablation.rs` — per-fixture inspector + the **calibration gate** (quiet-move eval
  swing; should stay ~hundreds, not thousands) + a **blunder-rate** (engine moves that lose material).
- `REFERENCE-eval-tuning.md` — the classical-engine tuning philosophy (Texel / Fishtest / SPRT / SPSA)
  and how it maps to 4PC. `experiments/EXP-001..009` — the full diagnostic → fix → tune chain.
- **Deferred:** self-play A-vs-B (true Elo) — the gold-standard comparison, but expensive (a full 4PC
  game is ~150 plies × a search per ply), so it waits for a game loop; Texel MSE is the fast proxy.

## Design rules (fixed)

- Search depth is always a multiple of 4 (one per four-player rotation).
- FEN4 and PGN4 are the native I/O formats — no external translation layer.
- Evaluation returns `[i16; 4]`, never a scalar; the search backs up the whole vector.
- V decomposition is fixed: `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ` (material, positional control, king
  safety, crossfire) — each component traces to exactly one query class.
- Line projection is always recomputed from scratch — no incremental indices, no piece ids.
- New strength-affecting levers ship default-off with an ablation arm.
- The strength gate passes before any NNUE training.
- `eval_value` (centipawns) and `ffa_points` (chess.com FFA scoring) are distinct — never conflate.

## Known spec discrepancies (engine is correct, spec text is not)

- §1.4 lists pawn promotion at the board edge; the real chess.com 4PC rule — and the engine — promote
  at the central crossing (rank 7 / file 7 / rank 6 / file 6).
- §7.3 places the en-passant capturing pawn on the wrong square in all four examples. The engine
  follows the §1.4-derived movement geometry.
