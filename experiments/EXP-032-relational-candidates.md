# EXP-032 — first mining-nominated relational candidates: pawn advancement breaks through

- **Date:** 2026-06-12 · **Status:** paired gate running on the winner
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

## Conditions (after)

- Deployed eval unchanged; gate verdict pending (12 pairs vs deployed, d8 cap 1200, running
  alongside the EXP-031 extension).
- If the gate confirms: pawn-adv is the first eval feature to earn a default-on discussion since
  the recalibration — wiring would go through P (a real positional term at last) with the
  measured scale, Tier-2 ship.

## Conclusion

PENDING the gate — but the process result already stands: **mine behavior → represent → gate**
produced in one pass what three experiments of statistical candidates couldn't. The discovery
loop the user mandated works.
