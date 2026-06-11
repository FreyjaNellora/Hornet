# Session 002 — Phase 1 (Board I/O) + Phase 0 (landed v0.2)

**Date:** 2026-06-01
**Duration:** continuation of session 001 (same calendar shift; user said "no kimi for now so
carry on", which resolved CO-001 and reopened work)
**Agent:** claude (Opus 4.8)

## Summary

Resolved CO-001 and **landed spec v0.2** (all 10 delta items integrated; new §10 created for
PGN4/protocol). Then built the **PGN4 structural parser/serializer** and round-tripped all 16
corpus games. 19 unit tests + 1 integration test green, clippy clean, fmt applied.

## What Was Done

**Phase 0 — landed v0.2 (CO-001 resolved):**
- Integrated all 10 delta items into `HORNET-BUILD-SPEC.md` at their anchors; header → v0.2 with a
  changelog. Items: value-system split (§1.7 table + §2.3 `eval_value`/`ffa_points` + appendix),
  castling per-player table (§1.5), underpromotion + `PromotedQueen` (§1.4), claim-win threshold +
  stalemate split + DKW behavior (§1.7/§1.8), canonical FEN4 (§1.3), bishop `[PENDING CALIBRATION]`
  (§2.3/appendix), EP tests rewrite (§7.3), PGN4 + file structure (§9).
- **Structural decision (the deferred "§6.5"):** created a **new top-level §10 "Protocol & I/O
  Formats"** documenting the protocol commands + the FEN4 grammar (incl. the field-6 and
  second-dialect caveats) + the PGN4 grammar. Placed before the Appendix so §§1–9 keep their
  numbers.
- Fixed a stale, geometrically-wrong EP example in §1.6 to match the corrected §7.3.
- Marked `HORNET-BUILD-SPEC-v0.2-DELTA.md` MERGED (kept as record). Resolved CO-001 with full audit
  trail. Updated STATUS + phase-0.

**Phase 1 — PGN4 structural parser:**
- `board/pgn4.rs`: `Pgn4Game` {tags, rounds}, `Pgn4Round` {number, plies}, `parse`/`serialize`,
  `Pgn4Error`, `tag()/start_fen4()/initial_board()/ply_count()`. Headers parsed manually (no regex
  dependency). Move stream tokenized into rounds of **raw ply tokens** (variable ply count per
  round; trailing markers like `R` / `Kh13-i14R` preserved verbatim).
- `tests/pgn4_roundtrip.rs` (integration): parses all **16** `baselines/*.pgn4`, resolves each
  `StartFen4 "4PC"` to the canonical board (16 pieces/player), and asserts `parse(serialize(g)) == g`.
  References the corpus via `CARGO_MANIFEST_DIR/../baselines` (no duplication).
- 4 pgn4 unit tests + the integration test. Total suite: 19 unit + 1 integration green.

## What Was Tried But Failed / Notes

- Nothing failed outright this session. Two clippy nits fixed: a collapsible nested-`if` in
  `parse_moves` (extracted `parse_round_marker`, which let me drop a now-unused `BadRoundNumber`
  error variant) and import ordering (fmt).

## Decisions Made

1. **PGN4 is parsed structurally only.** Ply tokens are kept raw; decoding them into concrete moves
   (from/to, SAN disambiguation, legality) needs the board + move generator (P2) and is a real
   engine-design step — deliberately deferred.
2. **PGN4 round-trip is structural, not byte-identical** (unlike FEN4). Source line-wrapping is
   normalized; the guarantee is `parse(serialize(g)) == g`. Documented in the module.
3. **§10 placement** for PGN4/protocol (see Phase 0 above) — my call as sole spec agent.
4. **Baselines referenced, not copied** into the crate (avoids a second copy that could drift).

## What's Next (priority order)

1. **P2 — Move generation** (the next real engine work): `Board` derived state (piece lists, cached
   king squares, make/unmake), legal move generation using castling tables (§1.5) + EP (§1.6),
   perft. This unlocks **PGN4 move semantics** (decode + replay the corpus move streams) and is the
   prerequisite for line projection (P3).
2. Decode `board/pgn4.rs` ply tokens into a `Move` once P2 exists; add move-stream **replay** tests
   over the corpus (currently only structural round-trip).
3. Resolve the board representation decision (currently a 196-cell mailbox) before P2 commits.

## Watch Items

- **Two FEN4 dialects** (unchanged): `tactical_samples.json` uses the `xxx`-corner dialect; not
  loadable by Hornet's FEN4 parser → strength gate (P4/P5) needs a converter. Now documented in
  spec §10.2.
- **FEN4 field 6** still stored raw (grammar unconfirmed). Documented in §10.2.
- **PGN4 ply semantics unverified** — structural round-trip proves tokenization, NOT that the moves
  are legal/correctly decoded. That verification arrives with P2.
- **Trailing ply markers** (`R`, `…R`) in the corpus are preserved but not interpreted (likely
  elimination/resignation markers) — interpret in P2.

## Open Questions

- Exact meaning of the single-letter trailing markers in the move stream (`R`, and suffixes like
  `Kh13-i14R`). Hypothesis: a player's elimination/resignation indicator. Confirm against chess.com
  PGN4 docs or by replay once P2 exists.
- Do all 16 corpus games truly start from `"4PC"`? (The test only asserts 16-pieces when
  `start_fen4() == "4PC"`; it passed for all 16, implying yes.)
