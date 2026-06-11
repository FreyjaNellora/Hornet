# EXP-021 — count_defenders fix (real attack scan): polarity + measured cost

- **Date:** 2026-06-10
- **Hypothesis:** replacing the inverted, adjacency-only `count_defenders` with a real attack scan
  (`board::attacks::is_attacked_by`) fixes the free-capture lever's polarity at a per-node cost
  small enough to keep the lever (pre-ratified rule: median nodes/sec drop > 10% → delete the
  lever and the scan outright).
- **Lever / change:** `move_order::count_defenders` → `is_defended(board, sq, victim_player)` =
  `is_attacked_by(board, sq, victim_player)` — "does the victim's own side defend the landing
  square." The old code checked `p.player != victim_player` (counted the capturer's pieces and
  bystanders as *defenders* — backwards) over only the 8 adjacent squares (missed every distant
  defender), and carried a dead single-arm `pawn_deltas` match (removed). The free-capture lever
  itself remains **default off** (EXP-020); this fixes what the lever does when enabled.

## Conditions (before)

- EXP-020 landed: both ordering flags default-off `OrderState` fields; `order()` uses
  `sort_by_cached_key`, so `score` — and any board scan inside it — runs **exactly once per move
  per sort** (measuring on the old `sort_by_key` would have inflated the scan cost by the sort's
  O(n·log n) key re-invocations; Opus review fold-in).
- Suite green 114 lib + 3 integration, including the new polarity regression test
  (`free_capture_bonus_prefers_undefended_victim`): Red knight can take an undefended Blue knight
  (f6) or a Blue knight defended by a **distant rook** down an open file (d6), with a **non-Blue
  piece adjacent** to the undefended victim (Yellow pawn g7). The old code fails this both ways
  (adjacency-only → rook invisible → ranked the defended capture free; inverted polarity → the
  bystander suppressed the genuinely free one). The fixed scan ranks the free capture first.
- Known accepted limits of the scan (ordering-only heuristic): ignores discovered defense
  (capturer vacating its square can unblock a defender's line); counts pinned defenders at face
  value; EP captures (victim square empty) get no bonus.

## Method

`bench_beam [bounty] [freecap] [mid] [depth]` mid-game mode: five seeded mid-game positions
(24 random opening plies from start via the `Game` driver, fixed seeds — capture-rich; the start
position has no captures so it cannot exercise the scan), search at depth 8, **beam 30**,
fwd-pruning + adaptive beam on. Sequential runs on an otherwise idle machine:

- `bench_beam 0 0 1 8` — freecap OFF (baseline nodes/sec, no scan)
- `bench_beam 0 1 1 8` — freecap ON (fixed scan; one `is_attacked_by` per scored capture)

Metric: **median nodes/sec** across the five positions, off vs on. Nodes/sec isolates per-node
cost from tree-shape change (with the lever on, ordering legitimately changes the tree, so node
*counts* are expected to differ). Decision rule (ratified in the dispatch plan entry before any
number was seen): median nodes/sec drop > 10% → **delete** the lever, the scan, and the bonus
path; ≤ 10% → keep as a default-off lever.

## Results

**Venue deviation (recorded):** the ratified venue was beam 30, but the first OFF-arm positions ran
30–47 min *each* (194M/128M nodes — 5×2 positions ≈ 4–5 h), while their nodes/sec was already
stable (68,097 / 68,418, 0.5% spread). Per-node ordering cost is **beam-independent** — `order()`
sorts *all* moves at a node before beam truncation — so the run was re-venued to **beam 10**
(~20–90 s/position), keeping depth 8 and all five positions. Cross-check: the beam-10 OFF
nodes/sec (63.8k–69.5k) brackets the banked beam-30 OFF numbers, confirming venue equivalence.

| Position | OFF nodes/sec | ON nodes/sec | Δ |
|----------|--------------|--------------|---|
| 1 | 69,456 | 69,906 | +0.6% |
| 2 | 64,337 | 65,813 | +2.3% |
| 3 | 63,782 | 63,757 | −0.0% |
| 4 | 66,641 | 67,444 | +1.2% |
| 5 | 64,318 | 64,766 | +0.7% |
| **median** | **64,337** | **65,813** | **+2.3% (noise)** |

Best moves identical OFF vs ON on all five positions; node counts differ slightly (1.4M–6.1M,
±0.1–6% — legitimate tree-shape change from ordering). **No position shows any slowdown**; the
median is inside run-to-run noise. The scan is cheap because it runs only on scored captures
(a small fraction of moves per node), short-circuits on the first defender found, and per-node
cost is dominated by leaf evaluation (always-recompute line projection) anyway.

## Conditions (after)

- `count_defenders` (inverted, adjacency-only, dead scaffolding) is **gone**; `is_defended` =
  `board::attacks::is_attacked_by` is the free-capture gate. Polarity regression test
  (`free_capture_bonus_prefers_undefended_victim`) pins the fix.
- **Lever kept, default off** (≤10% rule: measured ~0% cost). It remains an EXP-020-gated lever:
  enabling it in play requires a properly powered self-play arm showing strength gain — the
  *corrected* lever has never been measured for strength, only for cost.
- The `sort_by_cached_key` rescue reserved in the plan was never needed (it was made the baseline
  in EXP-020's refactor, before any cost was measured).

## Conclusion

**Fix kept — the honest-delete branch was not triggered.** The real attack scan costs nothing
measurable at the ordering call site (median +2.3%, i.e. noise), so the free-capture lever now
does what its name says at zero per-node cost. It stays default-off: cost was the EXP-021
question; strength is a future, properly powered EXP-020-harness question. Next consumers:
B5 corpus regeneration (now on corrected ordering when its config is decided) and any future
free-capture strength gate.
