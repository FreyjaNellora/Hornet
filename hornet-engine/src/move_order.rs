//! Move ordering (spec §6.2). Baseline: the transposition-table move first, then captures
//! by MVV-LVA (most-valuable victim, least-valuable attacker), then quiet moves. Killers and
//! a history heuristic are later additions.
//!
//! FFA bounty scoring (PITCH-ffa-bounty-scoring): captures are scored by BOTH `eval_value`
//! (centipawns, positional/SEE dimension) and `ffa_points` (victory points, objective dimension).
//! Hard Rule #8: the two value systems are distinct and never conflated.
//!
//! "Free capture" bonus: any capture where the victim has zero defenders gets a massive bonus,
//! even if it's "just" a pawn. In FFA, a free pawn is +1 point with zero risk — devastating
//! in mid/late game when pawn counts matter for promotion races.
//!
//! Both bounty and free-capture are **default-off ordering levers** on [`OrderState`] (Hard Rule
//! #6: in a beam search ordering *is* selection, so they are strength-affecting). Measured in
//! EXP-020; enable via `Searcher::with_ffa_bounty_order` / `with_free_capture_order`. They only
//! affect the maxn path (`search`/`root_move_values`/`search_depth`) — `search_flashlight` never
//! orders moves.

use crate::board::types::{PieceType, Player};
use crate::board::{Board, Move};

/// Maximum ply with dedicated killer slots (search depths stay small; deeper plies skip killers).
const MAX_PLY: usize = 128;
/// Killer-move bonus — ranked below every capture, above history-scored quiets.
const KILLER_BONUS: i32 = 9_000;

/// Per-searcher move-ordering state: two killer moves per ply plus a `[from][to]` history table,
/// created once per `Searcher` and carried across searches / iterative-deepening iterations.
/// Also carries the two ordering levers (default **off**, Hard Rule #6 — see the module docs).
pub struct OrderState {
    killers: Vec<[Option<Move>; 2]>,
    history: Vec<i32>, // 196×196, indexed `from*196 + to`
    /// FFA bounty term on captures (`victim ffa_points × 500`). Default off; EXP-020 lever.
    pub ffa_bounty: bool,
    /// Free-capture bonus (undefended victim → large bonus). Default off; EXP-020 lever.
    pub free_capture: bool,
}

impl OrderState {
    pub fn new() -> Self {
        OrderState {
            killers: vec![[None; 2]; MAX_PLY],
            history: vec![0; 196 * 196],
            ffa_bounty: false,
            free_capture: false,
        }
    }

    /// Record `mv` as a killer at `ply` (shift slot 0 → slot 1; ignore duplicates / out-of-range ply).
    pub fn add_killer(&mut self, ply: usize, mv: Move) {
        if let Some(slot) = self.killers.get_mut(ply)
            && slot[0] != Some(mv)
        {
            slot[1] = slot[0];
            slot[0] = Some(mv);
        }
    }

    /// Reward a quiet move that turned out best at a node.
    pub fn bump_history(
        &mut self,
        from: crate::board::types::Square,
        to: crate::board::types::Square,
        bonus: i32,
    ) {
        let i = from.index() as usize * 196 + to.index() as usize;
        self.history[i] = self.history[i].saturating_add(bonus);
    }

    fn killers_at(&self, ply: usize) -> [Option<Move>; 2] {
        self.killers.get(ply).copied().unwrap_or([None, None])
    }

    fn history_score(
        &self,
        from: crate::board::types::Square,
        to: crate::board::types::Square,
    ) -> i32 {
        self.history[from.index() as usize * 196 + to.index() as usize]
    }
}

impl Default for OrderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Order `moves` in place, best-first, for the current `board` at search `ply`.
///
/// `sort_by_cached_key` so `score` runs exactly once per move (stable, same order as
/// `sort_by_key`) — `sort_by_key` re-invokes the key O(n·log n) times during the sort, which
/// would multiply the cost of any per-capture board scan in `score`.
pub fn order(
    board: &Board,
    moves: &mut [Move],
    tt_move: Option<Move>,
    ply: usize,
    state: &OrderState,
) {
    let killers = state.killers_at(ply);
    moves.sort_by_cached_key(|m| std::cmp::Reverse(score(board, *m, tt_move, &killers, state)));
}

/// True if the victim's own side defends `square` (could recapture there). A real attack scan
/// ([`crate::board::attacks::is_attacked_by`] — rays, knights, pawn geometry; the same machinery
/// check/castling legality pays per move), replacing an adjacency-only count whose polarity was
/// inverted — it counted NON-victim pieces as "defenders" (EXP-020 measured the damage, EXP-021
/// the fix cost). Known limits, acceptable for ordering: ignores discovered defense (the capturer
/// vacating its square can unblock a defender's line) and counts pinned defenders at face value.
fn is_defended(board: &Board, square: crate::board::types::Square, victim_player: Player) -> bool {
    crate::board::attacks::is_attacked_by(board, square, victim_player)
}

fn score(
    board: &Board,
    m: Move,
    tt_move: Option<Move>,
    killers: &[Option<Move>; 2],
    state: &OrderState,
) -> i32 {
    if Some(m) == tt_move {
        return i32::MAX;
    }
    if m.flags.capture {
        // Victim value — EP captures land on an empty square; victim is a pawn.
        let victim_piece = board.piece_at(m.to);
        let victim_eval = victim_piece.map_or(PieceType::Pawn.eval_value() as i32, |p| {
            p.piece_type.eval_value() as i32
        });
        let victim_ffa = victim_piece.map_or(PieceType::Pawn.ffa_points() as i32, |p| {
            p.piece_type.ffa_points() as i32
        });

        let attacker = board
            .piece_at(m.from)
            .map_or(0, |p| p.piece_type.eval_value() as i32);

        // Captures rank above quiets (10_000 base).
        // eval_value dimension: bigger victim and smaller attacker rank higher.
        // ffa_points dimension: higher bounty (victory points) ranks higher.
        let mut score = 10_000 + victim_eval * 16 - attacker;

        if state.ffa_bounty {
            // Add FFA bounty term: queen (9) >> rook (5) >> knight/bishop (3) >> pawn (1).
            // King (20) is highest but king-capture is terminal (handled separately).
            // Scale: 500 × ffa_points so it competes with eval_value term but doesn't dominate.
            score += victim_ffa * 500;
        }

        // If the victim has no defenders, this capture is "free" — massive bonus. Even a free
        // pawn is worth taking (denies enemy material, gains +1 point). EP captures (victim
        // square empty) get no bonus.
        if state.free_capture
            && let Some(victim) = victim_piece
            && !is_defended(board, m.to, victim.player)
        {
            // Free capture! Bonus scales with FFA value (free queen > free pawn).
            score += 5_000 + victim_ffa * 200;
        }

        score
    } else if killers[0] == Some(m) || killers[1] == Some(m) {
        // Quiet killer move: a quiet that was best at this ply in a sibling node.
        KILLER_BONUS
    } else {
        // Remaining quiets ordered by the history heuristic (kept below the killer bonus).
        state.history_score(m.from, m.to).min(KILLER_BONUS - 1)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Square;
    use crate::board::types::{Piece, Player};

    #[test]
    fn ffa_bounty_queen_ranks_above_pawn() {
        // Red knight on e4 can capture Blue pawn on d6 or Blue queen on f6 (knight jumps).
        // With FFA bounty, queen (9 pts) should sort above pawn (1 pt).
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("e4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("d6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("f6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Queen)),
        );
        // Kings
        b.set_piece(
            Square::from_algebraic("h1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("a7").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("g14").unwrap(),
            Some(Piece::new(Player::Yellow, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("n8").unwrap(),
            Some(Piece::new(Player::Green, PieceType::King)),
        );
        b.recompute_zobrist();

        let mut moves = crate::move_gen::generate_legal(&mut b);
        let mut st = OrderState::new();
        st.ffa_bounty = true; // the lever under test (default off; MVV-LVA alone also passes here)
        order(&b, &mut moves, None, 0, &st);

        // Find the queen capture and pawn capture
        let queen_cap = moves
            .iter()
            .find(|m| m.to == Square::from_algebraic("f6").unwrap());
        let pawn_cap = moves
            .iter()
            .find(|m| m.to == Square::from_algebraic("d6").unwrap());

        assert!(queen_cap.is_some(), "queen capture should exist");
        assert!(pawn_cap.is_some(), "pawn capture should exist");

        // Queen capture should sort before pawn capture
        let queen_idx = moves
            .iter()
            .position(|m| m.to == Square::from_algebraic("f6").unwrap())
            .unwrap();
        let pawn_idx = moves
            .iter()
            .position(|m| m.to == Square::from_algebraic("d6").unwrap())
            .unwrap();
        assert!(
            queen_idx < pawn_idx,
            "queen capture (FFA 9 pts) should sort before pawn capture (FFA 1 pt)"
        );
    }

    #[test]
    fn ffa_bounty_rook_ranks_above_knight() {
        // Red knight on e4 can capture Blue knight on d6 or Blue rook on f6 (knight jumps).
        // Rook (5 pts) should sort above knight (3 pts).
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("e4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("d6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("f6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Rook)),
        );
        // Kings
        b.set_piece(
            Square::from_algebraic("h1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("a7").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("g14").unwrap(),
            Some(Piece::new(Player::Yellow, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("n8").unwrap(),
            Some(Piece::new(Player::Green, PieceType::King)),
        );
        b.recompute_zobrist();

        let mut moves = crate::move_gen::generate_legal(&mut b);
        let mut st = OrderState::new();
        st.ffa_bounty = true; // the lever under test (default off; MVV-LVA alone also passes here)
        order(&b, &mut moves, None, 0, &st);

        let rook_idx = moves
            .iter()
            .position(|m| m.to == Square::from_algebraic("f6").unwrap())
            .unwrap();
        let knight_idx = moves
            .iter()
            .position(|m| m.to == Square::from_algebraic("d6").unwrap())
            .unwrap();
        assert!(
            rook_idx < knight_idx,
            "rook capture (FFA 5 pts) should sort before knight capture (FFA 3 pts)"
        );
    }

    /// Four-kings board (all moves quiet, no captures) for testing killer/history ordering.
    fn four_kings() -> Board {
        let mut b = Board::empty();
        for (sq, pl) in [
            ("h1", Player::Red),
            ("a7", Player::Blue),
            ("g14", Player::Yellow),
            ("n8", Player::Green),
        ] {
            b.set_piece(
                Square::from_algebraic(sq).unwrap(),
                Some(Piece::new(pl, PieceType::King)),
            );
        }
        b.recompute_zobrist();
        b
    }

    #[test]
    fn free_capture_bonus_prefers_undefended_victim() {
        // Two equal-value captures for the Red knight on e4: Blue knight on f6 (UNDEFENDED) vs
        // Blue knight on d6 (defended by the Blue rook on d9 down the open d-file). With
        // `free_capture` on, the undefended capture must rank first — only the free-capture
        // bonus separates the two (same victim type, same attacker, bounty off).
        //
        // The pre-EXP-021 inverted/adjacency-only `count_defenders` fails this exact setup both
        // ways: it could not see the distant rook (adjacency-only) so it ranked the *defended*
        // d6 capture as free, and it counted the non-Blue piece next to f6 (the Yellow pawn on
        // g7) as a "defender" (inverted polarity) so the genuinely free f6 capture got nothing.
        let mut b = Board::empty();
        b.set_piece(
            Square::from_algebraic("e4").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("d6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("f6").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Knight)),
        );
        b.set_piece(
            Square::from_algebraic("d9").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::Rook)),
        );
        b.set_piece(
            Square::from_algebraic("g7").unwrap(),
            Some(Piece::new(Player::Yellow, PieceType::Pawn)),
        );
        // Kings
        b.set_piece(
            Square::from_algebraic("h1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("a7").unwrap(),
            Some(Piece::new(Player::Blue, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("g14").unwrap(),
            Some(Piece::new(Player::Yellow, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("n8").unwrap(),
            Some(Piece::new(Player::Green, PieceType::King)),
        );
        b.recompute_zobrist();

        let mut moves = crate::move_gen::generate_legal(&mut b);
        let mut st = OrderState::new();
        st.free_capture = true; // the lever under test (bounty stays off — equal FFA victims anyway)
        order(&b, &mut moves, None, 0, &st);

        let e4 = Square::from_algebraic("e4").unwrap();
        let free_idx = moves
            .iter()
            .position(|m| m.from == e4 && m.to == Square::from_algebraic("f6").unwrap())
            .expect("knight capture of undefended f6 should exist");
        let defended_idx = moves
            .iter()
            .position(|m| m.from == e4 && m.to == Square::from_algebraic("d6").unwrap())
            .expect("knight capture of defended d6 should exist");
        assert!(
            free_idx < defended_idx,
            "undefended capture (f6) must outrank the defended one (d6) with free_capture on"
        );
    }

    #[test]
    fn ordering_levers_default_off() {
        // Hard Rule #6 guard: the bounty and free-capture ordering levers ship default-off
        // (EXP-020). If a default flips, this fails before any strength change ships silently.
        let st = OrderState::new();
        assert!(!st.ffa_bounty, "ffa_bounty must default off");
        assert!(!st.free_capture, "free_capture must default off");
    }

    #[test]
    fn killer_outranks_other_quiets() {
        let mut b = four_kings();
        let mut moves = crate::move_gen::generate_legal(&mut b);
        assert!(
            moves.len() > 1 && moves.iter().all(|m| !m.flags.capture),
            "expected several quiet king moves"
        );
        let killer = moves[moves.len() - 1];
        let mut st = OrderState::new();
        st.add_killer(0, killer);
        order(&b, &mut moves, None, 0, &st);
        assert_eq!(moves[0], killer, "killer sorts ahead of other quiets");
    }

    #[test]
    fn history_orders_quiets() {
        let mut b = four_kings();
        let mut moves = crate::move_gen::generate_legal(&mut b);
        let favored = moves[moves.len() - 1];
        let mut st = OrderState::new();
        st.bump_history(favored.from, favored.to, 500);
        order(&b, &mut moves, None, 0, &st);
        assert_eq!(moves[0], favored, "history-rewarded quiet sorts first");
    }
}
