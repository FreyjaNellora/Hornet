# Hornet baseline assets

Human-played chess.com 4PC reference material for testing parsers, tuning
evaluation weights, and validating strength.

## Contents

### PGN4 game corpus (16 files)

`human_4pc_game_*.pgn4` — real chess.com 4PC games in chess.com's standard
PGN4 format. Multiple games include players rated 3000+ ELO. Source URLs
in each file's `[Site]` tag point back to chess.com.

**Use for:**

- **Parser round-trip test:** parse each file, serialize back, byte-compare.
  Catches PGN4 encoding bugs before they propagate.
- **Move-stream replay test:** ingest the move list, apply each move in
  turn, verify board state remains legal throughout. Catches move-gen and
  rule-interpretation bugs.
- **Position diversity sampling:** mid-game positions from these games are
  natural test fixtures for evaluator behavior.
- **Real-input strength gate:** when evaluator is functional, replay games
  position-by-position and compare engine's `bestmove` against the human's
  actual move. Strong human moves the engine misses are evidence of
  evaluator blind spots.

### Tactical samples fixture suite

`tactical_samples.json` — 25 curated tactical positions extracted from
3000+ ELO games, each a 5-6 move window with a clear right-or-wrong answer
judged by game outcome.

**Sample schema (per entry):**

- `id` — unique sample identifier (e.g. `S01`)
- `name` — human-readable position name
- `type` — tactical category (chain_reaction, queen_activation, etc.)
- `game` — source chess.com game ID (cross-reference back to the PGN4 file
  if present in the corpus)
- `round` — turn number in the source game
- `moves_to_replay` — full move sequence to reproduce the position from
  starting position (chess.com 4PC notation). Engine ingests these to
  reach the target position.
- `position_after` — human-readable description of the state.
- `test_move` — whose move is under test.
- `human_move` — the move the strong human actually played.
- `expected_category` — what kind of move the position calls for
  (capture, development, defense, etc.).
- `consequence` — what happened after the human's choice.
- `what_to_check` — diagnostic note about what the engine should see.
- `score` — scoring rubric:
  - `match: 2` — engine chose the same move as human
  - `category_match: 1` — engine chose a different move of the right
    category
  - `neutral: 0` — engine chose a defensible alternative
  - `anti_pattern: -1` — engine chose a clearly inferior or category-wrong
    move
- `fen4` — chess.com FEN4 string for the target position (use directly via
  `position fen4 <string>`).
- `nextturn` — the player to move at the test position.

**Source manifest** (in the JSON's `sources` key) lists each contributing
game's average ELO and player range so test difficulty can be calibrated.

**Notation note:** the `moves_to_replay` and `human_move` fields use
chess.com's 4PC notation (`h2-h3`, `Ne1-f3`, `Qg1xj4`). Some legacy entries
also include a `human_move_freyja` field with the same move translated to
an older project's notation — that field is not load-bearing for Hornet
and can be ignored.

**Use for:**

- **Strength gate primary signal:** percentage of samples where engine's
  `bestmove` matches the human move (the `match` scoring tier) gives a
  direct number for whether the evaluator approaches human-level tactical
  play.
- **Evaluator weight tuning:** when V's hand-tuned weights w₁..w₄ are
  changed, re-run the suite and compare aggregate score before/after.
  Texel-style tuning at small scale.
- **Regression suite:** anything that changes search or eval reruns the
  suite. Drops in score flag regressions before strength duels do.

## Migration provenance

These files originated from a prior project's `observer/baselines/`
directory. The PGN4 files are unmodified chess.com exports; the
`tactical_samples.json` schema documented here was authored against
the same data and remains accurate for Hornet use modulo the
`human_move_freyja` notation note above.
