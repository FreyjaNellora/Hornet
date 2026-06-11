# Pitch — Strength gate (tactical-fixture harness)

The strength gate (Hard Rule #7) is the bar the hand-tuned evaluator must clear **before** any NNUE
training (P7): on a suite of tactical positions, the engine's chosen move should match the strong
human's move often enough. This pitch is the **harness that measures it** and the signal to tune the
v0 eval weights against (README's "Texel-style tuning"). Eval/queries lane (Kimi); it only *consumes*
public board/move-gen/search APIs — no changes to search/board internals.

## Source fixtures

`baselines/tactical_samples.json` — 25 curated positions (S01–S25) from 3000+ ELO games. Per entry
(see `baselines/README.md` for the full schema): `moves_to_replay` (chess.com 4PC notation, from the
start position), `human_move` (the move under test), `test_move` (whose move it is), `nextturn`
(player to move at the test position), `expected_category`, and a `score` rubric.

## Reaching each position — no FEN4 converter needed

**Do not use the `fen4` field.** It's the non-native `xxx`-corner dialect the parser doesn't read.
Instead replay `moves_to_replay` from the start with the existing machinery:
`pgn4::decode_ply` → match against `generate_pseudo_legal` → `board.make_move` (exactly the
`tests/pgn4_replay.rs` loop). That reaches the test position with code that already works, and sidesteps
the converter entirely. Sanity-check `board.side_to_move == nextturn` after replay.

## Run + score

- `Searcher::search(&mut board, depth)` → best move. **Depth 4** for now — depth 8 is intractable
  until search pruning lands (claude's parallel item); 4 is fine for tactical shots.
- Decode `human_move` via `decode_ply` → (from, to, promotion); compare to the engine's returned
  `Move`. Exact match → the `match: 2` tier.
- Rubric: `match: 2`, `category_match: 1`, `neutral: 0`, `anti_pattern: -1`.

## The primary signal

The gate number is the **match rate**: % of the 25 samples where engine best move == `human_move`.
Report it as a single percentage. That's the strength-gate metric; everything else is secondary.

## Open / `[DEFINE]`

- **Category classification** (`category_match`, `anti_pattern`) needs an operational definition of
  move categories (capture / development / defense / …) and how to map an arbitrary engine move to one.
  Start with **match-rate only**; add the category tiers once the classifier is defined. Don't invent it.
- `human_move_freyja` field is legacy — ignore.

## Tuning loop

After the harness works, vary the v0 weights (`W_MATERIAL` / `W_POSITIONAL` / `W_SAFETY` /
`W_CROSSFIRE` in `eval.rs`), re-run, compare aggregate match rate before/after. That's how the eval
climbs toward the gate.

## Constraints

- `eval_value` (cp) vs `ffa_points` — the gate scores *move correctness*, not points; don't conflate.
- New file (a `strength_gate` example or integration test); read-only use of `Searcher::search`,
  `decode_ply`, `generate_pseudo_legal`. `Searcher::search`'s public signature is stable — claude's
  pruning work won't change it.

## Verify

```
cd hornet-engine
cargo test                  # stays green
```
Harness prints the match rate (e.g. "strength gate: 11/25 = 44%").
