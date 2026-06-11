//! Move-match: how often the engine's top move equals the human's actual move, over the human corpus.
//! A DENSE, sensitive eval-quality instrument — one datapoint per move (~1000s per corpus), unlike
//! outcome-MSE (one noisy datapoint per game, swamped by the 16-game noise floor). EXP-012 showed the
//! eval IS the move-chooser, so this measures the thing that matters, and it reads *positional*
//! decisions in quiet positions (which MSE and the tactical fixtures both miss). Use it to gate eval
//! changes: a real eval improvement should lift the match rate.
//!
//! Run: cargo run --release --example move_match [-- beam depth sample bounty freecap]
//!   Defaults `10 4 2 0 0` (the historical instrument config, ordering levers off).
//!   bounty/freecap = the EXP-020 ordering levers (1 = on); fwd-pruning/adaptive stay fixed on.

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::board::{Board, Move};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;

/// Resolve a PGN4 ply token to a concrete move (self-syncing `side_to_move` to the piece's owner, so
/// after this the board is set up for that player to move) — mirrors the replay harness, but does NOT
/// apply the move.
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
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let beam = arg(1).unwrap_or(10);
    let depth = arg(2).unwrap_or(4) as u32; // shallow: EXP-012 says depth barely moves the choice.
    let sample = arg(3).unwrap_or(2).max(1); // every Nth ply
    let bounty = arg(4).unwrap_or(0) != 0;
    let freecap = arg(5).unwrap_or(0) != 0;
    let (mut total, mut matched, mut games) = (0usize, 0usize, 0usize);

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
        games += 1;
        let mut ply = 0usize;
        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve(tok, &mut board) else {
                    break 'game; // replay diverged (elimination) — stop this game
                };
                if ply % sample == 0 {
                    let mut s = Searcher::new(16)
                        .with_beam_width(beam)
                        .with_forward_pruning(true)
                        .with_adaptive_beam(true)
                        .with_ffa_bounty_order(bounty)
                        .with_free_capture_order(freecap);
                    if let Some((eng, _)) = s.search(&mut board, depth) {
                        total += 1;
                        if eng.from == human.from && eng.to == human.to {
                            matched += 1;
                        }
                    }
                }
                board.make_move(human);
                ply += 1;
            }
        }
    }
    let rate = if total > 0 {
        100.0 * matched as f64 / total as f64
    } else {
        0.0
    };
    eprintln!(
        "move-match: {matched}/{total} = {rate:.1}%  over {games} games (beam {beam}, depth {depth}, every {sample} plies, bounty={} freecap={})",
        bounty as u8, freecap as u8
    );
}
