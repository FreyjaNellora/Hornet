# Hornet — Techniques and References

**Purpose:** Academic terminology, behavior summaries, and paper citations for
the techniques Hornet uses in search, evaluation, training, and validation.
This document complements `HORNET-BUILD-SPEC.md` — the spec tells you what to
build, this tells you what each technique is in the literature so you can
look up edge cases, performance tuning, and rigorous formulations as needed.

Also lists anti-patterns Hornet explicitly rejects, with the reasoning why.

---

## Search techniques

### Best-first selective search with forward pruning

Expand the most promising branches deeply, prune low-promise moves before
search descends, vary beam width across plies. Combined with late move
reductions and adaptive beam scheduling. Hornet's overall search identity.

### Late Move Pruning (LMP)

After move ordering, drop moves past a budget in the tail of the move list.
Hornet's specific instantiation adds a tactical-completeness guarantee:
captures, promotions, and checks are always preserved. Standard
chess-engine technique.

### Unified move-quality heuristic

A per-move score from a feature blend (policy logit + tactical score +
history table value) that feeds every search decision needing move ranking:
move ordering, late-move reductions, singular extensions, MCTS prior. One
signal, many consumers — engineering hygiene rather than algorithmic
novelty.

### Multi-player utility vector for Max^n

Evaluation returns a per-player utility vector V = ⟨U₁, U₂, U₃, U₄⟩; the
search backup rule preserves per-player components rather than collapsing
to a scalar. Standard for Max^n in N-player games.

**Reference:** Luckhardt & Irani 1986, *An Algorithmic Solution of N-Person
Games*.

### Forward pruning via bounded utility components (shallow pruning)

Compute per-player upper bounds on remaining utility from current state;
prune candidate moves that produce a V vector dominated for some player.
Provable in constant-sum games; works in bounded-sum games (like 4PC where
captures add a bounded number of points) with looser but still useful
bounds.

**References:**
- Sturtevant & Korf 2000, [*On Pruning Techniques for Multi-Player Games*](https://cdn.aaai.org/AAAI/2000/AAAI00-031.pdf), AAAI.
- Korf, [*Multi-player alpha-beta pruning*](https://faculty.cc.gatech.edu/~thad/6601-gradAI-fall2015/Korf_Multi-player-Alpha-beta-Pruning.pdf).

### Iterated Elimination of Strictly Dominated Strategies (IEDS)

At each node, build a dominance matrix over candidate moves; eliminate
strictly dominated moves (those worse than some alternative against every
opponent reply); recurse on the reduced beam. Strict dominance elimination
is order-independent, so the procedure is well-defined.

**References:**
- [*The Complexity of Iterated Strategy Elimination*](https://arxiv.org/pdf/0910.5107).
- Apt 2004, [*Uniform Proofs of Order Independence for Various Strategy Elimination Procedures*](https://arxiv.org/pdf/cs/0403024).

### Singular extensions

When one move's evaluation dominates its siblings, search that move one ply
deeper than the others. Standard chess search technique.

**Reference:** Anantharaman, Campbell, Hsu 1988, *Singular Extensions:
Adding Selectivity to Brute-Force Searching*.

### Quiescence search with multi-player rotation invariant

At search leaves, continue exploring only tactical moves (captures, checks,
forcing threats) until the position is "quiet." Hornet's version adds a
rotation-completion requirement: the quiescent leaf must land on a depth
that is a multiple of the number of players, so the perspective chain ends
at a full rotation boundary (see horizon-alignment invariant below).

**Reference:** Knuth & Moore 1975, *An Analysis of Alpha-Beta Pruning*.

### Hybrid alpha-beta + MCTS controller

Phase-dependent algorithm selection: Max^n / alpha-beta for shallow and
opening phases (large-beam, low overhead), MCTS for midgame phases (visit
reallocation toward root-relevant lines). Standard pattern in modern
engines.

### Horizon-alignment depth invariant (specific to N-player turn-rotational games)

Search depth must be a multiple of the number of players. Otherwise the
perspective chain ends mid-rotation and the leaf evaluation is
asymmetrically biased (root player has thought one more "turn" forward
than some opponents). For 4PC: valid depths are 4, 8, 12, 16, etc.

No canonical academic name found because most chess search literature
addresses 2-player games where the constraint is trivially satisfied. This
invariant is non-negotiable for correctness.

---

## Evaluation techniques

### Static line/ray projection per piece

Each piece on the board has its movement axes projected as rays (sliders)
or single-square attack patterns (jumpers). The result is a per-piece
record of every square the piece reaches: distance, first occupant, whether
the ray continues past the first occupant (for X-ray visibility on
sliders). Combined into a per-square inverse index ("which pieces reach
this square") used by tactical queries.

**Reference:** [*Vector Attacks*, chessprogramming.org](https://www.chessprogramming.org/Vector_Attacks)
— the mailbox-ray-walking primitive is the foundational technique for
non-bitboard slider attack generation. Hornet's line projection is this
primitive applied per-piece and indexed for tactical pattern queries.

### Tactical-pattern queries from line/ray data

Standard tactical detection (pin, fork, discovered attack, multi-attacker
convergence) read out of the precomputed line data. Each is a small read
over per-piece reach entries plus the per-square inverse index. No
deep search inside the eval; pattern detection is structural.

### Utility decomposition per player

```
Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ
```

- Mᵢ — material balance (cheap scalar)
- Pᵢ — positional control (line-coverage density, centrality-weighted)
- Sᵢ — king safety (friendly defenders − enemy attackers in king vicinity)
- Oᵢ — dominance / crossfire (subtracted; multiple opponents converging on
  player's pieces)
- w₁..w₄ — weights (hand-tuned at v0, NNUE-learned at v1)

The decomposition is structural: each component maps to exactly one query
class so the eval can't drift into miscellaneous bolted-on terms. Hand-
tuned starting weights provide a working evaluator before NNUE training
begins.

---

## NNUE techniques

### Dense MLP on structured distilled features

Network input is pre-computed structured features from queries (~50-100
features) rather than raw sparse piece-square binary indicators. Forward
pass is a plain dense MLP with no accumulator pattern.

Differs from canonical NNUE (Nasu 2018, Stockfish HalfKP) which uses sparse
binary inputs and incremental accumulator updates. Closer to AlphaZero /
KataGo input-representation philosophy, but with structured features rather
than raw planes.

**Reference:** Nasu 2018, [*Efficiently Updatable Neural-Network-based Evaluation Functions for Computer Shogi*](https://www.chessprogramming.org/NNUE) (the original NNUE paper, for context on what we're deviating from and why).

### Search-target value labels for training

Train the value head on position-level search-target centipawn labels (the
score the deeper search would return for this position), not on game
outcomes (which player eventually won). Standard supervised distillation:
student learns the teacher's per-position evaluation, not the eventual game
outcome.

**Reference:** Hinton, Vinyals, Dean 2015, [*Distilling the Knowledge in a Neural Network*](https://arxiv.org/abs/1503.02531).

### Teacher-quality gate before distillation

Don't initiate NNUE training until the teacher (the hand-tuned evaluator)
exceeds the strength bar the student is supposed to inherit. Standard
distillation hygiene — the student's ceiling is set by the teacher's
ceiling.

Hornet's specific bar: the hand-tuned evaluator should not be routinely
beatable by competent human players before being used as a training
teacher. Tactical fixture solve rates and human-vs-engine games are the
two signals checked.

---

## Validation methodology

### Ablation studies

Every new lever ships disabled by default and is measured both
independently (does this alone change strength?) and in combination (does
this lever still help in stack with the others?). Standard scientific
methodology applied to engine development.

### Counterbalanced experimental design (seat-fair measurement)

Strength measurements rotate seats to neutralize per-seat priors. In 4PC,
Yellow has a real third-mover advantage that is independent of evaluator
quality, so measurements without seat rotation systematically over-credit
Yellow strategies. Counterbalancing is standard methodology for any
turn-rotational game where seat assignment carries information.

### Round-trip parser validation

Format parsers (FEN4, PGN4) must round-trip: parse → serialize → parse
produces identical byte output. Tested against a corpus of real
chess.com 4PC PGN4 files. Catches encoding bugs before they propagate into
position-dependent code.

---

## Anti-patterns Hornet rejects

Each item below is something a reasonable design might include but Hornet
deliberately doesn't. Reason follows each.

### Multi-layer scalar feature pipeline ("swarm pipeline" style)

A pipeline of layered hand-tuned scalar features feeding each other in
non-linear chains. The structural problem: features within layers entangle
with non-linear interactions across layers, making the pipeline impossible
to ablate cleanly. You can't measure which layer is contributing what
because every layer's output feeds the next layer's input.

Hornet's structured V decomposition (Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ)
is decomposable by construction: each component traces to a specific query
class, ablation tests each independently.

### Hand-tuned monolithic scalar evaluation function

A single large function (thousands of lines) computing per-player scalar
scores by summing hand-coded terms with entangled interactions. Too coupled
to refactor, too opaque to tune, too tangled to extract clean component
priors from.

Hornet uses a four-component decomposition with one query class per
component. Tuning is per-component, not global. NNUE refines the
components separately too.

### Sparse binary piece-square features with HalfKP-style accumulator (on 14×14 board)

The canonical NNUE input architecture (~4,500 sparse binary indicators with
incremental accumulator updates) does not scale gracefully to 14×14 boards.
Bitboard-based techniques that make incremental updates fast on 8×8 (magic
bitboards, etc.) require board sizes that fit standard integer widths;
14×14 = 196 squares does not.

**Reference:** [Fairy-Stockfish maintainers' discussion on bitboard scaling beyond 12×10](https://github.com/fairy-stockfish/Fairy-Stockfish/blob/master/src/bitboard.cpp)
— boards larger than 12×10 "most likely need 256-bit bitboards" or non-magic
methods, neither of which is a clean drop-in.

Hornet uses dense MLP on structured features for this reason.

### Game-level value targets for NNUE training

Training the value head on the final FFA placement (who won the whole
game) rather than on per-position evaluations causes the value head to
learn seat bias rather than positional evaluation. The signal is too
noisy at the position level — every position in a winning game gets a
positive label regardless of whether it was actually winning at that
point.

Hornet uses position-level search-target centipawn labels (the deeper-
search evaluation for that specific position).

### Coalition dynamics as blended scalar weights

Paranoid + BRS + anti-leader + vulture opponent-modeling perspectives,
summed as weighted scalars into a single eval, collapses the per-
perspective information that downstream consumers (criticality signal,
search ordering, NNUE features) need. The weighted blend loses
"which perspective is dominating" — useful information that downstream
consumers can use if it's preserved.

Hornet treats each opponent perspective as a separate query type over
shared line data. The blend (if any) happens late, at the NNUE feature
layer where the network can learn weights, rather than early as
hand-coded constants.

### NNUE trained downstream of a weak evaluator

If the NNUE is trained on self-play games generated by a weak hand-tuned
evaluator, it inherits and magnifies that evaluator's blind spots: any
position class the evaluator mis-evaluates gets reinforced rather than
corrected during training, because the network learns that the
mis-evaluation is "winning."

The teacher-quality gate above is the mitigation: don't begin distillation
until the hand-tuned evaluator is strong enough that its blind spots are
narrow.

---

## Open problems for future work

Items deferred because they don't have clean answers yet.

### What makes a 4PC engine genuinely strong vs human-level

No public 4PC benchmark engine of known strength to duel against. Human-vs-
engine validation is the available signal but it's subjective and slow.
Tactical fixture solve rates give an objective signal but only on the
specific positions in the fixture suite.

### Heterogeneous query cost budget allocation

Some queries are cheap (material balance, basic capture enumeration); some
are expensive (multi-step BFS for tempo-aware tactical analysis). How to
allocate query time such that search depth stays predictable across
position types is an open question — answered partially by uniform query
cost (every query runs to fixed completion) but that leaves potential
strength on the table.

### NNUE generalization across game phases

Whether a single network handles opening / middlegame / endgame phases
well, or whether phase-specific networks win. Standard NNUE engines often
use a single net; the right answer for 4PC is unmeasured.
