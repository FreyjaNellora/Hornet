//! Human-vs-Hornet play session with automatic debug-report capture (the tester loop).
//!
//! The human plays one seat; the engine plays the other three. At game end (or resignation) a
//! **debug report** is written as an extended-header PGN4 file — small, email-able, and directly
//! ingestible by the project's replay/mining instruments. Tester reports go to `versus_games/`
//! (NOT the human-vs-human tuning corpus — engine games stay separate per the data policy).
//!
//! Run: cargo run --release --example play [-- seat depth]
//!   seat  = r|b|y|g (default r — Red moves first)
//!   depth = 4|8 (default 8; Hard Rule #1 — full rotations only). d8 ≈ 4–5 s per engine move.
//!
//! Commands at the prompt: a move (`h2h3`, `h2-h3`, `g7g8=D`, `O-O`, `O-O-O`),
//! `moves` (list legal), `board` (redraw), `resign`, `help`.

use hornet_engine::board::types::{PieceType, Player};
use hornet_engine::board::{Board, Move, Square, dkw_rule};
use hornet_engine::game::{Game, TurnOutcome};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::io::{BufRead, Write};
use std::time::Instant;

const ENGINE_FLASHLIGHT_CAP: usize = 1200;

fn piece_char(t: PieceType) -> char {
    match t {
        PieceType::Pawn => 'P',
        PieceType::Knight => 'N',
        PieceType::Bishop => 'B',
        PieceType::Rook => 'R',
        PieceType::Queen | PieceType::PromotedQueen => 'Q',
        PieceType::King => 'K',
    }
}

fn player_char(p: Player) -> char {
    match p {
        Player::Red => 'r',
        Player::Blue => 'b',
        Player::Yellow => 'y',
        Player::Green => 'g',
    }
}

fn draw(board: &Board) {
    println!();
    for rank in (0..14u8).rev() {
        let mut line = format!("{:2} ", rank + 1);
        for file in 0..14u8 {
            let sq = Square::from_rank_file(rank, file);
            if !sq.is_valid() {
                line.push_str("   ");
            } else {
                match board.piece_at(sq) {
                    Some(p) => {
                        line.push(player_char(p.player));
                        line.push(piece_char(p.piece_type));
                        line.push(' ');
                    }
                    None => line.push_str(" . "),
                }
            }
        }
        println!("{line}");
    }
    println!("    a  b  c  d  e  f  g  h  i  j  k  l  m  n");
    println!(
        "points R:{} B:{} Y:{} G:{}   to move: {:?}{}",
        board.points[0],
        board.points[1],
        board.points[2],
        board.points[3],
        board.side_to_move,
        if board.dkw[board.side_to_move.index()] {
            " (DKW)"
        } else {
            ""
        }
    );
}

/// `h2h3`, `h2-h3`, optional `=D/R/B/N`; `O-O` / `O-O-O` resolved against the legal set.
fn parse_move(input: &str, legal: &[Move]) -> Option<Move> {
    let s = input.trim();
    if s.eq_ignore_ascii_case("o-o") || s.eq_ignore_ascii_case("o-o-o") {
        let kingside = s.len() == 3;
        // The mover's castle to the matching side (one per side at most in the legal set).
        return legal
            .iter()
            .find(|m| {
                m.flags.castle
                    && (kingside == (m.to.file() > m.from.file()) || {
                        // For Blue/Green castling is along ranks; fall back to either castle if only one.
                        legal.iter().filter(|x| x.flags.castle).count() == 1
                    })
            })
            .copied();
    }
    let cleaned: String = s.replace('-', "").to_ascii_lowercase();
    let (mv_part, promo) = match cleaned.split_once('=') {
        Some((m, p)) => (
            m.to_string(),
            match p {
                "d" | "q" => Some(PieceType::Queen),
                "r" => Some(PieceType::Rook),
                "b" => Some(PieceType::Bishop),
                "n" => Some(PieceType::Knight),
                _ => None,
            },
        ),
        None => (cleaned, None),
    };
    // Split "h2h3" into from/to: files are a..n, ranks 1..14 (1-2 digits).
    let bytes = mv_part.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    // from = letter + 1-2 digits, then to = letter + 1-2 digits.
    let second_letter = bytes.iter().skip(1).position(|b| b.is_ascii_alphabetic())? + 1;
    let from = Square::from_algebraic(&mv_part[..second_letter])?;
    let to = Square::from_algebraic(&mv_part[second_letter..])?;
    legal
        .iter()
        .find(|m| {
            m.from == from
                && m.to == to
                && (promo.is_none()
                    || m.promotion == promo
                    || (promo == Some(PieceType::Queen)
                        && m.promotion == Some(PieceType::PromotedQueen)))
        })
        .copied()
}

fn move_str(m: &Move) -> String {
    let mut s = format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic());
    if let Some(p) = m.promotion {
        s.push('=');
        s.push(match p {
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            _ => 'D',
        });
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let human = match args.first().map(|s| s.as_str()) {
        Some("b") => Player::Blue,
        Some("y") => Player::Yellow,
        Some("g") => Player::Green,
        _ => Player::Red,
    };
    let depth: u32 = args
        .get(1)
        .and_then(|a| a.parse().ok())
        .filter(|d| *d == 4 || *d == 8)
        .unwrap_or(8);

    println!("=== Hornet play session ===");
    println!(
        "You are {:?}. Engine plays the rest at depth {depth} (flashlight cap {ENGINE_FLASHLIGHT_CAP}).",
        human
    );
    println!("Enter moves like h2h3 / g7g8=D / O-O. Commands: moves, board, resign, help.");

    let mut game = Game::from_start(0xC0FFEE);
    let mut searchers: Vec<Searcher> = (0..4).map(|_| Searcher::new(64)).collect();
    let mut record: Vec<String> = Vec::new();
    let mut engine_ms: Vec<u128> = Vec::new();
    let mut termination = String::from("game end");
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    draw(&game.board);
    let mut ply = 0usize;
    while game.active_count() > 1 && ply < 400 {
        let mover = game.board.side_to_move;
        if mover == human && !game.board.dkw[human.index()] && !game.board.dead[human.index()] {
            // Human turn.
            let legal = generate_legal(&mut game.board);
            if legal.is_empty() {
                println!("You have no legal moves — checkmate/stalemate; the DKW walk begins.");
                game.step(|_| None); // driver handles the transition
                continue;
            }
            print!("your move> ");
            let _ = std::io::stdout().flush();
            let Some(Ok(input)) = lines.next() else {
                termination = "input closed".into();
                break;
            };
            let cmd = input.trim();
            match cmd {
                "" => continue,
                "help" => {
                    println!("moves like h2h3, h2-h3, g7g8=D, O-O | moves, board, resign");
                    continue;
                }
                "board" => {
                    draw(&game.board);
                    continue;
                }
                "moves" => {
                    let list: Vec<String> = legal.iter().map(move_str).collect();
                    println!("{}", list.join(" "));
                    continue;
                }
                "resign" | "quit" => {
                    termination = "tester resigned".into();
                    break;
                }
                _ => {}
            }
            let Some(mv) = parse_move(cmd, &legal) else {
                println!("not a legal move ('moves' lists them, 'help' for syntax)");
                continue;
            };
            if let TurnOutcome::Moved(m) = game.step(|_| Some(mv)) {
                record.push(move_str(&m));
                ply += 1;
            }
            draw(&game.board);
        } else {
            // Engine (or DKW/dead-skip) turn.
            let seat = mover.index();
            let t0 = Instant::now();
            let outcome = {
                let sr = &mut searchers[seat];
                game.step(|bd| {
                    sr.search_flashlight(bd, depth, |_| ENGINE_FLASHLIGHT_CAP)
                        .map(|(m, _)| m)
                })
            };
            match outcome {
                TurnOutcome::Moved(m) => {
                    engine_ms.push(t0.elapsed().as_millis());
                    println!(
                        "{:?} plays {}   [{:.1}s]",
                        mover,
                        move_str(&m),
                        t0.elapsed().as_secs_f64()
                    );
                    record.push(move_str(&m));
                    ply += 1;
                    if mover.next() == human {
                        draw(&game.board);
                    }
                }
                TurnOutcome::EnteredDkw(p) => println!("{p:?} is checkmated — Dead King Walking."),
                TurnOutcome::Removed(p) => println!("{p:?} is eliminated."),
                TurnOutcome::Passed => {}
            }
        }
    }

    let pts = game.points();
    println!(
        "\n=== game over ({termination}) — points R:{} B:{} Y:{} G:{} ===",
        pts[0], pts[1], pts[2], pts[3]
    );

    // Debug report: extended-header PGN4, ingestible by every project instrument.
    let avg_ms = if engine_ms.is_empty() {
        0
    } else {
        engine_ms.iter().sum::<u128>() / engine_ms.len() as u128
    };
    let mut t = String::new();
    t.push_str("[Variant \"FFA\"]\n");
    t.push_str("[RuleVariants \"DeadKingWalking EnPassant PromoteTo=D\"]\n");
    t.push_str("[StartFen4 \"4PC\"]\n");
    t.push_str(&format!(
        "[Engine \"Hornet {} (flashlight d{depth} cap {ENGINE_FLASHLIGHT_CAP}, dkw_rule {})\"]\n",
        env!("CARGO_PKG_VERSION"),
        dkw_rule()
    ));
    t.push_str(&format!("[HumanSeat \"{:?}\"]\n", human));
    t.push_str(&format!("[Termination \"{termination}\"]\n"));
    t.push_str(&format!("[EngineAvgMoveMs \"{avg_ms}\"]\n"));
    t.push_str(&format!(
        "[Result \"R: {} - B: {} - Y: {} - G: {}\"]\n\n",
        pts[0], pts[1], pts[2], pts[3]
    ));
    for (i, chunk) in record.chunks(4).enumerate() {
        t.push_str(&format!("{}. {}\n", i + 1, chunk.join(" .. ")));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let fname = format!("hornet_report_{stamp}.pgn4");
    std::fs::write(&fname, t).expect("write report");
    println!("debug report written: {fname}");
    println!("(send this file back by email — it replays in the project's instruments)");
}
