//! Search-lever ablation benchmark: search the canonical start position with the two default-off
//! speed levers (forward pruning / LMR, adaptive beam) off and on, reporting node count + wall
//! time. This is the measured ablation arm the design rule requires for any new speed/strength
//! lever — and it shows whether the two levers stack.
//!
//! Run: cargo run --release --example search_bench

use hornet_engine::board::fen4;
use hornet_engine::search::Searcher;
use std::time::Instant;

fn run(depth: u32, beam: usize, fp: bool, ab: bool) -> (u64, f64, Option<(u8, u8)>) {
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    let mut s = Searcher::new(64)
        .with_beam_width(beam)
        .with_forward_pruning(fp)
        .with_adaptive_beam(ab);
    let t = Instant::now();
    let res = s.search(&mut board, depth);
    let secs = t.elapsed().as_secs_f64();
    (
        s.nodes,
        secs,
        res.map(|(m, _)| (m.from.index(), m.to.index())),
    )
}

fn main() {
    println!("Search-lever ablation — start position\n");

    let configs = [
        ("flat        ", false, false),
        ("lmr         ", true, false),
        ("adaptive    ", false, true),
        ("lmr+adaptive", true, true),
    ];

    let depth = 4;
    let beam = 20;
    println!("depth {depth}, beam {beam}:");
    println!("  config         nodes      time(s)   vs flat   move");
    let (flat_nodes, flat_secs, _) = run(depth, beam, false, false);
    for (label, fp, ab) in configs {
        let (nodes, secs, mv) = run(depth, beam, fp, ab);
        let cut = 100.0 * (1.0 - nodes as f64 / flat_nodes.max(1) as f64);
        let speedup = if secs > 0.0 { flat_secs / secs } else { 0.0 };
        println!("  {label}  {nodes:>9}  {secs:>8.3}   -{cut:>4.1}% {speedup:>4.1}x   {mv:?}");
    }

    // Deeper: the combined levers vs the known flat baseline (flat depth-8/beam-6 ≈ 6.4M nodes / ~89 s).
    println!("\ndepth 8, beam 6 (lmr+adaptive):");
    let (nodes, secs, mv) = run(8, 6, true, true);
    println!("  nodes {nodes}  time {secs:.2}s  move {mv:?}");
}
