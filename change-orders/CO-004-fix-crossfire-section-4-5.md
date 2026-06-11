# Change Order CO-004

**Date:** 2026-06-10
**Requested By:** Review cycle 2026-06-10 (blind code review, agent Fable; verified by Kimi + Opus)
**Target Phase:** Phase 0 — Spec / Reference
**Status:** resolved (user approved 2026-06-10; landed by Fable)

## What Needs Changing

Spec **§4.5 (Crossfire Query)** still specifies the **pre-EXP-005** formulation:

> `penalty += enemy_value * enemy_count + piece_value(pl.piece_type)` (for `enemy_count >= 2`)

This is the exact `value × count` scale bug (≈ value² units) that EXP-005→008 diagnosed and
removed — it swung the eval by thousands per quiet move and drowned material. The implemented
crossfire (`queries.rs::query_crossfire`) is **SEE-resolved material-at-risk**: per-attacking-player
two-sided SEE on each non-king piece (third parties excluded — no one recaptures to save another
player's piece), sum of positive SEE threats, bounded by the victim's value. The king is excluded
(its capture is terminal; handled by search — this exclusion is also what keeps crossfire
complementary to the objective-layer king-danger term, ENGINE-MATH §7.5).

## Why

A fresh agent building or reviewing against §4.5 as written would reintroduce a bug the project
already paid to find and kill. In a multi-agent setup with `agent-conduct.md` §1.5 naming the spec
as the authoritative reference, stale normative text is a regression vector, not a cosmetic issue.

## Impact Assessment

- [ ] Cosmetic
- [x] Structural (normative query definition replaced; no engine change — the engine is already correct)
- [ ] Architectural

## Affected Phases

| Phase | Impact |
|-------|--------|
| P0 (target) | Rewrite §4.5 to the SEE material-at-risk definition (mirror `queries.rs::query_crossfire` semantics: direct attackers only via `reaches_directly`, per-attacker SEE, positive threats summed, capped at victim value, king excluded). |
| P4/P5 | None — code already implements the corrected definition. |

## Recommended Fix

Replace §4.5's pseudocode with the as-built definition and add one line of history: "v0.1's
`enemy_value × enemy_count` formulation was a scale bug (EXP-005); do not reintroduce."

## Resolution

**Landed 2026-06-10** (user approval given in-session; Fable). `HORNET-BUILD-SPEC.md` §4.5
rewritten to the SEE material-at-risk definition, mirroring `queries.rs::query_crossfire` exactly:
direct attackers only (`reaches_directly` — sliders count only when the target is their first
blocker), per-attacking-player two-sided SEE against the owner's defenders (third parties never
enter), positive SEE threats summed, capped at the victim's value, king excluded (terminal —
search's territory; keeps crossfire complementary to the objective-layer king-danger term). The
history line warns against reintroducing `enemy_value × enemy_count`.
