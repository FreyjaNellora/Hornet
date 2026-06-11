//! Structural round-trip of every PGN4 game in the `baselines/` corpus.
//!
//! Baselines live at the project root (`Project_Hornet/baselines/`), one level above this
//! crate, so we reference them via `CARGO_MANIFEST_DIR/../baselines` rather than duplicating
//! them into the crate. (Spec §9 shows `baselines/` inside the crate; referencing avoids a
//! second copy that could drift.)

use hornet_engine::board::pgn4;
use hornet_engine::board::types::Player;
use std::fs;
use std::path::PathBuf;

fn baselines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines")
}

#[test]
fn all_corpus_games_parse_and_round_trip() {
    let dir = baselines_dir();
    let mut count = 0;

    for entry in fs::read_dir(&dir).expect("baselines dir should be readable") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("pgn4") {
            continue;
        }

        let text = fs::read_to_string(&path).unwrap();
        let game = pgn4::parse(&text)
            .unwrap_or_else(|e| panic!("parse failed for {}: {e}", path.display()));

        assert!(game.ply_count() > 0, "{} had no plies", path.display());

        // The start position resolves; corpus games all start from the canonical "4PC" setup.
        let board = game
            .initial_board()
            .unwrap_or_else(|e| panic!("initial_board failed for {}: {e}", path.display()));
        if game.start_fen4() == "4PC" {
            for p in Player::ALL {
                assert_eq!(
                    board.piece_count(p),
                    16,
                    "{}: {p:?} should start with 16 pieces",
                    path.display()
                );
            }
        }

        // Structural round-trip must be stable.
        let reparsed = pgn4::parse(&pgn4::serialize(&game)).unwrap();
        assert_eq!(
            reparsed,
            game,
            "round-trip changed structure of {}",
            path.display()
        );

        count += 1;
    }

    // Corpus size — keep in sync with CORPUS_GAMES in tests/pgn4_replay.rs (16 → 32 on 2026-06-10).
    assert_eq!(count, 32, "expected 32 PGN4 corpus files, found {count}");
}
