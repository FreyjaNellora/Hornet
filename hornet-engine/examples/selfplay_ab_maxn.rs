//! Self-play A/B on the **maxn path** — the move-ordering ablation venue (EXP-020).
//!
//! `selfplay_ab` drives `search_flashlight`, which never calls `move_order`, so it cannot measure
//! the ordering flags. This harness keeps its structure — six balanced 2-vs-2 seat splits (each
//! seat plays A in 3 of 6), deterministic seeds, seeded random openings, FFA-points scoring — but
//! each seat plays the default `search()` (beam Max^n), where ordering *is* selection at narrow
//! beams. Configs differ in beam width and the two ordering flags (FFA bounty, free-capture bonus).
//!
//! Run: selfplay_ab_maxn [a_beam] [b_beam] [a_bounty] [a_freecap] [b_bounty] [b_freecap] [depth=8] [games_per_split=2]
//!   e.g. `selfplay_ab_maxn 4 4 1 1 1 0 8 2` = beam 4 both, A(bounty+freecap) vs B(bounty only):
//!   the EXP-020 contamination pairing (arm i vs arm ii).

use hornet_engine::game::Game;
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

#[derive(Clone)]
struct Cfg {
    label: String,
    depth: u32,
    beam: usize,
    bounty: bool,
    freecap: bool,
}
impl Cfg {
    fn searcher(&self) -> Searcher {
        // Maxn play shape: fwd-pruning + adaptive beam on (the established config every recorded
        // maxn number used — same as move_match / bench_beam). No node budget (deprecated, unsound
        // mid-rotation per EXP-012); the adaptive taper bounds the tree.
        Searcher::new(32)
            .with_beam_width(self.beam)
            .with_forward_pruning(true)
            .with_adaptive_beam(true)
            .with_ffa_bounty_order(self.bounty)
            .with_free_capture_order(self.freecap)
    }
}

fn xs(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// One game; `a_seats[i]` = seat i runs config A. Returns final points + # players eliminated.
fn play_game(
    a: &Cfg,
    b: &Cfg,
    a_seats: [bool; 4],
    seed: u64,
    opening: usize,
    cap: usize,
) -> ([u16; 4], usize) {
    let mut game = Game::from_start(seed);
    let mut searchers: Vec<Searcher> = (0..4)
        .map(|i| {
            if a_seats[i] {
                a.searcher()
            } else {
                b.searcher()
            }
        })
        .collect();
    let mut rng = seed | 1;
    for ply in 0..cap {
        if game.active_count() <= 1 {
            break;
        }
        let seat = game.board.side_to_move.index();
        if ply < opening {
            let r = xs(&mut rng) as usize;
            game.step(|bd| {
                let l = generate_legal(bd);
                (!l.is_empty()).then(|| l[r % l.len()])
            });
        } else {
            let depth = if a_seats[seat] { a.depth } else { b.depth };
            let sr = &mut searchers[seat];
            game.step(|bd| sr.search(bd, depth).map(|(m, _)| m));
        }
    }
    let eliminated = 4 - game.active_count();
    (game.points(), eliminated)
}

fn main() {
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let (a_beam, b_beam) = (arg(1).unwrap_or(4), arg(2).unwrap_or(4));
    let (a_bounty, a_freecap) = (arg(3).unwrap_or(0) != 0, arg(4).unwrap_or(0) != 0);
    let (b_bounty, b_freecap) = (arg(5).unwrap_or(0) != 0, arg(6).unwrap_or(0) != 0);
    let depth = arg(7).unwrap_or(8) as u32;
    let per_split = arg(8).unwrap_or(2);
    let flag = |t: bool| -> &'static str { if t { "1" } else { "0" } };
    let a = Cfg {
        label: format!(
            "A(d{depth} b{a_beam} bo{} fc{})",
            flag(a_bounty),
            flag(a_freecap)
        ),
        depth,
        beam: a_beam,
        bounty: a_bounty,
        freecap: a_freecap,
    };
    let b = Cfg {
        label: format!(
            "B(d{depth} b{b_beam} bo{} fc{})",
            flag(b_bounty),
            flag(b_freecap)
        ),
        depth,
        beam: b_beam,
        bounty: b_bounty,
        freecap: b_freecap,
    };
    let (opening, cap) = (12usize, 140usize);

    // The six balanced 2-vs-2 seat assignments (each seat is A in 3, B in 3).
    let splits: [[bool; 4]; 6] = [
        [true, true, false, false],
        [true, false, true, false],
        [true, false, false, true],
        [false, true, true, false],
        [false, true, false, true],
        [false, false, true, true],
    ];

    eprintln!(
        "=== maxn A/B: {} vs {} ({per_split} game(s)/split, {opening} random opening plies, {cap}-ply cap) ===",
        a.label, b.label
    );
    let start = Instant::now();
    let (mut a_pts, mut b_pts) = (0u64, 0u64);
    let (mut a_wins, mut games, mut decisive) = (0usize, 0usize, 0usize);
    for (si, a_seats) in splits.iter().enumerate() {
        for g in 0..per_split {
            // Same deterministic seed formula as selfplay_ab — identical openings across pairings.
            let seed = ((si * per_split + g) as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let (pts, elim) = play_game(&a, &b, *a_seats, seed, opening, cap);
            let ap: u64 = (0..4).filter(|&i| a_seats[i]).map(|i| pts[i] as u64).sum();
            let bp: u64 = (0..4).filter(|&i| !a_seats[i]).map(|i| pts[i] as u64).sum();
            a_pts += ap;
            b_pts += bp;
            games += 1;
            if ap > bp {
                a_wins += 1;
            }
            if elim > 0 {
                decisive += 1;
            }
            eprintln!(
                "game {games:2}: A {ap:3} - B {bp:3}  ({elim} eliminated)  pts {pts:?}  [{:.0}s]",
                start.elapsed().as_secs_f64()
            );
        }
    }
    let n = games as f64;
    eprintln!("\n=== {} vs {} over {games} games ===", a.label, b.label);
    eprintln!(
        "points: A {a_pts} vs B {b_pts}   (per seat: A {:.1}, B {:.1})",
        a_pts as f64 / (n * 2.0),
        b_pts as f64 / (n * 2.0)
    );
    eprintln!(
        "A win-rate (A-seats out-score B-seats): {a_wins}/{games} = {:.0}%",
        100.0 * a_wins as f64 / n
    );
    eprintln!(
        "decisive games (≥1 elimination): {decisive}/{games} = {:.0}%",
        100.0 * decisive as f64 / n
    );
}
