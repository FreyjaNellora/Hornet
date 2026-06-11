# Hornet — pitch for new agents

**Welcome.** Read this first. It tells you what Hornet is, where everything
lives, what the hard rules are, and what's currently in flight. Five
minutes to read, twelve docs you can then navigate without getting lost.

---

## What Hornet is

A four-player chess engine built from a single foundational primitive:
**per-piece BFS line projection feeding a query engine that returns a
per-player utility vector V = ⟨U₁, U₂, U₃, U₄⟩ to a Max^n search.**

The eval contract is a vector, not a scalar — search backs up per-player
components without ever collapsing. The NNUE on top is a dense MLP over
structured query outputs, not the canonical sparse-binary-with-accumulator
NNUE that doesn't scale to 14×14 boards anyway.

Hornet is a clean rebuild — not a successor inside an existing codebase.
There's a prior project where the array-lines architecture was prototyped
and validated; lessons learned from that prototype informed the spec, but
no code or vocabulary carries over. Hornet stands on its own terms with
academic-term references throughout.

---

## Current project state (as of 2026-06-01)

**What exists:** Self-contained build spec, academic-techniques manifest,
research-citations manifest, chess.com rule verifications, spec
review-cycle artifacts, 16 real-human PGN4 game files, 25-entry tactical
fixture suite, this onboarding pitch, an adopted operations Playbook.

**What's in flight:** Kimi (one of the active agents) is releasing build
spec **v0.2** with three blocker fixes from the most recent review (#1
value-system split, #7 PGN4 ingestion in protocol, #10 underpromotion
support) and integrated chess.com rule verifications for items #3-6 and
#9 (castling, claim threshold, DKW behavior, stalemate scoring, K/Q
placement).

**What hasn't started:** Any Rust code. No `cargo new` yet. No
`hornet-engine/` directory. Implementation begins after spec v0.2 lands.

---

## Where to find what

All paths relative to `Project_Hornet/`.

| Doc | Read when |
|---|---|
| `HORNET-BUILD-SPEC.md` | You're implementing. This is the source of truth for what to build. Sections cover 4PC rules, data structures, line projection algorithm, query engine contract, NNUE architecture, search contract, test specification, performance targets, file structure. Self-contained — no Freyja knowledge assumed. |
| `TECHNIQUES-and-REFERENCES.md` | You want to look up a technique by its academic name (Korf shallow pruning, IEDS, quiescence, etc.) and find the paper it came from. Also catalogs anti-patterns Hornet rejects with reasoning. |
| `SOURCES-and-CITATIONS.md` | You want every external reference cited anywhere in Hornet's docs in one place. URLs, paper titles, where each is referenced from. |
| `VERIFICATION-claude-to-kimi-spec-review-2026-06-01.md` | You're implementing a 4PC rule and want the chess.com-authoritative answer (castling destinations per player, claim-win threshold, DKW behavior, stalemate scoring direction, K/Q placement, canonical FEN4 starting string). |
| `REVIEW-claude-on-hornet-spec-2026-06-01.md` + `RESPONSE-kimi-to-claude-spec-review-2026-06-01.md` | You want to follow the review thread. Read in filename date order. |
| `baselines/README.md` | You're implementing the FEN4/PGN4 parser, the tactical fixture suite, or the strength gate. Tells you what each file in `baselines/` is for. |
| `baselines/human_4pc_game_*.pgn4` | 16 real chess.com games (some at 3000+ ELO). Round-trip tests for parser. Move-stream replay tests for move-gen. |
| `baselines/tactical_samples.json` | 25 curated tactical positions with `fen4` + human's actual move + scoring rubric. **The strength gate suite.** |
| `Playbook/` | **Read `Playbook/README.md` before your first session.** Defines the operational protocol: factory model, session protocol, communication protocol, agent-conduct conventions, information hierarchy. The framework is project-agnostic; Hornet uses it as-is. |

---

## Hard rules — do not re-litigate

These are settled. If you think one of them is wrong, surface the
concern as a change order per the Playbook framework — don't silently
violate.

1. **Depth must be a multiple of 4.** 4PC turn rotation has four players;
   non-multiples leave the perspective chain ending mid-rotation,
   producing asymmetric horizon bias. Valid search depths: 4, 8, 12, 16.
   Not 6, not 10.

2. **FEN4 and PGN4 are native engine I/O formats.** No Node-side
   translation, no intermediate JSON, no shell-script glue. Engine
   ingests `position fen4 <string>` and `position pgn4 <filepath>`
   directly. Test against the 16 PGN4 corpus files in `baselines/`.

3. **Eval returns a per-player V vector, never a scalar.** Signature is
   `eval_4vec(&state) -> [i16; 4]`. Search backs it up via Max^n.
   No scalar collapse at the eval boundary.

4. **V decomposition is fixed:** `Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ`
   where Mᵢ is material, Pᵢ positional control, Sᵢ king safety, Oᵢ
   dominance/crossfire. Each component traces to exactly one query
   class. If you find yourself adding a fifth component or merging two
   into one, that's a change order.

5. **Line projection is always-recompute, never incremental.** Pipeline
   benchmark settled this. No `piece_id` on Board. No
   inverse-index-maintenance-during-update code.

6. **Additive discipline — anything that changes the played move.** Every
   lever that can change the move the engine plays — eval features, **move
   ordering, beam width/shape, LMR, killer/history heuristics, TT
   best-move-hint usage** — ships **default-off** with a **measured
   ablation arm** (self-play A/B or an equivalent recorded measurement),
   the same gate as eval changes. No silent or unmeasured
   strength-affecting changes. *(Amended by CO-006; measured basis:
   EXP-020 — one ordering heuristic alone changed 11.6% of played moves at
   beam 4. Corollary: future changes to killers/history/TT-hint fall under
   this gate; the existing baselines stay.)*

7. **Strength gate before NNUE training.** Hand-tuned evaluator must
   pass "humans-can't-routinely-beat-it" tactical fixture rate + direct
   human play. Teacher quality is the student's ceiling.

8. **Distinct eval vs FFA-points value systems.** Centipawn eval values
   (used in SEE, V's Mᵢ) are different from chess.com FFA point values
   (used in result tags). Don't conflate. See spec § 1.7.

---

## How to engage

1. **First session:** read this pitch, then `Playbook/README.md`, then
   `Playbook/framework/session-protocol.md`. That tells you how to take
   a shift.

2. **Implementing:** the build spec is your source of truth. Cross-reference
   `TECHNIQUES-and-REFERENCES.md` for the academic name of any technique
   you're working on; cross-reference `VERIFICATION-*` for 4PC rule
   details.

3. **Responding to specs or reviews:** use the inline `Accept` /
   `Pushback` / `Need-info` per-item convention established in
   `RESPONSE-kimi-to-claude-spec-review-2026-06-01.md`. Don't silently
   ignore items; mark each.

4. **Surfacing a concern:** use the Playbook's change-order template
   (`Playbook/framework/change-order-template.md`). Don't fix things
   you don't own and don't silently violate a hard rule.

5. **Closing a shift:** session note per the Playbook session protocol.
   Compress forward, preserve backward — current state goes in the
   shift handoff; full detail goes in the session note.

---

## Next concrete step (after spec v0.2 lands)

1. `cargo new hornet-engine` in this directory.
2. Implement `board/fen4.rs` first — round-trip the 16 PGN4 corpus files
   in `baselines/`.
3. Then per spec § 9 file structure, in roughly the order the spec
   tests demand: types → board → lines → queries → eval → search.
4. Strength gate against `baselines/tactical_samples.json` after each
   major milestone.

You don't need to ask for permission to start when v0.2 is in. The pitch
+ spec + Playbook are enough authorization. Surface a change order if
you hit a blocker.
