//! Strength-gate ablation for the quiescence lever: run the tactical fixtures (the same ones
//! `strength_gate.rs` uses) with quiescence OFF vs ON and report each match rate. The replay/parse
//! helpers are reused verbatim from `strength_gate.rs` (Kimi's gate); only `main`/`run_config` differ.
//!
//! Run: cargo run --release --example gate_ablation

use hornet_engine::board::pgn4::{self, DecodedMove};
use hornet_engine::board::types::{PieceType, Player, Square};
use hornet_engine::board::{Board, Move, fen4};
use hornet_engine::eval::eval_4vec;
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::move_gen::generate_pseudo_legal;
use hornet_engine::queries::{run_all_queries, see_capture};
use hornet_engine::search::Searcher;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn freyja_to_chesscom(token: &str) -> String {
    let squares = extract_squares_freyja(token);
    if squares.len() >= 2 {
        format!("{}-{}", squares[0], squares[1])
    } else {
        token.to_string()
    }
}

fn extract_squares_freyja(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if (b'a'..=b'n').contains(&bytes[i]) {
            let file = bytes[i] as char;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                let rank = &s[i + 1..j];
                if let Ok(r) = rank.parse::<u8>() {
                    if r >= 1 && r <= 14 {
                        out.push(format!("{}{}", file, r));
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

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
            board.side_to_move = p.player;
            generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion)
        }
        DecodedMove::Castle { kingside } => {
            use hornet_engine::move_gen::castle_king_destination;
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

fn replay_to_position(moves_to_replay: &str) -> Option<Board> {
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    for token in moves_to_replay.split_whitespace() {
        let chesscom_token = freyja_to_chesscom(token);
        if !apply_ply(&chesscom_token, &mut board) {
            return None;
        }
    }
    Some(board)
}

fn decode_human_move(token: &str, board: &mut Board) -> Option<hornet_engine::board::Move> {
    let token = token.trim_end_matches(['+', '#']);
    let token = token.replace("=Q", "=D");
    let decoded = pgn4::decode_ply(&token)?;
    match decoded {
        DecodedMove::Normal {
            from,
            to,
            promotion,
        } => {
            let Some(p) = board.piece_at(from) else {
                return None;
            };
            let saved_stm = board.side_to_move;
            board.side_to_move = p.player;
            let mv = generate_pseudo_legal(board)
                .into_iter()
                .find(|m| m.from == from && m.to == to && m.promotion == promotion);
            board.side_to_move = saved_stm;
            mv
        }
        DecodedMove::Castle { kingside } => {
            use hornet_engine::move_gen::castle_king_destination;
            let saved_stm = board.side_to_move;
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
            board.side_to_move = saved_stm;
            found
        }
    }
}

fn extract_samples(json: &str) -> Vec<HashMap<String, String>> {
    let mut samples = Vec::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut sample_start = None;
    for (i, c) in json.char_indices() {
        if c == '"' && !in_string {
            in_string = true;
        } else if c == '"' && in_string {
            in_string = false;
        } else if !in_string {
            if c == '{' {
                if depth == 1 {
                    sample_start = Some(i);
                }
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 1 {
                    if let Some(start) = sample_start {
                        let fragment = &json[start..=i];
                        if let Some(sample) = parse_sample(fragment) {
                            samples.push(sample);
                        }
                    }
                    sample_start = None;
                }
            }
        }
    }
    samples
}

fn parse_sample(fragment: &str) -> Option<HashMap<String, String>> {
    let mut sample = HashMap::new();
    for field in ["id", "name", "moves_to_replay", "human_move", "nextturn"] {
        if let Some(val) = extract_string_field(fragment, field) {
            sample.insert(field.to_string(), val);
        }
    }
    if sample.contains_key("id") {
        Some(sample)
    } else {
        None
    }
}

fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\": \"", field);
    let start = json.find(&pattern)?;
    let val_start = start + pattern.len();
    let mut val = String::new();
    let mut chars = json[val_start..].chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            break;
        } else if c == 92 as char {
            if let Some(next) = chars.next() {
                val.push(next);
            }
        } else {
            val.push(c);
        }
    }
    Some(val)
}

/// Run the fixtures with a given quiescence setting; return (matches, tested).
#[allow(dead_code)]
fn run_config(samples: &[HashMap<String, String>], depth: u32, quiescence: bool) -> (usize, usize) {
    let mut matches = 0;
    let mut tested = 0;
    for sample in samples {
        let moves_to_replay = sample.get("moves_to_replay").cloned().unwrap_or_default();
        let human_token = sample.get("human_move").cloned().unwrap_or_default();
        if moves_to_replay.is_empty() || human_token.is_empty() {
            continue;
        }
        let Some(mut board) = replay_to_position(&moves_to_replay) else {
            continue;
        };
        let Some(human_mv) = decode_human_move(&human_token, &mut board) else {
            continue;
        };
        tested += 1;
        eprint!("."); // progress (stderr is unbuffered)
        // Fast, fair config: speed levers on in BOTH arms so the search is tractable; the only
        // difference between arms is quiescence. Also the realistic deployment config.
        let mut searcher = Searcher::new(16)
            .with_beam_width(10)
            .with_forward_pruning(true)
            .with_adaptive_beam(true)
            .with_quiescence(quiescence)
            .with_node_budget(800_000); // bound each search so capture-dense fixtures can't hang
        if let Some((mv, _)) = searcher.search(&mut board, depth) {
            if mv == human_mv {
                matches += 1;
            }
        }
    }
    (matches, tested)
}

fn piece_letter(pt: PieceType) -> char {
    match pt {
        PieceType::Pawn => 'P',
        PieceType::Knight => 'N',
        PieceType::Bishop => 'B',
        PieceType::Rook => 'R',
        PieceType::Queen => 'Q',
        PieceType::PromotedQueen => 'q',
        PieceType::King => 'K',
    }
}

fn sq_alg(sq: Square) -> String {
    format!("{}{}", (b'a' + sq.file()) as char, sq.rank() + 1)
}

/// Describe a move at `board` (before it is made): mover, from-to, what it captures, promotion.
fn describe(board: &Board, mv: Move) -> String {
    let mover = board
        .piece_at(mv.from)
        .map(|p| piece_letter(p.piece_type))
        .unwrap_or('?');
    let mut s = format!("{mover}{}-{}", sq_alg(mv.from), sq_alg(mv.to));
    match board.piece_at(mv.to) {
        Some(c) => {
            s += &format!(
                " x{}({})",
                piece_letter(c.piece_type),
                c.piece_type.eval_value()
            )
        }
        None if mv.flags.capture => s += " x(ep)",
        None => s += " quiet",
    }
    if let Some(p) = mv.promotion {
        s += &format!(" ={}", piece_letter(p));
    }
    s
}

/// SEE of a move if it is a capture (best-case for the mover, centipawns); None if quiet.
fn see_of_move(lm: &LineMap, board: &Board, mv: Move) -> Option<i16> {
    let victim = board.piece_at(mv.to)?;
    let attacker = board.piece_at(mv.from)?;
    Some(see_capture(
        lm,
        mv.to,
        victim.piece_type.eval_value(),
        victim.player,
        attacker.player,
    ))
}

/// EXP-004: quality metric, not exact-match. Per fixture: SEE of the human and engine moves
/// (winning/losing capture) and the engine's value gap between its pick and the human move (how
/// strongly it disagrees). Separates "engine blundered" from "different reasonable move".
fn main() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines")
        .join("tactical_samples.json");
    let json = fs::read_to_string(&path).expect("read tactical_samples.json");
    let samples = extract_samples(&json);

    eprintln!("=== GATE QUALITY (depth 4, beam10+LMR+adaptive, 800k budget) ===");
    let mut lm = LineMap::new();
    let (mut tested, mut matches) = (0usize, 0usize);
    let (mut blunders, mut missed_wins, mut close, mut disagree) = (0usize, 0usize, 0usize, 0usize);
    // Calibration gate: how much a single move swings the mover's static eval. Quiet moves should
    // swing by ~tens (positional only); captures legitimately swing by ~piece value. Thousands = the
    // scale bug. Tracked separately for quiet vs capture moves.
    let (mut q_sum, mut q_max, mut q_n) = (0i64, 0i64, 0i64);
    let (mut c_sum, mut c_max, mut c_n) = (0i64, 0i64, 0i64);

    for sample in &samples {
        let moves_to_replay = sample.get("moves_to_replay").cloned().unwrap_or_default();
        let human_token = sample.get("human_move").cloned().unwrap_or_default();
        let id = sample.get("id").cloned().unwrap_or_default();
        if moves_to_replay.is_empty() || human_token.is_empty() {
            continue;
        }
        let Some(mut board) = replay_to_position(&moves_to_replay) else {
            continue;
        };
        let Some(human_mv) = decode_human_move(&human_token, &mut board) else {
            continue;
        };
        tested += 1;
        let mover = board.side_to_move.index();
        compute_lines(&board, &mut lm);
        let human_see = see_of_move(&lm, &board, human_mv);
        let human_desc = describe(&board, human_mv);

        let mut searcher = Searcher::new(16)
            .with_beam_width(10)
            .with_forward_pruning(true)
            .with_adaptive_beam(true)
            .with_node_budget(800_000);
        let rmv = searcher.root_move_values(&mut board, 4);
        let Some((best_mv, best_v)) = rmv.iter().copied().max_by_key(|(_, v)| v[mover]) else {
            eprintln!("[{id:>10}] no legal moves");
            continue;
        };
        let human_val = rmv
            .iter()
            .find(|(m, _)| *m == human_mv)
            .map(|(_, v)| v[mover]);
        let gap = human_val.map(|hv| best_v[mover] as i32 - hv as i32);
        let engine_see = see_of_move(&lm, &board, best_mv);
        let engine_desc = describe(&board, best_mv);
        let is_match = best_mv == human_mv;
        if is_match {
            matches += 1;
        }

        let verdict = if engine_see.is_some_and(|s| s < 0) {
            blunders += 1;
            "ENGINE-LOSES-MATERIAL"
        } else if human_see.is_some_and(|s| s > 0) && gap.is_some_and(|g| g > 0) {
            missed_wins += 1;
            "missed-win?"
        } else if gap.is_some_and(|g| g <= 30) {
            close += 1;
            "close"
        } else {
            disagree += 1;
            "disagree"
        };

        // EXP-005 calibration: the mover's STATIC eval after the human move vs after the engine
        // move. afterH << afterE → the static eval is miscalibrated; afterH ~ afterE but a huge
        // search gap → the depth-4 search (opponent replies) drives it.
        let pos_static = eval_4vec(&board, &mut lm)[mover];
        let u = board.make_move(human_mv);
        let after_h = eval_4vec(&board, &mut lm)[mover];
        board.unmake_move(u);
        let u = board.make_move(best_mv);
        let after_e = eval_4vec(&board, &mut lm)[mover];
        board.unmake_move(u);

        // Per-component diagnostic: which RAW query (M/P/S/O) swings on the human move.
        compute_lines(&board, &mut lm);
        let qv0 = run_all_queries(&lm, &board);
        let u = board.make_move(human_mv);
        compute_lines(&board, &mut lm);
        let qv1 = run_all_queries(&lm, &board);
        board.unmake_move(u);
        let dm = qv1.material[mover] as i32 - qv0.material[mover] as i32;
        let dp = qv1.positional[mover] as i32 - qv0.positional[mover] as i32;
        let ds = qv1.safety[mover] as i32 - qv0.safety[mover] as i32;
        let do_ = qv1.crossfire[mover] as i32 - qv0.crossfire[mover] as i32;

        let sh = (pos_static as i64 - after_h as i64).abs();
        if human_mv.flags.capture {
            c_sum += sh;
            c_max = c_max.max(sh);
            c_n += 1;
        } else {
            q_sum += sh;
            q_max = q_max.max(sh);
            q_n += 1;
        }
        let se = (pos_static as i64 - after_e as i64).abs();
        if best_mv.flags.capture {
            c_sum += se;
            c_max = c_max.max(se);
            c_n += 1;
        } else {
            q_sum += se;
            q_max = q_max.max(se);
            q_n += 1;
        }

        let hs = human_see
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".into());
        let g = gap.map(|g| g.to_string()).unwrap_or_else(|| "?".into());
        let _ = (after_e, g, &engine_desc);
        eprintln!(
            "[{id:>9}] H {human_desc:<19}(SEE {hs:<5}) swing={sh:<6} | rawΔ M={dm:<6} P={dp:<6} S={ds:<5} O={do_:<6} | {verdict}"
        );
    }

    eprintln!(
        "--- {matches}/{tested} match | blunders {blunders} | missed-wins {missed_wins} | close {close} | disagree {disagree} ---"
    );
    let qavg = if q_n > 0 { q_sum / q_n } else { 0 };
    let cavg = if c_n > 0 { c_sum / c_n } else { 0 };
    eprintln!(
        "--- CALIBRATION (eval stability): quiet-move swing avg={qavg} max={q_max} (n={q_n}) | capture-move swing avg={cavg} max={c_max} (n={c_n}) ---"
    );
    eprintln!(
        "    target after recalibration: quiet avg/max ~tens; captures bounded by piece value (<=~900)."
    );

    // ---- Blunder rate: replay the corpus, run the engine at each unique position, measure how
    // often its move loses material. A real (if coarse) play-quality signal to tune against, unlike
    // exact-move match. Capped for tractability. ----
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let (mut bp_pos, mut bp_capx, mut bp_hang) = (0usize, 0usize, 0usize);
    let mut hung_total = 0i64;
    'outer: for sample in &samples {
        let moves_to_replay = sample.get("moves_to_replay").cloned().unwrap_or_default();
        if moves_to_replay.is_empty() {
            continue;
        }
        let Ok(mut board) = fen4::parse(fen4::START_FEN4) else {
            continue;
        };
        for token in moves_to_replay.split_whitespace() {
            if bp_pos >= 150 {
                break 'outer; // cap
            }
            if seen.insert(board.zobrist) {
                let mover = board.side_to_move.index();
                compute_lines(&board, &mut lm);
                let before_risk = run_all_queries(&lm, &board).crossfire[mover] as i64;
                let mut s = Searcher::new(16)
                    .with_beam_width(10)
                    .with_forward_pruning(true)
                    .with_adaptive_beam(true)
                    .with_node_budget(250_000);
                if let Some((mv, _)) = s.search(&mut board, 4) {
                    bp_pos += 1;
                    if mv.flags.capture {
                        compute_lines(&board, &mut lm);
                        if see_of_move(&lm, &board, mv).is_some_and(|see| see < 0) {
                            bp_capx += 1; // captured into a loss
                        }
                    }
                    let undo = board.make_move(mv);
                    compute_lines(&board, &mut lm);
                    let after_risk = run_all_queries(&lm, &board).crossfire[mover] as i64;
                    board.unmake_move(undo);
                    let hung = (after_risk - before_risk).max(0);
                    hung_total += hung;
                    if hung > 200 {
                        bp_hang += 1; // move newly exposed > ~2 pawns of material
                    }
                }
            }
            if !apply_ply(&freyja_to_chesscom(token), &mut board) {
                break;
            }
        }
    }
    let avg_hung = if bp_pos > 0 {
        hung_total / bp_pos as i64
    } else {
        0
    };
    eprintln!(
        "--- BLUNDER RATE: {bp_pos} positions | capture-into-loss {bp_capx} | hangs(>200cp) {bp_hang} | avg newly-hung {avg_hung}cp ---"
    );
}
