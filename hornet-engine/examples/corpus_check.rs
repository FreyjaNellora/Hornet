//! Corpus health check: for every game file, report which instrument gate it fails —
//! `result_points` (outcome header), `pgn4::parse` (structure), `initial_board` (FEN4),
//! or replay divergence (shared replayer, ply coverage). Run after every ingest batch.
//!
//! Run: cargo run --release --example corpus_check [-- dir]   (default ../human_games)

use hornet_engine::board::pgn4::{self, result_points};
use hornet_engine::replay::{ReplayState, resolve_ply};
use std::fs;
use std::path::PathBuf;

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let dir = std::env::args()
        .nth(1)
        .map(|a| base.join(a))
        .unwrap_or(base.join("human_games"));

    let (mut total, mut ok, mut no_result, mut no_parse, mut no_board) = (0, 0, 0, 0, 0);
    let (mut full_replay, mut partial) = (0, 0);
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("games dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("pgn4"))
        .collect();
    entries.sort();

    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(&path).unwrap();
        total += 1;
        if result_points(&text).is_none() {
            no_result += 1;
            println!("NO-RESULT   {name}");
            continue;
        }
        let game = match pgn4::parse(&text) {
            Ok(g) => g,
            Err(e) => {
                no_parse += 1;
                println!("NO-PARSE    {name}: {e:?}");
                continue;
            }
        };
        let board = match game.initial_board() {
            Ok(b) => b,
            Err(e) => {
                no_board += 1;
                println!("NO-BOARD    {name}: {e:?}");
                continue;
            }
        };
        ok += 1;
        let (applied, plies, fail_info) = replay_coverage(board, &game);
        if applied == plies {
            full_replay += 1;
        } else {
            partial += 1;
            println!("PARTIAL     {name}: {applied}/{plies} plies | {fail_info}");
        }
    }

    println!(
        "\n{total} files: {ok} instrument-ready ({full_replay} replay fully, {partial} partially) | \
         {no_result} no-result, {no_parse} no-parse, {no_board} no-board"
    );
}

/// Replay `game` from `board`; returns (applied, decodable plies seen, failure description).
fn replay_coverage(
    mut board: hornet_engine::board::Board,
    game: &pgn4::Pgn4Game,
) -> (usize, usize, String) {
    let mut st = ReplayState::default();
    let (mut plies, mut applied) = (0usize, 0usize);
    let mut fail_info = String::new();
    'game: for round in &game.rounds {
        for tok in &round.plies {
            if pgn4::decode_ply(tok).is_none() {
                continue; // marker token
            }
            plies += 1;
            match resolve_ply(&mut board, tok, &mut st) {
                Some(mv) => {
                    board.make_move(mv);
                    applied += 1;
                }
                None => {
                    if let Some(pgn4::DecodedMove::Normal { from, to, .. }) = pgn4::decode_ply(tok)
                    {
                        fail_info = format!(
                            "token '{tok}' from={:?} to={:?} side_to_move={:?}",
                            board.piece_at(from),
                            board.piece_at(to),
                            board.side_to_move
                        );
                    } else {
                        fail_info = format!("token '{tok}' (castle)");
                    }
                    break 'game;
                }
            }
        }
    }
    (applied, plies, fail_info)
}
