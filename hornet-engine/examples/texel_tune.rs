//! Texel tuning for the v0 eval weights (Österlund 2014, adapted to 4PC).
//!
//! Fits `W_MATERIAL/POSITIONAL/SAFETY/CROSSFIRE` to corpus game OUTCOMES (final FFA points →
//! per-player placement) by minimizing the MSE between the sigmoid-mapped eval and the actual
//! outcome. Queries are run ONCE per position and cached, so the fit loop is pure arithmetic — the
//! whole tune runs in seconds. This is the classical hand-eval tuning method: don't pick the
//! numbers, fit them to who actually won.
//!
//! Run: cargo run --release --example texel_tune

use hornet_engine::board::pgn4;
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::queries::{
    elimination_proximity, king_danger_table_scalar, query_king_safety, query_pawn_connected,
    query_pawn_doubled, query_pawn_isolated, run_all_queries,
};
use hornet_engine::replay::{ReplayState, apply_ply};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

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
///
/// EXP-024/025 extension: alongside the four deployed components, five **candidate terms** are
/// cached so independent weights can be fitted without touching the eval — win-proximity (the
/// search-side win signal in eval frame), king-danger table (Kimi's non-linear shape), and the
/// three unbundled pawn-structure counts (C3.1).
struct Pos {
    m: [f64; 4],
    p: [f64; 4],
    s: [f64; 4],
    o: [f64; 4],
    /// win_i = Σ_{j≠i} prox_j − 3·prox_i (mean-zero by construction; + is good for i).
    win: [f64; 4],
    /// Non-linear attack-units king danger (table scalar; high = bad for i).
    dgr: [f64; 4],
    /// Isolated-pawn count (bad), doubled-pawn count (bad), connected-pawn count (good).
    iso: [f64; 4],
    dbl: [f64; 4],
    conn: [f64; 4],
    target: [f64; 4],
}

/// Number of fitted weights: [M, P, S, O, WIN, DGR, ISO, DBL, CONN].
const NW: usize = 9;
/// Signs match the deployed convention (O subtracts) and classical expectations for the
/// candidates (danger/iso/dbl penalize, win/conn reward). A fitted weight may go NEGATIVE for
/// the candidate terms (indices 4..9) — that reads as "signal in the opposite direction".
const SIGN: [f64; NW] = [1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, 1.0];

/// Mean-relative eval component for player `i` under candidate weights (matches `compute_utility`
/// for the first four; candidates extend the same mean-relative frame).
fn util(pos: &Pos, i: usize, w: &[f64; NW]) -> f64 {
    let mean = |a: &[f64; 4]| (a[0] + a[1] + a[2] + a[3]) / 4.0;
    let comps: [&[f64; 4]; NW] = [
        &pos.m, &pos.p, &pos.s, &pos.o, &pos.win, &pos.dgr, &pos.iso, &pos.dbl, &pos.conn,
    ];
    let mut u = 0.0;
    for c in 0..NW {
        if w[c] != 0.0 {
            u += SIGN[c] * w[c] * (comps[c][i] - mean(comps[c]));
        }
    }
    u
}

/// Mean squared error over all positions × 4 players.
fn mse(data: &[Pos], w: &[f64; NW], k: f64) -> f64 {
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
    // DEFAULT: the curated **human** corpus only (human_games/: baselines + verified rated
    // collected games, deduped, malformed excluded). Human and self-play data are kept separate
    // on principle — self-play reflects the engine's own biases, and the 2026-06-12 human-only
    // fits showed the mixed default diluting human behavioral signal ~8× (EXP-025 addendum 2).
    // HORNET_INCLUDE_SELFPLAY=1 adds selfplay_games/ explicitly (label any such fit "combined").
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut dirs = vec![base.join("human_games")];
    if std::env::var("HORNET_INCLUDE_SELFPLAY").is_ok_and(|v| v == "1") {
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
            let mut st = ReplayState::default();
            'game: for round in &game.rounds {
                for tok in &round.plies {
                    if ply % SAMPLE == 0 {
                        compute_lines(&board, &mut lines);
                        let qv = run_all_queries(&lines, &board);
                        let f = |a: [i16; 4]| [a[0] as f64, a[1] as f64, a[2] as f64, a[3] as f64];
                        // Candidate terms (EXP-024/025): computed independently per position.
                        let ks = query_king_safety(&lines, &board);
                        let prox = elimination_proximity(&qv.material, &ks);
                        let total: i32 = prox.iter().map(|&x| x as i32).sum();
                        let mut win = [0.0f64; 4];
                        let mut dgr = [0.0f64; 4];
                        for i in 0..4 {
                            win[i] = (total - 4 * prox[i] as i32) as f64;
                            dgr[i] = king_danger_table_scalar(&ks[i]) as f64;
                        }
                        data.push(Pos {
                            m: f(qv.material),
                            p: f(qv.positional),
                            s: f(qv.safety),
                            o: f(qv.crossfire),
                            win,
                            dgr,
                            iso: f(query_pawn_isolated(&board)),
                            dbl: f(query_pawn_doubled(&board)),
                            conn: f(query_pawn_connected(&board)),
                            target,
                        });
                    }
                    if !apply_ply(&mut board, tok, &mut st) {
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

    // Fit K (sigmoid scale) once, with the current deployed weights (candidates at 0 — this
    // reproduces the 4-weight baseline exactly; the EXP-023 self-check).
    let baseline: [f64; NW] = [4.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let (mut best_k, mut best_k_mse) = (0.0001, f64::MAX);
    let mut k = 0.00002;
    while k <= 0.01 {
        let e = mse(&data, &baseline, k);
        if e < best_k_mse {
            best_k_mse = e;
            best_k = k;
        }
        k += 0.00002;
    }
    eprintln!("fitted K={best_k:.4} | baseline weights {baseline:?} MSE={best_k_mse:.5}");

    // Local search the integer weights, K fixed. Per-index step sizes match each component's
    // natural scale (pawn counts are 0..~8 units vs material in the thousands of cp, so their
    // weights move in steps of 5 cp/unit). Deployed weights (0..4) stay non-negative; candidate
    // weights (4..9) may go negative — a negative fit = signal opposite to the classical sign.
    const STEP: [f64; NW] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0, 5.0, 5.0];
    let k = best_k;
    let mut w = baseline;
    let mut cur = mse(&data, &w, k);
    let mut improved = true;
    while improved {
        improved = false;
        for idx in 0..NW {
            for dir in [-1.0, 1.0] {
                let mut cand = w;
                cand[idx] += dir * STEP[idx];
                if idx < 4 {
                    cand[idx] = cand[idx].max(0.0);
                }
                let e = mse(&data, &cand, k);
                if e < cur - 1e-9 {
                    w = cand;
                    cur = e;
                    improved = true;
                }
            }
        }
    }
    eprintln!(
        "tuned weights M={} P={} S={} O={} | WIN={} DGR={} ISO={} DBL={} CONN={} | MSE={cur:.5} (baseline {best_k_mse:.5}, -{:.5})",
        w[0],
        w[1],
        w[2],
        w[3],
        w[4],
        w[5],
        w[6],
        w[7],
        w[8],
        best_k_mse - cur
    );
    eprintln!(
        "=> deployed components: W_MATERIAL={} W_POSITIONAL={} W_SAFETY={} W_CROSSFIRE={}",
        w[0] as i16, w[1] as i16, w[2] as i16, w[3] as i16
    );
    eprintln!(
        "=> candidate terms (EXP-024/025; sign convention: WIN+ DGR- ISO- DBL- CONN+): a 0 = no outcome signal at this corpus size; non-zero = candidate for a measured wiring arm"
    );

    // Single-term marginal fits: the pawn counts are strongly anti-correlated (a pawn is either
    // isolated or connected), so the joint fit can ride a collinear direction with huge opposing
    // weights. Fit each candidate ALONE on top of a FIXED base — the per-term marginal MSE drop
    // is the interpretable signal, and the spread across known-null terms estimates the noise
    // floor. Marginals are **base-sensitive** (candidate signal partially overlaps material's:
    // a collapsing player is also down material, so heavier M absorbs danger/structure signal) —
    // so report both canonical bases: the deployed eval (6,0,0,1) and the Texel-preferred shape
    // (4,0,0,1). A term that passes on BOTH is robust.
    let names = ["WIN", "DGR", "ISO", "DBL", "CONN"];
    for (base_name, base4) in [
        ("deployed (6,0,0,1)", [6.0, 0.0, 0.0, 1.0]),
        ("texel-shape (4,0,0,1)", [4.0, 0.0, 0.0, 1.0]),
    ] {
        let base: [f64; NW] = [
            base4[0], base4[1], base4[2], base4[3], 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let base_mse = mse(&data, &base, k);
        eprintln!("single-term marginal fits (base = {base_name}, MSE {base_mse:.5}):");
        for (t, name) in names.iter().enumerate() {
            let idx = 4 + t;
            let mut wt = base;
            let mut best = base_mse;
            let mut improved = true;
            while improved {
                improved = false;
                for dir in [-1.0, 1.0] {
                    let mut cand = wt;
                    cand[idx] += dir * STEP[idx];
                    let e = mse(&data, &cand, k);
                    if e < best - 1e-9 {
                        wt = cand;
                        best = e;
                        improved = true;
                    }
                }
            }
            eprintln!(
                "  {name}: weight {} | MSE {best:.5} (drop {:.5})",
                wt[idx],
                base_mse - best
            );
        }
    }
}
