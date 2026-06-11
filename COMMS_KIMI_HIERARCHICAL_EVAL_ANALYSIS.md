# Hierarchical / Multi-Scale Eval Analysis

**Date:** 2026-06-02  
**Re:** User's proposal: per-piece → piece-group → player-strategy → global-board hierarchical vectors  
**Author:** Kimi (Moonshot AI) — analysis only, no code changes

---

## Restating the User's Idea

A layered evaluation where information flows **bottom-up**:

```
Layer 0: Per-piece features (individual unit state)
    ↓
Layer 1: Piece-group features (clusters, formations, local tactics)
    ↓
Layer 2: Player-strategy features (overall position, material, safety)
    ↓
Layer 3: Global board features (inter-player dynamics, alliances, tempo)
    ↓
Output: V = ⟨U₁, U₂, U₃, U₄⟩
```

Each layer feeds the next. The intuition: a pawn's value depends on whether it's part of a pawn chain (Layer 1), whether that chain supports the king (Layer 2), and whether the player is ahead in development (Layer 3).

---

## How This Maps to What Hornet Already Has

### Current Architecture (flat)

```
Board → LineMap (all pieces, all reaches) → Queries (4 aggregate scalars per player) → V
```

The queries are **already hierarchical** but implicit:
- `query_material` = Layer 0 → Layer 2 (skip group, just sum)
- `query_positional` = Layer 0 → Layer 2 (centrality-weighted sum of reaches)
- `query_king_safety` = Layer 0 → Layer 1 (king vicinity) → Layer 2 (safety scalar)
- `query_crossfire` = Layer 0 → Layer 1 (convergence on one piece) → Layer 2 (penalty)

### What's Missing: Explicit Layer 1 (Piece Groups)

Hornet has no concept of:
- **Pawn chains** (connected pawns defending each other)
- **Piece coordination** (knight + bishop attacking same square = fork threat)
- **Local force concentration** (3 pieces vs 1 in a sector)
- **Battery** (queen + rook on same file)

These are **tactical patterns** that the current eval misses because it only looks at aggregate statistics.

---

## The Hierarchical Proposal: Detailed

### Layer 0: Per-Piece Features (already in LineMap)

For each piece, we have:
- `PieceLines`: all squares it reaches, with distance, blocker, xray
- `SquareReachers`: who reaches this piece (inverse index)

**New:** Compute per-piece **local value**:
```
piece_value(p) = material_value + mobility_bonus + safety_penalty + threat_potential
```

Where:
- `mobility_bonus` = count of empty squares reached × centrality
- `safety_penalty` = attacked_by_enemy ? −(attacker_value / defender_count) : 0
- `threat_potential` = value of highest-value enemy piece attacked

### Layer 1: Piece-Group Features (new)

**Groups** are discovered dynamically:

| Group Type | Detection | Example |
|------------|-----------|---------|
| Pawn chain | Pawns on adjacent files, defending each other | d4-e4-f4 |
| Battery | Two sliders on same ray | Queen + rook on e-file |
| Cluster | Pieces within radius-2 of each other | King + 3 defenders |
| Outpost | Piece on square enemy pawns can't attack | Knight on d6 |

**Per-group features:**
```
group_strength = sum(piece_values in group) + coordination_bonus
group_vulnerability = sum(enemy attacks on group members) − group_defenders
group_mobility = count of squares the group can reach together
```

**Coordination bonus:** When two pieces attack the same square, that's a threat. When they defend each other, that's resilience.

### Layer 2: Player-Strategy Features (current queries, enhanced)

Current queries become **weighted by group quality**:

```
Mᵢ = sum of material (unchanged)
Pᵢ = sum of group_mobility × centrality (not just individual mobility)
Sᵢ = king safety + group_vulnerability of king's cluster
Oᵢ = crossfire on groups (not just individual pieces)
```

### Layer 3: Global Board Features (new)

**Inter-player dynamics:**
- **Tempo:** Who's making threats? Count of forcing moves per player.
- **Alliance proxy:** Two players attacking the same third = implicit alliance.
- **Material imbalance:** Is one player way ahead? Others should ally against them.
- **Space control:** Which quadrant of the board does each player dominate?

---

## The Pruning Connection

Here's why this matters for shallow pruning:

**Current problem:** `Uᵢ = Mᵢ + 2Pᵢ + Sᵢ − Oᵢ` is not bounded because:
- `Pᵢ` scales with piece count × reach (unbounded-ish)
- `Sᵢ` can go arbitrarily negative
- `Oᵢ` can go arbitrarily high

**With hierarchical eval, each layer has natural bounds:**

| Layer | Bound | Why |
|-------|-------|-----|
| Layer 0 (per-piece) | `[−max_capture, +material]` | A piece can at best capture a queen, at worst be captured |
| Layer 1 (group) | `[−sum_group, +sum_group + coordination]` | Group can't be worth more than its pieces + synergy |
| Layer 2 (player) | `[−total_material, +total_material]` | Player can't gain more than all enemy material |
| Layer 3 (global) | Constant-sum by construction | ΣUᵢ = 0 (or fixed constant) |

**If Layer 3 is constant-sum, shallow pruning is sound.**

---

## Three Ways to Implement This

### Option A: Hierarchical Queries (incremental)

Keep the current query structure but add **group queries** between LineMap and player queries:

```rust
// New: Layer 1
pub struct GroupMap {
    pub groups: Vec<PieceGroup>,  // discovered formations
}

pub fn discover_groups(lines: &LineMap, board: &Board) -> GroupMap;

// Enhanced: Layer 2 (player queries now consume GroupMap)
pub fn query_material(board: &Board, groups: &GroupMap) -> [i16; 4];
pub fn query_positional(lines: &LineMap, groups: &GroupMap) -> [i16; 4];
// etc.
```

**Pros:** Minimal disruption to existing code. Group discovery is additive.
**Cons:** Still not constant-sum unless we redesign the final utility computation.

### Option B: Neural Hierarchical Eval (NNUE v2)

Instead of hand-tuned queries, train a neural net with **hierarchical attention**:

```
Input: Piece embeddings (Layer 0)
    ↓
Transformer layer 1: Piece-to-piece attention (discovers groups, Layer 1)
    ↓
Transformer layer 2: Group-to-player pooling (player state, Layer 2)
    ↓
Transformer layer 3: Player-to-board pooling (global state, Layer 3)
    ↓
Output: V = ⟨U₁, U₂, U₃, U₄⟩
```

**Pros:** The network learns group concepts automatically. Can be constant-sum by output design.
**Cons:** Requires training data, infrastructure, time. Not a quick fix.

### Option C: Constant-Sum Wrapper (quick fix)

Keep the current eval, but **normalize at the output layer**:

```rust
fn eval_4vec(board: &Board, lines: &mut LineMap) -> [i16; 4] {
    let qv = run_all_queries(lines, board);
    let raw = compute_utility(&qv);  // current: Uᵢ = Mᵢ + 2Pᵢ + Sᵢ − Oᵢ
    
    // New: constant-sum wrapper
    let sum: i32 = raw.iter().map(|&x| x as i32).sum();
    let avg = sum / 4;
    let mut v = [0i16; 4];
    for i in 0..4 {
        v[i] = (raw[i] as i32 - avg) as i16;  // zero-mean: Σvᵢ = 0
    }
    v
}
```

**Pros:** One-line change. Makes shallow pruning sound immediately.
**Cons:** Changes the eval semantics — now relative, not absolute. A position where all players are equal scores 0 for everyone. May confuse search.

---

## My Assessment

| Criterion | Option A | Option B | Option C |
|-----------|----------|----------|----------|
| Implementation effort | 2-3 sessions | 10+ sessions | 30 minutes |
| Pruning enablement | Partial (tighter bounds) | Full (learned constant-sum) | Full (forced constant-sum) |
| Eval quality improvement | Moderate | High | None (just normalized) |
| Risk to existing tests | Low | High | Medium (changes scores) |
| Alignment with user's vision | ✅✅ | ✅✅✅ | ✅ |

**My recommendation:**

1. **Short-term (now):** Option C (constant-sum wrapper) + test if shallow pruning fires. If it does, we have a win. If not, we learned something.

2. **Medium-term (after strength gate):** Option A (hierarchical queries) — add group detection to improve eval quality. This also tightens bounds naturally.

3. **Long-term (NNUE phase):** Option B (neural hierarchical eval) — the user's vision realized in the network architecture.

---

## What Claude Should Know

The user's vision is **not about pruning** — it's about **eval architecture**. The hierarchical design is how humans think about chess:
- "My knight is pinned" (Layer 0)
- "My kingside is under attack" (Layer 1)
- "I'm behind in development" (Layer 2)
- "I need to ally with Blue against Yellow" (Layer 3)

If we build this, pruning becomes a **side benefit** (constant-sum at Layer 3), not the main goal.

---

## Questions for User and Claude

1. **Is the priority pruning or eval quality?** If pruning, Option C now. If eval quality, Option A.
2. **Should Layer 3 be strictly constant-sum (ΣUᵢ = 0) or just bounded?** Constant-sum enables shallow pruning; bounded enables weaker pruning.
3. **How many group types to start with?** Pawn chains + batteries + clusters = 3. More = better eval, more computation.
4. **Does this replace the current V-decomposition or augment it?** Hard Rule #4 says V is fixed. Hierarchical features would feed INTO M/P/S/O, not replace them.

---

— Kimi
