//! Layer 3: Per-piece intent tensors (spec §4.8).
//!
//! For every piece on the board, compute a 3×3 intent matrix:
//! - Rows:    Offense, Defense, Vulnerability
//! - Columns: vs opponent A, vs opponent B, vs opponent C
//!
//! The intent tensor captures *who* this piece threatens, *who* it protects against,
//! and *who* threatens it — per opponent. This is richer than the flat per-player
//! query vectors (material/positional/safety/crossfire) because it preserves the
//! piece→opponent structure that gets lost in aggregation.
//!
//! L3 tensors are aggregated into L2 regional subspaces (zones.rs) and then into
//! L1 global synthesis (eval.rs). The search interface never sees tensors directly.
//!
//! Hard Rule #4 preserved: intent is an internal sub-readout, not a 5th component.

use crate::board::Board;
use crate::board::types::{Piece, PieceType, Player, Square};
use crate::lines::LineMap;

// ---------------------------------------------------------------------------
// L3 Intent Tensor
// ---------------------------------------------------------------------------

/// Intent dimensions for a piece against one opponent.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct IntentVector {
    /// How much this piece threatens that opponent's material.
    /// Scaled by target value × attack quality.
    pub offense: i16,
    /// How much this piece protects friendly material from that opponent.
    /// Scaled by protected value × defense quality.
    pub defense: i16,
    /// How vulnerable this piece is to that opponent.
    /// Scaled by own value × threat severity from that opponent.
    pub vulnerability: i16,
}

/// Per-piece intent tensor: intent against each of the 3 opponents.
/// Opponents are ordered by turn sequence from the piece owner:
///   opponent[0] = owner.next()
///   opponent[1] = owner.next().next()
///   opponent[2] = owner.next().next().next()
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PieceIntent {
    pub square: Square,
    pub piece: Piece,
    /// Intent vs opponent[0], opponent[1], opponent[2]
    pub vs: [IntentVector; 3],
}

impl PieceIntent {
    /// Get the intent vector for a specific opponent player.
    /// Returns None if the opponent is not one of this piece's 3 opponents
    /// (i.e., opponent == owner, which should never happen).
    pub fn intent_vs(&self, opponent: Player) -> Option<&IntentVector> {
        let ops = self.piece.player.opponents();
        for (i, &op) in ops.iter().enumerate() {
            if op == opponent {
                return Some(&self.vs[i]);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// IntentMap
// ---------------------------------------------------------------------------

/// Intent tensors for all pieces on the board.
pub struct IntentMap {
    pub intents: [PieceIntent; 128], // max pieces
    pub count: usize,
}

impl IntentMap {
    pub fn new() -> Self {
        IntentMap {
            intents: [PieceIntent {
                square: Square::new(0),
                piece: Piece::new(Player::Red, PieceType::Pawn),
                vs: [IntentVector::default(); 3],
            }; 128],
            count: 0,
        }
    }

    /// Sum all offense intent for a player (how much they threaten each opponent).
    /// Returns [offense_vs_red, offense_vs_blue, offense_vs_yellow, offense_vs_green]
    /// where the player's own slot is 0.
    pub fn offense_by_target(&self, player: Player) -> [i16; 4] {
        let mut v = [0i16; 4];
        for i in 0..self.count {
            let pi = &self.intents[i];
            if pi.piece.player != player {
                continue;
            }
            let ops = player.opponents();
            for (j, &op) in ops.iter().enumerate() {
                v[op.index()] += pi.vs[j].offense;
            }
        }
        v
    }

    /// Sum all defense intent for a player (how much they protect against each opponent).
    pub fn defense_by_threat(&self, player: Player) -> [i16; 4] {
        let mut v = [0i16; 4];
        for i in 0..self.count {
            let pi = &self.intents[i];
            if pi.piece.player != player {
                continue;
            }
            let ops = player.opponents();
            for (j, &op) in ops.iter().enumerate() {
                v[op.index()] += pi.vs[j].defense;
            }
        }
        v
    }

    /// Sum all vulnerability for a player (how threatened they are by each opponent).
    pub fn vulnerability_by_attacker(&self, player: Player) -> [i16; 4] {
        let mut v = [0i16; 4];
        for i in 0..self.count {
            let pi = &self.intents[i];
            if pi.piece.player != player {
                continue;
            }
            let ops = player.opponents();
            for (j, &op) in ops.iter().enumerate() {
                v[op.index()] += pi.vs[j].vulnerability;
            }
        }
        v
    }

    /// Aggregate offense minus vulnerability per player.
    /// Positive = player is more threatening than threatened.
    pub fn net_intent(&self) -> [i16; 4] {
        let mut net = [0i16; 4];
        for player in Player::ALL {
            let pi = player.index();
            let off = self.offense_by_target(player);
            let vuln = self.vulnerability_by_attacker(player);
            // Sum offense against all opponents, minus vulnerability from all opponents
            let total_offense: i16 = off.iter().sum();
            let total_vuln: i16 = vuln.iter().sum();
            net[pi] = total_offense - total_vuln;
        }
        net
    }
}

// ---------------------------------------------------------------------------
// Intent computation
// ---------------------------------------------------------------------------

/// Compute intent tensors for all pieces.
///
/// For each piece P belonging to player X:
/// - **Offense vs Y**: For each of Y's pieces that P attacks, sum (target_value × attack_quality)
/// - **Defense vs Y**: For each of X's pieces that Y attacks, if P defends it, sum (protected_value × defense_quality)
/// - **Vulnerability vs Y**: If Y attacks P, score (own_value × threat_severity_from_Y)
pub fn compute_intent_map(lines: &LineMap, board: &Board) -> IntentMap {
    let mut map = IntentMap::new();

    // First pass: build per-square attacker lists for efficient lookup
    // We need to know: for each square, which enemies attack it (and with what quality)
    let square_attackers = build_attack_index(lines);

    for i in 0..lines.piece_count {
        let pl = &lines.pieces[i];
        let piece = match board.piece_at(pl.square) {
            Some(p) if p.player == pl.player && p.piece_type == pl.piece_type => p,
            _ => continue,
        };

        let owner = piece.player;
        let ops = owner.opponents();
        let mut vs = [IntentVector::default(); 3];

        // --- OFFENSE: what does this piece threaten? ---
        // Iterate this piece's reach entries; for each enemy-occupied square,
        // score the threat.
        for e in pl.entries() {
            if let Some(target) = e.first_occupant {
                if target.player == owner {
                    continue; // can't attack own piece
                }
                // Find which opponent slot this target belongs to
                if let Some(slot) = opponent_slot(&ops, target.player) {
                    let threat_score = threat_quality(pl.piece_type, target.piece_type, e.distance);
                    vs[slot].offense += threat_score;
                }
            }
        }

        // --- DEFENSE: what friendly material does this piece protect? ---
        // For each friendly piece that is attacked by an enemy, if this piece
        // reaches that friendly piece's square, it contributes defense.
        for j in 0..lines.piece_count {
            let friendly = &lines.pieces[j];
            if friendly.player != owner {
                continue;
            }
            if friendly.square == pl.square {
                continue; // skip self
            }

            // Is this friendly piece under attack by any enemy?
            let friendly_attackers = &square_attackers[friendly.square.index() as usize];
            if friendly_attackers.attackers.is_empty() {
                continue; // no threat to defend against
            }

            // Does our piece reach the friendly square?
            if piece_reaches_square(pl, friendly.square) {
                // Defense vs each attacking opponent
                for (slot, &op) in ops.iter().enumerate() {
                    let op_attacks = friendly_attackers
                        .attackers
                        .iter()
                        .any(|&(attacker_player, _, _)| attacker_player == op);
                    if op_attacks {
                        let defense_score = defense_quality(
                            pl.piece_type,
                            friendly.piece_type,
                            piece_value(friendly.piece_type),
                        );
                        vs[slot].defense += defense_score;
                    }
                }
            }
        }

        // --- VULNERABILITY: who threatens this piece? ---
        let my_attackers = &square_attackers[pl.square.index() as usize];
        for &(attacker_player, attacker_piece_type, distance) in my_attackers.attackers.iter() {
            if let Some(slot) = opponent_slot(&ops, attacker_player) {
                let vuln_score = vulnerability_score(
                    piece_value(piece.piece_type),
                    attacker_piece_type,
                    distance,
                    my_attackers.attackers.len() as u8,
                    piece.player,
                    attacker_player,
                );
                vs[slot].vulnerability += vuln_score;
            }
        }

        if map.count < 128 {
            map.intents[map.count] = PieceIntent {
                square: pl.square,
                piece,
                vs,
            };
            map.count += 1;
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Helper: square attacker index
// ---------------------------------------------------------------------------

/// For each square, list of (attacker_player, attacker_piece_type, distance).
/// Built from LineMap for O(1) threat lookup during intent computation.
struct SquareAttackers {
    attackers: Vec<(Player, PieceType, u8)>,
}

fn build_attack_index(lines: &LineMap) -> [SquareAttackers; 196] {
    let mut index: [SquareAttackers; 196] = std::array::from_fn(|_| SquareAttackers {
        attackers: Vec::with_capacity(8),
    });

    for i in 0..lines.piece_count {
        let pl = &lines.pieces[i];
        for e in pl.entries() {
            if e.first_occupant.is_some() {
                // This piece attacks the occupant's square
                let target_sq = e.square;
                let idx = target_sq.index() as usize;
                index[idx]
                    .attackers
                    .push((pl.player, pl.piece_type, e.distance));
            }
        }
    }

    index
}

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

#[inline]
fn piece_value(pt: PieceType) -> i16 {
    pt.eval_value()
}

/// Which opponent slot (0,1,2) does `target` occupy in `owner`'s opponent list?
fn opponent_slot(ops: &[Player; 3], target: Player) -> Option<usize> {
    ops.iter().position(|&p| p == target)
}

/// Does a PieceLines record reach a specific square?
fn piece_reaches_square(pl: &crate::lines::PieceLines, sq: Square) -> bool {
    pl.entries().iter().any(|e| e.square == sq)
}

/// Threat quality: how dangerous is piece A attacking piece B?
/// Higher when attacker is stronger relative to target, and when target is valuable.
fn threat_quality(attacker: PieceType, target: PieceType, _distance: u8) -> i16 {
    let target_val = piece_value(target);
    let attacker_val = piece_value(attacker);

    // Base: target value (threatening a queen is more offensive than threatening a pawn)
    let base = target_val;

    // Multiplier: attacker stronger than target = more credible threat
    let mult = if attacker_val > target_val {
        15 // 1.5x
    } else if attacker_val == target_val {
        10 // 1.0x
    } else {
        7 // 0.7x — attacking up the value chain is less credible
    };

    (base * mult) / 10
}

/// Defense quality: how valuable is this piece defending a threatened friendly?
fn defense_quality(defender: PieceType, _protected_piece: PieceType, protected_value: i16) -> i16 {
    let defender_val = piece_value(defender);

    // Base: protected value
    let base = protected_value;

    // Multiplier: defender at least as strong as protected piece = solid defense
    let mult = if defender_val >= protected_value {
        12 // 1.2x — good defense
    } else {
        8 // 0.8x — under-defended
    };

    (base * mult) / 10
}

/// Turn-order proximity: how soon can this threatener act?
///
/// In 4PC, a threat from the next player is more dangerous than a threat
/// from a player 2-3 turns away, because you have fewer chances to escape.
///
/// Weight: 1.0 = next to move, 0.6 = 2 turns away, 0.3 = 3 turns away.
fn turn_proximity_weight(owner: Player, threatener: Player) -> i16 {
    // Turn order: Red(0) -> Blue(1) -> Yellow(2) -> Green(3)
    let o = owner.index() as i8;
    let t = threatener.index() as i8;
    // Distance in turns (1, 2, or 3)
    let dist = ((t - o + 4) % 4) as i16;
    match dist {
        1 => 10, // next: 1.0x
        2 => 6,  // 2 away: 0.6x
        3 => 3,  // 3 away: 0.3x
        _ => 5,  // self (shouldn't happen): 0.5x
    }
}

/// Vulnerability score: how threatened is this piece?
///
/// **Turn-order-aware:** a threat from the next player is weighted higher
/// than a threat from a player 2-3 turns away. This is critical for 4PC
/// because you can't recapture immediately — 2 other players move first.
fn vulnerability_score(
    own_value: i16,
    attacker_type: PieceType,
    _distance: u8,
    attacker_count: u8,
    piece_owner: Player,
    attacker_player: Player,
) -> i16 {
    let attacker_val = piece_value(attacker_type);

    // Base: own value (valuable pieces are more vulnerable when attacked)
    let base = own_value;

    // Threat credibility: attacker stronger = more vulnerable
    let threat_mult = if attacker_val >= own_value {
        15 // 1.5x — serious threat
    } else {
        8 // 0.8x — attacker is weaker, less credible
    };

    // Multiple attackers compound vulnerability
    let count_mult = 10 + attacker_count.saturating_sub(1) as i16 * 5; // 1.0, 1.5, 2.0, ...

    // Turn-order proximity: next player = 1.0, 2 away = 0.6, 3 away = 0.3
    let proximity = turn_proximity_weight(piece_owner, attacker_player);

    // Use i32 for intermediate to prevent overflow
    let score =
        (base as i32) * (threat_mult as i32) * (count_mult as i32) * (proximity as i32) / 1000;
    score.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

// ---------------------------------------------------------------------------
// Aggregation into eval components (feeds into queries.rs / eval.rs)
// ---------------------------------------------------------------------------

/// Aggregate L3 intent tensors into per-player vectors that feed the 4 eval components.
///
/// Returns 4 vectors, each [i16; 4]:
/// - [0] offense_aggregate:   feeds into positional (aggressive posture)
/// - [1] defense_aggregate:   feeds into safety (defensive posture)
/// - [2] vulnerability_agg:   feeds into crossfire (threat exposure)
/// - [3] net_intent:          composite (offense - vulnerability)
///
/// These are **additive sub-readouts** — they get folded into existing components
/// with small weights, not new components.
pub fn aggregate_intent_for_eval(map: &IntentMap) -> IntentEvalAggregate {
    let mut offense = [0i16; 4];
    let mut defense = [0i16; 4];
    let mut vulnerability = [0i16; 4];
    let mut net = [0i16; 4];

    for player in Player::ALL {
        let pi = player.index();
        let off = map.offense_by_target(player);
        let def = map.defense_by_threat(player);
        let vuln = map.vulnerability_by_attacker(player);

        // Total offense: sum of all threats this player makes
        offense[pi] = off.iter().sum();

        // Total defense: sum of all protection this player provides
        defense[pi] = def.iter().sum();

        // Total vulnerability: sum of all threats against this player
        vulnerability[pi] = vuln.iter().sum();

        // Net: total offense minus total vulnerability
        net[pi] = offense[pi] - vulnerability[pi];
    }

    IntentEvalAggregate {
        offense,
        defense,
        vulnerability,
        net,
    }
}

/// L3 intent aggregated for consumption by L1 (eval.rs).
#[derive(Clone, Debug, PartialEq)]
pub struct IntentEvalAggregate {
    /// Total offensive posture per player (sum of all threats made).
    pub offense: [i16; 4],
    /// Total defensive posture per player (sum of all protection provided).
    pub defense: [i16; 4],
    /// Total threat exposure per player (sum of all threats received).
    pub vulnerability: [i16; 4],
    /// Net intent: offense - vulnerability.
    pub net: [i16; 4],
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;
    use crate::lines::{LineMap, compute_lines};

    fn start() -> Board {
        fen4::parse(fen4::START_FEN4).unwrap()
    }

    #[test]
    fn intent_map_counts_all_pieces() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);
        assert_eq!(map.count, 64, "start position has 64 pieces");
    }

    #[test]
    fn start_position_symmetric_offense() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);
        let agg = aggregate_intent_for_eval(&map);

        // All players should have similar offense at start
        let avg = agg.offense.iter().map(|&x| x as i32).sum::<i32>() / 4;
        for (i, &off) in agg.offense.iter().enumerate() {
            let diff = (off as i32 - avg).abs();
            assert!(
                diff < 500,
                "player {i} offense {off} deviates too far from avg {avg}"
            );
        }
    }

    #[test]
    fn start_position_symmetric_vulnerability() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);
        let agg = aggregate_intent_for_eval(&map);

        let avg = agg.vulnerability.iter().map(|&x| x as i32).sum::<i32>() / 4;
        for (i, &vuln) in agg.vulnerability.iter().enumerate() {
            let diff = (vuln as i32 - avg).abs();
            assert!(
                diff < 500,
                "player {i} vulnerability {vuln} deviates too far from avg {avg}"
            );
        }
    }

    #[test]
    fn undefended_queen_has_high_vulnerability() {
        let mut b = Board::empty();
        let sq = Square::from_algebraic("d7").unwrap();
        b.set_piece(sq, Some(Piece::new(Player::Blue, PieceType::Queen)));
        // Red rook attacks the queen
        let attacker_sq = Square::from_algebraic("d1").unwrap();
        b.set_piece(attacker_sq, Some(Piece::new(Player::Red, PieceType::Rook)));

        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);

        let queen_intent = map
            .intents
            .iter()
            .find(|pi| pi.piece.piece_type == PieceType::Queen)
            .expect("queen in intent map");

        // Blue queen should have vulnerability vs Red
        let red_vuln = queen_intent.intent_vs(Player::Red).unwrap();
        assert!(
            red_vuln.vulnerability > 0,
            "undefended queen should be vulnerable to rook"
        );
        // Note: the queen DOES have offense — it attacks the rook! In FFA, a queen
        // attacks anything it reaches, even if it's "vulnerable." Offense and
        // vulnerability are independent dimensions.
        assert!(
            red_vuln.offense > 0,
            "queen attacks the rook, so it has offense"
        );
    }

    #[test]
    fn rook_threatens_queen_offense() {
        let mut b = Board::empty();
        let queen_sq = Square::from_algebraic("d7").unwrap();
        b.set_piece(queen_sq, Some(Piece::new(Player::Blue, PieceType::Queen)));
        let rook_sq = Square::from_algebraic("d1").unwrap();
        b.set_piece(rook_sq, Some(Piece::new(Player::Red, PieceType::Rook)));

        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);

        let rook_intent = map
            .intents
            .iter()
            .find(|pi| pi.piece.piece_type == PieceType::Rook)
            .expect("rook in intent map");

        // Red rook should have offense vs Blue queen
        let blue_offense = rook_intent.intent_vs(Player::Blue).unwrap();
        assert!(
            blue_offense.offense > 0,
            "rook should have offense vs queen"
        );
    }

    #[test]
    fn defended_piece_has_defense_intent() {
        let mut b = Board::empty();
        let queen_sq = Square::from_algebraic("d7").unwrap();
        b.set_piece(queen_sq, Some(Piece::new(Player::Blue, PieceType::Queen)));
        // Red rook attacks
        b.set_piece(
            Square::from_algebraic("d1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Rook)),
        );
        // Blue bishop defends the queen
        b.set_piece(
            Square::from_algebraic("b5").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Bishop)),
        );

        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);

        let bishop_intent = map
            .intents
            .iter()
            .find(|pi| pi.piece.piece_type == PieceType::Bishop)
            .expect("bishop in intent map");

        // Blue bishop should have defense vs Red (protecting the queen)
        let red_defense = bishop_intent.intent_vs(Player::Red).unwrap();
        assert!(
            red_defense.defense > 0,
            "bishop should defend queen vs Red rook"
        );
    }

    #[test]
    fn net_intent_zero_sum_approximate() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_intent_map(&lm, &b);
        let net = map.net_intent();

        // Net intent is NOT strictly zero-sum because the same piece can have both
        // offense and vulnerability (a queen attacks a rook AND is attacked by it).
        // The key invariant: each player's net should be in a reasonable range.
        for (i, &n) in net.iter().enumerate() {
            assert!(
                n.abs() < 10000,
                "player {i} net intent {n} unreasonably large"
            );
        }
    }
}
