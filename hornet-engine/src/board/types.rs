//! Board types: [`Square`], [`Player`], [`PieceType`], [`Piece`].
//!
//! Square indexing is `sq = rank * 14 + file`, range `0..195` (a 14x14 grid). Of the
//! 196 cells, 160 are valid; the four 3x3 corners (SW/SE/NW/NE) are unplayable.
//!
//! Per Hard Rule #8 there are two distinct value systems and they must never be
//! conflated: [`PieceType::eval_value`] (centipawns, used for Mᵢ / SEE / move ordering)
//! and [`PieceType::ffa_points`] (chess.com free-for-all points, used for result tags).

use std::fmt;

/// Side length of the (square) board.
pub const BOARD_SIZE: u8 = 14;
/// Total cells including the unplayable corners.
pub const TOTAL_SQUARES: usize = 196;
/// Playable cells (`TOTAL_SQUARES` minus the four 3x3 corners).
pub const VALID_SQUARES: usize = 160;

/// A board square, stored as a flat index `0..195` (`rank * 14 + file`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(pub u8);

impl Square {
    /// Construct from a raw `0..195` index.
    #[inline]
    pub const fn new(index: u8) -> Self {
        Square(index)
    }

    /// Construct from `(rank, file)`, each in `0..14`.
    #[inline]
    pub fn from_rank_file(rank: u8, file: u8) -> Self {
        debug_assert!(rank < BOARD_SIZE && file < BOARD_SIZE);
        Square(rank * BOARD_SIZE + file)
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 / BOARD_SIZE
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 % BOARD_SIZE
    }

    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }

    /// True iff the square is playable — i.e. not inside one of the four 3x3 corners.
    // Kept in the spec's literal form (§1.1 / §2.1) for fidelity; clippy would prefer
    // `RangeInclusive::contains`, but the explicit comparison matches the spec verbatim.
    #[allow(clippy::manual_range_contains)]
    #[inline]
    pub fn is_valid(self) -> bool {
        let r = self.rank();
        let f = self.file();
        !((r < 3 || r > 10) && (f < 3 || f > 10))
    }

    /// Algebraic coordinate, e.g. `h1` (files `a..n`, ranks `1..14`).
    pub fn to_algebraic(self) -> String {
        let file = (b'a' + self.file()) as char;
        let rank = self.rank() as u16 + 1;
        format!("{file}{rank}")
    }

    /// Parse an algebraic coordinate like `h1` or `n14`. Returns `None` if malformed
    /// or out of the `0..14` range (does not check corner validity).
    pub fn from_algebraic(s: &str) -> Option<Square> {
        let mut chars = s.chars();
        let file_c = chars.next()?;
        if !file_c.is_ascii_lowercase() {
            return None;
        }
        let file = (file_c as u8).checked_sub(b'a')?;
        let rank_num: u16 = chars.as_str().parse().ok()?;
        if rank_num < 1 || rank_num > BOARD_SIZE as u16 || file >= BOARD_SIZE {
            return None;
        }
        Some(Square::from_rank_file((rank_num - 1) as u8, file))
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_algebraic())
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Square({}={})", self.0, self.to_algebraic())
    }
}

/// The four players, in turn order. `repr(u8)` so `as usize` gives the index.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Player {
    Red = 0,
    Blue = 1,
    Yellow = 2,
    Green = 3,
}

impl Player {
    /// Next player in turn order: Red → Blue → Yellow → Green → Red.
    #[inline]
    pub fn next(self) -> Self {
        match self {
            Self::Red => Self::Blue,
            Self::Blue => Self::Yellow,
            Self::Yellow => Self::Green,
            Self::Green => Self::Red,
        }
    }

    /// The three opponents, in turn order starting after `self`.
    pub fn opponents(self) -> [Player; 3] {
        let a = self.next();
        let b = a.next();
        let c = b.next();
        [a, b, c]
    }

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Uppercase turn code used in the FEN4 turn field: `R`/`B`/`Y`/`G`.
    pub fn to_char(self) -> char {
        match self {
            Self::Red => 'R',
            Self::Blue => 'B',
            Self::Yellow => 'Y',
            Self::Green => 'G',
        }
    }

    pub fn from_char(c: char) -> Option<Player> {
        match c {
            'R' => Some(Self::Red),
            'B' => Some(Self::Blue),
            'Y' => Some(Self::Yellow),
            'G' => Some(Self::Green),
            _ => None,
        }
    }

    /// Lowercase prefix used in FEN4/PGN4 piece tokens, e.g. the `y` in `yR`.
    pub fn to_prefix(self) -> char {
        self.to_char().to_ascii_lowercase()
    }

    pub fn from_prefix(c: char) -> Option<Player> {
        Player::from_char(c.to_ascii_uppercase())
    }

    /// All four players in turn order.
    pub const ALL: [Player; 4] = [Player::Red, Player::Blue, Player::Yellow, Player::Green];
}

/// Piece kinds. `PromotedQueen` is distinct from `Queen` (per spec v0.2 §1.4) even
/// though it shares Queen's values; the distinction is carried at runtime, not in FEN4.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
    #[inline]
    pub fn is_slider(self) -> bool {
        matches!(
            self,
            Self::Bishop | Self::Rook | Self::Queen | Self::PromotedQueen
        )
    }

    /// Centipawn evaluation value (Mᵢ, SEE, move ordering). Hard Rule #8.
    ///
    /// `BISHOP = 450` is `[PENDING CALIBRATION]` per spec v0.2 §2.3.
    pub fn eval_value(self) -> i16 {
        match self {
            Self::Pawn => 100,
            Self::Knight => 300,
            Self::Bishop => 450,
            Self::Rook => 500,
            Self::Queen => 900,
            Self::King => 0,
            Self::PromotedQueen => 900,
        }
    }

    /// chess.com free-for-all points (result tags only). Distinct from [`Self::eval_value`]
    /// — never conflate the two (Hard Rule #8).
    pub fn ffa_points(self) -> u8 {
        match self {
            Self::Pawn => 1,
            Self::Knight => 3,
            Self::Bishop => 3,
            Self::Rook => 5,
            Self::Queen => 9,
            Self::King => 20,
            Self::PromotedQueen => 9,
        }
    }

    /// Uppercase FEN4/PGN4 piece letter. `PromotedQueen` serializes as `Q` (FEN4 cannot
    /// represent the promoted/normal distinction).
    pub fn to_char(self) -> char {
        match self {
            Self::Pawn => 'P',
            Self::Knight => 'N',
            Self::Bishop => 'B',
            Self::Rook => 'R',
            Self::Queen | Self::PromotedQueen => 'Q',
            Self::King => 'K',
        }
    }

    /// Parse an uppercase piece letter. Never yields `PromotedQueen` (not encodable in FEN4).
    pub fn from_char(c: char) -> Option<PieceType> {
        match c {
            'P' => Some(Self::Pawn),
            'N' => Some(Self::Knight),
            'B' => Some(Self::Bishop),
            'R' => Some(Self::Rook),
            'Q' => Some(Self::Queen),
            'K' => Some(Self::King),
            _ => None,
        }
    }
}

/// A piece is its kind plus its owner. No unique id — lines are always recomputed
/// from scratch (Hard Rule #5), so two pieces are equal iff same type and same player.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub piece_type: PieceType,
    pub player: Player,
}

impl Piece {
    #[inline]
    pub fn new(player: Player, piece_type: PieceType) -> Self {
        Piece { piece_type, player }
    }

    /// FEN4 token: lowercase player prefix + uppercase piece letter, e.g. `yR`.
    pub fn to_token(self) -> String {
        format!("{}{}", self.player.to_prefix(), self.piece_type.to_char())
    }

    /// Parse a FEN4 piece token like `yR` / `bP`. Returns `None` if malformed.
    pub fn from_token(tok: &str) -> Option<Piece> {
        let mut chars = tok.chars();
        let player = Player::from_prefix(chars.next()?)?;
        let piece_type = PieceType::from_char(chars.next()?.to_ascii_uppercase())?;
        if chars.next().is_some() {
            return None; // trailing junk
        }
        Some(Piece::new(player, piece_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity_partitions_160_and_36() {
        let valid = (0u8..=195).filter(|&i| Square::new(i).is_valid()).count();
        let invalid = (0u8..=195).filter(|&i| !Square::new(i).is_valid()).count();
        assert_eq!(valid, VALID_SQUARES, "expected 160 valid squares");
        assert_eq!(
            invalid,
            TOTAL_SQUARES - VALID_SQUARES,
            "expected 36 invalid squares"
        );
        assert_eq!(valid + invalid, TOTAL_SQUARES);
    }

    #[test]
    fn corner_squares_are_invalid() {
        // One representative cell from each 3x3 corner, plus inner-corner cells.
        for (r, f) in [(0u8, 0u8), (0, 13), (13, 0), (13, 13), (2, 2), (11, 11)] {
            assert!(
                !Square::from_rank_file(r, f).is_valid(),
                "({r},{f}) should be invalid"
            );
        }
        // Cells just inside the cross arms are valid.
        for (r, f) in [(3u8, 0u8), (0, 3), (6, 6), (10, 13)] {
            assert!(
                Square::from_rank_file(r, f).is_valid(),
                "({r},{f}) should be valid"
            );
        }
    }

    #[test]
    fn king_squares_match_spec() {
        // Spec §1.3 / §7.1 starting king squares.
        assert_eq!(Square::from_rank_file(0, 7).index(), 7); // Red h1
        assert_eq!(Square::from_rank_file(6, 0).index(), 84); // Blue a7
        assert_eq!(Square::from_rank_file(13, 6).index(), 188); // Yellow g14
        assert_eq!(Square::from_rank_file(7, 13).index(), 111); // Green n8
        assert_eq!(Square::new(7).to_algebraic(), "h1");
        assert_eq!(Square::new(84).to_algebraic(), "a7");
        assert_eq!(Square::new(188).to_algebraic(), "g14");
        assert_eq!(Square::new(111).to_algebraic(), "n8");
    }

    #[test]
    fn algebraic_round_trips() {
        for i in 0u8..=195 {
            let sq = Square::new(i);
            let s = sq.to_algebraic();
            assert_eq!(Square::from_algebraic(&s), Some(sq), "round-trip {s}");
        }
        assert_eq!(Square::from_algebraic("o1"), None); // 'o' would be file 14 (out of range)
        assert_eq!(Square::from_algebraic("a0"), None);
        assert_eq!(Square::from_algebraic("a15"), None);
    }

    #[test]
    fn turn_order_cycles() {
        assert_eq!(Player::Red.next(), Player::Blue);
        assert_eq!(Player::Green.next(), Player::Red);
        assert_eq!(
            Player::Red.opponents(),
            [Player::Blue, Player::Yellow, Player::Green]
        );
        assert_eq!(
            Player::Yellow.opponents(),
            [Player::Green, Player::Red, Player::Blue]
        );
        for p in Player::ALL {
            assert_eq!(Player::from_char(p.to_char()), Some(p));
            assert_eq!(Player::from_prefix(p.to_prefix()), Some(p));
        }
    }

    #[test]
    fn value_systems_are_distinct() {
        // Hard Rule #8: eval (centipawns) vs FFA points are different scales.
        assert_eq!(PieceType::Queen.eval_value(), 900);
        assert_eq!(PieceType::Queen.ffa_points(), 9);
        assert_eq!(PieceType::King.eval_value(), 0);
        assert_eq!(PieceType::King.ffa_points(), 20);
        assert_eq!(PieceType::Bishop.eval_value(), 450);
        assert_eq!(PieceType::Bishop.ffa_points(), 3);
        // PromotedQueen shares Queen's values but is a distinct variant.
        assert_eq!(
            PieceType::PromotedQueen.eval_value(),
            PieceType::Queen.eval_value()
        );
        assert_ne!(PieceType::PromotedQueen, PieceType::Queen);
    }

    #[test]
    fn piece_token_round_trips() {
        let p = Piece::new(Player::Yellow, PieceType::Rook);
        assert_eq!(p.to_token(), "yR");
        assert_eq!(Piece::from_token("yR"), Some(p));
        assert_eq!(
            Piece::from_token("bP"),
            Some(Piece::new(Player::Blue, PieceType::Pawn))
        );
        assert_eq!(Piece::from_token("xQ"), None);
        assert_eq!(Piece::from_token("yRR"), None);
    }
}
