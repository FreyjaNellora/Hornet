//! Deep rook analysis: when do rooks actually get developed, and where do they go?
//! Tracks rook moves across ALL plies (not just first 20), and rook-specific patterns.
//!
//! Run: cargo run --release --example rook_deep

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::{PieceType, Player, Square};
use hornet_engine::board::{Board, Move};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn resolve(token: &str, board: &mut Board) -> Option<Move> {
    match pgn4::decode_ply(token)? {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let p = board.piece_at(from)?;
            board.side_to_move = p.player;
            generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion)
        }
        DecodedMove::Castle { kingside } => {
            for pl in Player::ALL {
                board.side_to_move = pl;
                let dest = castle_king_destination(pl, kingside);
                if let Some(m) = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.flags.castle && m.to == dest)
                {
                    return Some(m);
                }
            }
            None
        }
    }
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");

    let mut rook_first_move_ply_sum = 0u64;
    let mut rook_first_move_count = 0u64;
    let mut rook_total_moves = 0u64;
    let mut rook_games_moved = 0u64;
    let mut game_count = 0u64;

    // All rook destinations across ALL plies
    let mut rook_dests: HashMap<String, u64> = HashMap::new();
    // Rook destinations in first 40 plies (opening + early middlegame)
    let mut rook_dests_40: HashMap<String, u64> = HashMap::new();
    // What piece was captured by the rook?
    let mut rook_captures: HashMap<String, u64> = HashMap::new();
    // How often does a rook move to a gate/quadrant/center zone square?
    let mut rook_zone_hits = [0u64; 4]; // gate, quadrant, center, other

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let Ok(game) = pgn4::parse(&text) else {
            continue;
        };
        let Ok(mut board) = game.initial_board() else {
            continue;
        };
        game_count += 1;

        let mut first_moved = false;
        let mut ply = 0u64;

        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve(tok, &mut board) else {
                    break 'game;
                };
                if let Some(piece) = board.piece_at(human.from) {
                    if piece.piece_type == PieceType::Rook {
                        let dest = human.to.to_algebraic();
                        *rook_dests.entry(dest.clone()).or_insert(0) += 1;
                        if ply < 40 {
                            *rook_dests_40.entry(dest.clone()).or_insert(0) += 1;
                        }
                        rook_total_moves += 1;
                        if !first_moved {
                            first_moved = true;
                            rook_first_move_ply_sum += ply;
                            rook_first_move_count += 1;
                        }
                        // Zone classification
                        let r = human.to.rank();
                        let f = human.to.file();
                        let is_gate =
                            (r >= 6 && r <= 7 && (f >= 2 && f <= 3 || f >= 10 && f <= 11))
                                || (f >= 6 && f <= 7 && (r >= 2 && r <= 3 || r >= 10 && r <= 11));
                        let is_quad = (r >= 4 && r <= 5 && f >= 4 && f <= 5)
                            || (r >= 4 && r <= 5 && f >= 8 && f <= 9)
                            || (r >= 8 && r <= 9 && f >= 4 && f <= 5)
                            || (r >= 8 && r <= 9 && f >= 8 && f <= 9);
                        let is_center = r >= 6 && r <= 7 && f >= 6 && f <= 7;
                        if is_gate {
                            rook_zone_hits[0] += 1;
                        } else if is_quad {
                            rook_zone_hits[1] += 1;
                        } else if is_center {
                            rook_zone_hits[2] += 1;
                        } else {
                            rook_zone_hits[3] += 1;
                        }
                        // Capture tracking
                        if human.flags.capture {
                            if let Some(victim) = board.piece_at(human.to) {
                                let vname = match victim.piece_type {
                                    PieceType::Pawn => "pawn",
                                    PieceType::Knight => "knight",
                                    PieceType::Bishop => "bishop",
                                    PieceType::Rook => "rook",
                                    PieceType::Queen => "queen",
                                    PieceType::King => "king",
                                    PieceType::PromotedQueen => "promoted_queen",
                                };
                                *rook_captures.entry(vname.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                }
                board.make_move(human);
                ply += 1;
            }
        }
        if first_moved {
            rook_games_moved += 1;
        }
    }

    eprintln!("=== Deep rook analysis over {game_count} human games ===");
    let avg_first = if rook_first_move_count > 0 {
        rook_first_move_ply_sum as f64 / rook_first_move_count as f64
    } else {
        0.0
    };
    let pct_games = if game_count > 0 {
        100.0 * rook_games_moved as f64 / game_count as f64
    } else {
        0.0
    };
    eprintln!(
        "Avg first move ply: {:.1} | % games moved: {:.1}% | total moves: {}",
        avg_first, pct_games, rook_total_moves
    );

    eprintln!("\nZone distribution (all rook moves):");
    let zone_names = ["gate", "quadrant", "center", "other"];
    for i in 0..4 {
        let pct = if rook_total_moves > 0 {
            100.0 * rook_zone_hits[i] as f64 / rook_total_moves as f64
        } else {
            0.0
        };
        eprintln!("  {}: {} ({:.1}%)", zone_names[i], rook_zone_hits[i], pct);
    }

    eprintln!("\nTop rook destinations (ALL plies):");
    let mut v: Vec<_> = rook_dests.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1));
    for (sq, count) in v.iter().take(12) {
        eprintln!("  {}: {}", sq, count);
    }

    eprintln!("\nTop rook destinations (first 40 plies = opening/early middlegame):");
    let mut v2: Vec<_> = rook_dests_40.iter().collect();
    v2.sort_by(|a, b| b.1.cmp(a.1));
    for (sq, count) in v2.iter().take(8) {
        eprintln!("  {}: {}", sq, count);
    }

    eprintln!("\nRook captures by victim type:");
    let mut v3: Vec<_> = rook_captures.iter().collect();
    v3.sort_by(|a, b| b.1.cmp(a.1));
    for (victim, count) in v3.iter().take(6) {
        eprintln!("  {}: {}", victim, count);
    }
}
