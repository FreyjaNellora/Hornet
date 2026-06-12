//! DKW-rule corpus arbitration (EXP-026): replay every corpus game's move stream against the
//! move generator under the active `HORNET_DKW_RULE` variant and report coverage. The corpus is
//! recorded chess.com behavior, and all corpus games share one RuleVariants config — so the
//! variant that maximizes replay coverage **is** the chess.com rule.
//!
//! Replay never sets the `dkw` flag (no game-flow), but `make_move` sets `dead` on king capture,
//! so the variants' **post-king-capture** semantics (swept ≈ capturable vs locked) are exactly
//! what this discriminates.
//!
//! Run: HORNET_DKW_RULE=0|1|2 cargo run --release --example replay_rules [-- dir ...]
//!   Default dirs: ../baselines and ../human_games.

use hornet_engine::board::dkw_rule;
use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::replay::{ReplayState, apply_ply};
use std::fs;
use std::path::PathBuf;

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dirs: Vec<PathBuf> = if args.is_empty() {
        vec![base.join("baselines"), base.join("human_games")]
    } else {
        args.iter().map(|a| base.join(a)).collect()
    };

    let mut games = 0usize;
    let mut fully = 0usize;
    let mut applied = 0usize;
    let mut total = 0usize;
    // Dedup by move-content (baselines and human_games overlap by game).
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    use std::hash::{Hash, Hasher};

    for dir in &dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
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
            let moves: String = text
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('['))
                .collect();
            let mut h = std::collections::hash_map::DefaultHasher::new();
            moves.hash(&mut h);
            if !seen.insert(h.finish()) {
                continue;
            }
            games += 1;
            let mut done = 0usize;
            let mut failed = false;
            let mut st = ReplayState::default();
            let mut recent: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            for round in &game.rounds {
                for tok in &round.plies {
                    if pgn4::decode_ply(tok).is_none() {
                        continue; // non-move token ("R"/"S" markers)
                    }
                    total += 1; // counted regardless of failure → totals comparable across rules
                    if failed {
                        continue;
                    }
                    if apply_ply(&mut board, tok, &mut st) {
                        done += 1;
                        recent.push_back(tok.clone());
                        if recent.len() > 4 {
                            recent.pop_front();
                        }
                    } else {
                        failed = true;
                        // HORNET_REPLAY_VERBOSE=1: classify the gap (which token kinds diverge).
                        if std::env::var("HORNET_REPLAY_VERBOSE").is_ok_and(|v| v == "1") {
                            let kind = match pgn4::decode_ply(tok) {
                                Some(DecodedMove::Castle { .. }) => "castle",
                                Some(DecodedMove::Normal {
                                    from,
                                    to,
                                    promotion,
                                }) => {
                                    if promotion.is_some() {
                                        "promotion"
                                    } else {
                                        match board.piece_at(from) {
                                            None => "empty-from",
                                            Some(p) => {
                                                // A king "capturing" its own piece = a DKW king
                                                // walk the live-mover move-gen refuses.
                                                let own_target = board
                                                    .piece_at(to)
                                                    .is_some_and(|t| t.player == p.player);
                                                match p.piece_type {
                                                    hornet_engine::board::PieceType::King
                                                        if own_target =>
                                                    {
                                                        "king-own-capture"
                                                    }
                                                    hornet_engine::board::PieceType::King => "king",
                                                    hornet_engine::board::PieceType::Pawn => "pawn",
                                                    hornet_engine::board::PieceType::Knight => {
                                                        "knight"
                                                    }
                                                    hornet_engine::board::PieceType::Bishop => {
                                                        "bishop"
                                                    }
                                                    hornet_engine::board::PieceType::Rook => "rook",
                                                    _ => "queen",
                                                }
                                            }
                                        }
                                    }
                                }
                                None => "undecodable",
                            };
                            println!(
                                "FAIL {} ply {} token {} kind {} after [{}]",
                                path.file_name().unwrap().to_string_lossy(),
                                done,
                                tok,
                                kind,
                                recent.iter().cloned().collect::<Vec<_>>().join(" ")
                            );
                        }
                    }
                }
            }
            applied += done;
            if !failed {
                fully += 1;
            }
        }
    }
    println!(
        "rule {}: {applied}/{total} plies replayed; {fully}/{games} games fully",
        dkw_rule()
    );
}
