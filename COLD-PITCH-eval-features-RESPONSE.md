# Response to Cold Pitch: Human-Free Eval Feature Development

**Author:** Kimi · **Date:** 2026-06-06 · **Status:** Research + recommendation — no code yet

---

## Executive Summary

**Human-free eval development IS possible, but not in the way the pitch assumes.** The drawish-self-play problem is the real blocker, and it has a specific cause (points-blind eval + no elimination incentive) with a specific fix (bounty-aware move selection). Three viable paths exist, in order of practicality:

1. **Fix self-play decisiveness first** (bounty-biased move selection + longer games) → then use self-play Elo as the validation signal. This is the standard approach (Stockfish SPSA, Leela Zero).
2. **SPSA on tactical fixtures** — a small, curated set of positions with known-good moves (no human games needed, just chess theory). Tunes weights to maximize solve rate.
3. **Admit the anchor is unavoidable** — ~25 human games may be enough for move-agreement tuning if paired with self-play confirmation.

The honest answer: **Path 1 is the only one that scales to NNUE.** Paths 2 and 3 are stopgaps.

---

## Part 1: The Research — How Engines Actually Do This

### 1.1 Stockfish: SPSA + Self-Play (the gold standard)

Stockfish tunes its eval with **SPSA** (Simultaneous Perturbation Stochastic Approxulation):
- Define ~100-300 eval parameters (piece values, PST weights, mobility bonuses, etc.)
- Play **thousands of self-play games** at fast time control (10s+0.1s)
- Each SPSA iteration: perturb all parameters simultaneously, measure win-rate delta, update
- Converges in ~100k-1M games

**Key insight:** Stockfish's self-play is **decisive** because its eval values checkmate (king = high value, not 0), so engines pursue eliminations. Hornet's eval is **points-blind** (king = 0 cp, Hard Rule #8) — the search has no incentive to capture kings, so games drag to the ply cap.

**Reference:** SPSA paper: Spall, J. C. (1998). "An Overview of the Simultaneous Perturbation Method for Efficient Optimization." Johns Hopkins APL Technical Digest.

### 1.2 Leela Chess Zero: MCTS + Neural Net from Self-Play

Leela Zero uses **no human games at all** for training:
- Self-play generates games with MCTS (policy network guides search, value network evaluates)
- Neural net trains on (position, policy, outcome) triples
- The value network learns to predict *who wins from this position* — no human reference needed
- Bootstrapping: weak net → generates games → trains stronger net → repeat

**Key insight:** The value network is trained on **outcomes**, not human moves. The signal is "did the side with this position eventually win?" — but this requires **decisive games** (someone wins). Drawish games = no signal.

**Reference:** Silver et al. (2017). "Mastering Chess and Shogi by Self-Play with a General Reinforcement Learning Algorithm." arXiv:1712.01815.

### 1.3 Komodo / Houdini: CLOP + Human-Free Parameter Tuning

CLOP (Confident Local Optimization) by Rémi Coulom:
- Treats each parameter as a dimension in a Gaussian process
- Plays games, measures result, updates belief about parameter landscape
- More sample-efficient than SPSA for small parameter counts (<50)

**Reference:** Coulom, R. (2011). "CLOP: Confident Local Optimization for Noisy Black-Box Parameter Tuning." Advances in Computer Games 13.

### 1.4 Texel Tuning — What It Actually Needs

Texel tuning (Österlund 2014) fits eval weights to **game outcomes** by minimizing MSE between sigmoid(eval) and actual result (0=loss, 0.5=draw, 1=win).

**Requirements:**
- A corpus of **decisive games** (wins/losses, not draws)
- The eval must be **differentiable** (or at least smooth) in the parameters
- Enough games that the noise averages out (~10k+ for stable convergence)

**Hornet's current state:**
- `texel_tune` exists and works (EXP-009, EXP-015)
- Self-play corpus: 133 games, but **drawish** (many hit 150-ply cap with no elimination)
- Human corpus: 16 games, decisive (real FFA outcomes) but tiny
- The seat-order bug in `texel_tune` was fixed (EXP-015), so outcome labels are now correct

**Verdict:** Texel tuning on self-play is viable **only if self-play becomes decisive.**

---

## Part 2: The Drawishness Problem — Diagnosis and Fix

### 2.1 Root Cause Analysis

Why are Hornet self-play games drawish?

| Factor | Effect |
|--------|--------|
| **Points-blind eval** (king = 0 cp) | Search doesn't value king captures → no elimination drive |
| **Balanced opening** | All players start equal, no early pressure |
| **Defensive play** | Max^n with equal evals → conservative, no risks |
| **150-ply cap** | Games end before endgame pressure builds |
| **No material-imbalance snowball** | 4PC: capturing a piece removes it from the board, but the other two players gain relatively → no runaway leader |

**The key:** In 2-player chess, a material advantage snowballs (fewer pieces = easier to force mate). In 4PC FFA, a material advantage is **diluted** — you still face 3 opponents, and the other two may gang up on you. So even "winning" positions don't convert easily without active king-hunting.

### 2.2 Fixes (in order of impact)

#### Fix A: Bounty-biased move selection (highest impact, easiest)
The search already has `ffa_points` (P1 N3 B3 R5 Q9 K20) in `types.rs`. The move ordering has a bounty term. **Extend this to the self-play policy:**

```
Current self-play policy: choose the search's top move (eval-greedy)
Bounty-biased policy: with probability ε, choose a capture move weighted by ffa_points(victim)
```

This injects **aggression** without changing the eval. The engine still evaluates positions the same way, but it *plays* more aggressively — seeking eliminations, creating decisive games.

**Implementation:** In `examples/bootstrap.rs` and `examples/selfplay.rs`, add an `aggression` parameter:
- `aggression = 0.0`: current behavior (eval-greedy)
- `aggression = 0.3`: 30% of the time, pick a capture move (weighted by bounty) instead of the search top move
- This is **not** a search change — it's a data-generation policy change

**Expected effect:** More king captures → more eliminations → more decisive games → stronger outcome labels.

#### Fix B: Longer ply cap (medium impact, trivial)
Current cap: 150 plies. Many games are "developing" at 150 plies in 4PC. Try 300 or 400 plies.

**Trade-off:** Slower data generation. But if bounty-biased selection creates eliminations, the cap becomes irrelevant (games end early).

#### Fix C: Opening diversity (already partially done)
`bootstrap.rs` already uses 12 random opening plies. This is good — keep it. More diversity = more varied positions = more learning signal.

#### Fix D: Make the eval value eliminations (harder, touches Hard Rule #8)
Hard Rule #8 says eval is points-blind (centipawns only, no FFA points). But the **search** could value eliminations without the eval changing:
- Terminal nodes: checkmate/stalemate already score MATE (±30,000 cp)
- King captures: the search sees the material swing (queen+pieces removed), but the **eval at the leaf** doesn't know a player was eliminated

**Option:** Add a small "elimination bonus" to the eval — e.g. when a player is dead, the eval for the surviving players gets a +500 cp bump. This is still centipawns, not FFA points. **This would require a change order** (Hard Rule #8 interpretation).

**Verdict:** Fix A (bounty-biased selection) is the lowest-risk, highest-impact fix. It doesn't touch the eval or search — just the self-play policy.

---

## Part 3: Three Viable Human-Free Development Loops

### Path 1: Bounty-Biased Self-Play + SPSA (the scalable path)

**Goal:** Generate decisive self-play games, then use SPSA to tune eval parameters.

**Steps:**
1. **Implement bounty-biased move selection** in `bootstrap.rs` (ε-greedy with capture weighting)
2. **Generate a decisive corpus** (target: 1,000+ games with >50% having eliminations)
3. **Implement SPSA tuner** (or extend `texel_tune` to do gradient-free optimization)
4. **Tune eval weights** (M, P, S, O) on the decisive corpus
5. **Validate by self-play Elo** (A-vs-B head-to-head)

**Pros:** Scales to NNUE. No human games needed. Standard approach.
**Cons:** Needs engineering (SPSA implementation). Needs compute (1,000+ games).
**Timeline:** 2-3 sessions for the infrastructure, then overnight runs.

### Path 2: Tactical Fixture SPSA (the fast path)

**Goal:** Use the 25 tactical positions in `baselines/tactical_samples.json` as a fixed benchmark.

**Insight:** These positions have **known-good moves** (the human's move). We don't need human games — we need the engine to find the tactic. The "solve rate" (engine move == human move) is the signal.

**Steps:**
1. **Define eval parameters** as the SPSA variables (e.g. W_MATERIAL, W_POSITIONAL, W_SAFETY, W_CROSSFIRE, plus any new feature weights)
2. **For each parameter set:** run the 25 fixtures through `examples/strength_gate.rs`, measure solve rate
3. **SPSA iterates:** perturb parameters, measure solve-rate delta, update
4. **Converge on max solve rate**

**Pros:** Fast (25 positions vs 1,000 games). No self-play needed. Directly optimizes tactical strength.
**Cons:** Only exercises tactics, not positional play. May overfit to the 25 fixtures.
**Timeline:** 1 session for SPSA harness, then a few hours of tuning.

### Path 3: Hybrid — Small Human Anchor + Self-Play Confirmation (the pragmatic path)

**Goal:** Use a small human corpus for move-agreement tuning, confirm with self-play.

**Insight:** The 16-game corpus is small but **not zero signal**. The 18.3% ceiling is real — it tells us the current features are insufficient. But we don't need 50 games to *test* a new feature — we need 50 games to *trust* the result.

**Steps:**
1. **Develop new features** (pawn structure, relational terms) using theory/chess knowledge
2. **Gate on move_tune with the 16-game corpus** — if it doesn't lift the 18.3% ceiling, it's probably dead
3. **Confirm with self-play Elo** — if move_tune says it's good, does it win more games?
4. **Acquire more human games** only when a feature passes step 2 and 3

**Pros:** Uses existing infrastructure (`move_tune`, `move_match`, `selfplay`). No new tuning method needed.
**Cons:** Still needs some human games for the initial gate. Self-play confirmation needs the drawishness fix.
**Timeline:** Immediate — this is what we've been doing, just with a clearer confirmation step.

---

## Part 4: Which Features to Try (Human-Free or Not)

### 4.1 Already Tested (all P=0)

| Feature | Type | Result |
|---------|------|--------|
| PST centrality | Per-square | P=0 |
| PST anti-centrality | Per-square | P=0 |
| PST zone-aware | Per-square | P=0 |
| PST per-piece | Per-square | P=0 |
| Zone control | Zone-sum | P=0 |
| Mobility | Count | P=0 |
| Development tempo | Dynamic count | P=0 |

### 4.2 Untested — Relational Features (the next class)

These are **not per-square** — they depend on relationships between pieces:

| Feature | Why it might work | How to test |
|---------|-------------------|-------------|
| **Pawn structure** (isolated/doubled/connected) | Relational, not positional; humans think in pawn chains | Implement, gate on move_tune |
| **Rook on open file** | Rooks are periphery pieces; open files = their purpose | Implement, gate on move_tune |
| **Outposts** (knight/bishop) | Defended by pawn, unattackable by enemy pawns | Implement, gate on move_tune |
| **King safety table** | Non-linear: multiple attackers compound | Build danger table, gate on move_tune |
| **Defended pieces** | Pieces defended by pawns = harder to dislodge | Implement, gate on move_tune |

**Key insight from PITCH-relational-eval-terms.md:** The reason prior features zeroed is **bundling** — all positional features shared one weight P. If pawn structure is good but PST is bad, the tuner zeros P and loses both. The fix: **unbundle** — each relational term gets its own weight, then bake survivors into P.

### 4.3 The Real Question: Can These Be Tuned Human-Free?

**No — not for the initial weight derivation.** The relational features need a signal to fit against. Options:

1. **Move-agreement on human corpus** (current method) — needs human games
2. **Self-play Elo** — needs decisive self-play (Fix A above)
3. **Tactical fixture solve rate** — needs the 25 fixtures (no human games!)

**Option 3 is the human-free path for relational features:**
- Implement pawn structure, rook open file, outposts, king safety table
- For each: run the 25 tactical fixtures, measure if solve rate improves
- If a feature improves solve rate → keep it, tune weight by fixture SPSA
- If not → drop it

**Caveat:** Tactical fixtures only test tactics, not positional play. A feature that helps positional play but not tactics would be falsely rejected. But given that **all prior positional features zeroed on move-agreement**, and move-agreement includes both tactical and positional positions, this is a reasonable filter.

---

## Part 5: Recommendation — What to Build

### Immediate (this session)

1. **Implement bounty-biased move selection** in `examples/bootstrap.rs`:
   ```rust
   // In the self-play loop, after getting the search's top move:
   let mv = if rng < aggression && !captures.is_empty() {
       // Weighted random capture by ffa_points(victim)
       weighted_random(&captures, |m| ffa_points(board.piece_at(m.to).unwrap()))
   } else {
       search_top_move
   };
   ```
   This is a 10-line change. Test: run 100 games, measure % with eliminations.

2. **If eliminations rise:** generate a 500-game decisive corpus overnight.

### Short-term (next 2-3 sessions)

3. **Implement one relational feature** (pawn structure is the biggest potential gain):
   - `query_pawn_structure(board) -> [i16; 4]` with isolated + doubled + connected
   - Unbundled: its own weight, not folded into P yet
   - Gate on tactical fixtures first, then on move_tune if fixtures pass

4. **Build SPSA harness** (or extend `texel_tune`):
   - Input: eval parameters, self-play corpus
   - Output: tuned parameters, measured win-rate
   - Start with just the 4 main weights (M, P, S, O) on decisive self-play

### Medium-term (after decisive corpus exists)

5. **Full relational feature suite:**
   - Pawn structure → P
   - Rook open file → P
   - Outposts → P
   - King safety table → S
   - Each unbundled, SPSA-tuned, self-play validated

6. **NNUE training** (P7) — only after the hand-tuned eval clears the strength gate

---

## Part 6: Honest Assessment — Is Human-Free Viable?

**Yes, but with a caveat.**

The standard engine development pipeline (Stockfish, Leela) is:
1. Hand-craft eval features (human knowledge)
2. Tune weights with self-play (SPSA / gradient descent)
3. Validate with self-play Elo
4. Optionally: confirm with human-game move-agreement

**Hornet is stuck at step 2 because self-play is drawish.** Fix the drawishness (bounty-biased selection), and the pipeline works. **No human games are needed for tuning — only for final validation.**

The minimum human data needed:
- **For tuning:** 0 games (if self-play is decisive)
- **For validation:** ~25 games for move-agreement sanity check
- **For NNUE training:** 10,000+ games (self-play generated)

**The real bottleneck is not human games — it's the eval's inability to drive eliminations, which makes self-play useless.** Fix that, and the rest follows.

---

## References

1. Spall, J. C. (1998). "An Overview of the Simultaneous Perturbation Method for Efficient Optimization." *Johns Hopkins APL Technical Digest*, 19(4), 482-492.
2. Silver, D., et al. (2017). "Mastering Chess and Shogi by Self-Play with a General Reinforcement Learning Algorithm." *arXiv:1712.01815*.
3. Coulom, R. (2011). "CLOP: Confident Local Optimization for Noisy Black-Box Parameter Tuning." *Advances in Computer Games 13*, 146-157.
4. Österlund, P. (2014). "Texel's Tuning Method." *Chess Programming Wiki*.
5. Sturtevant, N. R., & Korf, R. E. (2000). "On Pruning Techniques for Multi-Player Games." *AAAI/IAAI*, 201-207.
6. Korf, R. E. (1991). "Multi-player Alpha-Beta Pruning." *Artificial Intelligence*, 48(1), 99-111.

---

## Verify

```bash
cd hornet-engine
cargo test --lib                          # keep green

# Test bounty-biased self-play
cargo run --release --example bootstrap 100  # baseline
cargo run --release --example bootstrap 100  # with aggression=0.3 (after implementing)

# Compare: % games with eliminations
```
