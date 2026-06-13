# human_games_4pco — the OLD-ARRAY variant corpus (kept separate, never mixed)

These are human chess.com FFA games whose `StartFen4 "4PCo"` tag marks the **old starting
array**: Blue's and Green's king/queen home squares are exchanged relative to the current
standard (bQ a7 / bK a8, gQ n8 / gK n7; Red/Yellow identical). Same rules, **different starting
geometry** — and per the project's data discipline (dispatch ruling 2026-06-12) that makes them
a variant which must stay out of `human_games/`:

- Opening development, early queen geometry, and castle structure for Blue/Green differ by
  construction — mixing them would blur exactly the behavioral signals the mining program
  measures.
- Timeline evidence (GameNr ranges): 4PCo spans ~11.9M–49.8M, the standard array ~25.8M–104M.
  The old array dies out around ~50M — chess.com changed the setup; everything recent is the
  standard array, which is also the engine's canonical start.

Engine support: `fen4::START_FEN4_4PCO` + `pgn4::initial_board` read these files fine — every
instrument CAN run on this directory explicitly (e.g. `behavior_mine human_games_4pco`), it just
never happens implicitly. The ingest script routes `4PCo` exports here automatically.
