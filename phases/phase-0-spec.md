# Phase 0: Spec / Reference

## Commander's Intent

Keep `HORNET-BUILD-SPEC.md` the single, correct source of truth for what to build. The spec is
Reference: it changes only through deliberate, reviewed updates, and when it changes every
downstream phase file is checked for impact.

## Reading List (Start Here)

1. `STATUS.md`
2. `HORNET-BUILD-SPEC.md` (current canonical) + `HORNET-BUILD-SPEC-v0.2-DELTA.md` (pending patch)
3. Review thread, in date order: `REVIEW-claude-on-hornet-spec-2026-06-01.md` →
   `RESPONSE-kimi-to-claude-spec-review-2026-06-01.md` →
   `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`

## Write Scope

**Owns:** `HORNET-BUILD-SPEC.md`, `HORNET-BUILD-SPEC-v0.2-DELTA.md`, review-cycle artifacts.
**Read-only:** everything else.

## Current State

| Field | Value |
|-------|-------|
| Status | **v0.2 landed** (2026-06-01, by claude via CO-001) |
| Last Session | 2026-06-01 — v0.2 landed (10 delta items integrated, §10 added) |
| Blocking Issues | none |
| Next Action | none — future spec changes go through a new change order |

## Acceptance Checklist

- [x] All 10 delta items integrated at their named §-anchors in `HORNET-BUILD-SPEC.md`.
- [x] Header bumped to **v0.2**; delta file marked merged (kept as historical record).
- [x] eval vs FFA value systems present and never conflated (Hard Rule #8).
- [x] Downstream phase files checked for impact (P1 unblocked for PGN4; §10 = PGN4 contract).

## Active Watch Items

- CO-001 (open): requested by P1 — land v0.2. Coordinate with Kimi's in-flight release to avoid a
  double-edit of the spec.

## Rework Log

| Date | Requested By | What Changed | Why | Impact |
|------|-------------|-------------|-----|--------|
| | | | | |

## Downstream Notes

Every implementation phase (P1–P8) treats the landed spec as authoritative. The v0.2 deltas most
relevant downstream: value-system split (§1.7/§1.8 → P1 types, P5 eval), PGN4 ingestion
(§6.5/§9 → P1), underpromotion + `PromotedQueen` (§1.4 → P1, P2), castling tables (§1.5 → P2).
