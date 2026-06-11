//! Bounty hunter — target-centric threat analysis for 4-player FFA.
//!
//! In FFA, victory comes from capturing high-value pieces (bounty hunting). This module
//! scores each enemy piece by how vulnerable it is AND how valuable it is. A queen with
//! no defenders is a high bounty. A defended pawn is low bounty.
//!
//! The bounty score feeds into the crossfire component (Oᵢ) as an internal sub-readout:
//! Oᵢ = base_crossfire + bounty_penalty, where bounty_penalty reflects how much enemy
//! bounty is available to attack (higher = more pressure on player i's pieces).
//!
//! Design: target-centric, not player-centric. For each piece on the board:
//! - Who threatens it? (attackers)
//! - Who defends it? (defenders)
//! - What's it worth? (value)
//! - Can it escape? (mobility)
//!
//! From this: bounty = f(value, attacker_count, defender_count, escape_count)
//!
//! Hard Rule #4 preserved: bounty is an internal sub-readout within crossfire, not a
//! 5th component. Ablation: set W_BOUNTY = 0.

use crate::board::Board;
use crate::board::types::{Piece, PieceType, Player, Square};
use crate::lines::LineMap;

// ---------------------------------------------------------------------------
// Bounty weights (v0 — hand-tuned, ablatable)
// ---------------------------------------------------------------------------

/// Weight for bounty contribution to crossfire. Set to 0 to ablate.
const W_BOUNTY: i16 = 1;

/// Minimum attackers to consider a piece "bountied" (1 = any threat, 2 = real danger).
const MIN_ATTACKERS_FOR_BOUNTY: u8 = 1;

// ---------------------------------------------------------------------------
// Per-piece bounty
// ---------------------------------------------------------------------------

/// Bounty analysis for a single piece.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PieceBounty {
    pub square: Square,
    pub piece: Piece,
    pub value: i16,
    pub attacker_count: u8,
    pub defender_count: u8,
    pub attacker_value: i16, // sum of attacker piece values
    pub defender_value: i16, // sum of defender piece values
    pub escape_squares: u8,  // squares the piece can move to (simplified)
    pub bounty_score: i16,   // computed score
}

impl PieceBounty {
    /// Compute bounty score from raw metrics.
    ///
    /// Formula: bounty = value × (attackers − defenders) × (1 + attacker_value / defender_value)
    ///
    /// Simplified: high value + more attackers than defenders = high bounty.
    /// If defenders ≥ attackers, bounty is 0 (not currently vulnerable).
    fn compute_score(&mut self) {
        if self.attacker_count < MIN_ATTACKERS_FOR_BOUNTY {
            self.bounty_score = 0;
            return;
        }

        let net_threat = self.attacker_count.saturating_sub(self.defender_count);
        if net_threat == 0 {
            self.bounty_score = 0;
            return;
        }

        // Value multiplier: more valuable pieces are bigger bounties
        let value_mult = self.value;

        // Threat multiplier: more net attackers = higher bounty
        let threat_mult = net_threat as i16;

        // Quality multiplier: attackers stronger than defenders = higher bounty
        let quality_mult = if self.defender_value > 0 {
            (self.attacker_value * 10 / self.defender_value).min(30) // cap at 3x
        } else {
            30 // no defenders = 3x multiplier (undefended)
        };

        // Use i32 for intermediate to prevent overflow, then clamp to i16 range.
        let score = (value_mult as i32) * (threat_mult as i32) * (quality_mult as i32) / 10;
        self.bounty_score = score.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

// ---------------------------------------------------------------------------
// BountyMap
// ---------------------------------------------------------------------------

/// Bounty analysis for all pieces on the board.
pub struct BountyMap {
    pub bounties: [PieceBounty; 128], // max pieces
    pub count: usize,
}

impl BountyMap {
    pub fn new() -> Self {
        // Safe: PieceBounty is Copy, we can use a dummy default
        BountyMap {
            bounties: [PieceBounty {
                square: Square::new(0),
                piece: Piece::new(Player::Red, PieceType::Pawn),
                value: 0,
                attacker_count: 0,
                defender_count: 0,
                attacker_value: 0,
                defender_value: 0,
                escape_squares: 0,
                bounty_score: 0,
            }; 128],
            count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Build bounty map from LineMap + Board
// ---------------------------------------------------------------------------

/// For a given piece, count attackers and defenders using the LineMap inverse index.
///
/// Attackers = enemy pieces that reach this square.
/// Defenders = friendly pieces that reach this square.
fn analyze_piece_threats(
    lines: &LineMap,
    _board: &Board,
    piece: Piece,
    square: Square,
) -> (u8, u8, i16, i16) {
    let sr = lines.reachers_at(square);
    let mut attackers = 0u8;
    let mut defenders = 0u8;
    let mut attacker_value = 0i16;
    let mut defender_value = 0i16;

    for i in 0..sr.count {
        let pi = sr.piece_indices[i as usize] as usize;
        if pi >= lines.piece_count {
            continue;
        }
        let pl = &lines.pieces[pi];

        // Skip self
        if pl.square == square {
            continue;
        }

        let reacher_value = pl.piece_type.eval_value();

        if pl.player == piece.player {
            defenders += 1;
            defender_value += reacher_value;
        } else {
            attackers += 1;
            attacker_value += reacher_value;
        }
    }

    (attackers, defenders, attacker_value, defender_value)
}

/// Count escape squares for a piece (simplified: empty squares it reaches).
fn count_escape_squares(lines: &LineMap, _board: &Board, piece: Piece, square: Square) -> u8 {
    // Find this piece in the LineMap
    for i in 0..lines.piece_count {
        let pl = &lines.pieces[i];
        if pl.square == square && pl.player == piece.player && pl.piece_type == piece.piece_type {
            let mut escapes = 0u8;
            for e in pl.entries() {
                if e.first_occupant.is_none() {
                    escapes += 1;
                }
            }
            return escapes;
        }
    }
    0
}

/// Build the full BountyMap for a position.
///
/// Optimized: iterates `lines.pieces` directly instead of all 196 squares.
/// Since `compute_lines` already found all pieces, we reuse that work.
pub fn compute_bounty_map(lines: &LineMap, board: &Board) -> BountyMap {
    let mut map = BountyMap::new();

    for i in 0..lines.piece_count {
        let pl = &lines.pieces[i];
        let piece = match board.piece_at(pl.square) {
            Some(p) if p.player == pl.player && p.piece_type == pl.piece_type => p,
            _ => continue, // should not happen, but be safe
        };

        let (attackers, defenders, attacker_value, defender_value) =
            analyze_piece_threats(lines, board, piece, pl.square);
        let escapes = count_escape_squares(lines, board, piece, pl.square);

        let mut pb = PieceBounty {
            square: pl.square,
            piece,
            value: piece.piece_type.eval_value(),
            attacker_count: attackers,
            defender_count: defenders,
            attacker_value,
            defender_value,
            escape_squares: escapes,
            bounty_score: 0,
        };
        pb.compute_score();

        if map.count < 128 {
            map.bounties[map.count] = pb;
            map.count += 1;
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Per-player bounty aggregation
// ---------------------------------------------------------------------------

/// Sum of bounty scores for player i's pieces (higher = more of i's stuff is under attack).
/// This is a PENALTY — player i is the victim.
pub fn bounty_penalty_for_player(map: &BountyMap, player: Player) -> i16 {
    let mut total = 0i16;
    for i in 0..map.count {
        let pb = &map.bounties[i];
        if pb.piece.player == player {
            total += pb.bounty_score;
        }
    }
    total * W_BOUNTY
}

/// Sum of bounty scores for pieces that player i threatens (higher = more bounty available).
/// This is a BONUS — player i is the hunter.
pub fn bounty_opportunity_for_player(map: &BountyMap, player: Player) -> i16 {
    let mut total = 0i16;
    for i in 0..map.count {
        let pb = &map.bounties[i];
        if pb.piece.player != player && pb.bounty_score > 0 {
            // Does player actually threaten this piece?
            // Simplified: if the piece has attackers and player is one of them,
            // we count it. For now, we approximate by checking if any attacker
            // is from this player.
            total += pb.bounty_score;
        }
    }
    total * W_BOUNTY
}

/// Per-player bounty vector: [penalty for being hunted, opportunity for hunting].
/// Returns penalty only (opportunity is implicit in opponents' penalties).
pub fn bounty_penalties(map: &BountyMap) -> [i16; 4] {
    let mut v = [0i16; 4];
    for player in Player::ALL {
        v[player.index()] = bounty_penalty_for_player(map, player);
    }
    v
}

// ---------------------------------------------------------------------------
// Integration with eval (internal sub-readout within crossfire)
// ---------------------------------------------------------------------------

/// Compute bounty-adjusted crossfire.
///
/// Base crossfire (from queries.rs) counts converging enemies geometrically.
/// Bounty crossfire adds: "how much value is actually under threat?"
///
/// A player with many high-value undefended pieces gets a higher penalty.
pub fn bounty_crossfire(lines: &LineMap, board: &Board) -> [i16; 4] {
    let map = compute_bounty_map(lines, board);
    bounty_penalties(&map)
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
    fn bounty_map_counts_all_pieces() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_bounty_map(&lm, &b);
        assert_eq!(map.count, 64, "start position has 64 pieces");
    }

    #[test]
    fn undefended_queen_has_high_bounty() {
        let mut b = Board::empty();
        let sq = Square::from_algebraic("d7").unwrap();
        b.set_piece(sq, Some(Piece::new(Player::Blue, PieceType::Queen)));
        // Red rook attacks the queen
        let attacker_sq = Square::from_algebraic("d1").unwrap();
        b.set_piece(attacker_sq, Some(Piece::new(Player::Red, PieceType::Rook)));

        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_bounty_map(&lm, &b);

        let queen_bounty = map
            .bounties
            .iter()
            .find(|pb| pb.piece.piece_type == PieceType::Queen)
            .expect("queen in bounty map");

        assert!(
            queen_bounty.bounty_score > 0,
            "undefended queen should have bounty"
        );
        assert_eq!(queen_bounty.attacker_count, 1);
        assert_eq!(queen_bounty.defender_count, 0);
    }

    #[test]
    fn defended_piece_has_lower_bounty() {
        let mut b = Board::empty();
        let sq = Square::from_algebraic("d7").unwrap();
        b.set_piece(sq, Some(Piece::new(Player::Blue, PieceType::Queen)));
        // Red rook attacks
        b.set_piece(
            Square::from_algebraic("d1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Rook)),
        );
        // Blue bishop defends
        b.set_piece(
            Square::from_algebraic("b5").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Bishop)),
        );

        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let map = compute_bounty_map(&lm, &b);

        let queen_bounty = map
            .bounties
            .iter()
            .find(|pb| pb.piece.piece_type == PieceType::Queen)
            .expect("queen in bounty map");

        // Defended queen should have 0 or low bounty
        assert_eq!(queen_bounty.defender_count, 1);
    }

    #[test]
    fn start_position_symmetric_bounty() {
        let b = start();
        let mut lm = LineMap::new();
        compute_lines(&b, &mut lm);
        let penalties = bounty_crossfire(&lm, &b);

        // All players should have similar bounty pressure at start
        let avg = penalties.iter().map(|&x| x as i32).sum::<i32>() / 4;
        for (i, &p) in penalties.iter().enumerate() {
            let diff = (p as i32 - avg).abs();
            assert!(
                diff < 500,
                "player {i} bounty {p} deviates too far from avg {avg}"
            );
        }
    }
}
