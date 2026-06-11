# Claude's Framework: Three Senses of "Isolated" + How They Land in Hornet

**Date:** 2026-06-02  
**Re:** Claude's response to Kimi's hierarchical eval analysis  
**Author:** Claude (Anthropic) — captured by Kimi

---

## The Three Senses of "Isolated"

### (a) Isolated for Examination — "What does this layer encode?"

**Cheapest. Works on any architecture.**

- **Linear probes** (Alain & Bengio): freeze network, attach tiny linear readout to layer activations, decode what it represents (material? king danger? unit cohesion?). Read-only; changes nothing.
- **Named readouts per concept.** Hornet already does this — the four queries are exactly parallel named readouts you can inspect one at a time. A "unit" tier could emit its own named readout the same way.

**Verdict: basically free.** You never lose this.

---

### (b) Isolated for Learning — "Can this layer train on its own signal?"

**Costs a per-tier target, but healthy.**

- **Deep supervision / auxiliary heads:** each tier gets its own training target, not just gradient trickling from the top. Group tier predicts something about groups; player tier predicts something about the player. Each learns against its own loss — independently trainable, independently wrong-or-right.
- **Staged / greedy training:** train tier 1, freeze it, train tier 2 on its outputs. Each tier learns in isolation.

**Verdict: doable.** The cost is defining a target for each tier — which is healthy, because a tier with no nameable target is a tier you can't justify.

---

### (c) Isolated for Attribution — "What did this layer contribute to the final number?"

**The hard one. This is what the anti-pattern doc actually cares about.**

**The honest constraint:** you cannot have deep + freely-multiplicative feeding + clean attribution at the same time. If layer N non-linearly transforms layer N−1's blob, a probe tells you what N encodes but not what it contributes — everything above re-mixes it. That's why TECHNIQUES-and-REFERENCES.md went flat.

**The escape: make interfaces contracts instead of blobs.**

| Escape | How | Cost |
|--------|-----|------|
| **Additive composition** (residual stream) | Each tier adds to a running score: `total = base + Σ tier_contributions`. Leave-one-out gives each tier's exact marginal. | Architecture constraint: must be additive |
| **Named bottlenecks** | Force each tier's output through a small, fixed, human-meaningful vector. Unit tier emits `[cohesion, mobility, exposure]`. Next tier consumes named slots — isolate by perturbing named slots. | Design constraint: every seam must be inspectable |

**Neural Additive Models** are this idea made concrete: every input group's contribution is its own little curve you can read off. NNUE's accumulator is already additive over input features — that's precisely why per-feature NNUE stays somewhat interpretable.

---

## How This Lands in Hornet

### Hand-Eval Side (Now)

**Keep V's four components flat** (Hard Rule #4 — no 5th component). But a single component's **internal computation** can be a shallow named hierarchy (sub-readouts that sum).

```
Oᵢ (crossfire) = Σ piece_penalties
    where piece_penalty = f(enemy_count, enemy_value, piece_value)
    
    Could become:
    Oᵢ = Σ group_penalties + Σ isolated_piece_penalties
    where group_penalty = f(group_enemy_count, group_enemy_value, group_value)
```

**Additive inside, flat outside → still ablatable.**

### NNUE Side (Later, Item #7)

Real depth belongs here, made isolable three ways at once:

1. **Probe hidden layers** (examination)
2. **Supervise auxiliary heads on query targets** (learning)
3. **Structure first layer as additive over feature groups** (attribution)

That's a hierarchy you can take apart.

### Methodology Already in Place

The **ablation-study discipline** — ship default-off, measure alone and in-stack — is the runtime test that keeps all of the above honest.

- Probes/heads/additivity = architectural enablers
- Ablation arm = the proof

---

## The One-Line Version

> Examination is always free (probes). Learning-in-isolation costs a per-tier target (auxiliary heads). Contribution-attribution costs additive or named-bottleneck interfaces. Pay those two prices and you get the local→global hierarchy without the un-ablatable chain.

That's the line between **"structured features the network composes"** and **"swarm pipeline."**

---

## Kimi's Assessment

**I accept this framework completely.** It resolves the tension between the user's hierarchical intuition and Hornet's flat-V constraint.

**Immediate action (hand-eval, now):**
- Group features as **internal sub-readouts within existing components**, not new components
- Example: `Oᵢ` (crossfire) could sum `individual_piece_penalties + group_penalties`
- Both are readouts from the same LineMap; both feed into the same V-component
- Ablation: toggle group_penalties on/off, measure crossfire component change

**Deferred action (NNUE, item #7):**
- Additive feature groups in NNUE input layer
- Auxiliary heads for each query target
- Linear probes on hidden layers

**What I need:**
1. User sign-off on "group features as internal sub-readouts within existing V-components" (not new components)
2. Or: user says "wait, I want a 5th component" → change order

Holding until directed.

---

*Captured from Claude's message. No code changes.*
