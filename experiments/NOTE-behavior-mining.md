# NOTE — behavioral mining program (ongoing study; user mandate 2026-06-12)

The human corpus is **study material**: mine winner-vs-loser differential behavior, engineer
representations for what's found, place each where it fits (eval / search / ordering / objective
layer), gate through the standard instruments. Precedent: PST v3 (zone/visit mining → tables).
Tool: `examples/behavior_mine.rs` (winner = finished 1st/2nd; shared replayer).

## Pass 1 (2026-06-12, 140 games)

**Capture targeting** (victim's current points-standing among the mover's opponents):

| | leader | middle | trailing | total captures |
|---|---|---|---|---|
| winners | 47.4% | 33.7% | 18.9% | 3,487 |
| losers | 50.2% | 30.2% | 19.5% | 1,847 |

- **Winners capture ~1.9× more per player** — partly circular (captures earn the points that
  define winning) but the magnitude frames 4PC: activity is placement. A cleaner second-order
  read (capture *profitability* — SEE of human captures, winners vs losers) is queued.
- **Losers chase the leader; winners farm the middle** — a target-selection behavior. Candidate
  representation: an objective-layer targeting term (who to pressure) rather than an eval term;
  also relevant to move-ordering bounty shape.

**Zone destinations** (winner% − loser%, absolute zones): small deltas (≤3.4pp); winners' pawns
toward Center (+2.1pp — the central promotion lanes), bishops/rooks slightly toward far-side
quadrants/gates. **Instrument refinement needed:** zones are absolute while the four players see
the board from four directions — pass 2 must rotate each player's moves into a common frame (the
PST transform) before aggregating, which should sharpen placement preferences substantially.

## Queued passes

- **Player-relative frames** for destination maps (the pass-1 lesson).
- **Capture profitability**: SEE values of human captures, winners vs losers (do winners take
  *better* trades or just more of them?).
- **Timing**: when do winners trade vs develop vs push (game-phase stratified).
- **Aggression direction**: which *opponent* (left/right/across — tempo-relative) winners attack;
  feeds the retargeting/tempo discussion (rotation-2 dynamics).
- Each finding that suggests a feature goes through: representation → placement decision →
  winners-only agreement → paired gate (EXP-027).
