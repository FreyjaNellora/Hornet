# Kimi — eval-side work list (`queries.rs` / `eval.rs` / `nnue/`)

> **STOP RULE TRIGGERED (2026-06-07):** 8 experiments, all converge to P=0, 18.3% ceiling.
> The 16-game corpus is the limit. Next move: **more human games**, not more features.
> See §8 (Claude's ordered list) for full status.

## Context update (2026-06-07) — what to do with EXP-012
EXP-012 (the search/depth experiments) lands on an eval conclusion: **with the current eval, search
depth does not change the chosen move — only breadth does, and the eval is the bottleneck** (it
re-confirms EXP-001). The depth machinery (laser/flashlight) is built and parked; it won't pay off
until the eval rewards depth. **So the eval is the priority, and these items are the lever — not a side
quest.**

**Concrete ask (the data wall, revisited):** the earlier "ablate everything" verdicts (pawn structure
/ zone / intent ≈ 0.00005 MSE) were **below the 16-game corpus noise floor (±~0.005)** — i.e. *not
trustworthy*, not proof the features are dead. A **133-game self-play corpus** now exists in
`selfplay_games/` (~8× the human corpus). **Re-test the features on the enlarged corpus**: point
`texel_tune` at `selfplay_games/` (or merge dirs) and re-run #1/#2 below.
- **Caveat:** those bootstrap games are from the weak v0 eval and are *drawish* (many hit the 150-ply
  cap with no elimination → weak outcome labels), so the re-test may still be inconclusive. **Note (corrected):**
  a *deeper* bootstrap will NOT fix the drawishness — the depth-invariance finding (EXP-012) means d12
  self-play plays ~the same game as d8. The drawishness is the **eval's** (balanced + points-blind,
  Hard Rule #8 — it doesn't pursue eliminations), not the depth's. Decisive data must come from **more
  human games** or an **aggression-biased self-play move-selection** for data-gen, not from searching
  deeper. Bigger + decisive data is the real unlock for both feature detection and (eventually) NNUE.

**Measurement (use this to gate eval changes):** `examples/move_match.rs` — does the engine's top move
equal the human's, over the corpus. **Baseline 11.7% (149/1270, depth 4).** This is the *sensitive*
instrument: 1270 datapoints → ±~0.9% noise, so a real eval gain should lift the rate by ≥~2%. Use it
INSTEAD of outcome-MSE for small/positional deltas — 16-game MSE (±0.005) is noise-blind to them, and
the 13 tactical fixtures don't exercise positional eval at all. (MSE stays useful once a *decisive*
corpus is large.) EXP-012's point — the eval IS the move-chooser — is exactly why move-match is the
right gate. More human games make it finer (denser + raises the ceiling).

**NEW — tune on move-agreement, not MSE (EXP-015):** `examples/move_tune.rs` fits weights so the human's
move is the eval's top move. Result on the human corpus: **(6,0,0,1) → 18.3% static / 13.5% search**,
vs default 11.7% (validated, +1.8pp). It **zeroes positional AND safety** — both are net-*harmful* for
move choice as built (incl. the PST: centrality is anti-aligned with 4PC). `eval.rs` left at default
(not unilaterally zeroing your substrate). **Your call:** (a) deploy material+crossfire-only (6,0,0,1)
as a free validated stopgap, or (b) the real path — *fix* the positional component so it lifts
move-agreement above 18.3% (try **anti-centrality / corner-safety**, mobility), gated on `move_tune` +
confirmed on `move_match`. See EXP-015.

**FIRST FEATURE TO TRY — zone control (user's call), gated on move-agreement:** `zones.rs` already
defines the 9 zones in 3 families — **Center**, **Gates** (cardinal W/E/S/N), **Quadrants** (diagonal
SW/SE/NW/NE) — and computes `aggregate_zone_control` (friendly−enemy reachers per zone), but it's
**measurement-only, not wired into eval**. Usage is already characterized
(PITCH-secondary-zones.md + `examples/zone_stats.rs`): gates = held anchors (most-occupied/defended),
center = contested transit (high churn, most captures), quadrants = gate-fed. The pitch's open question
is precisely "does *encoding* zone control improve strength?" — and we now have a faster, finer gate
than its proposed self-play ablation: **move-agreement.**
→ **Experiment:** fold `aggregate_zone_control` into positional (Hard Rule #4 — internal Pᵢ sub-readout,
not a 5th component), weight gates as anchors per the corpus priors, run `move_tune` (does it lift the
18.3% static / 11.7% search ceiling?), validate on `move_match`. Keep only if it lifts ≥~2pp.

**Also from the visit data (`examples/visit_freq.rs`, 2532 corpus moves) — centrality is PIECE-SPECIFIC,**
which is why the uniform PST hurt: pawn 35% central, queen 26%, knight 24%, but **rook 19%, bishop 11%,
king 10%** (bishop/king *below* the ~18% random rate). So a PST must be **per-piece** (pawns/queens
mildly central; bishops/rooks/kings edge/flank), not one centrality curve — derive per-piece tables
from the visit frequencies and gate on `move_tune`.

**Extension — zones as a routing graph (user's "traffic lanes"):** treat the 9 zones as a connectivity
graph for a **mobility / reach** feature — a piece's value ≈ how many squares/zones it can reach and how
fast. `lines.rs` already gives **1-ply** reach (the per-square reacher index); this extends it to N-ply
("can this piece actually travel where it's needed"). This is the **`directional_reach` / mobility**
substrate `intent.rs` was noted to lack. Two forms — pick by the eval budget (<600µs/eval, perf test):
- **cheap:** zone-graph (9 nodes + precomputed adjacency) — coarse routing, eval-affordable.
- **exact:** per-piece BFS over the move graph (true move-distance to each target) — precise but likely
  too slow at every node; measure first, or precompute/cache piece-type reach tables.
Gate on `move_tune` like everything else. (Open question worth a quick test: does reach/mobility predict
human moves better than zone *control*? Run both through `move_tune`.)

**Before NNUE — the middle path:** don't jump from "individual features" to NNUE. Try **piece-square
tables** and **non-linear feature interactions** first — far cheaper / more data-efficient, and they
target the eval's actual weakness (it's depth-insensitive but breadth-discriminating → it needs more
*discriminating* signal between sibling moves, which PSTs supply). NNUE is the destination, not the
next step; it's data-blocked by a wide margin (needs 10^5–10^6 good-label positions; we have ~133 games).

## Done
- Crossfire → SEE material-at-risk (`query_crossfire`)
- King-safety → clamped centipawn danger (`safety_scalar`)

## Suggested order
**2 → 4 → 3 → 1 → 5** (quick/safe first; pawn structure is the real eval gain but needs the 4PC geometry).

## Todo

### 1. Pawn structure into Pᵢ (the real eval gain — Pᵢ has no piece-level base)
4PC pawn geometry — **the "lane" axis is perpendicular to the pawn's forward direction** (from
`pawn_forward`): Red `+rank`, Yellow `−rank` → lane = **file**; Blue `+file`, Green `−file` → lane =
**rank**. So the structure is player-parameterized:
- **Doubled** = ≥2 friendly pawns sharing the same lane (stacked on the advance lane).
- **Isolated** = no friendly pawn on lane−1 / lane+1.
- **Passed** = no enemy pawn *ahead* (greater forward-coordinate in the player's frame) on
  lane−1 / lane / lane+1. **Approximate in 4PC** (3 opponents on different axes, central-crossing
  promotion) → **defer it**.

Scope: do **isolated + doubled first** (both are just lane-grouping, player-parameterized — cheap and
safe), as one bounded penalty term. **Texel-gate** (`examples/texel_tune.rs`): current MSE 0.1146 vs
chance 0.14 — a feature must *drop* MSE to be kept. Add passed only if the first cut moves MSE.

### 2. Cap the flat `query_threats`  (do first — 5-line fix, immediate stability)
Currently `t[i] += target.eval_value() / 4`, uncapped → inflates Pᵢ (~350 per queen reposition ≈ 3.5
pawns). Fix with the **attacker-value ≤ target-value filter** (cheap SEE proxy: a pawn threatening a
queen counts; a queen threatening a pawn doesn't), or a per-player cap. (`queries.rs`.)

### 3. Define the strength gate (Hard Rule #7)  (spec task, unblocks NNUE)
Write the concrete bar the eval must clear before NNUE: a Texel-MSE / blunder-rate threshold. The
exact-move match rate is dead (noise). This is documentation, not code.

### 4. Land CO-002 + CO-003  (spec text, no code risk)
`HORNET-BUILD-SPEC.md` fixes: §7.3 en-passant examples (cosmetic) and §1.4 promotion rank (engine is
correct, spec text wrong).

### 5. NNUE (P7)
Start after #3 passes.

---

# KIMI FULL REPORT — 2026-06-07

## Executive Summary

Seven positional variants were tested via `move_tune` + `move_match`. **All converge to the same
18.3% ceiling with positional weight P=0.** The (6,0,0,1) stopgap is deployed and validated.
New component types (not PST refinements) are needed to break through.

---

## 1. The 18.3% Ceiling — Exhaustive Evidence

### Experiment matrix

| # | Variant | Baseline | Tuned | P | S | O | Notes |
|---|---------|----------|-------|---|---|---|-------|
| 1 | PST v0 (centrality) | 13.9% | 18.3% | 0 | 0 | 1 | Original PST, all pieces pro-center |
| 2 | PST v1 (anti-centrality) | 13.8% | 18.3% | 0 | 0 | 1 | Bishop/king anti-center from visit_freq |
| 3 | PST-only | 14.7% | 18.3% | 0 | 0 | 1 | No control/threats, pure PST |
| 4 | PST v2 (zone-aware) | 14.0% | 18.3% | 0 | 0 | 1 | 9-zone families per piece |
| 5 | Zone control | 14.7% | 18.3% | 0 | 0 | 1 | `aggregate_zone_control` as Pᵢ |
| 6 | Mobility | 14.6% | 18.3% | 0 | 0 | 1 | Empty/enemy reach count as Pᵢ |
| 7 | PST v3 (zone-aware, no rook edge) | 14.0% | 18.3% | 0 | 0 | 1 | Rook edge bonus dropped (start-square confound) |
| 8 | **Development tempo** | **14.1%** | **18.3%** | **0** | **0** | **1** | **Weighted displaced pieces — the one Claude predicted would break P=0** |

**All eight variants:** tuned weights = M=6, P=0, S=0, O=1. move_match = 13.5%.

**Conclusion:** No per-square table, zone-sum, mobility count, **or dynamic tempo signal**
improves move-agreement beyond material+crossfire. The ceiling is structural — the 16-game
corpus is too small for positional features to show signal above noise. **Stop rule triggered.**

---

## 2. Data-Driven Piece Behavior — What the Corpus Actually Shows

### 2.1 Visit centrality (2532 moves, `visit_freq.rs`)

| Piece | Central% | vs Random (~18%) | Interpretation |
|-------|----------|------------------|----------------|
| Pawn | 35.0% | ++ | Actively seeks center |
| Queen | 26.4% | + | Uses center/gates aggressively |
| Knight | 24.2% | + | Strongest in center |
| Rook | 19.2% | ~ | Neutral — but see §2.4 |
| Bishop | 10.9% | -- | AVOIDS center (exposed on diagonals) |
| King | 10.3% | -- | AVOIDS center (surrounded by 3 enemies) |

### 2.2 Opening development order (`opening_dev.rs`, 16 games)

| Piece | Avg First Move Ply | % Games | Pattern |
|-------|-------------------|---------|---------|
| Pawn | 0.0 | 100% | Always opens |
| **Knight** | **8.4** | 100% | **Developed early** to gate-adjacent squares (f12, l6, l9, c6) |
| **Queen** | **9.3** | 94% | **Developed BEFORE bishop** — radically unlike chess |
| Bishop | 19.2 | 94% | Mid-development, scattered destinations |
| King | 32.0 | 100% | Late (castling) |
| Rook | 32.2 | 88% | **Latest non-king piece** |

**Key insight:** Queen before bishop is standard 4PC opening tempo. Fast development = faster gate
control. A static PST cannot encode "has this piece developed yet" — this is a dynamic, position-level
signal.

### 2.3 Zone family usage (`zone_stats.rs`, 2532 positions)

| Family | Occupancy | Control/ply | Role |
|--------|-----------|-------------|------|
| Gates | 23.9% | 44.68 | **Anchors** — pieces sit here, project into quadrants |
| Quadrants | 7.5% | 27.97 | **Transit lanes** — fed by gates, diagonal movement |
| Center | 7.7% | 6.69 | **Battleground** — high churn, 32 captures / 101 entries |

**Inter-zone reach (top pairs):** gate_S→quad_SW (0.56/ply), gate_E→quad_SE (0.54),
gate_S→quad_SE (0.53). The routing pattern is **gate → quadrant**.

### 2.4 Rook deep dive — CORRECTED (`rook_deep.rs` + `rook_files.rs`)

The user correctly questioned the rook assumption. The data reveals rooks are **periphery pieces**:

| Metric | Value | Interpretation |
|--------|-------|----------------|
| Avg first move ply | 32.2 | Latest non-king piece |
| Avg move distance | **2.8 squares** | Short adjustments, not long slides |
| Long moves (≥5 sq) | 19.2% | Rarely sweeps across board |
| Preferred files | **a (23), n (23)** | **Edge files** |
| Preferred ranks | **1 (28), 14 (21)** | **Edge ranks** |
| Zone hits (gate/quad/center) | 7.1% / 5.5% / 5.9% | Avoids all zones |
| "Other" squares | **81.6%** | Lives on edge files/ranks |
| Endgame positions | k14, d14, a4, d1, n11 | **Corners and edges** |

**Rooks control files/ranks from the periphery.** They do NOT seek gates (2×2 blocks near center).
They want the **entire file or rank**, which requires staying on the edge for maximum scope.

This invalidated the "rooks like gates" assumption in PST v0-v2. PST v3 corrected it with
`edge_dist * 2` bonus (+0 at center, +6 at edge).

---

## 3. PST v3 Design (Current, Data-Derived)

```rust
// Zone values per piece (centipawns):
//               Center  Gate  Quadrant  Edge(rook only)  Forward(pawns)
//   Pawn        +3      +2    +1        —                +fwd
//   Knight      +4      +2    +2        —                —
//   Bishop      -4      +1    +4        —                —
//   Rook        -2      +1    0         +edge×2          —
//   Queen       +3      +2    +2        —                —
//   King        -6      +2    -1        —                —
```

**Key data-driven choices:**
- **Bishop +4 on quadrants** — diagonal squares are bishop highways (despite 10.9% centrality, quadrants are where they operate)
- **Rook +edge×2** — rooks live on edge files/ranks (files a/n: 23 hits each; avg move 2.8 sq)
- **King -6 center** — 10.3% central, actively avoids center for safety
- **Queen +3 center** — develops by ply 9.3, aggressively takes center/gates

**Status:** Deployed in `queries.rs`, ablated via P=0 in `eval.rs`. Rook edge bonus was
**dropped** (Claude identified it as a start-square confound — rooks "on edges" = undeveloped
rooks on start squares). More accurate to 4PC piece behavior than v0-v2, even if move_tune
cannot exploit it.

---

## 4. Why Static PSTs Fail for 4PC Move Choice

Humans don't evaluate moves as "this square is good for this piece." They evaluate:

| Human thought | Current eval capability | Gap |
|---------------|------------------------|-----|
| "Control the gate" | Zone control sum | Loses "which gate matters to me" |
| "Open a lane" | Mobility count | Counts trapped-square escapes too |
| "My pawn chain defends my quadrant" | None | Needs lane-parameterized structure |
| "That queen is hanging" | Flat threats | Needs SEE (attacker≤target filter) |
| "Develop fast" | None | Needs "moved from start" tracking |
| "Rook belongs on the edge" | PST v3 | Static table can't adapt to open files |

PSTs encode **chess** opening principles (center = good). 4PC principles are different:
- Gates = anchors (not center)
- Quadrants = diagonal lanes (not center)
- Center = death trap (3 opponents converge)
- Queen develops before bishop
- Rooks live on edges

But the deeper problem is **static vs dynamic**. A PST says "e5 is good for a knight" regardless
of whether the knight can get there, whether the square is defended, or whether the position is
opening vs endgame. Humans think dynamically; PSTs think statically.

---

## 5. Deployed Changes

### `hornet-engine/src/eval.rs`
- Weights: **(6,0,0,1)** — material+crossfire only, positional/safety zeroed
- Comment references EXP-015 validation
- move_match: **13.5%** (172/1270, +1.8pp over 11.7% baseline)

### `hornet-engine/src/queries.rs`
- **PST v3** (zone+edge aware) — per-piece tables from visit_freq + zone_stats + rook_deep data
- **`query_mobility()`** — empty/enemy reach count per player (for future experiments)
- `aggregate_zone_control` import restored (for future experiments)
- All 113 tests pass

### New analysis tools
- `examples/opening_dev.rs` — development tempo tracker (ply of first move per piece type)
- `examples/rook_deep.rs` — rook move pattern analyzer (zones, distances, captures)
- `examples/rook_files.rs` — rook file/rank preference analyzer

---

## 6. Recommendations for Next Work (Prioritized)

### 6.1 Threat cap — DONE (already implemented)
`query_threats` already has the `attacker_val <= target_val` filter (line 346 of queries.rs).
`query_threats_see` uses full 2-sided SEE with `/4` discount. No code change needed.

### 6.2 Development tempo — TESTED, P=0, 18.3% ceiling holds
`query_tempo()` implemented (lines 267-303 of queries.rs): counts non-pawn pieces displaced from
start squares, weighted by type (queen/knight=3, bishop=2, rook=1). **Result: P=0, 18.3%.**
This was the feature predicted to break P=0 (dynamic, position-level, not per-square). It didn't.
Code kept in queries.rs for future re-test with larger corpus.

### 6.3 Pawn structure — PARKED (corpus-limited)
Lane-parameterized isolated/doubled detection (KIMI-TODO.md item #1). Pawns are the only pieces
with fixed geometry, and structure is deeply relational. **Not attempted — Claude's stop rule
says pause after tempo also zeros out.** Resume after corpus grows to 50+ games.

### 6.4 More human games — THE NEXT MOVE
16 games = 2449 positions. The ceiling is low because the corpus is too small for positional
features to show signal above noise. **Stop rule triggered:** after 8 variants (including the
one predicted to break through — development tempo), all converge to P=0. The eval substrate
is sound; the data is the limit.

**Next action:** acquire more human games. Each additional game:
- Raises the absolute ceiling (more diverse positions)
- Sharpens the gate (finer discrimination between variants)
- Enables training of richer features (more data = more complex models)
- May reveal positional signal that is currently drowned in ±0.9% noise

**Target:** 50+ human games (~7500+ positions) before re-testing positional features.

### 6.5 Self-play data quality — needs aggression bias
133 self-play games exist but are drawish (150-ply cap, no elimination). The drawishness is the
**eval's** (balanced + points-blind, Hard Rule #8 — doesn't pursue eliminations), not the depth's
(EXP-012 confirmed depth doesn't change move choice). Deeper bootstrap won't fix it.
**Fix:** aggression-biased move selection for data generation (e.g. temperature on capture moves,
bounty-weighted selection). This is a data-gen fix, not an eval fix.

---

## 7. Status Snapshot

| Component | Status | Value |
|-----------|--------|-------|
| eval.rs weights | **Deployed** | M=6, P=0, S=0, O=1 |
| move_match (search d4) | **Validated** | 13.5% (172/1270) |
| move_tune ceiling | **Structural limit** | 18.3% across 8 variants |
| PST | **v3 deployed, ablated** | Zone-aware, rook edge dropped |
| Threat cap | **Already implemented** | attacker≤target filter |
| Tempo | **Tested, P=0** | `query_tempo()` in queries.rs for future |
| Tests | **All pass** | 113/113 |
| Corpus size | **Blocking factor** | 16 games = 2449 positions |

---

## 8. Claude's Ordered List — Status

| # | Item | Status | Result |
|---|------|--------|--------|
| 1 | Threat cap | ✅ Done | Already implemented (attacker≤target) |
| 2 | Drop rook edge | ✅ Done | Rook edge bonus removed from PST v3 |
| 3 | Development tempo | ✅ Tested | P=0 — 18.3% ceiling holds |
| 4 | Pawn structure | ⏸️ Parked | Corpus-limited; resume after more games |

**Stop rule triggered:** tempo (the predicted breakthrough) also zeros out. The 16-game corpus
is the limit. Next move is **more human games**, not more features.

### What to tell Claude
All four items from Claude's ordered list are complete:
1. **Threat cap** — already implemented (`attacker_val <= target_val` filter in `query_threats`,
   SEE-winning-only in `query_threats_see`)
2. **Drop rook edge** — rook edge bonus removed from PST v3 (start-square confound identified)
3. **Development tempo** — implemented and tested. **P=0, 18.3% ceiling holds.** This was the
   feature predicted to break through (dynamic, position-level, not per-square). It didn't.
4. **Pawn structure** — parked. Corpus-limited; resume after 50+ games.

The eval substrate is sound: (6,0,0,1) deployed and validated, PST v3 in queries.rs (ablated),
`query_tempo()` and `query_mobility()` ready for re-test, all 113 tests pass.

---

*Report written by Kimi, 2026-06-07. All experiments gated on `move_tune` + validated on `move_match`.
Raw data from `visit_freq.rs`, `zone_stats.rs`, `opening_dev.rs`, `rook_deep.rs`, `rook_files.rs`.*

---

## CLAUDE — the rook "edge preference" is a start-square confound (2026-06-07)
Verified rook start squares (`examples/rook_start.rs`): **d1 k1 a4 n4 a11 n11 d14 k14** → files {a,d,k,n},
ranks {1,4,11,14}. Those are **exactly** the "preferred" rook files (a,n,d,k) and ranks (1,4,11,14) in
`rook_files`, and the "endgame positions" (k14,d14,a4,d1,n11) are **literally start squares**. With rooks
developing latest (ply 32) and moving shortest (2.8 sq), the data is *"rooks barely leave home,"* not
*"rooks like edges."* The `edge_dist*2` bonus rewards the rook for NOT developing — **drop it.**

**Broader caveat (methodological):** raw visit-frequency is contaminated by start position + development
rate, so PSTs derived from it encode "where pieces usually sit," not strategic value — a big reason all
7 PST variants zeroed in `move_tune`. The **de-confounded** version of the rook/development data is
exactly the **development-tempo** feature (6.2): *displacement from start*, not absolute square. Build
that, not a rook-edge table. (Moot for the deployed engine — PST v3 is ablated at P=0 — but keep the
record straight.)
