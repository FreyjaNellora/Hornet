//! UCI-like protocol. Native FEN4/PGN4 ingestion (`position fen4 <string>`,
//! `position pgn4 <filepath>`, `position startpos`, each with an optional `moves <ply>...` list) per
//! Hard Rule #2, plus `go [depth N]` wired to the flashlight search (the current play-shape
//! recommendation — see `run`). Phase 8.
//!
//! Enough to play the engine end-to-end and to drive external self-play: a driver sets the position,
//! sends `go`, reads `bestmove <from-to>`, appends it to the move list, and repeats.
//!
//! **Telemetry (the engine's thinking, for a UI/debugger).** `go` first emits one
//! `info depth D multipv k score R .. B .. Y .. G .. nodes N nps NPS time MS move <from-to>` line per
//! ranked root candidate — the top one also carries `pv <line>` — then `bestmove`. The played move
//! equals `multipv 1` (the bare flashlight's choice; telemetry never changes it). Read-only query
//! commands let a UI render with **zero game logic of its own**: `board` (authoritative FEN4),
//! `legal` (legal moves for the side to move), `status` (side / points / dead / DKW / active count),
//! `eval` (the static eval breakdown — material/positional/safety/crossfire + king-safety counts).
//! The `info` schema is phase-agnostic: MCTS (`visits`/`winrate`) and NNUE fields can be appended
//! later without breaking consumers.

pub mod output;
pub mod parse;

use crate::board::types::{PieceType, Player};
use crate::board::{Board, Move, Square, fen4, pgn4};
use crate::game::{DrawReason, Game, TurnOutcome};
use crate::lines::{LineMap, compute_lines};
use crate::move_gen::{castle_king_destination, generate_legal, generate_pseudo_legal};
use crate::queries::{query_king_safety, run_queries_gated};
use crate::search::{SearchInfo, Searcher};
use parse::{Command, PositionBase};
use std::io::{self, BufRead, Write};
use std::time::Instant;

/// DKW-walk seed for the REPL's starting game.
const DEFAULT_SEED: u64 = 0xC0FFEE;

/// Run the protocol REPL on stdin/stdout until `quit` or EOF.
/// Per-level cap for the playing flashlight. SYNTHESIS (post-EXP-012/016): a generous cap
/// (≥~1000) takes deep search from −47% to ~even on the depth-pathology spectrum — the beam was
/// dropping the best line and breadth recovers it. 1200 is the measured ~even point.
const GO_FLASHLIGHT_CAP: usize = 1200;

pub fn run() {
    // The REPL is a **game server**: it holds a `Game` (so the engine — not any UI — owns the full
    // 4PC lifecycle: legality, checkmate→DKW, the dead-king walk, eliminations, and the EXP-034 draw
    // rules). A UI sends `move`/`go` and reads the authoritative `status`/`board`/`legal`/telemetry.
    let mut game = Game::from_start(DEFAULT_SEED);
    // Playing config: flashlight + generous cap (SYNTHESIS). The leaf eval is the LOADED one
    // (positional + king-safety ON) so a human opponent faces an engine with positional purpose —
    // the human-gate experiment. Self-play / instruments / tests keep the flat deployed eval.
    let mut searcher = Searcher::new(64).with_eval(crate::eval::eval_4vec_loaded);
    // Per-seat recently-vacated squares (the last few moves' from-squares) for the anti-undevelop
    // tie-break: among equally-best moves, don't move back onto a square you just left.
    let mut recent_from: [Vec<Square>; 4] = std::array::from_fn(|_| Vec::new());

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
            Command::NewGame { seed } => {
                game = Game::from_start(seed);
                recent_from = std::array::from_fn(|_| Vec::new());
                emit_status(&game);
            }
            Command::Position { base, moves } => match build_position(&base, &moves) {
                Ok(b) => {
                    game = Game::new(b, DEFAULT_SEED);
                    recent_from = std::array::from_fn(|_| Vec::new());
                    emit_status(&game);
                }
                Err(e) => println!("info string position error: {e}"),
            },
            Command::Go { depth } => engine_go(&mut game, &mut searcher, depth, &mut recent_from),
            Command::Move { mv } => apply_human(&mut game, &mv, &mut recent_from),
            Command::Display => println!(
                "info string side={:?} points={:?}",
                game.board.side_to_move, game.board.points
            ),
            Command::Board => println!("fen4 {}", fen4::serialize(&game.board)),
            Command::Legal => emit_legal(&game.board),
            Command::Status => emit_status(&game),
            Command::Eval => emit_eval(&game.board),
            Command::Unknown(s) => println!("info string unknown command: {s}"),
        }
        let _ = io::stdout().flush();
    }
}

/// Emit every legal move for the side to move (`from-to` tokens), one `legal …` line.
fn emit_legal(board: &Board) {
    let mut b = board.clone(); // generate_legal takes &mut; don't disturb the REPL board
    let mut line = String::from("legal");
    for m in &generate_legal(&mut b) {
        line.push(' ');
        line.push_str(&output::format_move(m));
    }
    println!("{line}");
}

/// The authoritative game-status line: side, active count, points, dead/DKW flags, and the game
/// state (`ongoing` / `draw-repetition` / `draw-fiftymove` / `over`). A UI renders straight from this.
fn emit_status(game: &Game) {
    let b = &game.board;
    let flags =
        |a: [bool; 4]| format!("{} {} {} {}", a[0] as u8, a[1] as u8, a[2] as u8, a[3] as u8);
    let active = (0..4).filter(|&i| !b.dead[i]).count();
    let state = if active <= 1 {
        "over"
    } else {
        match game.draw_status() {
            Some(DrawReason::Repetition) => "draw-repetition",
            Some(DrawReason::FiftyMove) => "draw-fiftymove",
            None => "ongoing",
        }
    };
    println!(
        "status side {:?} active {active} points R {} B {} Y {} G {} dead {} dkw {} state {state}",
        b.side_to_move,
        b.points[0],
        b.points[1],
        b.points[2],
        b.points[3],
        flags(b.dead),
        flags(b.dkw),
    );
}

/// Engine plays the side to move. For a live side it emits the `info` telemetry (its thinking),
/// then applies the move through the game driver — which handles the DKW walk / checkmate→DKW /
/// elimination itself — and reports `bestmove` (or the transition) + the new status.
fn engine_go(
    game: &mut Game,
    searcher: &mut Searcher,
    depth: u32,
    recent: &mut [Vec<Square>; 4],
) {
    if game.active_count() <= 1 {
        println!("info string game already over");
        return;
    }
    let side = game.board.side_to_move.index();
    let live = !game.board.dkw[side] && !game.board.dead[side];
    let choice = if live {
        let t0 = Instant::now();
        let info = searcher.search_flashlight_info(&game.board, depth, |_| GO_FLASHLIGHT_CAP);
        let ms = t0.elapsed().as_millis();
        let nps = if ms > 0 {
            (info.nodes as u128 * 1000 / ms) as u64
        } else {
            0
        };
        for (k, (mv, score)) in info.candidates.iter().enumerate() {
            let pv = (k == 0).then_some(info.pv.as_slice());
            let pc = piece_char(&game.board, mv.from);
            println!(
                "{}",
                output::format_info(k + 1, info.depth, *score, mv, pc, info.nodes, nps, ms, pv)
            );
        }
        pick_move(&info, side, &recent[side])
    } else {
        None // DKW king walks randomly inside game.step; no search needed
    };
    match game.step(|_| choice) {
        TurnOutcome::Moved(m) => {
            push_recent(&mut recent[side], m.from);
            println!("bestmove {}", output::format_move(&m));
        }
        TurnOutcome::EnteredDkw(p) => {
            println!("info string {p:?} checkmated or stalemated — dead-king walking")
        }
        TurnOutcome::Removed(p) => println!("info string {p:?} removed"),
        TurnOutcome::Passed => println!("bestmove (none)"),
    }
    report_end(game);
    emit_status(game);
}

/// The letter of the piece on `from` (`P/N/B/R/Q/K`, `?` if empty) — for the candidate telemetry.
fn piece_char(board: &Board, from: Square) -> char {
    match board.piece_at(from).map(|p| p.piece_type) {
        Some(PieceType::Pawn) => 'P',
        Some(PieceType::Knight) => 'N',
        Some(PieceType::Bishop) => 'B',
        Some(PieceType::Rook) => 'R',
        Some(PieceType::Queen | PieceType::PromotedQueen) => 'Q',
        Some(PieceType::King) => 'K',
        None => '?',
    }
}

/// Push a vacated from-square onto a seat's recent-history ring (keep the last few).
fn push_recent(recent: &mut Vec<Square>, from: Square) {
    recent.push(from);
    if recent.len() > 4 {
        recent.remove(0);
    }
}

/// **Anti-undevelop tie-break.** The engine's flat eval (material + crossfire only — positional and
/// safety are weight-0) ties most quiet moves, so it picks arbitrarily and will happily walk a piece
/// back to a square it just left (shuffling, or undeveloping a rook). Among the equally-best root
/// moves, prefer the first whose destination is NOT a square this seat recently vacated. It only ever
/// breaks ties — never trades away eval — so it can't weaken play; it just stops the pointless
/// back-and-forth and the drift back home. (The deep cure is a positional eval weight; this is the
/// cheap play-quality guard.)
fn pick_move(info: &SearchInfo, mover: usize, recent: &[Square]) -> Option<Move> {
    let top = info.candidates.first()?.1[mover];
    let mut fallback = None;
    for (mv, score) in &info.candidates {
        if score[mover] != top {
            break; // candidates are sorted best-first; we're past the tied-best set
        }
        if fallback.is_none() {
            fallback = Some(*mv);
        }
        if !recent.contains(&mv.to) {
            return Some(*mv); // a tied-best move that doesn't go back onto a vacated square
        }
    }
    fallback // every tied-best move returns to a vacated square (or there's only one) → play the best
}

/// Apply a human move (`from-to`, or a castle token) for the side to move via the game driver.
/// Rejects an illegal move (the engine is the legality authority; the UI never pre-decides).
fn apply_human(game: &mut Game, mv_str: &str, recent: &mut [Vec<Square>; 4]) {
    let mover = game.board.side_to_move.index();
    let mut b = game.board.clone();
    let legal = generate_legal(&mut b);
    match parse_move_token(mv_str, &legal) {
        Some(mv) => {
            if let TurnOutcome::Moved(m) = game.step(|_| Some(mv)) {
                push_recent(&mut recent[mover], m.from);
                println!("moved {}", output::format_move(&m));
            }
            report_end(game);
            emit_status(game);
        }
        None => println!("illegal {mv_str}"),
    }
}

/// After a move, claim a draw or announce the result if the game has ended (+10 each on a draw,
/// per EXP-034 / the chess.com FFA rules).
fn report_end(game: &mut Game) {
    if let Some(reason) = game.draw_status() {
        game.claim_draw();
        let r = match reason {
            DrawReason::Repetition => "repetition",
            DrawReason::FiftyMove => "fiftymove",
        };
        println!("gameover draw {r} points {:?}", game.points());
    } else if game.active_count() <= 1 {
        println!("gameover result points {:?}", game.points());
    }
}

/// Match a `from-to` (or castle) token against the legal set. The UI sends explicit squares (from a
/// click), so the `Normal` path covers castles too (the king's two-square move is in the legal set).
fn parse_move_token(token: &str, legal: &[Move]) -> Option<Move> {
    match pgn4::decode_ply(token)? {
        pgn4::DecodedMove::Normal {
            from,
            to,
            promotion,
        } => legal.iter().copied().find(|m| {
            m.from == from
                && m.to == to
                && (promotion.is_none()
                    || m.promotion == promotion
                    || (promotion == Some(PieceType::Queen)
                        && m.promotion == Some(PieceType::PromotedQueen)))
        }),
        pgn4::DecodedMove::Castle { kingside } => legal.iter().copied().find(|m| {
            m.flags.castle && (kingside == (m.to.file() > m.from.file()) || legal.iter().filter(|x| x.flags.castle).count() == 1)
        }),
    }
}

/// Emit the static eval breakdown for `board` (the `eval` command): one `info eval <component>`
/// line per per-seat query vector, then per-seat `info kingsafety` counts — "why this score."
fn emit_eval(board: &Board) {
    let mut lm = LineMap::new();
    compute_lines(board, &mut lm);
    let q = run_queries_gated(&lm, board, true, true);
    let row = |name: &str, v: [i16; 4]| {
        println!(
            "info eval {name} R {} B {} Y {} G {}",
            v[0], v[1], v[2], v[3]
        );
    };
    row("material", q.material);
    row("positional", q.positional);
    row("safety", q.safety);
    row("crossfire", q.crossfire);
    for (i, k) in query_king_safety(&lm, board).iter().enumerate() {
        let seat = ['R', 'B', 'Y', 'G'][i];
        println!(
            "info kingsafety {seat} defenders {} attackers {} attackvalue {} escapes {}",
            k.defenders, k.attackers, k.attack_value, k.escape_squares
        );
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
