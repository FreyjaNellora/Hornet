# How the Hornet engine's math works

A plain-but-precise account of the math the engine runs on, so diagnosing it is reasoning, not
guessing. (Companion to EXP-016, which uses this to explain the depth-pathology.)

## 1. The value model — Max^n (vector minimax for 4 players)
Every position gets a **4-vector**, one number per player:
```
V(s) = ⟨U_R, U_B, U_Y, U_G⟩          // U_i = "how good s is for player i", in centipawns
```
Search is **Max^n** (the N-player generalization of minimax). At a node where player `p` is to move,
`p` is assumed to pick the child that maximizes **its own** component:
```
value(node) = value( argmax_child  child.value[p] )     // the WHOLE vector of the chosen child backs up
```
Leaves get `V(leaf)`. The root move played = `argmax over root children of child.value[root_player]`.
Hard Rule #3: never collapse to a scalar — the full vector backs up, because "who's 2nd vs 3rd" matters
to how the others will play.

**The critical property (and the catch):** Max^n is only *correct* if **every opponent actually
maximizes its own `U`**. Unlike 2-player minimax, there is **no theorem that deeper Max^n is better** —
if opponents don't play that way (different config, or the eval is wrong), deeper search commits harder
to a wrong model. This is the root of EXP-016.

## 2. The evaluation — mean-relative, zero-sum by construction
Each component is made **relative to the 4-player mean**, then weighted:
```
U_i = w_M·(M_i − M̄) + w_P·(P_i − P̄) + w_S·(S_i − S̄) − w_O·(O_i − Ō)
        where  X̄ = (X_R + X_B + X_Y + X_G) / 4
```
- `M` material, `P` positional, `S` king-safety, `O` crossfire (SEE material-at-risk; **subtracted** —
  it's a liability). Deployed weights today: **(w_M, w_P, w_S, w_O) = (6, 0, 0, 1)** — i.e. material +
  crossfire only; positional and safety are switched off because tuning zeroed them.
- **Mean-relative ⇒ `Σ_i U_i ≈ 0`** (off by ≤3 from integer rounding). The eval is **zero-sum by
  construction.** This is deliberate: it makes Sturtevant–Korf shallow-pruning bounds tight (`SUM_UB =
  0`), and it expresses "my score relative to the field."
- **Points-blind (Hard Rule #8):** `V` is in centipawns and never sees `ffa_points`. King-capture /
  elimination / the +20 are **not** in the eval; they're handled at game-flow + in move ordering.

**This is the incomplete model.** `V` says the game is zero-sum material. The *true* value `V*_i` =
expected final FFA points, and FFA is **not** zero-sum: total points aren't conserved, eliminations pay
+20, and placement/alliance/kingmaker dynamics are non-transitive (being the material leader makes you
the *target*). So `argmax U ≠ argmax V*`. §5.

## 3. The search shapes — and their cost in branching `b`, depth `d`
Internal branching `b ≈ 30–40`. Three shapes, all keeping Max^n as the value model:
| shape | what it keeps | cost | sound? |
|---|---|---|---|
| full-width Max^n | every move at every node | `b^d` (intractable past d≈8) | yes |
| **laser** (`deep_floor 1`) | one line deep (beam→1 below the root rotation) | ~linear in `d` | **no** — prunes to a single line; can drop the best move; **depth-unstable** (it wandered at d20). *Discarded.* |
| **flashlight** (level beam, cap `W`) | top-`W` nodes per *level*, ranked by the mover's own eval-gain; Max^n backup over the kept tree | `~ W·b·d` evals | **approaches** sound: at `W=∞` it is *exactly* full-width Max^n (validated by test). At finite `W` it prunes, but **breadth-bounded, not depth-collapsed** → move-stable. *Chosen.* |

The flashlight is the deep mechanism (move-stable + linear-ish). Its one unsoundness is the cap: at
finite `W` it can still prune the true-best line — which the cap-spectrum in EXP-016 tests for.

## 4. Terminal & DKW scoring (search side)
- **No legal moves** → terminal. The mover's own component is overwritten with a mate-distance value
  `U_mover = −(MATE − ply)` (so faster mates are preferred / delayed-by-the-mated-side); the other three
  keep their positional value.
- **DKW (Dead-King-Walking):** a mated/stalemated player's king walks randomly (no agency); its other
  pieces freeze (un-capturable walls); a captured DKW king / DKW-stalemate removes the player. Point
  awards (+20 stalemate, +10 survivors) are **game-flow**, not eval (Hard Rule #8). In search, a DKW
  node is modeled as expectimax (average over the random king moves).

## 5. Why the math makes depth *hurt* (the EXP-016 core)
Three independent ways Max^n + this eval breaks "deeper is better":
1. **Beam pruning** (search): a true-best line that looks bad to `V` near the root is cut and never
   deepened. Shrinks as `W→∞`.
2. **Opponent-model mismatch** (Max^n): backup assumes opponents maximize `U`; if they don't, deeper =
   more committed to the wrong model. Intrinsic to Max^n.
3. **Eval-incompleteness** (the eval): `U ≈ relative material`, but the real objective `V*` is
   FFA-points / placement and **non-zero-sum**. Optimizing `U` harder (more depth) drifts *further* from
   `V*` — e.g. deeper search hoards material → becomes the leader → gets ganged up on → scores worse.
   **Not fixable by search.** This is the Gödel-incompleteness reading: search can't reach what the
   model can't express; more search amplifies the blind spot.

(2)+(3) are pruning-independent; (1) is pruning-monotone. EXP-016's cap-spectrum tells them apart.

## 6. The tuning math — fitting the weights
- **Texel:** quiet positions labeled by game result `r ∈ {0, ½, 1}`; map `U → winprob = σ(K·U)`; minimize
  `E = Σ (r − σ(K·U))²`. (Our `texel_tune`; the seat-order label bug that scrambled `r` is fixed.)
- **Move-agreement:** fit weights so the strong human's move is `argmax_m U(child_m)[p]` as often as
  possible. (Our `move_tune` / `move_match`. Sensitive where MSE is noise-blind; but top-1 is
  tactics-dominated, so it under-reads smooth positional value — that shows in outcomes instead.)
- **SPSA / self-play A/B:** for params that change *which move you play* but don't predict outcomes
  (search knobs, the win-term), the gate is **self-play win-rate**, not MSE/move-match. (Our
  `selfplay_ab`.)
- All of these are just optimizers over the weights; doing them *well* (regularized logistic regression,
  proper optimizers) is what the Python `tools/` are for — see tools/README.

## 7. The objective layer (win term + king-danger) — and the new constraints it creates
The search value is no longer just cp. At each flashlight leaf:
```
value_i = cp_eval_i  +  win_weight·(points_i − p̄)  −  danger_weight·king_danger_i/100
            └ means ┘    └──── goal: zero-sum ────┘    └── danger: NOT zero-sum ──┘
```
This makes depth pay and de-drawishes play (EXP-017/018), but it introduces real new complications —
this is the "where do the new constraints arise" breakdown:

1. **King-danger breaks zero-sum (the big one).** `cp_eval` and the win term are both mean-relative, so
   `Σ_i = 0`. King-danger subtracts an *independent* per-player penalty, so `Σ_i value = −danger_weight·Σ
   king_danger/100 ≠ 0`. The value vector is **non-zero-sum for the first time.**
   - *Fine for the flashlight's Max^n backup* — Max^n never needed zero-sum (each player just maximizes
     its own component).
   - *Forecloses zero-sum pruning* — any Sturtevant–Korf shallow pruning (`SUM_UB = 0`) is now unsound
     with danger on; it'd need danger made mean-relative, or the pruning disabled when danger is on.
   - *Conceptually more correct*: FFA isn't zero-sum, and "everyone's king can be in danger at once" is a
     common-bad, not redistributive. This is a **deliberate, correct break** — but it changes the math.

2. **The points↔cp unit bridge (`win_weight`) is a sensitive knob.** points (pawn 1 … king 20) vs cp
   (pawn 100). Too high → a king-capture (+20·win_weight) dominates → over-pursuit of king-hunts (the old
   pathology, now only partly bounded by "the search values only *reached* captures"). Too low → the
   objective is ignored (back to EXP-016). The usable weight is a window, found by A/B.

3. **Win term ↔ point-grabbing horizon.** points are monotonic; a leaf's win term reflects points banked
   *along its line*, biasing toward lines that score sooner. cp counters bad grabs (a losing capture
   shows in cp material), so the net bias is toward *good* captures + eliminations — intended — but only
   while `win_weight` stays balanced.

4. **Variable-depth leaves × accumulated points (flashlight-specific).** The level-beam leaves pruned
   nodes shallow and frontier nodes deep; deeper leaves banked more points en route → a structural
   points-edge independent of position quality. Largely self-correcting (captures are also in cp material,
   which carries quality; the eliminations it favors *are* the goal), but it's an asymmetry the pure-cp
   flashlight didn't have. Watch for over-valuing deep capture-heavy lines.

5. **King-danger vs crossfire (O) — NO double-count (resolved).** crossfire's SEE explicitly *excludes
   the king* (`queries.rs:458`: "king: capture is terminal (search handles it), not a material threat"),
   so crossfire = non-king material-at-risk (a cp *means* signal) and king-danger = the king/elimination
   threat (the points *goal* signal). They're **complementary, not redundant** — king-danger fills
   exactly the gap crossfire leaves. (This is also why the safety A/B gain stacked cleanly on top of
   crossfire rather than washing out.)

6. **Max^n's opponent-model assumption is untouched.** The objective layer narrows eval-incompleteness
   (EXP-016 cause 3) but Max^n still assumes each opponent maximizes its own (now objective-aware)
   component; against a differently-configured opponent (cause 2) deeper search can still mis-predict.
   Only a real opponent model fixes that.

**Headline:** the objective layer's deepest structural change is that **king-danger makes the value
non-zero-sum** — right for FFA, but it ends the zero-sum pruning option and adds the unit-bridge and
double-count knobs to manage.
