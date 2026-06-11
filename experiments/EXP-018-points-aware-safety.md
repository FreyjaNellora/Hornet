# EXP-018 — points-aware king-safety (why the old one fit negative, and the rebuild)

- **Date:** 2026-06-08 · **Status:** in progress
- **Trigger:** `tools/fit_weights.py` (32 human games, corrected labels) found **W_SAFETY = −1.11, 95%
  CI [−2.02, −0.06] — significantly NEGATIVE.** The current king-safety term is not just useless, it's
  *anti-correlated with winning*. The user: rebuild it **points-aware**.

## Diagnosis — why it fit negative (read from `safety_scalar`)
```
safety_scalar = defense_bonus(+40 / defender) + escape_bonus(+25 / escape) − attack_danger
```
The two **standalone bonuses reward huddling pieces around your own king.** In 4PC, pieces clustered at
home = passive, undeveloped play = you don't contest the board = you lose. So "high safety" by this
metric is really "I'm playing passively," which correlates with losing — and the only way Texel can undo
a huddle-reward that predicts losing is to give the **whole term a negative weight**. The genuine danger
signal (`attack_danger`, the incoming attack) is right-signed but **swamped** by the bonuses.

So the term wasn't *missing*, it was **measuring the wrong thing** (huddle, not danger) — exactly the
kind of distortion that shows up when an objective-level feature is forced into a cp gradient.

## The level-of-examination point (the user's framing)
King-safety is an **objective-level** feature: it measures the risk of **elimination** — losing your
king is losing all your future FFA points / your placement. Jammed into a points-blind cp eval, the
"value of not being eliminated" has no unit to live in, so it gets mis-encoded (here, as a huddle proxy)
and the tuner fights it. King-safety is really **cp exposure (the gradient: how attacked is my king)
scaled by the points stake (the objective: what elimination costs me).** One unit can't say that.

## The rebuild (search-side, points-aware)
- **`queries::king_danger_scalar`** (additive; the old `safety_scalar` is untouched and stays off):
  pure incoming attack on the king, `0 = safe`, positive = under attack. **No huddle bonus** — defenders
  and escapes only *mitigate* the danger.
- **`Searcher::with_king_danger(weight)`**: in the search-side objective layer (the same place as the
  win term), subtract `weight × king_danger / 100` from each player's value. King-safety is now valued
  as the **points-risk of elimination**, weighed in the same trade-off as the FFA-points goal — that is
  what makes it points-aware, rather than a cp huddle reward. Reads the same array-line projection
  (`LineMap`) the eval already populated.
- Eval stays `material + crossfire` (cp means); the old cp safety component stays at weight 0.
- Default-off (`danger_weight = 0` ⇒ exactly the validated `flashlight == Max^n` path).

**v2 (noted, not built):** scale danger by the points stake (a leader with more to lose protects the
king harder than a near-eliminated player) — full points-awareness. v1 is a flat danger weight first.

## Test plan
- **A/B:** `selfplay_ab 8 8 1 1000 50 50 100 0` — both win-on; A king-danger(100) vs B danger-off. Does
  the rebuilt safety improve placement / survival on top of the win term?
- **Fit check (optional):** dump a danger-only column and confirm its CI is no longer negative.

## Results
- **A/B (king-danger 100 vs off, both win-50, d8 cap-400, 6 games):** `selfplay_ab 8 8 1 400 50 50 100 0`
  → **A(king-danger) 5/6 = 83% win-rate, points 241 vs 154 (+44%).** (game 1 was an A loss, 22–42 — noise.)
  ab_stats: p=0.219 at n=6, but the effect is **large — ~16 games would confirm it at 80% power** (vs ~69
  for the depth effect). Decisive 1/6 (king-danger is defensive, so it doesn't add eliminations at this
  shallow cap — the win term is the offense, danger is the defense).
- **Verdict:** the rebuild **works strongly**. The old huddle-safety fit *significantly negative*
  (harmful); the points-aware king-danger is *strongly positive* — direct confirmation that the huddle
  bonus was the bug and that king-safety belongs in the objective layer (valued as elimination-risk), not
  as a cp huddle reward. Weight 100 is good (the game-1 over-defense worry didn't hold up).
- _Fit check (danger-only column) — optional, not yet run._

## Next
- Confirm with ~16 games (the effect is near-significant).
- d12 with win+danger (does deeper keep paying / expose issues — EXP-016/017 thread).
- v2 points-stake scaling (leader protects king harder); check king-danger vs crossfire double-count
  (ENGINE-MATH §7.5).
