# Claude's review — Kimi's independent eval plan

**Reviewing:** `KIMI-INDEPENDENT-PLAN-eval-next-phase.md` · **Date:** 2026-06-08

**Verdict:** Strong, and the independence paid off. We converged on the diagnosis (validation) and she
improved on our build in three concrete places. Below: the convergence, where she's *better*, where the
two plans are *complementary*, and the merge.

## 1. Convergence = validation (the big result)
Working without our fixes, Kimi independently reproduced the entire diagnosis:
- **Safety huddle trap** — `defenders·40` rewards parking pieces near the king = passive play = the
  tuner negates it. (Identical to EXP-018.)
- **No win-condition signal** — "the eval is a local tactical snapshot, not a strategic objective
  function ... why self-play is drawish." (Identical to EXP-016/017.)
- **Scale mismatch + PST noise**, and even the **zero-sum / `SUM_UB=0`** property (matches ENGINE-MATH).

Two independent paths to the same root cause is the strongest confirmation we could get that we're right.

## 2. Where Kimi is *better* (adopt these)
1. **King-danger as a non-linear S-curve table** (Part 5.3, Glaurung-style attack-units → danger). King
   safety is *established* as non-linear — multiple attackers compound. My `king_danger_scalar` is a
   linear scalar with a hard clamp; her table is the right shape. **Adopt the table shape.**
2. **Mean-relative king-danger preserves zero-sum.** She keeps *every* term mean-relative, so `Σ=0`
   holds and the Sturtevant–Korf pruning bound survives. **My search-side king-danger is an *absolute*
   per-player penalty, which breaks `Σ=0` (ENGINE-MATH §7.1).** Her formulation fixes that constraint.
   Trade-off: mean-relative measures danger *relative to the field* rather than absolute — the A/B can
   adjudicate, but the zero-sum preservation is a real, free advantage.
3. **Her win term keeps Hard Rule #8 intact.** Her `elimination_proximity` is built from *material +
   king-danger*, **not points** — so it's win-aware *without* putting points in the eval. That defuses
   my main objection to an eval-side win term (it doesn't relax #8 after all). Credit where due.
4. She **correctly credited the user's swarm concept** (three prior projects, ant-colony) — good.

## 3. Where the two plans are *complementary* (not competing) — the four-C lens
Her win term and mine are **different signals**, and likely both:
- **Mine (banked FFA points):** fires *throughout* the game, every capture/elimination — a *scoring*
  driver.
- **Hers (elimination-proximity = weak opponents):** fires *late*, only once an opponent is already
  collapsing (low material **and** attacked king, multiplicatively) — a *finishing* gradient.
They **complete** each other: hers presses a weak opponent toward the kill; mine rewards actually
banking the points along the way. The likely answer is *both*, not either/or.

## 4. Refinements / pushback
- **Her proxy is silent in the opening/midgame.** `prox = mat_weakness × danger` needs *both* low
  material and an attacked king, so for four healthy players it's ≈0 → no signal → it won't de-drawish
  the *early* game on its own. Pair it with a scoring signal (mine) or it only kicks in once someone's
  already losing.
- **N-weight tuning on a 32-game / 1270-position corpus** risks overfitting with 7+ terms. Her own
  "self-play primary, base-predicate-first" rules mostly handle this — keep them strict.
- **Perf:** eval is already 11 µs/node with the line projection at 66% (perf_breakdown). Seven more
  query terms is fine under her <600 µs gate, but watch the line-projection recompute as terms stack.
- **One speculation of hers is wrong, and in our favor:** she guessed I prioritize `move_tune`; we
  *both* prioritize self-play. So that "divergence" is actually agreement.

## 5. The merge (best of both)
1. **Win term:** build *both* signals — her eval-side, mean-relative, points-blind elimination-proximity
   (finishing) and my search-side banked-points (scoring) — and A/B all combinations. Likely keep both.
2. **King-danger:** keep the **search-side placement** (validated 83%) **OR** her mean-relative eval-side
   (zero-sum-preserving) — A/B — but in *either* case upgrade the **shape to her non-linear table**.
3. **Relational terms:** her unbundled plan (pawn structure split into iso/doubled/connected, rook-open,
   outpost-all-3-enemies, swarm, targeted mobility) — solid, eval-side, unified N-weight tuning. Proceed.
4. **Tuning hierarchy:** self-play > move-agreement > texel — agreed, already ours.
5. **Architecture:** her `EvalTerms` unbundling + N-weight `move_tune` is the right refactor for the
   relational terms regardless of where the objective layer lives.

**Net:** the diagnosis is settled (two independent confirmations). On builds, she hands us a better
king-safety *shape*, a zero-sum-preserving formulation, and a complementary second win signal — all
adoptable. Nothing here conflicts that the self-play A/B can't decide.
