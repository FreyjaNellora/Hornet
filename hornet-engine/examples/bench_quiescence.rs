use hornet_engine::board::fen4;
use hornet_engine::search::Searcher;

fn main() {
    let board = fen4::parse(fen4::START_FEN4).unwrap();

    for (name, depth) in [("d4_no_q", 4), ("d4_q", 4), ("d8_no_q", 8)] {
        let mut s = if name.contains("q") {
            Searcher::new(16).with_quiescence(true)
        } else {
            Searcher::new(16)
        };
        let mut b = board.clone();
        let start = std::time::Instant::now();
        let result = s.search(&mut b, depth);
        let elapsed = start.elapsed();
        println!(
            "{}: nodes={}, time={:?}, best={:?}",
            name,
            s.nodes,
            elapsed,
            result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        );
    }
}
