# Search-theory research — established names for what we derived, and forms to try

General game-tree / search theory (no 4PC specifics), sourced. The point: most of what we've been
deriving by hand already has a name, a formula, and decades of results — which gives us forms to borrow
instead of reinvent.

## 1. "Depth makes it worse" = **game-tree / search pathology** (our EXP-016, named)
- *"deeper minimax search results in worse play."* And the mechanism is exactly our diagnosis:
  *"minimaxing **amplifies the noise** introduced by the heuristic function used to evaluate the leaves
  of the game tree, leading to pathological behavior, where deeper searches produce worse evaluations."*
  ([chessprogramming: Search Pathology](https://www.chessprogramming.org/Search_Pathology))
- **Directly relevant to us:** there is a paper titled *"The multi-player version of minimax displays
  game-tree pathology"* — i.e. **Max^n is *known* to be more pathology-prone than 2-player minimax.**
  ([Springer](https://link.springer.com/chapter/10.1007/3-540-54563-8_70))
- *"every game should have some sections that are locally pathological"* — it's position-local, not
  global (matches our bimodal cap data). ([UMD, Zuckerman et al. 2018](https://www.cs.umd.edu/~nau/papers/zuckerman2018avoiding.pdf))
- **Technique offered — Error-Minimizing Minimax:** *"recognizes pathological subtrees ... and cuts off
  search accordingly (shallower search is more effective than deeper search in pathological subtrees)."*
  → in noisy subtrees, **search *shallower*, not deeper.** ([Semantic Scholar](https://www.semanticscholar.org/paper/Error-Minimizing-Minimax-:-Avoiding-Search-in-Game-Wilson-Parker/c714346099a0b35d9eadb1af202a6e789e8ce183))

## 2. Our "noise / separability" = the **singular margin** (singular extensions)
- *"A move is singular ... if the value returned by a d-ply search of that move is better than the d-ply
  values of all siblings of it by a significant amount called the **singular margin**."*
  ([chessprogramming: Singular Extensions](https://www.chessprogramming.org/Singular_Extensions))
- This **is** our separability signal, already formalized: a clear best move = large margin = *sharp*;
  bunched siblings = small margin = *quiet*. Engines detect it with a reduced-depth null-window search.
- Note: *"the singular margin is wider with bigger depth"* — the margin itself scales with depth (our
  depth-compounding term shows up here too).
- **For us:** narrow the beam where the move is singular (large margin), widen where it isn't. The margin
  is a ready-made, tested noise measure.

## 3. Our "adaptive cap rate" = **progressive widening** (MCTS) — and it gives the FORM
- *"child expansions are scheduled as a function of node visit counts, limiting the effective branching
  factor."* The width law: **`|A(s)| = c · N(s)^α`**, where `c, α` *"control the rate at which the action
  space is widened"* and `N(s)` is the visit count.
  ([Action Progressive Widening](https://www.emergentmind.com/topics/action-progressive-widening-apw),
  [Progressive Strategies for MCTS](https://www.researchgate.net/publication/23751563_Progressive_Strategies_for_Monte-Carlo_Tree_Search))
- **For us:** this is the principled answer to "what rate to widen by." Our cap can follow a power law
  `cap = c · (effort)^α` (effort = depth/visits), with `c, α` tuned — instead of a hand-picked schedule.

## 4. Our beam (keep top-W, hard-drop the rest) ≈ **Late Move Pruning**, and **LMR** softens it
- LMP *"entirely skips searching late-ordered quiet moves if the number of previously searched moves
  exceeds a threshold."* That's our hard beam cut. ([chessprogramming: LMR](https://www.chessprogramming.org/Late_Move_Reductions))
- **LMR is the softer, safer version:** *"search the first few moves at full depth, then ... the
  remaining moves are reduced in search depth, and only re-searched if the reduced search fails high."*
- **For us:** our hard drop is what causes the dropped-best-line pathology. An LMR-style **reduce-and-
  re-search** (search pruned candidates *shallower*, promote any that surprise) would keep cost down
  while recovering the rare best line the hard cut loses — likely fewer pathology errors than a flat cap.

## What to try (ranked by fit)
1. **Singular margin as the noise signal** for the adaptive cap — narrow when a move is clearly singular,
   widen when siblings are bunched. Reuses a proven detector instead of inventing one.
2. **Progressive-widening law for the cap:** `cap = clamp(c · effort^α, floor, k·max_branching)` — the
   power-law form gives a principled rate (`α`) and our hard ceiling stays the clamp.
3. **LMR-style soft beam:** replace the hard top-W drop with reduce-late-then-re-search — directly
   attacks the "best line pruned" cause of the depth-pathology (EXP-016 cause 1).
4. **Error-minimizing idea:** in high-noise (low-margin, quiet) subtrees, *don't* spend depth — deeper
   amplifies noise there. Pairs with the win/danger objective layer (which lowers the eval noise that
   drives the pathology in the first place).

Sources inline above.
