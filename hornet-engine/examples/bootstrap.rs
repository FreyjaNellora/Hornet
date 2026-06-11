//! Self-play BOOTSTRAP: play many full games from random openings and save each as PGN4, so the
//! games become a growing corpus (Texel / future NNUE / search-shape A-B all need games). Games
//! replay (like the human corpus) up to the first elimination; positions are labelled by the FINAL
//! placement points.
//!
//! Run: cargo run --release --example bootstrap [N]   (default N = 200)
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

/// One full game: `opening` random plies, then depth-8 laser (adaptive base 4, deep floor 1) to a
/// survivor or the ply cap. Returns the move list (in play order) + final points.
fn play_game(seed: u64, opening: usize, cap: usize) -> (Vec<String>, [u16; 4]) {
    let mut searcher = Searcher::new(32)
        .with_beam_width(4)
        .with_adaptive_beam(true)
        .with_deep_floor(1)
        .with_forward_pruning(true);
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
            game.step(|b| searcher.search(b, 8).map(|(m, _)| m))
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
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(200);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("selfplay_games");
    fs::create_dir_all(&dir).expect("create selfplay_games/");

    let start = Instant::now();
    for i in 0..n {
        let seed = (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (moves, pts) = play_game(seed, 12, 150);
        let path = dir.join(format!("sp_game_{i:04}.pgn4"));
        write_pgn4(&path, &moves, pts);
        let ok = if i == 0 { validates(&path) } else { true };
        eprintln!(
            "game {i:4}: {:3} plies  points {pts:?}  [{:.0}s total]{}",
            moves.len(),
            start.elapsed().as_secs_f64(),
            if i == 0 && !ok {
                "  !! VALIDATION FAILED"
            } else {
                ""
            }
        );
        if i == 0 && !ok {
            eprintln!("first game failed the PGN4 validation — stopping; fix the format.");
            return;
        }
    }
    eprintln!("wrote {n} games to {}", dir.display());
}
