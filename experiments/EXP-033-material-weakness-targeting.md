# EXP-033 — material-weakness targeting term: mining-nominated objective-layer shape

- **Date:** 2026-06-13 · **Status:** **CLOSED — flat gate NULL (no wiring). Proximity variant not run.**
- **Hypothesis:** the behavioral mining finding (67% of elimination victims rank last in material) can be encoded as a search-side objective-layer term: reward each player for having SEE-winning threats against their materially-weakest opponent. This is the specific representation the EXP-031 redirect nominated, replacing the generic "want points" win term.
- **Lever / change:** `Searcher::with_target_weight` + `query_target_pressure` — a cold candidate in the objective layer (deployed eval untouched, default-off, gated before any wiring).

## Build

### Reused (no change)

- `queries.rs::query_material` — per-player material count (same convention as mining heuristic)
- `queries.rs::turn_proximity_weight` — turn-distance weighting (1.0× next, 0.6× 2-away, 0.3× 3-away)
- `queries.rs::see_swap` / `see_capture` — existing SEE threat machinery for "pressure"
- `search.rs::eval_with_win` — insertion template (win/danger blocks precede the new term)

### New

- `queries.rs::query_target_pressure(lines, board, proximity)` — for each player, sum of SEE-winning threats against their materially-weakest opponent's pieces. If `proximity=true`, threats are weighted by turn proximity (next-to-move weak opponent = more urgent).
- `search.rs`: `target_weight: i16` field (default 0), `with_target_weight` builder, `with_target_proximity` shape knob
- `search.rs::eval_with_win`: target block inserted after win/danger, mean-relative + clamped to `±30_000`
- `examples/selfplay_ab.rs`: `target`/`tprox` fields in `Cfg`, arg parsing (args 16/17 = target weight, 18/19 = proximity flag), label formatting
- `examples/move_match.rs`: eval_id arms 5 (target 100 flat) and 6 (target 100 proximity)

### Signal shape

```
pressure[i] = Σ SEE-winning threats by player i against player i's weakest opponent
mean = average pressure across 4 players
adj[i] = target_weight × (pressure[i] - mean) / 100
```

Mean-relative (Σ≈0), clamped to mate bounds. The `/100` scales the raw pressure (centipawn-scale SEE values) to match the weight convention used by win/danger terms.

### Env-tunable scale

The weight is passed via the builder (`with_target_weight`), not a static env var. For sweep arms, the harness sets the weight directly per config. This is the same pattern as `win_weight`/`danger_weight`.

## Verification

- Suite: **117 lib tests pass** (unchanged — the term is cold/default-off)
- Deployed eval byte-identical: `gated_queries_match_full_eval` and rotation-equivariance tests pass unchanged (target_weight=0 → the target block is skipped, `eval_with_win` returns the same vector as before)
- Examples compile: `selfplay_ab`, `move_match`, and all others build clean

## Gate protocol (established methodology)

1. **Winners-only move_match** vs the 143-game baseline (14.1% all / 13.3% winners-only):
   - `move_match 10 4 2 0 0 5` = target 100 flat
   - `move_match 10 4 2 0 0 6` = target 100 proximity
   - Advance to paired gate only on a **clear winners-only beat** (EXP-030 sub-resolution rule: don't spend gate credibility on a coin flip)

2. **Paired gate** (EXP-027 design): if winners-only shows a clear lift, run 12 pairs at the best shape, with seed offset discipline

3. **Scale sweep** (if gate is ambiguous): try weights {50, 100, 200} × {flat, proximity} on winners-only first

## Conditions (after)

- Deployed eval unchanged (byte-identical to EXP-022). The target term is a cold search-side knob only.
- `selfplay_ab` gains target args 16-19; backward-compatible (defaults = 0/off).
- `move_match` gains eval_id 5/6; backward-compatible (default 0 = deployed).

## Measurement note — the move_match screen is BLIND to this term

The protocol's step-1 screen (`move_match … 5/6`) returned **byte-identical** numbers for eval 0
(deployed), 5 (target flat), and 6 (target proximity): all `485/3440 = 14.1% | 279/2097 = 13.3%`.
Reason: the target term lives in `eval_with_win`, which **only the flashlight path calls**;
`move_match` with `fcap=0` runs the beam `search()`, which never invokes it. So the beam screen
is structurally blind to any objective-layer term (unlike the EXP-029/032 leaf-eval candidates,
which swap the eval used everywhere). This is the same reason EXP-031's win/danger terms skipped
move_match and went straight to the paired gate. **Pivoted to the paired gate** (the right
instrument), confirming on pair 1 that the arms diverge (A 22/B 46 — not the null-tie signature),
so the term IS live on the play path.

## Gate verdict — flat (target 100) vs deployed: NULL

12 pairs, d8 cap 1200, paired seat-swap (EXP-027).

| | points | per seat-game | pair record | decisive |
|---|---|---|---|---|
| A = target 100 (flat) | 803 | 16.7 | **5–6–1** | 7/24 = 29% |
| B = deployed | 740 | 15.4 | | |

- Points favor A by **+8.5%**, but **the pair record is 5–6–1 AGAINST the target arm**, and the
  points lean is two outlier pairs (pair 7 +61, pair 11 +69 = +130 of the net +63; the other ten
  pairs net −67). Mean pair differential +5.25 ± 32.7 sd → **paired t(11) = 0.56, one-sided
  p ≈ 0.29.** No strength gain.
- **No wiring.** Default stays off; the term remains a cold knob.
- **Proximity variant (eval 6) NOT run.** Per the EXP-030 sub-resolution rule, a flat result this
  null doesn't justify another ~3h paired gate; revisit only if the user wants the proximity
  shape tested or the larger incoming corpus re-nominates the behavior.

## Conclusion

**Fifth instance of "mined/objective signal moves the engine but not toward winning"** (after P′,
S′, the EXP-030 linear terms, and pawn-adv). The mining finding is real — winners *do* eliminate
the materially-weakest (67%) — but this representation of it as a leaf-time pressure bonus buys no
self-play strength at weight 100. Plausibly because in engine-vs-engine play both sides already
funnel toward weak opponents via SEE/crossfire, so a term rewarding it nudges both arms equally;
or because elimination is a *search-horizon* outcome (who can actually be killed in N plies), not
a leaf-eval bias. The behavior stays in the repertoire; this encoding doesn't earn a default-on.

## Standing queue (flagged, not this experiment)

- Termination rules (repetition / 50-move / claim-win) — correctness needed before wide tester distribution; requires chess.com verification first
- 4PCo castle tail (recover 31 games' final plies)
- move_match re-baseline on the clean 143
