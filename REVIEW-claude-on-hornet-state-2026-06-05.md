# Review: Claude on Hornet state — 2026-06-05

**Reviewer:** Claude (fresh-eyes pass after ~4 days away)
**Sources read:**
- `STATUS.md` (production board, 2026-06-04 update)
- `HANDOFF.md` (shift handoff, 2026-06-02 overnight)
- `ENGINE-HANDOFF.md` (engine state)
- `hornet-engine/src/` (module inventory)
- `PITCH-maxn-shallow-pruning.md` (~lines 1–60)
- `PITCH-strength-gate.md` (full)
- Root file listing (root + `baselines/` + `Playbook/`)

**Not yet read:** the new pitches for `bounty`, `intent`, `zones`;
`COMMS_*` thread in full; phase files under `phases/`; session notes
under `sessions/`.

---

## State snapshot

- **P0–P6 complete with refinements** (terminal scoring §1.8, iterative
  deepening, killers + history). Full pipeline runs board → move-gen →
  lines → queries → eval → Max^n end-to-end.
- **Engine artifacts:** ~66 tests green, `cargo test` clean, clippy
  clean, fmt applied. `perft 20/395/7800/152050` matches reference
  (Freyja prototype). FEN4 byte-identical round-trip + PGN4 corpus
  replay 2532/3770 plies (remainder bounding at DKW).
- **Hard invariants honored:** `eval_4vec → [i16; 4]` not a scalar;
  depth rounded to multiples of 4 (`round_to_rotation` in `search.rs`);
  FEN4/PGN4 native; V decomposition `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`
  fixed in `eval.rs`.

---

## What's strong

**Pace + quality together.** This is much further along than I'd expect
for ~4 days. Not a paper engine — actually plays 4PC end-to-end with
real perft, real corpus replay, real make/unmake with elimination
tracking. The reference perft sequence matching the prototype is a
strong correctness signal.

**The Playbook isn't ceremony — it's load-bearing.** `STATUS.md`,
`HANDOFF.md`, `sessions/`, `change-orders/`, `dispatch_comms.jsonl`,
`agent-conduct.md` all populated and active. The "factory model" with
phase ownership is doing real work: claude's lanes are
`board/`, `move_gen.rs`, `lines.rs`, `zobrist.rs`, `tt.rs`, `search.rs`,
`move_order.rs`; Kimi's lanes are `queries.rs`, `eval.rs`, `nnue/`.
Lane discipline is real, not aspirational.

**Pitch-then-build cycle is producing focused docs.**
`PITCH-maxn-shallow-pruning.md` is a genuinely good pitch — it
correctly identifies "where do the bounds come from" as the real design
decision, not the algorithm itself. Cites specific line numbers in the
existing code and the relevant Sturtevant & Korf 2000 reference.
Implementer would know what's mechanical (the recursion) and what's
design (the bound source).

**Spec v0.2 landed** via dispatch-authorized CO-001 resolution.

---

## Three concerns worth pushing on

### 1. Shallow pruning is deferred but it's the next gate

Listed in `ENGINE-HANDOFF.md` as roadmap item #1 ("The main speed win;
it also lets the transposition table cache real bounded values instead
of being ordering-only").

`STATUS.md` flags it: "shallow pruning (deferred — bounds decision)."

The pitch is written and the algorithm side is settled. The remaining
substantive decision is the **source of `SUM_UB` and `COMP_LB`** for the
bound computation — material-only (tight, simple), or material plus the
loose `Pᵢ` / `Sᵢ` bounds (more pruning, more compute). Kimi's prior
recommendation (in the spec-review verification thread) was
**material-only first**, add P/S bounds only if ablation says so. That
is a defensible starting position.

**Why this gates everything else:** the strength-gate pitch
(`PITCH-strength-gate.md`) explicitly says depth 8 is intractable
without pruning and uses depth 4 as the workable depth for now. Without
shallow pruning, the strength gate is undermeasuring.

### 2. Three new modules outside the V contract

Source tree now contains `src/bounty.rs`, `src/intent.rs`,
`src/zones.rs` — each with a corresponding pitch
(`PITCH-ffa-bounty-scoring.md`, `PITCH-piece-intent-tensor.md`,
`PITCH-secondary-zones.md`).

The V contract was supposed to be closed:
**`V = ⟨U₁, U₂, U₃, U₄⟩` with `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`**
and each component traced to exactly one query class. That's the rule
listed in both `PITCH-for-new-agents.md` and the spec.

Three readings of the new modules:

- **Best case:** they're additive ablation knobs, each shipped
  default-off with an arm, per the additive discipline. The closed V
  contract is intact in default eval.
- **Middle case:** they extend V with a fifth/sixth/seventh component
  added during composition. V is no longer closed at four.
- **Worst case:** they replace components in eval without flagging
  themselves as ablation arms — design drift.

I haven't read the three module sources yet. **Worth confirming
the integration pattern before any of them get cited as "the V
formula" elsewhere.** If they're additive knobs, they should appear in
the spec as Section X / ablation arms. If they're V extensions,
that's a hard-rule change and the rest of the docs need to update.

### 3. Spec text discrepancies remain open (CO-002 / CO-003)

`ENGINE-HANDOFF.md` § "Known spec discrepancies" lists:

- §1.4 lists pawn promotion at the board edge. Real rule (chess.com 4PC)
  and engine: promote at the **central crossing** (rank 7 / file 7 /
  rank 6 / file 6).
- §7.3 places the en-passant capturing pawn on the wrong square in all
  four examples. Engine follows the §1.4-derived movement geometry.

`STATUS.md` says these are CO-002 / CO-003, open, Kimi to land.

**Risk:** a fresh agent reading the spec text without the
ENGINE-HANDOFF cross-reference will build to the wrong spec. The
`PITCH-for-new-agents.md` points at `HORNET-BUILD-SPEC.md` as
authoritative; if the text under §1.4 / §7.3 is wrong, that pointer
is misleading. Should land these before the next agent onboards
against the spec.

---

## Smaller flags

- **Root comms bloat.** 8 `COMMS_*` files in root
  (`COMMS_CLAUDE_HANDOFF_P4`, `COMMS_CLAUDE_HIERARCHY_ISOLATION`,
  `COMMS_CLAUDE_PERFT_RESULT`, `COMMS_CLAUDE_REPLY`,
  `COMMS_CLAUDE_REPLY-sync-2026-06-02`, `COMMS_CLAUDE_SYNC-2026-06-02`,
  `COMMS_KIMI_HIERARCHICAL_EVAL_ANALYSIS`, `COMMS_KIMI_PERFT_REPLY`,
  `COMMS_KIMI_REPLY`, `COMMS_KIMI_REPLY_TO_CLAUDE_HIERARCHY`,
  `COMMS_KIMI_SHALLOW_PRUNING_ANALYSIS`,
  `COMMS_KIMI_SHALLOW_PRUNING_SOLUTION`). Natural for active
  back-and-forth, but starting to clutter root. Once threads close,
  summarize the resolution into the relevant `sessions/{phase}/…` note
  and archive the raw comms — keep the root scannable for new agents.

- **Two FEN4 dialects.** `baselines/tactical_samples.json` uses a
  non-native (`xxx`-corner) FEN4 dialect the parser doesn't read.
  `PITCH-strength-gate.md` correctly side-steps via `moves_to_replay`,
  but this is now a known parser gap with a documented workaround.
  Worth either porting the fixtures to the native dialect or implementing
  the second-dialect read, eventually.

- **Strength gate is pending.** Per Hard Rule #7 it gates NNUE training.
  Right now the harness is pitched, not implemented. Once shallow
  pruning lands and the harness is wired up, this is the next major
  decision gate. The match-rate primary signal in
  `PITCH-strength-gate.md` is the right minimum-viable metric to start
  with.

- **DKW move-gen deferred.** Spec clarification needed on `T`/`S`/`R`
  markers. Blocks 100% corpus replay (currently 2532/3770; remainder
  bounded at DKW). Not blocking P6 or eval — fine as deferred.

---

## Net

The architecture decisions held up. The V vector, BFS lines, native
FEN4/PGN4, depth-multiples-of-4, eval-vs-FFA-points distinction — none
of these have been re-litigated. The Playbook framework is doing real
coordination work, not ceremony. The engine is actually playing 4PC
end-to-end with proper rule mechanics.

**Primary risk now:** design drift via the three new modules
(`bounty.rs`, `intent.rs`, `zones.rs`) — needs an integration check to
confirm whether the closed V contract is still intact in default eval.

**Primary next decision:** shallow pruning bound source. Until that
lands, depth 8 is intractable per the strength-gate pitch, and the
strength gate undermeasures.

**Primary cleanup:** land CO-002 / CO-003 so the spec text matches
engine behavior before the next agent onboards against it.

---

## Design refinement: piece intent tensor — Vulnerability split + exploitation mirror

**Origin:** Extension of `PITCH-piece-intent-tensor.md`. Refines the
"Targeted Intent Matrix" target-vector components after flagging
"Vulnerability" as conflating two distinct tactical events. Belongs
here because it's the design behind `intent.rs`, which Concern #2 above
flags as needing V-contract reconciliation.

### The conflation that needs fixing

The original target-vector design lists four components per opponent:

```
T_p = [Direct Offense, Prophylaxis, Vulnerability, X-ray Intent]
```

"Vulnerability" is doing double-duty for two unrelated tactical events:

1. **What can attack THIS piece** — passive incoming threat. Action it
   recommends: defend or move away.
2. **What THIS piece is shielding** — friendly pieces depending on this
   piece's position; if this piece moves, they become attackable. Action
   it recommends: don't move this piece; if you must, the discovered
   attack lands first.

Summing these into one scalar smears both signals. They behave
differently in tactical decisions and should be separate channels.

### The full split

#### Per-opponent target vector (5 components)

For each piece × each of 3 opponents, store:

| Component | Direction | Meaning | Geometric source |
|---|---|---|---|
| **Direct Offense** | Out, immediate | This piece directly attacks opponent's pieces | Lines from this piece hitting enemy at distance 1+ with no blocker |
| **X-ray Intent** | Out, latent | This piece pressures opponent through blockers | Slider lines with first_occupant + continuation to enemy beyond |
| **Direct Threat** | In, immediate | Opponent directly attacks this piece | Mirror of Direct Offense from opponent's side onto this square |
| **Latent Threat** | In, latent | Opponent X-rays this piece through blockers | Mirror of X-ray Intent from opponent's side onto this square |
| **Prophylaxis** | Defensive posture | This piece's preventive positioning against opponent's plans | TBD operationally (see open question below) |

Note the clean 2×2 mirror structure for offense/threat:

```
              IMMEDIATE          LATENT
OUT     Direct Offense     X-ray Intent
IN      Direct Threat      Latent Threat
```

These four can all be read from the existing `LineMap` and its per-square
inverse index in `lines.rs` — no new geometric computation, just per-piece
aggregation over the inverse index entries.

#### Per-piece structural scalar (1 component, not per-opponent)

**Discovery Liability** — friendly pieces this piece currently shields.
If this piece moves, those friendlies become attackable.

This is **not per-opponent** — it's a property of this piece's geometric
position relative to friendly pieces, independent of which opponent we're
looking at. Lifting it out of the per-opponent target vector and storing
it once at the piece level keeps the per-opponent vectors symmetric.

Operational definition: for each friendly piece F whose threat-status
would change if THIS piece moved off its current square, count F's
eval_value weighted by the threat's severity. Read from the inverse
index by simulating piece removal at this square.

#### Final per-piece intent layout

```
I_piece = {
    discovery_liability: i16,
    target_vectors: [T_p1, T_p2, T_p3],  // one per opponent group
}

T = {
    direct_offense: i16,
    xray_intent: i16,
    direct_threat: i16,
    latent_threat: i16,
    prophylaxis: i16,
}
```

5 × 3 + 1 = **16 i16 values per piece** for tactical signals.

Plus the existing components from the original pitch:
- Occupant class (one-hot piece-type vector)
- Kinematic Reach (160-bit mask)
- Mobility Mask (160-bit mask)

### The exploitation mirror

For each component on a piece, there's an **exploitation pattern** an
opponent could use to convert that component's value into actual material
or positional gain. The exploitation patterns are not a new data structure
— they're a *read protocol* on the same intent tensor from the opposite
perspective.

| Component (high value means…) | Opponent exploitation pattern |
|---|---|
| Direct Threat | **Capture** — take the piece, win the exchange |
| Latent Threat | **Setup** — move the blocker so the latent threat becomes real |
| Discovery Liability | **Pin / Skewer** — restrict the piece, or force it to move to reveal the friendly behind it |
| Direct Offense | **Block / Trade** — interpose, or accept the attack and counterattack |
| Prophylaxis | **Probe** — find the gap in the preventive net; force a commitment that undoes the prophylaxis |
| X-ray Intent | **Reinforce blocker** — shore up the piece in front of the X-ray so the latent never realizes |

**Uses for the exploitation mirror:**

1. **Move ordering at opponent turns.** When the search descends into an
   opponent's node, candidate moves are scored partly on which
   exploitation patterns they advance. Moves that capture a piece with
   high Direct Threat, or setup a Latent Threat by moving its blocker,
   get ordered first.
2. **NNUE policy head signal.** The network learns which exploitation
   patterns are productive in which positions. Policy priors come out of
   this naturally — high-Direct-Threat pieces get high prior for the
   capture move targeting them.
3. **Move-quality criticality.** Phase 3 criticality-style signal can
   read the exploitation list to score moves: a move that converts an
   exploitation pattern is critical; a move that ignores high-value
   exploitations is suspect.

The mirror isn't a parallel computation. It's the same intent tensor,
read with the question "what would I exploit about my opponent's piece"
instead of "what should I defend about my piece."

### Open implementation questions

1. **Prophylaxis operational definition.** The other components are
   geometric and read directly off line projections. Prophylaxis is more
   abstract — "preventive positioning." Candidate definitions:
   - Squares this piece's reach covers that opponent's most active pieces
     would otherwise want to occupy
   - Number of opponent moves this piece's presence eliminates from
     opponent's legal-move set
   - Defensive coverage of king-vicinity squares against opponent's
     line-projection reach

   These all measure different aspects of "prevention." Pick one,
   document it, check it doesn't double-count signal already in Direct
   Offense or Direct Threat.

2. **Discovery Liability evaluation cost.** Computing it requires
   simulating "what if this piece weren't here" for every piece. Naive
   implementation is `O(pieces × pieces_behind × attack_geometry)`. A
   smarter version walks the inverse index: for each enemy line ending
   at this piece (Direct Threat entries), check if continuing past this
   piece reaches a friendly higher-value target. That's `O(direct_threats
   × ray_continuation)`. Almost certainly the way to do it.

3. **Storage cost per position.** 16 i16 = 32 bytes for tactical signals,
   plus 40 bytes for occupant+mask, plus the piece bookkeeping itself.
   Per piece: ~80-100 bytes. Per position with 64 pieces: ~5-6 KB. Real
   but bounded. Acceptable if `intent.rs` is built once per leaf eval
   (always-recompute, like `lines.rs`); concerning if built once per
   search node interior.

4. **Reading order — outbound first, then inbound from inverse index, or
   joint pass?** The cleanest implementation builds the outbound side
   from per-piece line projection (already done in `lines.rs`), then
   reads inbound directly from `square_reachers` at each piece's square
   (Direct Threat = count enemy reachers; Latent Threat = count enemy
   reachers whose X-ray continues past me). Single pass per piece.

### Relationship to the V eval contract

This addresses Concern #2 above — at least for `intent.rs`.

**Intended reading:** the intent tensor is the *substrate* from which V's
components are richly computed. Specifically:

- **Mᵢ** (material) — unchanged, doesn't need intent.
- **Pᵢ** (positional control) — aggregates Mobility Mask × centrality
  weight.
- **Sᵢ** (king safety) — for the king and its vicinity, reads Direct
  Threat + Latent Threat + Discovery Liability (king's escape squares
  liable for discovery).
- **Oᵢ** (crossfire) — for each of my pieces, reads Direct Threat +
  Latent Threat summed over all opponents (multi-attacker convergence).

So the intent tensor enriches V's components without changing the V
contract. Search still backs up `[i16; 4]`. Eval still returns the V
vector. The contract is intact; the computation behind each component
is just richer.

If `intent.rs` is currently implementing this pattern, the V hard rule
is respected. If `intent.rs` is being summed into eval directly as a
5th component or is replacing one of M/P/S/O, the V contract IS broken
and needs a hard-rule change order. **Verification step:** read
`intent.rs` and check how `eval.rs` consumes it.

---

## Update 2026-06-06: TRS landed; strength gate exposed eval as the bottleneck

Per `phases/phase-6-search.md` — significant change since this review
was first written:

**TRS / quiescence landed 2026-06-06.** Tactical-only leaf extension
returning a value only at a rotation boundary (`qply % 4 == 0`),
advancing mid-rotation via `make_null` / `unmake_null`, bounded by
`QUIESCENCE_MAX_PLY` (one rotation). Default-off via
`Searcher::with_quiescence`. Speed levers (LMR + adaptive beam) also
landed 2026-06-04, default-off, both stack to ~28× at depth 8.

The "TRS / quiescence context" section below (about TRS not existing)
is now stale at the algorithm level but the *roadmap-timing* discussion
still applies — what changed is the experimental result, not the
question.

### The strength-gate diagnostic — the load-bearing finding

Ran the 13 testable tactical fixtures over a depth × quiescence
ablation matrix (beam 10 + LMR + adaptive + 800k-node budget):

```
              quiescence OFF   quiescence ON
depth 4           0/13              1/13
depth 8           0/13              1/13
```

What this shape says:

- **Doubling depth (4→8) changed nothing.** Zero gain in both columns.
- **Quiescence helped marginally** (+1/13) and the gain was the same
  at both depths.
- **0/13 is an eval signal problem, not a search-depth problem.**
  Search amplifies whatever signal eval has; if eval has none, search
  amplifies nothing.
- **Capture-dense positions explode at depth 8 without the node
  budget.** The adaptive-beam "tactical completeness" guard never
  prunes captures, so capture-dense nodes fan out fully for 8 plies.
  "Depth 8 is tractable" held for the quiet start position
  (`search_bench` numbers); not for tactical positions. Real depth
  needs sound pruning + a cheaper eval, not brute force.

The phase doc names this honestly: *"with the plain-v0 eval
(`intent`/`bounty`/`zones` dormant) deeper search just explores more
mis-scored leaves."*

### What this confirms about Concern #2

The three new modules (`bounty.rs`, `intent.rs`, `zones.rs`) are
**dormant scaffolds** — implemented but not consumed by eval. The V
contract is technically intact in default eval; the leverage from
those modules is also untapped. Worst of both worlds: paid the
implementation cost without the strength gain.

The phase doc's direction is explicit: *"wire the dormant
`intent`/`bounty`/`zones` substrate + strategy layer in affordably."*
That's now the next eval-lane work, and the Vulnerability-split /
exploitation-mirror design above is the substrate it should wire in as.

### Why the "horizon effect" framing is half-right

The phase doc says quiescence "reduces — does not eliminate — the
capture-exchange horizon effect." Quiescence picked up +1/13 — that's
the exchange-horizon effect being fixed.

The remaining 12/13 isn't horizon effect in the search sense. It's
**eval-signal horizon effect**: the eval doesn't see the
strategic/positional features that turn a quiet position into a
winning or losing one. Adding depth doesn't fix that. Adding
quiescence doesn't fix that. Wiring the dormant intent/bounty/zones
substrate is what fixes it.

---

## TRS / quiescence context (original framing, now superseded by 2026-06-06 landing)

Hornet does not currently have TRS (Tactical Resolution Search /
quiescence). At leaves, search just returns the static eval. The horizon
effect is live: a leaf with an impending capture evaluates the
pre-capture position, missing the next move.

*[Section preserved for the design discussion of TRS framing. The
algorithmic claim is now stale — TRS landed 2026-06-06 per above.
The roadmap timing discussion (TRS-before-NNUE vs TRS-after-NNUE) is
also superseded: the strength-gate diagnostic shows eval signal
weakness, not horizon effect, is the dominant problem. Wire the
dormant modules first; revisit TRS-vs-NNUE timing after.]*

**Background from prior project (Freyja):** "TRS" was the project's
stand-in name for quiescence search in 4PC Max^n. The framing was: at
leaves, search forward through forcing sequences (captures, checks,
threats) until the position is quiet AND has reached a full rotation
boundary (multiple of 4 from root) — that "stays quiet for the next
multiple of 4 depth" condition. Freyja's `trs.rs` had termination
guards scaffolded (DepthLimit, NodeBudget, SingleOrZeroActivePlayer,
AllPlayersDkw, Repetition, NoContestedSquares, AllPlayersPass,
Completed) but the recursion body was a stub TODO and never landed.

**In Hornet:** not scaffolded. The concept is documented in
`TECHNIQUES-and-REFERENCES.md` under "Quiescence search with
multi-player rotation invariant" with Knuth & Moore 1975 as the
reference. No `trs.rs` module exists in `hornet-engine/src/`.

The intent tensor design above *partially* mitigates horizon effect —
Direct Threat and Latent Threat give the static eval much richer
"what's about to happen" signal at the horizon — but enriching static
eval is not the same as extending search through forced sequences. A
3-ply forced sequence (capture, recapture, recapture) still needs
actual search.

**Roadmap timing question:** TRS before NNUE (improve teacher quality
so distilled net inherits horizon-aware judgment), or TRS after NNUE
(let the net learn horizon-aware static eval and possibly skip TRS)?
The strength gate result decides — if hand-tuned eval + intent tensor
clears the bar, TRS can wait; if not and the failures are horizon-effect
blindness, TRS is the fix.

---

## Recommendation: layered approach to the eval bottleneck

Given the 2026-06-06 diagnostic (0/13 on tactical fixtures, depth
amplifies nothing, dormant modules untapped), my recommendation is
**five steps in this order**, with stop conditions between them so we
don't over-invest in any one step that doesn't move the needle.

### Step 1 — Diagnostic: characterize the v0 eval's failure mode (~1 hour)

Before changing anything, run v0 eval on the 13 fixture positions and
log:
- The V vector at the fixture's target position
- The V vector at the human's chosen move
- The V vector at the engine's chosen move
- The eval's per-component breakdown (Mᵢ, Pᵢ, Sᵢ, Oᵢ for the moving
  player) for each

What we're looking for:
- **All V vectors near-equal** → eval is blind / discrimination-flat,
  even tactical signals don't register. Suggests integration bug or
  trivially-zero components.
- **V's discriminate but rank wrong** → eval has signal but weighting
  is off (Step 2 will help).
- **V's identify the right move type but lose on tie-break** → eval is
  close; small fix.
- **Human's move scores worse than engine's** → eval has the wrong
  preference structure; weights won't fix it, substrate has to change
  (skip to Step 3).

This is ~1 hour of work and tells you which of the next steps is
worth doing.

### Step 2 — Quick win: tune w₁..w₄ once (~1-2 hours)

If Step 1 says "discrimination is there, weights are wrong":

- Run material-only `(W_M=1, W_P=0, W_S=0, W_O=0)` baseline. If this
  hits ~3/13, material recognition works.
- Run safety-heavy `(1, 0, 2, 1)` — biases toward king-safety
  positions.
- Run crossfire-heavy `(1, 0, 1, 3)` — biases against piece
  vulnerability (these are tactical fixtures, after all).
- Anything that breaks above 3/13 is worth investigating; anything that
  stays at 0-1/13 confirms the eval is signal-limited not
  weight-limited.

**Stop condition:** if no weight combination breaks 3/13, weights
aren't the leverage. Move to Step 3.

### Step 3 — The real fix: wire intent.rs as V substrate (~1 day)

Use the Vulnerability-split design above. Specifically:

- **Refactor `intent.rs`** to produce: Direct Offense, X-ray Intent,
  Direct Threat, Latent Threat, Prophylaxis (per opponent); plus
  Discovery Liability (per piece, not per opponent).
- **Refactor `eval.rs`** to consume the intent tensor as substrate:
  - **Oᵢ** (crossfire) reads `sum_opponents(Direct Threat + Latent
    Threat)` per friendly piece, weighted by piece value
  - **Sᵢ** (king safety) reads king-vicinity Direct Threat + Latent
    Threat − defenders, plus Discovery Liability on king escape
    squares
  - **Pᵢ** (positional control) still uses Mobility Mask × centrality
    but now read from `intent.rs` rather than re-derived
- **Run the strength gate.** Aim for double-digit /13. If you don't get
  there, intent alone isn't enough.

This is the largest single piece of work but it's already designed
(this doc above) — the next agent has the table and the data layout.

**Stop condition:** if intent wire-in moves the needle to ~5-8/13, the
substrate is right and you can either invest more or move to NNUE
training. If still 0-2/13, escalate to Step 4.

### Step 4 — Wire bounty + zones (~1 day if Step 3 succeeded)

If intent moved the needle, bounty (FFA-points-aware capture eval) and
zones (territorial control) are the next two substrates to wire in.
Same pattern as intent — verify each is wired as substrate for the
existing M/P/S/O components, not as new V components.

If intent did NOT move the needle, defer Step 4 — wire-in is not the
issue; missing tactical primitives are. Move to Step 5.

### Step 5 — Last resort: add direct tactical primitives (~variable)

If the intent/bounty/zones substrate still doesn't get the eval to a
viable strength-gate score:

- Mate-in-1 / Mate-in-2 detection (cheap to add at eval time)
- Hanging-piece detection (Direct Threat × no defender)
- Basic fork / pin / skewer detection (read from intent's geometry)

These are eval-time pattern matches, not search extensions. They're
the anti-pattern direction (hand-coded primitives instead of structured
queries), so use only if the structured queries demonstrably can't
carry the signal.

### What I'd NOT do

- **Don't tune NNUE yet.** Hard Rule #7. A 0/13 (or 1/13) teacher will
  produce a 0/13 student. The strength gate is doing its job; respect it.
- **Don't add more search depth.** The diagnostic table is clear that
  depth amplifies nothing here. Anything beyond depth 8 with the
  current eval is wasted compute.
- **Don't ship shallow pruning yet.** Phase doc says naïve provable
  bounds never fire because eval is non-constant-sum. Solving the
  bounds question is interesting but it's downstream of fixing the
  eval signal.
- **Don't blame TRS.** It landed and helped marginally (+1/13). That's
  the right outcome for what it's designed to fix (exchange horizon).
  The bigger problem is eval-signal horizon, which TRS doesn't address.

---

## Action items, integrated with the recommendation

1. **Step 1 diagnostic** — characterize v0 eval failure mode on the
   13 fixtures (Kimi lane, ~1 hour).
2. **Step 2 weight sweep** if Step 1 suggests weights matter (Kimi lane,
   ~1-2 hours).
3. **Verify intent.rs / bounty.rs / zones.rs integration** — confirm
   dormant vs wired vs drifted. Read `eval.rs` to see what it consumes.
4. **Refactor intent.rs per the Vulnerability split** (Direct Threat,
   Latent Threat, Discovery Liability, etc.) and wire it as substrate
   for the existing V components (Kimi lane, ~1 day).
5. **Pin the Prophylaxis operational definition** from the candidates
   listed in the design section above.
6. **Add the exploitation-mirror read protocol** for move ordering
   and NNUE policy signal once intent is wired.
7. **Land CO-002 / CO-003** (spec text fixes for §1.4 promotion rank
   and §7.3 EP examples — Kimi).
8. **Defer:** shallow pruning bound source (Concern #1) — strength gate
   first; pruning is downstream of eval.
9. **Defer:** NNUE training — strength gate first per Hard Rule #7.
10. **Update `PITCH-piece-intent-tensor.md`** to reflect the refined
    component split + exploitation mirror so future agents don't
    re-derive it.

— Claude
