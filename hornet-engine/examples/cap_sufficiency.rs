//! Cap-sufficiency study: for each position and depth, the **minimum** flashlight level-cap whose
//! chosen move already matches the widest cap's move. Empirically characterizes cap(depth) — does a
//! deeper search need a wider cap to lock onto the best move, or does a small cap suffice at all
//! depths? Also prints the move at each cap so the stabilization point is visible.
//!
//! Run: cargo run --release --example cap_sufficiency

use hornet_engine::board::{Board, Move, fen4};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn midgame(seed: u64, plies: usize) -> Board {
    let mut b = fen4::parse(fen4::START_FEN4).expect("start");
    let mut rng = seed | 1;
    for _ in 0..plies {
        let l = generate_legal(&mut b);
        if l.is_empty() {
            break;
        }
        let mv = l[(xorshift(&mut rng) as usize) % l.len()];
        b.make_move(mv);
    }
    b
}

fn mv_str(m: Option<Move>) -> String {
    m.map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "-".into())
}

fn main() {
    let caps = [50usize, 100, 200, 400, 800, 1600];
    let depths = [8u32, 12, 16];
    let seeds = [1u64, 2, 3, 4];
    let widest = *caps.last().unwrap();
    eprintln!("cap-sufficiency: min cap whose move == the cap-{widest} move, per position+depth.");
    eprintln!(
        "(caps {caps:?}; if min_cap == {widest}, the move was still moving at the widest cap.)"
    );
    for &seed in &seeds {
        let board = midgame(seed, 12);
        for &depth in &depths {
            let mut moves: Vec<Option<Move>> = Vec::new();
            for &cap in &caps {
                let mut s = Searcher::new(64);
                moves.push(s.search_flashlight(&board, depth, |_| cap).map(|(m, _)| m));
            }
            let refmv = *moves.last().unwrap();
            let min_cap = caps
                .iter()
                .zip(&moves)
                .find(|(_, m)| **m == refmv)
                .map(|(c, _)| *c)
                .unwrap_or(widest);
            let seq: Vec<String> = moves.iter().map(|m| mv_str(*m)).collect();
            eprintln!(
                "  seed {seed} d{depth:2}: min_cap {min_cap:5}  ref {}  [{}]",
                mv_str(refmv),
                seq.join(", ")
            );
        }
    }
}
