use hornet_engine::board::Board;
use hornet_engine::board::types::{Piece, PieceType, Player, Square};
use hornet_engine::intent::compute_intent_map;
use hornet_engine::lines::{LineMap, compute_lines};

fn main() {
    println!("=== TURN-ORDER-RELATIVE VULNERABILITY (FIXED) ===");
    println!();
    println!("Turn order: Red -> Blue -> Yellow -> Green");
    println!();

    // Case 1: Red queen threatened by BLUE (next player)
    let mut b1 = Board::empty();
    b1.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Queen)),
    );
    b1.set_piece(
        Square::from_algebraic("g1").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Rook)),
    );

    let mut lm1 = LineMap::new();
    compute_lines(&b1, &mut lm1);
    let map1 = compute_intent_map(&lm1, &b1);

    let red_q = map1
        .intents
        .iter()
        .find(|pi| pi.piece.player == Player::Red && pi.piece.piece_type == PieceType::Queen)
        .unwrap();

    println!("Case 1: Red queen on g7, Blue rook on g1 (Blue moves NEXT)");
    println!(
        "  Vulnerability vs Blue: {}  <-- HIGHEST (immediate threat)",
        red_q.intent_vs(Player::Blue).unwrap().vulnerability
    );
    println!();

    // Case 2: Red queen threatened by YELLOW (2 moves away)
    let mut b2 = Board::empty();
    b2.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Queen)),
    );
    b2.set_piece(
        Square::from_algebraic("a7").unwrap(),
        Some(Piece::new(Player::Yellow, PieceType::Rook)),
    );

    let mut lm2 = LineMap::new();
    compute_lines(&b2, &mut lm2);
    let map2 = compute_intent_map(&lm2, &b2);

    let red_q2 = map2
        .intents
        .iter()
        .find(|pi| pi.piece.player == Player::Red && pi.piece.piece_type == PieceType::Queen)
        .unwrap();

    println!("Case 2: Red queen on g7, Yellow rook on a7 (Yellow moves in 2 turns)");
    println!(
        "  Vulnerability vs Yellow: {}  <-- MEDIUM (1 chance to escape)",
        red_q2.intent_vs(Player::Yellow).unwrap().vulnerability
    );
    println!();

    // Case 3: Red queen threatened by GREEN (3 moves away)
    let mut b3 = Board::empty();
    b3.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Queen)),
    );
    b3.set_piece(
        Square::from_algebraic("m7").unwrap(),
        Some(Piece::new(Player::Green, PieceType::Rook)),
    );

    let mut lm3 = LineMap::new();
    compute_lines(&b3, &mut lm3);
    let map3 = compute_intent_map(&lm3, &b3);

    let red_q3 = map3
        .intents
        .iter()
        .find(|pi| pi.piece.player == Player::Red && pi.piece.piece_type == PieceType::Queen)
        .unwrap();

    println!("Case 3: Red queen on g7, Green rook on m7 (Green moves in 3 turns)");
    println!(
        "  Vulnerability vs Green: {}  <-- LOWEST (2 chances to escape)",
        red_q3.intent_vs(Player::Green).unwrap().vulnerability
    );
    println!();

    // Case 4: ALL THREE attacking
    let mut b4 = Board::empty();
    b4.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Queen)),
    );
    b4.set_piece(
        Square::from_algebraic("g1").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Rook)),
    );
    b4.set_piece(
        Square::from_algebraic("a7").unwrap(),
        Some(Piece::new(Player::Yellow, PieceType::Rook)),
    );
    b4.set_piece(
        Square::from_algebraic("m7").unwrap(),
        Some(Piece::new(Player::Green, PieceType::Rook)),
    );

    let mut lm4 = LineMap::new();
    compute_lines(&b4, &mut lm4);
    let map4 = compute_intent_map(&lm4, &b4);

    let red_q4 = map4
        .intents
        .iter()
        .find(|pi| pi.piece.player == Player::Red && pi.piece.piece_type == PieceType::Queen)
        .unwrap();

    let v_blue = red_q4.intent_vs(Player::Blue).unwrap().vulnerability;
    let v_yellow = red_q4.intent_vs(Player::Yellow).unwrap().vulnerability;
    let v_green = red_q4.intent_vs(Player::Green).unwrap().vulnerability;

    println!("Case 4: Red queen on g7, ALL THREE rooks attacking");
    println!("  vs Blue (next):     {}  (weight 1.0)", v_blue);
    println!("  vs Yellow (2 away): {}  (weight 0.6)", v_yellow);
    println!("  vs Green (3 away):  {}  (weight 0.3)", v_green);
    println!("  TOTAL:              {}", v_blue + v_yellow + v_green);
    println!();

    println!("=== RATIOS ===");
    println!("  Blue / Yellow = {:.2}x", v_blue as f32 / v_yellow as f32);
    println!("  Blue / Green  = {:.2}x", v_blue as f32 / v_green as f32);
    println!(
        "  Yellow / Green = {:.2}x",
        v_yellow as f32 / v_green as f32
    );
    println!();
    println!("Expected ratios: 1.0 / 0.6 / 0.3 = 1.67x / 3.33x / 2.0x");
}
