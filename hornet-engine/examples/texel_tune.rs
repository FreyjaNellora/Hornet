//! Texel tuning for the v0 eval weights (Österlund 2014, adapted to 4PC).
//!
//! Fits `W_MATERIAL/POSITIONAL/SAFETY/CROSSFIRE` to corpus game OUTCOMES (final FFA points →
//! per-player placement) by minimizing the MSE between the sigmoid-mapped eval and the actual
//! outcome. Queries are run ONCE per position and cached, so the fit loop is pure arithmetic — the
//! whole tune runs in seconds. This is the classical hand-eval tuning method: don't pick the
//! numbers, fit them to who actually won.
//!
//! Run: cargo run --release --example texel_tune

use hornet_engine::board::Board;
use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use hornet_engine::queries::run_all_queries;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Decode + self-sync the mover + apply one PGN4 ply (mirrors tests/pgn4_replay.rs).
fn apply_ply(token: &str, board: &mut Board) -> bool {
    let Some(decoded) = pgn4::decode_ply(token) else {
        return false;
    };
    let mv = match decoded {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let Some(p) = board.piece_at(from) else {
                return false;
            };
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
    match mv {
        Some(m) => {
            board.make_move(m);
            true
        }
        None => false,
    }
}

/// Parse `[Result "name: pts - ..."]` → [R,B,Y,G] points. chess.com lists the Result in **score
/// order, not seat order**, so we JOIN each name to the `[Red]/[Blue]/[Yellow]/[Green]` headers.
/// Falls back to positional order when seat headers are absent (self-play games already written R,B,Y,G).
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
    // Seat-join: map each Result name to its R/B/Y/G slot via the headers.
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
    // Fallback: positional (self-play games are written in R,B,Y,G order).
    Some([pairs[0].1, pairs[1].1, pairs[2].1, pairs[3].1])
}

/// Final points → per-player placement target in [0,1] (1st = 1.0 … 4th = 0.0, ties averaged).
fn outcome_from_points(pts: [f64; 4]) -> [f64; 4] {
    let mut out = [0.0; 4];
    for i in 0..4 {
        let above = (0..4).filter(|&j| pts[j] > pts[i]).count() as f64;
        let ties = (0..4).filter(|&j| j != i && pts[j] == pts[i]).count() as f64;
        let rank = 1.0 + above + ties * 0.5;
        out[i] = (4.0 - rank) / 3.0;
    }
    out
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// One labelled position: raw query components per player + the game outcome per player.
struct Pos {
    m: [f64; 4],
    p: [f64; 4],
    s: [f64; 4],
    o: [f64; 4],
    target: [f64; 4],
}

/// Mean-relative eval component for player `i` under candidate weights (matches `compute_utility`).
fn util(pos: &Pos, i: usize, w: [f64; 4]) -> f64 {
    let mean = |a: &[f64; 4]| (a[0] + a[1] + a[2] + a[3]) / 4.0;
    w[0] * (pos.m[i] - mean(&pos.m))
        + w[1] * (pos.p[i] - mean(&pos.p))
        + w[2] * (pos.s[i] - mean(&pos.s))
        - w[3] * (pos.o[i] - mean(&pos.o))
}

/// Mean squared error over all positions × 4 players.
fn mse(data: &[Pos], w: [f64; 4], k: f64) -> f64 {
    let mut e = 0.0;
    let mut n = 0.0;
    for pos in data {
        for i in 0..4 {
            let d = sigmoid(k * util(pos, i, w)) - pos.target[i];
            e += d * d;
            n += 1.0;
        }
    }
    e / n
}

fn main() {
    // HORNET_HUMAN_ONLY=1 → the single curated human corpus (human_games/: baselines + verified rated
    // collected games, deduped, malformed excluded). HORNET_HUMAN_ONLY off also adds self-play.
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut dirs = vec![base.join("human_games")];
    if std::env::var("HORNET_HUMAN_ONLY").is_err() {
        dirs.push(base.join("selfplay_games"));
    }
    let mut data: Vec<Pos> = Vec::new();
    let mut lines = LineMap::new();
    const SAMPLE: usize = 3; // every 3rd position
    let mut games = 0;
    // Dedup games by move-content — baselines & collected_games overlap (different filenames/headers).
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();

    for dir in &dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            let Some(pts) = parse_result_points(&text) else {
                continue;
            };
            let target = outcome_from_points(pts);
            let Ok(game) = pgn4::parse(&text) else {
                continue;
            };
            let Ok(mut board) = game.initial_board() else {
                continue;
            };
            // Dedup by normalized move-content (baselines & collected_games overlap by game).
            let mut moves = String::new();
            for line in text.lines() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('[') {
                    continue;
                }
                for tok in l.split_whitespace() {
                    if tok.contains('-')
                        && tok
                            .chars()
                            .next()
                            .map_or(false, |c| c.is_ascii_alphabetic())
                    {
                        moves.push_str(tok);
                    }
                }
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            moves.hash(&mut h);
            if !seen.insert(h.finish()) {
                continue;
            }
            games += 1;
            let mut ply = 0usize;
            'game: for round in &game.rounds {
                for tok in &round.plies {
                    if ply % SAMPLE == 0 {
                        compute_lines(&board, &mut lines);
                        let qv = run_all_queries(&lines, &board);
                        let f = |a: [i16; 4]| [a[0] as f64, a[1] as f64, a[2] as f64, a[3] as f64];
                        data.push(Pos {
                            m: f(qv.material),
                            p: f(qv.positional),
                            s: f(qv.safety),
                            o: f(qv.crossfire),
                            target,
                        });
                    }
                    if !apply_ply(tok, &mut board) {
                        break 'game;
                    }
                    ply += 1;
                }
            }
        }
    }
    eprintln!("dataset: {} positions from {} games", data.len(), games);

    // HORNET_DUMP_CSV=1 → export per-player mean-relative components + outcome target, for the Python
    // tools (proper logistic fit + bootstrap CIs). One row per (position, player).
    if std::env::var("HORNET_DUMP_CSV").is_ok() {
        use std::io::Write;
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tools");
        fs::create_dir_all(&out).ok();
        let path = out.join("texel_positions.csv");
        let mut fh = fs::File::create(&path).expect("create csv");
        writeln!(fh, "dM,dP,dS,dO,target").unwrap();
        let mean = |a: &[f64; 4]| (a[0] + a[1] + a[2] + a[3]) / 4.0;
        for pos in &data {
            let (mm, mp, ms, mo) = (mean(&pos.m), mean(&pos.p), mean(&pos.s), mean(&pos.o));
            for i in 0..4 {
                writeln!(
                    fh,
                    "{},{},{},{},{}",
                    pos.m[i] - mm,
                    pos.p[i] - mp,
                    pos.s[i] - ms,
                    pos.o[i] - mo,
                    pos.target[i]
                )
                .unwrap();
            }
        }
        eprintln!("dumped {} rows to {}", data.len() * 4, path.display());
    }

    // Fit K (sigmoid scale) once, with the current weights.
    let baseline = [4.0, 1.0, 1.0, 1.0];
    let (mut best_k, mut best_k_mse) = (0.0001, f64::MAX);
    let mut k = 0.00002;
    while k <= 0.01 {
        let e = mse(&data, baseline, k);
        if e < best_k_mse {
            best_k_mse = e;
            best_k = k;
        }
        k += 0.00002;
    }
    eprintln!("fitted K={best_k:.4} | baseline weights {baseline:?} MSE={best_k_mse:.5}");

    // Local search the integer weights (deployable as i16), K fixed.
    let k = best_k;
    let mut w = baseline;
    let mut cur = mse(&data, w, k);
    let mut improved = true;
    while improved {
        improved = false;
        for idx in 0..4 {
            for delta in [-1.0, 1.0] {
                let mut cand = w;
                cand[idx] = (cand[idx] + delta).max(0.0);
                let e = mse(&data, cand, k);
                if e < cur - 1e-9 {
                    w = cand;
                    cur = e;
                    improved = true;
                }
            }
        }
    }
    eprintln!(
        "tuned weights M={} P={} S={} O={} | MSE={cur:.5} (baseline {best_k_mse:.5}, -{:.5})",
        w[0],
        w[1],
        w[2],
        w[3],
        best_k_mse - cur
    );
    eprintln!(
        "=> eval.rs: W_MATERIAL={} W_POSITIONAL={} W_SAFETY={} W_CROSSFIRE={}",
        w[0] as i16, w[1] as i16, w[2] as i16, w[3] as i16
    );
}
