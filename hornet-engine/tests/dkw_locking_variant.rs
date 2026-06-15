//! Pins the **locking variant** (EXP-026 rule 1, `HORNET_DKW_RULE=1`): a DKW player's pieces are
//! capturable (for no points) while their king walks; once the king falls, the remaining army is
//! **locked** — it stays on the board, un-capturable terrain.
//!
//! Rule 1 lost the EXP-026 arbitration (it is NOT chess.com's rule) but is kept as a
//! built, user-requested variant. It lives in its own integration-test binary because
//! `board::dkw_rule()` reads the env exactly once per process — this file's tests all run under
//! rule 1, isolated from the lib tests (which run under the default rule 2).

use hornet_engine::board::types::{Piece, PieceType, Player};
use hornet_engine::board::{Board, Square, dkw_rule};
use hornet_engine::move_gen::generate_legal;

fn at(s: &str) -> Square {
    Square::from_algebraic(s).unwrap()
}

/// One test fn so the env is set before the first `dkw_rule()` call, race-free.
#[test]
fn locking_variant_semantics() {
    // SAFETY/order: set before anything touches dkw_rule(); this binary runs only this test.
    unsafe { std::env::set_var("HORNET_DKW_RULE", "1") };
    assert_eq!(
        dkw_rule(),
        1,
        "this binary must run under the locking variant"
    );

    // --- While the king walks (DKW): pieces are capturable, for no points. ---
    let mut b = Board::empty();
    b.side_to_move = Player::Blue;
    b.set_piece(at("g7"), Some(Piece::new(Player::Blue, PieceType::Rook)));
    b.set_piece(at("g9"), Some(Piece::new(Player::Red, PieceType::Pawn))); // Red is DKW
    b.set_piece(at("g12"), Some(Piece::new(Player::Red, PieceType::King)));
    b.set_piece(at("a7"), Some(Piece::new(Player::Blue, PieceType::King)));
    b.set_piece(at("g14"), Some(Piece::new(Player::Yellow, PieceType::King)));
    b.set_piece(at("n8"), Some(Piece::new(Player::Green, PieceType::King)));
    b.recompute_zobrist();
    b.enter_dkw(Player::Red);

    let moves = generate_legal(&mut b);
    let cap = moves
        .iter()
        .find(|m| m.to == at("g9") && m.flags.capture)
        .copied()
        .expect("rule 1: a walking player's pawn IS capturable");
    b.make_move(cap);
    assert_eq!(
        b.points[Player::Blue.index()],
        0,
        "rule 1: capturing a DKW piece earns no points"
    );

    // --- After the king falls: the remaining army is LOCKED (un-capturable, persists). ---
    let mut b2 = Board::empty();
    b2.side_to_move = Player::Blue;
    b2.set_piece(at("g7"), Some(Piece::new(Player::Blue, PieceType::Rook)));
    b2.set_piece(at("g9"), Some(Piece::new(Player::Red, PieceType::Pawn))); // Red is DEAD
    b2.set_piece(at("g10"), Some(Piece::new(Player::Yellow, PieceType::Pawn))); // live, behind
    b2.set_piece(at("a7"), Some(Piece::new(Player::Blue, PieceType::King)));
    b2.set_piece(at("g14"), Some(Piece::new(Player::Yellow, PieceType::King)));
    b2.set_piece(at("n8"), Some(Piece::new(Player::Green, PieceType::King)));
    b2.recompute_zobrist();
    b2.retire_king(Player::Red); // marks dead; no king on board; army persists

    let moves2 = generate_legal(&mut b2);
    assert!(
        !moves2.iter().any(|m| m.to == at("g9")),
        "rule 1: a dead player's leftover piece is LOCKED — un-capturable"
    );
    assert!(
        !moves2.iter().any(|m| m.to == at("g10")),
        "rule 1: the locked piece still blocks the ray past it"
    );
    assert!(
        moves2.iter().any(|m| m.to == at("g8")),
        "the rook still slides up to the square before the locked piece"
    );
}
