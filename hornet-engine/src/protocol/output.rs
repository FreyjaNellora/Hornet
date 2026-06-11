//! Protocol output formatting. Phase 8.

use crate::board::Move;
use crate::board::types::PieceType;

/// Format a move as chess.com-style long algebraic `from-to` (+ `=D`/`=R`/`=B`/`=N` for a
/// promotion). Castling is emitted as the king's `from-to` — unambiguous, and a driver re-decodes it
/// (the move generator self-syncs the mover and matches the castle by destination).
pub fn format_move(mv: &Move) -> String {
    let mut s = format!("{}-{}", mv.from.to_algebraic(), mv.to.to_algebraic());
    if let Some(p) = mv.promotion {
        let c = match p {
            PieceType::Queen | PieceType::PromotedQueen => 'D',
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            PieceType::Pawn | PieceType::King => '?',
        };
        s.push('=');
        s.push(c);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;
    use crate::move_gen::generate_legal;

    #[test]
    fn formats_from_to() {
        let mut b = fen4::parse(fen4::START_FEN4).unwrap();
        let mv = generate_legal(&mut b)[0];
        let s = format_move(&mv);
        let parts: Vec<&str> = s.split('-').collect();
        assert_eq!(parts.len(), 2, "expected from-to, got {s}");
        assert_eq!(parts[0], mv.from.to_algebraic());
        assert_eq!(parts[1], mv.to.to_algebraic());
    }
}
