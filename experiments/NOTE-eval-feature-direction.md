# NOTE — eval-feature direction (the next eval gains)

- **Date:** 2026-06-06
- **Context:** EXP-009 showed the eval weights are optimal, so gains are in the *features*. This note
  records where the features should go and the concerns raised (cross-agent discussion + adjudication).

## The framing (correct)
Of V's four components, **Pᵢ (positional) is the only one without a piece-level base**:
- Mᵢ — material (degenerate base; predictive).
- Sᵢ — king-safety, reads king intent (centipawn danger; wired in recalibration).
- Oᵢ — crossfire, reads SEE attacker/defender (material-at-risk; wired in recalibration).
- **Pᵢ — flat centrality-mobility, no tactical substrate** → Texel finds it signal-thin.

So the concrete next eval gain is completing Pᵢ's base. (It's an *aggregation/fold*, not literal
recursion — but the asymmetry observation is what matters.)

## Pᵢ substrate candidates — and which actually moves the metric
1. **Mobility** (restructure `query_positional_control`) — **cleanup only.** Same reach×centrality
   geometry Texel already found thin → re-skinning it gives ~0 MSE delta.
2. **Threats** — **already** folded into Pᵢ (`query_threats`); the SEE variant was measured **null**
   (EXP-002, re-confirmed EXP-008). Not the adder.
3. **Pawn structure** (isolated / doubled / passed — new, bounded) — **the genuine signal-adder
   candidate.** Classically predictive. **Build this first.**

## Concern flagged (and resolved)
A cross-agent proposal was to wire `intent.rs` as Pᵢ's substrate (`Pᵢ = Σ directional_reach`).
**Wrong substrate:** `intent.rs` (as implemented) is offense/defense/vulnerability per opponent — a
**threat** substrate with **no mobility/directional-reach field**. So it can supplement **Oᵢ/threats**
(distance/value-weighted), **not** Pᵢ-mobility. (The intent *pitch* listed mobility masks; the *code*
doesn't have them — reason from the code.)

## Method (fixed)
Every new feature: **default-off ablation arm** (Hard Rule #6) + accept only on a **`texel_tune` MSE
drop** (EXP-009 infra). 4PC caveat: pawn structure may carry less than in 2-player (central-crossing
promotion, more tactical FFA) — the Texel delta judges, not intuition. Eval-lane work; logged in
ENGINE-HANDOFF "What's left" #4.
