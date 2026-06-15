//! PGN4 game replay: resolve PGN4 ply tokens against the move generator, with the fidelity
//! inferences from EXP-028. **The one shared implementation** — the replay test and the tooling
//! use these, so a fidelity fix lands everywhere at once.
//!
//! Design (mirrors the original `tests/pgn4_replay.rs` logic):
//! - **Self-syncing:** each normal ply's mover is read from the piece on its `from` square, not
//!   from turn rotation, so replay survives eliminations/DKW skips.
//! - **Pseudo-legal matching:** a real move is always pseudo-legal; this isolates move geometry
//!   from the legality filter (and tolerates DKW kings stepping into check).
//! - **DKW inference (EXP-028):** a live king can never capture its own piece, so a replayed king
//!   landing on its own piece *proves* the mover is Dead-King-Walking — set the flag and retry.
//! - **Rotation-aware castles (EXP-028):** a castle token names no player; resolving in fixed
//!   RBYG order misattributes it whenever two players can castle the same side. Resolve in
//!   rotation order from the expected next mover (`last_mover.next()`).

use crate::board::pgn4::{self, DecodedMove};
use crate::board::types::{PieceType, Player};
use crate::board::{Board, Move};
use crate::move_gen::{castle_king_destination, generate_pseudo_legal};

/// Per-game replay state (currently just the rotation tracker for castle resolution).
#[derive(Default)]
pub struct ReplayState {
    pub last_mover: Option<Player>,
}

/// Resolve a ply token to a concrete move **without applying it**. Self-syncs `side_to_move`
/// (and possibly the `dkw` flag — see module docs), so the board is set up for that player to
/// move afterwards. Returns `None` for undecodable tokens (e.g. "R"/"S" markers) and for moves
/// the generator cannot produce.
pub fn resolve_ply(board: &mut Board, token: &str, st: &mut ReplayState) -> Option<Move> {
    match pgn4::decode_ply(token)? {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let p = board.piece_at(from)?;
            board.side_to_move = p.player;
            let mut found = generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion);
            if found.is_none()
                && p.piece_type == PieceType::King
                && board.piece_at(to).is_some_and(|t| t.player == p.player)
            {
                // DKW inference (see module docs).
                board.enter_dkw(p.player);
                found = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.from == from && m.to == to && m.promotion == promotion);
            }
            if found.is_some() {
                st.last_mover = Some(p.player);
            }
            found
        }
        DecodedMove::Castle { kingside } => {
            let start = st.last_mover.map_or(Player::Red, |p| p.next());
            let mut pl = start;
            for _ in 0..4 {
                board.side_to_move = pl;
                let dest = castle_king_destination(pl, kingside);
                if let Some(m) = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.flags.castle && m.to == dest)
                {
                    st.last_mover = Some(pl);
                    return Some(m);
                }
                pl = pl.next();
            }
            None
        }
    }
}

/// Resolve + apply one ply. Returns `false` if the token can't be decoded or matched.
pub fn apply_ply(board: &mut Board, token: &str, st: &mut ReplayState) -> bool {
    match resolve_ply(board, token, st) {
        Some(m) => {
            board.make_move(m);
            true
        }
        None => false,
    }
}
