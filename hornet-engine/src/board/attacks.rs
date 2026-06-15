//! Attack detection: is a square attacked by a given player? Used for check
//! detection (move legality), castling legality, and king safety.

use super::types::{PieceType, Player, Square};
use super::{
    BISHOP_DIRS, Board, KING_DELTAS, KNIGHT_DELTAS, ROOK_DIRS, offset, pawn_capture_deltas,
};

/// True if any piece belonging to `by` attacks `sq`.
///
/// Pawn attacks are geometric (the square is attacked whether or not it is occupied).
/// Slider rays stop at the first piece and at the invalid corners.
pub fn is_attacked_by(board: &Board, sq: Square, by: Player) -> bool {
    // Pawns: a `by` pawn attacks `sq` if it sits one capture-step back from `sq`.
    for (dr, df) in pawn_capture_deltas(by) {
        if let Some(p) = offset(sq, -dr, -df).and_then(|s| board.piece_at(s))
            && p.player == by
            && p.piece_type == PieceType::Pawn
        {
            return true;
        }
    }
    // Knights.
    for (dr, df) in KNIGHT_DELTAS {
        if let Some(p) = offset(sq, dr, df).and_then(|s| board.piece_at(s))
            && p.player == by
            && p.piece_type == PieceType::Knight
        {
            return true;
        }
    }
    // King.
    for (dr, df) in KING_DELTAS {
        if let Some(p) = offset(sq, dr, df).and_then(|s| board.piece_at(s))
            && p.player == by
            && p.piece_type == PieceType::King
        {
            return true;
        }
    }
    // Diagonal sliders (bishop / queen).
    for (dr, df) in BISHOP_DIRS {
        if ray_attacks(board, sq, dr, df, by, true) {
            return true;
        }
    }
    // Orthogonal sliders (rook / queen).
    for (dr, df) in ROOK_DIRS {
        if ray_attacks(board, sq, dr, df, by, false) {
            return true;
        }
    }
    false
}

fn ray_attacks(board: &Board, from: Square, dr: i8, df: i8, by: Player, diagonal: bool) -> bool {
    let mut cur = from;
    loop {
        match offset(cur, dr, df) {
            None => return false,
            Some(next) => {
                if !next.is_valid() {
                    return false; // ray stops at the invalid corners
                }
                cur = next;
                if let Some(p) = board.piece_at(cur) {
                    // The first piece blocks the ray; it attacks iff it's a matching slider.
                    return p.player == by
                        && if diagonal {
                            matches!(
                                p.piece_type,
                                PieceType::Bishop | PieceType::Queen | PieceType::PromotedQueen
                            )
                        } else {
                            matches!(
                                p.piece_type,
                                PieceType::Rook | PieceType::Queen | PieceType::PromotedQueen
                            )
                        };
                }
            }
        }
    }
}
