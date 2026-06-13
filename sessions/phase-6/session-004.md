# Phase 6 — Session 004 (2026-06-12, Fable): the gates-verdict cycle

Continuation of session-003's shift. Three gate verdicts, two instrument traps, one corpus
discovery, all closed honestly. Nothing earned a config flip.

## Gate verdicts

- **EXP-032 pawn-adv**: run 1 VOID — `selfplay_ab` eval_id 4 fell through silently to deployed
  (both arms identical → 12/12 EXACT pair ties, the null signature). Fixed: ids 3/4 wired,
  unknown ids panic. Valid run 2: **NULL** (6–6–0, 809–800). The first instrument-beating
  agreement candidate (+1.5pp winners-only) bought zero self-play strength — fourth
  prediction≠play instance. No wiring; revisit only if the growing corpus re-nominates.
- **EXP-031 objective layer**: extension finished 14–9–1 BUT pairs 1–2 were move-for-move
  replays of the original run (seed index `si*per_split+g` renumbers when per_split changes;
  point vectors verified identical). Harness gained seed-offset arg 15. Honest 34 unique pairs:
  20–13–1, +8.7%, t=1.34 p≈0.09 → lean. **w100 variant** on common seeds: first 12 pairs +21.2%
  p≈0.044 (first config ever to touch the bar) → pre-registered extension of 12 fresh pairs
  (offset 12) came back DEAD EVEN (832–840). Combined 24: +9.5%, t(23)=1.27, p≈0.11. **Killed
  per the pre-registered rule.** Family pooling (58 pairs, both weights): +5.7/pair, t(57)=1.86,
  p≈0.034 — post-hoc, recorded as suggestive only. Defaults stay. Redirect: mining-nominated
  material-weakness targeting + the human tester gate.

## Mining program (passes 3 + 3b, prompted by the user's game-model hypothesis)

- Development order: pawn → knight → queen (own-move ~6.5 — early but knight leads) → bishop →
  rook; winners give the queen more early moves, losers touch their king 3× more early.
- **Promotions: winners 1.76 vs losers 0.67/seat-game (2.6×) — biggest differential found**
  (survivorship caveat). Promoted queens capture LESS per move than originals.
- King-raid proxy: winners higher every phase (early 2×); 25/33 king kills by winners.
- **Denial NOT differential** (victim progress 6.51 vs 6.57) — no denial feature; the edge is
  own advancement.
- **Elimination forensics: 67% of kill victims rank LAST in material (3% leader); 26/33 kills
  by rotation neighbors** → nominated objective-layer target selection by material weakness.
- Profitability (SEE): winners' trades NOT statically better (mid 60.2% vs 64.0% SEE>0), they
  overpay MORE, yet get answered less → the edge is unpunished captures (4-player tempo), not
  exchange values. Nothing for eval; validates proximity-weighted threat direction.

## Corpus: the user's 48-game batch + the 4PCo discovery

- Ingest script renumber threw (Measure-Object Double vs `{0:D4}`) → cleanup: 47 new + 1
  header-repaired = cc_game_0184..0232; PRIOR-batch GameNr dup found+removed
  (human_4pc_100783164 == cc_game_0051); old batch's 31 files git-mv'd to convention. Script
  fixed ([int] cast, 25-line GameNr scan, post-ingest corpus-wide uniqueness validation).
- **45/48 carried `StartFen4 "4PCo"` — the retired OLD chess.com array (Blue/Green K/Q
  exchanged), proven by a 4-way replay trial ladder** (canonical 1/45 full, BG-swap 13/45 +
  standard tails, ALL/RY 0/45; failure signature: kings on queen from-squares — 1-step king
  moves masquerade as queen moves until a long move exposes the swap). GameNr timeline: 4PCo
  spans 11.9–49.8M, standard 25.8–104M → array changed ~50M; engine canonical = current ✓.
- **Dispatch ruling: different starting geometry = a variant, kept separate** →
  `human_games_4pco/` (45 games + README); ingest auto-routes by StartFen4; `human_games/` =
  143 standard-array games, the only implicit instrument input.
- Lib: `fen4::START_FEN4_4PCO` + `pgn4::initial_board` match + pinning test (suite 117+1+3).
- New tool: `examples/corpus_check.rs` — per-file instrument gates + replay coverage + failure
  classification; run after every batch.
- Re-baselines (143 games / 10,705 positions): mining findings all hold; texel ISO only robust
  term (drop 0.0031–0.0037), WIN null.

## Carried queue

4PCo castle tail (king-onto-rook castle tokens + swapped-array castle geometry, 31 games lose
final plies); repetition/50-move/claim-win rules BEFORE wide tester distribution (verify
chess.com first); material-weakness targeting representation; move_match re-baseline on 143;
Elo-stratified mining + promotion curve at ~500 games; pst_value Green-transpose at next PST
revision; checkmate-DKW replay inference tail.

Commits this session: abc53c7 (tester loop), e394f1f (eval_id fix), 2f276bf (EXP-031 w50
verdict + seed offset), f63ea3a (comms), e4c7203 (mining pass 3), a28c71a (pass 3b), 1a13e8a
(EXP-032 close), 0e793de (w100 first 12), 05721b5 (ingest + 4PCo), 8078a28 (variant
quarantine), + this close.
