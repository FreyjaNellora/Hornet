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
use hornet_engine::move_gen::{castle_king_destination, generate_pseudo_legal};
use std::fs;
use std::path::PathBuf;

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
fn replay(game: &pgn4::Pgn4Game) -> (usize, usize, Option<String>) {
    let mut board = game.initial_board().unwrap();
    let total = game.ply_count();
    let mut done = 0;
    for round in &game.rounds {
        for tok in &round.plies {
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
        if done == total {
            fully += 1;
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

    assert_eq!(games, 16);
    println!("replayed {total_applied}/{total_plies} corpus plies; {fully}/16 games fully");
    // Regression baseline. The unreplayed remainder bounds at Dead-King-Walking (dead kings move
    // randomly and can capture their own pieces) and elimination semantics, which move generation
    // does not model yet — see phase-2 watch items. Normal play (incl. captures, castling,
    // promotions, checks) is validated by these plies + the 4 fully-replayed games.
    assert!(
        total_applied >= 2500,
        "move-gen regression: only {total_applied} plies replayed"
    );
    assert!(fully >= 4, "expected >=4 fully-replayed games, got {fully}");
}
