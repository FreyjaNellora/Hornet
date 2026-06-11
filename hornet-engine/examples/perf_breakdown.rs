//! Latency breakdown: where does search time actually go? Times the core ops — the line projection
//! (`compute_lines`, the array-line substrate everything sits on), the full eval, and move generation —
//! then a flashlight search, and attributes the search time. The flashlight evals every kept candidate,
//! so eval cost × node count should dominate; this measures it instead of guessing.
//!
//! Run on an idle machine: cargo run --release --example perf_breakdown

use hornet_engine::board::{Board, fen4};
use hornet_engine::eval::eval_4vec;
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

fn xs(s: &mut u64) -> u64 {
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
        b.make_move(l[(xs(&mut rng) as usize) % l.len()]);
    }
    b
}

/// Mean ns/call over `n` iterations of `f`, with warm-up. Returns µs/call.
fn time_us(n: usize, mut f: impl FnMut()) -> f64 {
    for _ in 0..(n / 10).max(5) {
        f();
    }
    let t = Instant::now();
    for _ in 0..n {
        f();
    }
    t.elapsed().as_nanos() as f64 / 1000.0 / n as f64
}

fn main() {
    let board = midgame(7, 16);
    let mut lm = LineMap::new();
    let mut mgb = board.clone();
    let n = 3000;

    let t_lines = time_us(n, || compute_lines(&board, &mut lm));
    let t_eval = time_us(n, || {
        eval_4vec(&board, &mut lm);
    });
    let t_mg = time_us(n, || {
        let _ = generate_legal(&mut mgb);
    });

    eprintln!("per-op cost (midgame position, release):");
    eprintln!("  compute_lines (line projection): {t_lines:7.2} µs");
    eprintln!(
        "  run_all_queries (eval − lines) : {:7.2} µs",
        t_eval - t_lines
    );
    eprintln!(
        "  eval_4vec (lines + queries)    : {t_eval:7.2} µs   <- the cost paid per searched node"
    );
    eprintln!("  generate_legal                 : {t_mg:7.2} µs");

    let mut s = Searcher::new(64);
    let t = Instant::now();
    let _ = s.search_flashlight(&board, 8, |_| 400);
    let secs = t.elapsed().as_secs_f64();
    let eval_share = s.nodes as f64 * t_eval / 1e6;
    eprintln!(
        "\nflashlight d8 cap400: {secs:.2}s, {} nodes ({:.0}k nodes/s)",
        s.nodes,
        s.nodes as f64 / secs / 1000.0
    );
    eprintln!(
        "  eval alone ≈ {eval_share:.2}s = {:.0}% of search time (rest = move-gen, backup, pruning sort)",
        100.0 * eval_share / secs
    );
    eprintln!(
        "\nlever: if eval dominates, the wins are (a) a cheaper eval or (b) fewer nodes (lower cap / smarter pruning)."
    );
}
