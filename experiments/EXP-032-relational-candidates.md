# EXP-032 — first mining-nominated relational candidates: pawn advancement breaks through

- **Date:** 2026-06-12 · **Status:** CLOSED — gate null, candidate stays cold
- **Hypothesis:** relational/behavioral features — one classical (rook on open lanes), one
  nominated by behavioral mining pass 2 (winners push pawns toward the central promotion
  crossing, +2.1pp Center / −2.6pp home in red-frame destinations) — can move the winners gap
  that EXP-030 showed linear counts cannot.
- **Lever / change:** two cold candidate evals (EXP-029 pattern, deployed eval untouched):
  `eval_4vec_rook_open` (open lane = no pawns on the rook's own-orientation lane, semi-open
  half; `HORNET_ROOK_OPEN_SCALE`) and `eval_4vec_pawn_adv` (sum of pawns' own-frame forward
  progress; `HORNET_PAWN_ADV_SCALE`). `queries::pawn_lanes` made pub(crate) for reuse.

## Results (winners-only move_match, upgraded instrument; baselines 14.1% / 13.3%)

| Arm | all | winners-only |
|-----|-----|--------------|
| deployed | 14.1% (485) | 13.3% (279) |
| rook-open 100 | 14.3% | 13.4% |
| rook-open 200 | 14.1% | 13.4% |
| rook-open 400 | 14.1% | 13.1% |
| **pawn-adv 8** | **15.1% (521)** | **14.8% (310)** |
| **pawn-adv 16** | **15.1% (518)** | **14.8% (311)** |

- **Pawn advancement: the first candidate to clearly beat the baseline** — +1.0pp all,
  **+1.5pp winners-only (+31 winner-matches, ~7× ISO's best effect)**, stable across a 2×
  scale range (a plateau, not a tuning spike). Advanced to the paired gate
  (scale 8 — the smaller distortion of the two equals).
- **Rook-open: flat** (≤+0.1pp at low scales, negative at 400) — not advanced. The classical
  guess lost to the mined behavior, which is the mining mandate's thesis in one line.

## Gate run 1: VOID (harness bug, fixed)

The first 12-pair gate run came back 0-0-12 — every pair an EXACT tie with identical point
vectors. That is the null signature (EXP-027): both arms played the same eval. Cause:
`selfplay_ab`'s `Cfg::searcher()` only mapped eval ids 1/2 (P′/S′); id 4 (pawn-adv, move_match's
numbering) fell through silently to deployed. Same defect class as the zero-weight fold-in —
a silent default where a hard error belonged. Fixed: ids 3/4 wired (rook-open/pawn-adv), unknown
ids now panic. Silver lining: the void run is an inadvertent second null-validation of the
paired design at d8 cap 1200 — 12/12 exact ties, seat variance cancels to the point.

## Gate run 2 (valid): NULL

12 pairs, d8 cap 1200, HORNET_PAWN_ADV_SCALE=8, B arm verified distinct (`Padv` label, every
pair contested): **pair record 6–6–0, points A(deployed) 809 vs B(pawn-adv) 800** — pair
differentials sum to +9 for deployed, mean +0.8 ± ~30 sd. Dead even.

## Conditions (after)

- **No wiring.** Pawn-adv stays a cold candidate fn (`eval_4vec_pawn_adv`, env-scaled).
  Deployed eval unchanged; nothing ships on a null (default-off discipline).
- Revisit trigger: the incoming human-game influx. At a larger corpus, re-run the winners-only
  instrument and the texel single-term fit; if the nomination repeats with a sharper scale,
  the gate gets another 12 pairs. Not before — 12 pairs resolve only LARGE effects, and the
  honest read is that any true effect here is small.

## Conclusion

**The instruments disagree, and the gate outranks the predictor.** Pawn-adv clearly beat the
winners-only agreement baseline (+1.5pp — humans who win DO advance pawns more, mining pass 3
confirmed winners promote 2.6×) but converted to ZERO self-play strength at these settings.
Fourth instance of the standing lesson: **prediction agreement ≠ play strength** (after P′, S′,
and the EXP-030 linear terms). Plausible mechanism: in engine-vs-engine play both sides push
pawns equally well already, or the term's gains are offset by what it displaces in the
mean-relative budget. The mining loop still did its job — it found a real human behavior and
the gate told us cheaply that this representation of it doesn't buy strength. The behavior
stays in the repertoire; the representation question stays open.
