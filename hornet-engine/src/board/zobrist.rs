//! Zobrist hashing: a 64-bit position key for the transposition table and repetition
//! detection. Keys are XOR-combined; [`Board::make_move`] updates the hash incrementally
//! (XOR is its own inverse), so this is the one place where state *is* maintained
//! incrementally — Hard Rule #5 governs line projection, not the hash.
//!
//! Hashed: piece placement, side to move, castling rights, en-passant target, and the
//! per-player dead flag (it changes the legal-move set). **Points are not hashed** (they
//! are unbounded and don't affect move generation); revisit if the evaluator becomes
//! points-sensitive at TT boundaries.

use super::Board;
use super::types::{Piece, Player, Square, TOTAL_SQUARES};
use std::sync::LazyLock;

/// Number of `PieceType` variants (Pawn..PromotedQueen).
const PIECE_KINDS: usize = 7;

struct Keys {
    pieces: [[[u64; TOTAL_SQUARES]; PIECE_KINDS]; 4],
    side: [u64; 4],
    castle_kingside: [u64; 4],
    castle_queenside: [u64; 4],
    dead: [u64; 4],
    en_passant: [u64; TOTAL_SQUARES],
}

/// SplitMix64 — a small, fast PRNG used once to fill the (deterministic) key tables.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Keys {
    fn generate() -> Self {
        let mut s = 0x0123_4567_89AB_CDEFu64;
        let mut next = move || splitmix64(&mut s);

        let mut pieces = [[[0u64; TOTAL_SQUARES]; PIECE_KINDS]; 4];
        for plane in pieces.iter_mut() {
            for kind in plane.iter_mut() {
                for sq in kind.iter_mut() {
                    *sq = next();
                }
            }
        }
        let mut fill4 = || {
            let mut a = [0u64; 4];
            for x in a.iter_mut() {
                *x = next();
            }
            a
        };
        let side = fill4();
        let castle_kingside = fill4();
        let castle_queenside = fill4();
        let dead = fill4();
        let mut en_passant = [0u64; TOTAL_SQUARES];
        for x in en_passant.iter_mut() {
            *x = next();
        }

        Keys {
            pieces,
            side,
            castle_kingside,
            castle_queenside,
            dead,
            en_passant,
        }
    }
}

static KEYS: LazyLock<Keys> = LazyLock::new(Keys::generate);

#[inline]
pub(crate) fn key_piece(p: Piece, sq: Square) -> u64 {
    KEYS.pieces[p.player as usize][p.piece_type as usize][sq.index() as usize]
}
#[inline]
pub(crate) fn key_side(p: Player) -> u64 {
    KEYS.side[p as usize]
}
#[inline]
pub(crate) fn key_castle_kingside(p: Player) -> u64 {
    KEYS.castle_kingside[p as usize]
}
#[inline]
pub(crate) fn key_castle_queenside(p: Player) -> u64 {
    KEYS.castle_queenside[p as usize]
}
#[inline]
pub(crate) fn key_dead(p: Player) -> u64 {
    KEYS.dead[p as usize]
}
#[inline]
pub(crate) fn key_en_passant(sq: Square) -> u64 {
    KEYS.en_passant[sq.index() as usize]
}

/// Compute a board's Zobrist hash from scratch (used at construction and to verify the
/// incremental updates in tests).
pub fn hash(board: &Board) -> u64 {
    let mut h = 0u64;
    for (i, cell) in board.squares.iter().enumerate() {
        if let Some(p) = cell {
            h ^= KEYS.pieces[p.player as usize][p.piece_type as usize][i];
        }
    }
    h ^= KEYS.side[board.side_to_move as usize];
    for p in 0..4 {
        if board.castle_kingside[p] {
            h ^= KEYS.castle_kingside[p];
        }
        if board.castle_queenside[p] {
            h ^= KEYS.castle_queenside[p];
        }
        if board.dead[p] {
            h ^= KEYS.dead[p];
        }
    }
    if let Some(ep) = board.en_passant {
        h ^= KEYS.en_passant[ep.index() as usize];
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;

    #[test]
    fn start_position_hash_is_stable_and_nonzero() {
        let b = fen4::parse(fen4::START_FEN4).unwrap();
        assert_ne!(b.zobrist, 0);
        assert_eq!(b.zobrist, hash(&b), "stored hash matches recompute");
        // Reparsing yields the same hash.
        let b2 = fen4::parse(fen4::START_FEN4).unwrap();
        assert_eq!(b.zobrist, b2.zobrist);
    }

    #[test]
    fn distinct_positions_differ() {
        let start = fen4::parse(fen4::START_FEN4).unwrap();
        let mut moved = start.clone();
        let mv = crate::move_gen::generate_pseudo_legal(&moved)[0];
        moved.make_move(mv);
        assert_ne!(start.zobrist, moved.zobrist, "a move changes the hash");
    }

    #[test]
    fn incremental_matches_recompute_and_unmake_restores() {
        use crate::move_gen::generate_pseudo_legal;
        let start = fen4::parse(fen4::START_FEN4).unwrap();
        let mut b = start.clone();
        let mut undos = Vec::new();
        for _ in 0..6 {
            let mv = generate_pseudo_legal(&b)[0];
            undos.push(b.make_move(mv));
            assert_eq!(
                b.zobrist,
                hash(&b),
                "incremental hash diverged from recompute"
            );
        }
        while let Some(u) = undos.pop() {
            b.unmake_move(u);
            assert_eq!(b.zobrist, hash(&b), "unmake must restore the hash");
        }
        assert_eq!(b, start);
        assert_eq!(b.zobrist, start.zobrist);
    }

    #[test]
    fn zobrist_tracks_capture_and_elimination() {
        use crate::board::Board;
        use crate::board::types::{Piece, PieceType, Square};
        use crate::move_gen::generate_pseudo_legal;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        b.set_piece(at("g7"), Some(Piece::new(Player::Red, PieceType::Rook)));
        b.set_piece(at("g8"), Some(Piece::new(Player::Blue, PieceType::King)));
        b.recompute_zobrist();

        let mv = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.to == at("g8"))
            .unwrap();
        let u = b.make_move(mv);
        assert_eq!(
            b.zobrist,
            hash(&b),
            "king-capture (dead flag + capture) tracked"
        );
        b.unmake_move(u);
        assert_eq!(b.zobrist, hash(&b));
    }

    #[test]
    fn zobrist_tracks_en_passant_target() {
        use crate::move_gen::generate_pseudo_legal;
        let mut b = fen4::parse(fen4::START_FEN4).unwrap();
        // A double push arms the EP target (XOR in).
        let dp = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.flags.double_push)
            .unwrap();
        let u1 = b.make_move(dp);
        assert!(b.en_passant.is_some());
        assert_eq!(b.zobrist, hash(&b), "EP target armed");
        // The next move clears it (XOR out).
        let nxt = generate_pseudo_legal(&b)[0];
        let u2 = b.make_move(nxt);
        assert_eq!(b.zobrist, hash(&b), "EP target cleared");
        b.unmake_move(u2);
        b.unmake_move(u1);
        assert_eq!(b.zobrist, hash(&b));
    }
}
