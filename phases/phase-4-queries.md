# Phase 4: Query Engine

## Commander's Intent

Turn the geometric `LineMap` (P3) into the four scalar-per-player query outputs the evaluator needs:
material, positional control, king safety, and crossfire — a `QueryVector`. This is the only thing
the hand-tuned evaluator reads. Owner: **Kimi**.

## Reading List (Start Here)

1. `STATUS.md`
2. `phases/phase-4-queries.md` — this file.
3. `COMMS_CLAUDE_HANDOFF_P4.md` — the P3 `LineMap` contract + Hard Rules for V.
4. `HORNET-BUILD-SPEC.md` §4 (query engine contract), §3 (line projection it consumes),
   §1.7/§2.3 (`eval_value` vs `ffa_points` — Hard Rule #8).
5. `hornet-engine/src/lines.rs` (`compute_lines`, `LineMap`, `PieceLines::entries`, `reachers_at`).

## Write Scope

**Owns:** `hornet-engine/src/queries.rs` (`QueryVector`, `query_material`, `query_positional_control`,
`query_king_safety`, `query_crossfire`, `run_all_queries`). May add query-specific tests.
**Read-only:** `board/`, `lines.rs`, everything else. Changes to those → change order.

## Current State

| Field | Value |
|-------|-------|
| Status | not-started — **ready** (P3 `LineMap` is stable + tested) |
| Last Session | — |
| Blocking Issues | none |
| Next Action | Implement `query_material` first (simplest; start-pos check `[4200; 4]`) |

## Acceptance Checklist (from spec §4 / §7.4)

- [ ] `QueryVector { material, positional, safety, crossfire }` — each `[i16; 4]`.
- [ ] `query_material` uses `eval_value()` (Hard Rule #8); start position = `[4200, 4200, 4200, 4200]`.
- [ ] `query_positional_control` — centrality-weighted empty-square control (§4.3 weight formula).
- [ ] `query_king_safety` — defenders / attackers / attack_value / escape_squares (§4.4), incl. the
      radius-2 knight check.
- [ ] `query_crossfire` — converging-enemy penalty (§4.5).
- [ ] `run_all_queries(lines, board) -> QueryVector` — the single entry point the evaluator calls.
- [ ] Tests per §7.4 (material start/after-capture, positional symmetry, king-safety defenders,
      crossfire empty/with-convergence). `cargo test`/`clippy`/`fmt` green.

## Active Watch Items

- **Hard Rule #4:** `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`. Each query = exactly one V component. No 5th
  component, no merging — that would be a change order.
- The `LineMap` is recompute-only (Hard Rule #5); queries read it, never cache across nodes.

## Downstream Notes

P5 (eval, also Kimi) consumes `QueryVector` to build `eval_4vec(&state) -> [i16; 4]` (Hard Rule #3 —
vector, never scalar). P6 (search, Claude) consumes `eval_4vec`. Keep the `QueryVector` field order
aligned with the V decomposition so the evaluator weights map 1:1.
