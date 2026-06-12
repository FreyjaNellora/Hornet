# EXP-030 — play-volume scale sweep (the eval attack): linear terms don't close the winners gap

- **Date:** 2026-06-12
- **Hypothesis:** the EXP-029-rejected candidates fail only by *volume* — at play-calibrated
  scales (well below the Texel-fitted prediction weights), ISO and/or DGR should lift
  winners-only agreement.
- **Lever / change:** candidate scales made env-tunable (`HORNET_PPRIME_SCALE`,
  `HORNET_SPRIME_SCALE` — read once per process; defaults = the fitted values). Sweep run on the
  **upgraded instrument** (shared `replay` module: all five replayers now use one implementation
  with the EXP-028 fidelity fixes; `pgn4_replay` floors raised 5000/15 → 7000/28).

## New standing baselines (upgraded instrument, 3,440 positions / 2,097 winner-moves)

Deployed eval: **14.1% all / 13.3% winners-only** (was 13.6/12.4 on 2,530 positions — the fixed
replayer harvests 36% more positions, reaching past castles/eliminations into late-game regions
where the material-driven engine agrees more; the winners gap narrows to 0.8pp but persists).

## Results

| Arm | all | winners-only |
|-----|-----|--------------|
| deployed | **14.1%** (485) | 13.3% (279) |
| ISO 400 | 13.9% | 13.4% |
| ISO 800 | 13.8% | **13.5%** (283) |
| ISO 1600 | 13.5% | 13.4% |
| DGR 1 | 12.9% | 12.4% |
| DGR 2 | 12.6% | 12.2% |

- **DGR: rejected at every volume** — monotonically harmful on both metrics even at weight 1
  (~1 pawn-equivalent max swing). A static danger penalty worsens move choice at any loudness;
  the term's Texel signal was outcome-symptom, full stop. Danger handling belongs to the
  **search** (the runtime objective-layer knob, EXP-017/018) — this now has a complete
  three-experiment paper trail (W_SAFETY=0 in EXP-015, fitted-scale rejection in EXP-029,
  all-volume rejection here).
- **ISO: winners-neutral, directionally right.** Every volume trades loser-agreement away for a
  slight winner-agreement gain (peak +4 matches at ISO 800 = +0.2pp — below instrument
  resolution). **No arm advances to the paired gate** (pre-registered "beats baseline" not met
  at resolution; gating a +0.2pp effect would spend the gate's credibility on a coin flip —
  deviation from the mechanical rule recorded with this reasoning).

## Conditions (after)

- Deployed eval unchanged (still byte-identical to EXP-022). Candidate fns + env scales remain
  for future sweeps.
- Instrument consolidation shipped: `src/replay.rs` (self-sync + DKW inference + rotation-aware
  castles) now feeds `pgn4_replay`, `texel_tune`, `move_match`, `move_diverge`, `replay_rules`.
  New move_match baselines above; texel coverage also grew (refits should re-anchor on it).

## Conclusion — what "the eval issue" actually is, after three experiments

The winners gap is not closable by **linear scalar counts at any volume**. The candidates'
predictive signal was real but not *actionable*: knowing isolated pawns correlate with losing
doesn't tell the engine which move to make differently, and penalizing static danger makes it
cower. The ISO direction (loser-agreement traded for winner-agreement) is the one live lead —
winner-aligned features exist, but they need to be **relational and tactical, not counting**:
the unbuilt C3 program (rook on open lanes, outposts, targeted mobility toward weak armies),
interaction terms, or ultimately the NNUE once the data pipeline matures. That — richer
features, not louder scalars and not deeper search — is the eval issue, now measured from three
sides.
