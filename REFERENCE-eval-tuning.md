# Reference — how classical engines tuned hand-eval weights (and what Hornet should copy)

The question: a hand-crafted eval has dozens of weights nobody knows the "right" value of. How did
Stockfish (and the classical-engine tradition) set them before NNUE? **Answer: they didn't pick the
numbers — they *fit* them to game outcomes.** The eval is a parametric model; the parameters are
learned from "who actually won," never from matching a specific move.

## The classical Stockfish eval (pre-NNUE, ~2020 and before)
- Dozens–hundreds of hand-crafted terms: material, piece-square tables, mobility, king safety, pawn
  structure (passed/isolated/doubled/backward), bishop pair, rooks on open files, threats, space.
- Each term a `(midgame, endgame)` pair, **tapered** (interpolated) by game phase.
- The *structure* (which features exist) was human expertise. The *values* were tuned, two ways:

### 1. Texel tuning — supervised fit to game results (the cheap, dataset method)
Source: Peter Österlund (Texel), 2014. ([chessprogramming wiki](https://www.chessprogramming.org/Texel's_Tuning_Method))
- Take a large game set (Texel used ~64k games → ~8.8M positions), each position **labelled with the
  game's eventual result** `R ∈ {0, 0.5, 1}`.
- Map the eval score `q` to a win probability with a sigmoid: `P = 1 / (1 + e^(−K·q/400))`. `K` is fit
  once to the data, then fixed.
- Cost = mean squared error: `E = (1/N) Σ (Rᵢ − sigmoid(K·qᵢ))²`.
- Optimize by **local search**: vary each parameter by ±1, keep the change if it lowers `E`, repeat
  until no improvement. Tunes hundreds of parameters at once; reported +100–187 Elo over untuned.
- The numbers are simply "whatever makes the eval best predict who won." You never hand-pick them.

### 2. Fishtest + SPRT + SPSA — fit to self-play Elo (the online method)
Source: [Stockfish Fishtest wiki](https://github.com/official-stockfish/fishtest/wiki).
- **Fishtest**: distributed framework; a candidate change plays thousands of games vs the baseline.
- **SPRT** (sequential probability ratio test): accept/reject a change with the *fewest* games,
  measuring whether it's a significant Elo gain.
- **SPSA** (simultaneous perturbation stochastic approximation): perturb many parameters at once,
  play games, step parameters toward what wins more. Online auto-tuning against game outcomes.

## The philosophy (the actual answer to "without knowing the numbers")
1. The eval is a **model fit to ground truth (game results)**, not derived analytically.
2. Only **ranking / relative values** matter; the sigmoid maps eval→probability. Absolute scale is
   convention (~100 cp = a pawn).
3. **Trust data, not intuition** — validate every change statistically.
4. **The target is always the *outcome*, never "match a specific move."** ← This is exactly why
   Hornet's move-match gate is noise (EXP-008): matching one 3000-Elo human's exact move was never the
   right objective. Stockfish would tune to "does this position lead to a win," not "did we play the
   same move."

## What Hornet should adopt (in order)
1. **Now — blunder rate** (built, `gate_ablation.rs`): a coarse sanity signal (does the engine's move
   lose material), tunable, uses corpus replay + SEE. Good enough to catch regressions while we
   recalibrate.
2. **Next — Texel tuning on our corpus** (the real fine-tune; directly from the playbook, uses data we
   already have):
   - Extract positions from the corpus games; label each by the eventual **per-player outcome**.
   - 4PC adaptation: the result isn't W/D/L — it's **placement (1st–4th) / points / elimination
     order**. Map each to a per-player target in `[0,1]` (e.g. normalised placement, or won = 1).
   - For each player, map its eval component `Uᵢ` through the sigmoid to a predicted place-probability;
     minimise MSE vs the actual outcome by local search over the weights (`W_MATERIAL…` and, later, the
     query sub-parameters). Use the quiescence (TRS) score for `q`, as Texel uses qsearch.
   - Output: weights that *predict 4PC outcomes*, set by data, not by hand.
3. **Later — self-play + SPSA** (true Elo): once a game loop exists, the gold-standard test, the way
   Fishtest works.

Bottom line: stop hand-reasoning weights against move-match. Build the outcome-labelled dataset from
the corpus and let Texel set the numbers — that is precisely how the hand-eval era did it.

Sources:
- [Texel's Tuning Method — chessprogramming wiki](https://www.chessprogramming.org/Texel's_Tuning_Method)
- [Automated Parameter Tuning in chess4j — jamesswafford.dev](https://jamesswafford.dev/automated-parameter-tuning-in-chess4j/)
- [Fishtest wiki (SPRT/SPSA) — official-stockfish](https://github.com/official-stockfish/fishtest/wiki)
- [Statistical Methods in Fishtest — Stockfish docs](https://official-stockfish.github.io/docs/fishtest-wiki/Fishtest-Mathematics.html)
