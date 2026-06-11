# Pitch — Selective Intent: L1/L2/L3 Scaling for 4PC Coordination

**Status:** Draft · **Author:** Kimi · **Date:** 2026-06-06

## The Problem

`intent.rs` computes a full L3 per-piece tensor (offense/defense/vulnerability vs each opponent). It was reverted for 5× slowdown. The 5× came from:
- `build_attack_index`: 196 `Vec` allocations per eval call
- Defense computation: O(n²) loop over all pieces × all pieces
- Full tensor materialization: 128 pieces × 3 opponents × 3 dimensions

Most pieces in most positions are tactically inert. Computing intent for a pawn on its starting square that attacks nothing wastes 90%+ of the work.

## The Insight: Scale by Layer

The engine already has three conceptual layers. The fix is to apply selective intent **at each layer respective to what that layer does best**:

| Layer | Scope | What it does best | Current state | Selective intent application |
|---|---|---|---|---|
| **L1** | Global position | Material sum, mean-relative normalization | Working (Mᵢ) | **Not needed** — L1 is aggregate, not tactical |
| **L2** | Regional zones | Territory control, spatial clustering | Dormant (`zones.rs`) | **Zone-targeted intent** — hot zones with concentrated material |
| **L3** | Per-piece | Piece-level offense/defense/vulnerability | Dormant (`intent.rs`) | **Selective piece intent** — only contested pieces |

## L1: Global — No Change

L1 is the eval vector V = ⟨U₁,U₂,U₃,U₄⟩. It aggregates. It doesn't need tactical substrate — it needs calibrated, zero-sum components. The recalibration fixed this. Leave L1 alone.

## L2: Regional — Zone-Targeted Intent (New)

`zones.rs` defines nine 2×2 zones (Center, 4 Gates, 4 Quadrants). It computes zone control but is **dormant** — never fed into eval.

**What L2 does best:** Spatial clustering. A zone with 3 enemy pieces and 2 friendly pieces is a **regional swarm opportunity** — the eval should know this without checking every square individually.

**Selective intent at L2:**
```
For each zone:
  - Count enemy material in/around the zone
  - Count friendly attackers that reach the zone
  - If enemy_value > threshold AND friendly_attackers > 1:
    - Score = enemy_value × friendly_attackers × centrality_bonus
    - Add to Pᵢ for each friendly attacker player
```

This is "regional target richness" — the zone tells the engine "there's concentrated enemy material here, and we have multiple pieces pointing at it."

**Why L2:** Zones are pre-defined, small (9 zones), cheap to compute. No per-piece iteration. No Vec allocations. The `compute_zone_control` in `zones.rs` already does the geometric work.

**Test:** Wire `aggregate_zone_control` into Pᵢ with a small weight. Texel-gate. If MSE drops, L2 intent carries signal.

## L3: Per-Piece — Selective Piece Intent (New)

`intent.rs` has full per-piece tensors but is **dormant** — never called from eval.

**What L3 does best:** Piece-level tactical truth. A knight forking two queens is a **piece-level coordination event** — the piece itself is the star of the situation.

**Selective intent at L3:**
```
Identify "contested pieces" (cheap, from LineMap):
  - Pieces with ≥2 enemy reachers (under concentrated attack)
  - Pieces with ≥2 friendly reachers on enemy targets (forking/multi-attacking)
  - Pieces whose king is in check or near-check

For each contested piece:
  - Compute offense intent: what enemies does this piece threaten?
    - Score = target_value × attacker_quality × turn_proximity
  - Compute vulnerability intent: who threatens this piece?
    - Score = own_value × threat_severity × turn_proximity
  - Skip defense intent (O(n²), already in crossfire)

Aggregate into Pᵢ (offense) and Oᵢ (vulnerability)
```

**Why selective:** In quiet positions, 5-10% of pieces are contested. In tactical positions, maybe 20-30%. Never 100%.

**Test:** Implement `query_selective_intent(lines, board) -> [i16; 4]`. Default-off. Texel-gate. Performance-gate (< 600 µs).

## How the Layers Interact

```
Position (L1)
  └── Zone "Center" (L2) — 3 enemy pieces, 2 friendly attackers
        └── Piece "Red Knight f6" (L3) — attacks Blue Queen d7 AND Yellow Rook h5
              └── L3 intent: offense vs Blue = 900×1.5, offense vs Yellow = 500×1.0
        └── Piece "Red Bishop e5" (L3) — attacks same Blue Queen d7
              └── L3 intent: offense vs Blue = 900×1.2
        └── L2 aggregate: Center zone target richness = (900+500) × 2 attackers × centrality
  └── Zone "GateW" (L2) — quiet, no concentrated material
        └── L2: no intent computed (below threshold)
```

L2 says "Center is hot." L3 says "Knight f6 and Bishop e5 are the attackers." L1 says "Red's positional score increases."

## Implementation Order (Test Each — One Sub-Term Per Texel Run)

**Critical rule:** One new sub-term per Texel run. Once several sub-terms feed Pᵢ, Texel tunes one weight and can't tell you which helped. Each phase is independent; ablate what doesn't drop MSE.

**Phase A: Pawn structure (already landed, verify)**
- Current: isolated + doubled penalties in Pᵢ
- Texel-gate: does it drop MSE vs baseline? (Current: marginal 0.07% drop)
- If no: ablate pawn structure before adding more Pᵢ sub-terms

**Phase B: L2 zone intent (cheapest, broadest)**
1. Wire `aggregate_zone_control` into Pᵢ with weight W_ZONE (default 0 = ablated)
2. Texel-gate: does zone intent drop MSE?
3. If yes: tune W_ZONE. If no: ablate, move to Phase C.

**Phase C: L3 selective offense → Pᵢ (sharpest, most expensive)**
1. Implement `query_selective_intent` — contested pieces only, **offense only**
2. Wire into Pᵢ only (not Oᵢ — crossfire already handles vulnerability)
3. Default-off with `HORNET_SELECTIVE_INTENT=1`
4. Texel-gate + performance-gate
5. If offense alone moves MSE, consider adding L3 vulnerability **only the turn-proximity nuance** (not raw threat count — crossfire has that)

**Phase D: L2 + L3 combined**
1. If both L2 and L3 pass individually, test together
2. Risk: overlapping signal — ablate if MSE doesn't drop further

**Phase E: Turn-proximity prototype (standalone)**
1. Even if L3 selective doesn't pass, the turn-proximity insight (attack next-to-move player = more valuable) is worth testing in isolation
2. Add `turn_proximity_weight` as a multiplicative bonus to existing SEE threats
3. Texel-gate: does it drop MSE?
4. Cheap: reuses existing `query_threats_see`, just adds proximity multiplier

## Why This Scales

| | Full Intent (reverted) | L2 Zone Intent | L3 Selective Intent |
|---|---|---|---|
| Scope | All pieces, all squares | 9 zones only | Contested pieces only |
| Cost | O(piece_count²) | O(9 × reachers_per_zone) | O(contested_pieces × reachers) |
| Typical work | 64 pieces × 3 ops = 192 tensors | 9 zones × ~20 reachers = 180 lookups | 5-15 pieces × ~10 reachers = 50-150 lookups |
| Allocations | 196 Vecs | 0 (fixed arrays) | 0 (fixed arrays) |
| Signal type | Broad, diluted | Regional concentration | Specific tactical events |

## What It Gives the Eval

The coordination signal:
- **"Gang up on high-value targets"** (L2: zone sees concentrated material; L3: pieces see swarm opportunity)
- **"Swarm from advantageous positions"** (L3: attacker quality × turn proximity = optimal timing)
- **"Coordinate pieces"** (L2: multiple friendly reachers in zone = coordination; L3: multi-attacker bonus)
- **"Target the weakest player"** (L2: zone with one player's concentrated material = vulnerability cluster)

Current eval has none of this. `query_threats_see` is per-piece, per-target, 2-sided. It doesn't know that two of your pieces attacking the same queen is better than one. It doesn't know that attacking the next-to-move player's queen is more valuable.

## Constraints

- Hard Rule #4: V stays M/P/S/O. L2/L3 intent feeds into Pᵢ and Oᵢ, not new components.
- Hard Rule #5: Always-recompute. No caching.
- Default-off + ablation per layer. Test L2 and L3 independently.
- Engine-only.

## Landmines

- **L3 vulnerability → Oᵢ double-counts crossfire.** Oᵢ already has SEE material-at-risk. Adding L3 vulnerability there counts the same threat twice. Do L3 offense → Pᵢ first; add vulnerability only if offense alone moves MSE, and only the turn-proximity nuance crossfire can't express.
- **L2 without L3 is coarse.** A zone says "Center is hot" but not which piece exploits it. L2 alone may not move MSE enough.
- **L3 without L2 is myopic.** A piece sees "I attack the queen" but misses that the queen is in a zone with 2 other enemy pieces (bigger swarm opportunity).
- **Don't recompute defense at L3.** Crossfire already handles vulnerability. L3 vulnerability should be additive nuance (turn proximity), not replacement.
- **Turn proximity is dynamic.** Weight depends on `board.side_to_move`. Recompute per eval call.
- **Overlapping signal.** L2 zone bonus + L3 multi-attacker bonus may double-count the same configuration. Test independently first.
- **One sub-term per Texel run.** Once several sub-terms feed Pᵢ, Texel tunes one weight and can't tell you which helped. Add features one at a time, gate each.

## Verify

```bash
cd hornet-engine
cargo test --lib                          # keep green

# Phase A: L2 zone intent
cargo run --release --example texel_tune  # MSE must drop vs 0.11445

# Phase B: L3 selective intent
cargo run --release --example texel_tune  # MSE must drop

# Phase C: combined
cargo run --release --example texel_tune  # MSE must drop further

# Performance gate throughout
cargo test --lib eval::tests::eval_performance_debug_mode  # < 600 µs
```

If a phase doesn't drop MSE, ablate it. The layers are independent.
