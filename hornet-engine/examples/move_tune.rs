//! Move-agreement tuner: fit the eval weights (M,P,S,O) so the human's actual move is the eval's
//! TOP-scored move as often as possible. This optimizes the thing we measure (move choice, EXP-012)
//! and care about — unlike outcome-MSE, which the PST cross-check (EXP-014) showed can reward a config
//! that plays *worse*. Static (depth-1) per-move eval, cached, so tuning is pure arithmetic. Validate
//! the winning weights with `move_match` (search-based) before trusting them.
//!
//! Run: cargo run --release --example move_tune

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::board::{Board, Move};
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::{castle_king_destination, generate_legal, generate_pseudo_legal};
use hornet_engine::queries::run_all_queries;
use std::fs;
use std::path::PathBuf;

/// One position: the mover's mean-relative [M,P,S,O] for each legal move, + the human move's index.
struct PosMoves {
    moves: Vec<[f64; 4]>,
    human: usize,
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

/// eval-of-child for the mover under weights w (mirrors `compute_utility`: O is subtracted).
fn score(c: &[f64; 4], w: [f64; 4]) -> f64 {
    w[0] * c[0] + w[1] * c[1] + w[2] * c[2] - w[3] * c[3]
}

/// Fraction of positions where the human's move is the top-scored move under w.
fn match_rate(data: &[PosMoves], w: [f64; 4]) -> f64 {
    let mut hit = 0usize;
    for pm in data {
        let best = (0..pm.moves.len())
            .max_by(|&a, &b| score(&pm.moves[a], w).total_cmp(&score(&pm.moves[b], w)))
            .unwrap();
        if best == pm.human {
            hit += 1;
        }
    }
    if data.is_empty() {
        0.0
    } else {
        hit as f64 / data.len() as f64
    }
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let mut lines = LineMap::new();
    let mut data: Vec<PosMoves> = Vec::new();
    let mean = |a: [i16; 4]| (a[0] as f64 + a[1] as f64 + a[2] as f64 + a[3] as f64) / 4.0;
    // HORNET_QUIET=1 → keep only positions with NO capture available (pure positional choice). Tests
    // whether positional value is genuinely absent or just masked by material in the full corpus.
    let quiet_only = std::env::var("HORNET_QUIET").is_ok();

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
                let p = board.side_to_move.index();
                let legal = generate_legal(&mut board);
                let has_capture = legal.iter().any(|m| board.piece_at(m.to).is_some());
                if legal.len() > 1 && !(quiet_only && has_capture) {
                    let mut moves = Vec::with_capacity(legal.len());
                    let mut human_idx = usize::MAX;
                    for (i, m) in legal.iter().enumerate() {
                        if m.from == human.from
                            && m.to == human.to
                            && m.promotion == human.promotion
                        {
                            human_idx = i;
                        }
                        let undo = board.make_move(*m);
                        compute_lines(&board, &mut lines);
                        let qv = run_all_queries(&lines, &board);
                        board.unmake_move(undo);
                        moves.push([
                            qv.material[p] as f64 - mean(qv.material),
                            qv.positional[p] as f64 - mean(qv.positional),
                            qv.safety[p] as f64 - mean(qv.safety),
                            qv.crossfire[p] as f64 - mean(qv.crossfire),
                        ]);
                    }
                    if human_idx != usize::MAX {
                        data.push(PosMoves {
                            moves,
                            human: human_idx,
                        });
                    }
                }
                board.make_move(human);
            }
        }
    }
    eprintln!(
        "move-agreement tuner: {} positions (depth-1 static eval)",
        data.len()
    );

    let baseline = [4.0, 1.0, 1.0, 1.0];
    eprintln!(
        "baseline (4,1,1,1): {:.1}% top-1 move-agreement",
        100.0 * match_rate(&data, baseline)
    );

    // Hill-climb integer weights (deployable as i16), objective = move-agreement.
    let mut w = baseline;
    let mut cur = match_rate(&data, w);
    let mut improved = true;
    while improved {
        improved = false;
        for idx in 0..4 {
            for delta in [-1.0, 1.0] {
                let mut cand = w;
                cand[idx] = (cand[idx] + delta).max(0.0);
                let r = match_rate(&data, cand);
                if r > cur + 1e-9 {
                    w = cand;
                    cur = r;
                    improved = true;
                }
            }
        }
    }
    eprintln!(
        "tuned: M={} P={} S={} O={} -> {:.1}% (baseline {:.1}%, +{:.1}pp)",
        w[0],
        w[1],
        w[2],
        w[3],
        100.0 * cur,
        100.0 * match_rate(&data, baseline),
        100.0 * (cur - match_rate(&data, baseline))
    );
}
