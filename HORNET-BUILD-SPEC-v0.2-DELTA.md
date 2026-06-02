# Hornet Build Specification v0.2 Delta

> **✅ STATUS: MERGED into `HORNET-BUILD-SPEC.md` (now v0.2) on 2026-06-01 via change order CO-001.**
> Retained as the historical patch record. The canonical spec is `HORNET-BUILD-SPEC.md`; do not
> implement from this delta directly. (Placement note: the delta's "§6.5" for PGN4 was landed as a
> new top-level **§10 Protocol & I/O Formats**, since the spec had no protocol/I-O section.)

**Date:** 2026-06-01  
**Re:** `HORNET-BUILD-SPEC.md` v0.1 → v0.2  
**Purpose:** Patch document applying all review fixes and verifications.

---

## Changes from v0.1

### 1. [BLOCKER FIX] Two distinct value systems (was §1.7, now §1.7+§1.8)

**Old:** Single `value()` function returning centipawns, used everywhere.  
**New:** Two separate functions:

```rust
impl PieceType {
    /// Eval value: centipawns for Mᵢ, SEE, move ordering
    pub fn eval_value(self) -> i16 {
        match self {
            Pawn => 100, Knight => 300, Bishop => 450,
            Rook => 500, Queen => 900, King => 0, PromotedQueen => 900,
        }
    }
    
    /// FFA points: chess.com scoring for result lines
    pub fn ffa_points(self) -> u8 {
        match self {
            Pawn => 1, Knight => 3, Bishop => 3,
            Rook => 5, Queen => 9, King => 20, PromotedQueen => 9,
        }
    }
}
```

- `query_material` uses `eval_value()`
- Result writer / PGN4 output uses `ffa_points()`
- Never mix them silently

### 2. [BLOCKER FIX] Castling rules specified (was §1.5, now §1.5)

**Standard mechanic:** King moves 2 squares toward rook; rook jumps to square king passed through.

| Player | Side | Pre-castle K | Pre-castle R | Post-castle K | Post-castle R | Empty squares required |
|--------|------|-------------|-------------|--------------|--------------|----------------------|
| Red | Kingside (O-O) | h1 | k1 | j1 | i1 | i1, j1 |
| Red | Queenside (O-O-O) | h1 | d1 | f1 | g1 | e1, f1, g1 |
| Blue | Kingside (O-O) | a7 | a4 | a5 | a6 | a5, a6 |
| Blue | Queenside (O-O-O) | a7 | a11 | a9 | a8 | a8, a9, a10 |
| Yellow | Kingside (O-O) | g14 | d14 | e14 | f14 | e14, f14 |
| Yellow | Queenside (O-O-O) | g14 | k14 | i14 | h14 | h14, i14, j14 |
| Green | Kingside (O-O) | n8 | n11 | n10 | n9 | n9, n10 |
| Green | Queenside (O-O-O) | n8 | n4 | n6 | n7 | n5, n6, n7 |

**Kingside vs queenside determination:** The rook on the same side as the queen (from starting position) is the queenside rook. For Red: Q at g1, so d1 rook is queenside. For Blue: Q at a8, so a11 rook is queenside.

### 3. [BLOCKER FIX] PGN4 ingestion added (was missing, now §9 + §6.5)

**New file:** `board/pgn4.rs` — PGN4 parser/writer

**Protocol commands:**
- `position fen4 <string>` — load position from FEN4 string
- `position pgn4 <filepath> [moves <n>]` — load game from PGN4, optionally advance to ply n
- Game export emits PGN4 to stdout or filepath

**Parser must handle:**
- Chess.com `from-to` notation: `d2-d4`, `Bn6xBg13`, `Rk14xk8`
- Standard SAN: `Nf3`, `Bxe5`, `O-O`, `O-O-O`
- Promotion: `e7-e8=D` (default queen), `=N`, `=B`, `=R` (underpromotion)
- Check `+`, mate `#`

**Round-trip test corpus:** 16 PGN4 files in `baselines/`

### 4. [FIX] En passant tests rewritten (was §7.3, now §7.3)

**Red-Blue EP:**
- Blue pawn at c4, pushes c4→e4 (East 2). EP target: d4.
- Red pawn at d3 captures EP: d3→d4, removing Blue's e4 pawn.

**Red-Green EP:**
- Green pawn at m4, pushes m4→k4 (West 2). EP target: l4.
- Red pawn at l3 captures EP: l3→l4, removing Green's k4 pawn.

**Blue-Yellow EP:**
- Yellow pawn at e13, pushes e13→c13 (South 2). EP target: d13.
- Blue pawn at d12 captures EP: d12→d13, removing Yellow's c13 pawn.

**Yellow-Green EP:**
- Green pawn at m11, pushes m11→k11 (West 2). EP target: l11.
- Yellow pawn at l12 captures EP: l12→l11, removing Green's k11 pawn.

**Invalid pairs (assert no EP possible):** Red↔Yellow, Blue↔Green.

### 5. [FIX] Claim-win threshold specified (was §1.7, now §1.8)

- **Only available in 2-player endgame** (not 3P or 4P)
- **Base threshold:** 21-point lead over second place
- **Zombie king adjustment:** +20 per DKW king on board
  - 1 zombie: 41-point lead required
  - 2 zombies: 61-point lead required
- **Insufficient material exception:** 1-point lead if opponent has insufficient checkmating material

### 6. [FIX] DKW behavior verified (was §1.7, now §1.7)

- Kimi's spec was correct: DKW king moves randomly each turn
- Dead army pieces are immovable walls
- DKW king CAN capture but does NOT receive points
- DKW stalemate: 10 points to EACH remaining live player

### 7. [FIX] Stalemate scoring corrected (was §1.8, now §1.8)

- **Live player stalemate:** Stalemated player receives 20 points (consolation), then eliminated
- **DKW king stalemate:** 10 points to EACH remaining live player, DKW king removed

### 8. [FIX] Promotion supports underpromotion (was §1.4, now §1.4)

- Default promotion: Queen
- Underpromotion allowed: `=N`, `=B`, `=R`
- `PieceType::PromotedQueen` distinct from `PieceType::Queen`

### 9. [VERIFY] King/Queen placement confirmed (was §1.3, now §1.3)

No change — Kimi's spec was correct. Canonical FEN4 starting position:

```
R-0,0,0,0-1,1,1,1-1,1,1,1-0,0,0,0-0-3,yR,yN,yB,yK,yQ,yB,yN,yR,3/3,yP,yP,yP,yP,yP,yP,yP,yP,3/14/bR,bP,10,gP,gR/bN,bP,10,gP,gN/bB,bP,10,gP,gB/bQ,bP,10,gP,gK/bK,bP,10,gP,gQ/bB,bP,10,gP,gB/bN,bP,10,gP,gN/bR,bP,10,gP,gR/14/3,rP,rP,rP,rP,rP,rP,rP,rP,3/3,rR,rN,rB,rQ,rK,rB,rN,rR,3
```

### 10. [VERIFY] Bishop value marked pending calibration (was §2.3, now §2.3)

`BISHOP_EVAL_VALUE = 450` — documented as `[PENDING CALIBRATION]`. Tournament self-play with values 300/350/400/450/500 to determine optimal.

---

## File Structure Update (§9)

```
hornet-engine/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── board/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── attacks.rs
│   │   ├── fen4.rs          # FEN4 parser/writer
│   │   ├── pgn4.rs          # NEW: PGN4 parser/writer
│   │   └── zobrist.rs
│   ├── lines.rs
│   ├── queries.rs
│   ├── eval.rs
│   ├── nnue/
│   │   ├── mod.rs
│   │   ├── network.rs
│   │   └── weights.rs
│   ├── search.rs
│   ├── move_gen.rs
│   ├── move_order.rs
│   ├── tt.rs
│   └── protocol/
│       ├── mod.rs
│       ├── parse.rs
│       └── output.rs
├── tests/
│   ├── integration_tests.rs
│   ├── perft.rs
│   └── pgn4_roundtrip.rs    # NEW: round-trip all baselines/
├── baselines/               # NEW: 16 PGN4 games + tactical_samples.json
│   ├── human_4pc_game_*.pgn4
│   ├── tactical_samples.json
│   └── README.md
├── Cargo.toml
└── README.md
```

---

## Implementation Order (Recommended)

1. **FEN4 parser** (`board/fen4.rs`) — round-trip canonical starting position
2. **PGN4 parser** (`board/pgn4.rs`) — round-trip all 16 baseline games
3. **Board + move generator** — legal move generation, make/unmake
4. **Line projection** (`lines.rs`) — compute_lines, all piece types
5. **Query engine** (`queries.rs`) — all queries, QueryVector
6. **Evaluator** (`eval.rs`) — ArrayLinesEvaluator, v0 weights
7. **Search** (`search.rs`) — Max^n, beam, move ordering
8. **NNUE** (`nnue/`) — MLP architecture, training pipeline
9. **Protocol** (`protocol/`) — UCI-like interface

---

## No Outstanding Blockers

All [BLOCKER] issues from review are resolved:
- ✅ #1 Two value systems → split into eval_value / ffa_points
- ✅ #3 Castling → per-player tables provided
- ✅ #7 PGN4 ingestion → parser spec + file structure + test corpus

All [VERIFY] items have ground truth:
- ✅ #5 DKW → spec was correct
- ✅ #6 Stalemate → split live vs DKW
- ✅ #9 K/Q placement → spec was correct, canonical FEN4 provided
- ✅ #4 Claim threshold → 21 base, +20 per zombie
- ⏳ #8 Bishop value → calibration tournament (non-blocking)

**Implementation can begin.**
