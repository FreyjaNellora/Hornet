//! Self-play A/B strength gate — the human-free way to ask "does config A beat config B?".
//!
//! In each game, config A holds 2 seats and config B the other 2, balanced over all six 2-vs-2 seat
//! splits so neither config gets a seat advantage. Games start from random openings; the score is each
//! config's summed FFA points (placement points, RBYG). Reports A-vs-B points, A's win-rate (A's two
//! seats out-scoring B's), and **decisiveness** (how many games actually reach an elimination) — which
//! is the whole point of a "play for the win" eval change.
//!
//! Configs are Searcher settings (depth, flashlight cap, and the search-side win-term weight), so A and
//! B differ within the same game. Used to gate the win term (win-on vs win-off) and to ask whether depth
//! pays once the search aims for the objective.
//!
//! Run: selfplay_ab [a_depth] [b_depth] [games] [cap] [a_win] [b_win] [a_danger] [b_danger]
//!   e.g. `selfplay_ab 8 8 1 1000 50 50 100 0` = d8 vs d8, cap 1000, both win-on(50),
//!        A king-danger(100) vs B danger-off — gates the points-aware safety rebuild.

use hornet_engine::game::Game;
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::time::Instant;

#[derive(Clone)]
struct Cfg {
    label: String,
    depth: u32,
    cap: usize,   // flashlight level-cap — the deep mechanism (move-stable, not the laser)
    win: i16,     // search-side win-term weight (0 = off; the FFA-points "goal" layer)
    wproxy: bool, // win signal: true = Kimi's elimination-proximity, false = banked points
    danger: i16,  // king-danger weight (0 = off; the points-aware safety rebuild)
    dtable: bool, // king-danger shape: true = Kimi's non-linear table, false = linear scalar
}
impl Cfg {
    fn searcher(&self) -> Searcher {
        Searcher::new(32)
            .with_win_term(self.win)
            .with_win_proxy(self.wproxy)
            .with_king_danger(self.danger)
            .with_danger_table(self.dtable)
    }
}

fn xs(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// One game; `a_seats[i]` = seat i runs config A. Returns final points + # players eliminated (dead).
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
            let (depth, cap) = if a_seats[seat] {
                (a.depth, a.cap)
            } else {
                (b.depth, b.cap)
            };
            let sr = &mut searchers[seat];
            game.step(|bd| sr.search_flashlight(bd, depth, |_| cap).map(|(m, _)| m));
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
    // args: a_depth b_depth games_per_split cap a_win b_win
    //   (cap = flashlight level-cap; a_win/b_win = win-term weight, 0 = off).
    // Defaults run the Phase-0 test: d8 vs d8, A win-on (w=100) vs B win-off (w=0).
    let cap = arg(4).unwrap_or(400);
    let (aw, bw) = (arg(5).unwrap_or(100) as i16, arg(6).unwrap_or(0) as i16);
    let (adg, bdg) = (arg(7).unwrap_or(0) as i16, arg(8).unwrap_or(0) as i16);
    let (adt, bdt) = (arg(9).unwrap_or(0) != 0, arg(10).unwrap_or(0) != 0);
    let (awp, bwp) = (arg(11).unwrap_or(0) != 0, arg(12).unwrap_or(0) != 0);
    let (ad, bd) = (arg(1).unwrap_or(8) as u32, arg(2).unwrap_or(8) as u32);
    let tag = |t: bool, c: &'static str| -> &'static str { if t { c } else { "" } };
    let a = Cfg {
        label: format!(
            "A(d{ad} c{cap} w{aw}{} k{adg}{})",
            tag(awp, "P"),
            tag(adt, "T")
        ),
        depth: ad,
        cap,
        win: aw,
        wproxy: awp,
        danger: adg,
        dtable: adt,
    };
    let b = Cfg {
        label: format!(
            "B(d{bd} c{cap} w{bw}{} k{bdg}{})",
            tag(bwp, "P"),
            tag(bdt, "T")
        ),
        depth: bd,
        cap,
        win: bw,
        wproxy: bwp,
        danger: bdg,
        dtable: bdt,
    };
    let per_split = arg(3).unwrap_or(1);
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

    let start = Instant::now();
    let (mut a_pts, mut b_pts) = (0u64, 0u64);
    let (mut a_wins, mut games, mut decisive) = (0usize, 0usize, 0usize);
    for (si, a_seats) in splits.iter().enumerate() {
        for g in 0..per_split {
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
