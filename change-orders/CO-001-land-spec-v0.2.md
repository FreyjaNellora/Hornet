# Change Order CO-001

**Date:** 2026-06-01
**Requested By:** Phase 1 — Board I/O (Session 001, agent claude)
**Target Phase:** Phase 0 — Spec / Reference (owner: Kimi)
**Status:** resolved (2026-06-01 — dispatch authorized claude; v0.2 landed)

## What Needs Changing

Land spec **v0.2**: integrate the 10 items in `HORNET-BUILD-SPEC-v0.2-DELTA.md` into
`HORNET-BUILD-SPEC.md` at their named section anchors, and bump the spec header from v0.1 to v0.2.
Keep the delta file as the historical patch record (mark it merged).

## Why

The spec is the Reference that every implementation phase depends on. It currently exists as v0.1 +
a separate, content-complete delta — a split source of truth. The delta itself concludes
"Implementation can begin," but P1's PGN4 work (Hard Rule #2: native PGN4 ingestion) is defined only
in the delta (§3 → §6.5/§9), so building against an un-landed spec would mean coding against a
moving target. Landing v0.2 removes the only gate on the implementation phases.

Evidence the delta is ready: the review→response→verification thread
(`REVIEW-…`, `RESPONSE-…`, `VERIFICATION-…`, all 2026-06-01) resolved every [BLOCKER] and [VERIFY]
item with chess.com ground truth; the delta's own closeout marks all blockers ✅.

## Impact Assessment

- [ ] Cosmetic
- [ ] Structural
- [x] Architectural (the spec is Reference; touching it can cascade to all phases — but here the
      changes are already-accepted clarifications/additions, not new design)

## Affected Phases

| Phase | Impact |
|-------|--------|
| P1 (requester) | Unblocks PGN4 ingestion (§6.5/§9); confirms value-system split for `types.rs` (§1.7/§1.8) and underpromotion/`PromotedQueen` (§1.4). FEN4 work unaffected (already v0.1-stable). |
| P0 (target) | Spec edited: 10 items integrated, header → v0.2, delta marked merged. |
| P2 | Castling tables (§1.5) + `PromotedQueen` (§1.4) now authoritative for move-gen. |
| P5 | `eval_value()` vs `ffa_points()` split (§1.7/§1.8) authoritative for V's Mᵢ and result tags. |

## Recommended Fix

Apply the delta items at their anchors, micro-staged by section group:
(a) §1.3–1.5 placement/promotion/castling; (b) §1.7–1.8 value systems + claim threshold + DKW +
stalemate; (c) §2.3 bishop `[PENDING CALIBRATION]`; (d) §6.5 + §9 PGN4 + file structure; (e) §7.3 EP
tests. Bump header to v0.2.

## Alternatives Considered

- **Leave v0.2 to Kimi, build P1 against v0.1+delta.** Rejected: split source of truth invites
  drift exactly on the PGN4 surface P1 implements; user authorized autonomous progress.
- **Raise the CO and wait for explicit per-item sign-off.** Rejected: user is asleep and authorized
  carrying on; the delta is content-complete and pre-accepted, so the merge is mechanical, not a
  design decision.

## Resolution

**Status:** open — execution deferred by the requesting agent pending a dispatch decision.

**Why deferred (decision for the user/dispatch):** I had standing authorization to "carry on,"
but on reading the full spec I judged that *executing* this merge tonight is not the mechanical
integration it first appeared, for two reasons that make it a real decision rather than rote work:

1. **Section-anchor mismatch.** The delta routes PGN4 ingestion to "§6.5," but the current spec's
   §6 is *Search Contract* and there is **no protocol/I-O section anywhere**. Landing PGN4 requires
   *creating* a new section and choosing where — a structural choice the spec owner (P0/Kimi)
   should make, not me.
2. **Kimi collision.** The pitch states Kimi is actively "releasing v0.2." Two agents landing v0.2
   independently produces divergent canonical specs — the exact failure the clean-boundaries rule
   prevents.

**Recommended next step (pick one):**
- (a) Kimi lands v0.2 (owns the spec; resolves the §6.5 placement natively), **or**
- (b) dispatch explicitly authorizes me to land it *and* confirms Kimi is not mid-edit — then I
  apply the merge and choose a sensible new "Protocol & I/O" section for PGN4.

**Impact of deferral:** none on P1 FEN4 work (v0.1-stable). P1 **PGN4** work (Stage 3) stays
blocked until this resolves. No other phase is started yet, so nothing else is affected.

**Resolved By:** dispatch (user) — "no kimi for now so carry on with your work" authorized claude as
sole agent to land v0.2 and to make the §6.5 placement call. Both deferral reasons dissolved: the
collision risk is gone (no Kimi), and the structural call is now mine to make as spec owner.
**Resolution Date:** 2026-06-01
**Resolution:** All 10 delta items integrated into `HORNET-BUILD-SPEC.md` at their anchors; header
bumped to v0.2 with a changelog. **§6.5 placement decision:** PGN4 ingestion + protocol commands +
the FEN4/PGN4 grammars were landed as a new top-level **§10 Protocol & I/O Formats** (placed before
the Appendix so §§1–9 numbering is unchanged), since the spec had no protocol/I-O section. Delta
file marked MERGED (kept as record). Also corrected a stale, geometrically-wrong EP example in §1.6
to match the verified §7.3 examples.
**Verification:** Spec re-scanned — version = 0.2; no residual `PieceType::value()` references; eval
vs FFA value systems both present and not conflated (Hard Rule #8); EP examples consistent between
§1.6 and §7.3.
