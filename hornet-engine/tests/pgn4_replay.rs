//! Replays the move stream of every PGN4 corpus game against the move generator, via the shared
//! `hornet_engine::replay` logic (self-syncing, pseudo-legal matching, EXP-028 fidelity
//! inferences — see that module's docs). Validates that the move **generator produces every
//! move that occurs in real games**.

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::Player;
use hornet_engine::replay::{ReplayState, apply_ply};
use hornet_engine::zones::ZONES;
use std::fs;
use std::path::PathBuf;

/// Size of the `baselines/` PGN4 corpus. Update when games are added (and recalibrate the floors
/// below against an actual run — they are regression floors, set just under observed values).
const CORPUS_GAMES: usize = 32;
/// Regression floor: total plies replayed across the corpus (observed 7098/7477 on 2026-06-12,
/// after the EXP-026 rule landing + EXP-028 replayer fidelity fixes; was 5058 pre-fix).
const MIN_PLIES_REPLAYED: usize = 7000;
/// Regression floor: games replayed end-to-end with no move-gen miss (observed 29/32, 2026-06-12;
/// was 15/32 pre-fix).
const MIN_GAMES_FULLY_REPLAYED: usize = 28;

fn baselines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines")
}

/// Replay a game; return (plies applied, total plies, first failing token if any).
/// Also prints zone control at every 10th ply for measurement.
fn replay(game: &pgn4::Pgn4Game) -> (usize, usize, Option<String>) {
    let mut board = game.initial_board().unwrap();
    let total = game.ply_count();
    let mut done = 0;
    let mut st = ReplayState::default();
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
            if !apply_ply(&mut board, tok, &mut st) {
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
    // Regression baseline. Validates move *geometry* against the chess.com corpus via the shared
    // `hornet_engine::replay` logic (EXP-026 capturable-no-points rule + EXP-028 fidelity
    // inferences: DKW own-capture, rotation-aware castles). Remaining known gaps: players who
    // went DKW via *checkmate* can't be inferred by the replayer (their phantom castle blocks),
    // plus a small notation tail — see EXP-028. Floors recalibrated 2026-06-12 (was: >=5000
    // plies / >=15 fully under the pre-EXP-026 frozen rule).
    assert!(
        total_applied >= MIN_PLIES_REPLAYED,
        "move-gen regression: only {total_applied} plies replayed (floor {MIN_PLIES_REPLAYED})"
    );
    assert!(
        fully >= MIN_GAMES_FULLY_REPLAYED,
        "expected >={MIN_GAMES_FULLY_REPLAYED} fully-replayed games, got {fully}"
    );
}
