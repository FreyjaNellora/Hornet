# Pitch — exchange-aware tactical eval (the strength-gate blocker)

Hornet is a four-player-chess engine in Rust (`hornet-engine/`). The hand-tuned evaluator
`eval_4vec(board, lines) -> [i16;4]` returns the per-player utility vector
`V = ⟨U₁,U₂,U₃,U₄⟩` that Max^n backs up; `Uᵢ = w₁·ΔMᵢ + w₂·ΔPᵢ + w₃·ΔSᵢ − w₄·ΔOᵢ` over four
query components (material, positional, king-safety, crossfire), each made relative to its
per-player mean so `Σᵢ Uᵢ ≈ 0` ([eval.rs:63](hornet-engine/src/eval.rs#L63)).

The evaluator clears **0 of 13** tactical fixtures in the strength gate. **This pitch is the
diagnosis of why, and the fix.** The job is not to add a new component or a new module — it is to
make the *existing* tactical query components understand **exchanges** (defended vs undefended,
attacker value vs target value) instead of counting raw attackers. That single gap is what makes the
eval reward meaningless threats as much as winning ones, so it can't rank a real tactic first.

## The evidence — it is the eval *signal*, not depth or weights

A depth × quiescence sweep over the 13 testable fixtures
(`examples/gate_ablation.rs`, speed levers on, 800k-node budget):

| | quiescence OFF | quiescence ON |
|---|---|---|
| **depth 4** | 0/13 | 1/13 |
| **depth 8** | 0/13 | 1/13 |

- **Depth can't help *through a broken eval*.** Doubling the horizon (4→8) changes nothing — but that
  does **not** prove depth is useless; it proves a faulty eval makes depth useless (shit in → shit
  out). Search *amplifies* eval quality, it can't create it, so the depth variable is **unmeasurable
  until the eval produces non-garbage leaves**. The depth question is *deferred, not settled* — the
  pruning/search work pays off the moment the eval below can rank a position.
- **Weights don't help.** The v0 weights were already retuned toward aggression
  (`W_MATERIAL=4, W_POSITIONAL=2, W_SAFETY=1, W_CROSSFIRE=1`,
  [eval.rs:19](hornet-engine/src/eval.rs#L19), comment: *"engine was too passive"*) — still 0/13.
- **Quiescence barely helps (+1).** The one fixture quiescence recovers is the tell: it recovers it
  by *resolving the capture sequence* and seeing who comes out ahead. The static eval can't see that
  — because the static eval doesn't model exchanges. An exchange-aware static eval captures that same
  signal **without** the search cost.

Conclusion: the gate is **eval-signal-bound** — fix the signal first (below); *then* the depth
re-test becomes meaningful (it isn't, today, because every leaf is garbage).

## The specific gap (all three grounded in current code)

Every tactical query counts *raw* attacks/attackers and ignores whether the exchange is actually won:

1. **Threats ignore the exchange outcome (no SEE).** `query_threats`
   ([queries.rs:99](hornet-engine/src/queries.rs#L99)) adds ¼·value(target) for *any* piece whose
   line hits *any* enemy at the first occupant — folded into Pᵢ
   ([queries.rs:243](hornet-engine/src/queries.rs#L243)). It is keyed on target value alone, with no
   least-valuable-attacker (LVA) or defender check, so it credits a **queen attacking a defended
   pawn** (QxP = −800 by SEE) the same positive sign as a **pawn attacking a hanging queen**
   (PxQ = +900). Note the inverse of the naïve intuition: a pawn attacking a *defended* queen is a
   real threat (PxQ then recapture = +800) — "defended" negates a threat only when LVA ≥ target.
2. **King-safety discards the attacker value it already computes.** `classify_reachers`
   ([queries.rs:134](hornet-engine/src/queries.rs#L134)) returns `(defenders, attackers, attack_value)`,
   but `safety_scalar` ([queries.rs:199](hornet-engine/src/queries.rs#L199)) collapses to
   `defenders − attackers + escapes` — **`attack_value` is thrown away**. A king pressured by a queen
   and by a pawn score the same. The ingredients for a better signal are already in hand and unused.
3. **Crossfire ignores defenders.** `query_crossfire`
   ([queries.rs:208](hornet-engine/src/queries.rs#L208)) penalises a piece only at ≥2 enemy attackers
   and never counts defenders — a doubly-attacked but doubly-defended piece is penalised as if it
   were hanging.

The unifying defect: **no component asks "would this exchange actually win material?"** That is the
exact judgement a 3000-Elo human makes and the engine cannot.

## Read these first (verify everything yourself)

- [eval.rs](hornet-engine/src/eval.rs) — `eval_4vec` (`:35`), `compute_utility` (`:63`, the
  mean-relative / zero-sum step), weights (`:19`), and the bounty fold into Oᵢ (`:68`).
- [queries.rs](hornet-engine/src/queries.rs) — the four queries + `query_threats`. **The reacher
  data you need already exists per square:** `lines.reachers_at(sq) -> SquareReachers` with
  `piece_indices`/`count`, and `lines.pieces[pi]` gives `.player` + `.piece_type.eval_value()`
  (used in `classify_reachers`, `:134`). `ReachEntry.first_occupant` is the first blocker on a
  piece's line (`query_threats`, `:103`).
- [lines.rs](hornet-engine/src/lines.rs) — `LineMap` (always-recompute per Hard Rule #5). For a
  *latent*-threat extension, `ReachEntry.xray_continues` (`:31`) projects past the first blocker
  — **[verify the field name/semantics before relying on it].**
- `examples/strength_gate.rs` (the gate) and `examples/gate_ablation.rs` (the sweep above);
  fixtures in `baselines/tactical_samples.json` (13 testable). This is your oracle — match rate
  before/after.
- `HORNET-BUILD-SPEC.md` §4 (queries) and the Hard Rule that V stays four components.

## The proposal

Make threats, king-safety, and crossfire **exchange-aware** using reacher data already computed:

- **Threats:** count ¼·(target value) only when the capture is profitable — target undefended, **or**
  the cheapest attacker on that square is worth less than the target. (The reacher list at the
  target square gives both attacker values and the defender count.)
- **King-safety:** fold the discarded `attack_value` in as **net pressure** — scale it by the attacker
  *surplus* (gate on `attackers > defenders`), **not** raw (raw re-introduces the counting bug: an
  over-defended king would look as exposed as a bare one) and **not** a raw attackers/defenders ratio
  (unstable — division blows up exactly when `defenders = 0`, the dangerous case). Undefended → full
  weight; over-defended → ~0.
- **Crossfire:** net **same-player** defenders against the (multi-opponent) attacker set, keeping the
  ≥2 convergence gate (`attackers > defenders AND attackers ≥ 2`). In FFA a non-owner piece on the
  square is an attacker, never a defender — no one recaptures to save another player's piece — so
  defenders are the owner's pieces only. The genuine 4PC danger (two *different* opponents converging)
  is already in the all-opponent attacker count.

These enrich the **existing** Pᵢ/Sᵢ/Oᵢ — **no 5th component** (Hard Rule #4), no new module, no
per-piece intent tensor (the full tensor per node was reverted for a 5× slowdown; this reuses the
reacher counts the queries already walk, so the per-node cost is near-flat).

## The decision you own: how much exchange to resolve statically

This is the real fork (like "where do the bounds come from" was for shallow pruning). More resolution
= sharper ranking = more per-node cost. Pick one, ship it **default-off with an ablation arm**, and
let the gate match rate decide:

1. **Defended-flag.** A threat/attack counts only if `attackers > defenders` at the target. Cheapest
   — both counts already exist in `classify_reachers`. Misses value asymmetry (defended pawn vs
   defended queen look the same).
2. **Cheapest-attacker check.** Profitable iff `min(attacker value) < target value` OR target
   undefended. Captures "pawn-threatens-queen = real, queen-threatens-defended-pawn = not." Needs the
   min attacker value per square — one pass over the existing reacher list.
3. **Lightweight static exchange evaluation (SEE).** On each contested square, play out
   attackers-vs-defenders cheapest-first → net material; use the sign/magnitude. Sharpest, bounded
   (only contested squares), most code. This is the static analogue of what quiescence does at the
   leaf — and the diagnostic says that resolution is exactly what's missing.

Recommendation: start at **2** (cheapest-attacker) — it's the smallest change that fixes the three
gaps above, and it's the level the evidence points at. Escalate to **3** only if the ablation says
the value asymmetry matters and the cost is affordable.

**Scope discipline (why 2 is a floor, not an under-shoot):** the static eval's job is to filter the
*obviously* dead threats (defended, attacker ≥ target), not to resolve every exchange. The recursive
cases — pinned/overloaded defenders, multi-move combinations, the 4PC capture race (Yellow threatens,
Blue takes) — are **search's** job, and search is exactly what fixing the eval unblocks (see the
depth note above). The per-player framing is right: each component reflects *that* player's own
profitable captures; who wins the capture race is a tempo/search question, not a static one. Don't
over-build the static eval toward full SEE — cheapest-attacker is the floor, the gate is the
escalation trigger.

## Constraints (engine design rules — honor all)

- **V stays M/P/S/O (Hard Rule #4).** Enrich the existing components; do not add a component or merge
  queries. Each component still traces to one query.
- **Default-off + ablation arm.** Gate the exchange-awareness behind a flag (mirror the eval-weight
  style) so the gate can measure on-vs-off. New strength levers ship disabled.
- **Always-recompute budget (Hard Rule #5).** `eval_4vec` runs per leaf and recomputes lines every
  call; the eval has a per-call budget (the debug-mode test asserts < 600 µs,
  [eval.rs:145](hornet-engine/src/eval.rs#L145)). Reuse the reacher walk the queries already do —
  don't add a second geometric pass.
- **Zero-sum must survive.** `compute_utility` relies on `Σᵢ Uᵢ ≈ 0`
  ([eval.rs:171](hornet-engine/src/eval.rs#L171) tests ±5) for shallow-pruning bounds. The
  mean-relative step handles this as long as the enriched components stay per-player scalars — keep
  them so.
- **Engine-only.**

## Done looks like

- The three queries are exchange-aware behind a default-off flag; on-vs-off is an ablation arm.
- **The gate match rate moves above 0/13** with the flag on (the whole point — if it doesn't, the
  hypothesis is wrong; say so rather than shipping a no-op). Report the per-fixture before/after.
- Unit tests: a pawn attacking a *hanging* queen scores ~value(queen); a pawn attacking a *defended*
  queen still scores (~value(queen) − value(pawn), the recapture cost); a **queen** attacking a
  *defended* pawn scores ~0 (LVA ≥ target); a doubly-attacked-doubly-defended piece gets no crossfire
  penalty.
- `eval_4vec` still < 600 µs in the debug-mode test; `Σ Uᵢ` still within ±5.
- `cargo test` green.

## Landmines (found while scoping this)

- **A threat signal already exists** (`query_threats`) and is wired into Pᵢ — this is an *edit* to
  existing queries, not a greenfield feature. Don't double-count it.
- **`safety_scalar` drops `attack_value`** — fixing that alone is a free upgrade, independent of the
  rest.
- **"Defended" is recursive.** A defender that is itself pinned/overloaded isn't a real defender; full
  SEE handles this, the defended-flag does not. Don't over-claim soundness for options 1–2.
- **Quiescence (= "TRS") already partially covers this at the leaf.** An exchange-aware static eval
  and quiescence overlap; measure them together (the gate sweep has both axes) so you don't attribute
  the same +N twice.
- **Mean-relative normalisation hides absolute scale.** A sharper component changes every player's
  deviation, not just one — sanity-check the zero-sum test after each change.

## Verify

```
cd hornet-engine
cargo test                                   # keep green, add the exchange tests
cargo run --release --example gate_ablation  # the match-rate oracle (0/13 today)
```

If the gate doesn't move, the exchange hypothesis is falsified for these fixtures — report that, with
the per-fixture detail, rather than shipping a lever that changes the eval without changing the gate.
