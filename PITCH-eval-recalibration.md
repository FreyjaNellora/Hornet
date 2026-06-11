# Pitch — the v0 evaluator is scale-miscalibrated (recalibrate before any eval features)

Hornet is a four-player-chess engine in Rust (`hornet-engine/`). The hand evaluator
`eval_4vec(board, lines) -> [i16;4]` returns the per-player utility vector V that Max^n backs up;
`Uᵢ = w₁·ΔMᵢ + w₂·ΔPᵢ + w₃·ΔSᵢ − w₄·ΔOᵢ` over four query components — material, positional,
king-safety, crossfire — each taken relative to its per-player mean so `Σᵢ Uᵢ ≈ 0`
([eval.rs:63](hornet-engine/src/eval.rs#L63)), with weights `W_MATERIAL=4, W_POSITIONAL=2,
W_SAFETY=1, W_CROSSFIRE=1` ([eval.rs:19](hornet-engine/src/eval.rs#L19)).

**Claim to confirm or refute: the four components are on wildly different raw scales (≈4 orders of
magnitude apart), so the weights are meaningless, crossfire dominates and destabilizes the vector, and
king-safety is scaled to near-invisibility.** This — not search depth, not missing tactical features —
is why the engine's move selection is noise. Verify it yourself with the recipe below; the fix and the
one real decision (how to rescale, and the crossfire formula) follow.

## The evidence (reproduce it)

`examples/gate_ablation.rs` prints, per fixture, the mover's **static** `eval_4vec` at the position and
after a single move. Observed (depth-4 baseline, `baselines/tactical_samples.json`):

```
       static   after-human-move   after-engine-move
S06    10732       -2959               8170      ← one bishop-takes-pawn swings the static eval by 13,691
S02     1508       -8010                249      ← a queen trade swings it by 9,518
S07     9282        9372               7311      ← engine PICKS the worse-static move (search ≠ leaves)
```
Static values span roughly **−8,000 .. +11,000** across these positions, and **a single move routinely
moves the eval by thousands.** A sane eval cannot swing that much on one move — the search then
faithfully maximizes the noise (this is why deeper search and exchange-aware threats both changed
nothing: garbage in, garbage out).

**Minimal independent check** (no harness): on any midgame position, call `eval_4vec`, make one quiet
move, call it again. The delta should be on the order of a positional nudge (tens of centipawns). It
isn't.

## The scale mismatch (read these, estimate the magnitudes)

- **Material `Mᵢ`** — `query_material` ([queries.rs:59](hornet-engine/src/queries.rs#L59)): Σ
  `eval_value` (P100 N300 B450 R500 Q900). Per-player ≈4200 at start; **deviations grow to thousands**
  as material imbalances open. Linear in piece value.
- **Positional `Pᵢ`** — `query_positional_control` ([queries.rs:84](hornet-engine/src/queries.rs#L84)):
  Σ `centrality_weight` (0–5) over empty reached squares, **plus** `query_threats`
  ([queries.rs:99](hornet-engine/src/queries.rs#L99)) `+value(target)/4`. Order ≈ hundreds.
- **Safety `Sᵢ`** — `safety_scalar` ([queries.rs:199](hornet-engine/src/queries.rs#L199)):
  `defenders − attackers + escapes`. Order ≈ **single digits** (≈ −5..+10). At `W_SAFETY=1` this is
  ~100–1000× smaller than material — **king safety is effectively invisible in V.** (It also discards
  the `attack_value` it computes at [queries.rs:172](hornet-engine/src/queries.rs#L172).)
- **Crossfire `Oᵢ`** — `query_crossfire` ([queries.rs:227](hornet-engine/src/queries.rs#L227)):
  `penalty = enemy_value * enemy_count + own_value`, where `enemy_value` is a **sum of centipawn piece
  values**. That is centipawn×count (≈ value-squared scale), per piece, summed → order **thousands to
  tens of thousands**, and the most volatile term (it jumps as attacker sets change).

So raw scale ≈ `S(1s) ≪ P(100s) < M(1000s) ≲ O(1000s–10000s)`. The `4/2/1/1` weights assume a common
scale that doesn't exist: **O and M dominate, S and P are noise, and O's `value×count` makes the whole
vector swing by thousands per move.**

## The decision you own: how to rescale, and the crossfire formula

1. **Common unit.** Put all four components in **centipawns** so the weights are interpretable.
   Material already is. Positional/safety need scaling up (a king with no escapes and two attackers
   should cost a meaningful fraction of a piece, not 3 "points"). Decide the per-component scale.
2. **Fix crossfire.** `enemy_value × enemy_count` is dimensionally wrong. Crossfire should be **material
   actually at risk** — i.e. the SEE/exchange amount on the piece (≈ `min(attacker, victim)` net of
   defenders), bounded by the victim's value, in centipawns — not value×count. (A clean per-square
   exchange primitive already exists: `queries::see_capture` / `see_swap`,
   [queries.rs](hornet-engine/src/queries.rs) — crossfire can read it.)
3. **Re-derive the weights** against the rescaled components.

This is the whole game: with the components on one scale and crossfire bounded by real material, the
eval stops swinging by thousands and the weights mean something.

## Constraints (engine rules — honor all)

- **V stays M/P/S/O (Hard Rule #4).** Recalibration changes magnitudes/formulas, not the 4-component
  structure; each component still traces to one query.
- **Zero-sum must survive.** `compute_utility` is mean-relative so `Σ Uᵢ ≈ 0`
  ([eval.rs:171](hornet-engine/src/eval.rs#L171)) — this enables Sturtevant–Korf shallow pruning
  (`SUM_UB≈3`). The rescaled components must still be per-player scalars that mean-subtract to ≈0.
- **Points-blind (Hard Rule #8 / §1.7).** `bounty.rs` folds `ffa_points` into Oᵢ
  ([bounty.rs:154](hornet-engine/src/bounty.rs#L154), [eval.rs:38](hornet-engine/src/eval.rs#L38)) —
  FFA points reaching V. **In scope:** rebuilding Oᵢ as SEE material-at-risk (centipawns) means the
  ffa_points bounty term leaves V entirely — Oᵢ becomes one clean centipawn term, which is the only way
  to put O on a common scale anyway (ffa_points 1–9 vs centipawns is itself a scale mismatch). The
  FFA-hunt preference already lives in move ordering ([move_order.rs:153](hornet-engine/src/move_order.rs#L153));
  keep `bounty.rs`'s per-square analysis as a substrate if useful, but it stops feeding V. Stage it as
  its own measured step if you want attribution.
- **Budget.** `eval_4vec` runs per leaf and recomputes lines; the debug test asserts < 600 µs
  ([eval.rs:145](hornet-engine/src/eval.rs#L145)). Stay within it.
- **Engine-only.**

## Done looks like

- The four components are on a common scale; crossfire reflects material-at-risk, not value×count.
- **Calibration gate (the new first-line acceptance check):** a single *quiet* move changes
  `eval_4vec` by ~its positional delta (tens of cp), **not thousands**. Add a test that asserts a
  bound on `|eval_after − eval_before|` over quiet moves on real positions.
- King-safety is no longer negligible; a king under attack with no escapes costs a meaningful fraction
  of a piece.
- `Σ Uᵢ` still ≈ 0; `eval_4vec` still < 600 µs; `cargo test` green.
- *Then* re-run the prior measurements **on the recalibrated eval** — they are currently invalid
  because they were taken on the broken one (garbage in): the **depth sweep** ("depth 4 = depth 8 =
  0/13") and the **`HORNET_SEE` off/on threats comparison** ("SEE null"). Both may change once the eval
  is stable; treat neither as settled. The calibration gate, not the match rate, is the acceptance
  criterion (exact-move match vs a 3000-Elo human is harsh — a different good move scores as a miss).

## Landmines

- **Don't add features to fix this.** Exchange-aware threats, deeper search, etc. all sit downstream of
  the scale bug and were measured to do nothing until it's fixed.
- **`safety_scalar` drops `attack_value`** — folding it in (net of defenders) is part of putting safety
  on a real scale, not a separate task.
- **Mean-relative hides absolute scale** — after each change, check the zero-sum test and the
  quiet-move stability bound, not just one position.
- **The match rate is exact-move-vs-3000-Elo-human** — a poor primary signal (a different good move
  scores as a miss). Calibrate against eval *stability* first.

## Verify

```
cd hornet-engine
cargo test                                   # keep green; add the quiet-move stability test
cargo run --release --example gate_ablation  # prints static eval before/after each move (the swings above)
```
If a single quiet move still swings the eval by thousands after the change, the recalibration isn't
done — report the residual rather than declaring victory on the match rate.
