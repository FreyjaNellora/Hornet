# EXP-020 — move-ordering flag ablation (3 arms): FFA bounty + free-capture bonus

- **Date:** 2026-06-10
- **Hypothesis:** the two move-ordering levers (`FFA_BOUNTY_MOVE_ORDER`, `FREE_CAPTURE_BONUS`),
  which shipped hardcoded `const = true` against Hard Rule #6, materially change the move played
  at narrow beams — and the free-capture lever's effect is *contamination* (its `count_defenders`
  gate is polarity-inverted: it fires the +5000 bonus on **defended** victims and misses free
  ones), so prior maxn-path numbers and the beam-4 bootstrap corpus were measured under a buggy
  heuristic.
- **Lever / change:** both flags moved from consts to `OrderState` fields, **default off**,
  settable via `Searcher::with_ffa_bounty_order` / `with_free_capture_order`. `order()` switched
  from `sort_by_key` to `sort_by_cached_key` (stable, identical order; prerequisite so EXP-021's
  cost measurement isn't inflated by the sort re-invoking `score`). Maxn path only —
  `search_flashlight` never calls `move_order` (EXP-017/018 results are clean).

## Conditions (before)

- Deployed eval weights `(6,0,0,1)`; suite green 112 lib + 3 integration; corpus = 32 human games.
- Both ordering flags hardcoded ON — the de-facto baseline under which every recorded maxn number
  was measured (EXP-015's move-agreement figures, blunder rates, the 133-game beam-4 bootstrap
  corpus in `selfplay_games/`).
- `count_defenders` (move_order.rs): polarity inverted (`p.player != victim_player`),
  adjacency-only radius-1 scan, dead `pawn_deltas` scaffolding. Called only inside the
  free-capture block — the bounty term has no identified defect; the two levers are independent.
- Maxn play shape (all arms, fixed): forward pruning ON, adaptive beam ON, no node budget.

**Golden references (pre-refactor const-true binary, release):**

- `bench_beam` (start position, depth 8): beam 30 = 34,831,298 nodes / 488.7s (best j1-i3) ·
  beam 20 = 14,599,071 / 206.6s (j1-i3) · beam 15 = 4,096,250 / 58.0s (j1-i3) ·
  beam 10 = 1,064,597 / 14.8s (d2-d3) · beam 8 = 448,764 / 6.1s (i2-i4) ·
  beam 6 = 157,904 / 2.0s (j2-j3).
- `move_match` (beam 10, depth 4, every 2 plies, 32 games): **347/2530 = 13.7%**.
  (Historic 13.5% was on the 16-game corpus — the 16→32 doubling re-anchors the absolute level;
  only within-run arm deltas are comparable.)

## Method

Three arms, fixed seeds, maxn path, beams 4 / 10 / 30:

| Arm | bounty | freecap | Meaning |
|-----|--------|---------|---------|
| (i) | on | on | de-facto historical baseline |
| (ii) | on | off | (i)−(ii) delta = the **contamination estimate** (isolates the buggy lever) |
| (iii) | off | off | Hard-Rule-#6 landing state — the new recorded baseline |

- **Equivalence gate first:** the refactored binary with both flags ON must reproduce the golden
  references *exactly* (node counts and matched/total), proving the refactor behavior-preserving
  before any arm is measured.
- `move_match [beam] [depth] [sample] [bounty] [freecap]` — 3 arms × beams {4, 10, 30}, depth 4.
  Beam-30 sample chosen by timing probe, held constant across its arms.
  **Reading the move_match deltas:** a between-arm delta is a **behavior-change frequency** (how
  often the lever changes the played move on corpus positions), *not* a quality signal — absolute
  move-match is dead as a tuning metric (EXP-004/005). Direction comes from self-play only.
- `selfplay_ab_maxn [a_beam] [b_beam] [a_bounty] [a_freecap] [b_bounty] [b_freecap] [depth] [games/split]`
  — new harness (selfplay_ab drives flashlight, which cannot exercise ordering): six balanced 2v2
  seat splits, deterministic seeds, 12 random opening plies, 140-ply cap, depth 8. Pairings:
  (i) vs (ii) — does the buggy lever change outcomes; (ii) vs (iii) — does the bounty lever.
- `move_diverge [beam] [depth] [sample] [a_bounty] [a_freecap] [b_bounty] [b_freecap]` — added
  mid-experiment: runs **both** configs on each sampled corpus position and counts differing
  choices. This is the direct per-position behavior-change (contamination) frequency, which
  match-rate deltas cannot show (two arms can match humans equally often while disagreeing with
  each other).

## Results

### Equivalence gate (flags-on refactored binary vs golden) — **PASS**

- `move_match 10 4 2 1 1`: 347/2530 = 13.7% — **exact** match to golden.
- `bench_beam 1 1`: **exact** node-count and best-move match at all six beams —
  34,831,298 / 14,599,071 / 4,096,250 / 1,064,597 / 448,764 / 157,904, moves
  j1-i3 / j1-i3 / j1-i3 / d2-d3 / i2-i4 / j2-j3 (times differ under CPU contention; only
  counts/moves are compared).

The const→`OrderState`-field refactor plus the `sort_by_key`→`sort_by_cached_key` switch is
behavior-preserving with flags on.

### move_match arms (matched/total vs the human move)

| Beam | (i) on/on | (ii) on/off | (iii) off/off |
|------|-----------|-------------|---------------|
| 4 | 340/2530 = 13.4% | 339/2530 = 13.4% | 342/2530 = 13.5% |
| 10 | 347/2530 = 13.7% | 344/2530 = 13.6% | 343/2530 = 13.6% |
| 30 (sample=2) | 345/2530 = 13.6% | 345/2530 = 13.6% | 345/2530 = 13.6% |

Net human-agreement is **insensitive** to the levers at every beam (±4/2530 across the whole
matrix; the beam-30 row is identical across arms) — which is exactly why the divergence
instrument below was added. The depth-4 beam-30 cost turned out low (~1.6 s/position), so the
beam-30 row uses the same full sample=2 as the other rows.

### Move divergence between arms (per-position behavior-change frequency, depth 4)

| Pairing | Lever isolated | Beam 4 | Beam 10 | Beam 30 (sample=4) |
|---------|----------------|--------|---------|--------------------|
| (i) vs (ii) | free-capture (the buggy one) | **294/2530 = 11.6%** | 24/2530 = 0.9% | 7/1270 = 0.6% |
| (ii) vs (iii) | FFA bounty | 45/2530 = 1.8% | — | — |
| (i) vs (iii) | both (total) | 326/2530 = 12.9% | — | — |

**The contamination estimate: at beam 4 the inverted free-capture heuristic changed the played
move on ~11.6% of corpus positions** (~1 in 9) — the per-move taint rate of the beam-4 bootstrap
corpus. The effect collapses at wider beams (0.9% at beam 10, 0.6% at beam 30): with a narrow
beam an ordering perturbation pushes moves *out of the expanded set*, while at wide beams it
mostly reorders within it. This empirically confirms STATUS's "wide-beam maxn runs mildly
affected at most" and the corpus-regeneration requirement (B5). The bounty lever is an order of
magnitude milder (1.8% at beam 4).

### Self-play pairings (beam 4, depth 8, 12 games each — six 2v2 splits × 2)

| Pairing | A pts | B pts | per seat | A win-rate | Decisive |
|---------|-------|-------|----------|-----------|----------|
| (i) vs (ii) — freecap lever | 228 | 223 | 9.5 vs 9.3 | 5/12 = 42% | 1/12 |
| (ii) vs (iii) — bounty lever | 282 | 240 | 11.8 vs 10.0 | 6/12 = 50% | 4/12 |

*(12 games is directional, not significant — treat point/win-rate gaps as a sign, not a
magnitude.)* The buggy freecap lever shows **no measurable strength contribution** (42% win-rate
with it ON — noise around even), so landing it off costs nothing. The bounty lever trends mildly
positive (+1.8 pts/seat) but at even win-rate — not close to a gate-pass; it stays default-off
and may earn default-on later through a properly powered self-play gate. Beam-30 self-play was
dropped: at 0.6% divergence the pairing cannot resolve anything in 12 games, and the per-game
cost at beam 30 is prohibitive (the divergence row already answers the wide-beam question).

## Conditions (after)

- Both ordering flags **default off** in `OrderState::new()` (guard test
  `ordering_levers_default_off`); enabled only via the two `Searcher` builders.
- `protocol/mod.rs` builds `Searcher` with defaults → `go` now plays the landing state (flags
  off) with no protocol change.
- Arm (iii) numbers are the **new recorded baseline** — re-baseline anything that compares against
  pre-2026-06-10 maxn figures (Kimi C2's move-agreement reads in particular).
- The beam-4 bootstrap corpus (`selfplay_games/`) remains tainted until regenerated (B5).
- `count_defenders` itself is untouched by this experiment; its fix-or-delete is EXP-021.

## Conclusion

**Confirmed, with the effect localized more sharply than hypothesized.** The inverted
free-capture heuristic was outcome-affecting almost exclusively at narrow beams: 11.6% of played
moves changed at beam 4 vs 0.9%/0.6% at beams 10/30. Concretely:

1. **The beam-4 bootstrap corpus is tainted at ~1 move in 9** — regenerate before tuning on it
   (B5). EXP-015's beam-10-era numbers and wide-beam runs are only marginally affected;
   flashlight results were never affected (no `move_order` calls).
2. **Landing both flags off costs nothing measurable** — the buggy lever's self-play contribution
   is noise (42% win-rate), and human-agreement is flat across all arms. Hard Rule #6 is restored
   with no strength regression.
3. **Arm (iii) is the new recorded baseline:** move_match 13.5% / 13.6% / 13.6% at beams
   4/10/30 (32-game corpus, depth 4, sample 2). Kimi C2 should read move-agreement against these,
   not the pre-flip figures.
4. The bounty lever's mild positive trend (+1.8 pts/seat, n=12) is a candidate for a future
   properly-powered gate, not a reason to ship it on.

Next: EXP-021 — fix `count_defenders` polarity (`is_attacked_by` scan) and decide fix-vs-delete
on the pre-ratified 10% nodes/sec rule.
