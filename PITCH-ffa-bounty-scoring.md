# Pitch — FFA bounty scoring (ffa_points in search)

Search currently optimizes `eval_value` (centipawns). The win condition is `ffa_points`.
`ffa_points` is "result tags only" today ([types.rs:205](hornet-engine/src/board/types.rs#L205)) —
not in the search at all. Terminal scoring is unwired: a no-legal-moves node returns the static
centipawn eval ([search.rs:103–105](hornet-engine/src/search.rs#L103-L105)), so delivering a
checkmate scores nothing.

## Spec (locked)

- capture/target score = `ffa_points(victim)`: P1 N3 B3 R5 Q9 K20
- small flat bonus for mate / king-capture moves
- feeds move ordering and the terminal score

## The two value tables

`ffa_points` ([types.rs:207](hornet-engine/src/board/types.rs#L207)) vs `eval_value`
([types.rs:193](hornet-engine/src/board/types.rs#L193)). Hard Rule #8: distinct, never conflate.

| Piece | `eval_value` (cp) | `ffa_points` |
|---|---|---|
| Pawn | 100 | 1 |
| Knight | 300 | 3 |
| Bishop | 450 | 3 |
| Rook | 500 | 5 |
| Queen | 900 | 9 |
| King | 0 | 20 |

## Targeting rationale — full-board census (4 players, start)

| Type | Count | Pts each | Pool | % |
|---|---|---|---|---|
| King | 4 | 20 | 80 | 33.9 |
| Rook | 8 | 5 | 40 | 16.9 |
| Queen | 4 | 9 | 36 | 15.3 |
| Pawn | 32 | 1 | 32 | 13.6 |
| Knight | 8 | 3 | 24 | 10.2 |
| Bishop | 8 | 3 | 24 | 10.2 |
| Total | 64 | | 236 | 100 |

Notes: bishop = knight = 3 in points (eval_value rates the bishop higher — don't conflate).
Rook pool (40) > queen pool (36). King capture eliminates a player and, depending on the
Dead-King-Walking rule (item #5, unpinned), may leave ~39 pts of that player's pieces capturable.

## Where it lands

- `move_order.rs` — MVV-LVA scores victims by `eval_value`
  ([move_order.rs:19–28](hornet-engine/src/move_order.rs#L19-L28)). Add a bounty term =
  `ffa_points(victim)`. Keep `eval_value` for the positional/SEE dimension (Hard Rule #8).
- `search.rs::maxn` terminal node — wire §1.8 terminal scoring (roadmap item #2): checkmate →
  +20 elimination, stalemate consolation, dead-king split. This is the only place the 20-pt king
  becomes visible to the search.
- `types.rs::ffa_points` — source values; `u8`, currently result-tags only.

## Constraints

- Hard Rule #8: `eval_value` (cp) for positional strength/SEE; `ffa_points` for the scoring
  objective. Never conflate.
- Default-off + ablation arm.
- Optional gate (later): rank targets by bounty, gate by the dominance/paranoid check so the
  engine doesn't overextend into the other two players. Cross-ref the dominance-vector work.

## Verify

```
cd hornet-engine
cargo test                  # 66 green today
```
