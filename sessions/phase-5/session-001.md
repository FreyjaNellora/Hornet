# Session 001 — Phase 5 (Eval) — C2/C3 continuation: defect → rework → first feature signals

**Date:** 2026-06-11
**Agents:** Kimi (C2.1–C2.3 draft + C3.1 pawn queries; session cut out mid-shift) → Fable
(audit, rework, tuning instrument, measurement; this note)

## What happened

Kimi's session died mid-C2/C3 with uncommitted work in `queries.rs`/`eval.rs` and her self-play
logs lost (0-byte files). Audit found her C3.1 pawn queries sound, and her C2 fold-in design
**inert as built**: candidate terms folded into P/S component *values* while
`W_POSITIONAL = W_SAFETY = 0` zero those components in the utility — any A/B arm would have
measured exactly nothing. Reworked per the approved plan: fold-ins reverted (eval hot path =
EXP-022 byte-identical, equality test green throughout), candidate-term tuning moved to where
independent weights exist — the tuner.

## Landed

- `queries.rs`: Kimi's `query_pawn_isolated/doubled/connected` kept, deduplicated through a
  shared `pawn_lanes` helper (clippy-clean; texel output reproduced exactly post-refactor).
- `texel_tune`: 9-weight fit `[M,P,S,O | WIN,DGR,ISO,DBL,CONN]` + **single-term marginal fits on
  two canonical bases** (deployed (6,0,0,1) and texel-shape (4,0,0,1)) — added after discovering
  marginals are base-sensitive (candidate signal overlaps material's). Self-check reproduces
  EXP-023 exactly with candidates at 0.
- **+49 human games ingested** (user-collected; found in `__pycache__/collected_games` — the
  collection script ran with the wrong cwd). RuleVariants audit across all 319 game headers:
  gameplay rules **uniform** (`FFA + DeadKingWalking + EnPassant + PromoteTo=D`; only
  privacy flags vary) → the user's data-contamination worry is unfounded for existing data, and
  the ingestion now filters on the standard rule string mechanically. Corpus: 290 games /
  17,003 positions (48% human).

## Findings (EXP-024 / EXP-025)

- **ISO (isolated pawns): the one robust term** — passes on every base and corpus
  (drops 0.00040–0.00081 vs floor ≈ 0.00005–0.00007). Fitted magnitude (~275–330 cp-equiv/pawn)
  is symptom-vs-cause suspect → P′-rebuild arm recorded, nothing shipped.
- **DGR (danger table): real but material-entangled** — strong at M=4 (0.00051–0.00070), mostly
  absorbed at M=6 (0.00015). S′ rebuild stays recommended with tempered expectations;
  the search-side runtime knob remains the primary vehicle.
- **CONN fragile** (base-dependent), **WIN and DBL null** (WIN sign-flips at floor magnitude).
- Self-play arms (win 50 vs 0; danger 100 vs 0; null control A≡B — flashlight cap 1200 d8):
  results in EXP-024 (the arms survived a VS Code restart as detached processes; polled manually).

## Rules thread (next shift)

User challenged the DKW frozen-pieces rule; chess.com Help Center confirms: **dead pieces are
capturable but award no points** ("Capturing dead pieces does not earn points") — contradicting
both the current engine rule (un-capturable) and `VERIFICATION-*.md` item #5 ("captured for
points", forum-sourced → needs a dated correction note). Post-king-capture behavior (user's
"locked" hypothesis) is unspecified officially → **corpus replay arbitrates** (3-variant
coverage measurement). All 319 corpus games share the same RuleVariants config, so the
arbitration measures a single consistent rule. Queued as the next shift: move-gen walls
capturable, no-points captures, eliminate_player sweep variants, eval treatment of dead armies,
replay 3-way, spec CO.

## Open

EXP-024 self-play fill-in (this shift's close), P′/S′ rebuild arms (Kimi), DKW rules shift
(queued), d8 instrument validation (open dispatch item).
