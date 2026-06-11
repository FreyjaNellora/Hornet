# Relational eval terms for 4PC — which, how many, how to define, how to weigh

Per-square / scalar positional is dead (EXP-015: 8 variants → P=0, on both gates, across 16→32 games).
The untested class is **relational** terms (the bulk of Stockfish's eval — see STOCKFISH-EVAL-MAP.md).
This is the plan to add them *without* repeating the failure.

## The method (read first) — unbundle, tune per-term, bake the winners
The reason positional keeps zeroing: it's **bundled** (control+threats+PST+… under one weight `P`), so
the tuner can only zero the *whole bundle*, killing any good term inside it.
1. **Expose each term as its own raw readout** in `queries.rs` (a separate `[i16;4]`), not summed into P.
2. **Tune a weight per term** (extended `move_tune`: fit N weights, not 4) on **move-agreement**, then
   confirm on **corrected `texel` (outcome-MSE)**.
3. **Keep a term only if it earns a non-zero weight on move-agreement AND doesn't hurt outcome-MSE.**
4. **Bake the survivors into `P`** with their fitted relative weights (Hard Rule #4: P stays one
   V-component; its *recipe* is the tuned relational mix). Drop the rest.
This lets pawn structure get a positive weight even while dead PST gets zeroed — impossible while bundled.

## How many
**Few at a time, each ablated independently.** With 32–50 games the gates have limited resolution, so
testing 15 terms at once is unfair (can't attribute, noise swamps). Start with **3–4**, add more only as
the corpus grows. Quality of definition > quantity.

## Which ones + 4PC definitions (priority order)
Geometry note: **lane = axis ⊥ the pawn's forward direction** — Red/Yellow → file, Blue/Green → rank.
"Enemy" = any of the **3 opponents** (their pawn geometry differs by their own forward direction).

| # | Term | 4PC definition | V slot | Notes |
|---|------|----------------|--------|-------|
| 1 | **Pawn structure** | **Isolated**: no friendly pawn on lane±1 (penalty). **Doubled**: ≥2 friendly pawns sharing a lane (penalty per extra). **Connected/phalanx**: friendly pawn on an adjacent lane at same/adjacent forward-rank (bonus). | P | The canonical relational term; the real swing. Defer *backward*/*passed* (hard in 4PC). |
| 2 | **Rook on open line** | Count friendly pawns on the rook's file **and** rank. 0 friendly on a line → **open** (bonus); only-enemy → **semi-open** (half). Rook controls both axes — take the better, or sum. | P | Replaces the wrong "rook-edge" bonus with the right idea: rooks want open *lines*, not the rim. |
| 3 | **Outpost** (knight/bishop) | Minor on a square that is (a) defended by a friendly pawn AND (b) not reachable by any enemy pawn's future attack. Bonus (larger for knights). | P | Relational (pawn-supported + enemy-pawn-proof). |
| 4 | **King pawn-shelter** | Count friendly pawns in the king's shelter (squares between king and its home edge). Penalty for missing. | S | **Damped by the points-blind rule** (king-capture≈0). Worth pairing with revisiting Hard Rule #8. |
| 5 | **Defended piece** (cheap) | Non-pawn piece defended by a friendly pawn/piece → small bonus (harder to dislodge). | P | Cheap, relational; good control/sanity term. |

## How to weigh
**Don't hand-pick. Fit per-term weights** with the extended `move_tune` (move-agreement objective) and
confirm on corrected `texel`. The fitted weight *is* the answer to "how much." A term that fits to ~0 is
telling you it's not what humans weigh — drop it (the same verdict the per-square tables got, but now
per-term so it doesn't poison the others).

## Sequencing / split of work
1. **Tooling (claude):** extend `move_tune` to fit N per-term weights from a vector of raw component
   readouts (not just M,P,S,O). This is the unbundled gate.
2. **Eval (Kimi):** implement terms #1–#3 as **separate readouts** in `queries.rs` (default-off /
   ablatable), starting with pawn structure (isolated+doubled+connected).
3. **Gate:** run each through the per-term `move_tune` + corrected `texel`; keep survivors; bake into P.
4. **Data:** the gates resolve per-term weights better with more games — target ~50+ before trusting a
   "this term is dead" verdict (per-term needs more signal than the bundled test did).

## Caveats (don't relearn them)
- Tactics (threats/sacrifice resolution) belong in **search/qsearch + SEE**, not as static terms (though
  a *bounded* static threat heuristic is legit — Stockfish has one). We already have `query_threats`
  (capped) + `query_crossfire` (SEE).
- Move-match (top-1) is **tactics/material-dominated**, so a real positional term may show more in
  *outcome-MSE* than in move-match. Watch both; weight outcome more once the corpus is decisive + large.
