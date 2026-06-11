# Pitch — Layer-3 individual-piece tensor (NNUE input features)

**Status & sequencing:** this is a *feature-representation* design, captured from notes. It is
**NNUE-input territory (roadmap item #7)**, gated behind the strength gate (Hard Rule #7) — don't
implement until the hand-tuned evaluator clears that gate. It is **not** a change to the hand-eval's
fixed 4-component V (Hard Rule #4: V stays M/P/S/O). Parts below are grounded in existing
primitives; three intent components are **named but not yet operationally defined — marked [DEFINE]**.
Do not invent definitions for them.

## Player indexing (root-relative)

Active player = **P0 (Root)**. Opponents = **P1, P2, P3 in turn order after root**
(e.g. Blue root → Yellow=P1, Green=P2, Red=P3). This is a reindexing of the fixed R/B/Y/G order;
everything per-opponent is expressed relative to the mover.

## Representation (sparse, per piece)

Every board coordinate owns a vector space; only **occupied** squares materialize the full tensor.
(Aligns with line projection, which already builds per-piece records only for pieces present.)

Per-piece components:
1. **Occupant class** — one-hot piece type. *(grounded: `PieceType`.)*
2. **Kinematic Reach** — 160-bit mask: squares reachable on an **empty board** (the movement
   pattern, ignoring occupancy). *(Not what `compute_lines` stores — that accounts for blockers — so
   this is a separate precompute: rays-to-edge for sliders, fixed deltas for steppers. Standard
   attack-table material.)*
3. **Mobility Mask** — 160-bit mask: squares **actually** reachable, accounting for pins/blocked
   moves. *(grounded: legal reach via `generate_legal` / line-projection reach + legality filter.)*
4. etc. (extensible)

`160-bit` = the 160 valid squares.

## Targeted intent matrix (per opponent)

`I_piece = [I_P1, I_P2, I_P3]` — one intent subvector per opponent (root-relative). Each subvector:
- **Direct Offense** — [DEFINE]
- **Prophylaxis** — [DEFINE]
- **Vulnerability** — [DEFINE]
- **X-ray Intent** — latent pressure through blockers. *(grounded: `ReachEntry.xray_continues`,
  [lines.rs:31](hornet-engine/src/lines.rs#L31) — the engine already projects past the first blocker.)*

## Mapping to existing primitives

| Component | Engine primitive | Status |
|---|---|---|
| Occupant class | `PieceType` | grounded |
| Kinematic reach | movement deltas / rays-to-edge (new precompute) | needs a per-type×square table |
| Mobility mask | `generate_legal` / line-projection reach + legality | grounded |
| X-ray intent | `ReachEntry.xray_continues` | grounded |
| Per-opponent split | inverse index knows each reacher's player; per-player `[_;4]` arrays | grounded (reindex to root-relative) |
| Direct Offense / Prophylaxis / Vulnerability | — | [DEFINE] |

## Constraints

- **Hard Rule #4:** V stays M/P/S/O. This is **NNUE input**, feeding the dense-MLP NNUE (item #7);
  it does not add a 5th V component.
- **Default-off + ablation arm.**
- **Keep features flat and probeable** (the isolable-hierarchy principle): each feature additive /
  linear-probeable so it stays independently ablatable — do not chain them into a feeding pipeline.
- **Per-opponent decomposition** is where the keep-perspectives-separate design and the
  dominance/paranoid work live at the piece level — the intent matrix is that, per piece.
- Engine-only.

## Open items (resolve before build)

- Operational definitions for **Direct Offense / Prophylaxis / Vulnerability** (X-ray is defined).
- Tensor shape — the notes write `(4k, w)`; clarify dimensions.
- How **Layer 3** composes with Layers 1–2 (not in these notes).

## Verify

```
cd hornet-engine
cargo test                  # 66 green today
```
Feature extraction validated against line projection on sample positions before any training use.
