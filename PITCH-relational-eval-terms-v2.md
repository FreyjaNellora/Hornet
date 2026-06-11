# Pitch — Relational Eval Terms for 4PC (v2, revised)

**Author:** Kimi · **Date:** 2026-06-06 · **Status:** Ready for Claude review

**What's new in v2:** Fresh online research (Chessprogramming wiki, Little Chess Evaluation Compendium, engine source analyses) informs revised predicates, conditionality, and magnitudes. New terms: targeted mobility, root pawn. King safety table rescaled for DKW. Unbundled tuning architecture specified. Swarm potential credited as user's prior art (three projects, ant-colony inspired).

---

## The Problem (unchanged from v1)

Per-square / scalar positional is dead: 8 variants all tune to P=0. Stockfish's eval is overwhelmingly relational — that entire class is untested here. It's the only place left for an eval gain short of NNUE.

**Why v1 wasn't enough:** v1 specified the predicates but left magnitudes as "fit, don't pick." Online research reveals that **conditionality** (when a feature is worth more/less) and **non-linearity** (compounding effects) are as important as the base predicate. v1's king safety said "build a table" — v2 specifies the table values, the attack-unit weights, and the DKW-aware scaling.

**The deeper problem:** The eval doesn't aim for the actual win. 4PC is won on FFA points (captures, +20 for eliminating a player, placement), but the eval is points-blind and optimizes material. That's why self-play is drawish and the engine never plays for eliminations. Fixing that is the keystone.

---

## Build Order — Win Term First, Then Positional

### Phase 0: The "Aim for the Win" Term — FIRST

A bounded eval term that values progress toward the win condition: being ahead on / close to scoring FFA points — eliminating an opponent, the +20, placement among the four. Keep it bounded so it doesn't re-create the pathological king-hunt (the reason king-capture is ~0 in search today). Default-off / ablatable.

**Bar:** In a self-play A/B it (a) wins head-to-head and (b) makes games decisive — eliminations actually happen instead of capping out.

**Why first:** This de-drawishes self-play, which is the thing that lets us tune everything else from self-play without a human-game corpus.

---

### Phase 1: Positional Terms — AFTER, and only once self-play is decisive

Each as a separate unbundled readout, **base predicate only** (no conditionality multipliers yet), fit by the N-weight tuner, kept only if it earns a non-zero weight. Add a conditionality multiplier only after the base predicate earns its weight — otherwise you're fitting dozens of un-tuned parameters on 32 games and overfitting.

| Order | Term | Effort | Notes |
|-------|------|--------|-------|
| 1 | Pawn structure (isolated, doubled, connected) | 1 session | Biggest potential gain; simple predicates |
| 2 | Rook open line | ½ session | Simple; our rook data supports it |
| 3 | King safety table | 1 session | Non-linear; addresses DKW |
| 4 | Outpost | ½ session | 3-opponent geometry is the hard part |
| 5 | Swarm potential | 1 session | **User's concept** — ant-colony inspired, carried across three prior projects. 4PC adaptation only. |
| 6 | Targeted mobility | ½ session | Raw mobility failed; this is the retry |
| 7 | Root pawn, defended piece | ¼ session each | Cheap sanity terms |

---

## The Method — 5 Non-Negotiables

1. **UNBUNDLE.** Each term is its own raw readout (`[i16;4]`). One weight per term. Never bundle.
2. **RIGHT SHAPE per term.** Linear for most (count × weight). **Non-linear for king safety** (attack-units → S-curve table).
3. **BASE PREDICATE FIRST.** No conditionality multipliers until the base predicate earns its weight. Fit the base, validate, then add conditions.
4. **FIT, don't pick.** Per-term weights fit by extended `move_tune` (N weights, not 4), confirmed on corrected `texel`. The fitted weight is the answer.
5. **KEEP only if it earns it.** Non-zero weight on move-agreement AND doesn't raise outcome-MSE. Drop dead terms; don't bundle them in.

---

## Terms — Exact 4PC Predicates + Starting Magnitudes

### 1. Pawn Structure — linear → P

**Three separate readouts (tune independently):**

| Readout | Predicate | Starting Magnitude* |
|---------|-----------|---------------------|
| `isolated[p]` | # pawns with no friendly pawn on lane±1 | −20 cp |
| `doubled[p]` | # extra pawns sharing a lane | −10 to −50 cp |
| `connected[p]` | # pawns with friendly on adjacent lane at same or +1 rank | +3 to +5 cp |
| `root[p]` | # pawns defending 2+ friendly pawns | −25 cp |

\*From Little Chess Evaluation Compendium (Toga, Fruit, Rebel). Tuner derives actual weights.

**4PC geometry:** lane = axis ⊥ forward direction. Red/Yellow → file; Blue/Green → rank.

**Defer:** backward (definition ambiguous per wiki), passed (4PC promotion is central-crossing with 3 opponents — genuinely hard).

---

### 2. Rook on Open Line — linear → P

For each friendly rook, score file AND rank:

```
file_openness = 0 if any friendly pawn on file
                1 if only enemy pawns on file (semi-open)
                2 if no pawns on file (open)
rank_openness = same for rank

bonus = sum(file_bonus, rank_bonus)  // try max vs sum, fit decides
```

| Condition | Starting Bonus | Source |
|-----------|---------------|--------|
| Open line | +8 to +20 cp | Chessprogramming wiki |
| Semi-open line | +4 to +10 cp (half) | Chessprogramming wiki |

**Why sum might be correct for 4PC:** Corner rooks (common per visit_freq data) control both a file and a rank. In 2-player chess, rooks rarely sit at file-rank intersections with both open.

**Conditionality (add AFTER base earns weight):**
- Points toward enemy king → ×1.5
- Doubled rooks on same open line → +50%
- No enemy pieces to attack → ×0.5

---

### 3. Outpost (knight/bishop) — linear → P

```
outpost[p] = # of p's minors where:
  (a) defended by friendly pawn
  (b) unattackable by ANY opponent pawn (check all 3 opponents' pawn-attack geometry)
  (c) advanced: past midline toward at least one opponent
```

| Condition | Starting Bonus | Source |
|-----------|---------------|--------|
| Base knight outpost | +10 to +16 cp | Toga log manual |

**Conditionality (add AFTER base earns weight):**
- Defended by two pawns → ×1.5
- No enemy minors on board → ×2.0
- In gate zone → ×2.0
- Bishop outpost (vs knight) → ×0.7

---

### 4. King Safety — non-linear → S

**Two parts:**

#### 4a. King Shelter (linear-ish)
```
shelter[p] = # friendly pawns between king and board center
             (4PC kings start on edges; center is the danger zone)
Missing shelter → penalty per missing pawn
```

#### 4b. King Danger (the table)
```
king_zone = king's squares + 2-3 squares toward board center
attack_units = sum over all enemy pieces attacking king_zone:
  minor (knight/bishop) → 2 units
  rook → 3 units
  queen → 5 units
  safe queen contact check → +6 units

danger = KingDangerTable[attack_units]  // lookup, not linear
```

**Table (more aggressive than 2-player for DKW):**

| Attack Units | Penalty (cp) | 2-Player Equivalent |
|-------------|-------------|---------------------|
| 0 | 0 | 0 |
| 1 | 0 | 0 |
| 2 | 5 | 1 |
| 3 | 10 | 2 |
| 4 | 20 | 3 |
| 5 | 35 | 5 |
| 6 | 55 | 7 |
| 8 | 110 | 12 |
| 10 | 200 | 18 |
| 15 | 500 | 56 |
| 20 | 800 | 100 |
| 30 | 1200 | 200 (capped) |

**Why more aggressive:** DKW = elimination = loss of all agency. 1200cp penalty is reasonable when the alternative is becoming a zombie.

**Scaling by opponent count (add AFTER base earns weight):**
- 3 opponents alive: full table
- 2 opponents alive: ×0.7
- 1 opponent alive: ×0.4

---

### 5. Swarm Potential — non-linear → P

**User's concept** — ant-colony inspired, carried across three prior projects. 4PC adaptation only.

```
swarm[p] = sum over all enemy pieces E:
  attackers = # of p's pieces that reach E
  if attackers >= 2:
    swarm_score += attackers × value(E) × proximity_bonus(E)

proximity_bonus(E) = 1.0 if E's player is next-to-move
                     0.6 if 2 turns away
                     0.3 if 3 turns away
```

**Why non-linear:** Two pieces attacking a queen is much better than one. SEE captures this tactically; swarm captures it positionally — the collective threat configuration.

---

### 6. Targeted Mobility — linear → P

Raw mobility count failed (P=0 in EXP-015). Retry with strategic weighting:

```
For each piece (not pawn, not king):
  reach = squares the piece can move to
  weighted_reach = sum over squares:
    3.0 if square is in enemy king zone
    2.0 if square is in gate zone
    2.0 if square has enemy piece
    1.0 if square is empty and central
    0.5 if square is empty and non-central
```

**Why this might work where raw mobility failed:** Raw mobility counts trapped-square escapes and irrelevant squares. Targeted mobility only counts squares that matter for 4PC strategy.

---

## Tuning Interface

### queries.rs
```rust
pub struct RelationalReadouts {
    pub isolated: [i16; 4],
    pub doubled: [i16; 4],
    pub connected: [i16; 4],
    pub root: [i16; 4],
    pub rook_open: [i16; 4],
    pub outpost: [i16; 4],
    pub shelter: [i16; 4],
    pub danger: [i16; 4],      // already in cp from table lookup
    pub swarm: [i16; 4],
    pub targeted_mobility: [i16; 4],
}
```

### move_tune (extended to N weights)
```rust
// Current: w = [W_MATERIAL, W_POSITIONAL, W_SAFETY, W_CROSSFIRE]
// New: w = [W_MATERIAL, W_ISOLATED, W_DOUBLED, W_CONNECTED, W_ROOT,
//           W_ROOK_OPEN, W_OUTPOST, W_SHELTER, W_DANGER, W_SWARM,
//           W_TARGETED_MOBILITY, W_CROSSFIRE]
//
// P_i = w_isolated × isolated[i] + w_doubled × doubled[i] + ...
// S_i = w_shelter × shelter[i] + danger[i]  (danger is already in cp, no weight)
```

**Hill-climb all weights simultaneously.** The tuner reports which weights are non-zero and which are zeroed.

### Gating Protocol (per term)
1. Implement readout, default-off, **base predicate only**.
2. **move_tune (N-weight):** non-zero weight AND lifts agreement above 18.3%?
3. **corrected texel:** winning combo lowers MSE?
4. **(when corpus ≥ 50 games OR self-play is decisive) self-play A/B:** wins more?
5. Keep if 2 AND 3 pass. Add conditionality only after base passes. Log verdict in EXP file.

---

## What's Different from v1

| Aspect | v1 | v2 |
|--------|-----|-----|
| Build order | Positional terms only | **Win term first**, then positional |
| Magnitudes | "fit, don't pick" | Starting points from Little Compendium + Toga + Rebel |
| Conditionality | Mentioned | **Deferred** — add only after base predicate earns weight |
| King safety table | "build a table" | Table values specified, DKW-rescaled |
| New terms | None | Targeted mobility, root pawn |
| Rook bonus | max(file, rank) | sum(file, rank) — 4PC corner rooks justify this |
| Outpost | Unattackable by one enemy | Unattackable by ALL 3 enemies' pawns |
| King zone | Toward one enemy | Toward board center (3 enemies converge) |
| Scaling | By game phase | By opponent count (3→2→1) — add after base passes |
| Tuning | 4 weights | N weights (one per term) |
| Swarm | Not in v1 | **User's prior art** — ant-colony inspired, three projects |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| N-weight tuning is slower | Start with 3-4 terms, not all 10 |
| Conditionality adds overfitting | **Don't add until base earns weight** |
| King safety table too aggressive | Cap at 1200cp; reduce if self-play becomes too cautious |
| Swarm potential double-counts SEE | Swarm is positional ("I COULD gang up"); SEE is tactical ("I DO gang up"). Different timescales. |
| Data still insufficient | If all terms zero, that's a real result — the eval is NNUE-bound |

---

## Acceptance / Done

- **Phase 0:** Win term implemented, default-off. Self-play A/B shows (a) head-to-head wins and (b) eliminations happen.
- **Phase 1:** Each positional term: separate readout, fitted weight, logged gate verdict. Base predicate only; conditionality added only after base passes.
- Survivors baked into P/S; eval.rs redeployed.
- `move_match` + corrected `texel` re-validated.
- lib tests green.
- **Target:** move-agreement above 18.3% with at least one positional weight > 0.

---

## Sources (Fresh Research)

1. Chessprogramming wiki — King Safety: https://www.chessprogramming.org/King_Safety
2. Chessprogramming wiki — Outposts: https://www.chessprogramming.org/Outposts
3. Chessprogramming wiki — Rook on Open File: https://www.chessprogramming.org/Rook_on_Open_File
4. Little Chess Evaluation Compendium: http://www.winboardengines.de/doc/LittleChessEvaluationCompendium.pdf
5. Little Chess Evaluation Compendium Addendum: https://www.chessprogramming.org/images/4/49/Addendum6LCEC_2012.pdf
6. Giraffe paper (Stockfish eval breakdown): https://arxiv.org/pdf/1509.01549
7. KTH thesis — Agent pawn structure: https://kth.diva-portal.org/smash/get/diva2:642238/FULLTEXT01.pdf
8. Luminex engine v5.4.0 (trapped knight, far piece penalties)
9. Carballo engine changelog (rook trapped, bishop trapped, pawn storm)
