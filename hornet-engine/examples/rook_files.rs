//! Rook file analysis: do rooks prefer open files? Which files do they end up on?
//!
//! Run: cargo run --release --example rook_files

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

    // Track rook positions at END of each game (where did they end up?)
    let mut end_positions: HashMap<String, u64> = HashMap::new();
    // Track all files rooks ever occupy
    let mut file_counts = [0u64; 14];
    // Track rank counts for rooks
    let mut rank_counts = [0u64; 14];
    // How many squares did each rook travel (distance)?
    let mut move_distances: Vec<u64> = Vec::new();

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

        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve(tok, &mut board) else {
                    break 'game;
                };
                if let Some(piece) = board.piece_at(human.from) {
                    if piece.piece_type == PieceType::Rook {
                        let dr = (human.to.rank() as i16 - human.from.rank() as i16).abs() as u64;
                        let df = (human.to.file() as i16 - human.from.file() as i16).abs() as u64;
                        move_distances.push(dr.max(df));
                        file_counts[human.to.file() as usize] += 1;
                        rank_counts[human.to.rank() as usize] += 1;
                    }
                }
                board.make_move(human);
            }
        }

        // Record final rook positions
        for i in 0..196 {
            let sq = Square::new(i as u8);
            if !sq.is_valid() {
                continue;
            }
            if let Some(piece) = board.piece_at(sq) {
                if piece.piece_type == PieceType::Rook {
                    *end_positions.entry(sq.to_algebraic()).or_insert(0) += 1;
                }
            }
        }
    }

    eprintln!("=== Rook file preference (destination files) ===");
    let file_names = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
    ];
    for f in 0..14 {
        eprintln!("  file {}: {}", file_names[f], file_counts[f]);
    }

    eprintln!("\n=== Rook rank preference (destination ranks) ===");
    for r in 0..14 {
        eprintln!("  rank {}: {}", r + 1, rank_counts[r]);
    }

    eprintln!("\n=== Rook move distances (max of |dr|, |df|) ===");
    let total_dist: u64 = move_distances.iter().sum();
    let avg_dist = if !move_distances.is_empty() {
        total_dist as f64 / move_distances.len() as f64
    } else {
        0.0
    };
    eprintln!("  avg distance: {:.1} squares", avg_dist);
    let long_moves = move_distances.iter().filter(|&&d| d >= 5).count();
    eprintln!(
        "  long moves (≥5 squares): {}/{} ({:.1}%)",
        long_moves,
        move_distances.len(),
        100.0 * long_moves as f64 / move_distances.len() as f64
    );

    eprintln!("\n=== Top endgame rook positions ===");
    let mut v: Vec<_> = end_positions.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1));
    for (sq, count) in v.iter().take(12) {
        eprintln!("  {}: {}", sq, count);
    }
}
