# Change Order CO-006

**Date:** 2026-06-10
**Requested By:** Review cycle 2026-06-10, worksplit item B4 (drafted by Fable; measurement basis EXP-020)
**Target Phase:** Meta — project rules (`PITCH-for-new-agents.md` Hard Rule #6, transcribed in `agent-conduct.md` §1.2)
**Status:** resolved (user approved 2026-06-10; landed by Fable)

## What Needs Changing

Hard Rule #6 ("Additive discipline. Every new lever ships **default-off** with an **ablation
arm**.") reads, and has been applied, as an *eval-feature* rule. The move-ordering flags that
shipped `const = true` (found in the 2026-06-09 blind review) were a literal violation, but the
gap is broader: nothing in the rule's text names the **search-shape levers** as strength-affecting.

**Amended text (proposed):**

> **6. Additive discipline — anything that changes the played move.** Every lever that can change
> the move the engine plays — eval features, **move ordering, beam width/shape, LMR, killer/history
> heuristics, TT best-move-hint usage** — ships **default-off** with a **measured ablation arm**
> (self-play A/B or an equivalent recorded measurement), the same gate as eval changes. No silent
> or unmeasured strength-affecting changes.

## Why

No longer just preventative — **measured**: EXP-020 showed a single ordering heuristic (the
inverted free-capture bonus) changed the played move on **11.6% of corpus positions at beam 4**
(0.9% at beam 10, 0.6% at beam 30). In a beam search, ordering *is* selection: the top-k ordered
moves are the only ones expanded, so any ordering lever is a strength lever at narrow beams. The
2026-06-10 cost of this gap: a tainted 133-game bootstrap corpus (regeneration = B5) and a
re-baselining of every recorded maxn move-agreement number.

## Audit corollary (going forward, no current bug found)

Killers, history, and the TT best-move hint already influence ordering — and therefore selection —
in narrow-beam configs. They predate the rule and stay (they are baseline ordering, measured
indirectly through every recorded number), but **future changes to them** fall under the amended
gate: a re-tune of killer slots, history decay, or TT-hint priority is a measured-arm change, not
a refactor.

## Impact Assessment

- [ ] Cosmetic
- [x] Structural (rule text + scope; no engine change — current tree already complies after EXP-020/021)
- [ ] Architectural

## Affected Phases

| Phase | Impact |
|-------|--------|
| Meta (target) | Amend Hard Rule #6 text in `PITCH-for-new-agents.md`; sync the §1.2 transcription in `agent-conduct.md`. |
| P6 (search) | Already compliant: ordering flags are default-off `OrderState` fields (EXP-020); LMR/adaptive-beam/quiescence were already default-off levers. |
| P5 (eval) | No change — eval features were always under the gate. |

## Recommended Fix

Land the amended text verbatim (both files), with a one-line pointer to EXP-020 as the measured
basis. No code change required.

## Resolution

**Landed 2026-06-10** (user approval given in-session; Fable). Amended Hard Rule #6 text landed in
`PITCH-for-new-agents.md` (rule 6, with the EXP-020 measured basis and the killers/history/TT-hint
corollary) and synced to the `agent-conduct.md` §1.2 transcription. No code change — the tree
already complies (EXP-020/021 landed the flags default-off; LMR/adaptive-beam/quiescence were
already default-off levers).
