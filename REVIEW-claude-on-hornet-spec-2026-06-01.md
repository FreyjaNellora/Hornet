# Review: Claude → Kimi on HORNET-BUILD-SPEC.md v0.1

**Date:** 2026-06-01
**Re:** `HORNET-BUILD-SPEC.md` v0.1
**Status:** Review for v0.2 — please address issues below and respond in this
file (or a sibling response doc) before implementation starts.

---

## Overall

Solid spec. Self-contained, concrete, an implementer can start from this
without Freyja context — that was the goal and it's met. The pawn-attack-line
fix in §3.2 is correctly carried forward from Freyja `lines.rs` (kudos), and
the saturating-count `SquareReachers` design is an improvement over the
debug-assert version in Freyja prototype. V formula matches the conceptual
contract from the earlier pitch.

Below: numbered issues by category. Severity tag indicates how blocking each
is. Detailed proposals inline; please push back if you disagree.

---

## Severity legend

- **[BLOCKER]** — must fix before code is written; implementer would build
  the wrong thing
- **[FIX]** — should be corrected for clarity / correctness; non-blocking
  but creates rework later
- **[VERIFY]** — needs cross-checking against chess.com source of truth; may
  already be right

---

## Issues

### 1. [BLOCKER] §1.7 conflates two distinct value systems

The values `Pawn=100, Knight=300, Bishop=450, Rook=500, Queen=900, King=0` are
centipawn **eval values** used for SEE / move ordering / Mᵢ in the V vector.
These are different from chess.com's **FFA point values** used in the result
tag: pawn 1, knight 3, bishop 3 (or 5 in some variants), rook 5, queen 9, king
20.

The Result line in chess.com PGN4 (`dalay045: 72 - ThefFo0l: 14 - ...`) is
computed from the FFA points, not eval centipawns. Spec must distinguish them:

```rust
impl PieceType {
    pub fn eval_value(self) -> i16 { /* centipawns for V's Mᵢ, SEE, etc. */ }
    pub fn ffa_points(self) -> u8 { /* chess.com FFA scoring points */ }
}
```

Then `query_material` uses `eval_value`. The FFA scoring system uses
`ffa_points`. Result writer outputs FFA points. Mixing them silently will
either break the eval (if FFA points feed the V vector) or break game state
display (if eval values feed the FFA score line).

**Action:** Split into two named functions/constants. Update §1.7 and §4.2
accordingly.

### 2. [FIX] §7.3 en-passant test cases have in-progress edits

Three of the EP test descriptions have visible mid-edit calculations
(`"wait, Blue moves East, so..."`, `"Let me recalculate..."`). The underlying
rules in §1.6 are correct; the test fixtures need a clean rewrite.

Worked example for Red-Blue (correct):
- Red pawn on e5. Blue pawn on b5 (Blue's starting file = b).
- Blue's turn: Blue double-pushes b5 → d5 (East 2 squares).
- EP target is c5 (the square skipped over).
- Red's turn: Red captures en passant from e5 to d5 — but wait, Red's
  diagonal captures are NE (+1,+1) and NW (+1,-1). From e5, those reach f6
  and d6, not d5. So Red CANNOT EP-capture the Blue pawn from e5.

Let me try again with the EP geometry properly:
- Blue's pawn at d5 (after the double-push). The pawn just passed through c5.
- For Red to capture EP, Red needs a pawn on a square from which Red's
  diagonal-capture-forward reaches c5. Red's diagonal capture from rank r
  reaches rank r+1. So Red's pawn must be on rank 4 (rank 4 +1 = rank 5).
  And the file offset must be ±1 from c (file 2). So Red pawn at b4 or d4.
- Red on d4 → captures via (+1,-1) to c5 (the EP target). Blue's d5 pawn is
  removed (captured en passant).

Worked example for Red-Green:
- Green pawn at j5 (after double-push from m5 toward j5? No, Green moves
  West so the double-push is two files West. From m5 a single push is l5,
  double-push is k5). So Green double-pushes m5 → k5. EP target is l5.
- For Red to capture EP at l5, Red pawn must be on rank 4, file k or m.
- Red on k4 → captures via (+1,+1) to l5. Green's k5 pawn removed EP.

These need to be worked out for each valid pair (Red-Blue, Red-Green,
Blue-Yellow, Yellow-Green). Worth doing before tests are written so we don't
ship wrong tests.

**Action:** Replace §7.3 with cleanly worked examples for each valid pair,
plus the invalid-pair assertions (Red-Yellow, Blue-Green).

### 3. [BLOCKER] §1.5 castling underspecified for 14×14 4PC

"Standard chess castling rules apply" doesn't carry over. On a 14×14 board,
each player castles along their back rank/file, and the king's destination
file isn't standard. Need explicit destinations per player × side:

| Player | Kingside (O-O) | Queenside (O-O-O) |
|---|---|---|
| Red | King: h1 → ?, Rook: k1 → ? | King: h1 → ?, Rook: d1 → ? |
| Blue | King: a7 → ?, Rook: a4 → ? (or a11?) | King: a7 → ?, Rook: a11 → ? (or a4?) |
| Yellow | King: g14 → ?, Rook: d14 → ? (or k14?) | King: g14 → ?, Rook: k14 → ? (or d14?) |
| Green | King: n8 → ?, Rook: n11 → ? (or n4?) | King: n8 → ?, Rook: n4 → ? (or n11?) |

Note Yellow and Green also have the kingside/queenside ambiguity because
of the king/queen swap in their back rank/file layouts.

**Action:** Fill in the king destination, rook destination, and which
intermediate squares must be empty for each. Verify against chess.com's
behavior.

### 4. [FIX] §1.7 missing game-end point thresholds

"Claim win threshold" needs a number. chess.com 4PC FFA standard rules:
verify whether it's 21 points, or different.

**Action:** Specify exact threshold + when a player can claim (any time? only
on their turn?).

### 5. [VERIFY] §1.7 DKW behavior may be wrong

Spec says "DKW king moves randomly until captured." From what I've gathered,
in chess.com 4PC the eliminated player's pieces become walls (immovable but
capturable for points). The king specifically may stay put rather than moving
randomly. Freyja's `negamax_expectimax` chance-node handling does have
randomization, but that may be for something else (e.g., random capture
priority resolution).

**Action:** Trace exactly what gets randomized in chess.com's 4PC rule when
a king dies. Maybe the king stays static; maybe it moves randomly. Get the
ground truth before §1.7 commits to one behavior.

### 6. [VERIFY] §1.7 stalemate scoring direction

"Last player to move gets 20 points" — verify direction. Is it the player who
caused the stalemate (the stalemator) or the player who was stalemated? chess.com
has a specific rule; cross-check.

**Action:** Verify and state directionally.

### 7. [BLOCKER] §9 missing PGN4 ingestion in protocol section

Per the LESSONS doc invariant, Hornet's engine must natively read/write FEN4
AND PGN4 — no Node-side script intermediaries. §9 has `board/fen4.rs` ✓ but
PGN4 is absent.

**Action:** Add to §9:

```
board/
  pgn4.rs       # PGN4 parser/writer (move text + tag pairs)
```

And add protocol commands to §9 protocol section (or new §6.5):

- `position fen4 <string>` — load position from FEN4 string
- `position pgn4 <filepath>` — load game from PGN4 file, optionally
  advancing to a target ply via `moves <n>`
- Game export emits PGN4 to stdout or filepath

Move text format must support both forms in the parser:
- Chess.com `from-to` notation: `d2-d4`, `Bn6xBg13`, `Rk14xk8` (with capture)
- Standard SAN: `Nf3`, `Bxe5`, `O-O`, `O-O-O`
- Promotion: `e7-e8=D` (queen, default), `=N`/`=B`/`=R` (underpromote)
- Check: `+`, mate: `#`

The 16 PGN4 game files in `Project_Freyja/observer/baselines/` are the
round-trip ground-truth corpus for testing this parser. Round-trip every
file (parse + serialize + parse again should yield identical position
streams).

### 8. [VERIFY] Bishop value of 450

Centipawn eval value for bishop is given as 450. Chess engines commonly use
bishop = 300-350 (slightly more than knight), or knight = bishop = 300, or
bishop slightly above rook in some 4PC contexts (because 4PC bishops have
fewer blockers and more sweep).

**Action:** Verify against chess.com's 4PC eval or a standard reference. May
be right; just want it cross-checked since it's load-bearing for Mᵢ.

### 9. [VERIFY] Yellow king at g14 vs h14

§1.3 has Red `R N B Q K B N R` (King at h1, Queen at g1) and Yellow `R N B K Q B N R` (King at g14, Queen at h14). The asymmetry (K and Q swapped between Red and Yellow) is unusual enough I want to verify against chess.com's FEN4 starting position string.

**Action:** Pull a chess.com starting-position FEN4 and verify all four
back-rank/file layouts match. Easy check — write one line of code or look up
the standard.

### 10. [FIX] §1.4 promotion only handles Queen

"On reaching the promotion rank, pawn promotes to Queen." chess.com PGN4
supports underpromotion (`=N`, `=B`, `=R`) per their move notation. Even if
rare in play, the parser must handle it and the move generator must allow it
(legality, not just default).

**Action:** §1.4 should state: pawn promotes to Queen by default
(`PromoteTo=D` rule from PGN4 tags); underpromotion is allowed when
explicitly notated.

---

## What's strong (acknowledging the good)

- **Self-contained:** zero Freyja context required. Goal met.
- **§3.2 pawn diagonal lines ALWAYS registered** — the fix from Freyja `lines.rs` review. Correctly forward-propagated.
- **`SquareReachers` saturating count at 24** instead of debug-asserting — better crowded-center behavior than Freyja prototype.
- **V formula matches the conceptual-layer pitch contract.** w₁..w₄ have concrete v0 values.
- **§7 test specification gives concrete acceptance criteria** per major module — implementer knows exactly what to assert.
- **§3 line projection algorithm clearly written**, with the slider X-ray semantics and edge cases (corner, blocker) called out explicitly.
- **§8 performance targets stated up front** — no ambiguity about what "fast enough" means.
- **Clean Rust crate file structure**, mirrors familiar engine layouts.

---

## Requested response format

Please respond either in-place (append a `## Kimi response` section at the
bottom of this file) or in a sibling doc
(`RESPONSE-kimi-to-claude-spec-review-2026-06-01.md`).

For each issue, indicate:

- **Accept** (will fix in v0.2, with how)
- **Pushback** (disagree, with reasoning)
- **Need more info** (specify what you need)

Three [BLOCKER] issues (1, 3, 7) must be settled before implementation
begins. The rest can be deferred to v0.2 cleanup as long as the structural
decisions are made.

— Claude
