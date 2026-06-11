# Pitch — Max^n shallow pruning (and real TT bound cutoffs)

Hornet is a four-player-chess engine in Rust (`hornet-engine/`). Search is **Max^n**: every node
maximizes the **moving player's own component** of a per-player utility vector
`V = ⟨U₁,U₂,U₃,U₄⟩`; the vector is backed up whole, never collapsed to a scalar. The search runs
today, full-width at the root and beam-limited (top 30) at interior nodes, but it does **no value
pruning at all** — it expands every move in the beam to the horizon. This is roadmap item #1 in
`ENGINE-HANDOFF.md`: add **Max^n shallow pruning** (the main speed win), which also lets the
transposition table cache *real bounded values* instead of being used only for move ordering.

Your job: make the search cut off branches that provably cannot change the parent's choice. The
algorithm is standard; the one real decision — **where the pruning bounds come from** — is yours,
and it's described in full below so you can choose with eyes open.

## Read these first (verify everything yourself)

- `hornet-engine/src/search.rs` — the searcher. `maxn` (the recursion you'll modify) is at
  `:94`; the full-width root loop at `:64–92`; the interior beam cutoff `.take(self.beam_width)`
  at `:113`; the `nodes` counter at `:35`. Note the builder pattern you should mirror for the
  new toggle: `with_beam_width` (`:50`) and the test-only `with_eval` (`:57`). Depth is forced
  to a multiple of 4 by `round_to_rotation` (`:138–142`).
- `hornet-engine/src/tt.rs` — the transposition table. The `Bound` enum (`Exact`/`Lower`/`Upper`)
  already exists (`:12–17`) but is **never used for cutoffs** — every store today passes
  `Bound::Exact` and the table is read for the best-move hint only (`search.rs:72` and `:107`).
  `store` is depth-preferred replacement (`:87–106`); `probe` is exact-key (`:80–83`).
- `hornet-engine/src/eval.rs` — the evaluator. `eval_4vec` (`:32`) returns `[i16;4]`. The v0
  weights are `W_MATERIAL=1, W_POSITIONAL=2, W_SAFETY=1, W_CROSSFIRE=1` (`:16–19`); the utility
  is `Uᵢ = Mᵢ + 2·Pᵢ + Sᵢ − Oᵢ` in `compute_utility` (`:49–58`). **This is the function whose
  output range determines your bounds.**
- `hornet-engine/src/queries.rs` — where the four components come from, with their ranges:
  - **Material `Mᵢ`** (`:59–68`): sum of piece `eval_value` — pawn 100, knight 300, bishop 450,
    rook 500, queen 900, king 0. Start = 4200/player, 16800 total on board.
  - **Positional `Pᵢ`** (`:84–94`): centrality-weighted (0–5, `:74–81`) count of empty squares
    each piece reaches. Not bounded by material; scales with piece count × reach.
  - **Safety `Sᵢ`** (`:134–183`): `defenders − attackers + escapes`; can go **negative**.
  - **Crossfire `Oᵢ`** (`:190–215`): ≥ 0, **subtracted**, so `−Oᵢ ≤ 0`; can be large.
- `hornet-engine/src/move_order.rs` — MVV-LVA via a **stable sort** (`:9–11`). Ordering is
  deterministic given the same board + TT move; rely on this for an exact ablation test.
- `HORNET-BUILD-SPEC.md` §6 (Search Contract, `:608`) and §6.4 (TT carries an exact/lower/upper
  flag). **The spec does not define the pruning algorithm or any bounds** — that's why this is a
  design task, not a transcription.
- `TECHNIQUES-and-REFERENCES.md` → "Forward pruning via bounded utility components (shallow
  pruning)" (`:45–55`): the technique and the two papers — Sturtevant & Korf 2000, *On Pruning
  Techniques for Multi-Player Games*; and Korf, *Multi-player alpha-beta pruning*.

## The algorithm (shallow / immediate pruning)

Shallow pruning in Max^n is provably sound given two bounds over every reachable leaf:
`SUM_UB ≥ U₁+U₂+U₃+U₄` and `COMP_LB ≤ Uᵢ` for each i.

Thread **one** extra pair into the recursion: the parent's moving player `p` and `alpha` = the
best value of component `p` the parent has secured from an already-searched sibling. At a child
node where player `q` moves, track `best_q` = the max `[q]` component seen so far among its
expanded sub-children. Then the most the parent can ever extract for itself from this subtree is

```
UB_p = SUM_UB − best_q − 2·COMP_LB        // the 2 = the two players that are neither p nor q
```

because the sub-child the node will actually return maximizes `[q]` (so `[q] ≥ best_q`), the four
components sum to ≤ `SUM_UB`, and the other two each ≥ `COMP_LB`. If `UB_p ≤ alpha`, this subtree
can't beat what the parent already holds → **stop expanding this node's remaining moves**. The
value it returns is then an upper bound on `[p]` (tag the TT entry `Upper`, not `Exact`).

Two non-negotiables:
- **Immediate parent only.** Use the *direct* parent's `alpha`. Threading a bound across multiple
  ancestors ("deep pruning") is unsound in general Max^n — don't.
- **Sound or off.** A prune must never cut a branch that could change the parent's choice. Under
  valid bounds, pruning-on must return the **same root move** as pruning-off (only fewer nodes).
  That equality is your correctness oracle.

## The decision you own: where do the bounds come from?

This is the whole game. With loose bounds the prune test never fires; with tight bounds it fires
often. The trap: a *naïve provable* `SUM_UB`/`COMP_LB` for this eval is enormous (positional is
unbounded-ish, crossfire/safety push `COMP_LB` deeply negative so `−2·COMP_LB` adds huge slack),
so `UB_p` stays far above any realistic `alpha` and you get **zero cutoffs**. Three honest ways
out, in increasing aggressiveness — pick one (and wire it default-off regardless):

1. **Provably-sound static bounds.** Conservative constants derived from the eval's structure.
   Airtight, lands fast, but prunes little until tightened. Good if you want the machinery
   (cutoff, `Bound` tagging, toggle, ablation test, TT value-cutoffs) correct and measurable
   first, with bound-tightening as a follow-up.
2. **Measured / clamped bounds.** Measure component min/max and the max sum over the real PGN4
   corpus (positions exercised by `hornet-engine/tests/pgn4_replay.rs`; tactical fixtures in
   `baselines/tactical_samples.json`), add a margin, and **clamp the eval into that envelope** so
   prunes stay sound on outliers. Real cutoffs immediately. Cost: a measurement pass, and the
   clamp slightly changes the eval at extreme positions — so pruning-on is no longer byte-identical
   to pruning-off there (the clamped eval becomes the eval). The ablation arm is how you justify it.
3. **Position-derived bounds.** Compute the bounds per node from the live board: material left
   (+ promotion slack — see the landmine below) plus structural caps on positional/safety/crossfire.
   Fully sound, no eval change, real cutoffs on material-swingy lines; quiet positions still prune
   little because the non-material slack stays loose. Most implementation effort.

Whatever you choose, **measure it**: report nodes pruned-on vs pruned-off at a fixed depth on a
handful of real positions. If the number is ~0, the bounds are too loose — say so rather than
shipping a no-op.

## Constraints (engine design rules — honor all)

- **Default-off + ablation arm.** New strength/speed levers ship disabled. Add a builder toggle
  mirroring `with_beam_width` (e.g. `with_shallow_pruning(bool)`), default off. There is no
  existing feature-flag infra — this is the first; keep it simple and reusable.
- **Vector, never scalar** (Hard Rule #3). Back up the whole `[i16;4]`. Pruning changes *which*
  branches you visit, never the backup rule.
- **Depth stays a multiple of 4** (Hard Rule #1) — already handled; don't disturb it.
- **Determinism.** Ordering is a stable sort; keep results reproducible so the ablation equality
  test is exact.
- **TT correctness.** If you start consuming TT values for cutoffs (the "stop being
  ordering-only" half of this item), gate on stored `depth` and respect the `Bound` flag — an
  `Upper`/`Lower` entry is not an exact value. Today the table is ordering-only precisely because
  beam values are approximate; bounded values are what make value-cutoffs legitimate.

## Done looks like

- A prune cutoff in `maxn` that fires under the chosen bounds and measurably **drops the node
  count** versus pruning-off at the same depth.
- Root move **identical** pruning-on vs pruning-off under sound bounds (option 2 documents its
  extreme-position exception).
- TT entries tagged `Exact` when a node is fully expanded, `Upper`/`Lower` when it was pruned;
  optionally consumed for depth-gated, bound-respecting cutoffs.
- New tests: one proving the prune triggers (construct a position / synthetic eval where `best_q`
  forces `UB_p ≤ alpha`), one proving value-preservation (same root move with/without). Mirror the
  existing `with_eval` synthetic-eval tests in `search.rs:218–264`.
- `cargo test` green (66 today). Perft regression unchanged (20/395/7800/152050). PGN4 replay
  still ≥ 2500.

## Landmines (found while scoping this)

- **Loose static bounds ⇒ 0 cutoffs.** Confirmed by inspection: `SUM_UB − 2·COMP_LB` dwarfs any
  realistic `alpha`. Don't be surprised; it's why the bound choice matters.
- **Promotion breaks material monotonicity.** A pawn → queen adds +800, so "material only
  decreases with depth" is false. A sound per-node material upper bound must add promotion slack
  (≤ `min(pawns, depth) · 800`). Relevant to option 3.
- **Beam already makes interior node values approximate** (`search.rs:113`). Shallow pruning is
  sound *within the beam-limited tree* it would otherwise search — reason about it against that
  tree, not the full game tree.
- **TT is ordering-only today.** `store` always passes `Bound::Exact` (`search.rs:89`, `:123`)
  and probes read `best_move` only. Turning on value-cutoffs is a real change with its own
  correctness surface — depth + bound gating.

## Verify

```
cd hornet-engine
cargo test                  # 66 green today; keep them green, add yours
```

The perft test (`move_gen.rs:395`) and the replay test (`tests/pgn4_replay.rs`) are your
regression guards — pruning must not change move generation or legal-move counts, only search
node counts.
