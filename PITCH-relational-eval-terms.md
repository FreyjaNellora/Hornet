# PITCH — relational eval terms for 4PC (unbundled, fit, gated)

Build the **relational** eval features (the class Stockfish's eval is mostly made of) the right way, so
they don't die the way per-square features did. This is the concrete spec: which terms, their exact 4PC
predicates, the construction *shape* per term, the tuning interface, and the gating bar.

**Reading order:** SOURCES.md (refs + quotes) → HOW-EVAL-TERMS-ARE-MADE.md (the research) →
STOCKFISH-EVAL-MAP.md (the menu) → RELATIONAL-TERMS.md (the plan) → this (execution).

## Why this, why now
- Per-square / scalar positional is **dead**: 8 variants (centrality, anti-centrality, per-piece,
  zone-aware, edge-aware, zone-control, mobility, dev-tempo) all tune to **P=0** on both gates, across
  16→32 games (EXP-015). Reweighting the existing bundle is exhausted.
- Stockfish's eval is overwhelmingly **relational** — pawn structure, outposts, rook-on-open-line,
  king-safety (SOURCES.md). That whole class is **untested** here. It's the only place left for an
  eval gain short of NNUE (which is data-blocked by a wide margin).

## The method — 5 non-negotiables (this is *why* prior attempts zeroed)
1. **UNBUNDLE.** Each term is its **own raw readout** (`[i16;4]`) — never summed into one positional
   number. Bundling good + dead terms under one weight `P` lets the tuner only zero the *whole bundle*;
   that's why a good pawn-structure term would have died with the dead PST. One weight per term.
2. **RIGHT SHAPE per term.** Most terms are **linear** (count × weight). King-safety is **non-linear**
   (attack-units → S-curve table) because *multiple attackers compound* — "the whole is greater than
   the sum of the parts" (King Safety, SOURCES.md). A single linear weight cannot express that; build
   the table.
3. **FIT, don't pick.** No hand-chosen magnitudes. Per-term weights fit by the extended `move_tune`
   (move-agreement), confirmed on corrected `texel` (outcome-MSE). The fitted weight *is* the answer to
   "how much."
4. **KEEP only if it earns it.** A term survives only if it gets a non-zero weight on move-agreement
   **and** doesn't raise outcome-MSE. A term that fits to ~0 is dead — drop it (don't bundle it in).
5. **BAKE survivors into `P`.** Once fitted, fold the kept terms (with their relative weights) into the
   positional component `P` — Hard Rule #4 intact (P stays one V-component; its *recipe* is the tuned
   relational mix).

## Terms to build (in order) — exact 4PC predicates
Geometry: **lane = axis ⊥ the player's forward direction** — Red/Yellow → file, Blue/Green → rank.
"forward" = toward the board per `pawn_forward`. "Enemy" = each of the **3 opponents** (their pawn
geometry differs by their own forward direction). All readouts default-off / ablatable; emit each as a
separate `[i16;4]` (per-player).

### 1. Pawn structure — *the swing* · linear · → P
Three **separate** readouts (so each tunes independently):
- **isolated[p]** = # of player p's pawns with **no friendly pawn on lane−1 or lane+1** (any rank). Penalty.
- **doubled[p]** = # of *extra* friendly pawns sharing a lane (2 on a lane → 1; 3 → 2). Penalty.
- **connected[p]** = # of player p's pawns with a friendly pawn on an adjacent lane at the **same
  forward-rank (phalanx)** or **one rank behind (supported)**. Bonus.
Defer **backward** (the wiki itself calls the definition "ambiguous", SOURCES.md) and **passed**
(4PC promotion is central-crossing with 3 opponents — genuinely hard). Wiki magnitude refs exist but
**fit the weights, don't hardcode**.

### 2. Rook on open line · linear · → P
For each friendly rook, look at its **file and its rank**:
- **open** = a line (file or rank) with **no pawns of any color** on it → full bonus units.
- **semi-open** = a line with **only enemy pawns** → half (per Rook on Open File, SOURCES.md).
- A rook controls both axes → score the better of {file, rank}, or sum (try both, fit).
- **rook_open[p]** = summed open/semi units over p's rooks. This is the *correct* rook idea — rooks
  want open lines, not the rim (the "rook-edge" PST was a start-square confound).

### 3. Outpost (knight/bishop) · linear · → P
A minor on a square that is **(a)** defended by a friendly pawn, **(b)** un-attackable by *any*
opponent pawn — no opponent pawn can ever advance to a square attacking it (check all 3 opponents'
pawn-attack geometry), **(c)** advanced (past the midline toward an opponent). Per Outposts, SOURCES.md.
- **outpost[p]** = count of outposted minors (consider knight > bishop weight). v2 conditionality:
  worth more if **two** pawns defend it / if opponents lack a minor to trade it off.

### 4. King safety — *non-linear* · → S
Two parts:
- **king_shelter[p]** (linear-ish): # friendly pawns in the king's shelter (squares between the king and
  its home edge / facing the board). Missing-shelter → penalty.
- **king_danger[p]** (the table): define a **king zone** (king's squares + 2–3 facing the board);
  accumulate **attack units** over the 3 opponents' pieces attacking the zone (minor 2, rook 3, queen 5;
  + units for safe checks), then **index a non-linear S-curve table** → danger in cp (King Safety,
  SOURCES.md). **Build the table, not a linear weight.**
- ⚠ **Damped by Hard Rule #8** (eval is points-blind; king-capture ≈ 0). Pair this term with a decision
  on whether to relax that — making the king matter would also de-drawish self-play. Flag, don't assume.

### 5. Defended piece · linear · → P · cheap sanity term
- **defended[p]** = # of p's non-pawn pieces defended by a friendly pawn (harder to dislodge) → small
  bonus. Cheap, relational, good control.

## Tuning interface (how the terms get weighed)
- **queries.rs:** add a `query_relational(lines, board) -> RelationalReadouts` returning each term above
  as its **own** `[i16;4]`. Do **not** fold into `positional` yet.
- **move_tune (extended):** must fit one weight **per readout** (N weights, not the current 4) on
  move-agreement, reading the `RelationalReadouts` struct, and report which earn a non-zero weight. The
  `RelationalReadouts` field names are the interface between the readouts and the N-weight tuner.
- **texel (corrected):** confirm the fitted combo lowers outcome-MSE (the seat-order label bug is fixed,
  so this gate is now valid; use `HORNET_HUMAN_ONLY=1`).
- **Bake:** fold survivors into `P` with their fitted relative weights; redeploy `eval.rs`.

## Gating protocol (per term)
1. Implement the readout (separate `[i16;4]`).
2. **move_tune (N-weight):** non-zero weight + lifts agreement above the current ~19.2% ceiling?
3. **corrected texel (human-only):** the winning combo lowers MSE beyond noise?
4. **(when corpus ≥ ~50 games) self-play A/B:** does it actually win more? — the real bar.
5. Keep if 2 **and** 3 pass (ideally 4). Log the verdict (kept/dropped + weights) in an EXP file.

## Pitfalls — from the research, don't relearn them
- **correlation ≠ causation** → require *two* gates, not one (Texel pitfall, SOURCES.md).
- **independence / data** → a "this term is dead" verdict needs game **count** (~50+), not just
  positions; per-term needs more signal than the bundled test did.
- **representational gaps** → reweighting can't invent a concept; that's the whole reason we add new
  *features*, not new weights.
- **non-linearity** → king-safety as a table, not a weight.
- **tactics belong in search** → don't build static fork/sac detectors; qsearch + SEE handle those (a
  *bounded* static threat term is fine — we already have `query_threats` + `query_crossfire`).

## Acceptance / done
- Each term: a separate readout, a fitted weight, a logged gate verdict.
- Survivors baked into `P`; `eval.rs` redeployed; `move_match` + corrected `texel` re-validated; lib
  tests green.
- **Target:** move-agreement above the current ~19.2% ceiling **with P>0** — the first real positional
  signal in the eval. If every relational term also fits to ~0, that's a hard, real result: the eval is
  NNUE-bound, and the next move is corpus scale, not features.
