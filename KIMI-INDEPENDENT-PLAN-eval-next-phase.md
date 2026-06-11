# Kimi — Independent Plan: Eval Next Phase

**Date:** 2026-06-06 · **Context:** Claude's independent ask in `COLD-PITCH-kimi-objective-layer.md`  
**Constraint:** Write without anchoring to Claude's fixes (objective-handling change, safety rebuild, adaptive-beam). Compare plans side-by-side after.

---

## Part 1: Math Breakdown of Current Eval (as it stands)

### The formula

```
Vᵢ = 6·ΔMᵢ + 0·ΔPᵢ + 0·ΔSᵢ − 1·ΔOᵢ
```

where `ΔXᵢ = Xᵢ − X̄` (deviation from per-component mean across 4 players).

### What each component actually computes

| Component | What it measures | Scale at start | Scale midgame |
|-----------|------------------|----------------|---------------|
| **M** (material) | Sum of `eval_value()` per piece | 4,200 cp/player | 2,500–3,500 cp |
| **P** (positional) | `control + threats + PST` | ~0 (symmetric) | ±50–200 cp |
| **S** (safety) | `defenders·40 + escapes·25 − attack_danger` | ~+200 cp/player | ±100–400 cp |
| **O** (crossfire) | SEE-resolved material-at-risk per player | ~0 | 0–500 cp |

### The mean-relative normalization

```rust
let mean_m = sum(material) / 4;
let dm = material[i] - mean_m;  // ~ ±500 cp midgame
```

**Why this is clever:** `Σᵢ Vᵢ ≈ 0` exactly (off by at most 3 from integer rounding). This gives Sturtevant–Korf shallow pruning a tight `SUM_UB = 0` bound instead of −5348.

**Why this is dangerous:** A component with small absolute scale gets amplified by the mean-relative step. If all players have similar safety (~200), `ΔSᵢ` is tiny. But if one player's king is under heavy attack (S=−400) while others are safe (S=+200), the delta is −600 — multiplied by `W_SAFETY`, it can swing the eval.

### The real problem: what the eval optimizes

The current eval is **material + crossfire only** (P=0, S=0). It values:
1. Having more material than the average player
2. Having less material-at-risk than the average player

It does NOT value:
- Making progress toward eliminations
- Pawn structure quality
- Piece activity / coordination
- King safety (the S component exists but is zeroed)
- Positional advantages beyond PST noise

**The eval is a local tactical snapshot, not a strategic objective function.** This is why self-play is drawish: both sides trade material symmetrically, no one pushes for the win.

### Risk surface

1. **Scale mismatch:** Crossfire (O) is bounded by piece values (max ~900 per piece), but material (M) is 4,200. The ratio M:O = 6:1 means crossfire matters only when material is equal — which it usually is in balanced positions.
2. **PST is noise:** The per-square tables (zone-aware v3) add ~±10 cp per piece. With 16 pieces, that's ±160 cp total — but mean-relative reduces it to ~±40 cp. At W_P=0, it's ignored entirely.
3. **Safety huddle trap:** The S component rewards defenders around the king (`defenders·40`). In 4PC, this correlates with undeveloped pieces. The tuner correctly negated it (W_S=0), but the underlying predicate is flawed.
4. **No win-condition signal:** The eval doesn't know that 4PC is won by FFA points. It treats a position where you're about to eliminate a player the same as a position where everyone is equal.

---

## Part 2: Research — How Strong Engines Build Relational Terms

### Sources consulted

1. **Chessprogramming wiki** (King Safety, Outposts, Pawn Structure, Rook on Open File)
2. **Little Chess Evaluation Compendium** (Toga/Fruit/Rebel pawn structure, Stockfish king safety)
3. **Glaurung 2.x source** (king safety table, attack-unit weights)
4. **KTH thesis** "Agent: A pawn structure evaluation function"
5. **Giraffe paper** (deep learning for chess, feature design lessons)
6. **Luminex/Carballo changelogs** (4PC engine eval evolution)

### Key findings

#### A. Pawn structure (the biggest untapped gain)

Strong engines (Stockfish, Komodo, Houdini) score pawn structure at **20–50 cp per defect**:
- Isolated pawn: −20 to −30 cp
- Doubled pawn: −10 to −50 cp (worse if isolated too)
- Backward pawn: −10 to −20 cp
- Passed pawn: +20 to +800 cp (non-linear by rank)

**The insight:** Pawn structure is **relational** — it depends on the arrangement of multiple pawns, not any single square. This is why per-square PSTs can't capture it.

**4PC adaptation:** Lane geometry is player-parameterized (file for R/Y, rank for B/G). The predicate is identical; only the axis changes.

#### B. King safety (the huddle trap)

Stockfish's king safety is **attack-weighted, not defense-weighted**:
- Count attack units (piece-type-weighted attackers near king)
- Map attack units → danger score via a **non-linear table** (S-curve)
- Defenders only **mitigate** danger; they don't add standalone bonus

**The trap our eval fell into:** `defenders·40` is a standalone bonus. It rewards parking pieces near the king. Stockfish's approach: danger = f(attack_units / defenders), where defenders are in the denominator, not a separate additive term.

**Glaurung's table (simplified):**
```
attack_units:  0   2   4   6   8  10  12  14  16  18  20+
danger_score:  0  20  60 120 200 300 400 500 600 700 800
```
The curve is **super-linear** — light attack is cheap, heavy attack is devastating.

#### C. Rook on open file

Simple, powerful, well-validated:
- Rook on open file (no friendly pawns): +20 cp
- Rook on semi-open file (no enemy pawns): +10 cp
- Rook on 7th rank: +30 cp

**4PC adaptation:** File for R/Y, rank for B/G. The "7th rank" equivalent is the promotion-adjacent rank/file.

#### D. Outposts

A knight outpost (defended by pawn, can't be attacked by enemy pawns) is worth +20–30 cp. The conditionality matters: only valuable if it's on a useful square (central, near enemy territory).

**4PC challenge:** 3 opponents mean 3 enemy pawn structures. An outpost against Red might be attackable by Blue's pawns. The predicate needs to check ALL enemy pawn directions.

#### E. Mobility (the failed attempt)

Raw mobility (count of reachable squares) failed in our tuning. Research shows why: **mobility is only valuable when it leads to something** — threats, control of key squares, coordination.

**Targeted mobility:** Count mobility squares that are (a) in enemy territory, (b) attack enemy pieces, or (c) support friendly pieces. This is what strong engines actually measure (Stockfish's "mobility bonus" is piece-specific and territory-aware).

---

## Part 3: My Independent Plan — Build Order + Gating

### Core principle: The win term must come first

The user's work order is clear: **win term FIRST, then positional terms.** I agree completely, for a reason that goes beyond the user's stated rationale:

**Without a win term, self-play is a random walk with material conservation.** Two engines with identical evals will trade symmetrically, never push for eliminations. The 133-game corpus hitting the 150-ply cap is not a data problem — it's an **objective problem.** No amount of positional terms will fix it if the engine doesn't know what it's playing for.

**The win term is the keystone that makes everything else tunable.**

### Phase 0: "Aim for the Win" Term (1 session)

**What it is:** A bounded eval term that values progress toward the actual win condition.

**How to build it (my independent take, not Claude's):**

The win condition in 4PC is: eliminate opponents, collect +20 per elimination, place 1st/2nd/3rd. The eval is points-blind (Hard Rule #8). But the eval CAN measure **proximity to elimination** without knowing points:

```
win_term[i] = f(weak_opponents) − f(weak_self)
```

Where `f(weak)` measures how close a player is to being eliminated:
- Low material = weak
- King under heavy attack = weak  
- Few legal moves = weak
- Already eliminated = −∞ (but search handles this)

**My specific predicate (independent of Claude's approach):**

```rust
/// Proximity to elimination: how "close to death" is each player?
/// Returns [0..100] where 100 = very weak (low material + attacked king).
fn elimination_proximity(board: &Board, lines: &LineMap, ks: &[KingSafety; 4]) -> [i16; 4] {
    let mut prox = [0i16; 4];
    for player in Player::ALL {
        let i = player.index();
        let mat = material[i];  // from query_material
        // Material weakness: below 2000 cp = vulnerable
        let mat_weakness = (2500 - mat).max(0).min(1500) / 15;  // 0..100
        // King danger: from king_danger_scalar (already computed)
        let danger = king_danger_scalar(&ks[i]).min(600) / 6;  // 0..100
        // Combine: a player is weak if BOTH low material AND attacked
        prox[i] = (mat_weakness * danger / 100).min(100);
    }
    prox
}
```

**The win term for player i:**
```
win[i] = Σ_{j≠i} prox[j] - 3 * prox[i]
```

This says: "I want my opponents to be weak, and I want to not be weak." The factor of 3 on `prox[i]` encodes that self-preservation matters more than aggression (you can't win if you're eliminated).

**Why bounded (0..100 per player, ~±300 total):** So it doesn't swamp material. A player who's slightly ahead on material but much safer should still be favored.

**Gating:** Self-play A/B. Config A: win_term on (W=1). Config B: win_term off (W=0). Measure:
1. Does A win more points than B? (strength)
2. Does A produce more eliminations? (decisiveness)
3. Does A reach the ply cap less often? (the key metric)

**Bar:** ≥10% reduction in ply-cap games, OR ≥5% more eliminations, OR positive Elo equivalent in points.

### Phase 1: Unbundle the Eval (1 session)

Before adding positional terms, the eval architecture needs to support N independent weights. Currently:

```rust
fn compute_utility(qv: &QueryVector) -> [i16; 4] {
    // M, P, S, O only — 4 bundled components
}
```

**My architectural change (independent of Claude's N-weight tuner):**

```rust
/// Each term is a separate [i16; 4] readout with its own weight.
pub struct EvalTerms {
    pub material: [i16; 4],       // W=6 (fixed, the anchor)
    pub crossfire: [i16; 4],      // W=1 (validated)
    pub win: [i16; 4],            // W=0..? (Phase 0)
    pub pawn_iso: [i16; 4],       // W=0 (Phase 1)
    pub pawn_doubled: [i16; 4],   // W=0 (Phase 1)
    pub rook_open: [i16; 4],      // W=0 (Phase 1)
    pub king_danger: [i16; 4],    // W=0 (Phase 1)
    // ... more terms
}

/// Compute utility: Σ weight_k · Δterm_k for each player.
/// All terms are mean-relative (zero-sum).
fn compute_utility(terms: &EvalTerms, weights: &[i16]) -> [i16; 4] {
    // ...
}
```

**Why this matters:** Each term can be independently zeroed, tuned, and ablated. The move_tune.rs example needs extension from 4 weights to N weights — hill-climb over all integer weights simultaneously.

### Phase 2: Positional Terms (after win term gates)

**Order and rationale:**

| Order | Term | Why this order | Effort |
|-------|------|----------------|--------|
| 2.1 | **Pawn structure** (isolated, doubled) | Biggest known gain from 2PC engines; simplest predicate | ½ session |
| 2.2 | **Rook open line** | Simple; our rook data supports it; complements pawn structure | ½ session |
| 2.3 | **King danger table** (not safety) | Non-linear; addresses the huddle trap; needs careful tuning | 1 session |
| 2.4 | **Outpost** (knights) | 4PC-specific geometry challenge; moderate gain | ½ session |
| 2.5 | **Swarm potential** | User's concept; ant-colony inspired; 4PC-specific | 1 session |
| 2.6 | **Targeted mobility** | Raw mobility failed; this is the retry | ½ session |
| 2.7 | **Root pawn, defended piece** | Cheap sanity terms; small gain | ¼ session each |

**For each term:**
1. Build the predicate (base only, no conditionality)
2. Add to `EvalTerms` as a separate readout
3. Run `move_tune` — does it earn a non-zero weight?
4. Run `selfplay_ab` — does it win head-to-head vs. the version without it?
5. **Keep only if both gates pass.**

### Phase 3: Conditionality (after base predicates earn weight)

Once a base predicate has a validated non-zero weight, add conditionality multipliers:

- **Pawn structure:** connected pawns bonus, passed pawn bonus (by rank)
- **Rook open line:** rook on 7th rank bonus, doubled rooks on open file
- **King danger:** DKW-aware scaling (danger from the player who moves next = worse)
- **Outpost:** outpost on central square vs. edge = more valuable

**Rule:** No conditionality until the base predicate earns ≥5 cp weight in tuning.

---

## Part 4: Tuning Strategy (Independent of Claude's Approach)

### The three tuning instruments

| Instrument | What it measures | Sample size | Noise |
|------------|------------------|-------------|-------|
| `move_tune` | Human move agreement | 1,270 positions | ±0.9% |
| `selfplay_ab` | Head-to-head strength | 100+ games | ±5% points |
| `texel_tune` | Outcome prediction (MSE) | 16 games | ±0.005 MSE |

**My priority order (different from Claude's):**

1. **Primary gate: `selfplay_ab` with win term on.** Once the win term makes games decisive, self-play becomes the real instrument. A term that improves move-agreement but loses self-play is harmful.
2. **Secondary gate: `move_tune` on human corpus.** For terms that don't affect self-play decisiveness (e.g., pawn structure in quiet positions), human move agreement is the fine-grained check.
3. **Tertiary gate: `texel_tune` on decisive corpus only.** Once we have games with actual eliminations, outcome prediction becomes meaningful. Before that, it's noise.

### The N-weight tuner

Extend `move_tune.rs` from 4 weights to N:

```rust
/// Score a position under N weights.
fn score_n(c: &[[f64; 4]], w: &[f64], term_idx: usize) -> f64 {
    w[term_idx] * (c[term_idx][0] - mean(c[term_idx]))
}

/// Hill-climb all integer weights simultaneously.
fn hill_climb(data: &[PosMoves], w: &mut [f64]) {
    let mut cur = match_rate_n(data, w);
    loop {
        let mut improved = false;
        for idx in 0..w.len() {
            for delta in [-1.0, 1.0] {
                let mut cand = w.to_vec();
                cand[idx] = (cand[idx] + delta).max(0.0);
                let r = match_rate_n(data, &cand);
                if r > cur + 1e-9 {
                    *w = cand;
                    cur = r;
                    improved = true;
                }
            }
        }
        if !improved { break; }
    }
}
```

**Constraint:** Material weight (W_MATERIAL) stays fixed at 6. It's the anchor. All other weights start at 0 and hill-climb upward.

---

## Part 5: Specific Term Designs (My Independent Take)

### 5.1 Pawn Structure

```rust
/// Three separate readouts (tune independently):
pub fn query_pawn_isolated(board: &Board) -> [i16; 4] {
    // Count pawns with no friendly pawn on lane±1
    // Penalty: −1 per isolated pawn (weight will scale it)
}

pub fn query_pawn_doubled(board: &Board) -> [i16; 4] {
    // Count extra pawns beyond the first per lane
    // Penalty: −1 per extra pawn (weight will scale it)
}

pub fn query_pawn_connected(board: &Board) -> [i16; 4] {
    // Count pawns with friendly pawn on adjacent lane at same or +1 rank
    // Bonus: +1 per connected pawn
}
```

**Why separate readouts:** The tuner might find that isolated is harmful (−20 cp) but connected is helpful (+5 cp). Bundled, they might cancel out and get zeroed.

### 5.2 Rook Open Line

```rust
pub fn query_rook_open(board: &Board, lines: &LineMap) -> [i16; 4] {
    // For each friendly rook:
    //   +1 if on open line (no friendly pawns on that file/rank)
    //   +0.5 if on semi-open (no enemy pawns — actually this is worse in 4PC)
    // In 4PC, semi-open means "one enemy has no pawns there" — but 2 others might.
    // Simpler: just "no friendly pawns" = open line.
}
```

### 5.3 King Danger Table (the safety rebuild)

```rust
/// Attack units: weighted count of enemy pieces attacking king vicinity.
/// Knight/bishop = 2, rook = 3, queen = 5, pawn = 1, king = 0.
fn attack_units(ks: &KingSafety) -> i16 {
    // Use ks.attackers + ks.attack_value to derive units
    // attack_value / 150 ≈ attack units (pawn=100, knight=300, etc.)
    (ks.attack_value / 150).min(20)
}

/// Danger table: non-linear mapping from attack units to danger score.
/// S-curve: light attack is cheap, heavy attack is devastating.
static DANGER_TABLE: [i16; 21] = [
    0,  5, 15, 30, 50, 75, 105, 140, 180, 225,
    275, 330, 390, 455, 525, 600, 680, 765, 855, 950, 1000
];

pub fn query_king_danger(ks: &[KingSafety; 4]) -> [i16; 4] {
    // For each player: DANGER_TABLE[attack_units(&ks[i])]
    // This is PURE danger — no defender bonus, no escape bonus.
    // Defenders only reduce attack_units (via ks.attack_value, which is
    // already computed with defender awareness in classify_reachers).
}
```

**Key difference from current safety_scalar:** No standalone `defenders·40` or `escapes·25`. Danger is purely about incoming attack, mitigated only by the attack_value calculation (which already considers defenders).

### 5.4 Outpost

```rust
pub fn query_outpost(board: &Board, lines: &LineMap) -> [i16; 4] {
    // For each friendly knight:
    //   Is it defended by a friendly pawn?
    //   Can ANY enemy pawn attack it (check all 3 opponents' pawn directions)?
    //   Is it in enemy territory (past the mid-rank/file)?
    //   +1 if all three conditions met
}
```

**4PC-specific:** Enemy pawns move in 3 different directions. A square safe from Red's pawns might be attackable by Blue's. The predicate must check all three.

### 5.5 Swarm Potential (User's Concept)

```rust
/// Ant-colony inspired: reward coordinated piece clusters near enemy kings.
/// For each enemy king, count friendly pieces within radius 2.
/// More pieces = stronger swarm = higher elimination potential.
pub fn query_swarm(board: &Board, lines: &LineMap) -> [i16; 4] {
    // For each player i:
    //   For each enemy player j:
    //     Count i's pieces within Chebyshev distance 2 of j's king
    //     Weight by piece value (queen=5, rook=3, etc.)
    //   Sum over all enemies
}
```

**Credit:** This is the user's concept, carried across three prior projects. The 4PC adaptation is mine.

### 5.6 Targeted Mobility

```rust
pub fn query_targeted_mobility(lines: &LineMap, board: &Board) -> [i16; 4] {
    // For each piece, count mobility squares that are:
    //   (a) in enemy territory (past mid-board), OR
    //   (b) attack an enemy piece, OR
    //   (c) are attacked by a friendly piece (coordination)
    // Weight by piece type (knight mobility = more valuable than pawn mobility)
}
```

---

## Part 6: Gating Criteria (Hard Rules)

For ANY term to be merged:

1. **Default-off lever:** The term must have an `on/off` toggle (env var or config flag).
2. **Ablation arm:** There must be a way to run A/B tests with the term on vs. off.
3. **Self-play gate:** The term must show measurable improvement in `selfplay_ab` (points or decisiveness).
4. **Move-agreement gate:** OR the term must lift `move_tune` rate by ≥1pp on the human corpus.
5. **Performance gate:** `eval_4vec` must stay <600 µs in debug mode.
6. **Zero-sum gate:** `Σᵢ Vᵢ` must stay within ±10 of zero.

**The hierarchy:** Self-play > move-agreement > texel. A term that passes self-play but fails move-agreement is still a keep (it improves strength even if humans don't play that way). A term that passes move-agreement but fails self-play is a drop (it overfits to human style).

---

## Part 7: What I Would Do Differently from Claude (Speculative)

Since Claude hasn't shared their plan, I can only speculate on differences based on the codebase:

1. **Win term design:** Claude's `win_weight` adds mean-relative FFA points directly to the search value. My approach adds a **derived elimination-proximity** term to the eval (not search), keeping the eval points-blind but win-aware. The difference: Claude's is search-side (affects tree value), mine is eval-side (affects leaf scoring). Both are valid; mine keeps the eval-evaluator boundary cleaner.

2. **Safety rebuild:** Claude's `danger_weight` subtracts `king_danger_scalar` from the search value. My approach builds a **danger table** (non-linear, attack-unit-based) as a separate eval term. The difference: Claude's is linear subtraction; mine is a shaped lookup. The table gives finer control over the danger curve.

3. **Tuning priority:** Claude seems to prioritize `move_tune` (human agreement). I prioritize `selfplay_ab` (strength) once the win term is on. The difference: I believe the engine should play strong chess first, then match human style; Claude may believe human style is the better proxy.

4. **Architecture:** Claude has already built `win_weight` and `danger_weight` into `Searcher`. I would keep them in the eval (`eval_4vec`) as separate QueryVector components, making them tunable by the same N-weight machinery as all other terms. The difference is architectural — search-side vs. eval-side objective layer.

---

## Summary: My Recommended Next Steps

1. **Build the win term** (Phase 0) as an `elimination_proximity` predicate in `queries.rs`, wire it into `eval.rs` as a new component `W` (or fold into `positional` temporarily).
2. **Extend `move_tune.rs`** to N weights (hill-climb over all terms).
3. **Run `selfplay_ab`** with win-on vs. win-off. Gate: ≥10% fewer ply-cap games.
4. **Once gated, build pawn structure** (isolated + doubled) as separate readouts.
5. **Gate each term** on self-play + move-agreement. Drop anything that doesn't earn its weight.
6. **Only after base predicates validate, add conditionality** (connected pawns, 7th rank, DKW scaling).

The win term is the keystone. Everything else is decoration until the engine knows what it's playing for.
