//! Self-play A-vs-B harness — the gold-standard strength "vs" (Texel MSE is the fast proxy).
//!
//! Drives four `Searcher`s through a 4PC game from the start position, each seat playing its own
//! config, and scores by accumulated FFA points (`board.points`). To compare config A vs config B,
//! A is rotated through all four seats (B fills the rest), which cancels seat bias — relevant because
//! seat advantage is real in 4PC. Higher average points/seat = stronger.
//!
//! Scope: plays full games via the `Game` driver (the Dead-King-Walking lifecycle, §1.7/1.8). The
//! first few plies are seeded **random openings** so each game diverges — self-play is otherwise
//! deterministic and produces identical games. Compares SEARCH configs (depth/levers — runtime-
//! settable); eval-weight comparison needs runtime weights (an eval.rs refactor), deferred.
//!
//! Run: cargo run --release --example selfplay

use hornet_engine::game::Game;
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Cfg {
    label: &'static str,
    depth: u32,
    beam: usize,
    fwd: bool,
    adaptive: bool,
    quiescence: bool,
    budget: u64,
}

fn searcher(c: Cfg) -> Searcher {
    Searcher::new(16)
        .with_beam_width(c.beam)
        .with_forward_pruning(c.fwd)
        .with_adaptive_beam(c.adaptive)
        .with_quiescence(c.quiescence)
        .with_node_budget(c.budget)
}

/// A small xorshift64 PRNG for seeded random opening moves.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Play one full game (until one survivor or the ply cap) via the `Game` driver. The first `opening`
/// plies are **uniformly-random legal moves** (seeded), so each game takes a different trajectory —
/// this diversifies the otherwise-deterministic self-play and makes eliminations / DKW far more
/// likely to actually occur. Returns final FFA points per seat (RBYG).
fn play_game(cfgs: [Cfg; 4], max_plies: usize, opening: usize, seed: u64) -> [u16; 4] {
    let mut s: Vec<Searcher> = cfgs.iter().map(|&c| searcher(c)).collect();
    let mut game = Game::from_start(seed);
    let mut rng = seed | 1;
    for ply in 0..max_plies {
        if game.active_count() <= 1 {
            break; // one survivor left → game over
        }
        if ply < opening {
            // Seeded random opening: a live player plays a uniformly-random legal move (DKW players
            // are still driven by the Game lifecycle).
            let r = xorshift(&mut rng) as usize;
            game.step(|b| {
                let legal = generate_legal(b);
                (!legal.is_empty()).then(|| legal[r % legal.len()])
            });
        } else {
            let p = game.board.side_to_move.index();
            let depth = cfgs[p].depth;
            game.step(|board| s[p].search(board, depth).map(|(mv, _)| mv));
        }
    }
    game.points()
}

fn main() {
    // Depth-12 ceiling with a node budget that caps per-move work (so depth 12 is manageable: the
    // search reaches whatever fits in the budget, never runs away). Tuning the budget + beam for
    // ~4 min/game; both seats share the config here so the timing is clean (swap a/b to compare).
    let a = Cfg {
        label: "d12",
        depth: 12,
        beam: 10,
        fwd: true,
        adaptive: true,
        quiescence: false,
        budget: 150_000,
    };
    let b = Cfg {
        label: "d12",
        depth: 12,
        beam: 10,
        fwd: true,
        adaptive: true,
        quiescence: false,
        budget: 150_000,
    };
    let max_plies = 100;
    let opening = 12; // seeded random opening plies → diverse trajectories + exercises DKW

    eprintln!(
        "=== SELF-PLAY: A={} vs B={} ({opening} random opening plies, {max_plies}-ply cap, A rotated through 4 seats) ===",
        a.label, b.label
    );
    let (mut a_total, mut b_total) = (0u32, 0u32);
    for a_seat in 0..4 {
        let mut cfgs = [b; 4];
        cfgs[a_seat] = a;
        let t = Instant::now();
        let seed = (a_seat as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let pts = play_game(cfgs, max_plies, opening, seed);
        let secs = t.elapsed().as_secs_f64();
        a_total += pts[a_seat] as u32;
        for (seat, &pt) in pts.iter().enumerate() {
            if seat != a_seat {
                b_total += pt as u32;
            }
        }
        eprintln!(
            "  A in seat {a_seat}: points {pts:?}  (A={})  [{secs:.0}s]",
            pts[a_seat]
        );
    }
    let a_avg = a_total as f64 / 4.0; // A played 4 seat-games
    let b_avg = b_total as f64 / 12.0; // B played 12 seat-games
    eprintln!(
        "--- A={} avg {a_avg:.1} pts/seat | B={} avg {b_avg:.1} pts/seat ---",
        a.label, b.label
    );
    eprintln!(
        "    (4 games = one rotation; high variance — many rotations needed for significance.)"
    );
}
