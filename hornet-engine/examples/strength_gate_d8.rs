use hornet_engine::board::Board;
use hornet_engine::board::fen4;
use hornet_engine::board::types::Player;
use hornet_engine::move_gen::generate_pseudo_legal;
use hornet_engine::search::Searcher;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Sample struct — unified view of the tactical sample
// ---------------------------------------------------------------------------
#[derive(Debug)]
struct Sample {
    id: String,
    name: String,
    moves_to_replay: String, // space-separated Freyja notation
    human_move_freyja: String,
    nextturn: String,
}

// ---------------------------------------------------------------------------
// JSON parsing — manual, no external deps
// ---------------------------------------------------------------------------
fn load_samples() -> Vec<Sample> {
    let tactical_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("baselines")
        .join("tactical_samples.json");
    let json_str = fs::read_to_string(&tactical_path).expect("read tactical_samples.json");

    let mut samples = Vec::new();
    let sample_objects = extract_json_objects_at_depth(&json_str, 2);

    for obj in sample_objects {
        let id = extract_json_string_field(&obj, "id").unwrap_or_default();
        let name = extract_json_string_field(&obj, "name").unwrap_or_default();
        let nextturn = extract_json_string_field(&obj, "nextturn").unwrap_or_default();

        // Human move: prefer human_move_freyja, fall back to human_move
        let human_move_freyja = extract_json_string_field(&obj, "human_move_freyja")
            .or_else(|| extract_json_string_field(&obj, "human_move"))
            .unwrap_or_default();

        // Replay moves: prefer moves_to_replay, fall back to extracting from sequences
        let moves_to_replay = extract_json_string_field(&obj, "moves_to_replay")
            .filter(|s| !s.is_empty() && s != "n/a")
            .or_else(|| {
                // Try human_moves_sequence
                extract_json_string_array(&obj, "human_moves_sequence")
                    .and_then(|arr| extract_moves_from_sequence(&arr))
            })
            .or_else(|| {
                // Try test_moves_sequence
                extract_json_string_array(&obj, "test_moves_sequence")
                    .and_then(|arr| extract_moves_from_sequence(&arr))
            })
            .or_else(|| {
                // Try test_moves_window
                extract_json_string_array(&obj, "test_moves_window")
                    .and_then(|arr| extract_moves_from_sequence(&arr))
            })
            .unwrap_or_default();

        if !id.is_empty() {
            samples.push(Sample {
                id,
                name,
                moves_to_replay,
                human_move_freyja,
                nextturn,
            });
        }
    }
    samples
}

/// Extract all JSON objects that are at exactly `target_depth` in the brace nesting.
fn extract_json_objects_at_depth(json: &str, target_depth: usize) -> Vec<String> {
    let mut objects = Vec::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut obj_start = None;

    for (i, c) in json.char_indices() {
        if c == '"' && !in_string {
            in_string = true;
        } else if c == '"' && in_string {
            // Check for escaped quote
            let mut backslash_count = 0;
            let mut j = i;
            while j > 0 {
                j -= 1;
                if json.as_bytes()[j] == b'\\' {
                    backslash_count += 1;
                } else {
                    break;
                }
            }
            if backslash_count % 2 == 0 {
                in_string = false;
            }
        } else if !in_string {
            if c == '{' {
                if depth == target_depth - 1 {
                    obj_start = Some(i);
                }
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == target_depth - 1 {
                    if let Some(start) = obj_start {
                        objects.push(json[start..=i].to_string());
                    }
                    obj_start = None;
                }
            }
        }
    }
    objects
}

fn extract_json_string_field(obj: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\": \"", field);
    let start = obj.find(&pattern)?;
    let val_start = start + pattern.len();
    let mut val = String::new();
    let mut chars = obj[val_start..].chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            break;
        } else if c == '\\' {
            if let Some(next) = chars.next() {
                val.push(next);
            }
        } else {
            val.push(c);
        }
    }
    Some(val)
}

fn extract_json_string_array(obj: &str, field: &str) -> Option<Vec<String>> {
    let pattern = format!("\"{}\": [", field);
    let start = obj.find(&pattern)?;
    let arr_start = start + pattern.len();
    // Find matching ]
    let mut depth = 1;
    let mut in_str = false;
    let mut arr_end = arr_start;
    for (i, c) in obj[arr_start..].char_indices() {
        if c == '"' && !in_str {
            in_str = true;
        } else if c == '"' && in_str {
            let mut bs = 0;
            let mut j = arr_start + i;
            while j > 0 {
                j -= 1;
                if obj.as_bytes()[j] == b'\\' {
                    bs += 1;
                } else {
                    break;
                }
            }
            if bs % 2 == 0 {
                in_str = false;
            }
        } else if !in_str {
            if c == '[' {
                depth += 1;
            } else if c == ']' {
                depth -= 1;
                if depth == 0 {
                    arr_end = arr_start + i;
                    break;
                }
            }
        }
    }
    let arr_content = &obj[arr_start..arr_end];
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_item_str = false;
    for c in arr_content.chars() {
        if c == '"' && !in_item_str {
            in_item_str = true;
            current.clear();
        } else if c == '"' && in_item_str {
            in_item_str = false;
            items.push(current.clone());
            current.clear();
        } else if in_item_str {
            if c == '\\' {
                // skip, next char is literal
            } else {
                current.push(c);
            }
        }
    }
    if items.is_empty() { None } else { Some(items) }
}

/// Extract Freyja moves from a sequence of strings like "Ql7xh3+" or "R84: Green Rk6-k14+"
fn extract_moves_from_sequence(arr: &[String]) -> Option<String> {
    let mut moves = Vec::new();
    for item in arr {
        let s = strip_prefixes(item);
        // Find square-square patterns: [a-n][1-9][0-4]? followed by optional x/- and [a-n][1-9][0-4]?
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Look for piece letter followed by square
            let mut j = i;
            if j < bytes.len() && bytes[j].is_ascii_uppercase() {
                j += 1;
            }
            // Try to parse first square
            let sq1_start = j;
            if sq1_start < bytes.len() && (b'a'..=b'n').contains(&bytes[sq1_start]) {
                let file1 = bytes[sq1_start] as char;
                let mut rank1_end = sq1_start + 1;
                while rank1_end < bytes.len() && bytes[rank1_end].is_ascii_digit() {
                    rank1_end += 1;
                }
                if rank1_end > sq1_start + 1 {
                    let rank1 = std::str::from_utf8(&bytes[sq1_start + 1..rank1_end]).unwrap_or("");
                    if let Ok(r) = rank1.parse::<u8>() {
                        if r >= 1 && r <= 14 {
                            // Look for separator (x or -) and second square
                            let mut k = rank1_end;
                            if k < bytes.len() && (bytes[k] == b'x' || bytes[k] == b'-') {
                                k += 1;
                            }
                            // Skip optional piece letter before second square
                            if k < bytes.len() && bytes[k].is_ascii_uppercase() {
                                k += 1;
                            }
                            let sq2_start = k;
                            if sq2_start < bytes.len() && (b'a'..=b'n').contains(&bytes[sq2_start])
                            {
                                let file2 = bytes[sq2_start] as char;
                                let mut rank2_end = sq2_start + 1;
                                while rank2_end < bytes.len() && bytes[rank2_end].is_ascii_digit() {
                                    rank2_end += 1;
                                }
                                if rank2_end > sq2_start + 1 {
                                    let rank2 =
                                        std::str::from_utf8(&bytes[sq2_start + 1..rank2_end])
                                            .unwrap_or("");
                                    if let Ok(r2) = rank2.parse::<u8>() {
                                        if r2 >= 1 && r2 <= 14 {
                                            moves.push(format!(
                                                "{}{}{}{}",
                                                file1, rank1, file2, rank2
                                            ));
                                            i = rank2_end;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            i += 1;
        }
    }
    if moves.is_empty() {
        None
    } else {
        Some(moves.join(" "))
    }
}

fn strip_prefixes(s: &str) -> String {
    let mut s = s.to_string();
    // Remove "R84: " prefix
    if let Some(pos) = s.find(':') {
        s = s[pos + 1..].trim().to_string();
    }
    // Remove color names
    for color in ["Red ", "Blue ", "Yellow ", "Green "] {
        if s.starts_with(color) {
            s = s[color.len()..].to_string();
        }
    }
    // Remove parentheticals
    let mut result = String::new();
    let mut in_paren = false;
    for c in s.chars() {
        if c == '(' {
            in_paren = true;
        } else if c == ')' {
            in_paren = false;
        } else if !in_paren {
            result.push(c);
        }
    }
    result
        .replace('+', "")
        .replace('#', "")
        .replace("=Q", "q")
        .replace("=D", "q")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// Move application (Freyja notation -> Board)
// ---------------------------------------------------------------------------

fn parse_freyja_squares(
    token: &str,
) -> Option<(
    String,
    String,
    Option<hornet_engine::board::types::PieceType>,
)> {
    let token = token.to_lowercase();
    let bytes = token.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    // Extract from square
    let file1 = bytes[0] as char;
    let mut rank_end = 2;
    while rank_end < bytes.len() && bytes[rank_end].is_ascii_digit() {
        rank_end += 1;
    }
    if rank_end == 1 {
        rank_end = 2;
    }
    let rank1 = std::str::from_utf8(&bytes[1..rank_end]).unwrap_or("");
    let from_str = format!("{}{}", file1, rank1);

    // Extract to square
    if rank_end >= bytes.len() {
        return None;
    }
    let file2 = bytes[rank_end] as char;
    let mut rank2_end = rank_end + 2;
    while rank2_end < bytes.len() && bytes[rank2_end].is_ascii_digit() {
        rank2_end += 1;
    }
    if rank2_end == rank_end + 1 {
        rank2_end = rank_end + 2;
    }
    let rank2 = std::str::from_utf8(&bytes[rank_end + 1..rank2_end]).unwrap_or("");
    let to_str = format!("{}{}", file2, rank2);

    // Promotion
    let promo = if rank2_end < bytes.len() {
        match bytes[rank2_end] as char {
            'q' | 'd' => Some(hornet_engine::board::types::PieceType::Queen),
            'r' => Some(hornet_engine::board::types::PieceType::Rook),
            'b' => Some(hornet_engine::board::types::PieceType::Bishop),
            'n' => Some(hornet_engine::board::types::PieceType::Knight),
            _ => None,
        }
    } else {
        None
    };

    Some((from_str, to_str, promo))
}

fn apply_freyja_move(token: &str, board: &mut Board) -> bool {
    let Some((from_str, to_str, promo)) = parse_freyja_squares(token) else {
        return false;
    };

    let Some(from) = hornet_engine::board::Square::from_algebraic(&from_str) else {
        return false;
    };
    let Some(to) = hornet_engine::board::Square::from_algebraic(&to_str) else {
        return false;
    };

    let Some(p) = board.piece_at(from) else {
        return false;
    };

    // Special case: castling (king moves 2 squares horizontally)
    let is_castling = p.piece_type == hornet_engine::board::types::PieceType::King
        && ((from.file() as i8 - to.file() as i8).abs() == 2);

    if is_castling {
        // Determine kingside vs queenside by matching against known destinations
        let saved_stm = board.side_to_move;
        board.side_to_move = p.player;
        let ks_dest = hornet_engine::move_gen::castle_king_destination(p.player, true);
        let qs_dest = hornet_engine::move_gen::castle_king_destination(p.player, false);
        let kingside = to == ks_dest;
        let queenside = to == qs_dest;
        let dest = if kingside {
            ks_dest
        } else if queenside {
            qs_dest
        } else {
            board.side_to_move = saved_stm;
            return false;
        };
        let mv = generate_pseudo_legal(board)
            .into_iter()
            .find(|m| m.flags.castle && m.to == dest);
        board.side_to_move = saved_stm;
        match mv {
            Some(m) => {
                board.make_move(m);
                true
            }
            None => false,
        }
    } else {
        let Some(p) = board.piece_at(from) else {
            return false;
        };
        let saved_stm = board.side_to_move;
        board.side_to_move = p.player;
        let mv = generate_pseudo_legal(board)
            .into_iter()
            .find(|m| m.from == from && m.to == to && m.promotion == promo);
        board.side_to_move = saved_stm;
        match mv {
            Some(m) => {
                board.make_move(m);
                true
            }
            None => false,
        }
    }
}

fn replay_to_position(moves_to_replay: &str) -> Option<Board> {
    let mut board = fen4::parse(fen4::START_FEN4).unwrap();
    for token in moves_to_replay.split_whitespace() {
        if !apply_freyja_move(token, &mut board) {
            eprintln!("Failed to apply: {} (stm={:?})", token, board.side_to_move);
            // Debug: if it looks like castling, print king position and rights

            return None;
        }
    }
    Some(board)
}

fn decode_human_move_freyja(token: &str, board: &mut Board) -> Option<hornet_engine::board::Move> {
    let (from_str, to_str, promo) = parse_freyja_squares(token)?;
    let from = hornet_engine::board::Square::from_algebraic(&from_str)?;
    let to = hornet_engine::board::Square::from_algebraic(&to_str)?;

    let p = board.piece_at(from)?;
    let saved_stm = board.side_to_move;
    board.side_to_move = p.player;
    let mv = generate_pseudo_legal(board)
        .into_iter()
        .find(|m| m.from == from && m.to == to && m.promotion == promo);
    board.side_to_move = saved_stm;
    mv
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
fn main() {
    let samples = load_samples();
    let mut results = Vec::new();
    let mut matches = 0;
    let mut fails = 0;
    let mut skipped = 0;

    println!("=== STRENGTH GATE (depth 4 + quiescence) ===");
    println!("Testing {} tactical samples", samples.len());
    println!();

    for sample in &samples {
        if sample.moves_to_replay.is_empty() {
            println!(
                "[SKIP] {} {}: no replay moves available",
                sample.id, sample.name
            );
            skipped += 1;
            continue;
        }
        if sample.human_move_freyja.is_empty() || sample.human_move_freyja == "n/a" {
            println!(
                "[SKIP] {} {}: no human move available",
                sample.id, sample.name
            );
            skipped += 1;
            continue;
        }

        let mut board = match replay_to_position(&sample.moves_to_replay) {
            Some(b) => b,
            None => {
                println!("[FAIL] {} {}: replay failed", sample.id, sample.name);
                fails += 1;
                continue;
            }
        };

        let expected_player = match sample.nextturn.as_str() {
            "Yellow" => Some(Player::Yellow),
            "Green" => Some(Player::Green),
            "Red" => Some(Player::Red),
            "Blue" => Some(Player::Blue),
            _ => None,
        };
        if let Some(exp) = expected_player {
            if board.side_to_move != exp {
                eprintln!(
                    "{}: side_to_move mismatch: got {:?}, expected {:?}",
                    sample.id, board.side_to_move, exp
                );
            }
        }

        let human_mv = match decode_human_move_freyja(&sample.human_move_freyja, &mut board) {
            Some(m) => m,
            None => {
                println!(
                    "[FAIL] {} {}: decode failed: {}",
                    sample.id, sample.name, sample.human_move_freyja
                );
                fails += 1;
                continue;
            }
        };

        let mut searcher = Searcher::new(16).with_quiescence(true);
        let result = searcher.search(&mut board, 4);
        let engine_mv_str = match result {
            Some((mv, _)) => {
                let from = mv.from.to_algebraic();
                let to = mv.to.to_algebraic();
                let capture = if mv.flags.capture { "x" } else { "-" };
                format!("{}{}{}", from, capture, to)
            }
            None => "(no legal moves)".to_string(),
        };
        let matched = result.map(|(mv, _)| mv == human_mv).unwrap_or(false);
        if matched {
            matches += 1;
        }
        results.push((
            sample.id.clone(),
            sample.name.clone(),
            matched,
            engine_mv_str.clone(),
            sample.human_move_freyja.clone(),
        ));
        let status = if matched { "MATCH" } else { "MISS" };
        println!(
            "[{}] {} {}: engine={} human={}",
            status, sample.id, sample.name, engine_mv_str, sample.human_move_freyja
        );
    }

    let tested = results.len();
    println!();
    println!("=== RESULTS ===");
    println!(
        "Match rate: {}/{} = {:.1}% ({} failed, {} skipped)",
        matches,
        tested,
        (matches as f64 / tested.max(1) as f64) * 100.0,
        fails,
        skipped
    );

    let misses: Vec<_> = results.iter().filter(|(_, _, m, _, _)| !m).collect();
    if !misses.is_empty() {
        println!();
        println!("=== MISSES ===");
        for (id, name, _, engine, human) in misses {
            println!("  {} ({}): engine={} human={}", id, name, engine, human);
        }
    }
}
