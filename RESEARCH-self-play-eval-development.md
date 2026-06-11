# Research: Developing Chess Engine Evaluation Features WITHOUT Human Games

**Date:** 2026-06-06
**Context:** Hornet 4PC engine

---

## 1. Self-Play Tuning Methods

### 1.1 SPSA (Simultaneous Perturbation Stochastic Approximation)

**What it is:** A gradient-free optimization method that perturbs ALL parameters simultaneously, plays games, and steps toward what wins.

**How it works for chess eval:**
1. Start with a parameter vector theta.
2. Generate a random perturbation vector Delta (each component +/-1 with probability 0.5).
3. Play N games with theta + c*Delta and N games with theta - c*Delta.
4. Compute the estimated gradient: g_hat = [score(theta+cD) - score(theta-cD)] / (2c*D_i).
5. Step: theta <- theta - a*g_hat.
6. Decay a and c over iterations.

**Why it is the right tool for search-shape tuning:**
- Eval has ~4-20 parameters; search shape has ~5-10 (beam width, deep floor, LMR threshold, etc.).
- SPSA needs only **2 games per iteration** (one with +D, one with -D) regardless of parameter count.
- The gradient is noisy — but that is fine because the step size decays.

**Stockfish use:** Fishtest runs SPSA on eval parameters continuously. A typical SPSA session: 200k games, 100+ parameters, converges in ~2 weeks on distributed hardware. For Hornet smaller param space, convergence is much faster.

**Hornet applicability:**
- **Eval weights:** Already exhausted (EXP-009: 4 weights are optimal; further gains need new features, not weight tuning).
- **Search-shape knobs:** PERFECT target. beam_width, deep_floor, adaptive_beam taper, forward_pruning threshold, LMR_LATE_MOVES.
- **Implementation:**  — a loop that perturbs Searcher config, runs selfplay.rs mini-matches (say 20 games), and steps the vector.

**Key paper:** Spall 1992, *Multivariate Stochastic Approximation Using a Simultaneous Perturbation Gradient Approximation*.

---

### 1.2 Texel Tuning — Can It Work with Self-Play?

**What it is:** Fit eval parameters to predict game outcomes. Positions are labeled with the result (W/D/L -> 1/0.5/0), cost = MSE between sigmoid(eval) and result.

**The drawishness problem:**
- Texel needs **decisive games**. If 80% of games are draws, the label is 0.5 for most positions -> the gradient is flat -> tuning stalls.
- Stockfish solution: use **quiet positions from fast games** (not the whole game), and the games are decisive because Stockfish at short TC still blunders into losses.

**Can it work for Hornet self-play?**
- **Current self-play (EXP-013):** 150-ply cap, no eliminations, points [0,17,10,13] — weak labels. Texel on this gets MSE ~0.132 (worse than human corpus 0.1146). The tuner zeroes everything except material (EXP-014).
- **The fix:** Make self-play **decisive** first (see section 2), THEN Texel-tune on it. The method is sound; the data is the blocker.

**Texel on self-play outcomes (4PC adaptation):**
- Label: per-player placement points -> normalize to [0,1].
- Sigmoid: per-player eval component -> predicted placement probability.
- Cost: sum_i (result_i - sigmoid(U_i))^2.
- This is exactly what  does — it works, but only if the games have signal.

---

### 1.3 Reinforcement Learning / Zero-Style Self-Play

**What it is:** AlphaZero/Leela Chess Zero style. A neural net generates both policy (move probabilities) and value (position evaluation). MCTS uses the net to guide search. Games are self-played, outcomes train the net.

**The loop:**
1. Net generates policy pi(s) and value v(s) for position s.
2. MCTS runs N simulations, using pi as prior and v as leaf evaluation.
3. MCTS produces an improved policy pi-prime(s) (visit counts -> move probabilities).
4. Self-play a game: at each position, store (s, pi-prime, z) where z = game outcome.
5. Train net to minimize: L = (z - v(s))^2 - pi-prime * log pi(s) + c*||theta||^2.
6. Repeat.

**Why it is NOT the next step for Hornet:**
- **Data hunger:** Leela trained on ~1 billion self-play positions. Hornet has ~900 positions from 16 human games + a few hundred self-play games.
- **Net architecture:** Hornet NNUE is Phase 7, gated behind the strength gate (Hard Rule #7). The hand-eval must be strong enough to serve as a teacher before NNUE training begins.
- **The drawishness problem is worse:** Zero-style training needs wins/losses as the z-label. If games are draws, z = 0.5 everywhere -> the value head learns nothing.

**When to use it:** After the hand-eval passes the strength gate AND a large decisive self-play corpus exists (10k+ games). Then Zero-style is the path to superhuman strength.

**Key paper:** Silver et al. 2017, *Mastering Chess and Shogi by Self-Play with a General Reinforcement Learning Algorithm* (arXiv:1712.01815).

---

### 1.4 CLOP and Other Parameter Tuning

**CLOP:** A Bayesian optimization method for tuning chess parameters. It models the win-rate as a Gaussian process and selects the next parameter point to test based on expected improvement.

**How it differs from SPSA:**
- SPSA = stochastic gradient descent (many small steps, noisy gradient).
- CLOP = Bayesian optimization (fewer evaluations, smarter point selection).

**When to use CLOP vs SPSA:**
- **CLOP:** Few parameters (<=10), expensive evaluations (long games), need sample efficiency.
- **SPSA:** Many parameters (20+), cheap evaluations (fast games), can afford noise.

**Hornet applicability:**
- For eval **feature weights** (4-8 parameters once relational terms land), CLOP is viable.
- For **search-shape** (5-10 parameters), SPSA is better (more params, faster games).

**Other methods:**
- **Local search (Texel-style):** Perturb one parameter at a time +/-1, keep if MSE drops. Works for 4 weights, does not scale to 20+.
- **Genetic algorithms:** Population of configs, tournament selection, crossover/mutation. Used in some engines (e.g., Rodent). Slower than SPSA for this scale.
- **CMA-ES:** Covariance Matrix Adaptation Evolution Strategy. More sample-efficient than SPSA, harder to implement. Overkill for Hornet current param count.

**Key reference:** Coulom 2011, *CLOP: Confident Local Optimization for Noisy Black-Box Parameter Tuning* (used in CrazyAra and other engines).

---

## 2. The Drawishness Problem

### 2.1 The Problem (Hornet-Specific)

**Current state (EXP-013):**
- Self-play games hit the 150-ply cap with **no eliminations**.
- Final points: [0,17,10,13], [2,10,4,10] — small differences, no zeros.
- The eval is **points-blind** (king capture = 0 cp but = 20 FFA points). The search does not value eliminations.
- Result: cautious, balanced play. No one takes risks -> no one dies -> draws.

**Why this matters:**
- Outcome-based tuning (Texel, SPSA) needs variance in outcomes. All draws = zero gradient.
- Self-play Elo needs wins/losses. All draws = Elo difference approx 0, even if one config is genuinely better.

---

### 2.2 Solutions — From Literature and Practice

#### A. Aggressive Move Selection (Temperature, Epsilon-Greedy)

**Temperature (tau):** At move selection, sample from a softmax over move scores with temperature tau.
- tau -> 0: deterministic (always pick best move = current behavior).
- tau -> infinity: uniform random.
- **tau = 0.5-1.0:** noisy but biased toward good moves -> more blunders -> more decisive games.

**Epsilon-greedy:** With probability epsilon, play a random legal move instead of the search top choice.
- epsilon = 0.05-0.10 is typical for training data generation.
- Leela uses a Dirichlet noise at the root (similar idea: perturb the policy prior).

**Hornet implementation:**
```rust
// In selfplay.rs: add temperature to move selection
fn pick_move_with_temperature(scores: &[(Move, [i16;4])], mover: usize, tau: f64) -> Move {
    let exp_scores: Vec<f64> = scores.iter()
        .map(|(_, v)| (v[mover] as f64 / tau).exp())
        .collect();
    let sum: f64 = exp_scores.iter().sum();
    let r = rng.gen::<f64>() * sum;
    // cumulative pick...
}
```

**Expected effect:** More captures, more king-hunts, more eliminations. The games become decisive enough for Texel/SPSA.

---

#### B. Forced Opening Diversity

**What it is:** Do not play from the start position. Use a large opening book or random opening moves to ensure each game starts differently.

**Hornet already does this (EXP-010/013):** 12 random opening plies. This is good but not enough — the random moves are uniform, not principled.

**Improvements:**
1. **Biased random openings:** Weight moves by capture probability, king proximity, etc. — encourage sharp positions.
2. **Opening book from human games:** Even 16 games give ~100 unique positions. Use those as starting points.
3. **Forced asymmetric starts:** Start with one player already having a material advantage (e.g., Red is up a pawn). This creates a natural target for the other three to gang up on.

**Implementation:**
```rust
// In bootstrap.rs: biased random opening
fn biased_random_move(board: &Board, rng: &mut impl Rng) -> Move {
    let legal = generate_legal(board);
    let weights: Vec<f64> = legal.iter().map(|m| {
        let mut score = 1.0;
        if m.is_capture() { score += 5.0; }
        if attacks_king_zone(m) { score += 3.0; }
        score
    }).collect();
    weighted_sample(&legal, &weights, rng)
}
```

---

#### C. Eval Perturbation / Noise Injection

**What it is:** Add random noise to the static eval at leaf nodes. This makes the search explore lines it would otherwise dismiss, creating more diverse and decisive games.

**How Leela does it:** FPU (First Play Urgency) reduction + virtual loss + Dirichlet noise at the root.

**Hornet implementation:**
```rust
// In eval.rs: add noise to leaf eval
pub fn eval_4vec_noisy(board: &Board, lines: &mut LineMap, noise_scale: i16) -> [i16; 4] {
    let mut v = eval_4vec(board, lines);
    for i in 0..4 {
        v[i] += rng.gen_range(-noise_scale..=noise_scale);
    }
    v
}
```
- noise_scale = 50-100 cp (half a pawn to a pawn) — enough to change move choice occasionally.
- Only inject at **search leaves** (not at the root, where it would just play randomly).

**Expected effect:** The search occasionally misevaluates a position, leading to unsound sacrifices or missed defenses -> decisive games.

---

#### D. Alternative Objectives — When Outcomes Are Too Weak

If games are still too drawish, change what you are optimizing for:

**1. Material advantage at game end (instead of placement points):**
- Label each position by the mover material difference vs average at game end.
- This is a **continuous** signal (not discrete win/loss/draw) -> always has gradient.
- Correlates with winning but is denser.

**2. Max-N-ply survival (instead of elimination):**
- Label: how many plies did this player survive?
- A player that dies at ply 50 is worse than one that dies at ply 150.
- Continuous, always defined, no draws.

**3. Capture count / bounty earned:**
- Label: total ffa_points the player captured during the game.
- Directly optimizes for the FFA objective, even in no-elimination games.

**4. Combo: placement + material + survival:**
```
target = 0.5 * placement_score + 0.3 * material_advantage + 0.2 * survival_fraction
```
- Multi-objective tuning — the optimizer gets signal from whichever dimension has variance.

**Hornet implementation:**
```rust
// In texel_tune.rs: alternative label
fn alternative_label(game: &Pgn4Game, player: Player) -> f64 {
    let placement = placement_score(game.result, player); // 1.0, 0.67, 0.33, 0.0
    let material_adv = game.final_material[player] as f64 / 1000.0; // in pawns
    let survival = game.ply_of_death[player] as f64 / game.total_plies as f64;
    0.5 * placement + 0.3 * material_adv + 0.2 * survival
}
```

---

#### E. The Points-Blind Eval Fix — Make the Search Value Eliminations

**The root cause of drawishness:** The eval returns V = <M,P,S,O> in centipawns. King capture = 0 cp (king has no eval_value). But in FFA, king capture = 20 points + eliminates a player.

**The search does not know this.** So it plays to maximize centipawns, not FFA points. A move that eliminates a player (huge FFA gain) scores 0 in the eval.

**Fixes (pick one):**

**1. Bounty term in eval (relax Hard Rule #8 slightly):**
- Add a small term to O_i: bounty_penalty = ffa_points_at_risk for pieces under attack.
- This makes the eval aware that attacking a king is valuable (20 points).
- Scale: 1 FFA point approx 10-20 cp (empirically: a pawn = 1 pt = 100 cp).
- **Risk:** Conflates eval_value and ffa_points — Hard Rule #8 says never do this. But a bounded, small bounty term may be the lesser evil.

**2. Terminal scoring enhancement (already partially done):**
- In search.rs, a checkmate/stalemate node already returns -(MATE - ply).
- A **king-capture** node should return a large positive score for the capturer (e.g., +MATE/2).
- This makes the search **want** to capture kings, even if the static eval does not value it.

**3. Move ordering bounty (already done in move_order.rs):**
- MVV-LVA already uses ffa_points(victim) as a tiebreaker.
- This helps the search **find** king captures, but does not make the eval **value** them.

**Recommendation:** Implement #2 (terminal king-capture score) first — it is search-level, not eval-level, so it does not violate Hard Rule #8. Then test if self-play becomes more decisive.

---

## 3. Feature Development Without Human Reference

### 3.1 The Validation Problem

**Current Hornet state:**
- 8 positional variants tested -> all tune to P=0 (EXP-015).
- Move-match ceiling = 18.3% (material+crossfire only).
- The problem: **no signal to validate against.** Human games are too few; self-play is too drawish.

**The question:** How do you know a new feature is good if you cannot match human moves?

---

### 3.2 Validation Signals — In Order of Reliability

#### Signal 1: Self-Play Elo (The Gold Standard)

**How it works:** Play config A vs config B in a head-to-head match. Compute Elo difference from the score.

**For 4PC specifically:**
- A plays in seat 0, B in seats 1-3. Then rotate A through all 4 seats. Average A points per seat.
- This cancels seat bias (Yellow third-mover advantage is real).
- Score by FFA points, not W/L/D (4PC has placements, not binary outcomes).

**How many games for significance?**
- In 2-player chess: ~1000 games for +/-5 Elo at 95% confidence (SPRT can accept/reject in fewer).
- In 4PC: **higher variance** (4 players, seat bias, more randomness). Estimate: **2000-4000 games** for +/-10 Elo.
- At Hornet current speed (~6 min/game at depth-8, ~15 min at depth-12): 2000 games = **200 hours** (8 days) at depth-8, or **500 hours** (21 days) at depth-12.
- **This is tractable but slow.** The workaround: use **faster games** (depth-4, flat beam) for bulk screening, then depth-8/12 for confirmation.

**SPRT for 4PC:**
- Sequential Probability Ratio Test: accept/reject a change with the minimum number of games.
- For 4PC, the score is points per game (not W/L). Use a **paired t-test** on the point difference.
- SPRT parameters: H0 (null hypothesis) = 0 Elo gain, H1 (alternative) = 5 Elo gain. alpha = beta = 0.05.
- Expected games to decision: ~500-1500 for a real 5 Elo gain.

**Hornet implementation:**
```rust
// In examples/spsa_tune.rs or a new sprt_harness.rs
struct SprtState {
    wins: u32, losses: u32, draws: u32,
    elo0: f64, elo1: f64, alpha: f64, beta: f64,
}
impl SprtState {
    fn llr(&self) -> f64 {
        // Log-likelihood ratio for 4PC point-scoring
        // Adapted from Fishtest SPRT formula
    }
    fn status(&self) -> SprtStatus { /* Continue / Accept / Reject */ }
}
```

---

#### Signal 2: Internal Consistency Checks

**Does the feature correlate with what it should?**

For each new feature, check:
1. **Game phase correlation:** King safety should matter more in the opening/middlegame than endgame. Pawn structure should matter throughout. If the feature is flat across phases, it is noise.
2. **Material correlation:** A feature should not correlate perfectly with material (or it is redundant). Compute correlation between feature and material across positions — if r > 0.8, it is not adding new signal.
3. **Positional correlation:** Two features should not correlate perfectly with each other (or they are double-counting). Compute feature-feature correlation matrix.
4. **Tactical sanity:** A feature that claims a position is good should not be on a position where SEE says you are losing material.

**Hornet implementation:**
```rust
// In examples/feature_audit.rs
fn audit_feature(corpus: &[Position]) -> FeatureAudit {
    let phase = corpus.iter().map(|p| game_phase(p)).collect();
    let material = corpus.iter().map(|p| query_material(p)).collect();
    let feature = corpus.iter().map(|p| query_new_feature(p)).collect();
    FeatureAudit {
        phase_correlation: correlation(&feature, &phase),
        material_correlation: correlation(&feature, &material),
        see_agreement: corpus.iter().filter(|p| {
            feature(p) > 0 && see_worst_capture(p) < 0
        }).count() as f64 / corpus.len() as f64,
    }
}
```

---

#### Signal 3: Blunder Rate (Already Implemented)

**What it is:** gate_ablation.rs checks whether the engine move loses material (SEE-negative capture, hangs a piece).

**Why it is a good sanity signal:**
- It does not require human reference.
- It is objective: losing material is bad, full stop.
- A new feature that **reduces** blunder rate is almost certainly good (or at least not harmful).

**Limitation:** It only catches tactical blunders, not positional errors. A feature can pass the blunder gate and still be strategically bad.

---

#### Signal 4: Search Stability

**Does the feature make the search more stable?**

A stable search:
- The best move does not oscillate with small depth changes.
- The eval of the best move increases monotonically with depth (if it drops, the search found a refutation — the eval was over-optimistic).

**Measurement:**
```rust
// In examples/search_stability.rs
for depth in [4, 8, 12, 16] {
    let (mv, score) = searcher.search(board, depth);
    record(depth, mv, score);
}
// Check: does best move change? Does score trend upward?
```

**Why this matters:** A good eval makes the search **converge** — deeper search agrees with shallower search. A bad eval makes the search **diverge** — deeper search finds refutations the eval missed.

---

### 3.3 The Feature Development Loop (Human-Free)

```
1. INTUITION -> Define the feature predicate
        |
        v
2. IMPLEMENT -> Add query_xxx() in queries.rs, default-off, ablatable
        |
        v
3. INTERNAL CHECK -> Run feature_audit.rs:
   - Does it correlate with phase? material? other features?
   - Does it pass blunder-rate sanity?
        |
        v
4. TEXEL PRE-SCREEN -> Run texel_tune on self-play corpus:
   - Does the feature get a non-zero weight?
   - Does it lower MSE (even slightly)?
   - If no: the feature is dead on this corpus. Ablate.
        |
        v
5. SELF-PLAY A/B -> Run 100-500 fast games (depth-4, flat beam):
   - Config A = baseline, Config B = baseline + feature
   - Does B score more points per seat?
   - If no significant difference: feature is too weak to detect at this speed.
        |
        v
6. DEEP CONFIRMATION -> Run 100-200 slow games (depth-8/12):
   - Confirm the fast-game result.
   - If confirmed: feature is real. Ship it.
   - If not: the feature helps shallow search but not deep search.
```

**Key insight from EXP-015:** The move-match gate (vs human moves) is **not** part of this loop. It has been retired as a tuning signal. The loop above is fully human-free.

---

## 4. Four-Player Specific Considerations

### 4.1 FFA Dynamics — How Self-Play Elo Works with 4 Players

**The problem:** In 2-player chess, Elo is well-defined: A beats B, A gains Elo, B loses Elo. In 4-player FFA, the outcome is a **placement vector** (1st, 2nd, 3rd, 4th), not a binary win/loss.

**How to compute Elo in 4PC:**

**Method 1: Pairwise Elo (the standard approach)**
- Treat each game as 6 pairwise comparisons (R vs B, R vs Y, R vs G, B vs Y, B vs G, Y vs G).
- For each pair, the winner is the one with more points (or better placement).
- Update Elo for each pair independently.
- A player overall Elo is the average of their pairwise Elos.

**Method 2: Bradley-Terry model (generalized to 4 players)**
- The probability that player i beats player j is: P(i > j) = 10^((Elo_i - Elo_j)/400).
- For 4 players, the probability of a specific ranking is the product of pairwise probabilities.
- Maximum likelihood estimation over many games gives the Elo ratings.

**Method 3: Points-per-game (simplest)**
- Just compute average FFA points per game for each config.
- A config that scores 15 pts/game is better than one that scores 10 pts/game.
- No Elo math needed — just a t-test on the point difference.

**Hornet current approach (EXP-010):** Method 3 — average points per seat. This is fine for A/B testing. For a public rating system, Method 1 or 2 is better.

---

### 4.2 Elimination-Based Outcomes vs Draw-Based Outcomes

**Elimination-based:**
- The game ends when <=1 player remains.
- Outcome: who survived, in what order.
- **Pros:** Clear signal (survival is binary). Matches the real FFA win condition.
- **Cons:** Games can be very long (200+ plies). Some games may never end (infinite loop of cautious play).

**Draw-based (current Hornet):**
- Game ends at a ply cap (150 plies).
- Outcome: points accumulated so far.
- **Pros:** Fixed length, predictable runtime.
- **Cons:** Weak signal (small point differences). Does not match the real win condition.

**Hybrid (recommended):**
- Play to elimination OR 200-ply cap, whichever comes first.
- If capped, score by points + a survival bonus (still alive = +5 pts).
- This gives strong signal for eliminations and moderate signal for capped games.

**Implementation:**
```rust
// In game.rs: hybrid scoring
const PLY_CAP: usize = 200;
const SURVIVAL_BONUS: u16 = 5;

fn final_points(game: &Game) -> [u16; 4] {
    let mut pts = game.points;
    for p in Player::ALL {
        if !game.board.dead[p.index()] {
            pts[p.index()] += SURVIVAL_BONUS;
        }
    }
    pts
}
```

---

### 4.3 The Points-Blind Eval Problem

**Restated:** The eval is in centipawns. King capture = 0 cp. But in FFA, king capture = 20 points + eliminates a player. The search optimizes cp, not points.

**Why this causes drawishness:**
- A move that wins a pawn = +100 cp. The search likes this.
- A move that checkmates a player = +0 cp (king has no eval_value). The search ignores this.
- So the engine trades pawns instead of hunting kings -> no eliminations -> draws.

**Solutions (in order of invasiveness):**

**1. Terminal scoring (search-level, already partially done):**
- A checkmate node returns -(MATE - ply) for the mated player.
- A **king-capture** node should return +MATE/2 for the capturer (and -(MATE - ply) for the victim).
- This makes the search **want** to deliver king-captures, without changing the eval.

**2. Bounty-aware eval (eval-level, violates Hard Rule #8 slightly):**
- Add a small term: bounty_at_risk = sum ffa_points(victim) for pieces under attack.
- Scale: 1 FFA point approx 10-20 cp.
- This makes the eval aware that attacking a king (20 pts) is like attacking a queen (9 pts) + a rook (5 pts) + ...

**3. Separate value head (NNUE-level):**
- The NNUE has two outputs: v_cp (centipawn value) and v_pts (FFA point value).
- The search uses v_cp for move ordering and v_pts for terminal decisions.
- This is the cleanest solution but requires NNUE (Phase 7).

**Recommendation for now:** Implement #1 (terminal king-capture score) + #2 (small bounty term, gated). Test if self-play becomes more decisive.

---

## 5. Concrete Implementation Plan for Hornet

### Phase 0: Fix the Drawishness (Prerequisite for Everything Else)

**Goal:** Make self-play games decisive enough for tuning.

1. **Terminal king-capture scoring:**
   - In search.rs::maxn, when a move captures a king, return a large positive score for the capturer.
   - v[capturer] += MATE / 2; v[victim] = -(MATE - ply);
   - This makes the search value eliminations.

2. **Temperature in self-play:**
   - Add tau: f64 to Searcher (default 0 = deterministic).
   - In selfplay.rs, set tau = 0.5 for training games.
   - This creates more blunders -> more decisive games.

3. **Biased random openings:**
   - In bootstrap.rs, weight random moves by capture probability.
   - This starts games in sharper positions.

4. **Hybrid scoring (survival bonus):**
   - In game.rs, add +5 points for surviving to the cap.
   - This rewards cautious play slightly less.

**Expected result:** Self-play games have eliminations 20-50% of the time (vs 0% today). Outcome variance increases. Texel and SPSA now have signal.

---

### Phase 1: Bootstrap Corpus + Texel Pre-Screen

**Goal:** Generate a large enough corpus to pre-screen features.

1. **Run bootstrap with Phase 0 fixes:**
   - 1000 games at depth-4 (fast, ~1 min/game).
   - 500 games at depth-8 (quality, ~6 min/game).
   - Total: ~2 days of compute.

2. **Texel pre-screen:**
   - For each new feature, run texel_tune on the bootstrap corpus.
   - Gate: non-zero weight AND MSE drop > 0.001 (above noise).
   - If a feature passes, proceed to Phase 2.

---

### Phase 2: SPSA for Search Shape

**Goal:** Optimize search knobs (beam width, deep floor, LMR threshold, etc.).

1. **Parameterize Searcher:**
   - Extract all knobs into a SearchConfig struct.
   - beam_width, deep_floor, adaptive_beam taper, LMR_LATE_MOVES, LMR_MIN_DEPTH, QUIESCENCE_MAX_PLY.

2. **SPSA loop:**
   - Perturb the config vector.
   - Run 20 fast games (depth-4) vs baseline.
   - Step toward the winner.
   - Repeat for 100 iterations.

3. **Confirmation:**
   - Run 100 depth-8 games with the SPSA-winning config vs baseline.
   - t-test on point difference.

---

### Phase 3: Relational Feature Development

**Goal:** Build and validate pawn structure, rook-on-open-line, outpost, king-safety table.

1. **Implement each feature as a separate query (already started in query_pawn_structure).**
2. **Internal audit:** feature_audit.rs — correlation, blunder rate, search stability.
3. **Texel pre-screen:** On the Phase 1 corpus.
4. **Self-play A/B:** 100 fast games, then 50 slow games.
5. **Keep only features that pass both gates.**

---

### Phase 4: Deep Confirmation + SPRT

**Goal:** Confirm that the accumulated improvements are real.

1. **SPRT harness:** Implement SPRT for 4PC point-scoring.
2. **Run SPRT:** New config vs baseline, depth-12, target 5 Elo gain.
3. **If accepted:** The config is a real improvement. Update the default.

---

## 6. Summary — What to Build, in Order

| Priority | Item | Method | Effort | Expected Impact |
|----------|------|--------|--------|-----------------|
| **P0** | Fix drawishness (terminal king-capture, temperature, biased openings) | Code + config | 1 session | Unlocks all tuning |
| **P1** | Bootstrap corpus (1000 fast + 500 slow games) | bootstrap.rs run | 2 days compute | Data for pre-screen |
| **P2** | SPSA for search shape | spsa_tune.rs | 1 session + compute | +10-30 Elo |
| **P3** | Relational features (pawn structure, rook lines, outposts) | queries.rs + audit | 2-3 sessions | +5-15 Elo each |
| **P4** | SPRT harness | sprt_harness.rs | 1 session | Statistical rigor |
| **P5** | King-safety table (non-linear) | queries.rs + table | 1 session | +10-20 Elo |
| **P6** | NNUE training (Phase 7) | nnue/ | 5+ sessions | +50-100 Elo |

**The key insight:** The eval-feature wall is not a feature-capacity problem — it is a **data-quality problem**. Fix the drawishness -> get decisive games -> Texel/SPSA/self-play all work -> features can be validated human-free.

**The honest fallback:** If fixing drawishness does not work (e.g., the eval is too weak to exploit temperature), then **some human games are unavoidable** as an external anchor. But the minimum viable set is small: ~50 decisive human games (not 16 drawish ones) would be enough to bootstrap the rest.

---

## References

- Oesterlund 2014, *Texel Tuning Method*: https://www.chessprogramming.org/Texel's_Tuning_Method
- Stockfish Fishtest wiki (SPRT/SPSA): https://github.com/official-stockfish/fishtest/wiki
- Spall 1992, *SPSA*: https://www.jhuapl.edu/SPSA/
- Coulom 2011, *CLOP*: https://www.remi-coulom.fr/CLOP/
- Silver et al. 2017, *AlphaZero*: https://arxiv.org/abs/1712.01815
- Sturtevant & Korf 2000, *Max^n pruning*: https://cdn.aaai.org/AAAI/2000/AAAI00-031.pdf
- Hornet experiments: EXP-009 (Texel), EXP-010 (self-play), EXP-013 (bootstrap), EXP-015 (move-agreement), EXP-014 (PST), PITCH-relational-eval-terms.md, PITCH-selective-intent-scaling.md
