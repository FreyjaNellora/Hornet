//! Self-play BOOTSTRAP: play many full games from random openings and save each as PGN4, so the
//! games become a growing corpus (Texel / future NNUE / search-shape A-B all need games). Games
//! replay (like the human corpus) up to the first elimination; positions are labelled by the FINAL
//! placement points.
//!
//! **B5 regeneration config (2026-06-10).** The original 133-game corpus was generated on the
//! maxn path at beam 4 with the inverted free-capture ordering heuristic live — EXP-020 measured
//! that bug changing 11.6% of played moves at beam 4 — and was drawish (150-ply cap, few
//! eliminations; EXP-013). This config replaces it: **flashlight d8 cap 1200** (SYNTHESIS
//! recommendation, same shape protocol `go` plays; the flashlight never calls `move_order`) with
//! the **objective layer on (win 50, king-danger 100, linear/banked)** — EXP-017 measured that
//! layer lifting decisiveness 0/6 → 3/6 and doubling the depth win-rate — and a **200-ply cap**
//! (EXP-013's own recommendation). The objective layer here is a data-quality choice (stronger
//! outcome labels), recorded, not a shipped play default.
//!
//! Run: cargo run --release --example bootstrap [N] [START]   (defaults N = 150, START = 0)
//! Generates games START..START+N (seeds derive from the game index, so disjoint ranges from
//! parallel instances are collision-free — ~12 min/game single-threaded at this config, so the
//! 150-game corpus is generated as several parallel range instances).
//! Output: `selfplay_games/sp_game_NNNN.pgn4`

use hornet_engine::board::Move;
use hornet_engine::board::pgn4;
use hornet_engine::board::types::PieceType;
use hornet_engine::game::{Game, TurnOutcome};
use hornet_engine::move_gen::generate_legal;
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// from-to (+ `=D/=R/=B/=N` promotion) — the notation `decode_ply` round-trips.
fn move_str(m: &Move) -> String {
    let mut s = format!("{}-{}", m.from.to_algebraic(), m.to.to_algebraic());
    if let Some(p) = m.promotion {
        s.push('=');
        s.push(match p {
            PieceType::Queen | PieceType::PromotedQueen => 'D',
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            _ => 'D',
        });
    }
    s
}

/// One full game: `opening` random plies, then **flashlight d8 cap 1200 + objective layer**
/// (win 50, king-danger 100 — see the module docs for the measured basis) to a survivor or the
/// ply cap. Returns the move list (in play order) + final points.
fn play_game(seed: u64, opening: usize, cap: usize) -> (Vec<String>, [u16; 4]) {
    const FLASHLIGHT_CAP: usize = 1200;
    let mut searcher = Searcher::new(32).with_win_term(50).with_king_danger(100);
    let mut game = Game::from_start(seed);
    let mut rng = seed | 1;
    let mut moves = Vec::new();
    for ply in 0..cap {
        if game.active_count() <= 1 {
            break;
        }
        let outcome = if ply < opening {
            let r = xorshift(&mut rng) as usize;
            game.step(|b| {
                let l = generate_legal(b);
                (!l.is_empty()).then(|| l[r % l.len()])
            })
        } else {
            game.step(|b| {
                searcher
                    .search_flashlight(b, 8, |_| FLASHLIGHT_CAP)
                    .map(|(m, _)| m)
            })
        };
        if let TurnOutcome::Moved(mv) = outcome {
            moves.push(move_str(&mv));
        }
    }
    (moves, game.points())
}

fn write_pgn4(path: &PathBuf, moves: &[String], pts: [u16; 4]) {
    let mut t = String::new();
    t.push_str("[Variant \"FFA\"]\n");
    t.push_str("[StartFen4 \"4PC\"]\n");
    t.push_str(&format!(
        "[Result \"R: {} - B: {} - Y: {} - G: {}\"]\n\n",
        pts[0], pts[1], pts[2], pts[3]
    ));
    for (i, chunk) in moves.chunks(4).enumerate() {
        t.push_str(&format!("{}. {}\n", i + 1, chunk.join(" .. ")));
    }
    fs::write(path, t).expect("write pgn4");
}

/// Light check that texel_tune can load a written game: parses + a 4-value Result.
fn validates(path: &PathBuf) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let has_result = text
        .lines()
        .find(|l| l.starts_with("[Result"))
        .map(|l| l.matches(" - ").count() == 3)
        .unwrap_or(false);
    pgn4::parse(&text)
        .map(|g| !g.rounds.is_empty())
        .unwrap_or(false)
        && has_result
}

fn main() {
    let arg = |n: usize| {
        std::env::args()
            .nth(n)
            .and_then(|a| a.parse::<usize>().ok())
    };
    let n = arg(1).unwrap_or(150);
    let start_idx = arg(2).unwrap_or(0);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("selfplay_games");
    fs::create_dir_all(&dir).expect("create selfplay_games/");

    let start = Instant::now();
    for i in start_idx..start_idx + n {
        let seed = (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (moves, pts) = play_game(seed, 12, 200);
        let path = dir.join(format!("sp_game_{i:04}.pgn4"));
        write_pgn4(&path, &moves, pts);
        let first = i == start_idx;
        let ok = if first { validates(&path) } else { true };
        eprintln!(
            "game {i:4}: {:3} plies  points {pts:?}  [{:.0}s total]{}",
            moves.len(),
            start.elapsed().as_secs_f64(),
            if first && !ok {
                "  !! VALIDATION FAILED"
            } else {
                ""
            }
        );
        if first && !ok {
            eprintln!("first game failed the PGN4 validation — stopping; fix the format.");
            return;
        }
    }
    eprintln!(
        "wrote games {start_idx}..{} to {}",
        start_idx + n,
        dir.display()
    );
}
