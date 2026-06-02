# Hornet Build Specification

**Version:** 0.2  
**Date:** 2026-06-01 (v0.1: 2026-05-31)  
**Purpose:** Self-contained specification for building Hornet from scratch. No knowledge of Freyja assumed.

**v0.2 changelog:** Integrated all 10 items from `HORNET-BUILD-SPEC-v0.2-DELTA.md` (now merged) per
change order CO-001 — value-system split (§1.7/§2.3), castling tables (§1.5), PGN4 ingestion + the
new §10 Protocol & I/O, underpromotion (§1.4), claim-win threshold + stalemate scoring (§1.8), DKW
behavior (§1.7), EP tests (§7.3), canonical FEN4 (§1.3), bishop value pending calibration (§2.3).

---

## 1. Four-Player Chess Rules (4PC)

### 1.1 Board Geometry

- **Grid:** 14×14 squares, indexed `0..195`
- **Invalid corners:** Four 3×3 corners are unplayable
  - SW: ranks 0-2, files 0-2
  - SE: ranks 0-2, files 11-13
  - NW: ranks 11-13, files 0-2
  - NE: ranks 11-13, files 11-13
- **Valid squares:** 160 (196 - 36)
- **Square index:** `sq = rank * 14 + file` where `rank, file ∈ 0..13`
- **Validity check:** A square is valid iff NOT `((rank < 3 || rank > 10) && (file < 3 || file > 10))`

### 1.2 Players and Turn Order

```
enum Player { Red = 0, Blue = 1, Yellow = 2, Green = 3 }
```

- **Turn order:** Red → Blue → Yellow → Green → Red ...
- **Next player:** `Red→Blue, Blue→Yellow, Yellow→Green, Green→Red`
- **Opponents:** Each player has 3 opponents (all others)

### 1.3 Starting Position

Each player has 16 pieces arranged on their back rank and second rank:

**Red (South):** Back rank 0 (display rank 1), files 3-10
- Pieces: R N B Q K B N R (left to right: d1, e1, f1, g1, h1, i1, j1, k1)
- Pawns: rank 1 (display rank 2), files 3-10 (d2-k2)

**Blue (West):** Back file 0 (display file a), ranks 3-10
- Pieces: R N B K Q B N R (bottom to top: a4, a5, a6, a7, a8, a9, a10, a11)
- Pawns: file 1 (display file b), ranks 3-10 (b4-b11)

**Yellow (North):** Back rank 13 (display rank 14), files 3-10
- Pieces: R N B K Q B N R (left to right: d14, e14, f14, g14, h14, i14, j14, k14)
- Pawns: rank 12 (display rank 13), files 3-10 (d13-k13)

**Green (East):** Back file 13 (display file n), ranks 3-10
- Pieces: R N B Q K B N R (bottom to top: n4, n5, n6, n7, n8, n9, n10, n11)
- Pawns: file 12 (display file m), ranks 3-10 (m4-m11)

**King squares in starting position:**
- Red: h1 (index 7)
- Blue: a7 (index 84)
- Yellow: g14 (index 188)
- Green: n8 (index 111)

**Canonical FEN4 starting position** (verified vs chess.com; see §10 for the FEN4 grammar):

```
R-0,0,0,0-1,1,1,1-1,1,1,1-0,0,0,0-0-3,yR,yN,yB,yK,yQ,yB,yN,yR,3/3,yP,yP,yP,yP,yP,yP,yP,yP,3/14/bR,bP,10,gP,gR/bN,bP,10,gP,gN/bB,bP,10,gP,gB/bQ,bP,10,gP,gK/bK,bP,10,gP,gQ/bB,bP,10,gP,gB/bN,bP,10,gP,gN/bR,bP,10,gP,gR/14/3,rP,rP,rP,rP,rP,rP,rP,rP,3/3,rR,rN,rB,rQ,rK,rB,rN,rR,3
```

### 1.4 Piece Movement

**Pawn forward direction per player:**
- Red: +rank (North)
- Blue: +file (East)
- Yellow: -rank (South)
- Green: -file (West)

**Pawn capture deltas per player:**
- Red: `(+1, +1)` and `(+1, -1)` — NE and NW
- Blue: `(+1, +1)` and `(-1, +1)` — NE and SE
- Yellow: `(-1, +1)` and `(-1, -1)` — SE and SW
- Green: `(+1, -1)` and `(-1, -1)` — NW and SW

**Pawn moves:**
- Forward push: 1 square in forward direction (must be empty)
- Double push: 2 squares on first move only (both squares must be empty)
- Capture: 1 square diagonally (must contain enemy piece)
- En passant: See Section 1.6
- Promotion: On reaching the player's promotion rank (Red→rank 13, Blue→file 13, Yellow→rank 0, Green→file 0), the pawn promotes. **Default: Queen** (PGN4 `=D`). **Underpromotion allowed:** `=N`, `=B`, `=R`. A queen produced by promotion is `PieceType::PromotedQueen`, a variant distinct from `Queen` (same values, but tracked separately).

**Knight:** 8 L-jumps: `(±2, ±1)` and `(±1, ±2)`

**King:** 1 square in any of 8 directions

**Slider pieces (Bishop, Rook, Queen, PromotedQueen):**
- Bishop: 4 diagonal directions
- Rook: 4 orthogonal directions
- Queen/PromotedQueen: All 8 directions
- Walk full ray until: out of bounds, invalid corner, or second occupant (for X-ray)

### 1.5 Castling

- Each player can castle kingside and/or queenside
- Standard chess castling rules apply (king and rook unmoved, no pieces between, king not in check, king doesn't pass through check)
- Castling rights: 2 bits per player × 4 players = 8 bits total
- **Mechanic:** King moves 2 squares toward the rook; rook jumps to the square the king passed through.

**Per-player castling table** (verified vs chess.com):

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

**Kingside vs queenside:** the rook on the same side as the queen (from the starting position) is
the queenside rook. Red: Q at g1 → d1 rook is queenside. Blue: Q at a8 → a11 rook is queenside.

### 1.6 En Passant

**Critical rule:** En passant only works between orthogonal neighbors (players whose pawns move perpendicular to each other).

**Valid en passant pairs:**
- Red ↔ Blue (Red moves North, Blue moves East — orthogonal)
- Red ↔ Green (Red moves North, Green moves West — orthogonal)
- Blue ↔ Yellow (Blue moves East, Yellow moves South — orthogonal)
- Yellow ↔ Green (Yellow moves South, Green moves West — orthogonal)

**Invalid en passant pairs (parallel pawn directions):**
- Red ↔ Yellow (both move along rank axis — never en passant)
- Blue ↔ Green (both move along file axis — never en passant)

**Why:** Red and Yellow pawns move along the same axis (rank). They can never pass each other while remaining pawns because they'd promote first. Same for Blue and Green along the file axis.

**En passant trigger:** When a pawn makes a double push, the skipped square becomes an en passant target for exactly one ply. Only orthogonal-neighbor pawns can capture en passant.

**En passant edge cases to test** (worked examples in §7.3):
- Red-Blue: Blue pawn at c4 pushes c4→e4 (East 2), EP target d4; Red pawn at d3 captures d3→d4.
- Near invalid corners: an EP capture that would land on an invalid corner square must be rejected.
- Multiple en passant targets: two different pawns create EP opportunities simultaneously.

### 1.7 Capture Scoring, Value Systems, and Elimination

**Two distinct value systems — never conflate them (Hard Rule #8):**

| Piece | `eval_value()` (centipawns) | `ffa_points()` (FFA points) |
|-------|----------------------------|-----------------------------|
| Pawn | 100 | 1 |
| Knight | 300 | 3 |
| Bishop | 450 *(pending calibration)* | 3 |
| Rook | 500 | 5 |
| Queen | 900 | 9 |
| King | 0 | 20 |
| PromotedQueen | 900 | 9 |

- `eval_value()` — centipawns; used for the material query Mᵢ, SEE, and move ordering.
- `ffa_points()` — chess.com free-for-all points; used only for result lines / PGN4 output.
- Capturing a piece in FFA awards its `ffa_points()`. The evaluator never sees FFA points.

**Elimination & Dead-King-Walking (DKW):**
- A player is eliminated when their king is captured.
- The eliminated player's non-king pieces become **immovable walls**.
- The **DKW king moves randomly** each turn until captured or the game ends. It **can capture but
  receives no points** for captures.
- DKW king stalemate: **10 points to each remaining live player**, and the DKW king is removed.
- Last player standing wins; if multiple players remain at game end, points determine placement.

### 1.8 Game End Conditions

- Checkmate: King has no legal moves and is in check → player eliminated.
- **Stalemate (live player):** king has no legal moves and is not in check → the stalemated player
  receives **20 points (consolation)**, then is eliminated.
- **Stalemate (DKW king):** **10 points to each remaining live player**; the DKW king is removed
  (see §1.7).
- Threefold repetition: Draw claim.
- 50-move rule: Draw claim.

**Claim-win threshold** (verified vs chess.com):
- Available **only in a 2-player endgame** (not 3P or 4P).
- **Base threshold:** a **21-point lead** over second place.
- **Zombie-king adjustment:** **+20 per DKW king on the board** (1 zombie → 41-point lead; 2 zombies
  → 61-point lead).
- **Insufficient-material exception:** a **1-point lead** suffices if the opponent has insufficient
  checkmating material.

---

## 2. Data Structures

### 2.1 Square

```rust
pub struct Square(pub u8); // 0..195

impl Square {
    pub fn rank(self) -> u8 { self.0 / 14 }
    pub fn file(self) -> u8 { self.0 % 14 }
    pub fn index(self) -> u8 { self.0 }
    
    pub fn is_valid(self) -> bool {
        let r = self.rank();
        let f = self.file();
        !((r < 3 || r > 10) && (f < 3 || f > 10))
    }
}
```

### 2.2 Player

```rust
#[repr(u8)]
pub enum Player { Red = 0, Blue = 1, Yellow = 2, Green = 3 }

impl Player {
    pub fn next(self) -> Self { /* Red→Blue→Yellow→Green→Red */ }
    pub fn opponents(self) -> [Player; 3] { /* all other players */ }
    pub fn index(self) -> usize { self as usize }
}
```

### 2.3 PieceType

```rust
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    PromotedQueen = 6,
}

impl PieceType {
    pub fn is_slider(self) -> bool {
        matches!(self, Bishop | Rook | Queen | PromotedQueen)
    }

    /// Centipawn eval value (Mᵢ, SEE, move ordering). Hard Rule #8.
    pub fn eval_value(self) -> i16 {
        match self {
            Pawn => 100, Knight => 300, Bishop => 450, // BISHOP: [PENDING CALIBRATION]
            Rook => 500, Queen => 900, King => 0, PromotedQueen => 900,
        }
    }

    /// chess.com FFA points (result lines / PGN4 output). Never conflate with eval_value.
    pub fn ffa_points(self) -> u8 {
        match self {
            Pawn => 1, Knight => 3, Bishop => 3,
            Rook => 5, Queen => 9, King => 20, PromotedQueen => 9,
        }
    }
}
```

> **`BISHOP_EVAL_VALUE = 450` is `[PENDING CALIBRATION]`** — to be resolved by self-play tournament
> over `{300, 350, 400, 450, 500}`. `ffa_points` are fixed by chess.com rules.

### 2.4 Piece

```rust
pub struct Piece {
    pub piece_type: PieceType,
    pub player: Player,
    // NO piece_id — always recompute lines from scratch
}
```

**Equality:** Two pieces are equal if same type and same player. No unique ID.

### 2.5 Board

```rust
pub struct Board {
    squares: [Option<Piece>; 196],        // 14×14, None = empty or invalid
    piece_lists: [[Option<(PieceType, Square)>; 32]; 4], // per player, max 32 pieces
    piece_counts: [u8; 4],                // active pieces per player
    king_squares: [u8; 4],                // king position per player, 255 = eliminated
    zobrist: u64,                         // hash for TT
    castling_rights: u8,                  // 8 bits (2 per player)
    en_passant: Option<Square>,           // EP target square
    en_passant_pushing_player: Option<Player>, // who created the EP opportunity
    side_to_move: Player,                 // whose turn it is
    dkw_flags: [bool; 4],                 // Dead King Walking flags
}
```

**Invariants:**
- `squares` and `piece_lists` are always in sync
- `king_squares[p] == 255` iff player p is eliminated
- `piece_counts[p]` equals number of `Some` entries in `piece_lists[p]`
- Zobrist hash is updated on every mutation

---

## 3. Line Projection Algorithm

### 3.1 Core Function

```rust
pub fn compute_lines(board: &Board) -> LineMap
```

**Algorithm:**
1. For each player in turn order:
   2. For each of player's pieces:
      3. Create `PieceLines` for this piece
      4. Project rays based on piece type (see below)
      5. Store in `LineMap.pieces[next_index]`
      6. Increment `LineMap.piece_count`
7. Build inverse index: for each reach entry, add to `LineMap.square_reachers[sq]`

**Time:** O(pieces × rays × board_size) ≈ 60 × 8 × 13 = ~6k operations  
**Space:** O(pieces × max_reach) ≈ 60 × 104 = ~6k entries

### 3.2 Per-Piece Ray Projection

#### Slider (Bishop, Rook, Queen, PromotedQueen)

```
for each direction (dr, df) in piece_directions:
    first_occupant = None
    for step = 1..13:
        nr = rank + dr * step
        nf = file + df * step
        if out_of_bounds or invalid_corner: break
        sq = Square(nr, nf)
        distance = step
        
        if first_occupant is None:
            // Before first blocker
            occupant = board.piece_at(sq)
            if occupant is Some:
                first_occupant = occupant
                push ReachEntry::blocked(sq, distance, occupant, xray=true)
            else:
                push ReachEntry::empty(sq, distance)
        else:
            // Past first blocker (X-ray)
            occupant = board.piece_at(sq)
            if occupant is Some:
                push ReachEntry::blocked(sq, distance, occupant, xray=false)
                break  // Second occupant terminates ray
            else:
                push ReachEntry::empty(sq, distance)
```

**Directions:**
- Bishop: `(1,1), (1,-1), (-1,1), (-1,-1)`
- Rook: `(1,0), (-1,0), (0,1), (0,-1)`
- Queen: all 8 directions

#### Knight

```
for each (dr, df) in [(2,1), (2,-1), (-2,1), (-2,-1), (1,2), (1,-2), (-1,2), (-1,-2)]:
    nr = rank + dr
    nf = file + df
    if out_of_bounds or invalid_corner: continue
    sq = Square(nr, nf)
    occupant = board.piece_at(sq)
    if occupant is Some:
        push ReachEntry::blocked(sq, 1, occupant, xray=false)
    else:
        push ReachEntry::empty(sq, 1)
```

#### King

Same as knight but directions are all 8 adjacent squares `(±1, 0), (0, ±1), (±1, ±1)`. Distance always 1. No X-rays.

#### Pawn

**Forward push:**
```
(dr, df) = forward_direction(player)
// One step
nr = rank + dr, nf = file + df
if valid and empty:
    push ReachEntry::empty(sq, 1)
    
    // Two steps on first move
    if is_starting_rank(player, rank):
        nr2 = rank + 2*dr, nf2 = file + 2*df
        if valid and empty:
            push ReachEntry::empty(sq2, 2)
```

**Diagonal captures (ALWAYS registered, regardless of occupancy):**
```
for each (cdr, cdf) in capture_deltas(player):
    nr = rank + cdr
    nf = file + cdf
    if out_of_bounds or invalid_corner: continue
    sq = Square(nr, nf)
    occupant = board.piece_at(sq)
    if occupant is Some:
        push ReachEntry::blocked(sq, 1, occupant, xray=false)
    else:
        push ReachEntry::empty(sq, 1)
```

**Rationale:** Pawn attack zone is geometric. The square is in the pawn's attack zone whether empty or occupied. Query engine decides if it's a capture, defense, or empty threat.

### 3.3 Data Layout

```rust
pub struct ReachEntry {
    pub square: Square,
    pub distance: u8,           // steps from piece to this square
    pub first_occupant: Option<Piece>, // None = path clear to here
    pub xray_continues: bool,   // true only on first-occupant entry for sliders
}

pub struct PieceLines {
    pub player: Player,
    pub piece_type: PieceType,
    pub square: Square,         // current position
    pub reach: [ReachEntry; 104], // flat array, all rays concatenated
    pub reach_count: usize,     // valid entries in reach
}

pub struct SquareReachers {
    pub piece_indices: [u8; 24], // which pieces reach this square
    pub distances: [u8; 24],     // distance from each piece
    pub count: u8,               // valid entries (saturates at 24)
}

pub struct LineMap {
    pub pieces: [PieceLines; 128], // max 128 pieces (4×32)
    pub piece_count: usize,
    pub square_reachers: [SquareReachers; 196], // per-square inverse index
}
```

**Key invariant:** `reach[0..reach_count]` are valid; `reach[reach_count..]` are uninitialized.

---

## 4. Query Engine Contract

### 4.1 QueryVector

```rust
pub struct QueryVector {
    pub material: [i16; 4],      // Mᵢ: sum of piece values per player
    pub positional: [i16; 4],    // Pᵢ: centrality-weighted control
    pub safety: [i16; 4],        // Sᵢ: king defenders - attackers + escapes
    pub crossfire: [i16; 4],     // Oᵢ: converging enemy attack value
}
```

### 4.2 Material Query

```rust
fn query_material(board: &Board) -> [i16; 4]
```

For each player, sum `PieceType::eval_value()` for all active pieces (never `ffa_points()` — Hard
Rule #8).  
**Starting position:** `[4200, 4200, 4200, 4200]` (8×100 + 2×300 + 2×450 + 2×500 + 900)

### 4.3 Positional Control Query

```rust
fn query_positional_control(lines: &LineMap) -> [i16; 4]
```

For each player's pieces, sum centrality weight of every empty square they control (ReachEntry with `first_occupant == None`).

**Centrality weight:**
```
dr = distance from center rank (6.5)
df = distance from center file (6.5)
dist = max(dr, df)
weight = if dist > 5 { 0 } else { 5 - dist }
```

Center squares (ranks 5-8, files 5-8) have highest weight.

### 4.4 King Safety Query

```rust
fn query_king_safety(lines: &LineMap, board: &Board) -> [KingSafety; 4]
```

For each player's king:

**Radius-1 vicinity (8 adjacent squares):**
- Count friendly pieces reaching each square → `defenders`
- Count enemy pieces reaching each square → `attackers`
- Sum enemy piece values reaching → `attack_value`
- Count empty, non-enemy-attacked adjacent squares → `escape_squares`

**Radius-2 knight threats:**
- Check all 8 knight-jump squares around king
- Count enemy knights reaching → add to `attackers` and `attack_value`

```rust
pub struct KingSafety {
    pub defenders: u8,
    pub attackers: u8,
    pub attack_value: i16,
    pub escape_squares: u8,
}
```

### 4.5 Crossfire Query

```rust
fn query_crossfire(lines: &LineMap) -> [i16; 4]
```

For each player's pieces, find squares where multiple enemies converge:

```
for each piece pl of player:
    reachers = lines.reachers_at(pl.square)
    enemy_count = number of non-player pieces in reachers
    enemy_value = sum of piece values of enemy reachers
    if enemy_count >= 2:
        penalty += enemy_value * enemy_count + piece_value(pl.piece_type)
```

**Rationale:** Multiple enemies attacking the same piece = crossfire. Penalty scales with enemy count and their value, plus the value of the threatened piece.

### 4.6 Master Query

```rust
fn run_all_queries(lines: &LineMap, board: &Board) -> QueryVector
```

Runs all queries and returns `QueryVector`. This is the only function the evaluator calls.

### 4.7 Utility Computation

```rust
fn compute_utility(qv: &QueryVector, player: Player) -> i16 {
    let pi = player.index();
    let m = qv.material[pi];
    let p = qv.positional[pi];
    let s = qv.safety[pi];
    let o = qv.crossfire[pi];
    
    // Uᵢ = w₁·Mᵢ + w₂·Pᵢ + w₃·Sᵢ − w₄·Oᵢ
    m * W_MATERIAL + p * W_POSITIONAL + s * W_SAFETY - o * W_CROSSFIRE
}
```

**v0 weights:** `W_MATERIAL = 1, W_POSITIONAL = 2, W_SAFETY = 1, W_CROSSFIRE = 1`

---

## 5. NNUE Architecture

### 5.1 Input Layer

- **Size:** ~50-100 features (from QueryVector + additional queries)
- **Type:** Dense f32 vector
- **Source:** Query engine outputs, not raw board state

**Feature categories:**
- Material: 4 values (normalized by 100)
- Positional: 4 values (normalized)
- Safety: 4 values (defenders, attackers, attack_value, escapes)
- Crossfire: 4 values (normalized)
- Threat surface: per-square threat distances (aggregated)
- Capture opportunities: count, total value, best capture
- Fork threats: count, total forked value
- Pin count: number of pinned pieces
- King mobility: escape squares

### 5.2 Network Architecture

```
Input:        ~100 features
              ↓
Linear(100, 256) + ReLU
              ↓
Linear(256, 32) + ReLU
              ↓
┌─────────────┼─────────────┐
↓             ↓             ↓
Value Head   Policy Head   Exchange Head
Linear(32,4) Linear(32,N)  Linear(32,4)
```

**Value head:** 4 outputs, one per player (centipawns)  
**Policy head:** Variable outputs, one per legal move (softmax probabilities)  
**Exchange head:** 4 outputs, expected material change per player

### 5.3 Training

- **Target:** Search-evaluated centipawn values (not game outcomes)
- **Loss:** MSE on value head + cross-entropy on policy head
- **Teacher:** Search with current best net (strength-gated)
- **Student:** New net learning from teacher's evaluations

---

## 6. Search Contract

### 6.1 Algorithm

**Max^n** with beam search:
- Each node: current player maximizes their own component of V
- Beam width: expand top K moves by heuristic ordering
- Depth: multiples of 4 (round up), fixed at 8 for now

### 6.2 Move Ordering

1. TT move (if hit)
2. Captures (MVV-LVA: Most Valuable Victim, Least Valuable Attacker)
3. Killer moves (from previous iterations)
4. History heuristic scores
5. Remaining moves

### 6.3 Eval-Search Interface

```rust
trait Evaluator {
    fn eval_4vec(&self, state: &GameState) -> [i16; 4];
    fn eval_scalar(&self, state: &GameState, player: Player) -> i16;
}
```

- Search calls `eval_4vec()` at leaf nodes
- Returns vector V = ⟨U₁, U₂, U₃, U₄⟩
- Max^n backs up per-player maxima
- No scalar collapse until final move selection

### 6.4 Transposition Table

- Key: Zobrist hash
- Entry: depth, flag (exact/lower/upper), score vector, best move
- Size: power of 2, configurable (default 16 MB)

---

## 7. Test Specification

### 7.1 Board Construction Tests

**Test: Starting position piece counts**
- Assert each player has exactly 16 pieces
- Assert piece counts: 8 pawns, 2 knights, 2 bishops, 2 rooks, 1 queen, 1 king

**Test: Starting position king squares**
- Red: index 7 (h1)
- Blue: index 84 (a7)
- Yellow: index 188 (g14)
- Green: index 111 (n8)

**Test: Starting position castling rights**
- All 8 bits set (0xFF)

### 7.2 Line Projection Tests

**Test: Rook in center**
- Place rook at g7 (rank 6, file 6) on empty board
- Expected: 26 reach entries
- N: 7 squares, S: 6 squares, E: 7 squares, W: 6 squares
- All entries: `first_occupant == None`

**Test: Rook blocked by friendly**
- Rook at g7, friendly pawn at g9
- g9 entry: `first_occupant == Some(Pawn)`, `xray_continues == true`
- g10 entry: exists (X-ray), `first_occupant == None`

**Test: Rook blocked by enemy**
- Rook at g7, enemy knight at g9
- g9 entry: `first_occupant == Some(Knight)`, `xray_continues == true`

**Test: Bishop diagonals**
- Bishop at g7 on empty board
- NE: 4 squares (h8, i9, j10, k11), NW: 4, SE: 4, SW: 3
- Total: 15 reach entries

**Test: Queen combines rook + bishop**
- Queen at g7 on empty board
- Total: 41 reach entries (26 orthogonal + 15 diagonal)

**Test: Knight jumps**
- Knight at g7 on empty board
- 8 reach entries, all distance 1

**Test: King in corner**
- King at d1 (rank 0, file 3)
- Valid neighbors: d2, e1, e2 (c1 and c2 are invalid corners)
- Total: 3 reach entries

**Test: Pawn forward push**
- Red pawn at d2 on empty board
- d3 (1 step), d4 (2 steps on first move)
- Plus diagonals: e3 (valid, empty), c3 (INVALID corner — skipped)
- Total: 3 reach entries

**Test: Pawn diagonal capture**
- Red pawn at d2, enemy knight at e3
- Forward: d3, d4
- Diagonal: e3 (blocked by knight), c3 (invalid)
- Total: 3 reach entries, e3 has `first_occupant == Some(Knight)`

### 7.3 En Passant Tests

EP is possible only between orthogonal-neighbor players (§1.6). The four valid pairs:

**Test: Red-Blue EP**
- Blue pawn at c4 pushes c4→e4 (East 2). EP target: d4.
- Red pawn at d3 captures EP: d3→d4, removing Blue's e4 pawn.

**Test: Red-Green EP**
- Green pawn at m4 pushes m4→k4 (West 2). EP target: l4.
- Red pawn at l3 captures EP: l3→l4, removing Green's k4 pawn.

**Test: Blue-Yellow EP**
- Yellow pawn at e13 pushes e13→c13 (South 2). EP target: d13.
- Blue pawn at d12 captures EP: d12→d13, removing Yellow's c13 pawn.

**Test: Yellow-Green EP**
- Green pawn at m11 pushes m11→k11 (West 2). EP target: l11.
- Yellow pawn at l12 captures EP: l12→l11, removing Green's k11 pawn.

**Test: Invalid pairs (assert no EP possible):** Red↔Yellow and Blue↔Green (parallel pawn axes).

**Test: En passant near invalid corner**
- An EP capture whose landing square would be an invalid corner must be rejected.

### 7.4 Query Tests

**Test: Material starting position**
- Expected: `[4200, 4200, 4200, 4200]`

**Test: Material after capture**
- Remove Blue queen
- Expected Red: 4200, Blue: 3300, others: 4200

**Test: Positional control symmetric**
- Starting position: all players have similar control values
- Max difference from average < 25%

**Test: King safety defenders**
- Starting position: each king has >0 defenders

**Test: Crossfire empty board**
- Empty board: all crossfire values = 0

**Test: Crossfire with convergence**
- Place 2 enemy rooks attacking same friendly knight
- Friendly player's crossfire > 0

### 7.5 Evaluator Tests

**Test: Starting position symmetry**
- All 4 scores within ±500 of each other

**Test: Scalar matches 4vec**
- `eval_scalar(state, player) == eval_4vec(state)[player.index()]`

**Test: Performance**
- 1000 iterations of `eval_4vec` on starting position
- Release mode: average < 30 µs
- Debug mode: average < 200 µs

---

## 8. Performance Targets

| Metric | Target | Freyja Baseline |
|--------|--------|-----------------|
| Line projection | < 10 µs | ~5 µs |
| Query engine | < 10 µs | ~8 µs |
| Full eval (lines + queries + MLP) | < 30 µs | 21 µs |
| Search nodes/sec | > 100K | TBD |
| Memory per position | < 16 KB | ~8 KB |

---

## 9. File Structure (Proposed)

```
hornet-engine/
├── src/
│   ├── main.rs              # CLI / UCI entry point
│   ├── lib.rs               # Module declarations
│   ├── board/
│   │   ├── mod.rs           # Board struct, accessors, mutations
│   │   ├── types.rs         # Square, Player, PieceType, Piece
│   │   ├── attacks.rs       # Attack generation (for move gen)
│   │   ├── fen4.rs          # FEN4 parsing/serialization
│   │   ├── pgn4.rs          # PGN4 parsing/serialization (see §10)
│   │   └── zobrist.rs       # Zobrist hash keys
│   ├── lines.rs             # Line projection (compute_lines)
│   ├── queries.rs           # Query engine (run_all_queries)
│   ├── eval.rs              # ArrayLinesEvaluator
│   ├── nnue/
│   │   ├── mod.rs           # NNUE evaluator
│   │   ├── network.rs       # MLP architecture
│   │   └── weights.rs       # Weight loading/saving
│   ├── search.rs            # Max^n search with beam
│   ├── move_gen.rs          # Legal move generation
│   ├── move_order.rs        # Move ordering (TT, MVV-LVA, killers, history)
│   ├── tt.rs                # Transposition table
│   └── protocol/            # UCI-like protocol
│       ├── mod.rs
│       ├── parse.rs
│       └── output.rs
├── tests/
│   ├── integration_tests.rs
│   ├── perft.rs             # Performance test (move generation)
│   └── pgn4_roundtrip.rs    # Round-trip all baselines/ games (§10)
├── baselines/               # 16 PGN4 games + tactical_samples.json (for tests/strength gate)
│   ├── human_4pc_game_*.pgn4
│   ├── tactical_samples.json
│   └── README.md
├── Cargo.toml
└── README.md
```

---

## 10. Protocol & I/O Formats

Hornet ingests and emits FEN4 and PGN4 natively (Hard Rule #2) — no external translation layer,
no intermediate JSON, no shell glue.

### 10.1 Protocol Commands

- `position fen4 <string>` — load a position from a FEN4 string.
- `position pgn4 <filepath> [moves <n>]` — load a game from a PGN4 file, optionally advancing to
  ply `n`.
- Game export emits PGN4 to stdout or a filepath.

### 10.2 FEN4 Grammar (chess.com dialect — Hornet's native format)

Format: `<turn>-<dead>-<castleK>-<castleQ>-<points>-<extra>-<board>` (six dash-separated header
fields, then the board).

| Field | Meaning |
|-------|---------|
| turn | side to move: `R` / `B` / `Y` / `G` |
| dead | per-player eliminated flags, RBYG order, comma-separated `0`/`1` |
| castleK | per-player kingside castling rights, RBYG, `0`/`1` |
| castleQ | per-player queenside castling rights, RBYG, `0`/`1` |
| points | per-player FFA points, RBYG, integers |
| extra | a single trailing counter (the lone `0` in the start position) — reserved, see note |
| board | 14 ranks separated by `/`, from display rank 14 (top) down to display rank 1 (bottom) |

Each rank is a comma-separated list of tokens; a token is either a piece
(`<lowercase player><uppercase piece>`, e.g. `yR`, `bP`) or a positive integer = that many
consecutive empty cells. **Empty runs include the invalid corner cells**, so every rank sums to
exactly 14 columns. Piece letters: `P N B R Q K` (a `PromotedQueen` serializes as `Q`).

> **Note (extra field):** the lone counter's full grammar (draw clock and/or en passant) is not yet
> confirmed from a real mid-game chess.com FEN4. Preserve it verbatim for byte-exact round-trips
> until confirmed.

> **Note (second dialect):** `baselines/tactical_samples.json` stores positions in a *different*
> FEN4 dialect (literal `xxx` for corner cells, space-separated trailer). That dialect is **not**
> Hornet's native format; converting those fixtures is a strength-gate-phase concern.

### 10.3 PGN4 Grammar (chess.com)

**Headers** — bracketed tag pairs, e.g. `[Variant "FFA"]`, `[RuleVariants "..."]`,
`[StartFen4 "4PC"]` (the shorthand `"4PC"` denotes the canonical start of §1.3), player names + ELOs,
`[Result "..."]`, `[Termination "..."]`, `[CurrentMove "n"]`.

**Move stream** — numbered rounds, up to four plies per round separated by `..`, e.g.
`1. h2-h3 .. b7-c7 .. g13-g12 .. m8-l8`.

**Move notation the parser must accept:**
- From-to: `d2-d4`; captures `Bn6xBg13`, `Rk14xk8`, `Nk2xl4` (captured piece's token may follow `x`).
- SAN: `Nf3`, `Bxe5`.
- Castling: `O-O`, `O-O-O`.
- Promotion: `e7-e8=D` (default queen); underpromotion `=N` / `=B` / `=R`.
- Annotations: check `+`, mate `#`.

**Round-trip corpus:** the 16 `baselines/human_4pc_game_*.pgn4` files (`tests/pgn4_roundtrip.rs`).

---

## Appendix: Constants Reference

```rust
// Board
pub const BOARD_SIZE: usize = 14;
pub const TOTAL_SQUARES: usize = 196;
pub const VALID_SQUARES: usize = 160;
pub const PLAYERS: usize = 4;
pub const MAX_PIECES_PER_PLAYER: usize = 32;

// Lines
pub const MAX_RAYS_PER_PIECE: usize = 8;
pub const MAX_SQUARES_PER_RAY: usize = 13;
pub const MAX_REACH_PER_PIECE: usize = 104;
pub const MAX_PIECES_TOTAL: usize = 128; // 4 * 32

// Piece EVAL values (centipawns) — used for Mᵢ/SEE/ordering. Distinct from FFA points
// (see PieceType::ffa_points). Never conflate (Hard Rule #8).
pub const PAWN_VALUE: i16 = 100;
pub const KNIGHT_VALUE: i16 = 300;
pub const BISHOP_VALUE: i16 = 450; // [PENDING CALIBRATION] — self-play over {300,350,400,450,500}
pub const ROOK_VALUE: i16 = 500;
pub const QUEEN_VALUE: i16 = 900;
pub const KING_VALUE: i16 = 0;

// Eval weights (v0)
pub const W_MATERIAL: i16 = 1;
pub const W_POSITIONAL: i16 = 2;
pub const W_SAFETY: i16 = 1;
pub const W_CROSSFIRE: i16 = 1;

// Search
pub const MAX_DEPTH: usize = 32;
pub const DEFAULT_BEAM_WIDTH: usize = 30;
```
