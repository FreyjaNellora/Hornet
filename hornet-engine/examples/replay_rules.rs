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

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::board::{Board, dkw_rule};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use std::fs;
use std::path::PathBuf;

/// Decode + self-sync the mover + apply one ply (mirrors tests/pgn4_replay.rs, plus two
/// fidelity fixes). `last_mover` is the previous ply's mover, used to resolve castle tokens.
fn apply_ply(token: &str, board: &mut Board, last_mover: &mut Option<Player>) -> bool {
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
            let mut found = generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion);
            // DKW inference: a live king can NEVER capture its own piece, so a corpus king
            // landing on its own piece proves the mover is Dead-King-Walking (the replayer has
            // no game-flow to set the flag). Infer it and retry — the DKW move set generates
            // own-captures (EXP-026 rule 2).
            if found.is_none()
                && p.piece_type == hornet_engine::board::PieceType::King
                && board.piece_at(to).is_some_and(|t| t.player == p.player)
            {
                board.enter_dkw(p.player);
                found = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.from == from && m.to == to && m.promotion == promotion);
            }
            if found.is_some() {
                *last_mover = Some(p.player);
            }
            found
        }
        DecodedMove::Castle { kingside } => {
            // A castle token names no player. Trying players in fixed RBYG order misattributes
            // the castle whenever two players can castle the same side (EXP-028 forensics: the
            // dominant silent-divergence cause). Resolve in ROTATION order from the expected
            // next mover instead.
            let start = last_mover.map_or(Player::Red, |p| p.next());
            let mut found = None;
            let mut pl = start;
            for _ in 0..4 {
                board.side_to_move = pl;
                let dest = castle_king_destination(pl, kingside);
                if let Some(m) = generate_pseudo_legal(board)
                    .into_iter()
                    .find(|m| m.flags.castle && m.to == dest)
                {
                    found = Some(m);
                    *last_mover = Some(pl);
                    break;
                }
                pl = pl.next();
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
            let mut last_mover: Option<Player> = None;
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
                    if apply_ply(tok, &mut board, &mut last_mover) {
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
