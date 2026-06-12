//! Behavioral mining (the user mandate): study the human corpus for **winner-vs-loser
//! differential behavior**, as raw material for representations placed where they fit
//! (eval / search / ordering / objective layer). The PST-v3 precedent: zone/visit mining →
//! table values → eval.
//!
//! Studies in this pass (winners = finished 1st/2nd):
//! 1. **Capture targeting** — when a player captures, where does the victim stand in the
//!    *current* points race (leader / middle / trailing among opponents)? "Whom do winners
//!    attack?" → objective-layer / ordering material.
//! 2. **Piece destinations by zone** — per piece type, which board zones do moves land in?
//!    Winner-vs-loser deltas → PST/zone-table material.
//!
//! Run: cargo run --release --example behavior_mine [-- dir]   (default ../human_games)

use hornet_engine::board::pgn4::{self, result_points, winner_seats};
use hornet_engine::board::types::PieceType;
use hornet_engine::replay::{ReplayState, resolve_ply};
use hornet_engine::zones::ZONES;
use std::fs;
use std::path::PathBuf;

const NZONES: usize = 10; // 9 named zones + "other"

fn zone_of(sq: hornet_engine::board::Square) -> usize {
    for (zi, z) in ZONES.iter().enumerate() {
        if z.squares().into_iter().any(|s| s == sq) {
            return zi;
        }
    }
    9
}

fn pt_idx(t: PieceType) -> usize {
    match t {
        PieceType::Pawn => 0,
        PieceType::Knight => 1,
        PieceType::Bishop => 2,
        PieceType::Rook => 3,
        PieceType::Queen | PieceType::PromotedQueen => 4,
        PieceType::King => 5,
    }
}

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let dir = std::env::args()
        .nth(1)
        .map(|a| base.join(a))
        .unwrap_or(base.join("human_games"));

    // [winner-class][victim-standing]: standing 0 = victim leads the mover's other opponents,
    // 1 = middle, 2 = trailing. (Standing by current points at the moment of capture.)
    let mut target = [[0u32; 3]; 2];
    // [winner-class][piece][zone] move-destination counts.
    let mut dest = [[[0u32; NZONES]; 6]; 2];
    let mut games = 0usize;

    for entry in fs::read_dir(&dir).expect("games dir") {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let Some(final_pts) = result_points(&text) else {
            continue;
        };
        let winners = winner_seats(final_pts);
        let Ok(game) = pgn4::parse(&text) else {
            continue;
        };
        let Ok(mut board) = game.initial_board() else {
            continue;
        };
        games += 1;
        let mut st = ReplayState::default();
        'game: for round in &game.rounds {
            for tok in &round.plies {
                let Some(mv) = resolve_ply(&mut board, tok, &mut st) else {
                    if pgn4::decode_ply(tok).is_some() {
                        break 'game; // real divergence — stop this game
                    }
                    continue; // marker token
                };
                let mover = board.side_to_move;
                let wclass = usize::from(!winners[mover.index()]); // 0 = winner, 1 = loser
                if let Some(p) = board.piece_at(mv.from) {
                    dest[wclass][pt_idx(p.piece_type)][zone_of(mv.to)] += 1;
                }
                if mv.flags.capture
                    && let Some(victim) = board.piece_at(mv.to)
                    && victim.player != mover
                {
                    // Victim's standing among the mover's opponents, by current points.
                    let vp = board.points[victim.player.index()];
                    let opp_pts: Vec<u16> = mover
                        .opponents()
                        .iter()
                        .map(|o| board.points[o.index()])
                        .collect();
                    let above = opp_pts.iter().filter(|&&x| x > vp).count();
                    let standing = above.min(2); // 0 leader, 1 middle, 2 trailing
                    target[wclass][standing] += 1;
                }
                board.make_move(mv);
            }
        }
    }

    println!("behavior_mine over {games} games ({})", dir.display());
    println!("\n== capture targeting: victim's CURRENT points standing among mover's opponents ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let tot: u32 = target[w].iter().sum();
        if tot == 0 {
            continue;
        }
        println!(
            "{name}: leader {:4.1}% | middle {:4.1}% | trailing {:4.1}%   ({tot} captures)",
            100.0 * target[w][0] as f64 / tot as f64,
            100.0 * target[w][1] as f64 / tot as f64,
            100.0 * target[w][2] as f64 / tot as f64,
        );
    }

    println!("\n== move destinations by zone (winner% − loser%, per piece type) ==");
    let zname = |zi: usize| -> &str { if zi == 9 { "other" } else { ZONES[zi].name } };
    let pname = ["pawn", "knight", "bishop", "rook", "queen", "king"];
    for pt in 0..6 {
        let wt: u32 = dest[0][pt].iter().sum();
        let lt: u32 = dest[1][pt].iter().sum();
        if wt == 0 || lt == 0 {
            continue;
        }
        let mut deltas: Vec<(usize, f64, f64, f64)> = (0..NZONES)
            .map(|z| {
                let wp = 100.0 * dest[0][pt][z] as f64 / wt as f64;
                let lp = 100.0 * dest[1][pt][z] as f64 / lt as f64;
                (z, wp, lp, wp - lp)
            })
            .collect();
        deltas.sort_by(|a, b| b.3.abs().partial_cmp(&a.3.abs()).unwrap());
        let top: Vec<String> = deltas
            .iter()
            .take(3)
            .filter(|d| d.3.abs() >= 0.5)
            .map(|d| {
                format!(
                    "{} {:+.1}pp (w {:.1}% / l {:.1}%)",
                    zname(d.0),
                    d.3,
                    d.1,
                    d.2
                )
            })
            .collect();
        if !top.is_empty() {
            println!("{:7}: {}", pname[pt], top.join(" | "));
        }
    }
}
