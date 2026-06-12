# EXP-031 — objective layer on the powered paired gate: a lean, not a pass (yet)

- **Date:** 2026-06-12 · **Status:** extension running
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

## Conditions (after)

- Play defaults unchanged (`go` = plain flashlight cap 1200). Nothing ships on 7–5.
- The direct human gate is available regardless: the REPL plays end-to-end
  (`cargo run` → `position startpos` / `go depth 8`) — the user playing the engine is the
  beats-humans measurement itself, and doesn't wait on A/B resolution.

## Conclusion

PENDING the 36-pair total. Standing lesson reinforced: every prior "big" self-play effect has
shrunk under the paired instrument — strength claims in this project are now earned slowly and
honestly or not at all.
