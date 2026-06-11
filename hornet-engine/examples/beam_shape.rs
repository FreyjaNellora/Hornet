//! Confirm the recommended beam shape: widening ONLY rotation-1 (floor-2 deep) reaches the same move
//! as a uniform wide cap — at much lower cost. Tested on the "sharp" positions (seeds 2,3) that
//! needed wide breadth in the cap-sufficiency study. If the moves match, "wide root + floor deep" is
//! the efficient broadening method.
//!
//! Run: cargo run --release --example beam_shape

use hornet_engine::board::{Board, Move, fen4};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

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
        b.make_move(l[(xorshift(&mut rng) as usize) % l.len()]);
    }
    b
}

fn mv_str(m: Option<Move>) -> String {
    m.map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "-".into())
}

fn run(label: &str, board: &Board, depth: u32, cap_at: impl Fn(u32) -> usize) {
    let mut s = Searcher::new(64);
    let t = Instant::now();
    let mv = s.search_flashlight(board, depth, cap_at).map(|(m, _)| m);
    eprintln!(
        "    {label:22}: {:7.2}s  {:>10} nodes  move {}",
        t.elapsed().as_secs_f64(),
        s.nodes,
        mv_str(mv)
    );
}

fn main() {
    let w = 1600usize;
    for &seed in &[2u64, 3] {
        let board = midgame(seed, 12);
        eprintln!("seed {seed} @ d12:");
        run("uniform 1600", &board, 12, |_| w);
        run("root-wide 1600/floor2", &board, 12, |l| {
            if l < 4 { w } else { 2 }
        });
    }
}
