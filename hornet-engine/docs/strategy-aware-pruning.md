# Strategy-Aware Pruning for Max^n Search

## Problem

At depth 4, the engine gets 0/13 tactical matches against human 3000+ Elo games. Depth 8 is intractable (~145s per position at beam=30). The current beam pruning is dumb: it keeps the top-N moves by generic move ordering, regardless of whose turn it is or what that player is trying to do.

When Red (root) evaluates a candidate move, the search must explore Blue's replies. But Blue has 40+ legal moves. Most of them don't matter to Red. Red only cares about Blue moves that:
1. Hurt Red directly (capture Red pieces, check Red king)
2. Help Blue win (farm points, eliminate others — making Blue stronger relative to Red)

Current code prunes Blue's moves based on Blue's own score (Blue maximizes Blue). This is wrong. Red should prune based on "how much does this Blue move affect Red's outcome?"

## Core Insight: Strategy Emerges from L1-L3

Strategy is not a separate layer. It emerges from the interaction of:

- **L3 intent tensors**: per-piece, per-opponent (offense, defense, vulnerability)
- **L2 zone control**: regional control and pressure
- **L1 eval**: global material/positional/safety/crossfire balance

From L3, we can read a player's current strategy:
- Concentrated offense vs one opponent → "attack that player"
- High defense, scattered offense → "turtle and wait"
- High offense vs everyone, low vulnerability → "aggressive expansion"
- High vulnerability, concentrated defense → "defensive scramble"

## Proposed Solution: Dynamic Strategy-Aware Beam

### Step 1: Infer Opponent Strategy from L3

At each opponent node, compute a lightweight L3 intent map (or cache it). From the opponent's intent, extract:

```
opponent_strategy(opponent, root_player) -> StrategyProfile {
    primary_target: which player opponent attacks most (from L3 offense vector)
    aggression_level: total offense / total defense ratio
    vulnerability_level: total vulnerability / piece count
    phase_inferred: "buildup" | "attack" | "cleanup" | "defensive"
}
```

### Step 2: Score Opponent Moves from Root's Perspective

For each opponent move, compute a "root-relevance score":

```
root_relevance(move) = 
    α * damage_to_root(move) +           // captures root pieces, checks root king
    β * opponent_benefit(move) * threat_weight // how much does this help opponent win
```

Where `threat_weight` depends on opponent's inferred strategy:
- If opponent's primary target is root → threat_weight = 1.0 (their moves directly hurt us)
- If opponent's primary target is someone else → threat_weight = 0.3 (indirect threat)
- If opponent is in "cleanup" phase → threat_weight = 0.5 for all (they're getting stronger)

### Step 3: Beam-Prune by Root Relevance

Sort opponent moves by `root_relevance` (not by opponent's own score). Beam-keep only the top moves that score high on root relevance. Prune moves that:
- Don't capture root pieces
- Don't significantly help opponent
- Don't create threats against root

This is NOT the same as "best moves for opponent." It's "moves that most affect root's outcome."

## Implementation Plan

### Phase 1: Lightweight L3 at Search Nodes
- Add `compute_intent_map` call inside `maxn` (or cache it)
- Extract `primary_target` and `aggression_level` per player
- Cost: ~400µs per node (same as eval). Mitigation: only compute at ply 1, 4, 8... (rotation boundaries), not every node.

### Phase 2: Root-Relevance Move Scoring
- After generating legal moves at opponent node, score each move by:
  - Delta in root's material (captures)
  - Delta in root's safety (checks, threats)
  - Delta in opponent's FFA points (indirect threat)
- Sort by this score, then apply beam

### Phase 3: Adaptive Beam Width by Strategy
- If opponent's primary target is root → wider beam (their replies matter more)
- If opponent's primary target is someone else → narrower beam (prune more)
- If opponent is in "cleanup" phase → medium beam (they're dangerous to everyone)

## Open Questions

1. **Performance**: L3 computation is expensive. Can we cache intent maps across the search tree? Can we approximate strategy from cheaper queries?

2. **Balance (α, β)**: How much to weight direct damage vs indirect opponent benefit? This is the "farming vs gatekeeping" balance the user described.

3. **Stability**: Strategy inference might oscillate move-to-move. Should we smooth it across plies?

4. **Interaction with existing move ordering**: Current move ordering (MVV-LVA + FFA bounty + killers + history) is already decent. Should root-relevance scoring REPLACE it at opponent nodes, or COMBINE with it?

## Expected Outcome

With strategy-aware pruning, depth 8 should become feasible because:
- At opponent nodes where opponent is not targeting root, beam can be very narrow (prune 80%+ of moves)
- At opponent nodes where opponent IS targeting root, beam stays wide but only for relevant moves
- Net effect: fewer nodes searched, but the kept nodes are the ones that actually matter to root's decision

Target: depth 8 in <30s per position, which would allow full strength gate (13 samples) in ~6 minutes.
