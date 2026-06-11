# EXP-005 — validate the harness (and find the real bug)

- **Date:** 2026-06-06
- **Hypothesis:** the gate's implausible results (EXP-004) come from a broken measurement — wrong
  positions (replay drift), a miscalibrated eval, or SEE over-counting. Find which.
- **Lever / change:** instrumentation. Cross-checked fixtures against the embedded ground-truth games;
  `gate_ablation.rs` now also prints the mover's **static eval** at the position and after the human
  vs engine move.

## Findings

### 1. Data + replay are CORRECT (not the problem)
The JSON embeds the full games. S04's `moves_to_replay` is exactly game `95992584` rounds 1–4, and its
`human_move` `Nd3-e5` is the real round-5 Red move; the replayed board has the knight on d3. The
fixtures reference real moves on the right board. (Some "human hangs material" cases are also just
*checks/sacrifices* SEE-1 can't read — e.g. S18 `Qg9xm9+`.)

### 2. The static eval is wildly miscalibrated (the real bug)
```
       static   aftH     aftE     gap        note
S01    -2006   -5456   -1116     9056
S02     1508   -8010     249     7949    queen trade swings static by 9518
S06    10732   -2959    8170    10832    one Bxp swings static by 13691
S07     9282    9372    7311     1385    engine CHOSE worse-static move (7311 < 9372)
S22    -1342    2164    1393     6572    engine chose worse-static AND SEE-losing capture
S23     -336    -449    -645     7204    static prefers human; search says opposite
```
- A **single move swings the static eval by thousands — up to ~13,700** (S06). Static values span
  −8010..+10732. A sane eval cannot move that much on one move.
- The **search backup is incoherent with the leaves**: in S07/S22/S23 the engine picks the move with
  the *worse* static eval, and the search gaps don't track the static deltas. Not a search bug — the
  search is faithfully maximizing a garbage signal (= EXP-001's "depth can't help through a broken
  eval", shit in → shit out).

### 3. Root cause (concrete)
`query_crossfire` ([queries.rs:227](hornet-engine/src/queries.rs#L227)):
`penalty = enemy_value * enemy_count + own_value`, where `enemy_value` is a **sum of centipawn piece
values**. That is a centipawn×count (≈ value-squared-scale) term, per piece, summed — orders of
magnitude larger than the *linear* material/positional/safety terms. The weights `4/2/1/1` assume the
four components share a scale; they don't. Crossfire dominates and destabilizes the whole vector.

## Conclusion
The entire arc converges here. **Depth didn't help (EXP-001), SEE didn't help (EXP-002), the misses are
"positional" (EXP-003), the gate is suspect (EXP-004) — all symptoms of one cause: the eval components
are scale-mismatched and the crossfire term swings by thousands per move.** The strength gate has been
measuring eval instability, not engine strength.

**Fix = eval recalibration, not features and not search:**
1. Put the four query components on a common scale (crossfire/positional vs material differ by ~100×).
2. Fix the `enemy_value × enemy_count` crossfire formula (it should reflect *material at risk*, not
   value×count — e.g. min(attacker, victim) net of defenders, i.e. an SEE-style at-risk amount).
3. Re-derive `W_MATERIAL/POSITIONAL/SAFETY/CROSSFIRE` against the rescaled components.
4. **New calibration gate:** a single quiet move should swing the eval by ~its positional delta (tens),
   not thousands. Use eval *stability* as the first check, then re-run move-matching.

`queries.rs`/`eval.rs` are Kimi's lane — this is the hand-off. SEE pin over-count (S01/S06 negatives)
is secondary to the scale bug.
