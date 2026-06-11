# EXP-002 — exchange-aware (SEE) threat scoring

- **Date:** 2026-06-06
- **Hypothesis:** the gate fails because the eval can't tell a winning capture from a losing one;
  replacing the flat target-value threat term with exchange-resolved (SEE) threats will lift it.
- **Lever / change:** `HORNET_SEE=1` env flag (default off) selects `query_threats_see` over the flat
  `query_threats`. SEE is direct-only (X-ray staged out), 2-sided per attacking player, in
  centipawns (`eval_value`, Hard Rule #8-clean), discounted `/4`, folded into **Pᵢ**. `queries.rs`.

## Conditions (before)
- Eval v0, weights `4/2/1/1`, mean-relative (zero-sum). `query_threats` = `+value(target)/4` for any
  enemy first-occupant, no defender/attacker-value check, folded into Pᵢ.
- Search: beam 10 + LMR + adaptive, 800k node budget. Fixtures: 13 testable.
- Baseline (SEE off): depth-4 qOFF **0/13**, qON 1/13.

## Method
`examples/gate_ablation.rs`, depth 4 × quiescence {off,on}, release, isolated target. Run twice:
`HORNET_SEE` unset (off) then `=1` (on). On-vs-on isolated by the env flag, one build.

## Results

| | quiescence OFF | quiescence ON |
|---|---|---|
| **SEE off** | 0/13 | 1/13 |
| **SEE on** | 0/13 | 1/13 |

**No change.** SEE-resolved threats produce identical match counts to the flat term.

## Conditions (after)
- `query_threats_see` + `see_swap` + `reaches_directly` implemented and unit-tested
  (`see_swap_resolves_exchanges`, `see_threats_credit_only_winning_captures`), **default-off**
  (`HORNET_SEE`). Only the **threats** component is exchange-aware; **safety and crossfire are still
  flat**. V intact (threats → Pᵢ, centipawns). Suite green.

## Conclusion
**Null result — the threats-only form of the exchange-aware hypothesis is not supported on these 13.**
Candidate reasons, in priority order:
1. **We haven't looked at the misses.** Every eval hypothesis so far is a guess about what the 13
   fixtures require. Before adding more eval machinery, **dump human-move vs engine-move + position
   type per fixture** (→ EXP-003). That tells us whether the misses are even capture/threat-shaped, or
   king-safety / quiet-positional / multi-move (which need the depth re-test on a better eval), or a
   metric artifact (engine plays a reasonable non-identical move).
2. **Only 1 of 3 components done.** The pitch's full hypothesis is exchange-aware threats **and**
   safety **and** crossfire. Safety/crossfire SEE are not implemented.
3. **Signal wash-out.** Threats are `/4`, into Pᵢ (`W_POSITIONAL=2`), then mean-normalized — small vs
   material (`W_MATERIAL=4`). Confirm SEE actually altered *selection* (move-diff off vs on); identical
   chosen moves would point to wash-out (or env non-propagation), not just "didn't match the human."

**Next:** EXP-003 — inspect the 13 misses before extending SEE. Do not pile on eval features blind.
The SEE lever stays in (correct, tested, default-off) for when the full exchange-aware eval is run.

**Re-test status (2026-06-06):** re-run on the *recalibrated* eval (EXP-008) — **still null** (quiet
267→278, match 1/13→1/13). SEE-as-eval is a confirmed dead end even on a sane eval; its residual value
is as a blunder *filter* (EXP-004, S22), not an eval term.
