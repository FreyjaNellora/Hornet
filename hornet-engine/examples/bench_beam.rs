//! Beam-width node/time bench on the maxn path, with the EXP-020 ordering levers exposed and a
//! seeded mid-game mode (the EXP-021 ordering-cost instrument — the start position has no
//! captures, so it cannot exercise the free-capture scan).
//!
//! Run: cargo run --release --example bench_beam [-- bounty freecap mid depth beam count]
//!   Defaults `0 0 0 8 30 5`. bounty/freecap = ordering levers (1 = on).
//!   mid=0: start position, beam sweep {30,20,15,10,8,6} (the golden-reference mode; ignores
//!          beam/count).
//!   mid=1: `count` seeded mid-game positions (24 random opening plies each) searched at `beam`;
//!          reports per-position nodes, time, nodes/sec, and the median nodes/sec. Per-node
//!          ordering cost is beam-independent (`order()` sorts all moves before beam truncation),
//!          so a narrow beam measures the same per-node cost on a far smaller tree.

use hornet_engine::board::fen4;
use hornet_engine::game::Game;
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;

fn searcher(beam: usize, bounty: bool, freecap: bool) -> Searcher {
    Searcher::new(16)
        .with_beam_width(beam)
        .with_forward_pruning(true)
        .with_adaptive_beam(true)
        .with_ffa_bounty_order(bounty)
        .with_free_capture_order(freecap)
}

fn xs(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// A mid-game board: `opening` seeded random plies from the start via the `Game` driver
/// (same pattern as the self-play harnesses, so positions are capture-rich and reproducible).
fn midgame_board(seed: u64, opening: usize) -> hornet_engine::board::Board {
    let mut game = Game::from_start(seed);
    let mut rng = seed | 1;
    for _ in 0..opening {
        if game.active_count() <= 1 {
            break;
        }
        let r = xs(&mut rng) as usize;
        game.step(|b| {
            let l = generate_legal(b);
            (!l.is_empty()).then(|| l[r % l.len()])
        });
    }
    game.board.clone()
}

fn main() {
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let bounty = arg(1).unwrap_or(0) != 0;
    let freecap = arg(2).unwrap_or(0) != 0;
    let mid = arg(3).unwrap_or(0) != 0;
    let depth = arg(4).unwrap_or(8) as u32;
    let mid_beam = arg(5).unwrap_or(30);
    let count = arg(6).unwrap_or(5) as u64;
    println!(
        "bench_beam: bounty={} freecap={} mid={} depth={depth} beam={mid_beam} count={count}",
        bounty as u8, freecap as u8, mid as u8
    );

    if !mid {
        let board = fen4::parse(fen4::START_FEN4).unwrap();
        for beam in [30, 20, 15, 10, 8, 6] {
            let mut s = searcher(beam, bounty, freecap);
            let mut b = board.clone();
            let start = std::time::Instant::now();
            let result = s.search(&mut b, depth);
            let elapsed = start.elapsed();
            println!(
                "beam={}: nodes={}, time={:?}, best={:?}",
                beam,
                s.nodes,
                elapsed,
                result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
            );
        }
    } else {
        const OPENING: usize = 24;
        let mut rates: Vec<f64> = Vec::new();
        for i in 0..count {
            let seed = (i + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let mut b = midgame_board(seed, OPENING);
            let mut s = searcher(mid_beam, bounty, freecap);
            let start = std::time::Instant::now();
            let result = s.search(&mut b, depth);
            let secs = start.elapsed().as_secs_f64();
            let nps = s.nodes as f64 / secs;
            rates.push(nps);
            println!(
                "pos {} (seed {seed:#018x}): nodes={}, time={:.1}s, nodes/sec={:.0}, best={:?}",
                i + 1,
                s.nodes,
                secs,
                nps,
                result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
            );
        }
        rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "median nodes/sec over {} positions: {:.0}",
            rates.len(),
            rates[rates.len() / 2]
        );
    }
}
