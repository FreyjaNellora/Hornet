//! Move divergence between two ordering configs over the human corpus (EXP-020).
//!
//! `move_match` arm deltas only bound the *net* change in human-agreement; two configs can have
//! near-equal match rates while choosing different moves on many positions. This harness replays
//! the same sampled corpus positions and runs **both** configs' searchers on each, reporting how
//! often the chosen move differs — the direct per-position behavior-change (contamination)
//! frequency of an ordering lever.
//!
//! Run: cargo run --release --example move_diverge [-- beam depth sample a_bounty a_freecap b_bounty b_freecap]
//!   Defaults `4 4 2 1 1 1 0` (arm (i) vs arm (ii) at beam 4 — the EXP-020 contamination pairing).

use hornet_engine::board::pgn4;
use hornet_engine::replay::{ReplayState, resolve_ply};
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let beam = arg(1).unwrap_or(4);
    let depth = arg(2).unwrap_or(4) as u32;
    let sample = arg(3).unwrap_or(2).max(1);
    let (a_bounty, a_freecap) = (arg(4).unwrap_or(1) != 0, arg(5).unwrap_or(1) != 0);
    let (b_bounty, b_freecap) = (arg(6).unwrap_or(1) != 0, arg(7).unwrap_or(0) != 0);
    let searcher = |bounty: bool, freecap: bool| {
        Searcher::new(16)
            .with_beam_width(beam)
            .with_forward_pruning(true)
            .with_adaptive_beam(true)
            .with_ffa_bounty_order(bounty)
            .with_free_capture_order(freecap)
    };
    let (mut total, mut differ, mut games) = (0usize, 0usize, 0usize);

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
        games += 1;
        let mut ply = 0usize;
        let mut st = ReplayState::default();
        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(human) = resolve_ply(&mut board, tok, &mut st) else {
                    break 'game;
                };
                if ply % sample == 0 {
                    let mut sa = searcher(a_bounty, a_freecap);
                    let mut sb = searcher(b_bounty, b_freecap);
                    if let (Some((ma, _)), Some((mb, _))) =
                        (sa.search(&mut board, depth), sb.search(&mut board, depth))
                    {
                        total += 1;
                        if ma.from != mb.from || ma.to != mb.to {
                            differ += 1;
                        }
                    }
                }
                board.make_move(human);
                ply += 1;
            }
        }
    }
    let rate = if total > 0 {
        100.0 * differ as f64 / total as f64
    } else {
        0.0
    };
    eprintln!(
        "move-diverge: {differ}/{total} = {rate:.1}%  over {games} games (beam {beam}, depth {depth}, every {sample} plies, A bo{} fc{} vs B bo{} fc{})",
        a_bounty as u8, a_freecap as u8, b_bounty as u8, b_freecap as u8
    );
}
