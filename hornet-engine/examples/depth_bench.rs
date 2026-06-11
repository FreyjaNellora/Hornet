//! Measure **full-rotation** search cost (no node budget → depth is a clean multiple of 4, Hard Rule
//! #1) at various beam widths, to find a beam that makes full depth-8 / depth-12 tractable for
//! bootstrap self-play. The root is always full-width; only the internal beam narrows.
//!
//! Run: cargo run --release --example depth_bench

use hornet_engine::board::fen4;
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn main() {
    // A representative midgame: start + 16 seeded-random plies (reproducible).
    let mut board = fen4::parse(fen4::START_FEN4).expect("start FEN4");
    let mut rng = 0x1234_5678u64;
    for _ in 0..16 {
        let legal = generate_legal(&mut board);
        if legal.is_empty() {
            break;
        }
        let mv = legal[(xorshift(&mut rng) as usize) % legal.len()];
        board.make_move(mv);
    }

    eprintln!("midgame (start + 16 random plies); FULL depth, no budget.");
    eprintln!(
        "LASER f1 = depth-first, deep floor 1 (Type B); FLASH = level/frontier beam, cap W (Type A)."
    );
    for &depth in &[12u32, 16, 20] {
        run("LASER f1  ", 1, &board, depth);
        run_flash(&board, depth, 1000);
        run_flash_growth(&board, depth, 500, 8000);
    }
}

/// Flashlight (Type A): level/frontier beam, fixed top-`cap` per level. Cost ~linear in depth.
fn run_flash(board: &hornet_engine::board::Board, depth: u32, cap: usize) {
    let mut s = Searcher::new(64);
    let t = Instant::now();
    let mv = s.search_flashlight(board, depth, |_| cap).map(|(m, _)| m);
    let secs = t.elapsed().as_secs_f64();
    let mvs = mv
        .map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "(none)".into());
    eprintln!(
        "  depth {depth:2} FLASH cap{cap:5}: {secs:8.2}s  {:>11} nodes  move {mvs}",
        s.nodes
    );
}

/// Flashlight with a per-rotation **growing** cap: `base << rotation`, capped at `ceiling` (the
/// "respect more positions deeper" idea). Rotation = level / 4.
fn run_flash_growth(board: &hornet_engine::board::Board, depth: u32, base: usize, ceiling: usize) {
    let mut s = Searcher::new(64);
    let t = Instant::now();
    let mv = s
        .search_flashlight(board, depth, |level| (base << (level / 4)).min(ceiling))
        .map(|(m, _)| m);
    let secs = t.elapsed().as_secs_f64();
    let mvs = mv
        .map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "(none)".into());
    eprintln!(
        "  depth {depth:2} FLASH grow {base}->{ceiling}: {secs:8.2}s  {:>11} nodes  move {mvs}",
        s.nodes
    );
}

/// Noise-adaptive: narrow (floor 1) when there's a real tactic, broad (width 6) when quiet.
#[allow(dead_code)]
fn run_noise(board: &hornet_engine::board::Board, depth: u32) {
    let mut s = Searcher::new(64)
        .with_beam_width(6) // broad, used at quiet nodes
        .with_deep_floor(1) // narrow, used at noisy nodes
        .with_forward_pruning(true)
        .with_noise_adaptive(true);
    let mut b = board.clone();
    let t = Instant::now();
    let mv = s.search(&mut b, depth).map(|(m, _)| m);
    let secs = t.elapsed().as_secs_f64();
    let mvs = mv
        .map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "(none)".into());
    eprintln!(
        "  depth {depth:2} NOISE n1/b6: {secs:8.2}s  {:>11} nodes  move {mvs}",
        s.nodes
    );
}

fn run(kind: &str, floor: usize, board: &hornet_engine::board::Board, depth: u32) {
    let mut s = Searcher::new(64)
        .with_beam_width(4)
        .with_forward_pruning(true)
        .with_adaptive_beam(true)
        .with_deep_floor(floor);
    let mut b = board.clone();
    let t = Instant::now();
    let mv = s.search(&mut b, depth).map(|(m, _)| m);
    let secs = t.elapsed().as_secs_f64();
    let mvs = mv
        .map(|m| format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic()))
        .unwrap_or_else(|| "(none)".into());
    eprintln!(
        "  depth {depth:2} {kind}: {secs:8.2}s  {:>11} nodes  move {mvs}",
        s.nodes
    );
}
