# EXP-026 — DKW/dead-army rules: corpus arbitration (user challenge → measured verdict)

- **Date:** 2026-06-12
- **Hypothesis (user, 2026-06-11):** DKW pieces should be capturable while the king walks, then
  locked on the board once the king is captured — against the engine's EXP-011 model (fully
  frozen un-capturable; army swept on death).
- **Lever / change:** `board::dkw_rule()` — three rule variants (`HORNET_DKW_RULE=0|1|2`),
  threaded through `move_gen::is_wall`, the DKW-king move set, `make_move` capture scoring, and
  the game-flow death handling. Rules-correctness, not a strength lever.

## Authority check (before measuring)

- chess.com **Help Center** (fetched 2026-06-12): dead pieces remain on the board, can be
  captured, and **"Capturing dead pieces does not earn points"** — settles capturability and
  points directly; **silent** on what happens after the dead king itself is captured.
- `VERIFICATION-*.md` item #5 claimed dead pieces are captured **for points** (forum-sourced) —
  **contradicted by the Help Center**; corrected with a dated note.
- RuleVariants audit: all 319 corpus game headers share one gameplay config
  (`FFA DeadKingWalking EnPassant PromoteTo=D`) → the corpus encodes a single consistent rule.

## Method

`examples/replay_rules.rs`: replay every deduped corpus game's move stream (baselines +
human_games = 140 games / 33,771 move plies) against the move generator under each variant;
totals counted identically across variants. Replay never sets the `dkw` flag but `make_move`
sets `dead` on king capture, so the discriminating surface is exactly the **post-king-capture**
rule (the open question). DKW-phase capturability is settled by the Help Center text, not replay.

## Results

| Variant | Plies replayed | Games fully |
|---------|---------------|-------------|
| 0 — frozen un-capturable, swept on death | 24,945 | 72/140 |
| 1 — capturable while walking, **locked after death** | **23,648** | **61/140** |
| 2 — capturable always, never swept | 24,945 | 72/140 |

- **Variant 1 refuted** (−1,297 plies, −11 full games): recorded chess.com games capture dead
  players' pieces *after* their king has fallen.
- Variants 0 and 2 tie **in replay** (replay-equivalent by construction: replay neither sweeps
  nor freezes); between them the Help Center text decides for 2 — rule 0's game-flow sweep
  contradicts "remain on the board" and its freeze contradicts "can be captured".
- Remaining coverage gap (68/140 games diverge somewhere) is **not DKW-related** — rule 2 is
  maximal capturability and the gap persists. Separate move-gen/notation fidelity frontier;
  out of scope here.

## Conditions (after)

- **Default rule = 2**: DKW/dead armies are immovable, capturable by anyone for **zero points**,
  never swept; the walking king may capture any adjacent piece **including its own**
  (corpus-observed); `in_check` ignores dead/DKW armies (unchanged). Variants 0/1 remain as
  env diagnostics.
- Latent scoring bug fixed en passant: in-search captures of dead-owned pieces previously awarded
  full points.
- **Search and game flow now agree** — no sweep exists anywhere, dissolving the EXP-011
  inconsistency (search left armies on the board, game flow removed them).
- Suite 115 lib + 3 integration green; replay floors unchanged (replay-equivalence); spec §1.7
  rewritten (CO-007); VERIFICATION item #5 corrected.
- **Eval tension recorded, deliberately unresolved:** queries still count dead/DKW armies as
  the owner's material. Zeroing them re-creates the EXP-011 king-hunt pathology (vanishing
  victim material over-values king-captures). Valuing dead armies as neutral terrain is a
  future *measured* eval arm, not a rules question.
- The EXP-023 self-play corpus was generated under rule 0 — its games remain valid tuning data
  (labels unaffected; texel default is human-only anyway), but future bootstraps generate under
  rule 2; note the provenance difference if mixing.

## Conclusion

The user's challenge was right on the headline (capturable, not frozen — and worthless, fixing a
scoring bug nobody had noticed) and wrong on the post-death lock — which is exactly what a
corpus-arbitrated process is for: hypotheses go in, recorded reality decides. The engine now
plays chess.com's actual rule under a measured verdict, with the diagnostic variants preserved
for re-measurement as the corpus grows.
