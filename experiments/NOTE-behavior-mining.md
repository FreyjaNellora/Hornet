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

## Pass 2 (2026-06-12, 140 games — player-relative frames, answer rate, phase split)

- **Leader-chasing is a midgame behavior:** losers' captures hit the current leader 53.2% in
  midgame vs winners' 47.3%; the gap vanishes late (46.1 vs 45.3). (Early-phase rows are a
  tie-handling artifact — points are all zero, so every victim reads "leader". Ignore early.)
- **Winners' captures stick:** answered-on-square within ≤4 plies — winners 30.2%/25.4%
  (mid/late) vs losers 34.3%/31.7%. Winners trade 4–6pp more safely; the engine's SEE already
  embodies this, so no new representation needed (validates the existing crossfire design).
- **Red-frame destinations (now sharp):** winners push pawns to **Center** (+2.1pp — the
  promotion crossing) and away from home (GateS −2.6pp); queens follow (+0.9 Center); winners'
  **rooks to the home gate** (+1.5pp GateS — the lift/back-rank-center entry). Losers shuffle
  near home. → **Nominated: pawn forward-progress** (built as `eval_4vec_pawn_adv`, EXP-032)
  alongside the classical rook-open-lane candidate.
- Instrument note: true C4 rotations used; found en route that `queries::pst_value`'s **Green
  branch is a transpose, not a rotation** — currently harmless (the zone PST is
  transpose-symmetric; the equivariance test can't distinguish) but a latent bug if the PST ever
  becomes file-asymmetric. Flagged for the next PST revision.

## Pass 3 (2026-06-12, 140 games — development order, promotions, king-raid, denial, eliminations)

Prompted by the user's game-model hypothesis (pawn race → early queens → promote → swarm kings;
center-guarding as promotion denial; "systematic elimination based on something").

- **Development order (mean own-move index of first move):** pawn 1.0 → knight ~4.8 →
  **queen ~6.5** → bishop ~9.5 → rook ~16 → king ~18.5. The queen comes out early by classical
  standards but the KNIGHT leads. Order is nearly identical for winners and losers — but the
  **profile** differs: winners spend 18.8% of their first 8 moves on the queen vs losers 15.4%,
  and losers touch their king ~3× more early (2.3% vs 0.8% — an early-trouble marker).
- **Promotions = the biggest differential found yet: winners 1.76/seat-game vs losers 0.67
  (2.6×)**; mean ply 172 vs 152. (Survivorship caveat: winners live longer, so more late plies
  to promote in — part of the gap is consequence, not just cause.) Promoted queens capture
  slightly LESS per move than originals (22.9% vs 26.4%) — the promoted queen is presence and
  material, not a distinctly more murderous piece.
- **King-raid proxy (destinations within Cheb ≤2 of an enemy king):** winners higher in EVERY
  phase — early 4.7% vs 2.3% (2×), mid 10.6 vs 9.0, late 15.0 vs 10.9 (+4.1pp). Winners deliver
  25/33 king kills. The swarm is real and winner-differential.
- **Promotion denial is NOT differential:** pawn-victim progress 6.51 vs 6.57, advanced-victim
  share 55.8% vs 55.4% — killing runners is universal behavior, both classes do it equally.
  The winner edge is getting YOUR pawns through, not stopping theirs → supports weighting own
  advancement (the gated pawn-adv term) over a denial term. No denial feature nominated.
- **Elimination forensics (33 king kills):** victims at the kill are the WEAK — **67% rank last
  in material (82% bottom-half); 48% last in points (66% bottom-half); only 3% of kills hit the
  material leader.** Elimination is opportunistic predation on the materially weakest, not
  regicide on the leader. Rotation offset: 26/33 kills are by a rotation NEIGHBOR (+1: 13,
  across: 7, +3: 13) — your killers are adjacent in turn order. Small n; re-run at 500+ games.
  → **Nomination: objective-layer target selection should rank opponents by MATERIAL weakness
  (better elimination predictor than points standing), pending the win-term gate verdicts.**

## Queued passes

- **Player-relative frames** for destination maps (the pass-1 lesson).
- **Capture profitability**: SEE values of human captures, winners vs losers (do winners take
  *better* trades or just more of them?).
- **Timing**: when do winners trade vs develop vs push (game-phase stratified).
- **Aggression direction**: which *opponent* (left/right/across — tempo-relative) winners attack;
  feeds the retargeting/tempo discussion (rotation-2 dynamics).
- Each finding that suggests a feature goes through: representation → placement decision →
  winners-only agreement → paired gate (EXP-027).
