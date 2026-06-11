use hornet_engine::board::fen4;
use hornet_engine::lines::LineMap;
use hornet_engine::search::Searcher;

fn main() {
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    let mut searcher = Searcher::new(16);
    let start = std::time::Instant::now();
    let result = searcher.search(&mut board, 4);
    let elapsed = start.elapsed();
    println!(
        "Depth 4 search: {:?} in {:?}",
        result.map(|(m, _)| (m.from.to_algebraic(), m.to.to_algebraic())),
        elapsed
    );
}
