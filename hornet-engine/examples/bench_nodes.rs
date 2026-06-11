use hornet_engine::board::fen4;
use hornet_engine::search::Searcher;

fn main() {
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    let mut searcher = Searcher::new(16);
    let start = std::time::Instant::now();
    let result = searcher.search(&mut board, 4);
    let elapsed = start.elapsed();
    println!(
        "Nodes: {}, Time: {:?}, Best: {:?}",
        searcher.nodes,
        elapsed,
        result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
    );
}
