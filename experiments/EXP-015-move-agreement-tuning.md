# EXP-015 — move-agreement tuning (the objective pivot)

- **Date:** 2026-06-07 · **Status:** harness built; first result validated
- **Why:** EXP-014's cross-check showed outcome-MSE is the wrong objective on 16 games — it scored a
  move-match *regression* (PST tune, 9.7% vs 11.7%) as an improvement. So pivot the eval objective from
  "predict the winner" (Texel/MSE) to "pick the strong human's move" (move-agreement) — the thing
  EXP-012 showed actually decides games, and a far denser signal (one labelled example per move).

## Tools
- `examples/move_match.rs` — measurement: does the engine's *search* (d4) top move equal the human's.
  Baseline (4,1,1,1): **11.7%** (149/1270).
- `examples/move_tune.rs` — optimizer: hill-climbs the eval weights so the human's move is the eval's
  *top* move most often (depth-1 static eval per child, cached → tuning is arithmetic).

## Result (human corpus, 16 games)
| objective | weights M,P,S,O | move-agreement |
|---|---|---|
| MSE-tuned (EXP-014) | 4,5,2,1 | move_match **9.7%** (regression) |
| default | 4,1,1,1 | static 13.9% · move_match 11.7% |
| **move-agreement-tuned** | **6,0,0,1** | static **18.3%** (+4.4pp) · move_match **13.5%** (+1.8pp) |

Validated on the *independent* search instrument: (6,0,0,1) lifts move_match 11.7% → 13.5% (+23
matches, beyond the ±0.9% noise). **Both instruments agree, and they point opposite to MSE.**

## Findings
1. **The objective pivot works.** Move-agreement found a real gain (+1.8pp validated) where MSE found a
   regression. Use move-agreement to gate/tune eval from here.
2. **Positional AND safety are net-HARMFUL for move choice as currently built** — the tuner zeroes both
   (P=0, S=0). The useful signals are **material (M=6) and crossfire/SEE (O=1)**. This includes the
   EXP-014 PST (it lives in `positional`): centrality is not just unhelpful, it's *anti-aligned* with
   good 4PC moves (center = surrounded by three opponents).
3. **18.3% is the ceiling of the current components.** Re-weighting can't go higher — the gain beyond
   here must come from *better components*, not new weights.

## Decision / handoff
- `eval.rs` left at the default (4,1,1,1) — **not** unilaterally zeroing Kimi's positional substrate.
- **Kimi's call (eval lane):** either (a) deploy material+crossfire-only (6,0,0,1) as a validated
  stopgap (+1.8pp), or (b) better — *fix* the positional component so it lifts move-agreement above
  18.3% (the current control+threats+PST hurt; try **anti-centrality / corner-safety** for 4PC,
  mobility, etc.), gated on `move_tune` + validated on `move_match`. (b) is the real path; (a) is free.
- **Caveat:** 16 games, ceiling = "agree with non-optimal humans". More human games raise the ceiling
  and sharpen the gate. A deploy candidate should also be confirmed by self-play before shipping.

## Update 2026-06-07 (Kimi shipped + claude validated)
Kimi deployed (6,0,0,1) to `eval.rs` and exhaustively tested SIX positional variants (PST v0/v1/v2,
PST-only, zone control, mobility) — **all converge to P=0 / 18.3%** (her report in KIMI-TODO). She also
found via `examples/opening_dev.rs` that 4PC develops **queen before bishop** (queen avg ply 9.3, knight
8.4, bishop 19.2) — opposite of chess; a *development-tempo* signal.

**Claude cross-check — is positional masked by material, or genuinely dead?** Ran `move_tune` on
**quiet (no-capture) positions only** (401 of 2449), where material is constant and can't mask:
- FULL: tuned 6,0,0,1 → 18.3% · QUIET: tuned **M=4 P=0 S=1 O=2 → 24.9%** (baseline 23.2%).
- **Positional stays P=0 even on quiet positions → genuinely dead, not masked.** This *strengthens*
  Kimi's conclusion. Refinement: **safety + crossfire DO carry quiet-position signal** (S=1, O=2) —
  not useless, just material-dominated over the full corpus. Deploy stays (6,0,0,1) (full-set optimum).
- Deploy confirmed: `move_match` = 13.5% (172/1270), the validated +1.8pp. Tree green (111 lib tests).

**Conclusion:** the current eval substrate is tapped out for move-choice by reweighting/PST/zone/mobility.
Going past 18.3% needs **new component _types_** — pawn structure (relational), SEE-resolved threats
(tactical), development tempo (dynamic) — or more human games. All eval-lane (Kimi).

## Final 2026-06-07 — 8 variants, ceiling is structural; DATA-blocked
Kimi tested **development tempo** (`query_tempo`: non-pawn pieces off their start, weighted) — the
*dynamic, de-confounded* feature claude predicted would break P=0. It **also → P=0 / 18.3%** (baseline
14.1%). That's **8 static variants** now (centrality, anti-centrality, per-piece, zone-aware, edge-aware,
zone-control, mobility, tempo), all zeroed. Threat cap confirmed already implemented; rook-edge bonus
dropped (start-square confound). `query_tempo`/`query_mobility`/PST v3 left in code, ablated. 111 green.

**What 18.3% means:** the *best* eval matches the human top move only ~18% of the time → **~80% of human
moves are not what a material+crossfire eval picks** → they are positional/strategic. Positional matters;
the finding is that **no static per-square/scalar table encodes the positional logic humans use.** The
missing signal is **relational** (pawn structure) and **dynamic** (threats/sacrifices that resolve only
under search/qsearch — belongs in SEARCH, not static eval).

**Two frontiers, both DATA-blocked (not effort-blocked):**
1. **Pawn structure** — the one *relational* static type still untested (parked; needs games to gate).
2. **Outcome-based tuning on DECISIVE games** — move-match (top-1) has a positional ceiling: smooth
   positional value shows up in *who wins*, not single-move agreement. Needs decisive games; ours are
   drawish (points-blind eval), human corpus = 16.

**Next move (convergent): more decisive human games** (target 50+). Then re-run move_tune (denser),
enable outcome tuning, test pawn structure first. The eval substrate is sound; data is the bottleneck.

## Seat-order bug in texel — found + fixed 2026-06-07 (corrects the MSE narrative)
`texel_tune::parse_result_points` read the `[Result "name: pts - ..."]` line **positionally** as
[R,B,Y,G]. But chess.com lists it in **score order, not seat order** (e.g. `Green: 76` first). So every
human game's outcome labels were **scrambled** → all prior texel/MSE numbers (the "MSE wall", EXP-014's
PST −0.00063 "win", the original "ablate everything ≈ 0.00005") ran on wrong labels and are unreliable.
**Move-match/move-tune were never affected** (they don't read `[Result]`), so the 18.3% / P=0 conclusions
stand. Fixed: join each Result name to the `[Red]/[Blue]/[Yellow]/[Green]` headers (positional fallback
for self-play games, which are written R,B,Y,G). Added `HORNET_HUMAN_ONLY=1` (skip the drawish self-play).

**Corrected-label re-run (human-only, 18 games, 907 pos):** baseline (4,1,1,1) MSE 0.11478 → tuned
**M=4 P=2 S=1 O=1, MSE 0.11477 (Δ −0.00001).** With correct labels the positional gain **collapses to
noise** — the earlier larger "gains" were partly the model fitting scrambled (≈random) labels. So
corrected outcome-MSE now **agrees with move-match**: no real positional signal on this corpus. The wall
is **data scarcity (18 games), not the label bug.** Deployed eval stays (6,0,0,1). The fix is essential
infra: outcome-tuning is now correct and will be a valid gate once a decisive corpus exists.
