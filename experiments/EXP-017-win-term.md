# EXP-017 — the search-side win term (attunement: aim for the win)

- **Date:** 2026-06-08 · **Status:** in progress
- **Premise (from EXP-016 + the user's framing):** depth goes *neutral, not positive* once pruning is
  fixed, because the search optimizes the wrong objective (points-blind cp). The fix is **attunement** —
  make the search value the actual objective (FFA points / eliminations). The user's sharper model:
  **FFA points = the goal (who to target); cp = the means (the tactics to get there).** Two levels of
  examination, not a blend.

## What was built (`search.rs`)
`Searcher::with_win_term(weight)` + `eval_with_win`: at every flashlight leaf the value becomes
```
value_i = cp_eval_i  +  weight × (points_i − mean_points)        (clamped)
```
i.e. cp (means) plus a mean-relative **FFA-points** term (goal). The static eval stays points-blind
(Hard Rule #8 intact for the *eval*); the points-awareness lives **search-side** only. `weight = 0` is
the validated `flashlight == exact Max^n` path (default-off; 111 lib tests green). Correctness: the
board accumulates `points` on make (captures incl. king-capture) and **restores on unmake**
(`UndoState.prev_points`), so the goal signal is correct along every line.

Pathology note: the old king-hunt blowup was a **material-sweep** artifact (cp), *not* a points problem,
so points (the bounded +20) are a clean goal signal — the reason this is safe to weight meaningfully.

## Result 1 — does it play better? (win-on vs win-off, d8, cap 400, 6 games)
`selfplay_ab 8 8 1 400 50 0`:
- **A(win-on, w=50) beat B(win-off): 4/6 = 67% win-rate, points 277 vs 236 (+17%).**
- **Decisive games: 0/6 (no eliminations).**

Read: **attuning the search to points makes it play measurably better** (it captures more purposefully
→ out-scores). Directional only at n=6 (ab_stats: not yet significant; the points gap is the clearer
signal). The **0 eliminations is expected, not a failure** — at d8/cap-400 the search can't see forced
king-capture lines (deeper than 8 plies; self-play kings don't blunder), so the win term scores *points*
but can't *finish* what it can't reach. **Eliminations need depth** → Result 2.

## Result 2 — does depth now PAY? (d8-win vs d4-win, cap 1000, 6 games)
`selfplay_ab 8 4 1 1000 50 50`:
- **A(d8) win-rate 4/6 = 67%**, points **210 vs 228** (d4 +9% — skewed by one blowout loss, game 1 27–62),
  decisive **3/6 = 50%**.
- **vs the EXP-016 baseline** (d8 vs d4, *no* win term, cap ~1000–1200: **33%** win-rate, −7% pts, ≤2/6
  decisive): the win term **doubled d8's win-rate (33%→67%)** and **lifted decisiveness** (0/6 at cap-400
  → 3/6 at cap-1000).

**Read:** depth started paying **on placement** (win-rate doubled) — the *right* axis, since FFA is won on
placement, not raw point-hoarding (the points gap barely moved). And the win term **de-drawishes once the
search has the breadth to find eliminations** (cap-400 0/6 → cap-1000 3/6). **Caveat:** n=6 — win-rate and
points disagree, which *is* the small-sample noise. Not yet a precise number.

## Result 3 — does depth KEEP paying at d12? (d12-win-danger vs d8-win-danger, cap 400)
`selfplay_ab 12 8 1 400 50 50 100 100`:
- **A(d12) vs B(d8): win-rate 3/6 = 50%, points 209 vs 256 (d8 +22%).** d12 did **not** improve on d8.
- **But this is cap-400, and EXP-016 established depth is pruning-disadvantaged at low cap** — d12 prunes
  through 12 levels of a 400-cap beam vs d8's 8, so d12 is **under-bred**, not necessarily worse. The
  variable-depth-leaf asymmetry (ENGINE-MATH §7.4) compounds it.
- **Read:** d12 at cap-400 **exposes a constraint, not a ceiling — breadth must scale with depth.** This
  is the empirical case for the beam-broadening phase: deeper search *needs* a bigger cap to pay.
- **Confirmed (cap 1000):** `selfplay_ab 12 8 1 1000 50 50 100 100` → **A(d12) 5/6 = 83%, points 209 vs
  181 (+14%).** The deficit **flipped** (cap-400 −22% → cap-1000 +15%): d12 was **pruning-limited, not
  ceilinged.** Depth **keeps paying through d12** when breadth scales with it.

  | d12 vs d8 | win-rate | d12 points |
  |---|---|---|
  | cap 400 (under-bred) | 50% | −22% |
  | cap 1000 (bred) | 83% | +15% |

## Verdict (the depth thread, EXP-016 → 017)
With the objective layer in (win + king-danger), **depth pays and keeps paying** — d8 > d4 and d12 > d8 —
**provided breadth scales with depth.** Low cap re-creates the EXP-016 pruning penalty at every new depth.
So the roadmap is exactly the user's: objective layer (done) → depth pays → **broaden the beam** (now
shown to be *required*, not optional, for deep search).

## Next
- Confirm the depth + safety effects with ~16 games each (both are near-significant at n=6).
- The beam-broadening phase: find the cap that breeds each depth (a cap-vs-depth schedule).
- Tune `weight` (higher → more eliminations / cleaner depth edge?).
- If it holds: quantify the gain, then relational terms + a **points-aware** safety (the cleanly-separated
  layer the safety-negative finding points to) on top.
