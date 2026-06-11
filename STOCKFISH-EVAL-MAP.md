# Stockfish eval terms → 4PC (Hornet) mapping

**Purpose:** the full set of Stockfish *classical* hand-crafted eval terms, each mapped to how (or
whether) it applies to 4-player chess. Use this as the menu for eval features — every candidate gates
on `move_tune` + `move_match` (and outcome-tuning once decisive games exist).

**Sources (verified from source, not memory):** [evaluate.cpp (sf_11)](https://github.com/official-stockfish/Stockfish/blob/sf_11/src/evaluate.cpp),
[pawns.cpp (sf_11)](https://github.com/official-stockfish/Stockfish/blob/sf_11/src/pawns.cpp). This is
the *classical* eval (≤ SF11); SF12+ kept it as a fallback, SF16 removed it for NNUE. NNUE learns these
same patterns implicitly. **The 4PC-translation column is my design reasoning, NOT a Stockfish fact —
each must be validated.**

## 4PC translation principles (apply to every term)
1. **4 players, not 2.** "Enemy" = the 3 opponents. SF terms that say "enemy pawn / enemy attacker"
   become "any of 3 opponents" (sum or max). Hornet's mean-relative V + `query_crossfire` already do
   multi-opponent.
2. **Cross board (14×14, 4 dead 3×3 corners, four 8×3 arms).** "Center = good" is FALSE here (move-match
   proved it). Files/ranks are player-relative; the pawn **lane = axis perpendicular to forward** (Red/Yellow
   → file; Blue/Green → rank).
3. **King is capturable (elimination), not check/mate.** King-safety = avoid capture. BUT the eval is
   **points-blind** (Hard Rule #8) — king-capture ≈ 0 in search today, so king-threat terms are damped by
   design (this is why self-play is drawish; revisit it).
4. **V contract = M/P/S/O (Hard Rule #4).** Every term folds into Material / Positional / Safety /
   crossfire(O), never a 5th component.
5. **Our finding (EXP-015):** static *per-square/scalar* positional is dead for move-match (8 variants,
   all P=0). The untested, promising class is **relational** (pawn structure, outposts, rook-on-open-line,
   king-shelter) and **dynamic** (threats — which belong partly in search/qsearch). This map flags which
   is which.

---

## Material & piece-value terms → **M**
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| Piece values | Base material | 4PC values differ (bishops/rooks debated on the cross) — re-tune | **HAVE** (M=6 dominant) |
| **Imbalance** | Bonuses for material *combinations* (bishop pair, rook redundancy, knight+pawn synergy) | Applies; **bishop pair** is the cheap classic | **NEW** — cheap, relational-ish, worth a test |
| PSQT | Per-square positional value per piece | **DEAD** — center hostile; 8 variants P=0 | tested, ablated (PST v3) |

## Mobility & piece placement → **P**
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| MobilityBonus | # safe squares a piece attacks | Applies; **tested → P=0 move-match** (may help outcomes) | parked (`query_mobility`, ablated) |
| **Outpost / ReachableOutpost** | Minor on a square pawn-supported & not pawn-attackable | **relational**; reparam over 3 opponents' pawns + lane | **NEW — high** (relational, the live class) |
| MinorBehindPawn | Minor directly behind a friendly pawn | Applies (lane-relative "behind") | NEW — cheap |
| BishopPawns | Penalty: own pawns on bishop's color | Color squares exist on the cross | NEW — cheap |
| **RookOnFile (open/semi-open)** | Rook on a file with no friendly (or any) pawns | **THE correct rook term** — rooks want *open lines*, not edges (fixes the rook-edge confound). Reparam "file" → the rook's open rank/file line | **NEW — high** (replaces the dropped edge bonus) |
| RookOnQueenFile | Rook shares file with own queen | Applies | NEW — minor |
| TrappedRook | Rook low-mobility near own king | Applies | NEW — minor |
| LongDiagonalBishop | Bishop on long diagonal through center | Center hostile → questionable | low |
| CorneredBishop | Chess960 corner trap | **N/A** | — |

## King safety → **S**
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| KingAttackWeights + safe-check (Q/R/B/N) + KingDanger | Weighted count of enemy attackers near king + danger from safe checks | **Big reparam**: danger = sum over 3 opponents' attackers. The SF king-danger formula is the sophisticated version of what we have | **HAVE (basic)** `safety_scalar` (clamped danger); could enrich |
| **Pawn shelter / storm** (ShelterStrength, UnblockedStorm, BlockedStorm) | Friendly pawns in front of king; enemy pawns advancing on it | **relational**; reparam over the king's shelter direction + 3 storms | **NEW — high** (we have danger, not shelter) |
| KingProtector | Piece distance from own king | Applies | NEW — minor |
| PawnlessFlank / FlankAttacks | King exposed on a pawnless side | Applies | NEW — minor |
| WeakQueen | Queen pinned / discovered-attack | Applies (pins exist) | NEW — minor (→ also threats) |
| ⚠ damping | — | All king terms are weakened by the **points-blind** rule (king-capture≈0). Decide if king-safety should matter more (would de-drawish self-play) | open design Q |

## Threats → **P** (threat) / **O** (SEE material-at-risk)
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| Hanging | Undefended attacked pieces | = SEE material-at-risk | **HAVE** `query_crossfire` (O) |
| ThreatByMinor/Rook/King (by attacked type) | Bonus for attacking enemy pieces, scaled | Reparam over 3 opponents | **HAVE (basic)** `query_threats` (capped, attacker≤target); could granularize by type |
| ThreatBySafePawn / ThreatByPawnPush | Safe pawn attacks / threatening pushes | Applies, lane-relative | NEW — medium |
| RestrictedPiece | Limiting enemy mobility | Applies | NEW — minor |
| KnightOnQueen / SliderOnQueen | Piece set to fork/attack enemy queen | Applies | NEW — minor (tactical → maybe search, not eval) |

## Pawn structure → **P** (pawns.cpp — the canonical RELATIONAL class)
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| **Isolated** | No friendly pawn on adjacent lanes | lane = ⊥ forward (Red/Yellow file, Blue/Green rank) | **PARKED — Kimi #1, "real gain"** (data-blocked) |
| **Doubled** | ≥2 friendly pawns same lane | same | PARKED — Kimi #1 |
| **Connected / phalanx / supported** | Pawns defending each other / adjacent same rank | reparam to lane frame | PARKED — high |
| Backward | Lags behind adjacent-lane pawns, can't advance safely | reparam | medium |
| WeakUnopposed / WeakLever | Weak pawn not opposed / capturable from 2 sides | reparam over 3 opponents | medium |
| Passed | No enemy stopper ahead | **HARD in 4PC** (3 opponents, central crossing promotion) — KIMI defers | deferred |

## Board-level → **P** / meta
| SF term | Measures | 4PC translation | Hornet |
|---|---|---|---|
| Space | Safe squares behind own pawns (center) | Center hostile → reinterpret as **own-quadrant/territory control** | **PARTIAL** — `zones.rs` zone-control is a space analog (measured, untested in eval) |
| Tempo / Initiative | Side-to-move bonus + position-complexity adjustment | Dev-tempo tested → P=0 move-match (queen-before-bishop is real but doesn't pick the move) | parked (`query_tempo`, ablated) |

---

## Priority for 4PC (given EXP-015)
The decisive split: SF's eval is **mostly relational** (pawn structure, outposts, rook-on-open-line,
king-shelter, threats) — *not* per-square tables. Our move-match work **killed the per-square class** and
left **the entire relational class untested** (except parked pawn structure). So the SF map says the eval
gain is in exactly the features we haven't reached yet.

1. **Pawn structure** (isolated/doubled/connected) — the canonical relational term, Kimi's #1. *The* swing.
2. **Rook on open line** — relational, cheap, and it *replaces* the wrong rook-edge bonus with the right
   idea (rooks want open lines, not the rim). Directly settles the rook debate.
3. **Outposts** (pawn-supported minors) + **king pawn-shelter** — relational, the next tier.
4. **Imbalance / bishop pair** — cheap material-combination test (folds into M).
5. Enrich **threats** (by attacker/attacked type) and **king-danger** (3-opponent attacker formula) — we
   have basic versions.

**Caveats that gate all of it:**
- **Data.** Relational terms likely won't move *move-match* (top-1 is material/tactics-dominated) but
  should move *outcomes* — which needs decisive games (ours are drawish). So most of this is blocked on
  more human games + outcome-tuning, same conclusion as EXP-015.
- **Tactics belong in search, not eval.** KnightOnQueen / SliderOnQueen / sacrifice resolution are
  search+qsearch+SEE work (Hornet has qsearch); don't over-build them as static terms.
- **The points-blind king** damps every king term — decide whether to relax Hard Rule #8 so the engine
  values eliminations (would also de-drawish self-play).
