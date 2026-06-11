//! Replays the move stream of every PGN4 corpus game against the move generator.
//!
//! Self-syncing: each ply's mover is read from the piece on its `from` square (rather than
//! trusting turn rotation), so replay survives eliminations/DKW and validates that the move
//! **generator produces every move that occurs in real games**. Matching is against
//! *pseudo-legal* moves (a real move is always pseudo-legal; this isolates move geometry from
//! the legality filter, and tolerates DKW kings moving into check).

use hornet_engine::board::Board;
use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::eval::eval_4vec;
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use hornet_engine::zones::{ZONES, aggregate_zone_control};
use std::fs;
use std::path::PathBuf;

/// Size of the `baselines/` PGN4 corpus. Update when games are added (and recalibrate the floors
/// below against an actual run — they are regression floors, set just under observed values).
const CORPUS_GAMES: usize = 32;
/// Regression floor: total plies replayed across the corpus (observed 5058/7477 on 2026-06-10).
const MIN_PLIES_REPLAYED: usize = 5000;
/// Regression floor: games replayed end-to-end with no move-gen miss (observed 15/32, 2026-06-10).
const MIN_GAMES_FULLY_REPLAYED: usize = 15;

fn baselines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines")
}

/// Decode + match + apply one ply token. Returns false if it can't be decoded or matched.
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
            board.side_to_move = p.player; // self-sync to the actual mover
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

/// Replay a game; return (plies applied, total plies, first failing token if any).
/// Also prints zone control at every 10th ply for measurement.
fn replay(game: &pgn4::Pgn4Game) -> (usize, usize, Option<String>) {
    let mut board = game.initial_board().unwrap();
    let total = game.ply_count();
    let mut done = 0;
    let mut lines = LineMap::new();
    for round in &game.rounds {
        for tok in &round.plies {
            // Track piece positions by phase and player
            if done % 20 == 0 && done > 0 {
                let phase = if done <= 20 {
                    "early"
                } else if done <= 100 {
                    "mid"
                } else {
                    "late"
                };

                // Count piece types per zone per player
                let mut zone_pieces: [[(u8, u8, u8, u8, u8); 4]; 9] = [[(0, 0, 0, 0, 0); 4]; 9]; // (pawn, knight, bishop, rook, queen)
                for (zi, zone) in ZONES.iter().enumerate() {
                    for sq in zone.squares() {
                        if let Some(p) = board.piece_at(sq) {
                            let pi = p.player.index();
                            let (mut pa, mut n, mut b, mut r, mut q) = zone_pieces[zi][pi];
                            match p.piece_type {
                                hornet_engine::board::types::PieceType::Pawn => pa += 1,
                                hornet_engine::board::types::PieceType::Knight => n += 1,
                                hornet_engine::board::types::PieceType::Bishop => b += 1,
                                hornet_engine::board::types::PieceType::Rook => r += 1,
                                hornet_engine::board::types::PieceType::Queen => q += 1,
                                _ => {}
                            }
                            zone_pieces[zi][pi] = (pa, n, b, r, q);
                        }
                    }
                }

                // Print non-empty zones
                for (zi, zone) in ZONES.iter().enumerate() {
                    for player in Player::ALL {
                        let (pa, n, b, r, q) = zone_pieces[zi][player.index()];
                        if pa + n + b + r + q > 0 {
                            println!(
                                "  ply {:3} {} {}: {}=p{}n{}b{}r{}q{}",
                                done,
                                phase,
                                zone.name,
                                player.to_char(),
                                pa,
                                n,
                                b,
                                r,
                                q
                            );
                        }
                    }
                }
            }
            if pgn4::decode_ply(tok).is_none() {
                continue; // non-move token (e.g. "R"/"S" result/resign marker) — not a move-gen test
            }
            if !apply_ply(tok, &mut board) {
                return (done, total, Some(tok.clone()));
            }
            done += 1;
        }
    }
    (done, total, None)
}

#[test]
fn decode_ply_handles_corpus_notation() {
    use hornet_engine::board::types::{PieceType, Square};
    let sq = |s: &str| Square::from_algebraic(s).unwrap();
    let normal = |f: &str, t: &str, p| DecodedMove::Normal {
        from: sq(f),
        to: sq(t),
        promotion: p,
    };

    assert_eq!(pgn4::decode_ply("h2-h3"), Some(normal("h2", "h3", None)));
    assert_eq!(pgn4::decode_ply("Ne1-f3"), Some(normal("e1", "f3", None)));
    assert_eq!(
        pgn4::decode_ply("Bn6xBg13"),
        Some(normal("n6", "g13", None))
    );
    assert_eq!(
        pgn4::decode_ply("Qb7xg12+#"),
        Some(normal("b7", "g12", None))
    );
    assert_eq!(
        pgn4::decode_ply("g7-g8=D"),
        Some(normal("g7", "g8", Some(PieceType::Queen)))
    );
    assert_eq!(
        pgn4::decode_ply("h11-g11=D"),
        Some(normal("h11", "g11", Some(PieceType::Queen)))
    );
    assert_eq!(
        pgn4::decode_ply("Kh13-i14R"),
        Some(normal("h13", "i14", None))
    );
    assert_eq!(
        pgn4::decode_ply("O-O"),
        Some(DecodedMove::Castle { kingside: true })
    );
    assert_eq!(
        pgn4::decode_ply("O-O-O"),
        Some(DecodedMove::Castle { kingside: false })
    );
    assert_eq!(pgn4::decode_ply("R"), None);
}

#[test]
fn corpus_games_replay_against_move_gen() {
    let dir = baselines_dir();
    let mut games = 0;
    let mut total_applied = 0usize;
    let mut total_plies = 0usize;
    let mut fully = 0;

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }
        let game = pgn4::parse(&fs::read_to_string(&path).unwrap()).unwrap();
        let (done, total, fail) = replay(&game);
        games += 1;
        total_applied += done;
        total_plies += total;
        if fail.is_none() {
            fully += 1; // reached the end (skipping trailing non-move tokens) with no move-gen miss
        } else {
            println!(
                "{}: {done}/{total} plies (stopped at {:?})",
                path.display(),
                fail
            );
        }
        // Every game must at least clear its opening before any divergence.
        assert!(done >= 8, "{}: only replayed {done} plies", path.display());
    }

    assert_eq!(games, CORPUS_GAMES);
    println!(
        "replayed {total_applied}/{total_plies} corpus plies; {fully}/{CORPUS_GAMES} games fully"
    );
    // Regression baseline. This validates move *geometry* against the chess.com corpus, whose DKW is
    // *takeable*; the replay never sets the DKW flag, so Hornet's **fully-frozen** DKW rule (a DKW
    // player's pieces are un-capturable — `move_gen::DKW_PIECES_REMOVABLE = false`) intentionally
    // diverges wherever the corpus captures a DKW piece (incl. a dead king taking its own). Skipping
    // trailing non-move markers ("R"/"S") helps; the future *removable* variant would restore full
    // corpus fidelity. DKW rules are validated by the unit tests + `game.rs`; see EXP-011.
    // Floors recalibrated 2026-06-10 for the 32-game corpus (was 16: >=2500 plies, >=8 fully).
    assert!(
        total_applied >= MIN_PLIES_REPLAYED,
        "move-gen regression: only {total_applied} plies replayed (floor {MIN_PLIES_REPLAYED})"
    );
    assert!(
        fully >= MIN_GAMES_FULLY_REPLAYED,
        "expected >={MIN_GAMES_FULLY_REPLAYED} fully-replayed games, got {fully}"
    );
}
