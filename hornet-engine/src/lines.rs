//! Per-piece line projection (spec §3). Hornet's foundational primitive: for every
//! piece, project its reach (rays for sliders, steps for knight/king, push + always-on
//! diagonals for pawns), recording empty squares, the first blocker (with an X-ray flag),
//! and the X-ray continuation past it. A per-square inverse index ("who reaches here")
//! is built alongside.
//!
//! **Always-recompute (Hard Rule #5):** there is no incremental update and no `piece_id`.
//! [`compute_lines`] fills a caller-owned, reusable [`LineMap`] from scratch each call;
//! hold one (boxed) buffer and recompute per node rather than allocating.

use crate::board::types::{Piece, PieceType, Player, Square, TOTAL_SQUARES};
use crate::board::{
    BISHOP_DIRS, Board, KING_DELTAS, KNIGHT_DELTAS, ROOK_DIRS, offset, pawn_capture_deltas,
    pawn_forward,
};

/// Max reach entries per piece (a centre queen reaches ~41; 104 is the spec ceiling).
pub const MAX_REACH_PER_PIECE: usize = 104;
/// Max pieces tracked (4 players × 32).
pub const MAX_PIECES_TOTAL: usize = 128;
/// Max distinct pieces recorded as reaching a single square.
pub const MAX_REACHERS_PER_SQUARE: usize = 24;

/// One square a piece reaches, with how far and what (if anything) blocks there.
#[derive(Clone, Copy, Debug)]
pub struct ReachEntry {
    pub square: Square,
    pub distance: u8,
    /// `Some` if a piece occupies this square (the ray's blocker); `None` if empty.
    pub first_occupant: Option<Piece>,
    /// True only on the *first* occupant of a slider ray — i.e. the ray X-rays past here.
    pub xray_continues: bool,
}

impl ReachEntry {
    fn empty(square: Square, distance: u8) -> Self {
        ReachEntry {
            square,
            distance,
            first_occupant: None,
            xray_continues: false,
        }
    }
    fn blocked(square: Square, distance: u8, occupant: Piece, xray: bool) -> Self {
        ReachEntry {
            square,
            distance,
            first_occupant: Some(occupant),
            xray_continues: xray,
        }
    }
}

impl Default for ReachEntry {
    fn default() -> Self {
        ReachEntry {
            square: Square::new(0),
            distance: 0,
            first_occupant: None,
            xray_continues: false,
        }
    }
}

/// All reach entries for one piece (flat array, all rays concatenated).
#[derive(Clone)]
pub struct PieceLines {
    pub player: Player,
    pub piece_type: PieceType,
    pub square: Square,
    pub reach: [ReachEntry; MAX_REACH_PER_PIECE],
    pub reach_count: usize,
}

impl PieceLines {
    fn new(player: Player, piece_type: PieceType, square: Square) -> Self {
        PieceLines {
            player,
            piece_type,
            square,
            reach: [ReachEntry::default(); MAX_REACH_PER_PIECE],
            reach_count: 0,
        }
    }

    fn push(&mut self, e: ReachEntry) {
        if self.reach_count < MAX_REACH_PER_PIECE {
            self.reach[self.reach_count] = e;
            self.reach_count += 1;
        }
    }

    /// The valid reach entries (`reach[0..reach_count]`).
    pub fn entries(&self) -> &[ReachEntry] {
        &self.reach[..self.reach_count]
    }
}

impl Default for PieceLines {
    fn default() -> Self {
        PieceLines::new(Player::Red, PieceType::Pawn, Square::new(0))
    }
}

/// Inverse index for one square: which pieces (by index into [`LineMap::pieces`]) reach it.
#[derive(Clone)]
pub struct SquareReachers {
    pub piece_indices: [u8; MAX_REACHERS_PER_SQUARE],
    pub distances: [u8; MAX_REACHERS_PER_SQUARE],
    pub count: u8,
}

impl Default for SquareReachers {
    fn default() -> Self {
        SquareReachers {
            piece_indices: [0; MAX_REACHERS_PER_SQUARE],
            distances: [0; MAX_REACHERS_PER_SQUARE],
            count: 0,
        }
    }
}

/// All per-piece lines plus the per-square inverse index. Large (~110 KB) — keep one
/// boxed buffer and reuse it via [`compute_lines`].
pub struct LineMap {
    pub pieces: [PieceLines; MAX_PIECES_TOTAL],
    pub piece_count: usize,
    pub square_reachers: [SquareReachers; TOTAL_SQUARES],
}

impl LineMap {
    pub fn new() -> Self {
        LineMap {
            pieces: std::array::from_fn(|_| PieceLines::default()),
            piece_count: 0,
            square_reachers: std::array::from_fn(|_| SquareReachers::default()),
        }
    }

    /// Inverse index at a square: which pieces reach it.
    pub fn reachers_at(&self, sq: Square) -> &SquareReachers {
        &self.square_reachers[sq.index() as usize]
    }

    fn reset(&mut self) {
        self.piece_count = 0;
        for sr in self.square_reachers.iter_mut() {
            sr.count = 0;
        }
    }

    fn add_to_index(&mut self, piece_index: u8, pl: &PieceLines) {
        for e in pl.entries() {
            let sr = &mut self.square_reachers[e.square.index() as usize];
            if (sr.count as usize) < MAX_REACHERS_PER_SQUARE {
                let n = sr.count as usize;
                sr.piece_indices[n] = piece_index;
                sr.distances[n] = e.distance;
                sr.count += 1;
            }
        }
    }
}

impl Default for LineMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Recompute all lines for `board` into `out` (cleared first). Pieces are visited in
/// turn order (Red, Blue, Yellow, Green), then by square index, so `LineMap::pieces`
/// indices are stable for a given board.
pub fn compute_lines(board: &Board, out: &mut LineMap) {
    out.reset();
    for player in Player::ALL {
        for i in 0..TOTAL_SQUARES as u8 {
            let sq = Square::new(i);
            match board.piece_at(sq) {
                Some(p) if p.player == player => {
                    if out.piece_count >= MAX_PIECES_TOTAL {
                        return;
                    }
                    let pl = project_piece(board, p, sq);
                    let idx = out.piece_count as u8;
                    out.add_to_index(idx, &pl);
                    out.pieces[out.piece_count] = pl;
                    out.piece_count += 1;
                }
                _ => {}
            }
        }
    }
}

fn project_piece(board: &Board, piece: Piece, sq: Square) -> PieceLines {
    let mut pl = PieceLines::new(piece.player, piece.piece_type, sq);
    match piece.piece_type {
        PieceType::Pawn => project_pawn(board, piece.player, sq, &mut pl),
        PieceType::Knight => project_steps(board, sq, &KNIGHT_DELTAS, &mut pl),
        PieceType::King => project_steps(board, sq, &KING_DELTAS, &mut pl),
        PieceType::Bishop => project_slider(board, sq, &BISHOP_DIRS, &mut pl),
        PieceType::Rook => project_slider(board, sq, &ROOK_DIRS, &mut pl),
        PieceType::Queen | PieceType::PromotedQueen => {
            project_slider(board, sq, &BISHOP_DIRS, &mut pl);
            project_slider(board, sq, &ROOK_DIRS, &mut pl);
        }
    }
    pl
}

fn project_slider(board: &Board, from: Square, dirs: &[(i8, i8)], pl: &mut PieceLines) {
    for &(dr, df) in dirs {
        let mut first_occupant = false;
        let mut cur = from;
        let mut dist = 0u8;
        loop {
            let Some(next) = offset(cur, dr, df) else {
                break;
            };
            if !next.is_valid() {
                break; // ray stops at the invalid corners
            }
            cur = next;
            dist += 1;
            match board.piece_at(cur) {
                None => pl.push(ReachEntry::empty(cur, dist)),
                Some(occ) if !first_occupant => {
                    pl.push(ReachEntry::blocked(cur, dist, occ, true));
                    first_occupant = true; // X-ray continues past the first blocker
                }
                Some(occ) => {
                    pl.push(ReachEntry::blocked(cur, dist, occ, false));
                    break; // a second occupant terminates the ray
                }
            }
        }
    }
}

fn project_steps(board: &Board, from: Square, deltas: &[(i8, i8)], pl: &mut PieceLines) {
    for &(dr, df) in deltas {
        let Some(to) = offset(from, dr, df) else {
            continue;
        };
        if !to.is_valid() {
            continue;
        }
        match board.piece_at(to) {
            None => pl.push(ReachEntry::empty(to, 1)),
            Some(occ) => pl.push(ReachEntry::blocked(to, 1, occ, false)),
        }
    }
}

fn project_pawn(board: &Board, player: Player, from: Square, pl: &mut PieceLines) {
    let (fdr, fdf) = pawn_forward(player);

    // Forward push is registered only when the square is empty (it is not an attack).
    if let Some(one) = offset(from, fdr, fdf)
        && one.is_valid()
        && board.piece_at(one).is_none()
    {
        pl.push(ReachEntry::empty(one, 1));
        if on_start_rank(player, from)
            && let Some(two) = offset(one, fdr, fdf)
            && two.is_valid()
            && board.piece_at(two).is_none()
        {
            pl.push(ReachEntry::empty(two, 2));
        }
    }

    // Diagonal capture squares are ALWAYS registered (the attack zone is geometric);
    // the query engine decides whether each is a capture, a defence, or an empty threat.
    for (cdr, cdf) in pawn_capture_deltas(player) {
        let Some(to) = offset(from, cdr, cdf) else {
            continue;
        };
        if !to.is_valid() {
            continue;
        }
        match board.piece_at(to) {
            None => pl.push(ReachEntry::empty(to, 1)),
            Some(occ) => pl.push(ReachEntry::blocked(to, 1, occ, false)),
        }
    }
}

/// A pawn's starting rank/file (where the double push is available). Mirrors the same
/// predicate in `move_gen` — keep them in sync.
fn on_start_rank(player: Player, sq: Square) -> bool {
    match player {
        Player::Red => sq.rank() == 1,
        Player::Blue => sq.file() == 1,
        Player::Yellow => sq.rank() == 12,
        Player::Green => sq.file() == 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Square {
        Square::from_algebraic(s).unwrap()
    }

    /// Project a single piece on an otherwise-`others`-populated empty board; return its
    /// `PieceLines` (always index 0 here) inside the computed map.
    fn lines_for(piece: Piece, sq: Square, others: &[(Square, Piece)]) -> Box<LineMap> {
        let mut b = Board::empty();
        b.set_piece(sq, Some(piece));
        for &(s, p) in others {
            b.set_piece(s, Some(p));
        }
        let mut lm = Box::new(LineMap::new());
        compute_lines(&b, &mut lm);
        lm
    }

    fn reach_count(piece: Piece, sq: Square) -> usize {
        let lm = lines_for(piece, sq, &[]);
        assert_eq!(lm.piece_count, 1);
        lm.pieces[0].reach_count
    }

    #[test]
    fn slider_reach_counts_match_spec_7_2() {
        // Spec §7.2 reference counts on an empty board.
        assert_eq!(
            reach_count(Piece::new(Player::Red, PieceType::Rook), at("g7")),
            26
        );
        assert_eq!(
            reach_count(Piece::new(Player::Red, PieceType::Bishop), at("g7")),
            15
        );
        assert_eq!(
            reach_count(Piece::new(Player::Red, PieceType::Queen), at("g7")),
            41
        );
        assert_eq!(
            reach_count(Piece::new(Player::Red, PieceType::Knight), at("g7")),
            8
        );
        assert_eq!(
            reach_count(Piece::new(Player::Red, PieceType::King), at("d1")),
            3
        );
    }

    #[test]
    fn pawn_reach_matches_spec_7_2() {
        // Red pawn d2: forward d3, d4 (start-rank double), diagonal e3; c3 is an invalid corner.
        let lm = lines_for(Piece::new(Player::Red, PieceType::Pawn), at("d2"), &[]);
        let pl = &lm.pieces[0];
        assert_eq!(pl.reach_count, 3);
        let squares: Vec<Square> = pl.entries().iter().map(|e| e.square).collect();
        assert!(squares.contains(&at("d3")));
        assert!(squares.contains(&at("d4")));
        assert!(squares.contains(&at("e3")));
    }

    #[test]
    fn rook_xrays_through_first_blocker() {
        // Rook g7, friendly pawn on g9: g9 is the first occupant (xray continues), g10 is
        // an X-ray empty entry beyond it.
        let lm = lines_for(
            Piece::new(Player::Red, PieceType::Rook),
            at("g7"),
            &[(at("g9"), Piece::new(Player::Red, PieceType::Pawn))],
        );
        let pl = &lm.pieces[0];
        let g9 = pl.entries().iter().find(|e| e.square == at("g9")).unwrap();
        assert!(g9.first_occupant.is_some());
        assert!(g9.xray_continues, "first blocker on a slider ray X-rays");
        let g10 = pl.entries().iter().find(|e| e.square == at("g10")).unwrap();
        assert!(
            g10.first_occupant.is_none(),
            "X-ray reaches g10 past the friendly pawn"
        );
        // The ray does not continue past a second occupant.
        assert!(pl.entries().iter().all(|e| e.square != at("g7")));
    }

    #[test]
    fn inverse_index_records_reachers() {
        let lm = lines_for(Piece::new(Player::Red, PieceType::Rook), at("g7"), &[]);
        let sr = lm.reachers_at(at("g8")); // one square north of the rook
        assert_eq!(sr.count, 1);
        assert_eq!(sr.piece_indices[0], 0);
        assert_eq!(sr.distances[0], 1);
        // A square the rook cannot reach has no reachers.
        assert_eq!(lm.reachers_at(at("a1")).count, 0);
    }

    #[test]
    fn start_position_projects_all_64_pieces() {
        use crate::board::fen4;
        let b = fen4::parse(fen4::START_FEN4).unwrap();
        let mut lm = Box::new(LineMap::new());
        compute_lines(&b, &mut lm);
        assert_eq!(lm.piece_count, 64, "16 pieces × 4 players");
    }
}
