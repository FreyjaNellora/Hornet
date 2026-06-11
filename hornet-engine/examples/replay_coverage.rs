//! Replay-coverage diagnostic + robust replay. Raw `generate_pseudo_legal` replay desyncs at
//! king-captures / DKW events (it never produces a king-capture), cascading and dumping the rest of
//! the game. This adds a **force-apply fallback**: try the legal resolve first (handles normal moves,
//! castling, en-passant); on failure, apply the decoded from→to directly, and if the victim is a king,
//! eliminate that player (sweep). Plus suffix stripping (`+ # T R S`). Measures recovery.
//!
//! Run: cargo run --release --example replay_coverage

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::{PieceType, Player};
use hornet_engine::board::{Board, Move, Piece};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use std::fs;
use std::path::PathBuf;

/// Strip trailing annotation chars (`+ # T R S` and `##`/`++`). Safe: a complete move ends in a digit
/// (rank) or `D` (the forced-queen promotion), never in these.
fn strip(tok: &str) -> String {
    tok.trim_end_matches(|c| matches!(c, '+' | '#' | 'T' | 'R' | 'S'))
        .to_string()
}

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

/// Faithfully apply a human ply: legal resolve first, else force-apply (king-captures, post-desync).
/// Returns false only if the token can't be decoded to a real move at all.
fn robust_apply(token: &str, board: &mut Board) -> bool {
    let clean = strip(token);
    if clean.is_empty() {
        return false;
    }
    if let Some(m) = resolve(&clean, board) {
        board.make_move(m);
        return true;
    }
    // Force-apply fallback (resolve only fails for king-captures or a desynced board).
    if let Some(DecodedMove::Normal {
        from,
        to,
        promotion,
    }) = pgn4::decode_ply(&clean)
    {
        if let Some(mp) = board.piece_at(from) {
            board.side_to_move = mp.player;
            let victim = board.piece_at(to);
            let placed = match promotion {
                Some(pt) => Piece {
                    player: mp.player,
                    piece_type: pt,
                },
                None => mp,
            };
            board.set_piece(from, None);
            board.set_piece(to, Some(placed));
            if let Some(v) = victim {
                if v.piece_type == PieceType::King {
                    board.eliminate_player(v.player); // sweep the eliminated player's pieces
                }
            }
            return true;
        }
    }
    false
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let (mut tot_plies, mut tot_replayed) = (0usize, 0usize);
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let Ok(game) = pgn4::parse(&text) else {
            continue;
        };
        let total: usize = game.rounds.iter().map(|r| r.plies.len()).sum();
        let Ok(mut board) = game.initial_board() else {
            continue;
        };
        let mut replayed = 0usize;
        let mut fail: Option<String> = None;
        'game: for round in &game.rounds {
            for tok in &round.plies {
                if robust_apply(tok, &mut board) {
                    replayed += 1;
                } else if strip(tok).is_empty() {
                    continue; // standalone status token (S/R/T) — a non-move ply; skip and continue
                } else {
                    fail = Some(tok.clone());
                    break 'game;
                }
            }
        }
        tot_plies += total;
        tot_replayed += replayed;
        let name = path.file_name().unwrap().to_string_lossy();
        let pct = 100.0 * replayed as f64 / total.max(1) as f64;
        eprintln!(
            "{name}: {replayed}/{total} ({pct:.0}%)  {}",
            fail.map(|t| format!("stop: {t}"))
                .unwrap_or_else(|| "full".into())
        );
    }
    eprintln!(
        "\nTOTAL: {tot_replayed}/{tot_plies} plies replayed ({:.0}%)",
        100.0 * tot_replayed as f64 / tot_plies.max(1) as f64
    );
}
