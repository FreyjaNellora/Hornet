//! Move ordering (spec §6.2). Baseline: the transposition-table move first, then captures
//! by MVV-LVA (most-valuable victim, least-valuable attacker), then quiet moves. Killers and
//! a history heuristic are later additions.

use crate::board::types::PieceType;
use crate::board::{Board, Move};

/// Order `moves` in place, best-first, for the current `board`.
pub fn order(board: &Board, moves: &mut [Move], tt_move: Option<Move>) {
    moves.sort_by_key(|m| std::cmp::Reverse(score(board, *m, tt_move)));
}

fn score(board: &Board, m: Move, tt_move: Option<Move>) -> i32 {
    if Some(m) == tt_move {
        return i32::MAX;
    }
    if m.flags.capture {
        // Victim value (EP captures land on an empty square — the victim is a pawn).
        let victim = board
            .piece_at(m.to)
            .map_or(PieceType::Pawn.eval_value() as i32, |p| {
                p.piece_type.eval_value() as i32
            });
        let attacker = board
            .piece_at(m.from)
            .map_or(0, |p| p.piece_type.eval_value() as i32);
        // Captures rank above quiets; bigger victim and smaller attacker rank higher.
        10_000 + victim * 16 - attacker
    } else {
        0
    }
}
