//! Visit-frequency analysis of the human corpus: where do pieces actually get moved — per player and
//! per piece type — and how *central* is that, vs the centrality the move-agreement tuner (EXP-015)
//! found anti-aligned with good 4PC play? Exploratory: surfaces the data-derived "good squares"
//! (and who concentrates where) to replace chess-centrality for 4PC.
//!
//! Run: cargo run --release --example visit_freq

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

/// Center = the middle 6×6 (ranks/files 4..=9), the squares a centrality PST most rewards.
fn central(r: u8, f: u8) -> bool {
    (4..=9).contains(&r) && (4..=9).contains(&f)
}

fn pt_idx(pt: PieceType) -> usize {
    match pt {
        PieceType::Pawn => 0,
        PieceType::Knight => 1,
        PieceType::Bishop => 2,
        PieceType::Rook => 3,
        PieceType::Queen | PieceType::PromotedQueen => 4,
        PieceType::King => 5,
    }
}
const PT_NAME: [&str; 6] = ["pawn", "knight", "bishop", "rook", "queen", "king"];

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let mut total = [0u32; 4];
    let mut cen = [0u32; 4];
    let (mut sr, mut sf) = ([0u64; 4], [0u64; 4]);
    let mut pt_cen = [(0u32, 0u32); 6]; // (central, total) per piece type
    let mut sq_count: HashMap<(u8, u8), u32> = HashMap::new();

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
                let pt = board.piece_at(human.from).map(|pc| pc.piece_type);
                let (r, f) = (human.to.rank(), human.to.file());
                total[p] += 1;
                sr[p] += r as u64;
                sf[p] += f as u64;
                if central(r, f) {
                    cen[p] += 1;
                }
                *sq_count.entry((r, f)).or_default() += 1;
                if let Some(pt) = pt {
                    let i = pt_idx(pt);
                    pt_cen[i].1 += 1;
                    if central(r, f) {
                        pt_cen[i].0 += 1;
                    }
                }
                board.make_move(human);
            }
        }
    }

    let gt: u32 = total.iter().sum();
    let gc: u32 = cen.iter().sum();
    let pct = |a: u32, b: u32| {
        if b > 0 {
            100.0 * a as f64 / b as f64
        } else {
            0.0
        }
    };
    eprintln!("=== visit frequency over the human corpus ({gt} moves) ===");
    eprintln!(
        "OVERALL central (6x6 middle, ranks/files 4-9): {gc}/{gt} = {:.1}%  (random ~ 18%)",
        pct(gc, gt)
    );
    eprintln!("per player (R,B,Y,G = 0,1,2,3): moves, central%, centroid(rank,file):");
    for p in 0..4 {
        eprintln!(
            "  P{p}: {:4} moves  central {:5.1}%  centroid (r{:4.1}, f{:4.1})",
            total[p],
            pct(cen[p], total[p]),
            sr[p] as f64 / total[p].max(1) as f64,
            sf[p] as f64 / total[p].max(1) as f64
        );
    }
    eprintln!("per piece type — central%:");
    for i in 0..6 {
        eprintln!(
            "  {:6}: central {:5.1}%  ({} of {})",
            PT_NAME[i],
            pct(pt_cen[i].0, pt_cen[i].1),
            pt_cen[i].0,
            pt_cen[i].1
        );
    }
    let mut v: Vec<_> = sq_count.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("top 14 destination squares:");
    for ((r, f), c) in v.into_iter().take(14) {
        eprintln!(
            "  {:>4} (r{r:2},f{f:2}): {c}",
            Square::from_rank_file(r, f).to_algebraic()
        );
    }
}
