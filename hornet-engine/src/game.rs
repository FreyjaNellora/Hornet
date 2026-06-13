//! Game-flow driver: plays a 4PC game ply-by-ply, handling the full Dead-King-Walking lifecycle
//! (§1.7/1.8) that `make_move` alone does not — the checkmate/stalemate → DKW transition, the dead
//! king's random walk, DKW-king-stalemate scoring, and game end. Used by self-play and any external
//! play loop so the DKW rules live in one place.

use crate::board::types::{PieceType, Player};
use crate::board::{Board, Move, fen4};
use crate::move_gen::{generate_legal, in_check};

/// 50-move-rule threshold, in plies. 200 = 50 full 4-player rounds.
///
/// **NOTE (EXP-034):** the chess.com Help Center states the 50-move rule draws the game (+10 to
/// each remaining player) but does NOT state the counting unit (per ply vs per full round).
/// `Board::extra` (FEN4 field 6) is "the lone counter" and *may* hold their clock, but we have no
/// mid-game FEN4 to confirm its grammar. 200 plies is the **conservative** choice: it never draws
/// a still-progressing game early, and threefold repetition catches genuine shuffles far sooner.
/// Tune once verified against live play.
const FIFTY_MOVE_PLIES: u32 = 200;

/// Why a game ended in a draw under the FFA draw rules. Each awards **+10 to every player still
/// in the game** (alive or zombie/DKW) — chess.com Help Center: "insufficient material, threefold
/// repetition, or the 50-move rule → remaining players receive +10 points each."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawReason {
    /// The current position has occurred three times (squares, side-to-move, rights, dead/DKW
    /// flags all equal — i.e. the same Zobrist key, which excludes points).
    Repetition,
    /// No capture or pawn move for [`FIFTY_MOVE_PLIES`] plies.
    FiftyMove,
}

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
    /// Zobrist key after every turn (including the start position) — the threefold-repetition
    /// log. A position drawn when its key appears three times. (EXP-034.)
    history: Vec<u64>,
    /// Plies since the last capture or pawn move — the 50-move-rule clock. (EXP-034.)
    halfmove_clock: u32,
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
        let start = board.zobrist;
        Game {
            board,
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
            history: vec![start],
            halfmove_clock: 0,
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
                if crate::board::dkw_rule() == 0 {
                    self.board.eliminate_player(p); // rule 0: remove all its pieces
                } else {
                    self.board.retire_king(p); // rules 1/2 (EXP-026): the army stays on the board
                }
                self.board.make_null(); // pass; rotation now skips the removed player
                self.record(false);
                return TurnOutcome::Removed(p);
            }
            let pick = (self.next_rng() % legal.len() as u64) as usize;
            let mv = legal[pick];
            let reset = mv.flags.capture; // a DKW king is never a pawn
            self.apply(mv); // a DKW king may capture an enemy king → eliminate + sweep
            self.record(reset);
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
            self.record(false);
            return TurnOutcome::EnteredDkw(p);
        }

        match choose(&mut self.board) {
            Some(mv) => {
                // 50-move clock resets on a capture or any pawn move (read before the move applies).
                let reset = mv.flags.capture
                    || self
                        .board
                        .piece_at(mv.from)
                        .is_some_and(|pc| pc.piece_type == PieceType::Pawn);
                self.apply(mv);
                self.record(reset);
                TurnOutcome::Moved(mv)
            }
            None => {
                self.board.make_null(); // defensive: a chooser that declines passes the turn
                self.record(false);
                TurnOutcome::Passed
            }
        }
    }

    /// Record the post-turn position for the draw rules: advance (or reset) the 50-move clock and
    /// log the new Zobrist key for repetition counting.
    fn record(&mut self, reset_clock: bool) {
        self.halfmove_clock = if reset_clock { 0 } else { self.halfmove_clock + 1 };
        self.history.push(self.board.zobrist);
    }

    /// Whether the game has reached a drawing condition (pure query — does not mutate or score;
    /// see [`Game::claim_draw`]). Drivers that want FFA draw rules call this after each turn.
    pub fn draw_status(&self) -> Option<DrawReason> {
        let cur = self.board.zobrist;
        if self.history.iter().filter(|&&h| h == cur).count() >= 3 {
            return Some(DrawReason::Repetition);
        }
        if self.halfmove_clock >= FIFTY_MOVE_PLIES {
            return Some(DrawReason::FiftyMove);
        }
        None
    }

    /// Plies since the last capture or pawn move (the 50-move clock). For inspection/tests.
    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    /// If a draw condition is met, award **+10 to every player still in the game** (alive or
    /// zombie/DKW, per the Help Center) and return the reason; otherwise `None`. Call once when
    /// [`Game::draw_status`] first fires, then end the game — re-calling re-awards points.
    pub fn claim_draw(&mut self) -> Option<DrawReason> {
        let reason = self.draw_status()?;
        for q in Player::ALL {
            let j = q.index();
            if !self.board.dead[j] {
                self.board.points[j] += 10;
            }
        }
        Some(reason)
    }

    /// Make a move, then apply the game-level death rule. **Rule 0** (pre-EXP-026): any player
    /// this move fully eliminated (king captured) has its remaining pieces swept off the board.
    /// **Rules 1/2** (EXP-026): the dead army stays on the board — `make_move` already set the
    /// `dead` flag, so nothing further is needed, and search and game flow finally agree (the
    /// EXP-011 search/game inconsistency disappears: no sweep exists anywhere).
    fn apply(&mut self, mv: Move) {
        let dead_before = self.board.dead;
        self.board.make_move(mv);
        if crate::board::dkw_rule() == 0 {
            for q in Player::ALL {
                let j = q.index();
                if self.board.dead[j] && !dead_before[j] {
                    self.board.eliminate_player(q);
                }
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

    /// Four kings, far apart, each toggling between two empty squares. The start position recurs
    /// every 8 plies, so its third occurrence — and a threefold-repetition draw — lands at ply 16.
    #[test]
    fn threefold_repetition_detected_and_scored() {
        use crate::board::Square;
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty(); // empty() clears all castle rights → king moves don't alter the hash
        b.set_piece(at("h1"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("a6"), Some(Piece::new(Player::Blue, PieceType::King)));
        b.set_piece(at("g14"), Some(Piece::new(Player::Yellow, PieceType::King)));
        b.set_piece(at("n7"), Some(Piece::new(Player::Green, PieceType::King)));
        b.recompute_zobrist();
        let mut game = Game::new(b, 1);
        // Each player's king toggles between (home, away); none is ever adjacent to another.
        let toggle = [("h1", "i1"), ("a6", "a7"), ("g14", "h14"), ("n7", "n6")];

        let mut fired_at = None;
        for i in 1..=16 {
            assert!(game.draw_status().is_none(), "drew early at ply {}", i - 1);
            let (x, y) = toggle[game.board.side_to_move.index()];
            game.step(|bd| {
                let from = if bd.piece_at(at(x)).is_some_and(|pc| pc.player == bd.side_to_move) {
                    at(x)
                } else {
                    at(y)
                };
                let to = if from == at(x) { at(y) } else { at(x) };
                generate_legal(bd).into_iter().find(|m| m.from == from && m.to == to)
            });
            if game.draw_status().is_some() {
                fired_at = Some(i);
                break;
            }
        }
        assert_eq!(fired_at, Some(16), "threefold should land at ply 16");
        assert_eq!(game.draw_status(), Some(DrawReason::Repetition));

        // claim_draw awards +10 to each player still in the game (none dead here).
        let before = game.board.points;
        assert_eq!(game.claim_draw(), Some(DrawReason::Repetition));
        for i in 0..4 {
            assert_eq!(game.board.points[i], before[i] + 10);
        }
    }

    /// The 50-move clock advances on quiet moves and resets on a pawn move or a capture.
    #[test]
    fn halfmove_clock_resets_on_pawn_and_capture() {
        use crate::board::Square;
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        // Red: king + a pawn that can push, and a knight that can capture Blue's pawn.
        b.set_piece(at("h1"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("h2"), Some(Piece::new(Player::Red, PieceType::Pawn)));
        b.set_piece(at("e4"), Some(Piece::new(Player::Red, PieceType::Knight)));
        b.set_piece(at("d6"), Some(Piece::new(Player::Blue, PieceType::Pawn)));
        // Other kings so the game is well-formed and nobody is in check.
        b.set_piece(at("a6"), Some(Piece::new(Player::Blue, PieceType::King)));
        b.set_piece(at("g14"), Some(Piece::new(Player::Yellow, PieceType::King)));
        b.set_piece(at("n7"), Some(Piece::new(Player::Green, PieceType::King)));
        b.recompute_zobrist();
        let mut game = Game::new(b, 1);

        // A helper that plays a chosen from→to for the side to move.
        let mut play = |game: &mut Game, from: &str, to: &str| {
            let (f, t) = (at(from), at(to));
            game.step(|bd| generate_legal(bd).into_iter().find(|m| m.from == f && m.to == t));
        };

        play(&mut game, "h1", "i1"); // Red quiet king move → clock 1
        assert_eq!(game.halfmove_clock(), 1);
        play(&mut game, "a6", "a7"); // Blue quiet → clock 2
        assert_eq!(game.halfmove_clock(), 2);
        play(&mut game, "g14", "h14"); // Yellow quiet → 3
        play(&mut game, "n7", "n6"); // Green quiet → 4
        assert_eq!(game.halfmove_clock(), 4);
        play(&mut game, "h2", "h3"); // Red PAWN push → reset to 0
        assert_eq!(game.halfmove_clock(), 0, "pawn move resets the clock");
        play(&mut game, "a7", "a6"); // Blue quiet → 1
        play(&mut game, "h14", "g14"); // Yellow quiet → 2
        play(&mut game, "n6", "n7"); // Green quiet → 3
        play(&mut game, "e4", "d6"); // Red knight CAPTURES Blue pawn → reset to 0
        assert_eq!(game.halfmove_clock(), 0, "capture resets the clock");
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
