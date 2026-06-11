# Change Order CO-005

**Date:** 2026-06-10
**Requested By:** Review cycle 2026-06-10 (blind code review, agent Fable; verified by Kimi + Opus)
**Target Phase:** Phase 0 — Spec / Reference
**Status:** resolved (user approved 2026-06-10; landed by Fable)

## What Needs Changing

Two as-built divergences the spec still states normatively:

**1. Eval weights (§4.7 + Appendix).** Spec says v0 weights `W_MATERIAL=1, W_POSITIONAL=2,
W_SAFETY=1, W_CROSSFIRE=1`. Deployed (`eval.rs`) is **`(6, 0, 0, 1)`** — validated by EXP-015
move-agreement tuning and the scipy/bootstrap fit (positional: CI includes 0 → noise; safety:
significantly **negative** as built → off pending the rebuild). The spec's §4.7 also shows
`compute_utility` returning a raw weighted sum; the implementation is **mean-relative**
(`ΔXᵢ = Xᵢ − X̄`, giving Σᵢ Uᵢ ≈ 0 — see ENGINE-MATH §2), which §4.7 should state.

**2. Board struct (§2.5).** The spec specifies `piece_lists` (+sync invariant), `piece_counts`,
cached `king_squares` (255 = eliminated), and a packed `castling_rights: u8`. None of these were
built; the implemented `Board` (`board/mod.rs`) is the squares array + per-player bool arrays
(`dead`, `dkw`, `castle_kingside`, `castle_queenside`), `points`, the raw `extra` field, EP target +
pushing player, and the incremental zobrist. King lookup is a scan (`king_square()`), not a cache.
The spec's invariants ("squares and piece_lists always in sync") are invariants of a structure that
does not exist.

## Why

Same hazard as CO-004: the spec is the document new agents build against. An agent "restoring" the
spec'd piece lists or weights would be re-litigating settled decisions without knowing it. The
weights especially: `(1,2,1,1)` vs `(6,0,0,1)` is not a nuance — positional and safety are
currently *off* for measured reasons documented in EXP-015/EXP-018 and SYNTHESIS-next-attempt.

## Impact Assessment

- [ ] Cosmetic
- [x] Structural (normative data-structure + constants text replaced to match as-built; no engine change)
- [ ] Architectural

## Affected Phases

| Phase | Impact |
|-------|--------|
| P0 (target) | §2.5 rewritten to the as-built Board; §4.7/Appendix weights updated to `(6,0,0,1)` with pointers to EXP-015 and the safety-rebuild plan; mean-relative normalization stated. |
| P1–P6 | None — code already is the as-built reference. |

## Recommended Fix

- §2.5: document the implemented `Board` fields and the actual invariants (zobrist excluded from
  equality; incremental hash verified against recompute; `extra` preserved verbatim for byte-exact
  FEN4 round-trips). Note explicitly that piece lists / cached king squares are **not** maintained
  (Hard Rule #5's always-recompute philosophy extends here until a measured need exists).
- §4.7 / Appendix: deployed weights `(6,0,0,1)` + the mean-relative formula, with a one-line
  pointer: positional/safety are zero pending the safety rebuild + relational terms (see
  `REVIEW-claude-on-kimi-independent-plan.md`); do not re-tune by hand.

## Resolution

**Landed 2026-06-10** (user approval given in-session; Fable). `HORNET-BUILD-SPEC.md`:
- **§2.5** rewritten to the as-built `Board` (verified against `board/mod.rs`): squares array +
  per-player `dead`/`dkw`/castle bool arrays, `points`, verbatim `extra`, EP target + pushing
  player, incremental `zobrist`. Real invariants stated (zobrist excluded from equality, verified
  vs recompute, recompute required after direct-write self-syncs; `extra` byte-exact; `dkw`
  runtime-only). Explicit "deliberately not maintained" note for piece lists / counts / cached
  king squares / packed castling byte, with the Hard-Rule-#5 rationale.
- **§4.7** rewritten: mean-relative computation (`ΔXᵢ = Xᵢ − X̄`, Σ Uᵢ ≈ 0, the Sturtevant–Korf
  rationale, ENGINE-MATH §2 pointer) + deployed weights `(6,0,0,1)` with the EXP-015/EXP-009 basis,
  why positional/safety are zero, why material is 6 under mean-relativity, and the
  do-not-hand-retune warning.
- **Appendix** weight constants updated `(1,2,1,1)` → `(6,0,0,1)` with the same pointer.
