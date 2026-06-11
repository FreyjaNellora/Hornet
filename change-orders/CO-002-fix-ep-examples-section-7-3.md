# Change Order CO-002

**Date:** 2026-06-02
**Requested By:** Phase 2 — Move Generation (Session 001, agent claude)
**Target Phase:** Phase 0 — Spec / Reference (owner: Kimi)
**Status:** resolved (landed 2026-06-06; header reconciled 2026-06-10 — user authorized closing the open CO backlog)

## What Needs Changing

The en-passant **examples in spec §7.3** (landed from delta item #4) place the capturing pawn on the
**wrong square** — on the EP target's own file/rank, implying a *straight* push rather than a
diagonal capture. All four examples have the same error. Example (Red-Blue):

> "Blue pawn at c4, pushes c4→e4 (East 2). EP target: d4. **Red pawn at d3 captures EP: d3→d4**…"

`d3→d4` is `(+1 rank, 0 file)` — a forward push, not a diagonal capture. Red can only capture onto
d4 from **c3** or **e3** (its NE/NW capture deltas, §1.4). The capturing pawn squares should be:

| Pair | EP target | Capturing pawn should be at (not as written) |
|------|-----------|----------------------------------------------|
| Red-Blue | d4 | **e3** (or c3) — written "d3" |
| Red-Green | l4 | **k3** (or m3) — written "l3" |
| Blue-Yellow | d13 | **c12** (or c14) — written "d12" |
| Yellow-Green | l11 | **k12** (or m12) — written "l12" |

(The Blue-Yellow / Yellow-Green push notation also looks axis-confused; reconcile against §1.4 when fixing.)

## Why

Found while implementing P2 en-passant generation + make/unmake. The **normative** rules (§1.4
movement/capture deltas, §1.6 EP orthogonality) are correct and the engine follows them; only the
**illustrative §7.3 examples** are wrong. Caught because the engine's EP test couldn't use the
spec's example geometry (it isn't a legal pawn capture).

## Impact Assessment

- [x] Cosmetic (documentation/examples — the normative rules are unaffected; no behavioral change)
- [ ] Structural
- [ ] Architectural

## Affected Phases

| Phase | Impact |
|-------|--------|
| P0 (target) | Fix the four §7.3 EP example squares. |
| P2 (requester) | None functionally — engine already follows §1.4/§1.6. Test uses corrected geometry. |

## Recommended Fix

Correct the capturing-pawn squares in the four §7.3 examples per the table above, and re-check the
push notation against §1.4 forward directions.

## Resolution

**Landed 2026-06-06** — `HORNET-BUILD-SPEC.md` §7.3 updated. Corrected capturing-pawn squares for all four EP examples (Red-Blue, Red-Green, Blue-Yellow, Yellow-Green). The normative rules in §1.4/§1.6 were always correct; only the illustrative examples were wrong.
