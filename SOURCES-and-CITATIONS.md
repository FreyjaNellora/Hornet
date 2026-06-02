# Hornet — Sources and citations

Single manifest of every academic paper, reference document, and external
authoritative source cited in Hornet's documentation. Grouped by topic;
each entry includes URL plus where it's referenced inside Hornet.

When implementing a technique, consult the cited source for rigorous
formulation, edge cases, and prior empirical results before deviating
from the literature.

---

## Multi-player game search

### Multi-player Max^n utility vector backup

- Luckhardt, C. and Irani, K. 1986. *An Algorithmic Solution of N-Person
  Games*. AAAI.
- **Status:** No verified open-access URL. Common citation in multi-player
  search literature; check institutional library access for the proceedings.
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Multi-player utility
  vector for Max^n. `HORNET-BUILD-SPEC.md` § 6 Search Contract (V-vector
  backup).

### Shallow pruning for multi-player Max^n

- Sturtevant, N. R. and Korf, R. E. 2000. *On Pruning Techniques for
  Multi-Player Games*. AAAI.
- **URL:** https://cdn.aaai.org/AAAI/2000/AAAI00-031.pdf
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Forward pruning via
  bounded utility components.

### Multi-player alpha-beta pruning

- Korf, R. E. *Multi-Player Alpha-Beta Pruning*. AI Journal 48(1):99-111,
  1991.
- **URL:** https://faculty.cc.gatech.edu/~thad/6601-gradAI-fall2015/Korf_Multi-player-Alpha-beta-Pruning.pdf
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Forward pruning via
  bounded utility components.

### Iterated strategy elimination (game theory)

- Apt, K. 2004. *Uniform Proofs of Order Independence for Various Strategy
  Elimination Procedures*.
- **URL:** https://arxiv.org/pdf/cs/0403024
- *The Complexity of Iterated Strategy Elimination*.
- **URL:** https://arxiv.org/pdf/0910.5107
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Iterated Elimination
  of Strictly Dominated Strategies (IEDS).

---

## Single-player chess search techniques

### Alpha-beta and quiescence search

- Knuth, D. E. and Moore, R. W. 1975. *An Analysis of Alpha-Beta Pruning*.
  Artificial Intelligence 6(4):293-326.
- **Status:** No verified open-access URL; standard reference, available in
  academic databases.
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Quiescence search
  with multi-player rotation invariant.

### Singular extensions

- Anantharaman, T., Campbell, M. and Hsu, F.-h. 1988. *Singular Extensions:
  Adding Selectivity to Brute-Force Searching*. ICCA Journal 11(4):135-143.
- **Status:** No verified open-access URL; classic chess-search reference.
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Singular extensions.

### Vector / mailbox slider attack generation

- Chess Programming Wiki, *Vector Attacks*.
- **URL:** https://www.chessprogramming.org/Vector_Attacks
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Static line/ray
  projection per piece. Hornet's slider line projection is this primitive
  applied per-piece and indexed for tactical queries.

### NNUE (Efficiently Updatable Neural-Network Evaluation)

- Nasu, Y. 2018. *Efficiently Updatable Neural-Network-based Evaluation
  Functions for Computer Shogi*.
- Chess Programming Wiki overview: https://www.chessprogramming.org/NNUE
- **Status:** Original 2018 paper distributed in Japanese; chessprogramming.org
  provides the most-cited overview in English.
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Dense MLP on
  structured distilled features. Cited as the design Hornet deviates from
  (sparse binary inputs + HalfKP accumulator) and the reason.

---

## Neural network training

### Knowledge distillation

- Hinton, G., Vinyals, O. and Dean, J. 2015. *Distilling the Knowledge in
  a Neural Network*.
- **URL:** https://arxiv.org/abs/1503.02531
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Search-target value
  labels for training.

### AlphaZero (general game self-play)

- Silver, D. et al. 2017. *Mastering Chess and Shogi by Self-Play with a
  General Reinforcement Learning Algorithm*.
- **URL:** https://arxiv.org/abs/1712.01815
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Dense MLP on
  structured distilled features (input representation philosophy comparison).

### Representation in neural board-game engines

- Hammersborg, P. et al. *Representation Matters*. ECAI 2024.
- **URL:** https://arxiv.org/abs/2304.14918
- **Note:** The headline +180 Elo from representation alone is partially
  attributable to value loss change, not representation alone. Cite the
  weaker version.
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` general support for
  structured features over raw board state.

---

## Chess engine reference / counter-examples

### Bitboard scaling beyond 8×8

- Fairy-Stockfish source: `src/bitboard.cpp` and maintainer discussions.
- **URL:** https://github.com/fairy-stockfish/Fairy-Stockfish/blob/master/src/bitboard.cpp
- **URL:** https://github.com/fairy-stockfish/Fairy-Stockfish/issues/6
- **Referenced from:** `TECHNIQUES-and-REFERENCES.md` § Sparse binary
  piece-square features (anti-pattern). 14×14 boards do not fit canonical
  bitboard techniques without significant engineering work.

### YaneuraOu (9×9 Shogi specialized bitboards)

- **URL:** https://github.com/yaneurao/YaneuraOu/blob/master/source/bitboard.h
- **Referenced from:** prior deep-research workflow; geometry-specific
  techniques that don't generalize beyond 9×9.

### Crafty (incremental attack maps cautionary tale)

- Chess Programming Wiki, *Incremental Updates*.
- **URL:** https://www.chessprogramming.org/Incremental_Updates
- **Referenced from:** prior pipeline-benchmark analysis. Crafty famously
  abandoned incremental attack maps once recompute became fast enough; the
  bookkeeping cost exceeded recomputation cost. Informed Hornet's choice
  to always-recompute line projections rather than incrementally update.

### Attack and Defend Maps (Chess 4.5 style)

- Chess Programming Wiki, *Attack and Defend Maps*.
- **URL:** https://www.chessprogramming.org/Attack_and_Defend_Maps
- **Referenced from:** prior research; named as not-quite-the-analog for
  Hornet's per-piece line / per-square inverse index design.

### Efficient sliding-piece attack generation

- Chess Programming Wiki, *Efficient Generation of Sliding Piece Attacks*.
- **URL:** https://www.chessprogramming.org/Efficient_Generation_of_Sliding_Piece_Attacks
- **Referenced from:** background context for slider attack techniques
  (magic bitboards, rotated bitboards, Kogge-Stone fill, etc.).

---

## 4-Player Chess rules (chess.com authoritative)

### Help Center and rule articles

- **chess.com Help Center, 4 Player Chess (4PC):** https://support.chess.com/article/668-4-player-chess-4pc
- **chess.com 4 Player Chess terms page:** https://www.chess.com/terms/4-player-chess
- **Referenced from:** `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  items 4 (claim threshold), 5 (DKW behavior), 6 (stalemate scoring).

### Dead King Walking (DKW) standardization

- chess.com forum thread on DKW rule:
  https://www.chess.com/clubs/forum/view/4pc-variant-dead-king-walking
- **Note:** DKW became standard chess.com 4PC rule on 2019-05-21.
- **Referenced from:** `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  item 5.

### Claim-win threshold rules

- chess.com forum thread on autoclaim:
  https://www.chess.com/forum/view/chess-variants/4-player-chess-autoclaim-question
- **Referenced from:** `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  item 4.

### Stalemate scoring

- chess.com forum, *Stalemate in 4-player*:
  https://www.chess.com/forum/view/general/stalemate-in-4-player
- **Referenced from:** `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  item 6.

### Castling in 4PC

- Chess Wiki entry on 4-player chess:
  https://chess.fandom.com/wiki/4-player_chess
- **Referenced from:** `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  item 3.

### FEN4 / PGN4 format specification

- Chess.com Variants Wiki, *FEN4*:
  https://chess-variants.fandom.com/wiki/FEN4
- Four-Player Chess Wikibook, Notation:
  https://en.wikibooks.org/wiki/Four-Player_Chess/Notation
- chess.com blog, *4 Player Chess FEN* (AdamRaichu, updated 2021-03-12):
  https://www.chess.com/blog/AdamRaichu/4-player-chess-fen
- Reference Rust parser implementation:
  https://github.com/TheThirdOne/fen4
- **Referenced from:** `HORNET-BUILD-SPEC.md` § 9 (proposed `board/fen4.rs`
  and `board/pgn4.rs` modules); `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md`
  item 9.

---

## Status notes

- Some classical references (Luckhardt & Irani 1986, Knuth & Moore 1975,
  Anantharaman et al. 1988) lack reliable open-access URLs but are
  canonical enough that their citation alone is sufficient — implementers
  can locate them via institutional access. If an implementer cannot
  access one of these and needs the specifics, the chessprogramming.org
  wiki summary pages typically cover the practical algorithmic content.
- All chess.com source citations are forum threads, blog posts, and Help
  Center articles. These are authoritative for chess.com 4PC rules
  specifically (which differ in some details from other 4PC variants).
- Locally-cached copies of papers were not added to the repository to
  avoid copyright concerns. If link rot becomes a problem, request
  authoritative PDFs through institutional access and re-cite by DOI.
