//! Behavioral mining (the user mandate): study the human corpus for **winner-vs-loser
//! differential behavior**, as raw material for representations placed where they fit
//! (eval / search / ordering / objective layer). The PST-v3 precedent: zone/visit mining →
//! table values → eval.
//!
//! Studies (winners = finished 1st/2nd; phases: early <20 plies, mid <100, late ≥100):
//! 1. **Capture targeting** — when a player captures, where does the victim stand in the
//!    *current* points race (leader / middle / trailing among opponents)? Phase-split.
//! 2. **Capture answer rate** — was the capturing piece itself captured on that square within
//!    the next rotation (≤4 plies)? "Do winners take better trades or just more?" (square+window
//!    proxy: if the capturer moves away first, a later capture there is miscounted — noted.)
//! 3. **Piece destinations by zone, player-relative** — destinations rotated into Red's frame
//!    (true C4 rotations; NB `queries::pst_value`'s Green branch is a *transpose*, harmless for
//!    today's transpose-symmetric zone PST but not a true rotation — mining uses the real one).
//!
//! Run: cargo run --release --example behavior_mine [-- dir]   (default ../human_games)

use hornet_engine::board::Square;
use hornet_engine::board::pgn4::{self, result_points, winner_seats};
use hornet_engine::board::types::{PieceType, Player};
use hornet_engine::replay::{ReplayState, resolve_ply};
use hornet_engine::zones::ZONES;
use std::fs;
use std::path::PathBuf;

const NZONES: usize = 10; // 9 named zones + "other"
const NPHASE: usize = 3; // early / mid / late

fn zone_of(sq: Square) -> usize {
    for (zi, z) in ZONES.iter().enumerate() {
        if z.squares().into_iter().any(|s| s == sq) {
            return zi;
        }
    }
    9
}

/// Rotate `sq` from `player`'s perspective into Red's frame (true C4 board symmetry).
fn red_frame(sq: Square, player: Player) -> Square {
    let (r, f) = (sq.rank(), sq.file());
    match player {
        Player::Red => sq,
        Player::Blue => Square::from_rank_file(13 - f, r),
        Player::Yellow => Square::from_rank_file(13 - r, 13 - f),
        Player::Green => Square::from_rank_file(f, 13 - r),
    }
}

fn phase_of(ply: usize) -> usize {
    if ply < 20 {
        0
    } else if ply < 100 {
        1
    } else {
        2
    }
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

    // [winner-class][phase][victim-standing]: 0 = victim leads the mover's other opponents,
    // 1 = middle, 2 = trailing. (Standing by current points at the moment of capture.)
    let mut target = [[[0u32; 3]; NPHASE]; 2];
    // [winner-class][piece][zone] move-destination counts, destinations in RED-FRAME.
    let mut dest = [[[0u32; NZONES]; 6]; 2];
    // [winner-class][phase]: (captures, captures answered on-square within ≤4 plies).
    let mut answer = [[(0u32, 0u32); NPHASE]; 2];
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
        let mut ply = 0usize;
        // Pending captures awaiting an "answer": (applied-ply, square index, wclass, phase).
        let mut pending: Vec<(usize, u8, usize, usize)> = Vec::new();
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
                let phase = phase_of(ply);
                if let Some(p) = board.piece_at(mv.from) {
                    dest[wclass][pt_idx(p.piece_type)][zone_of(red_frame(mv.to, mover))] += 1;
                }
                if mv.flags.capture
                    && let Some(victim) = board.piece_at(mv.to)
                    && victim.player != mover
                {
                    // Victim's standing among the mover's opponents, by current points.
                    let vp = board.points[victim.player.index()];
                    let above = mover
                        .opponents()
                        .iter()
                        .filter(|o| board.points[o.index()] > vp)
                        .count();
                    target[wclass][phase][above.min(2)] += 1;

                    // Answer tracking: does this capture answer a pending one on this square?
                    let sq_idx = mv.to.index();
                    if let Some(pos) = pending
                        .iter()
                        .position(|&(p0, s, _, _)| s == sq_idx && ply - p0 <= 4)
                    {
                        let (_, _, w0, ph0) = pending.swap_remove(pos);
                        answer[w0][ph0].1 += 1; // the earlier capture got answered
                    }
                    answer[wclass][phase].0 += 1;
                    pending.push((ply, sq_idx, wclass, phase));
                }
                pending.retain(|&(p0, _, _, _)| ply.saturating_sub(p0) <= 4);
                board.make_move(mv);
                ply += 1;
            }
        }
    }

    println!("behavior_mine over {games} games ({})", dir.display());
    let phn = ["early", "mid  ", "late "];
    println!(
        "\n== capture targeting by phase: victim's CURRENT points standing among opponents =="
    );
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        for ph in 0..NPHASE {
            let tot: u32 = target[w][ph].iter().sum();
            if tot == 0 {
                continue;
            }
            println!(
                "{name} {}: leader {:4.1}% | middle {:4.1}% | trailing {:4.1}%   ({tot})",
                phn[ph],
                100.0 * target[w][ph][0] as f64 / tot as f64,
                100.0 * target[w][ph][1] as f64 / tot as f64,
                100.0 * target[w][ph][2] as f64 / tot as f64,
            );
        }
    }

    println!("\n== capture answer rate (capturer captured on-square within ≤4 plies) ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let line: Vec<String> = (0..NPHASE)
            .filter(|&ph| answer[w][ph].0 > 0)
            .map(|ph| {
                format!(
                    "{} {:.1}% ({}/{})",
                    phn[ph],
                    100.0 * answer[w][ph].1 as f64 / answer[w][ph].0 as f64,
                    answer[w][ph].1,
                    answer[w][ph].0
                )
            })
            .collect();
        println!("{name}: {}", line.join(" | "));
    }

    println!("\n== move destinations by RED-FRAME zone (winner% − loser%, per piece type) ==");
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
