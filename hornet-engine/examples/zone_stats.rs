//! Empirical study of the nine "secondary zones" (36 squares) over the 16 human PGN4 games.
//!
//! Replays each game against the move generator (self-syncing on the mover, like the replay
//! test) and, at every reached position, measures per-zone: occupancy, reach/control, entries
//! (moves landing in the zone), captures on the zone, friendly-defender support, per-seat usage,
//! and an inter-zone reach matrix (which zone's pieces project into which other zone).
//!
//! Run: cargo run --release --example zone_stats

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::{Player, Square};
use hornet_engine::board::{Board, Move};
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const ZONES: [(&str, &str, [&str; 4]); 9] = [
    ("center ", "center", ["g7", "h7", "g8", "h8"]),
    ("gate_W ", "gate", ["c7", "d7", "c8", "d8"]),
    ("gate_E ", "gate", ["k7", "l7", "k8", "l8"]),
    ("gate_S ", "gate", ["g3", "h3", "g4", "h4"]),
    ("gate_N ", "gate", ["g11", "h11", "g12", "h12"]),
    ("quad_SW", "quad", ["e5", "f5", "e6", "f6"]),
    ("quad_SE", "quad", ["i5", "j5", "i6", "j6"]),
    ("quad_NW", "quad", ["e9", "f9", "e10", "f10"]),
    ("quad_NE", "quad", ["i9", "j9", "i10", "j10"]),
];

const PNAME: [&str; 4] = ["R", "B", "Y", "G"];

fn sq(s: &str) -> Square {
    Square::from_algebraic(s).unwrap()
}

/// Decode + match + apply one ply; return the applied Move (mirrors the replay harness).
fn apply_ply(token: &str, board: &mut Board) -> Option<Move> {
    let decoded = pgn4::decode_ply(token)?;
    let mv = match decoded {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let p = board.piece_at(from)?;
            board.side_to_move = p.player;
            generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion)
        }
        DecodedMove::Castle { kingside } => {
            let mut found = None;
            for pl in Player::ALL {
                board.side_to_move = pl;
                let dest = castle_king_destination(pl, kingside);
                if let Some(m) = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.flags.castle && m.to == dest)
                {
                    found = Some(m);
                    break;
                }
            }
            found
        }
    };
    let m = mv?;
    board.make_move(m);
    Some(m)
}

fn main() {
    // slot 0..36 = zone*4 + k; map square index -> slot.
    let mut slot_square = [Square::new(0); 36];
    let mut sq_to_slot = [usize::MAX; 196];
    for (zi, (_, _, squares)) in ZONES.iter().enumerate() {
        for (k, s) in squares.iter().enumerate() {
            let slot = zi * 4 + k;
            let square = sq(s);
            slot_square[slot] = square;
            sq_to_slot[square.index() as usize] = slot;
        }
    }
    let zone_of = |sq_idx: usize| -> Option<usize> {
        let s = sq_to_slot[sq_idx];
        if s == usize::MAX { None } else { Some(s / 4) }
    };

    // Accumulators.
    let mut total_plies: u64 = 0;
    let mut occ = [0u64; 36];
    let mut occ_pl = [[0u64; 4]; 36];
    let mut reach_pl = [[0u64; 4]; 36];
    let mut entries = [0u64; 36];
    let mut entries_pl = [[0u64; 4]; 36];
    let mut caps = [0u64; 36];
    let mut def_sum = [0u64; 36]; // friendly defenders of occupied zone squares
    let mut def_cnt = [0u64; 36];
    let mut support = [[0u64; 9]; 9]; // support[A][B] = piece sitting in zone A reaches zone B

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("pgn4"))
        .collect();
    files.sort();

    let mut lm = Box::new(LineMap::new());
    let mut games = 0;
    let mut plies_per_game: Vec<usize> = Vec::new();
    let mut player_fam: HashMap<String, [u64; 3]> = HashMap::new();
    let mut player_games: HashMap<String, u32> = HashMap::new();
    let mut player_elo: HashMap<String, (u64, u64)> = HashMap::new();

    for path in &files {
        let game = pgn4::parse(&fs::read_to_string(path).unwrap()).unwrap();
        let mut board = game.initial_board().unwrap();
        games += 1;
        let mut done = 0usize;

        let seat_tags = ["Red", "Blue", "Yellow", "Green"];
        let elo_tags = ["RedElo", "BlueElo", "YellowElo", "GreenElo"];
        let names: [String; 4] =
            std::array::from_fn(|i| game.tag(seat_tags[i]).unwrap_or("?").to_string());
        for i in 0..4 {
            *player_games.entry(names[i].clone()).or_insert(0) += 1;
            if let Some(e) = game.tag(elo_tags[i]).and_then(|s| s.parse::<u64>().ok()) {
                let ent = player_elo.entry(names[i].clone()).or_insert((0, 0));
                ent.0 += e;
                ent.1 += 1;
            }
        }

        'plies: for round in &game.rounds {
            for tok in &round.plies {
                let Some(m) = apply_ply(tok, &mut board) else {
                    break 'plies;
                };
                done += 1;

                // Move-event stats: entry / capture into a zone.
                if let Some(slot) = {
                    let s = sq_to_slot[m.to.index() as usize];
                    if s == usize::MAX { None } else { Some(s) }
                } {
                    entries[slot] += 1;
                    if m.flags.capture {
                        caps[slot] += 1;
                    }
                    if let Some(p) = board.piece_at(m.to) {
                        entries_pl[slot][p.player.index()] += 1;
                    }
                }

                // Positional snapshot.
                compute_lines(&board, &mut lm);
                total_plies += 1;
                for slot in 0..36 {
                    let s = slot_square[slot];
                    let zb = slot / 4;

                    if let Some(p) = board.piece_at(s) {
                        occ[slot] += 1;
                        occ_pl[slot][p.player.index()] += 1;
                        let sr = lm.reachers_at(s);
                        let mut fd = 0u64;
                        for i in 0..sr.count as usize {
                            if lm.pieces[sr.piece_indices[i] as usize].player == p.player {
                                fd += 1;
                            }
                        }
                        def_sum[slot] += fd;
                        def_cnt[slot] += 1;
                        let fam_idx = match ZONES[slot / 4].1 {
                            "center" => 0,
                            "gate" => 1,
                            _ => 2,
                        };
                        player_fam
                            .entry(names[p.player.index()].clone())
                            .or_insert([0; 3])[fam_idx] += 1;
                    }

                    let sr = lm.reachers_at(s);
                    for i in 0..sr.count as usize {
                        let pi = sr.piece_indices[i] as usize;
                        reach_pl[slot][lm.pieces[pi].player.index()] += 1;
                        if let Some(za) = zone_of(lm.pieces[pi].square.index() as usize) {
                            support[za][zb] += 1;
                        }
                    }
                }
            }
        }
        plies_per_game.push(done);
    }

    let tp = total_plies as f64;
    let zf = |zi: usize, arr: &[u64; 36]| -> u64 { (0..4).map(|k| arr[zi * 4 + k]).sum() };
    let zf2 = |zi: usize, arr: &[[u64; 4]; 36]| -> u64 {
        (0..4).map(|k| arr[zi * 4 + k].iter().sum::<u64>()).sum()
    };

    println!("=== Zone study over {games} human games ===");
    println!(
        "positions analyzed: {total_plies}  (plies/game: {:?})",
        plies_per_game
    );
    println!();

    // Per-zone summary.
    println!("zone     fam     occ%  ctrl/ply  entries  caps  avgDef");
    let mut order: Vec<usize> = (0..9).collect();
    order.sort_by(|&a, &b| zf(b, &occ).cmp(&zf(a, &occ)));
    for &zi in &order {
        let occ_z = zf(zi, &occ);
        let reach_z = zf2(zi, &reach_pl);
        let ent_z = zf(zi, &entries);
        let cap_z = zf(zi, &caps);
        let ds: u64 = (0..4).map(|k| def_sum[zi * 4 + k]).sum();
        let dc: u64 = (0..4).map(|k| def_cnt[zi * 4 + k]).sum();
        let occ_pct = 100.0 * occ_z as f64 / (tp * 4.0);
        let ctrl = reach_z as f64 / tp;
        let avg_def = if dc > 0 { ds as f64 / dc as f64 } else { 0.0 };
        println!(
            "{}  {:6}  {:4.1}  {:7.2}  {:7}  {:4}  {:5.2}",
            ZONES[zi].0, ZONES[zi].1, occ_pct, ctrl, ent_z, cap_z, avg_def
        );
    }
    println!(
        "(occ% = avg fraction of a zone's 4 squares held; ctrl/ply = avg reachers on the zone)"
    );
    println!();

    // Family rollup.
    println!("family   occ%  ctrl/ply  entries  caps  avgDef");
    for fam in ["center", "gate", "quad"] {
        let zs: Vec<usize> = (0..9).filter(|&zi| ZONES[zi].1 == fam).collect();
        let occ_f: u64 = zs.iter().map(|&zi| zf(zi, &occ)).sum();
        let reach_f: u64 = zs.iter().map(|&zi| zf2(zi, &reach_pl)).sum();
        let ent_f: u64 = zs.iter().map(|&zi| zf(zi, &entries)).sum();
        let cap_f: u64 = zs.iter().map(|&zi| zf(zi, &caps)).sum();
        let ds: u64 = zs
            .iter()
            .flat_map(|&zi| (0..4).map(move |k| def_sum[zi * 4 + k]))
            .sum();
        let dc: u64 = zs
            .iter()
            .flat_map(|&zi| (0..4).map(move |k| def_cnt[zi * 4 + k]))
            .sum();
        let nsq = (zs.len() * 4) as f64;
        let occ_pct = 100.0 * occ_f as f64 / (tp * nsq);
        let avg_def = if dc > 0 { ds as f64 / dc as f64 } else { 0.0 };
        println!(
            "{:7}  {:4.1}  {:7.2}  {:7}  {:4}  {:5.2}",
            fam,
            occ_pct,
            reach_f as f64 / tp,
            ent_f,
            cap_f,
            avg_def
        );
    }
    println!();

    // Per-seat occupancy share of each family (which seat uses which zones).
    println!("seat occupancy share by family (% of that family's occupied square-plies):");
    println!("        center   gate   quad");
    for pl in 0..4 {
        let mut row = [0.0f64; 3];
        for (fi, fam) in ["center", "gate", "quad"].iter().enumerate() {
            let zs: Vec<usize> = (0..9).filter(|&zi| ZONES[zi].1 == *fam).collect();
            let mine: u64 = zs
                .iter()
                .flat_map(|&zi| (0..4).map(move |k| occ_pl[zi * 4 + k][pl]))
                .sum();
            let total: u64 = zs.iter().map(|&zi| zf(zi, &occ)).sum();
            row[fi] = if total > 0 {
                100.0 * mine as f64 / total as f64
            } else {
                0.0
            };
        }
        println!(
            "  {}    {:5.1}  {:5.1}  {:5.1}",
            PNAME[pl], row[0], row[1], row[2]
        );
    }
    println!();

    // Inter-zone reach matrix: support[A][B] per ply (A-row pieces projecting into B-col).
    println!("inter-zone reach (avg pieces-in-rowzone projecting into col-zone, per ply):");
    print!("        ");
    for zi in 0..9 {
        print!("{:>7}", ZONES[zi].0.trim());
    }
    println!();
    for a in 0..9 {
        print!("{}", ZONES[a].0);
        for b in 0..9 {
            print!("{:7.2}", support[a][b] as f64 / tp);
        }
        println!();
    }
    println!();

    // Top inter-zone support pairs (off-diagonal).
    let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
    for a in 0..9 {
        for b in 0..9 {
            if a != b {
                pairs.push((a, b, support[a][b] as f64 / tp));
            }
        }
    }
    pairs.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap());
    println!("top inter-zone reach pairs (A -> B):");
    for (a, b, v) in pairs.into_iter().take(10) {
        println!(
            "  {} -> {}   {:.2}/ply",
            ZONES[a].0.trim(),
            ZONES[b].0.trim(),
            v
        );
    }
    println!();

    // Per recurring human player: how their zone-occupancy splits across families.
    println!("recurring players (>1 game): zone-family occupancy profile");
    println!("player                  games  avgElo  center%  gate%  quad%");
    let mut plist: Vec<(&String, &[u64; 3])> = player_fam
        .iter()
        .filter(|(n, _)| player_games.get(*n).copied().unwrap_or(0) > 1)
        .collect();
    plist.sort_by(|a, b| player_games.get(b.0).cmp(&player_games.get(a.0)));
    for (name, fam) in plist {
        let tot = (fam[0] + fam[1] + fam[2]) as f64;
        if tot == 0.0 {
            continue;
        }
        let g = player_games.get(name).copied().unwrap_or(0);
        let (es, ec) = player_elo.get(name).copied().unwrap_or((0, 0));
        let avg_elo = if ec > 0 { es as f64 / ec as f64 } else { 0.0 };
        println!(
            "{:22}  {:4}  {:6.0}   {:5.1}  {:5.1}  {:5.1}",
            name,
            g,
            avg_elo,
            100.0 * fam[0] as f64 / tot,
            100.0 * fam[1] as f64 / tot,
            100.0 * fam[2] as f64 / tot
        );
    }
}
