# Cold pitch — develop new 4PC eval features WITHOUT depending on human games

## The problem
Hornet's search can go deep, but depth doesn't change the move it plays — the **evaluation** decides the
move, so the eval is the bottleneck. The deployed eval is essentially **material + crossfire (SEE
material-at-risk) only**; its "positional" and "king-safety" components are weighted **0**, because when
tuned they didn't improve play.

Every positional feature tried so far tuned to **weight 0**: piece-square tables (center-, edge-, and
per-piece variants), 2×2 zone control, piece mobility, development tempo. Reweighting the existing
components is exhausted — a *different kind* of feature is needed (relationships between pieces, pawns,
lines, the king — not per-square tables).

## The real constraint — this is the research question
We have only a small set of strong-human games and they collect **slowly** (captured by hand, one at a
time). So the question is **not just "which features"** — it's:

> **What is the best way to come up with, tune, and validate new eval features WITHOUT relying on a
> human-game corpus — and is that even possible?**

A feature's *definition* can come from theory; the hard part is **deriving its weight and proving it
helps with no human reference to match against.** That method is the heart of this pitch.

## What exists, and the catch
- A **self-play harness**: the engine can play itself and generate unlimited games for free.
- **Catch:** current self-play is **drawish** — games run to the move cap with no eliminations — so their
  outcomes are weak labels. Outcome-based tuning has little to learn from decisionless games.
- Existing tuners that match the engine to *human* moves require human games — the dependency we're
  trying to avoid. `texel_tune` fits to game *outcomes* (works on self-play too, **if** those games are
  decisive).

## Constraints (4PC, not 2-player)
- Eval returns a 4-vector `V = <M,P,S,O>` (material, positional, safety, crossfire), one per player; new
  signal folds into one of those four — **no 5th component**.
- 14×14 cross board, four 3×3 dead corners; **four players**, each a different forward direction; a
  pawn's "lane" is **perpendicular to its forward direction** (Red/Yellow → file, Blue/Green → rank).
- The **king is capturable** (capturing it eliminates a player; no checkmate-only terminal). The eval is
  currently **points-blind** (it does not value eliminating a player).

## The job
1. **Research how engines develop eval features without human games** — self-play tuning, automated
   parameter tuning (e.g. SPSA), reinforcement-learning / zero-style self-play, fitting to self-play
   outcomes. How do you *define* a feature, *derive its weight*, and *prove it helps* when you have no
   human reference? Use the literature; cite your sources.
2. **Confront the drawish-self-play problem head-on.** If self-play is decisionless, what gives you a
   usable training/validation signal? Solve it, or determine honestly that some human/external anchor is
   unavoidable (and how little of it you'd need).
3. **Propose + build 4PC eval features AND the human-free development loop** to tune and validate them.
   Implement, tune, and gate them by whatever human-free signal your research concludes is best.

## The bar
A feature **and** a development loop that measurably improves play *without* leaning on human games —
ideally demonstrated by self-play strength (one config beating another head-to-head). A clean "human-free
development isn't viable here, and here's the minimum external data actually required" is also a valid,
valuable result.

Start from `src/eval.rs`, `src/queries.rs`, and the self-play harness. Bring your own design and your own
method — no answer is handed to you here, on purpose.
