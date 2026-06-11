# Sources — where our eval/search information comes from

External references behind the eval + search work, with **direct quotes**. Quotes were retrieved from
the linked pages via web fetch; click through to verify exact wording. (The 4PC *application* of any of
this is our own design — see HOW-EVAL-TERMS-ARE-MADE.md / RELATIONAL-TERMS.md / STOCKFISH-EVAL-MAP.md.)

## Stockfish — engine source + wiki
**Used for:** correcting the claim that SF's static eval has no threats (it does); the full classical
term list (→ STOCKFISH-EVAL-MAP.md); pawn-structure definitions.

- **evaluate.cpp, classical eval (sf_11)** — https://github.com/official-stockfish/Stockfish/blob/sf_11/src/evaluate.cpp
  - The static `threats()` function contains these named terms: **ThreatByMinor, ThreatByRook,
    ThreatByKing, ThreatByRank, Hanging, ThreatBySafePawn, ThreatByPawnPush, RestrictedPiece,
    WeakUnopposedPawn, KnightOnQueen, SliderOnQueen.** The `Hanging` penalty is `S(23, 20)`.
- **pawns.cpp (sf_11)** — https://github.com/official-stockfish/Stockfish/blob/sf_11/src/pawns.cpp
  - Pawn-structure terms: Isolated, Backward, Doubled, Connected (by rank), WeakUnopposed, WeakLever;
    plus the `ShelterStrength` / `UnblockedStorm` king-safety inputs.
- **Stockfish (wiki)** — https://www.chessprogramming.org/Stockfish
  - > "Stockfish 16, released June 30, 2023, removes the classical evaluation from the engine and focuses on NNUE neural networks."
  - > "Classical Evaluation (traditional hand-crafted evaluation) has been removed since version 16."

## Eval-term definitions — Chessprogramming wiki
**Used for:** how each relational term is *defined* (the predicate) and typical magnitudes.

- **King Safety** — https://www.chessprogramming.org/King_Safety
  - > "King zone is usually defined as squares to which enemy King can move plus two or three additional squares facing enemy position."
  - > "Stockfish counts each minor piece attack on a king zone ... as 2 attack units, rook attack on king zone as 3 attack units and a queen attack as 5 attack units."
  - > "Typical curve is S-shaped: it raises slowly at first, then it goes up faster, becoming almost flat at the end."
- **Outposts** — https://www.chessprogramming.org/Outposts
  - > "a chess term most often related to knights in the center or on the opponent's half of the board, defended by an own pawn, and either no longer attackable by opponent pawns at all"
  - > "squares on a half-open file on the opponent's half of the board, defended by own pawns"
- **Rook on Open File** — https://www.chessprogramming.org/Rook_on_Open_File
  - > "An open file is usually defined as a file with no pawns on it - a semi-open file as containing only the enemy pawns."
  - > "Bonuses applied to a rook on an open file vary from 8 to 20 centipawns."
  - > "Typical bonus for a semi-open file is half of that for a fully open file."
- **Isolated Pawn** — https://www.chessprogramming.org/Isolated_Pawn
  - > "a pawn with no pawns of the same color on the neighboring files"
  - > "Many programs tend to evaluate an isolated pawn in the center as weaker than on the wing, as it can be attacked from more directions"
- **Pawn Structure / Doubled / Backward** — https://www.chessprogramming.org/Pawn_Structure ·
  https://www.chessprogramming.org/Doubled_Pawn · https://www.chessprogramming.org/Backward_Pawns_(Bitboards)
  - Kmoch's backward-pawn definition: > "A half-free pawn on the second or third rank whose stop square lacks pawn protection but is controlled by a sentry." (the wiki notes this definition is "ambiguous").

## Tuning — how weights are derived
**Used for:** the weight-fitting method behind `texel_tune` / `move_tune`, and its pitfalls.

- **Texel's Tuning Method** — https://www.chessprogramming.org/Texel%27s_Tuning_Method
  - > "Ri is the result of the game corresponding to position i; 0 for black win, 0.5 for draw and 1 for white win."
  - Local search: > "for each parameter, try incrementing by 1, then decrementing by 2, accepting improvements" (the wiki's `localOptimize`).
  - Pitfall (correlation≠causation): a feature merely *correlated* with winning can get a weird weight (the wiki's example: a queen on b7/g7 valued ≈ −128cp from the poisoned-pawn pattern).
- **Automated Tuning** — https://www.chessprogramming.org/Automated_Tuning
- **Evaluation** — https://www.chessprogramming.org/Evaluation

---
*Quotes retrieved via web fetch from the linked pages; verify exact wording at the source. Add new
sources here as they're used, with the URL and a direct quote.*
