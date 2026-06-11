# EXP-004 — quality metric (blunder vs different) + resolve S02

- **Date:** 2026-06-06
- **Hypothesis:** exact-match (0/13) conflates "blundered" with "played a different good move". A
  quality metric — SEE of each move + the engine's value gap between its pick and the human move —
  will say whether a real strength problem exists, and resolve the S02 "declined queen capture".
- **Lever / change:** instrumentation. New `Searcher::root_move_values` (all root moves + values) and
  `queries::see_capture` (public per-move SEE). `gate_ablation.rs` prints per fixture: human/engine
  move, each move's SEE, the value gap, and a verdict.

## Conditions (before)
Engine: depth 4, beam 10 + LMR + adaptive, 800k budget, baseline eval. 13 fixtures.

## Results

```
[S01] H Qf6-c6 xN(300) SEE=-600 | E Qf6-f9 quiet    | gap=9056   disagree
[S02] H Qd7-g4 xQ(900) SEE=0    | E Qd7-d5 quiet    | gap=7949   disagree   (even TRADE, not a win)
[S03] H Qj4-j9 quiet   SEE=-    | E Ne1-d3 quiet    | gap=3615   disagree
[S04] H Nd3-e5 quiet   SEE=-    | E Pj2-j3 quiet    | gap=7151   disagree
[S05] H Qj4-g4 quiet   SEE=-    | E Ni3-h1 quiet    | gap=7511   disagree
[S06] H Bm8-h3 xP(100) SEE=-350 | E Bm8-l9 quiet    | gap=10832  disagree
[S07] H Qk7-f2 quiet   SEE=-    | E Qk7-g3 quiet    | gap=1385   disagree
[S10] H Ka7-a5 quiet   SEE=-    | E Qd7-d6 quiet    | gap=6884   disagree
[S17] H Qg1-j4 quiet   SEE=-    | E Qg1-i3 quiet    | gap=4472   disagree
[S18] H Qg9-m9 xP(100) SEE=-800 | E Qg9-f9 quiet    | gap=11499  disagree
[S21] H Qi5-m5 xP(100) SEE=+100 | E Pg2-g4 quiet    | gap=221    missed-win?
[S22] H Qh14-f12 quiet SEE=-    | E Qh14-c9 xP(100) SEE=-800 | gap=6572  ENGINE-LOSES-MATERIAL
[S23] H Rd14-d12 quiet SEE=-    | E Qf6-f8 quiet    | gap=7204   disagree
--- 0/13 match | blunders 1 | missed-wins 1 | close 0 | disagree 11 ---
```

## Conclusion
**The measurement is suspect — we have been optimizing against an unvalidated gate.**
- **S02 resolved:** the declined "queen capture" is an even **trade** (SEE 0). Engine decline is fine,
  not a bug.
- **The engine rates the human moves as catastrophic** — value gaps of thousands of cp (up to
  ~11,500), *including on quiet human moves* (S03/S05/S10 are quiet, gaps 3600–7500). And
  eval-independent **SEE flags three human captures as material-losing** (S01 −600, S06 −350, S18 −800).
- **A 3000-Elo player does not hang a queen for a pawn or play 70-pawn-losing quiet moves.** So at least
  one of these is true, and all undermine the gate:
  1. **Replay drift** — `moves_to_replay` is decoded token-by-token (freyja→chesscom + pseudo-legal
     match); a single mis-decode lands a legal-but-wrong move and every downstream position/eval is on
     the wrong board.
  2. **v0 eval miscalibration** — a quiet move scoring −7000 means the eval swings tens of pawns on
     non-captures.
  3. **SEE over-counts defenders** — `see_capture` has no pin/legality check; a pinned "defender" makes
     a sound capture look losing (known staged-out limitation).
- **S22:** a genuine engine blunder (plays `Qxc9`, SEE −800). SEE-as-eval was null (EXP-002), but **SEE
  as a move filter would catch this** — a real use for the SEE code.

## Next
- **EXP-005 — validate the harness before any more eval work.** (a) Replay fidelity: does the replayed
  board match the fixture's source position? (b) Eval calibration: are `eval_4vec` magnitudes sane on
  known positions? (c) SEE pins: does `see_capture` over-count? Until these pass, **0/13, the SEE-null,
  and these gaps are not trustworthy signals** — they may be measuring drift/miscalibration, not strength.
- The earlier conclusions (eval-bound, positional-not-tactical) stand only if the harness is sound;
  re-confirm after EXP-005.
