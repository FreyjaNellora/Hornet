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
    /// Player whose double-push created the current `en_passant` target (needed to
    /// locate the capturable pawn). `None` when there is no en passant target.
    pub en_passant_pushing_player: Option<Player>,
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
            en_passant_pushing_player: None,
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

// ---------------------------------------------------------------------------
// Move geometry helpers (shared by attack detection and move generation)
// ---------------------------------------------------------------------------

/// Knight move offsets.
pub(crate) const KNIGHT_DELTAS: [(i8, i8); 8] = [
    (2, 1),
    (2, -1),
    (-2, 1),
    (-2, -1),
    (1, 2),
    (1, -2),
    (-1, 2),
    (-1, -2),
];
/// King / adjacency offsets.
pub(crate) const KING_DELTAS: [(i8, i8); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];
/// Diagonal ray directions (bishop / queen).
pub(crate) const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
/// Orthogonal ray directions (rook / queen).
pub(crate) const ROOK_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// A pawn's forward step `(d_rank, d_file)` for its player.
pub(crate) fn pawn_forward(player: Player) -> (i8, i8) {
    match player {
        Player::Red => (1, 0),
        Player::Blue => (0, 1),
        Player::Yellow => (-1, 0),
        Player::Green => (0, -1),
    }
}

/// A pawn's two diagonal capture offsets for its player.
pub(crate) fn pawn_capture_deltas(player: Player) -> [(i8, i8); 2] {
    match player {
        Player::Red => [(1, 1), (1, -1)],
        Player::Blue => [(1, 1), (-1, 1)],
        Player::Yellow => [(-1, 1), (-1, -1)],
        Player::Green => [(1, -1), (-1, -1)],
    }
}

/// Step a square by `(d_rank, d_file)`, returning `None` if it leaves the 14×14 grid.
/// Does not check corner validity — callers test [`Square::is_valid`] where needed.
pub(crate) fn offset(sq: Square, dr: i8, df: i8) -> Option<Square> {
    let r = sq.rank() as i8 + dr;
    let f = sq.file() as i8 + df;
    if (0..14).contains(&r) && (0..14).contains(&f) {
        Some(Square::from_rank_file(r as u8, f as u8))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Moves and make / unmake
// ---------------------------------------------------------------------------

/// Flags describing the kind of a move. Promotion is signalled by [`Move::promotion`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MoveFlags {
    pub capture: bool,
    pub double_push: bool,
    pub en_passant: bool,
    pub castle: bool,
}

/// A move. `promotion` holds the chosen target (`Knight`/`Bishop`/`Rook`/`Queen`); a
/// queen promotion lands on the board as `PromotedQueen`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceType>,
    pub flags: MoveFlags,
}

impl Move {
    /// A non-capturing, non-special move.
    pub fn quiet(from: Square, to: Square) -> Self {
        Move {
            from,
            to,
            promotion: None,
            flags: MoveFlags::default(),
        }
    }
}

/// Everything needed to reverse a [`Board::make_move`].
#[derive(Clone, Debug)]
pub struct UndoState {
    mv: Move,
    moved_piece: Piece,
    captured: Option<(Square, Piece)>,
    prev_castle_kingside: [bool; 4],
    prev_castle_queenside: [bool; 4],
    prev_en_passant: Option<Square>,
    prev_ep_pusher: Option<Player>,
    prev_side_to_move: Player,
    prev_dead: [bool; 4],
    prev_points: [u16; 4],
}

impl Board {
    /// Apply a move, returning the state needed to reverse it. Trusts `mv`'s flags
    /// (it must have been produced for the current `side_to_move`).
    pub fn make_move(&mut self, mv: Move) -> UndoState {
        let mover = self.side_to_move;
        let moved_piece = self
            .piece_at(mv.from)
            .expect("make_move: empty from-square");

        let mut undo = UndoState {
            mv,
            moved_piece,
            captured: None,
            prev_castle_kingside: self.castle_kingside,
            prev_castle_queenside: self.castle_queenside,
            prev_en_passant: self.en_passant,
            prev_ep_pusher: self.en_passant_pushing_player,
            prev_side_to_move: mover,
            prev_dead: self.dead,
            prev_points: self.points,
        };

        // Resolve the captured square (en passant differs from `to`).
        let cap_sq = if mv.flags.en_passant {
            let pusher = self
                .en_passant_pushing_player
                .expect("en passant move without a pushing player");
            let (dr, df) = pawn_forward(pusher);
            offset(mv.to, dr, df)
        } else if mv.flags.capture {
            Some(mv.to)
        } else {
            None
        };
        if let Some(csq) = cap_sq
            && let Some(cp) = self.piece_at(csq)
        {
            undo.captured = Some((csq, cp));
            self.points[mover.index()] += cp.piece_type.ffa_points() as u16;
            self.set_piece(csq, None);
            if cp.piece_type == PieceType::King {
                self.dead[cp.player.index()] = true;
            }
            self.clear_castle_right_if_rook_home(csq, cp);
        }

        // Move the piece (applying promotion).
        self.set_piece(mv.from, None);
        let placed = match mv.promotion {
            Some(PieceType::Queen) => Piece::new(mover, PieceType::PromotedQueen),
            Some(pt) => Piece::new(mover, pt),
            None => moved_piece,
        };
        self.set_piece(mv.to, Some(placed));

        // Castling moves the rook too.
        if mv.flags.castle {
            let (rfrom, rto) = castle_rook_move(mover, mv.to);
            let rook = self.piece_at(rfrom).expect("castle: rook missing");
            self.set_piece(rfrom, None);
            self.set_piece(rto, Some(rook));
        }

        // En passant target: cleared, then re-armed on a double push.
        self.en_passant = None;
        self.en_passant_pushing_player = None;
        if mv.flags.double_push {
            let (dr, df) = pawn_forward(mover);
            self.en_passant = offset(mv.from, dr, df);
            self.en_passant_pushing_player = Some(mover);
        }

        // Castling-right updates for king / rook moves.
        if moved_piece.piece_type == PieceType::King {
            self.castle_kingside[mover.index()] = false;
            self.castle_queenside[mover.index()] = false;
        } else if moved_piece.piece_type == PieceType::Rook {
            self.clear_castle_right_if_rook_home(mv.from, moved_piece);
        }

        self.side_to_move = self.next_live_player(mover);
        undo
    }

    /// Reverse a [`Board::make_move`].
    pub fn unmake_move(&mut self, undo: UndoState) {
        let mover = undo.prev_side_to_move;
        let mv = undo.mv;

        if mv.flags.castle {
            let (rfrom, rto) = castle_rook_move(mover, mv.to);
            let rook = self.piece_at(rto).expect("unmake castle: rook missing");
            self.set_piece(rto, None);
            self.set_piece(rfrom, Some(rook));
        }

        self.set_piece(mv.to, None);
        self.set_piece(mv.from, Some(undo.moved_piece));
        if let Some((csq, cp)) = undo.captured {
            self.set_piece(csq, Some(cp));
        }

        self.castle_kingside = undo.prev_castle_kingside;
        self.castle_queenside = undo.prev_castle_queenside;
        self.en_passant = undo.prev_en_passant;
        self.en_passant_pushing_player = undo.prev_ep_pusher;
        self.points = undo.prev_points;
        self.dead = undo.prev_dead;
        self.side_to_move = undo.prev_side_to_move;
    }

    /// Next player in turn order who is not eliminated.
    fn next_live_player(&self, from: Player) -> Player {
        let mut p = from.next();
        for _ in 0..4 {
            if !self.dead[p.index()] {
                return p;
            }
            p = p.next();
        }
        from
    }

    fn clear_castle_right_if_rook_home(&mut self, sq: Square, piece: Piece) {
        if piece.piece_type != PieceType::Rook {
            return;
        }
        let (ks, qs) = castle_rook_homes(piece.player);
        if sq == ks {
            self.castle_kingside[piece.player.index()] = false;
        }
        if sq == qs {
            self.castle_queenside[piece.player.index()] = false;
        }
    }
}

/// (kingside-rook-home, queenside-rook-home) starting squares per player (§1.5).
fn castle_rook_homes(player: Player) -> (Square, Square) {
    let sq = |s: &str| Square::from_algebraic(s).expect("valid square");
    match player {
        Player::Red => (sq("k1"), sq("d1")),
        Player::Blue => (sq("a4"), sq("a11")),
        Player::Yellow => (sq("d14"), sq("k14")),
        Player::Green => (sq("n11"), sq("n4")),
    }
}

/// (rook_from, rook_to) for a castle, keyed by the king's destination square (§1.5).
fn castle_rook_move(player: Player, king_to: Square) -> (Square, Square) {
    let sq = |s: &str| Square::from_algebraic(s).expect("valid square");
    let kt = king_to.to_algebraic();
    let (rf, rt) = match (player, kt.as_str()) {
        (Player::Red, "j1") => ("k1", "i1"),
        (Player::Red, "f1") => ("d1", "g1"),
        (Player::Blue, "a5") => ("a4", "a6"),
        (Player::Blue, "a9") => ("a11", "a8"),
        (Player::Yellow, "e14") => ("d14", "f14"),
        (Player::Yellow, "i14") => ("k14", "h14"),
        (Player::Green, "n10") => ("n11", "n9"),
        (Player::Green, "n6") => ("n4", "n7"),
        _ => panic!("invalid castle destination {kt} for {player:?}"),
    };
    (sq(rf), sq(rt))
}
