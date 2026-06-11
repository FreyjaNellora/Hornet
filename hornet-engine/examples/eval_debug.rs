use hornet_engine::board::types::{Piece, PieceType, Player, Square};
use hornet_engine::board::{Board, fen4};
use hornet_engine::bounty::bounty_crossfire;
use hornet_engine::eval::eval_4vec;
use hornet_engine::intent::{PieceIntent, aggregate_intent_for_eval, compute_intent_map};
use hornet_engine::lines::{LineMap, compute_lines};
use hornet_engine::queries::run_all_queries;

fn show_piece_tensor(pi: &PieceIntent) {
    let ops = pi.piece.player.opponents();
    println!(
        "    {:?} {:?} at {}:",
        pi.piece.player,
        pi.piece.piece_type,
        pi.square.to_algebraic()
    );
    for (i, &op) in ops.iter().enumerate() {
        let v = &pi.vs[i];
        println!(
            "      vs {:?}: offense={}, defense={}, vuln={}",
            op, v.offense, v.defense, v.vulnerability
        );
    }
}

fn analyze_position(name: &str, b: &Board) {
    let mut lm = Box::new(LineMap::new());
    compute_lines(b, &mut lm);

    let qv = run_all_queries(&lm, b);
    let bounty = bounty_crossfire(&lm, b);
    let intent_map = compute_intent_map(&lm, b);
    let intent_agg = aggregate_intent_for_eval(&intent_map);
    let eval = eval_4vec(b, &mut lm);

    println!("\n========================================");
    println!("  {}", name);
    println!("========================================");
    println!("EVAL: {:?}", eval);
    println!("\n--- Flat Components ---");
    println!("Material:      {:?}", qv.material);
    println!("Positional:    {:?}", qv.positional);
    println!("Safety:        {:?}", qv.safety);
    println!("Crossfire:     {:?}", qv.crossfire);
    println!("Bounty:        {:?}", bounty);

    println!("\n--- L3 Intent Aggregates ---");
    println!("Offense:       {:?}", intent_agg.offense);
    println!("Defense:       {:?}", intent_agg.defense);
    println!("Vulnerability: {:?}", intent_agg.vulnerability);
    println!("Net:           {:?}", intent_agg.net);

    // Show ALL piece tensors
    println!("\n--- L3 Per-Piece Tensors ---");
    for player in Player::ALL {
        println!("\n  {:?}'s pieces:", player);
        for pi in intent_map.intents[..intent_map.count]
            .iter()
            .filter(|pi| pi.piece.player == player)
            .filter(|pi| {
                // Only show pieces with non-trivial intent
                let has_off = pi.vs.iter().any(|v| v.offense > 0);
                let has_def = pi.vs.iter().any(|v| v.defense > 0);
                let has_vuln = pi.vs.iter().any(|v| v.vulnerability > 0);
                has_off || has_def || has_vuln
            })
        {
            show_piece_tensor(pi);
        }
    }
}

fn main() {
    // 1. Starting position
    let start = fen4::parse(fen4::START_FEN4).unwrap();
    analyze_position("Starting Position", &start);

    // 2. Undefended queen scenario - the classic tactical test
    let mut b3 = Board::empty();
    b3.set_piece(
        Square::from_algebraic("d7").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Queen)),
    );
    b3.set_piece(
        Square::from_algebraic("d1").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Rook)),
    );
    analyze_position("Undefended Queen vs Rook", &b3);

    // 3. Defended queen scenario
    let mut b4 = Board::empty();
    b4.set_piece(
        Square::from_algebraic("d7").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Queen)),
    );
    b4.set_piece(
        Square::from_algebraic("d1").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Rook)),
    );
    b4.set_piece(
        Square::from_algebraic("b5").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Bishop)),
    );
    analyze_position("Defended Queen vs Rook", &b4);

    // 4. Complex: Red queen fork - attacks two enemy pieces
    let mut b5 = Board::empty();
    b5.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Queen)),
    );
    b5.set_piece(
        Square::from_algebraic("g1").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Rook)),
    );
    b5.set_piece(
        Square::from_algebraic("a7").unwrap(),
        Some(Piece::new(Player::Yellow, PieceType::Knight)),
    );
    analyze_position("Red Queen Forks Blue Rook + Yellow Knight", &b5);

    // 5. Crossfire: one piece attacked by multiple enemies
    let mut b6 = Board::empty();
    b6.set_piece(
        Square::from_algebraic("g7").unwrap(),
        Some(Piece::new(Player::Red, PieceType::Knight)),
    );
    b6.set_piece(
        Square::from_algebraic("g1").unwrap(),
        Some(Piece::new(Player::Blue, PieceType::Rook)),
    );
    b6.set_piece(
        Square::from_algebraic("a7").unwrap(),
        Some(Piece::new(Player::Yellow, PieceType::Rook)),
    );
    analyze_position("Red Knight Crossfire (Blue+Yellow Rooks)", &b6);
}
