//! Board representation and native I/O (FEN4 / PGN4).
//!
//! Square indexing is `sq = rank * 14 + file`, `0..195`. Of the 196 cells, 160 are
//! valid; the four 3x3 corners are unplayable. See `types::Square::is_valid`.

pub mod attacks;
pub mod fen4;
pub mod pgn4;
pub mod types;
pub mod zobrist;

use self::types::{Piece, PieceType, Player, Square, TOTAL_SQUARES};

/// The game board: piece placement plus the state encoded by a FEN4 string.
///
/// This is the I/O-focused core. Derived structures used by later phases (piece
/// lists, cached king squares, zobrist hash, line maps) are **not** maintained here
/// yet — they are added when move generation (P2) and line projection (P3) need them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Board {
    /// 14x14 grid indexed by [`Square::index`]; `None` = empty *or* invalid corner.
    pub squares: [Option<Piece>; TOTAL_SQUARES],
    /// Whose turn it is (FEN4 field 1).
    pub side_to_move: Player,
    /// Eliminated/dead flag per player, RBYG order (FEN4 field 2).
    pub dead: [bool; 4],
    /// Kingside castling right per player (FEN4 field 3).
    pub castle_kingside: [bool; 4],
    /// Queenside castling right per player (FEN4 field 4).
    pub castle_queenside: [bool; 4],
    /// Score / points per player (FEN4 field 5).
    pub points: [u16; 4],
    /// FEN4 field 6 (the lone counter). Stored raw pending confirmation of its full
    /// grammar from a real mid-game chess.com FEN4 (it may encode the draw clock and/or
    /// en passant). Preserved verbatim so round-trips stay byte-exact.
    pub extra: String,
    /// En passant target, if known. Not yet extracted from FEN4 — see [`Board::extra`].
    pub en_passant: Option<Square>,
}

impl Board {
    /// An empty board: no pieces, Red to move, all rights cleared, `extra = "0"`.
    pub fn empty() -> Self {
        Board {
            squares: [None; TOTAL_SQUARES],
            side_to_move: Player::Red,
            dead: [false; 4],
            castle_kingside: [false; 4],
            castle_queenside: [false; 4],
            points: [0; 4],
            extra: "0".to_string(),
            en_passant: None,
        }
    }

    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq.index() as usize]
    }

    #[inline]
    pub fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.squares[sq.index() as usize] = piece;
    }

    /// Number of pieces a player currently has on the board.
    pub fn piece_count(&self, player: Player) -> usize {
        self.squares
            .iter()
            .filter(|c| matches!(c, Some(p) if p.player == player))
            .count()
    }

    /// Locate a player's king, scanning the board (no cached king square yet).
    pub fn king_square(&self, player: Player) -> Option<Square> {
        self.squares
            .iter()
            .enumerate()
            .find_map(|(i, cell)| match cell {
                Some(p) if p.player == player && p.piece_type == PieceType::King => {
                    Some(Square::new(i as u8))
                }
                _ => None,
            })
    }
}
