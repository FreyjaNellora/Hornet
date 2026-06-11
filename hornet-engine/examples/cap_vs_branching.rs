//! Cap-vs-branching study: for positions spanning a game (opening → thinned-out), record the
//! **minimum flashlight cap whose move == the widest cap's move** (cap-needed-to-converge) alongside
//! the **branching factor** (legal-move count) and piece count. Tests the hypothesis that the needed
//! cap scales with branching — the basis for an adaptive `cap = clamp(k·branching, floor, ceiling)`,
//! where `ceiling = k · max_branching` is the hard limit (the busiest board you can ever face).
//!
//! Exports `tools/cap_branching.csv` for `tools/fit_cap.py`.
//! Run: cargo run --release --example cap_vs_branching

use hornet_engine::board::types::Player;
use hornet_engine::board::{Board, Move, fen4};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn xs(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// Random walk `plies` deep — captures thin the board, so larger `plies` ≈ fewer pieces.
fn walk(seed: u64, plies: usize) -> Board {
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

fn piece_total(b: &Board) -> usize {
    Player::ALL.iter().map(|&p| b.piece_count(p)).sum()
}

fn main() {
    let caps = [50usize, 100, 200, 400, 800, 1600, 3200];
    let widest = *caps.last().unwrap();
    let depth = 8u32;
    let ply_targets = [4usize, 12, 24, 40, 60, 80, 100, 120];
    let seeds = [1u64, 2, 3, 4, 5];

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tools");
    fs::create_dir_all(&out).ok();
    let path = out.join("cap_branching.csv");
    let mut fh = fs::File::create(&path).expect("create csv");
    writeln!(fh, "ply,pieces,branching,cap_needed,converged").unwrap();

    let mut max_branching = 0usize;
    let mut max_cap_needed = 0usize;
    eprintln!(
        "cap-vs-branching @ d{depth} (caps {caps:?}); cap_needed = min cap stable up to the widest."
    );
    for &plies in &ply_targets {
        for &seed in &seeds {
            let mut board = walk(seed, plies);
            let legal = generate_legal(&mut board);
            if legal.len() < 2 {
                continue; // terminal / trivial
            }
            let branching = legal.len();
            let pcs = piece_total(&board);

            let moves: Vec<Option<Move>> = caps
                .iter()
                .map(|&cap| {
                    Searcher::new(64)
                        .search_flashlight(&board, depth, |_| cap)
                        .map(|(m, _)| m)
                })
                .collect();
            let refmv = *moves.last().unwrap();
            // smallest cap from which the move is STABLE (== refmv at that cap and every wider one)
            let mut cap_needed = widest;
            for i in 0..caps.len() {
                if moves[i..].iter().all(|m| *m == refmv) {
                    cap_needed = caps[i];
                    break;
                }
            }
            // "converged" = it stabilized strictly before the widest cap (so widest isn't a floor artifact)
            let converged = cap_needed < widest || moves[moves.len() - 2] == refmv;
            max_branching = max_branching.max(branching);
            max_cap_needed = max_cap_needed.max(cap_needed);

            writeln!(
                fh,
                "{plies},{pcs},{branching},{cap_needed},{}",
                converged as u8
            )
            .unwrap();
            eprintln!(
                "  ply {plies:3} seed {seed}: pieces {pcs:2}  branching {branching:3}  -> cap_needed {cap_needed:5}{}",
                if converged { "" } else { "  (still moving!)" }
            );
        }
    }
    eprintln!("\nmax branching seen: {max_branching}  | max cap_needed: {max_cap_needed}");
    eprintln!("hard ceiling ≈ k · {max_branching} (busiest board) — real positions sit under it.");
    eprintln!(
        "wrote {} — analyze with: py tools/fit_cap.py",
        path.display()
    );
}
