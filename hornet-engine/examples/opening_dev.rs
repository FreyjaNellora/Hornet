//! Opening development tracker: how fast do pieces get developed (move from start square)?
//! Tracks per-piece-type: ply of first move, average ply of all moves, % of games where piece moves.
//!
//! Run: cargo run --release --example opening_dev

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

fn pt_name(pt: PieceType) -> &'static str {
    match pt {
        PieceType::Pawn => "pawn",
        PieceType::Knight => "knight",
        PieceType::Bishop => "bishop",
        PieceType::Rook => "rook",
        PieceType::Queen => "queen",
        PieceType::King => "king",
        PieceType::PromotedQueen => "promoted_queen",
    }
}

fn pt_idx(pt: PieceType) -> usize {
    match pt {
        PieceType::Pawn => 0,
        PieceType::Knight => 1,
        PieceType::Bishop => 2,
        PieceType::Rook => 3,
        PieceType::Queen => 4,
        PieceType::King => 5,
        PieceType::PromotedQueen => 4,
    }
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");

    // Per-piece-type stats: (first_move_ply_sum, first_move_count, total_moves, games_present)
    let mut stats: [(u64, u64, u64, u64); 6] = [(0, 0, 0, 0); 6];
    // Track which pieces have moved in each game, keyed by "game_idx:piece_type"
    let mut game_count = 0u64;

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

        // Track first move per piece type in this game
        let mut first_moved: [bool; 6] = [false; 6];
        let mut ply = 0u64;

        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve(tok, &mut board) else {
                    break 'game;
                };
                if let Some(piece) = board.piece_at(human.from) {
                    let idx = pt_idx(piece.piece_type);
                    stats[idx].2 += 1; // total_moves
                    if !first_moved[idx] {
                        first_moved[idx] = true;
                        stats[idx].0 += ply; // first_move_ply_sum
                        stats[idx].1 += 1; // first_move_count
                    }
                }
                board.make_move(human);
                ply += 1;
            }
        }
        // Count games where each piece type moved at least once
        for i in 0..6 {
            if first_moved[i] {
                stats[i].3 += 1;
            }
        }
    }

    eprintln!("=== Opening development over {game_count} human games ===");
    eprintln!("piece    | avg_first_move_ply | %_games_moved | total_moves");
    eprintln!("---------|--------------------|---------------|------------");
    let names = ["pawn", "knight", "bishop", "rook", "queen", "king"];
    for i in 0..6 {
        let avg_first = if stats[i].1 > 0 {
            stats[i].0 as f64 / stats[i].1 as f64
        } else {
            0.0
        };
        let pct_games = if game_count > 0 {
            100.0 * stats[i].3 as f64 / game_count as f64
        } else {
            0.0
        };
        eprintln!(
            "{:8} | {:18.1} | {:13.1}% | {}",
            names[i], avg_first, pct_games, stats[i].2
        );
    }

    // Also: which specific squares do pieces move TO in the first 20 plies?
    eprintln!("\n=== First-move destinations by piece type (first 20 plies) ===");
    let mut dest_counts: [HashMap<String, u64>; 6] = [
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    ];

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
        let mut ply = 0u64;

        'game2: for round in &game.rounds {
            for tok in &round.plies {
                if ply >= 20 {
                    break 'game2;
                }
                let Some(human) = resolve(tok, &mut board) else {
                    break 'game2;
                };
                if let Some(piece) = board.piece_at(human.from) {
                    let idx = pt_idx(piece.piece_type);
                    let dest = human.to.to_algebraic();
                    *dest_counts[idx].entry(dest).or_insert(0) += 1;
                }
                board.make_move(human);
                ply += 1;
            }
        }
    }

    for i in 0..6 {
        let mut v: Vec<_> = dest_counts[i].iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        eprintln!("\n{} top destinations (first 20 plies):", names[i]);
        for (sq, count) in v.iter().take(8) {
            eprintln!("  {}: {}", sq, count);
        }
    }
}
