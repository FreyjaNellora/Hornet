use hornet_engine::board::Board;
use hornet_engine::board::fen4;
use hornet_engine::board::types::Player;
use hornet_engine::move_gen::generate_pseudo_legal;
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    // Just test S01 from start position for speed baseline
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    let mut searcher = Searcher::new(16);

    let start = Instant::now();
    let result = searcher.search(&mut board, 4);
    let elapsed = start.elapsed();
    println!(
        "Start position depth 4: {:?} in {:?}",
        result.map(|(m, _)| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic())),
        elapsed
    );
}
