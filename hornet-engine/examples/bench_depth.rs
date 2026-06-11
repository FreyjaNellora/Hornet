use hornet_engine::board::fen4;
use hornet_engine::search::Searcher;

fn main() {
    let board = fen4::parse(fen4::START_FEN4).unwrap();

    // Just test baseline depth 8
    let mut s = Searcher::new(16)
        .with_forward_pruning(true)
        .with_adaptive_beam(true);
    let mut b = board.clone();
    let start = std::time::Instant::now();
    let result = s.search(&mut b, 8);
    let elapsed = start.elapsed();
    println!(
        "depth 8 with pruning: nodes={}, time={:?}, best={:?}",
        s.nodes,
        elapsed,
        result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
    );
}
