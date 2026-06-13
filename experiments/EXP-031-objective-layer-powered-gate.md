# EXP-031 — objective layer on the powered paired gate: a lean, not a pass

- **Date:** 2026-06-12 · **Status:** **CLOSED — both weights. Consistent lean, never
  significant; defaults stay off; compute redirects to the mining-nominated representation.**
- **Hypothesis:** the search-side objective layer (win 50 + king-danger 100 — the EXP-017 config
  whose unpaired 6-game reads suggested doubled win-rates) beats the plain eval on the paired
  gate, justifying flipping the play defaults (the first gated strength upgrade toward the
  user's beats-humans bootstrap goal).
- **Lever / change:** none yet — measurement only. `selfplay_ab` paired (EXP-027), d8 cap 1200.

## Results (first 12 pairs / 24 games)

| | points | per seat-game | pair record | decisive |
|---|---|---|---|---|
| A = win 50 + danger 100 | 771 | 16.1 | **7–5** | 6/24 = 25% |
| B = off (deployed play config) | 755 | 15.7 | | |

**Read:** a positive lean (+2.1% points, 7–5 pairs) that does not clear any honest bar at n=12
(7–5 is p≈0.39 one-sided under the null). The EXP-017 enthusiasm ("win-rate doubled") was the
unpaired instrument's noise; the real effect, if present, is modest at these weights — which were
first guesses, never tuned. Extension to 36 total pairs running; weight variants (win 100,
danger 50, table shape) are the follow-up sweep if the extension stays ambiguous.

## Results (extension, 24 pairs / 48 games)

| | points | per seat-game | pair record | decisive |
|---|---|---|---|---|
| A = win 50 + danger 100 | 1587 | 16.5 | **14–9–1** | 14/48 = 29% |
| B = off (deployed play config) | 1417 | 14.8 | | |

**Seed-collision dedup:** the extension's pairs 1–2 are move-for-move REPLAYS of the original
run's pairs 1–2 (identical point vectors verified). The harness seed index `si*per_split+g`
renumbers when per_split changes (2→4), so split 0 regenerated seeds 1–2. The combined verdict
uses the 34 UNIQUE pairs (duplicates dropped: one A-win, one B-win). Harness fixed: seed-offset
arg 15 — extensions must pass the pair count already played.

## Verdict (34 unique pairs / 68 games)

- Pair record **20–13–1**; points **A 2266 vs B 2084** (per seat-game 16.7 vs 15.3, **+8.7%**;
  mean pair differential +5.4 pts, sd 23.3).
- **Paired t = 1.34** (df 33, one-sided p ≈ 0.09); **exact sign test p ≈ 0.15**.
- A consistent positive lean across both runs — but it does not clear 0.05 on any honest test
  at first-guess weights (win 50 / danger 100, never tuned). **Not a pass. Defaults stay.**

## Conditions (after)

- Play defaults unchanged (`go` = plain flashlight cap 1200). Nothing ships on p ≈ 0.09.
- Follow-up per plan (variants before more pairs): **win 100 + danger 100 vs deployed**, 12
  pairs, seed offset 0 ON PURPOSE — common random numbers with the original w50 run, so the
  two arms are comparable head-to-head on identical openings. If the variant's lean is no
  bigger, the next lever is danger 50 / table shape, then more pairs at the best variant only.
- The direct human gate is available regardless: the REPL and `examples/play.rs` play
  end-to-end — the user playing the engine is the beats-humans measurement itself.

## Variant sweep: win 100 + danger 100 (12 pairs, common seeds with the w50 arm)

| | points | per seat-game | pair record | decisive | paired t (one-sided) |
|---|---|---|---|---|---|
| A = win 100 + danger 100 | 904 | 18.8 | **7–5** | 7/24 = 29% | **t(11)=1.88, p≈0.044** |
| B = off (deployed) | 746 | 15.5 | | | |

- **+21.2% points (+13.2/pair)** vs the w50 arm's +2.1% on the SAME openings — same 7–5 pair
  record, ~10× the margin. CRN head-to-head (per-seed differential w100−w50): +11.8/pair,
  t(11)=1.40, p≈0.10 — directional, not conclusive on its own.
- A marginal pass with a multiple-comparisons caveat (second config tested). Per the
  pre-specified rule — more pairs at the best variant only — **extension running: 12 fresh
  pairs at w100 (seed offset 12, the new harness arg). 24 total pairs decide the flip**: if the
  true effect is ~+13/pair, t at 24 pairs ≈ 2.6 — clears 0.05 cleanly; a shrink-to-lean kills it.

## w100 extension (12 fresh pairs, seed offset 12) and final verdict

The extension came back **dead even: 7–5 pairs but points A 832 vs B 840 (−8)**. The first 12
pairs' p≈0.044 was the lucky half. Combined w100 over 24 pairs: **14–10, points 1736 vs 1586
(+9.5%), t(23)=1.27, one-sided p≈0.11.** Per the pre-registered rule (shrink-to-lean kills it):
**NOT a pass. The `go` defaults stay plain flashlight.**

Family evidence, recorded honestly: pooling both weights (58 pairs: w50 +5.35/pair over 34,
w100 +6.25/pair over 24) gives mean +5.7/pair, t(57)=1.86, p≈0.034 — but the pooling decision
was made AFTER seeing the per-config results, so this is suggestive, not confirmatory. If the
true effect is ~+6 pts/pair (~0.25 sd), a properly powered single-config gate needs ~100+ pairs
(30+ hours) — compute that the mining program can spend better.

## Conclusion

The objective layer in its current shape (banked-points win term + king danger) produces a
remarkably CONSISTENT ~+9% points lean at both tried weights across 58 honest pairs — and never
clears the bar. Decision: defaults stay off; no more pairs at these shapes. Two better paths,
both already queued: (1) **the mining-nominated objective-layer representation** — elimination
forensics shows kills target the MATERIAL-weakest opponent (67% last-rank), so target selection
by material weakness is likely the right shape where the generic win term is the wrong one;
(2) **the human gate** — at this effect size the tester loop (versus_games/) measures what
actually matters, beats-humans, faster than self-play significance does. Standing lesson, third
time today: every effect shrinks under the paired instrument; ship nothing on a lucky half.
