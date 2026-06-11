# Hornet — fresh-session seed (2026-06-10)

Portable context for starting a new chat on the Hornet engine. Self-contained; read top to
bottom. Pointers to the canonical docs are at the end. **First action in the new session: run
`cargo test` to confirm the recorded green state before touching anything.**

---

## What Hornet is

A zero-dependency Rust engine for **four-player chess (chess.com FFA — free-for-all, four armies,
last-standing / most-points wins)**. Not two-player chess: four players move in rotation (R→B→Y→G),
players get eliminated, and scoring is **points-based and non-zero-sum**. The board is a 14×14 cross
(160 valid squares; the four 3×3 corners are removed). Native I/O is **FEN4** and **PGN4** (chess.com's
four-player formats) — no external translation layer.

## Architecture (one primitive drives everything)

Per-piece BFS **line projection** (every piece's reach, X-ray past the first blocker, plus a
per-square inverse index) → a **query engine** that emits a per-player utility vector
**V = ⟨U₁,U₂,U₃,U₄⟩** → a **Max^n** search that backs V up, each node maximizing the *moving* player's
own component. **The evaluation is a vector `[i16;4]`, never a scalar.** A dense-MLP NNUE over the
query outputs is the eventual replacement for the hand-tuned evaluator — gated behind a strength gate,
not started.

V decomposition is fixed: `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`
(material, positional control, king safety, crossfire) — each component traces to exactly one query
class. Deployed weights: **`(6,0,0,1)`** i.e. `W_MATERIAL=6, W_POSITIONAL=0, W_SAFETY=0, W_CROSSFIRE=1`.

## Current state (2026-06-10)

- **Pipeline complete, P0–P6:** board → move-gen → line projection → query engine → eval → Max^n
  runs end-to-end. **Suite fully green: 112 lib + 3 integration** against the **32-game** human
  reference corpus (replay floors recalibrated to ≥5000 plies / ≥15 games full; observed 5058/7477,
  15/32).
- **Protocol wired (UCI-like):** `position startpos | fen4 <fen> | pgn4 <path> [moves …]` + `go [depth]`
  → `bestmove <from-to>`. Output round-trips back through `position … moves`, so it's drivable for
  external self-play.
- **Self-play runs full games** (`examples/selfplay.rs`, ~3.7 min/game at depth 12).
- **Dead-King-Walking fully implemented** (EXP-011): LIVE / DKW (checkmated king walks randomly,
  earns no points, its non-king pieces are **frozen walls — immovable and un-capturable**) / DEAD (all
  pieces removed). Search treats a DKW node as **expectimax** (king is random) and does **not** sweep on
  king-capture during search (that over-values king-hunts; the sweep is game-flow only, in `game.rs`).
- **perft from start = 20 / 395 / 7800 / 152050** (regression-tested; `perft(2)=395` is correct — the
  gap vs 400 is a discovered pin).
- **Eval recalibrated:** an old crossfire `value×count` scale bug swung the eval by thousands per move
  and drowned material; fixed (crossfire → SEE material-at-risk, safety → clamped centipawn danger,
  `ffa_points` lifted out of the eval so it's points-blind again). Texel tuning confirms the **4 weights
  are already optimal → remaining eval gains are in the *features*, not the weights.**

## Open work (technical backlog, by priority)

### 1. Move-ordering defect — `count_defenders` inverted + ships default-on  *(outcome-affecting)*
`move_order::count_defenders` (move_order.rs:79) counts a "defender" when `p.player != victim_player`
— backwards: a recapturer is the victim's *own* side. So the `FREE_CAPTURE_BONUS` fires on **defended**
pieces and misses genuinely free ones. It's also adjacency-only and carries dead `match`/`pawn_deltas`
scaffolding. Both `FREE_CAPTURE_BONUS` and `FFA_BOUNTY_MOVE_ORDER` are hardcoded `const = true`
(move_order.rs:17,20) — against the default-off discipline.

**Why it's not cosmetic:** in a **beam** search, ordering *is* selection (the top-k ordered moves are
the only ones expanded), so a backwards ordering heuristic changes the move actually played at narrow
beams.

**Fix = a measured 3-arm flip, not a silent edit.** The two flags are independent and only the
free-capture path carries the bug (`count_defenders` is called solely inside the `FREE_CAPTURE_BONUS`
block, move_order.rs:160-164; `FFA_BOUNTY_MOVE_ORDER` has no identified defect). Measure, fixed seeds,
maxn at beam 30 and beam 4: (i) both on — the de-facto baseline every recorded number used; (ii)
free-capture off / bounty on — isolates the bug, this delta is the contamination estimate; (iii) both
off — the landing state and new baseline. Run `move_match` + a short seeded self-play A/B per arm.
**Land with both flags `false`.** Then fix-or-delete `count_defenders`, decided by a *measured* cost:
the correct cheap candidate is an `is_attacked_by(board, m.to, victim.player)` attack scan — **not**
LineMap (ordering runs at interior nodes where no LineMap exists; the inverse index would mean
projecting lines per node, the cost the original shortcut avoided). If the scan cost is unacceptable,
delete the function and the bonus path. Any reintroduced free-capture bonus is a default-off ablation
arm. Remove the dead scaffolding either way.

**Corpus contamination note (so nobody re-proves it):** the `selfplay_games/` bootstrap corpus was
generated on the **maxn path at beam 4** with this bug live → **regenerate before tuning on it.**
Flashlight-path experiments are **clean** — `search_flashlight` (search.rs:293-397) never calls
`move_order`; only `root_move_values` (412) and `search_depth` (432) do (verified 2026-06-10).

### 2. Gate the zero-weight queries  *(pure perf)*
`W_POSITIONAL` and `W_SAFETY` are `0`, yet `run_all_queries` computes positional control, threats, PST,
and the full king-safety scan every leaf and multiplies by zero. Skip what the weights zero out (keep
the king-safety path available to the search-side danger term, which reads it independently). Acceptance
= eval-output equality on a position sweep + a perf number. Independent of item 1; lands any time.

### 3. Protocol `go` config drift
`protocol/mod.rs` still ships the maxn path with a **deprecated** 2M node budget (cut mid-rotation,
unsound per EXP-012). Align `go` with the current search-shape recommendation (flashlight + a generous
per-rotation cap), and say which in the doc.

### 4. Spec drift (drafted as CO-004 / CO-005, awaiting approval — spec edits are gated)
`HORNET-BUILD-SPEC.md` §4.5 still specifies the pre-EXP-005 `value×count` crossfire; the appendix
weights say `(1,2,1,1)` vs deployed `(6,0,0,1)`; §2.5's Board struct (piece lists / cached king squares
/ packed castling byte) was never built that way. Code is correct; the spec text is stale. Also still
open from before: CO-002 (§7.3 EP examples, cosmetic) and CO-003 (§1.4 promotion rank — real; 4PC
promotes at the **central crossing**, not the edge; engine already correct).

### 5. The eval frontier — where real strength comes from next
The 4 weights are optimal, so gains are in the **features**. `Pᵢ` (positional) is the only V component
without a piece-level base (Mᵢ/Sᵢ/Oᵢ all got tactical bases in the recalibration; Pᵢ is flat
centrality-mobility), which is why Texel finds it signal-thin. **Build pawn-structure first**
(isolated / doubled / passed — new, bounded; classically predictive, caveat: 4PC central-promotion
geometry may dampen it — the Texel delta judges). Mobility re-skinning won't move Texel; threats (Oᵢ)
are already folded in and SEE-threat tested null (EXP-002). `intent.rs` is a *threat* substrate (can
supplement Oᵢ, **not** Pᵢ-mobility). Every feature ships **default-off as an ablation arm**, accepted
only on a `texel_tune` MSE drop.

## Fixed design rules (do not violate)

- Evaluation returns `[i16;4]`, never a scalar; the search backs up the whole vector.
- Search depth is always a multiple of 4 (one ply per four-player rotation).
- FEN4 / PGN4 are the native I/O formats — no external translation layer.
- Line projection is always recomputed from scratch — no incremental indices, no piece ids.
- `eval_value` (centipawns) and `ffa_points` (FFA scoring) are distinct — **never conflate** them
  (Hard Rule #8: the evaluator is points-blind; the FFA-hunt preference lives in move ordering).
- **Any strength-affecting lever — eval feature, move ordering, beam width/shape, LMR, killers/history,
  TT-hint usage — ships default-off with a measured (self-play) ablation arm.** (This is the rule the
  `count_defenders` flags violated.)
- Tuning is **outcome-based** (`texel_tune` MSE vs corpus `[Result]`), not exact-move-match (dead as a
  metric: 0–2/13, noise). Self-play A-vs-B (true Elo) is the gold standard but expensive (a full game is
  ~150 plies × a search per ply), so it's the deferred check; Texel-MSE is the fast proxy. More decisive
  self-play game outcomes are the main thing that would sharpen the gate.
- The strength gate passes before any NNUE training (Phase 7, not started).

## Build & run

```
cd hornet-engine
cargo test     # 112 lib + 3 integration, all green as of 2026-06-10
cargo run      # UCI-like REPL on stdin/stdout (position / go / bestmove)
```
Repo: `github.com/FreyjaNellora/Hornet`.

## Where to read more

- `HORNET-BUILD-SPEC.md` — full architecture/reference (§9 module tree; note the §4.5/appendix/§2.5
  drift in backlog item 4).
- `STATUS.md` — live production board (current state, blockers, watch items).
- `ENGINE-HANDOFF.md` — longer engine catch-up (this seed is its compression).
- `experiments/` — the diagnostic → fix → tune chain (EXP-001…009 eval; EXP-011 DKW; EXP-012 search
  shape; EXP-015 move-agreement; EXP-016 depth pathology; EXP-017/018 win-term & points-aware safety).
- `REFERENCE-eval-tuning.md` — the classical Texel/Fishtest/SPRT tuning method and how it maps to 4PC.
