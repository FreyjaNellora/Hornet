//! Game-flow driver: plays a 4PC game ply-by-ply, handling the full Dead-King-Walking lifecycle
//! (§1.7/1.8) that `make_move` alone does not — the checkmate/stalemate → DKW transition, the dead
//! king's random walk, DKW-king-stalemate scoring, and game end. Used by self-play and any external
//! play loop so the DKW rules live in one place.

use crate::board::types::Player;
use crate::board::{Board, Move, fen4};
use crate::move_gen::{generate_legal, in_check};

/// What advancing the game by one turn did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    /// A normal move, or a DKW king's random walk, was played (carries the move, for recording).
    Moved(Move),
    /// A turn was passed with no move (defensive: a chooser declined). Not a real 4PC event.
    Passed,
    /// The side to move had no legal move and is now Dead-King-Walking (checkmate, or stalemate +20).
    EnteredDkw(Player),
    /// A DKW king was stalemated and removed — that player is now fully out.
    Removed(Player),
}

/// A 4PC game in progress, with a small PRNG for the random dead-king walks (§1.7).
pub struct Game {
    pub board: Board,
    rng: u64,
}

impl Game {
    /// A game from the canonical 4PC start. `seed` drives the random DKW king walks (reproducible).
    pub fn from_start(seed: u64) -> Self {
        Self::new(
            fen4::parse(fen4::START_FEN4).expect("start FEN4 parses"),
            seed,
        )
    }

    /// A game from an arbitrary position.
    pub fn new(board: Board, seed: u64) -> Self {
        Game {
            board,
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// xorshift64* — deterministic per seed; used only to pick a random DKW king move.
    fn next_rng(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Players still in the game (not fully `dead` — LIVE or DKW). The game is over at `<= 1`.
    pub fn active_count(&self) -> usize {
        Player::ALL
            .iter()
            .filter(|&&p| !self.board.dead[p.index()])
            .count()
    }

    /// Final/current placement points per player (RBYG order).
    pub fn points(&self) -> [u16; 4] {
        self.board.points
    }

    /// Advance one turn for the current side. `choose` picks a LIVE player's move (e.g. a search);
    /// the driver handles the DKW king's random walk and the checkmate/stalemate transitions +
    /// scoring itself, so callers never special-case DKW.
    pub fn step(&mut self, choose: impl FnOnce(&mut Board) -> Option<Move>) -> TurnOutcome {
        let p = self.board.side_to_move;
        let legal = generate_legal(&mut self.board);

        if self.board.is_dkw(p) {
            if legal.is_empty() {
                // DKW-king stalemate (§1.8): +10 to each remaining LIVE player, then remove the king.
                for q in Player::ALL {
                    let j = q.index();
                    if q != p && !self.board.dead[j] && !self.board.dkw[j] {
                        self.board.points[j] += 10;
                    }
                }
                self.board.eliminate_player(p); // remove all its pieces (§1.7)
                self.board.make_null(); // pass; rotation now skips the removed player
                return TurnOutcome::Removed(p);
            }
            let pick = (self.next_rng() % legal.len() as u64) as usize;
            let mv = legal[pick];
            self.apply(mv); // a DKW king may capture an enemy king → eliminate + sweep
            return TurnOutcome::Moved(mv);
        }

        // LIVE player.
        if legal.is_empty() {
            // Checkmate (in check) or stalemate (not). Stalemate → +20 consolation (§1.8). Either way
            // the player becomes Dead-King-Walking; its king walks on its next turn.
            if !in_check(&self.board, p) {
                self.board.points[p.index()] += 20;
            }
            self.board.enter_dkw(p);
            self.board.make_null(); // the mated player does not move this turn; pass
            return TurnOutcome::EnteredDkw(p);
        }

        match choose(&mut self.board) {
            Some(mv) => {
                self.apply(mv);
                TurnOutcome::Moved(mv)
            }
            None => {
                self.board.make_null(); // defensive: a chooser that declines passes the turn
                TurnOutcome::Passed
            }
        }
    }

    /// Make a move, then enforce §1.7 at the game level: any player this move fully eliminated (king
    /// captured) has its remaining pieces swept off the board. (The search's `make_move` deliberately
    /// leaves them on, to avoid over-valuing king-captures; the *game* board removes them.)
    fn apply(&mut self, mv: Move) {
        let dead_before = self.board.dead;
        self.board.make_move(mv);
        for q in Player::ALL {
            let j = q.index();
            if self.board.dead[j] && !dead_before[j] {
                self.board.eliminate_player(q);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game from the start, choosing the first legal move each turn, terminates (one survivor or
    /// the ply cap) and the DKW lifecycle keeps the board consistent.
    #[test]
    fn game_runs_to_completion_with_first_move_policy() {
        let mut game = Game::from_start(1);
        let mut plies = 0;
        while game.active_count() > 1 && plies < 400 {
            game.step(|b| generate_legal(b).into_iter().next());
            plies += 1;
        }
        // Either it reduced to one survivor, or it hit the cap — both are valid terminations.
        assert!(game.active_count() >= 1);
        // Zobrist stays consistent with a from-scratch recompute after the whole lifecycle.
        let mut check = game.board.clone();
        check.recompute_zobrist();
        assert_eq!(
            game.board.zobrist, check.zobrist,
            "hash stayed in sync across DKW lifecycle"
        );
    }

    #[test]
    fn eliminate_player_sweeps_all_their_pieces() {
        use crate::board::Square;
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        b.set_piece(at("h1"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("h2"), Some(Piece::new(Player::Red, PieceType::Pawn)));
        b.set_piece(at("a1"), Some(Piece::new(Player::Red, PieceType::Rook)));
        b.set_piece(at("g7"), Some(Piece::new(Player::Blue, PieceType::Queen)));
        b.recompute_zobrist();

        b.eliminate_player(Player::Red); // §1.7: fully eliminated → all pieces removed

        assert!(b.dead[Player::Red.index()]);
        assert_eq!(
            b.piece_count(Player::Red),
            0,
            "every Red piece is swept off the board"
        );
        assert_eq!(
            b.piece_count(Player::Blue),
            1,
            "other players are untouched"
        );
        let mut recomputed = b.clone();
        recomputed.recompute_zobrist();
        assert_eq!(
            b.zobrist, recomputed.zobrist,
            "hash stays in sync after the sweep"
        );
    }
}
