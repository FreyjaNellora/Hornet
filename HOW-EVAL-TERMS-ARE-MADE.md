# How chess-engine eval terms are made and defined (research) — and what it means for 4PC

Researched from the Chessprogramming wiki + Texel's method (sources at bottom). Goal: understand how
relational eval terms are actually *constructed, defined, and weighted* in real engines, so Hornet's
4PC versions are built right.

## 1. The anatomy of an eval term — 4 layers
Every hand-crafted term is the same pipeline:
1. **Chess concept** (intuition): "an isolated pawn is weak", "a knight nobody can kick is strong".
2. **Computable predicate** (the *definition*): a geometric/logical test over piece+pawn relationships,
   classically a bitboard operation. This is the part people mean by "how it's defined."
3. **Feature value**: the predicate yields a *count* or *boolean* (e.g. # isolated pawns, is-outpost).
4. **Weight**: the feature value × a centipawn weight (or a table lookup) → its eval contribution.

The "art" is layer 2 (a crisp, cheap predicate) and the "science" is layer 4 (fitting the weight to
data, not guessing). Almost everything is **relational** — defined by a piece's relationship to *pawns*
(own and enemy), to *files/ranks*, or to the *king zone* — which is exactly the class we haven't tested.

## 2. The definitions (sourced) — predicate patterns to copy
- **Isolated pawn**: no friendly pawn on the adjacent files. → penalty.
- **Doubled pawn**: ≥2 friendly pawns on a file. → penalty.
- **Backward pawn** (Kmoch): a half-free pawn whose *stop square* lacks pawn protection but is
  controlled by an enemy "sentry" pawn. *The wiki itself calls this "ambiguous"* — a warning that some
  concepts resist a clean predicate.
- **Outpost**: a minor that is (a) **defended by a friendly pawn**, (b) **can never be attacked by an
  enemy pawn** (no enemy pawn on adjacent files ahead of it), (c) on the opponent's half / half-open
  file. Bonus ~10–16cp, **larger if two pawns defend it or the opponent has no minor to trade it off**.
- **Rook on open file**: no pawns on the rook's file → bonus ~8–20cp; **semi-open** (only enemy pawns) →
  about half.
The shared shape: a term is a relationship to **pawns and lines**, and the bonus is **conditional**
(more when better-supported / less tradeable). Conditionality matters — see §3.

## 3. The most important finding: terms aren't all linear — king safety is a NON-LINEAR table
King safety is the canonical relational term and it's built differently from "count × weight":
1. Define a **king zone** (the king's squares + 2–3 squares facing the enemy).
2. **Count attackers** and accumulate **attack units** weighted by piece type — minor = 2, rook = 3,
   queen = 5 (Stockfish), plus units for **safe checks** (+6 for a safe queen check).
3. **Look the accumulated units up in a NON-LINEAR table** — an S-curve (Glaurung: 0 at low indices →
   ~650cp at index 80+). It rises slowly, then steeply, then flattens.
4. Add **pawn shelter / storm** (missing pawns in front of the king → penalty; enemy pawns advancing →
   penalty), scaled by enemy material (less material → safety matters less; encourages trades).

**Why the table:** "the whole is greater than the sum of the parts" — two attackers are *much* more than
twice one attacker. A single linear weight **cannot** express that; you need a table (or a quadratic).
**This is the lesson for us:** our move_tune fits one linear weight per term. That's fine for pawn
structure / outposts / rook-on-line (genuinely ~linear), but **king safety and any "attackers compound"
effect need a table/quadratic construction**, tuned — not a linear weight. (Same idea as "non-linear
feature interactions" — the thing per-square tables and single weights both miss.)

## 4. How the weights are *derived* — you don't pick them, you fit them (Texel's method)
The standard since 2014 (Peter Österlund):
1. **Data**: ~64k fast games → ~8.8M **quiet** positions (taken from quiescence; exclude opening-book +
   mate-score positions), each labeled with the **game result** (1 / 0.5 / 0).
2. **Map eval → win probability** with a sigmoid `1/(1+10^(−s·K/4))`, K a scaling constant (~1.13) fit
   once to the existing eval.
3. **Cost** = mean squared error `E = Σ (result − sigmoid(eval))²`.
4. **Optimize** the weights to minimize E — Texel uses **local search** (per parameter: try +1, then −2,
   keep improvements; repeat until nothing improves). Gradient/Gauss-Newton also work.
This is *exactly* our `texel_tune` (now that the seat-order label bug is fixed) and the same idea as
`move_tune` (which swaps the outcome label for "did it pick the human's move").

**Documented pitfalls — they map 1:1 onto what we hit:**
- **Correlation ≠ causation**: the tuner can assign weird values to features merely *correlated* with
  winning (the wiki's example: a queen on b7 valued −128cp from the poisoned-pawn pattern). → why a
  feature can "tune well" yet not be real; why we cross-check move-match *and* outcome.
- **Independence**: many positions per game aren't independent — but enough *games* swamps it. → why we
  need game **count**, not just position count.
- **Can't learn what it can't represent**: tuning can't invent a concept the eval has no feature for. →
  why reweighting our existing components plateaued (P=0); the gain needs *new* features, not new weights.

## 5. The design loop (how a term actually gets adopted) + the modern alternative
- **Loop**: intuition → write the predicate → add as a *separate, ablatable* term → **tune its weight** →
  **A/B test for Elo / win-rate** → keep only if it measurably gains. "Sounds principled" is not the bar;
  *measured gain* is. (Our gates: move-match + corrected-texel + eventually self-play win-rate.)
- **Modern alternative — NNUE**: instead of hand-defining terms, a small net learns the relational
  patterns implicitly from millions of labeled positions; this is why Stockfish *removed* its classical
  eval in SF16. Hand-crafted = interpretable + data-cheap; NNUE = stronger but data-hungry. For 4PC we're
  on the hand-crafted path until the corpus is NNUE-sized.

## 6. What this means for Hornet (4PC)
1. **Definitions must be re-derived for 4PC geometry** (lane ⊥ forward; 3 opponents; cross board;
   capturable king) — done in RELATIONAL-TERMS.md.
2. **Build the right *shape* per term**: pawn structure / outpost / rook-on-line = linear (count × weight,
   tune with move_tune). **King safety = attack-units → non-linear table** (don't model it as one linear
   weight — it will under-read, like everything bundled did).
3. **Conditionality is part of the definition**: outpost-defended-by-two-pawns, rook open-vs-semi-open,
   shelter scaled by enemy material — encode the conditions, not just the base feature.
4. **Weigh by fitting, never by hand** — per-term weights (unbundled) via move_tune, confirmed on
   corrected-texel, ultimately A/B'd by self-play. Keep a term only if it earns its weight.
5. **The pitfalls are our open risks**: correlation≠causation (cross-check two gates), independence (need
   game count → keep collecting), representational gaps (this is why we add *new* relational features).

## Sources
- [King Safety](https://www.chessprogramming.org/King_Safety) · [Outposts](https://www.chessprogramming.org/Outposts) ·
  [Rook on Open File](https://www.chessprogramming.org/Rook_on_Open_File) ·
  [Isolated Pawn](https://www.chessprogramming.org/Isolated_Pawn) · [Doubled Pawn](https://www.chessprogramming.org/Doubled_Pawn) ·
  [Backward Pawns](https://www.chessprogramming.org/Backward_Pawns_(Bitboards)) · [Pawn Structure](https://www.chessprogramming.org/Pawn_Structure)
- [Texel's Tuning Method](https://www.chessprogramming.org/Texel's_Tuning_Method) · [Automated Tuning](https://www.chessprogramming.org/Automated_Tuning) · [Evaluation](https://www.chessprogramming.org/Evaluation)
