//! UCI-like protocol. Native FEN4/PGN4 ingestion (`position fen4 <string>`,
//! `position pgn4 <filepath>`, `position startpos`, each with an optional `moves <ply>...` list) per
//! Hard Rule #2, plus `go [depth N]` wired to the flashlight search (the current play-shape
//! recommendation — see `run`). Phase 8.
//!
//! Enough to play the engine end-to-end and to drive external self-play: a driver sets the position,
//! sends `go`, reads `bestmove <from-to>`, appends it to the move list, and repeats.

pub mod output;
pub mod parse;

use crate::board::types::Player;
use crate::board::{Board, fen4, pgn4};
use crate::move_gen::{castle_king_destination, generate_pseudo_legal};
use crate::search::Searcher;
use parse::{Command, PositionBase};
use std::io::{self, BufRead, Write};

/// Run the protocol REPL on stdin/stdout until `quit` or EOF.
/// Per-level cap for the playing flashlight. SYNTHESIS (post-EXP-012/016): a generous cap
/// (≥~1000) takes deep search from −47% to ~even on the depth-pathology spectrum — the beam was
/// dropping the best line and breadth recovers it. 1200 is the measured ~even point.
const GO_FLASHLIGHT_CAP: usize = 1200;

pub fn run() {
    let mut board = fen4::parse(fen4::START_FEN4).expect("start FEN4 parses");
    // Playing config (B3, 2026-06-10): the **flashlight** with a generous per-level cap — the
    // current search-shape recommendation (SYNTHESIS; "flashlight + a generous cap, never the
    // laser"). Replaces the deprecated maxn + 2M node-budget config: EXP-012 showed the budget
    // cuts mid-rotation (unsound). Objective-layer knobs (win term / king danger) stay
    // default-off until the C2 self-play gate passes. TT persists across `go` calls in a game.
    let mut searcher = Searcher::new(64);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Some(cmd) = parse::parse(&line) else {
            continue;
        };
        match cmd {
            Command::Uci => {
                println!("id name Hornet");
                println!("id author Project Hornet");
                println!("uciok");
            }
            Command::IsReady => println!("readyok"),
            Command::Quit => break,
            Command::Position { base, moves } => match build_position(&base, &moves) {
                Ok(b) => board = b,
                Err(e) => println!("info string position error: {e}"),
            },
            Command::Go { depth } => {
                match searcher.search_flashlight(&board, depth, |_| GO_FLASHLIGHT_CAP) {
                    Some((mv, _)) => println!("bestmove {}", output::format_move(&mv)),
                    None => println!("bestmove (none)"), // mover has no legal moves (eliminated)
                }
            }
            Command::Display => {
                println!(
                    "info string side={:?} points={:?}",
                    board.side_to_move, board.points
                );
            }
            Command::Unknown(s) => println!("info string unknown command: {s}"),
        }
        let _ = io::stdout().flush();
    }
}

/// Build a board from a base position, then apply the `moves` ply list.
fn build_position(base: &PositionBase, moves: &[String]) -> Result<Board, String> {
    let mut board = match base {
        PositionBase::Start => fen4::parse(fen4::START_FEN4).map_err(|e| e.to_string())?,
        PositionBase::Fen4(s) => fen4::parse(s).map_err(|e| e.to_string())?,
        PositionBase::Pgn4(path) => load_pgn4(path)?,
    };
    for (i, tok) in moves.iter().enumerate() {
        if !apply_ply(tok, &mut board) {
            return Err(format!("move {} ({tok}) is not legal here", i + 1));
        }
    }
    // `apply_ply` self-syncs `side_to_move` with direct writes that bypass the incremental
    // Zobrist update (a no-op for an in-rotation move list, a silent desync otherwise). The
    // board goes straight into a TT-keyed search, so recompute once before handing it out.
    board.recompute_zobrist();
    Ok(board)
}

/// Load a PGN4 file and replay its plies to the final reachable position.
fn load_pgn4(path: &str) -> Result<Board, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let game = pgn4::parse(&text).map_err(|e| e.to_string())?;
    let mut board = game.initial_board().map_err(|e| e.to_string())?;
    let done = |mut b: Board| {
        b.recompute_zobrist(); // see build_position: apply_ply's self-sync bypasses the hash
        Ok(b)
    };
    for round in &game.rounds {
        for tok in &round.plies {
            if !apply_ply(tok, &mut board) {
                return done(board); // stop at the first undecodable/illegal ply (e.g. DKW)
            }
        }
    }
    done(board)
}

/// Decode + self-sync the mover + apply one ply token (mirrors `tests/pgn4_replay.rs`). Accepts the
/// `from-to` and `Pf-t`/`O-O` notations `decode_ply` understands, so `bestmove` output round-trips.
///
/// The self-sync assigns `board.side_to_move` directly, which does NOT update the incremental
/// Zobrist hash (and the castle branch probes all four players the same way). That is fine for
/// replay-matching, but callers must `recompute_zobrist()` before searching the resulting board —
/// `build_position` / `load_pgn4` do.
fn apply_ply(token: &str, board: &mut Board) -> bool {
    use pgn4::DecodedMove;
    let Some(decoded) = pgn4::decode_ply(token) else {
        return false;
    };
    let mv = match decoded {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let Some(p) = board.piece_at(from) else {
                return false;
            };
            board.side_to_move = p.player; // self-sync to the actual mover
            generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion)
        }
        DecodedMove::Castle { kingside } => {
            let mut found = None;
            for pl in Player::ALL {
                board.side_to_move = pl;
                let dest = castle_king_destination(pl, kingside);
                if let Some(m) = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.flags.castle && m.to == dest)
                {
                    found = Some(m);
                    break;
                }
            }
            found
        }
    };
    match mv {
        Some(m) => {
            board.make_move(m);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_plus_moves_then_go_returns_a_move() {
        // position startpos moves h2-h3 ; then a search must return a legal bestmove.
        // Mirrors the shipped `go` path (flashlight; small cap for test speed).
        let board = build_position(&PositionBase::Start, &["h2-h3".to_string()]).unwrap();
        // h2-h3 was Red; after it, Blue is to move.
        assert_ne!(board.side_to_move, Player::Red);
        let mut s = Searcher::new(8);
        let (mv, _) = s.search_flashlight(&board, 4, |_| 200).expect("has a move");
        let text = output::format_move(&mv);
        assert!(text.contains('-'), "bestmove is from-to: {text}");
    }

    #[test]
    fn illegal_move_in_list_is_reported() {
        let r = build_position(&PositionBase::Start, &["a1-a8".to_string()]);
        assert!(r.is_err(), "an impossible move should be rejected");
    }
}
