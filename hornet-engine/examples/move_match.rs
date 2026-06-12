//! Move-match: how often the engine's top move equals the human's actual move, over the human corpus.
//! A DENSE, sensitive eval-quality instrument — one datapoint per move (~1000s per corpus), unlike
//! outcome-MSE (one noisy datapoint per game, swamped by the 16-game noise floor). EXP-012 showed the
//! eval IS the move-chooser, so this measures the thing that matters, and it reads *positional*
//! decisions in quiet positions (which MSE and the tactical fixtures both miss). Use it to gate eval
//! changes: a real eval improvement should lift the match rate.
//!
//! Run: cargo run --release --example move_match [-- beam depth sample bounty freecap eval fcap]
//!   Defaults `10 4 2 0 0 0 0` (the historical instrument config, ordering levers off).
//!   bounty/freecap = the EXP-020 ordering levers (1 = on); fwd-pruning/adaptive stay fixed on.
//!   eval = leaf eval (EXP-029): 0 deployed, 1 P′ (iso), 2 S′ (danger-table).
//!   fcap > 0 = use the FLASHLIGHT at that per-level cap instead of beam Max^n (the depth-pays
//!   shape per EXP-017; `beam` is ignored). For d4-vs-d8 horizon comparisons.
//!
//! Reports two rates: **all** moves, and **winners-only** (agreement counted only on moves
//! played by players who finished 1st/2nd — "imitate winners, not losers"; blunder-prone
//! losing play stops polluting the target).

use hornet_engine::board::pgn4;
use hornet_engine::replay::{ReplayState, resolve_ply};
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;

/// Parse `[Result "name: pts - ..."]` → [R,B,Y,G] points (seat-joined via the player headers;
/// positional fallback). Mirrors texel_tune's parser.
fn parse_result_points(text: &str) -> Option<[f64; 4]> {
    let header = |tag: &str| -> Option<String> {
        text.lines()
            .find(|l| l.starts_with(&format!("[{tag} ")))
            .and_then(|l| l.split('"').nth(1))
            .map(|s| s.to_string())
    };
    let seats = [
        header("Red"),
        header("Blue"),
        header("Yellow"),
        header("Green"),
    ];
    let line = text.lines().find(|l| l.starts_with("[Result"))?;
    let mut pairs: Vec<(String, f64)> = Vec::new();
    for part in line.split(" - ") {
        let Some(c) = part.rfind(": ") else { continue };
        let name = part[..c]
            .trim_start_matches("[Result")
            .trim()
            .trim_start_matches('"')
            .trim()
            .to_string();
        let num: String = part[c + 2..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
            .collect();
        if let Ok(n) = num.trim().parse::<f64>() {
            pairs.push((name, n));
        }
    }
    if pairs.len() != 4 {
        return None;
    }
    if seats.iter().all(|s| s.is_some()) {
        let seat_names: Vec<&str> = seats.iter().map(|s| s.as_deref().unwrap()).collect();
        let mut pts = [f64::NAN; 4];
        for (nm, n) in &pairs {
            if let Some(i) = seat_names.iter().position(|s| s == nm) {
                pts[i] = *n;
            }
        }
        if pts.iter().all(|p| !p.is_nan()) {
            return Some(pts);
        }
    }
    Some([pairs[0].1, pairs[1].1, pairs[2].1, pairs[3].1])
}

/// Which players finished 1st or 2nd (ties resolved generously — a tie for 2nd counts).
fn winners(pts: [f64; 4]) -> [bool; 4] {
    std::array::from_fn(|i| {
        let strictly_above = (0..4).filter(|&j| pts[j] > pts[i]).count();
        strictly_above < 2
    })
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let beam = arg(1).unwrap_or(10);
    let depth = arg(2).unwrap_or(4) as u32; // shallow: EXP-012 says depth barely moves the choice.
    let sample = arg(3).unwrap_or(2).max(1); // every Nth ply
    let bounty = arg(4).unwrap_or(0) != 0;
    let freecap = arg(5).unwrap_or(0) != 0;
    let eval_id = arg(6).unwrap_or(0);
    let fcap = arg(7).unwrap_or(0);
    let (mut total, mut matched, mut games) = (0usize, 0usize, 0usize);
    let (mut w_total, mut w_matched) = (0usize, 0usize);

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let Ok(game) = pgn4::parse(&text) else {
            continue;
        };
        let Ok(mut board) = game.initial_board() else {
            continue;
        };
        // Winners-only weighting: which seats finished 1st/2nd (None → count all moves only).
        let win_seats = parse_result_points(&text).map(winners);
        games += 1;
        let mut ply = 0usize;
        let mut st = ReplayState::default();
        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve_ply(&mut board, tok, &mut st) else {
                    break 'game; // replay diverged (elimination) — stop this game
                };
                if ply % sample == 0 {
                    let mut s = Searcher::new(16)
                        .with_beam_width(beam)
                        .with_forward_pruning(true)
                        .with_adaptive_beam(true)
                        .with_ffa_bounty_order(bounty)
                        .with_free_capture_order(freecap);
                    s = match eval_id {
                        1 => s.with_eval(hornet_engine::eval::eval_4vec_pprime),
                        2 => s.with_eval(hornet_engine::eval::eval_4vec_sprime),
                        _ => s,
                    };
                    let result = if fcap > 0 {
                        s.search_flashlight(&board, depth, |_| fcap)
                    } else {
                        s.search(&mut board, depth)
                    };
                    if let Some((eng, _)) = result {
                        // `resolve` self-synced side_to_move to the human mover.
                        let mover_won = win_seats.is_some_and(|w| w[board.side_to_move.index()]);
                        let hit = eng.from == human.from && eng.to == human.to;
                        total += 1;
                        if hit {
                            matched += 1;
                        }
                        if mover_won {
                            w_total += 1;
                            if hit {
                                w_matched += 1;
                            }
                        }
                    }
                }
                board.make_move(human);
                ply += 1;
            }
        }
    }
    let pct = |m: usize, t: usize| {
        if t > 0 {
            100.0 * m as f64 / t as f64
        } else {
            0.0
        }
    };
    eprintln!(
        "move-match: all {matched}/{total} = {:.1}% | winners-only {w_matched}/{w_total} = {:.1}%  over {games} games (beam {beam}, depth {depth}, every {sample} plies, bounty={} freecap={} eval={eval_id} fcap={fcap})",
        pct(matched, total),
        pct(w_matched, w_total),
        bounty as u8,
        freecap as u8
    );
}
