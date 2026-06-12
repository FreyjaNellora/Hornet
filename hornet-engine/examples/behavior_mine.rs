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
//! 4. **Development order** — the own-move index of each piece type's FIRST move, plus the
//!    piece-type share of each player's first 8 own moves (does the queen really come out first?).
//! 5. **Promotions** — rate per seat-game, mean ply, and capture rates of promoted vs original
//!    queens (is the promoted queen actually the raid weapon?).
//! 6. **King-raid proxy** — share of move destinations within Chebyshev ≤2 of an enemy king,
//!    phase-split, plus king captures (DKW kings) as a share of all captures.
//! 7. **Promotion denial** — when the victim is an enemy pawn, how advanced was it (own-frame
//!    rank progress), and what share of pawn-victims were advanced (progress ≥6)? Do winners
//!    guard the crossing by killing runners, or just out-race them?
//! 8. **Elimination forensics** — at each KING capture (the final kill — exact attribution,
//!    unlike DKW entry which the replayer infers lazily): the victim's points rank and material
//!    rank among the 4 at that moment, and the rotation offset killer→victim. "Systematic
//!    elimination based on what?"
//! 9. **Capture profitability** — SEE of the captured square (`queries::see_capture`, attacker
//!    best-case) at the moment of each human capture, plus an overpay marker (mover worth more
//!    than victim). "Do winners take better-valued trades, or just more of them?"
//!
//! Run: cargo run --release --example behavior_mine [-- dir]   (default ../human_games)

use hornet_engine::board::{Board, Square};
use hornet_engine::board::pgn4::{self, result_points, winner_seats};
use hornet_engine::board::types::{PieceType, Player};
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::queries::see_capture;
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

/// Classical-value material count per player (mining heuristic: P1 N3 B3 R5 Q9, kings 0).
fn material(board: &Board) -> [i32; 4] {
    let mut m = [0i32; 4];
    for rank in 0..14u8 {
        for file in 0..14u8 {
            let sq = Square::from_rank_file(rank, file);
            if !sq.is_valid() {
                continue;
            }
            if let Some(p) = board.piece_at(sq) {
                m[p.player.index()] += match p.piece_type {
                    PieceType::Pawn => 1,
                    PieceType::Knight | PieceType::Bishop => 3,
                    PieceType::Rook => 5,
                    PieceType::Queen | PieceType::PromotedQueen => 9,
                    PieceType::King => 0,
                };
            }
        }
    }
    m
}

/// Does `to` land within Chebyshev distance 2 of any enemy king still on the board
/// (live or DKW — pressure on a walking king is the raid being finished)?
fn near_enemy_king(board: &Board, to: Square, mover: Player) -> bool {
    for rank in 0..14u8 {
        for file in 0..14u8 {
            let sq = Square::from_rank_file(rank, file);
            if !sq.is_valid() {
                continue;
            }
            if let Some(p) = board.piece_at(sq)
                && p.piece_type == PieceType::King
                && p.player != mover
            {
                let dr = (i32::from(to.rank()) - i32::from(rank)).abs();
                let df = (i32::from(to.file()) - i32::from(file)).abs();
                if dr.max(df) <= 2 {
                    return true;
                }
            }
        }
    }
    false
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
    // [winner-class][piece]: (sum of first-move own-move index, seats where that type moved).
    let mut first_mv = [[(0u64, 0u32); 6]; 2];
    // [winner-class][piece]: moves within each seat's first 8 own moves.
    let mut early_profile = [[0u32; 6]; 2];
    // [winner-class]: (promotions, sum of promotion plies); seat-games per class.
    let mut promo = [(0u32, 0u64); 2];
    let mut seatgames = [0u32; 2];
    // [winner-class]: (promoted-Q moves, promoted-Q captures, original-Q moves, original-Q captures).
    let mut qcap = [(0u32, 0u32, 0u32, 0u32); 2];
    // [winner-class][phase]: (destinations within Cheb ≤2 of an enemy king, total moves).
    let mut kprox = [[(0u32, 0u32); NPHASE]; 2];
    // [winner-class]: (king captures, total captures).
    let mut kcap = [(0u32, 0u32); 2];
    // [winner-class]: (pawn-victim count, sum of victims' own-frame rank progress, advanced ≥6).
    let mut denial = [(0u32, 0u64, 0u32); 2];
    // Elimination forensics at king captures: victim's rank (0 = top) by points / material,
    // killer→victim rotation offset (1 = victim moves right after the killer), killer class.
    let (mut elim_n, mut elim_by_winner) = (0u32, 0u32);
    let mut elim_pts_rank = [0u32; 4];
    let mut elim_mat_rank = [0u32; 4];
    let mut elim_offset = [0u32; 4];
    // [winner-class][phase]: (captures, SEE>0, SEE cp sum, overpay (mover>victim), victim cp sum).
    let mut profit = [[(0u32, 0u32, 0i64, 0u32, 0i64); NPHASE]; 2];
    let mut lm = LineMap::new();
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
        for seat in 0..4 {
            seatgames[usize::from(!winners[seat])] += 1;
        }
        let mut st = ReplayState::default();
        let mut ply = 0usize;
        let mut own_moves = [0u32; 4];
        let mut first_seen: [[Option<u32>; 6]; 4] = [[None; 6]; 4];
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
                own_moves[mover.index()] += 1;
                let omi = own_moves[mover.index()];
                if let Some(p) = board.piece_at(mv.from) {
                    let pt = pt_idx(p.piece_type);
                    dest[wclass][pt][zone_of(red_frame(mv.to, mover))] += 1;
                    if first_seen[mover.index()][pt].is_none() {
                        first_seen[mover.index()][pt] = Some(omi);
                    }
                    if omi <= 8 {
                        early_profile[wclass][pt] += 1;
                    }
                    match p.piece_type {
                        PieceType::PromotedQueen => {
                            qcap[wclass].0 += 1;
                            if mv.flags.capture {
                                qcap[wclass].1 += 1;
                            }
                        }
                        PieceType::Queen => {
                            qcap[wclass].2 += 1;
                            if mv.flags.capture {
                                qcap[wclass].3 += 1;
                            }
                        }
                        _ => {}
                    }
                }
                if mv.promotion.is_some() {
                    promo[wclass].0 += 1;
                    promo[wclass].1 += ply as u64;
                }
                kprox[wclass][phase].1 += 1;
                if near_enemy_king(&board, mv.to, mover) {
                    kprox[wclass][phase].0 += 1;
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

                    // Profitability: SEE of the captured square as the position stands.
                    compute_lines(&board, &mut lm);
                    let vval = victim.piece_type.eval_value();
                    let see = see_capture(&lm, mv.to, vval, victim.player, mover);
                    let e = &mut profit[wclass][phase];
                    e.0 += 1;
                    if see > 0 {
                        e.1 += 1;
                    }
                    e.2 += i64::from(see);
                    if let Some(p) = board.piece_at(mv.from)
                        && p.piece_type.eval_value() > vval
                    {
                        e.3 += 1;
                    }
                    e.4 += i64::from(vval);

                    kcap[wclass].1 += 1;
                    match victim.piece_type {
                        PieceType::King => {
                            kcap[wclass].0 += 1;
                            // Forensics: standing/material BEFORE the kill applies.
                            elim_n += 1;
                            if winners[mover.index()] {
                                elim_by_winner += 1;
                            }
                            let v = victim.player.index();
                            let prank = (0..4)
                                .filter(|&o| o != v && board.points[o] > board.points[v])
                                .count();
                            let mat = material(&board);
                            let mrank = (0..4).filter(|&o| o != v && mat[o] > mat[v]).count();
                            elim_pts_rank[prank] += 1;
                            elim_mat_rank[mrank] += 1;
                            elim_offset[(v + 4 - mover.index()) % 4] += 1;
                        }
                        PieceType::Pawn => {
                            let prog = red_frame(mv.to, victim.player).rank();
                            denial[wclass].0 += 1;
                            denial[wclass].1 += u64::from(prog);
                            if prog >= 6 {
                                denial[wclass].2 += 1;
                            }
                        }
                        _ => {}
                    }

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
        for seat in 0..4 {
            let w = usize::from(!winners[seat]);
            for pt in 0..6 {
                if let Some(i) = first_seen[seat][pt] {
                    first_mv[w][pt].0 += u64::from(i);
                    first_mv[w][pt].1 += 1;
                }
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

    println!("\n== development: mean own-move index of a piece type's FIRST move ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let line: Vec<String> = (0..6)
            .filter(|&pt| first_mv[w][pt].1 > 0)
            .map(|pt| {
                format!(
                    "{} {:.1} (n={})",
                    pname[pt],
                    first_mv[w][pt].0 as f64 / f64::from(first_mv[w][pt].1),
                    first_mv[w][pt].1
                )
            })
            .collect();
        println!("{name}: {}", line.join(" | "));
    }

    println!("\n== development profile: piece-type share of each player's first 8 own moves ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let tot: u32 = early_profile[w].iter().sum();
        if tot == 0 {
            continue;
        }
        let line: Vec<String> = (0..6)
            .map(|pt| {
                format!(
                    "{} {:.1}%",
                    pname[pt],
                    100.0 * f64::from(early_profile[w][pt]) / f64::from(tot)
                )
            })
            .collect();
        println!("{name}: {}   ({tot} moves)", line.join(" | "));
    }

    println!("\n== promotions ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let (n, plysum) = promo[w];
        let (pqm, pqc, qm, qc) = qcap[w];
        println!(
            "{name}: {:.2}/seat-game ({n} in {} seat-games), mean ply {:.0} | capture rate: promoted-Q {:.1}% ({pqc}/{pqm}), original-Q {:.1}% ({qc}/{qm})",
            f64::from(n) / f64::from(seatgames[w].max(1)),
            seatgames[w],
            if n > 0 { plysum as f64 / f64::from(n) } else { 0.0 },
            100.0 * f64::from(pqc) / f64::from(pqm.max(1)),
            100.0 * f64::from(qc) / f64::from(qm.max(1)),
        );
    }

    println!("\n== king-raid proxy: destinations within Cheb ≤2 of an enemy king ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let line: Vec<String> = (0..NPHASE)
            .filter(|&ph| kprox[w][ph].1 > 0)
            .map(|ph| {
                format!(
                    "{} {:.1}% ({}/{})",
                    phn[ph],
                    100.0 * f64::from(kprox[w][ph].0) / f64::from(kprox[w][ph].1),
                    kprox[w][ph].0,
                    kprox[w][ph].1
                )
            })
            .collect();
        let (kc, allc) = kcap[w];
        println!(
            "{name}: {} | king captures {:.1}% of captures ({kc}/{allc})",
            line.join(" | "),
            100.0 * f64::from(kc) / f64::from(allc.max(1))
        );
    }

    println!("\n== promotion denial: pawn capture victims, own-frame rank progress ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let (n, progsum, adv) = denial[w];
        if n == 0 {
            continue;
        }
        println!(
            "{name}: mean victim progress {:.2} | advanced (rank ≥6) {:.1}% ({adv}/{n})",
            progsum as f64 / f64::from(n),
            100.0 * f64::from(adv) / f64::from(n)
        );
    }

    println!("\n== capture profitability (SEE of the captured square; overpay = mover worth > victim) ==");
    for (w, name) in [(0usize, "winners"), (1, "losers ")] {
        let line: Vec<String> = (0..NPHASE)
            .filter(|&ph| profit[w][ph].0 > 0)
            .map(|ph| {
                let (n, pos, ssum, over, vsum) = profit[w][ph];
                format!(
                    "{}: SEE>0 {:.1}% | mean SEE {:+.0}cp | overpay {:.1}% | mean victim {:.0}cp ({n})",
                    phn[ph],
                    100.0 * f64::from(pos) / f64::from(n),
                    ssum as f64 / f64::from(n),
                    100.0 * f64::from(over) / f64::from(n),
                    vsum as f64 / f64::from(n)
                )
            })
            .collect();
        println!("{name}: {}", line.join("\n         "));
    }

    println!("\n== elimination forensics (at king capture; {elim_n} kills, {elim_by_winner} by winners) ==");
    if elim_n > 0 {
        let pct = |a: &[u32; 4]| -> String {
            (0..4)
                .map(|r| format!("{:.0}%", 100.0 * f64::from(a[r]) / f64::from(elim_n)))
                .collect::<Vec<_>>()
                .join(" / ")
        };
        println!("victim points rank   (top/2nd/3rd/last): {}", pct(&elim_pts_rank));
        println!("victim material rank (top/2nd/3rd/last): {}", pct(&elim_mat_rank));
        println!(
            "killer→victim rotation offset (+1 victim moves next / +2 across / +3 victim moved before): {} / {} / {}",
            elim_offset[1], elim_offset[2], elim_offset[3]
        );
    }
}
