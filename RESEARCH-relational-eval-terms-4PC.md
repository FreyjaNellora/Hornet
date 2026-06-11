# Research: Relational Eval Terms for 4PC — My Own Approach

**Date:** 2026-06-06 · **Sources:** Chessprogramming wiki, Little Chess Evaluation Compendium, academic papers, engine source analyses (all fetched fresh, not from existing project files)

---

## 1. How Relational Terms Are Actually Built (What I Learned)

### 1.1 The Universal Pattern

Every relational eval term follows the same 4-layer pipeline:

```
Chess concept → Computable predicate → Feature value → Weighted score
```

**Layer 1: Chess concept** — "an isolated pawn is weak"
**Layer 2: Predicate** — "this pawn has no friendly pawns on adjacent files"
**Layer 3: Feature value** — count of isolated pawns (0, 1, 2, ...)
**Layer 4: Weight** — feature × centipawns (fitted, never guessed)

The art is Layer 2 (a crisp, cheap predicate). The science is Layer 4 (fitting to data).

### 1.2 Key Insight: Conditionality Matters

From the Little Chess Evaluation Compendium and Stockfish source analysis:

> "Outpost value can be increased if it is defended by two pawns (thus making exchange sacrifice much less profitable) or if opponent has no minors that he can exchange for an outpost piece."

This means: **the base predicate is not enough.** The *conditions around it* change the score. A knight on d5 is not just "outpost = +10cp" — it's "outpost + defended-by-two-pawns + no-enemy-minor-to-trade = +16cp."

This is why single-weight-per-feature tuning fails: the same feature has different value in different contexts.

### 1.3 Non-Linearity Is Required for Compounding Effects

From the Chessprogramming wiki King Safety page (direct quote):

> "Stockfish counts each minor piece attack on a king zone as 2 attack units, rook attack as 3, queen as 5. The typical curve is S-shaped: it raises slowly at first, then goes up faster, becoming almost flat at the end."

The Glaurung 1.2 safety table (quoted directly):
```
0,0,0,1,1,2,3,4,5,6,8,10,13,16,20,25,30,36,42,48,55,62,70,80,90,100...
```

**Key finding:** 1 attacker = 0 penalty. 2 attackers = small penalty. 3 attackers = bigger. 5+ = catastrophic. A single linear weight **cannot express this.** You need either:
- A lookup table (attack units → centipawns)
- A quadratic/exponential formula
- Piece-count scaling (more attackers = each additional one matters more)

This directly explains why our bundled P=0 tuning failed: king safety, threats, and crossfire all have compounding effects that a single linear weight cannot capture.

---

## 2. Pawn Structure — The Foundation

### 2.1 What I Found Online

**Definitions (from Chessprogramming wiki + Little Compendium):**

| Term | Predicate | Typical Magnitude |
|------|-----------|-------------------|
| **Isolated** | No friendly pawn on adjacent files | −20 cp |
| **Doubled** | ≥2 friendly pawns on same file | −10 to −50 cp per extra pawn |
| **Connected/phalanx** | Friendly pawn on adjacent file at same/adjacent rank | +3 to +5 cp per pawn |
| **Passed** | No enemy pawn ahead on same/adjacent files | +25 cp, +50 if protected |
| **Backward** | Stop square lacks pawn protection, controlled by enemy sentry | −20 to −35 cp |
| **Root pawn** | One pawn defends two others (e.g., b3 with a4,c4) | −25 cp |

**Critical conditionality from the Little Compendium:**
- Doubled pawns part of a group of 3+ → **no penalty** (they support each other)
- Doubled pawns that are **fixed** (blocked by enemy pawns) → **−75 cp** (worse than isolated!)
- Two connected passers on 6th rank → **better than a rook**

### 2.2 My 4PC Adaptation

**The lane problem:** In 4PC, pawns move in 4 different directions. A "file" for Red/Yellow is a "rank" for Blue/Green. So the predicate must be **player-relative:**

```
For player P:
  lane = axis perpendicular to P's forward direction
  lane-1, lane+1 = adjacent lanes
  "ahead" = greater forward-coordinate in P's frame
```

**My proposed predicates:**

| Term | 4PC Predicate | Why This Works |
|------|---------------|----------------|
| **isolated[P]** | # pawns with no friendly pawn on lane±1 (any rank) | Same as 2-player, just lane-relative |
| **doubled[P]** | # extra pawns sharing a lane (2→1, 3→2, etc.) | Same as 2-player, lane-relative |
| **connected[P]** | # pawns with friendly pawn on adjacent lane at same or +1 forward-rank | Phalanx + supported |
| **passed[P]** | # pawns with no enemy pawn ahead on lane±1 or lane | Deferred — 4PC promotion is central-crossing with 3 opponents, genuinely hard |
| **root[P]** | # pawns that defend 2+ friendly pawns | From Little Compendium, −25 cp |

**Conditionality I want to add (not in existing project docs):**
- Connected pawns on the **gate lanes** (lanes leading to gate zones) → bonus ×1.5 (gates are anchors in 4PC)
- Isolated pawns in the **center** (ranks 5-8, files 5-8) → penalty ×2 (center is more dangerous with 3 enemies)
- Doubled pawns where the **front pawn is advanced** (≥4 forward steps) → less penalty (space advantage)

---

## 3. Rook on Open Line — The Correct Rook Idea

### 3.1 What I Found Online

From Chessprogramming wiki (direct quote):

> "An open file is usually defined as a file with no pawns on it — a semi-open file as containing only the enemy pawns. Bonuses applied to a rook on an open file vary from 8 to 20 centipawns. Typical bonus for a semi-open file is half of that for a fully open file."

From the Little Compendium:
> "Rooks will get some bonus for each open file on the board, occupied or not by own or enemy rooks. +3 cps for each file. Existing semi-open files might get 15 cps each."

**Key nuance:** Some engines (Rebel) give extra bonus for **doubled rooks** on an open file. Some increase bonus if the file has a bearing against the enemy king.

### 3.2 My 4PC Adaptation

**The 4PC insight:** Rooks control **both files AND ranks** (they're sliders). In 4PC, a rook on the edge can control an entire rank or file. So:

```
For each friendly rook:
  file_openness = 0 if any pawn on file, 1 if only enemy pawns, 2 if no pawns
  rank_openness = same for rank
  bonus = max(file_bonus, rank_bonus) OR sum (try both, fit)
```

**Why max vs sum:** A rook on a corner (e.g., a1) controls file a AND rank 1. If both are open, does it get double bonus? In 2-player chess, rooks rarely control both simultaneously (they're on one square). But in 4PC, corner rooks are common (our data shows rooks live on edges). So **sum might be correct for 4PC** where rooks actually sit at file-rank intersections.

**Conditionality:**
- Rook on open line that points toward **enemy king** → bonus ×1.5 (king is capturable in 4PC)
- Rook on open line with **no enemy pieces to attack** → bonus ×0.5 (open line with no target is less valuable)
- **Doubled rooks** on same open line → additional +50% (coordination bonus)

---

## 4. Outposts — Knight/Bishop Strong Squares

### 4.1 What I Found Online

From Chessprogramming wiki (direct quote):

> "a square on a half-open file on the opponent's half of the board, defended by an own pawn, and either no longer attackable by opponent pawns at all."

From the Outposts page (fetched):
> "Toga log user manual advocates a bonus for a knight outpost on a central square as 10 centipawns, but it is possible to see bonuses as large as 16 centipawns."

**Key conditionality:**
- Defended by **two pawns** → bonus increases (exchange sac is less profitable)
- Opponent has **no minor to trade** → bonus increases (knight can't be kicked)

### 4.2 My 4PC Adaptation

**The 3-opponent problem:** In 4PC, an outpost must be unattackable by **all 3 opponents' pawns**, not just one. This is harder to achieve but more valuable when it happens.

```
outpost[P] = # of P's minors (knight/bishop) where:
  (a) defended by friendly pawn
  (b) unattackable by ANY opponent pawn (check all 3 opponents' pawn-attack geometry)
  (c) advanced: past the midline toward at least one opponent
```

**4PC-specific conditionality:**
- Outpost in a **gate zone** → bonus ×2 (gates are anchors, controlling a gate is huge)
- Outpost defended by **two pawns** → bonus ×1.5
- Outpost with **no enemy minors on the board** → bonus ×2 (can't be traded off)
- **Bishop outpost** vs **knight outpost** → knight gets larger bonus (knight is better on outpost — can't be blocked by pawns)

---

## 5. King Safety — The Non-Linear Table

### 5.1 What I Found Online

From Chessprogramming wiki (direct quote):

> "King zone is usually defined as squares to which enemy King can move plus two or three additional squares facing enemy position."

Attack unit accumulation (Stockfish):
- Minor on king zone: 2 units
- Rook on king zone: 3 units  
- Queen on king zone: 5 units
- Safe queen contact check: +6 units

The S-curve table (rescaled to centipawns):
```
attack_units:  0  1  2  3  4  5  6  7  8  9  10  15  20  25  30  40  50
penalty(cp):   0  0  1  2  3  5  7  9 12 15  18  35  56  80 100 140 200
```

**Key insight:** The table caps around 500cp (index 50+). This means even catastrophic king danger is bounded — it won't outweigh a queen. But in practice, king danger + material threats = resignation.

### 5.2 My 4PC Adaptation

**The DKW problem:** In 4PC, king safety is not just "avoid checkmate" — it's "avoid DKW" (Dead-King-Walking). DKW means your pieces freeze, you become a target, and you lose all agency. So king danger in 4PC is **more severe** than in 2-player chess.

**My proposed table (more aggressive than 2-player):**
```
attack_units:  0  1  2  3  4  5  6  7  8  9  10  15  20  25  30  40  50
penalty(cp):   0  0  5 10 20 35 55 80 110 150 200 350 500 650 800 1000 1200
```

**Why more aggressive:** DKW is effectively elimination. Losing 1200cp of positional value is reasonable when the alternative is becoming a zombie.

**4PC-specific king zone:**
- King zone = king's squares + 2-3 squares toward the **board center** (not toward one enemy — all 3 enemies converge from different directions)
- **Pawn shelter:** friendly pawns between king and the board center (not "in front" — 4PC kings start on edges, center is the danger zone)
- **Pawn storm:** enemy pawns advancing toward the king's shelter

**Scaling by opponent count:**
- With 3 opponents alive: king safety matters most (full table)
- With 2 opponents alive: king safety matters less (table ×0.7)
- With 1 opponent alive: king safety matters least (table ×0.4)

This is because with fewer opponents, there are fewer directions of attack.

---

## 6. Piece Coordination — The Missing Class

### 6.1 What I Found Online

This was the least-documented area. Most sources mention it in passing:

From the Little Compendium:
> "4 own pieces placed at the 4 ends of a square shape of board squares, 4 squares each side, would deserve +20 cps bonus, as no matter what the particular pieces at the 4 ends are, they seem to communicate in an uncannily coordinated way."

From engine source analyses:
- **Trapped pieces:** knight with 0 mobility = −10 cp (Luminex engine)
- **Far piece penalties:** pieces far from own king = penalty (encourages better piece coordination)
- **Bishop pair bonus:** +50 cp (well-documented)
- **Bad bishop:** trapped behind own pawns = penalty

### 6.2 My 4PC Adaptation — "Swarm Potential" (Prior Art)

**This concept is not original to this document.** It has been developed across three prior projects, inspired by ant colony behavior — ants raiding nests and finding food through collective action. The mapping to 4PC FFA: multiple pieces attacking the same target from different directions, analogous to ants swarming a food source.

In 4PC FFA, coordination means something different: **multiple pieces attacking the same target from different directions.** This is how you eliminate a player.

```
swarm_potential[P] = sum over all enemy pieces E:
  # of P's pieces that attack E × value(E) × proximity_bonus

Where proximity_bonus = 1.0 if E is next-to-move, 0.6 if 2-away, 0.3 if 3-away
```

**Why this matters:** Two pieces attacking a queen is much better than one. The SEE already captures this tactically, but swarm_potential captures it **positionally** — the collective threat configuration, not the immediate tactical execution.

**Related: Piece mobility quality**
Not just "how many squares can I reach" but "how many squares can I reach that matter":
- Square in enemy king zone → counts ×3
- Square in gate zone → counts ×2
- Square with enemy piece → counts ×2
- Square with no strategic value → counts ×0.5

This is different from raw mobility — it's **targeted mobility**.

---

## 7. How to Weigh Everything — My Proposed Method

### 7.1 The Problem with Current Approach

Current `move_tune` fits ONE weight per component (M, P, S, O). But relational terms need **per-term weights**:

```
P_i = w_isolated × isolated[i]
    + w_doubled × doubled[i]
    + w_connected × connected[i]
    + w_outpost × outpost[i]
    + w_rook_open × rook_open[i]
    + ...
```

If `w_isolated = 0` but `w_outpost = 5`, the tuner should find that. Bundling everything under P=0 loses good terms with bad terms.

### 7.2 My Proposed Tuning Architecture

**Phase 1: Unbundled move-agreement tuning**
- Fit one weight per relational term (6-10 weights total)
- Each term is its own `[i16; 4]` readout
- Hill-climb all weights simultaneously (not one at a time)
- Gate: non-zero weight AND lifts move-agreement above 18.3%

**Phase 2: Outcome confirmation (once decisive corpus exists)**
- Texel-tune the winning combo on self-play outcomes
- Gate: MSE drop vs baseline

**Phase 3: Self-play A/B**
- Config with relational terms vs config without
- Gate: statistically significant point difference

### 7.3 The Honest Fallback

If even unbundled tuning zeros everything:
- The 16-game corpus is truly the limit
- We need 50+ human games OR decisive self-play
- The relational terms are sound, but the data is insufficient to prove it

---

## 8. Summary — My Approach vs. Standard Chess

| Aspect | Standard Chess | 4PC FFA (My Approach) |
|--------|---------------|----------------------|
| King safety | Avoid checkmate | Avoid DKW (worse than mate) |
| King zone | Toward one enemy | Toward center (3 enemies converge) |
| Pawn lanes | Files only | Files OR ranks (player-relative) |
| Outpost | Unattackable by one enemy's pawns | Unattackable by ALL 3 enemies' pawns |
| Rook value | Open files | Open files + open ranks (corner rooks) |
| Coordination | Bishop pair, connected rooks | Swarm potential (multi-attacker bonus) |
| Non-linearity | King safety table only | King safety + swarm + gate bonuses |
| Scaling | By game phase | By opponent count (3→2→1) |

---

## Sources (All Freshly Fetched)

1. **Chessprogramming wiki — King Safety** (fetched 2026-06-06): https://www.chessprogramming.org/King_Safety
2. **Chessprogramming wiki — Outposts** (fetched 2026-06-06): https://www.chessprogramming.org/Outposts
3. **Chessprogramming wiki — Rook on Open File** (fetched via search): https://www.chessprogramming.org/Rook_on_Open_File
4. **Little Chess Evaluation Compendium** (fetched via search): http://www.winboardengines.de/doc/LittleChessEvaluationCompendium.pdf
5. **Little Chess Evaluation Compendium Addendum** (fetched via search): https://www.chessprogramming.org/images/4/49/Addendum6LCEC_2012.pdf
6. **Giraffe paper** (fetched via search): https://arxiv.org/pdf/1509.01549 (Stockfish eval breakdown)
7. **KTH thesis — Agent pawn structure** (fetched via search): https://kth.diva-portal.org/smash/get/diva2:642238/FULLTEXT01.pdf
8. **Chessify — Piece Activity** (fetched via search): https://chessify.me/blog/chess-position-evaluation
9. **Luminex engine v5.4.0** (fetched via search): trapped knight, far piece penalties
10. **Carballo engine changelog** (fetched via search): rook trapped, bishop trapped, pawn storm
