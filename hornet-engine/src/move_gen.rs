//! Pseudo-legal and legal move generation, plus perft. See spec §1.4–1.6.
//!
//! Coverage: pawn (push / double-push / diagonal capture / en passant with the §1.6
//! orthogonality rule / promotion), knight, sliders (bishop / rook / queen), king steps,
//! and castling (§1.5, with through-check legality). King-capture elimination is handled
//! in `make_move`/`unmake_move`. **Dead-King-Walking (§1.7) is implemented**: a `board.dkw[p]`
//! mover generates king-only moves (no self-check filter; applied by `generate_legal`). DKW/dead
//! armies are immovable; their **capturability varies by rule variant** (`board::dkw_rule()`,
//! EXP-026 — see `is_wall`). `in_check` ignores DKW/dead armies in every variant (immovable
//! pieces threaten nothing). The full lifecycle lives in `game.rs`. See EXP-011/EXP-026.

use crate::board::attacks::is_attacked_by;
use crate::board::types::{PieceType, Player, Square};
use crate::board::{
    BISHOP_DIRS, Board, KING_DELTAS, KNIGHT_DELTAS, Move, MoveFlags, ROOK_DIRS, offset,
    pawn_capture_deltas, pawn_forward,
};

/// Whether the piece on `sq` is an **un-capturable wall** under the active DKW rule variant
/// (`board::dkw_rule()`, EXP-026). Walls always block movement; what varies is capturability:
///
/// - rule 0 (pre-EXP-026): a DKW player's non-king pieces are un-capturable (fully-frozen).
/// - rule 1: DKW pieces are capturable (for no points); a **dead** player's leftover army is
///   locked — un-capturable terrain (the user-hypothesis post-king-capture rule).
/// - rule 2: nothing is un-capturable (dead/DKW pieces capturable throughout, for no points).
fn is_wall(board: &Board, sq: Square) -> bool {
    let Some(p) = board.piece_at(sq) else {
        return false;
    };
    if p.piece_type == PieceType::King {
        return false;
    }
    match crate::board::dkw_rule() {
        0 => board.dkw[p.player.index()],
        1 => board.dead[p.player.index()],
        _ => false,
    }
}

/// Generate pseudo-legal moves for the side to move. Does **not** filter moves that
/// leave the mover's own king in check — see [`generate_legal`].
pub fn generate_pseudo_legal(board: &Board) -> Vec<Move> {
    let mover = board.side_to_move;
    let mut moves = Vec::with_capacity(48);
    if board.is_dkw(mover) {
        // Dead-King-Walking (§1.7): only the king moves — king steps, no castle. The
        // no-self-check filter is applied by `generate_legal` (a DKW king may step into check).
        // Captures earn no points (`make_move`). Under rules 1/2 (EXP-026) the walking king may
        // capture ANY capturable piece on an adjacent square — **including its own frozen army**
        // (corpus-observed: a dead king taking its own piece); under rule 0 frozen walls block it.
        if let Some(from) = board.king_square(mover) {
            if crate::board::dkw_rule() == 0 {
                gen_steps(board, mover, from, &KING_DELTAS, &mut moves);
            } else {
                for &(dr, df) in &KING_DELTAS {
                    let Some(to) = offset(from, dr, df) else {
                        continue;
                    };
                    if !to.is_valid() {
                        continue;
                    }
                    match board.piece_at(to) {
                        None => moves.push(Move::quiet(from, to)),
                        // Any capturable piece, own or enemy (own king can't be adjacent to
                        // itself; enemy king captures stay generatable — §1.8 elimination).
                        Some(_) if !is_wall(board, to) => moves.push(Move {
                            from,
                            to,
                            promotion: None,
                            flags: MoveFlags {
                                capture: true,
                                ..MoveFlags::default()
                            },
                        }),
                        Some(_) => {}
                    }
                }
            }
        }
        return moves;
    }
    for i in 0..196u8 {
        let from = Square::new(i);
        if !from.is_valid() {
            continue;
        }
        match board.piece_at(from) {
            Some(p) if p.player == mover => match p.piece_type {
                PieceType::Pawn => gen_pawn(board, mover, from, &mut moves),
                PieceType::Knight => gen_steps(board, mover, from, &KNIGHT_DELTAS, &mut moves),
                PieceType::King => {
                    gen_steps(board, mover, from, &KING_DELTAS, &mut moves);
                    gen_castles(board, mover, from, &mut moves);
                }
                PieceType::Bishop => gen_rays(board, mover, from, &BISHOP_DIRS, &mut moves),
                PieceType::Rook => gen_rays(board, mover, from, &ROOK_DIRS, &mut moves),
                PieceType::Queen | PieceType::PromotedQueen => {
                    gen_rays(board, mover, from, &BISHOP_DIRS, &mut moves);
                    gen_rays(board, mover, from, &ROOK_DIRS, &mut moves);
                }
            },
            _ => {}
        }
    }
    moves
}

/// Legal moves for the side to move (filters out moves leaving the mover in check).
/// Takes `&mut Board` so it can make/unmake in place; the board is unchanged on return.
pub fn generate_legal(board: &mut Board) -> Vec<Move> {
    let mover = board.side_to_move;
    if board.is_dkw(mover) {
        // A DKW king walks randomly and may step into check (it is already eliminated), so every
        // pseudo-legal king move counts as legal. (No-legal-moves here = DKW-king stalemate, §1.8.)
        return generate_pseudo_legal(board);
    }
    let mut legal = Vec::new();
    for mv in generate_pseudo_legal(board) {
        let undo = board.make_move(mv);
        if !in_check(board, mover) {
            legal.push(mv);
        }
        board.unmake_move(undo);
    }
    legal
}

/// Count leaf nodes to `depth` plies (each ply is the next live player's move).
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mover = board.side_to_move;
    let dkw = board.is_dkw(mover); // DKW king moves are all legal (no self-check filter)
    let mut nodes = 0;
    for mv in generate_pseudo_legal(board) {
        let undo = board.make_move(mv);
        if dkw || !in_check(board, mover) {
            nodes += perft(board, depth - 1);
        }
        board.unmake_move(undo);
    }
    nodes
}

/// Is `player`'s king currently attacked by an opponent that could actually capture it?
///
/// A **dead** opponent contributes nothing (king removed; its non-king pieces are inert walls). A
/// **DKW** opponent threatens only with its randomly-walking king (its frozen pieces can't capture).
/// A **live** opponent threatens with all its pieces.
pub fn in_check(board: &Board, player: Player) -> bool {
    let Some(k) = board.king_square(player) else {
        return false;
    };
    player.opponents().iter().any(|&opp| {
        let i = opp.index();
        if board.dead[i] {
            false
        } else if board.dkw[i] {
            king_adjacent(board, k, opp)
        } else {
            is_attacked_by(board, k, opp)
        }
    })
}

/// Is `by`'s king on a square adjacent to `sq`? (A DKW king's only threat is its king-step capture.)
fn king_adjacent(board: &Board, sq: Square, by: Player) -> bool {
    KING_DELTAS.iter().any(|&(dr, df)| {
        offset(sq, dr, df)
            .and_then(|s| board.piece_at(s))
            .is_some_and(|p| p.player == by && p.piece_type == PieceType::King)
    })
}

fn gen_steps(board: &Board, mover: Player, from: Square, deltas: &[(i8, i8)], out: &mut Vec<Move>) {
    for &(dr, df) in deltas {
        if let Some(to) = offset(from, dr, df) {
            if !to.is_valid() {
                continue;
            }
            match board.piece_at(to) {
                None => out.push(Move::quiet(from, to)),
                Some(p) if p.player != mover && !is_wall(board, to) => out.push(Move {
                    from,
                    to,
                    promotion: None,
                    flags: MoveFlags {
                        capture: true,
                        ..Default::default()
                    },
                }),
                _ => {} // own piece, or a frozen DKW/dead wall, blocks
            }
        }
    }
}

fn gen_rays(board: &Board, mover: Player, from: Square, dirs: &[(i8, i8)], out: &mut Vec<Move>) {
    for &(dr, df) in dirs {
        let mut cur = from;
        loop {
            match offset(cur, dr, df) {
                None => break,
                Some(next) => {
                    if !next.is_valid() {
                        break;
                    }
                    cur = next;
                    match board.piece_at(cur) {
                        None => out.push(Move::quiet(from, cur)),
                        Some(p) => {
                            if p.player != mover && !is_wall(board, cur) {
                                out.push(Move {
                                    from,
                                    to: cur,
                                    promotion: None,
                                    flags: MoveFlags {
                                        capture: true,
                                        ..Default::default()
                                    },
                                });
                            }
                            break; // any piece — enemy, own, or frozen wall — blocks the ray
                        }
                    }
                }
            }
        }
    }
}

fn gen_pawn(board: &Board, mover: Player, from: Square, out: &mut Vec<Move>) {
    let (fdr, fdf) = pawn_forward(mover);

    // Forward push (+ double push from the starting rank).
    if let Some(one) = offset(from, fdr, fdf)
        && one.is_valid()
        && board.piece_at(one).is_none()
    {
        push_pawn_move(mover, from, one, MoveFlags::default(), out);
        if on_start_rank(mover, from)
            && let Some(two) = offset(one, fdr, fdf)
            && two.is_valid()
            && board.piece_at(two).is_none()
        {
            out.push(Move {
                from,
                to: two,
                promotion: None,
                flags: MoveFlags {
                    double_push: true,
                    ..Default::default()
                },
            });
        }
    }

    // Diagonal captures and en passant.
    for (cdr, cdf) in pawn_capture_deltas(mover) {
        if let Some(to) = offset(from, cdr, cdf) {
            if !to.is_valid() {
                continue;
            }
            match board.piece_at(to) {
                Some(p) if p.player != mover && !is_wall(board, to) => {
                    push_pawn_move(
                        mover,
                        from,
                        to,
                        MoveFlags {
                            capture: true,
                            ..Default::default()
                        },
                        out,
                    );
                }
                None if board.en_passant == Some(to)
                    && board
                        .en_passant_pushing_player
                        .is_some_and(|pusher| ep_orthogonal(mover, pusher)) =>
                {
                    out.push(Move {
                        from,
                        to,
                        promotion: None,
                        flags: MoveFlags {
                            capture: true,
                            en_passant: true,
                            ..Default::default()
                        },
                    });
                }
                _ => {}
            }
        }
    }
}

/// Emit a pawn move, expanding to the four promotion choices when `to` is on the
/// player's promotion edge.
fn push_pawn_move(mover: Player, from: Square, to: Square, flags: MoveFlags, out: &mut Vec<Move>) {
    if on_promotion_edge(mover, to) {
        for promo in [
            PieceType::Queen,
            PieceType::Rook,
            PieceType::Bishop,
            PieceType::Knight,
        ] {
            out.push(Move {
                from,
                to,
                promotion: Some(promo),
                flags,
            });
        }
    } else {
        out.push(Move {
            from,
            to,
            promotion: None,
            flags,
        });
    }
}

fn on_start_rank(player: Player, sq: Square) -> bool {
    match player {
        Player::Red => sq.rank() == 1,
        Player::Blue => sq.file() == 1,
        Player::Yellow => sq.rank() == 12,
        Player::Green => sq.file() == 12,
    }
}

fn on_promotion_edge(player: Player, sq: Square) -> bool {
    // chess.com 4PC promotes at the **central crossing** (the far edge of the player's own
    // 8-square file/rank span), NOT the literal board edge. Spec §1.4 says the edge — that is
    // wrong (see CO-003); confirmed by replaying the 16-game corpus.
    match player {
        Player::Red => sq.rank() == 7,
        Player::Blue => sq.file() == 7,
        Player::Yellow => sq.rank() == 6,
        Player::Green => sq.file() == 6,
    }
}

/// En passant is only legal between players whose pawns move on perpendicular axes
/// (§1.6): Red↔Yellow and Blue↔Green (parallel axes) can never capture en passant.
fn ep_orthogonal(a: Player, b: Player) -> bool {
    let rank_axis = |p| matches!(p, Player::Red | Player::Yellow);
    rank_axis(a) != rank_axis(b)
}

/// Castle specs for `player`: `(is_kingside, king_to, empties, king_path)` per §1.5.
/// `empties` must be vacant; `king_path` (home, transit, destination) must be unattacked.
#[allow(clippy::type_complexity)]
fn castle_specs(
    player: Player,
) -> [(
    bool,
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
); 2] {
    match player {
        Player::Red => [
            (true, "j1", &["i1", "j1"], &["h1", "i1", "j1"]),
            (false, "f1", &["e1", "f1", "g1"], &["h1", "g1", "f1"]),
        ],
        Player::Blue => [
            (true, "a5", &["a5", "a6"], &["a7", "a6", "a5"]),
            (false, "a9", &["a8", "a9", "a10"], &["a7", "a8", "a9"]),
        ],
        Player::Yellow => [
            (true, "e14", &["e14", "f14"], &["g14", "f14", "e14"]),
            (false, "i14", &["h14", "i14", "j14"], &["g14", "h14", "i14"]),
        ],
        Player::Green => [
            (true, "n10", &["n9", "n10"], &["n8", "n9", "n10"]),
            (false, "n6", &["n5", "n6", "n7"], &["n8", "n7", "n6"]),
        ],
    }
}

/// The king's destination square for a castle (useful for decoding `O-O` / `O-O-O`).
pub fn castle_king_destination(player: Player, kingside: bool) -> Square {
    let specs = castle_specs(player);
    let (_, king_to, _, _) = if kingside { specs[0] } else { specs[1] };
    Square::from_algebraic(king_to).expect("valid castle square")
}

/// Emit legal castle moves (king home, rights held, path empty, king not in/through/into check).
fn gen_castles(board: &Board, mover: Player, king_from: Square, out: &mut Vec<Move>) {
    let sq = |s: &str| Square::from_algebraic(s).expect("valid castle square");
    for (is_kingside, king_to, empties, king_path) in castle_specs(mover) {
        let has_right = if is_kingside {
            board.castle_kingside[mover.index()]
        } else {
            board.castle_queenside[mover.index()]
        };
        if !has_right {
            continue;
        }
        if king_from != sq(king_path[0]) {
            continue; // king not on its home square
        }
        if empties.iter().any(|s| board.piece_at(sq(s)).is_some()) {
            continue; // squares between king and rook not all empty
        }
        // Mirror `in_check` (EXP-026/028): a dead opponent threatens nothing; a DKW opponent
        // threatens only with its walking king (its immovable army gives no check — the raw
        // attack scan would phantom-block castles chess.com allows); a live opponent threatens
        // with everything.
        let attacked = king_path.iter().any(|s| {
            let path_sq = sq(s);
            mover.opponents().iter().any(|&o| {
                let i = o.index();
                if board.dead[i] {
                    false
                } else if board.dkw[i] {
                    king_adjacent(board, path_sq, o)
                } else {
                    is_attacked_by(board, path_sq, o)
                }
            })
        });
        if attacked {
            continue; // king is in / passes through / lands in check
        }
        out.push(Move {
            from: king_from,
            to: sq(king_to),
            promotion: None,
            flags: MoveFlags {
                castle: true,
                ..Default::default()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;

    fn start() -> Board {
        fen4::parse(fen4::START_FEN4).unwrap()
    }

    #[test]
    fn red_has_20_opening_moves() {
        let b = start();
        // No checks possible at the start, so pseudo-legal == legal here.
        assert_eq!(generate_pseudo_legal(&b).len(), 20);
        let mut b2 = b.clone();
        assert_eq!(generate_legal(&mut b2).len(), 20);
    }

    #[test]
    fn make_unmake_restores_board_for_all_opening_moves() {
        let mut b = start();
        let original = b.clone();
        for mv in generate_pseudo_legal(&b) {
            let undo = b.make_move(mv);
            b.unmake_move(undo);
            assert_eq!(b, original, "make/unmake changed the board for {mv:?}");
        }
    }

    #[test]
    fn double_push_sets_and_clears_en_passant() {
        let mut b = start();
        // Red d2-d4 (double push) should arm the EP target d3.
        let d2 = Square::from_algebraic("d2").unwrap();
        let mv = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.from == d2 && m.flags.double_push)
            .expect("d2 double push exists");
        let undo = b.make_move(mv);
        assert_eq!(b.en_passant, Square::from_algebraic("d3"));
        assert_eq!(b.en_passant_pushing_player, Some(Player::Red));
        assert_eq!(b.side_to_move, Player::Blue);
        b.unmake_move(undo);
        assert_eq!(b.en_passant, None);
        assert_eq!(b.side_to_move, Player::Red);
    }

    #[test]
    fn perft_matches_known_values() {
        // Independently reproduces Freyja's invariants (clean rebuild, zero shared code).
        let mut b = start();
        assert_eq!(perft(&mut b, 1), 20);
        assert_eq!(perft(&mut b, 2), 395);
        assert_eq!(perft(&mut b, 3), 7800);
        assert_eq!(perft(&mut b, 4), 152050);
    }

    /// Documents why perft(2) = 395, not 400. Three Red openings reduce Blue's 20 replies:
    /// - `d2-d4` (-1): occupancy — d4 blocks Blue's `b4-d4` double push.
    /// - `f2-f3` / `f2-f4` (-2 each): vacating f2 opens the Red queen's g1-diagonal
    ///   (g1-f2-e3-d4-c5-b6), **pinning Blue's b6 pawn against its king on a7** — both of
    ///   b6's pushes become illegal. 1 + 2 + 2 = 5, so 400 - 5 = 395.
    #[test]
    fn opening_pin_explains_perft2() {
        let mut b = start();
        let child = |b: &mut Board, from: &str, to: &str| {
            let mv = generate_pseudo_legal(b)
                .into_iter()
                .find(|m| {
                    m.from == Square::from_algebraic(from).unwrap()
                        && m.to == Square::from_algebraic(to).unwrap()
                })
                .unwrap();
            let undo = b.make_move(mv);
            let n = perft(b, 1);
            b.unmake_move(undo);
            n
        };
        assert_eq!(child(&mut b, "d2", "d4"), 19, "d4 blocks b4-d4");
        assert_eq!(
            child(&mut b, "f2", "f3"),
            18,
            "f2-f3 pins b6 via the g1 queen"
        );
        assert_eq!(
            child(&mut b, "f2", "f4"),
            18,
            "f2-f4 pins b6 via the g1 queen"
        );
        // A control move that changes nothing for Blue.
        assert_eq!(child(&mut b, "h2", "h3"), 20);
    }

    #[test]
    fn make_unmake_handles_a_capture() {
        use crate::board::types::{Piece, PieceType};
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        let g7 = Square::from_algebraic("g7").unwrap();
        let g10 = Square::from_algebraic("g10").unwrap();
        b.set_piece(g7, Some(Piece::new(Player::Red, PieceType::Rook)));
        b.set_piece(g10, Some(Piece::new(Player::Blue, PieceType::Knight)));
        let original = b.clone();

        let cap = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.from == g7 && m.to == g10)
            .expect("rook can capture the knight on g10");
        assert!(cap.flags.capture);

        let undo = b.make_move(cap);
        assert_eq!(
            b.piece_at(g10),
            Some(Piece::new(Player::Red, PieceType::Rook))
        );
        assert_eq!(b.piece_at(g7), None);
        assert_eq!(
            b.points[Player::Red.index()],
            PieceType::Knight.ffa_points() as u16
        );
        b.unmake_move(undo);
        assert_eq!(
            b, original,
            "capture make/unmake must fully restore the board"
        );
    }

    #[test]
    fn castling_generates_and_round_trips() {
        use crate::board::types::{Piece, PieceType};
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        b.castle_kingside[Player::Red.index()] = true;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        b.set_piece(at("h1"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("k1"), Some(Piece::new(Player::Red, PieceType::Rook)));
        let original = b.clone();

        let castle = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.flags.castle)
            .expect("Red kingside castle should be generated");
        assert_eq!(castle.to, at("j1"));

        let undo = b.make_move(castle);
        assert_eq!(
            b.piece_at(at("j1")),
            Some(Piece::new(Player::Red, PieceType::King))
        );
        assert_eq!(
            b.piece_at(at("i1")),
            Some(Piece::new(Player::Red, PieceType::Rook))
        );
        assert_eq!(b.piece_at(at("h1")), None);
        assert_eq!(b.piece_at(at("k1")), None);
        assert!(!b.castle_kingside[Player::Red.index()]);
        b.unmake_move(undo);
        assert_eq!(
            b, original,
            "castle make/unmake must fully restore the board"
        );
    }

    #[test]
    fn castling_blocked_through_check() {
        use crate::board::types::{Piece, PieceType};
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        b.castle_kingside[Player::Red.index()] = true;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        b.set_piece(at("h1"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("k1"), Some(Piece::new(Player::Red, PieceType::Rook)));
        // Blue rook on i14 attacks down the i-file through i1 (the king's transit square).
        b.set_piece(at("i14"), Some(Piece::new(Player::Blue, PieceType::Rook)));
        assert!(
            generate_pseudo_legal(&b).iter().all(|m| !m.flags.castle),
            "must not castle through an attacked square"
        );
    }

    #[test]
    fn en_passant_make_unmake() {
        use crate::board::types::{Piece, PieceType};
        // Correct §1.4 geometry (the §7.3 example has a typo — see CO-002):
        // Blue c4->e4 (East 2), EP target d4; Red pawn on e3 captures NW onto d4, removing e4.
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        b.set_piece(at("e3"), Some(Piece::new(Player::Red, PieceType::Pawn)));
        b.set_piece(at("e4"), Some(Piece::new(Player::Blue, PieceType::Pawn)));
        b.en_passant = Some(at("d4"));
        b.en_passant_pushing_player = Some(Player::Blue);
        let original = b.clone();

        let ep = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.flags.en_passant)
            .expect("Red e3 should capture en passant onto d4");
        assert_eq!(ep.to, at("d4"));

        let undo = b.make_move(ep);
        assert_eq!(
            b.piece_at(at("d4")),
            Some(Piece::new(Player::Red, PieceType::Pawn))
        );
        assert_eq!(
            b.piece_at(at("e4")),
            None,
            "the double-pushed Blue pawn is captured"
        );
        assert_eq!(b.piece_at(at("e3")), None);
        b.unmake_move(undo);
        assert_eq!(b, original, "EP make/unmake must fully restore the board");
    }

    #[test]
    fn en_passant_rejected_between_parallel_players() {
        use crate::board::types::{Piece, PieceType};
        // Red and Yellow move on the same (rank) axis — EP between them is illegal (§1.6).
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        b.set_piece(at("e3"), Some(Piece::new(Player::Red, PieceType::Pawn)));
        b.en_passant = Some(at("d4"));
        b.en_passant_pushing_player = Some(Player::Yellow); // parallel axis
        assert!(
            generate_pseudo_legal(&b)
                .iter()
                .all(|m| !m.flags.en_passant),
            "Red↔Yellow en passant must not be generated"
        );
    }

    #[test]
    fn promotion_make_unmake() {
        use crate::board::types::{Piece, PieceType};
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        // Red pawn on d7 promotes by pushing to d8 (Red's promotion edge = internal rank 7).
        b.set_piece(at("d7"), Some(Piece::new(Player::Red, PieceType::Pawn)));
        let original = b.clone();

        let moves = generate_pseudo_legal(&b);
        assert_eq!(
            moves
                .iter()
                .filter(|m| m.to == at("d8") && m.promotion.is_some())
                .count(),
            4,
            "Q/R/B/N promotion choices"
        );
        let queen_promo = *moves
            .iter()
            .find(|m| m.promotion == Some(PieceType::Queen))
            .unwrap();

        let undo = b.make_move(queen_promo);
        assert_eq!(
            b.piece_at(at("d8")),
            Some(Piece::new(Player::Red, PieceType::PromotedQueen)),
            "a queen promotion lands as PromotedQueen"
        );
        assert_eq!(b.piece_at(at("d7")), None);
        b.unmake_move(undo);
        assert_eq!(b, original);
    }

    #[test]
    fn king_capture_eliminates_and_unmake_restores() {
        use crate::board::types::{Piece, PieceType};
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        b.set_piece(at("g7"), Some(Piece::new(Player::Red, PieceType::Rook)));
        b.set_piece(at("g8"), Some(Piece::new(Player::Blue, PieceType::King)));
        let original = b.clone();

        let cap = generate_pseudo_legal(&b)
            .into_iter()
            .find(|m| m.to == at("g8"))
            .expect("rook captures the Blue king on g8");
        let undo = b.make_move(cap);
        assert!(
            b.dead[Player::Blue.index()],
            "capturing the king eliminates Blue"
        );
        assert_eq!(
            b.points[Player::Red.index()],
            PieceType::King.ffa_points() as u16
        );
        assert_eq!(b.king_square(Player::Blue), None);
        assert_eq!(
            b.side_to_move,
            Player::Yellow,
            "dead Blue is skipped in rotation"
        );
        b.unmake_move(undo);
        assert!(!b.dead[Player::Blue.index()]);
        assert_eq!(
            b, original,
            "king-capture make/unmake must fully restore the board"
        );
    }

    #[test]
    fn dkw_king_walks_and_may_take_even_its_own_pieces() {
        // EXP-026 rule 2 (corpus-arbitrated): the walking king may capture any adjacent piece,
        // including its OWN frozen army (corpus-observed: a dead king taking its own piece).
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        b.set_piece(at("g7"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("g8"), Some(Piece::new(Player::Red, PieceType::Pawn))); // own frozen pawn
        b.set_piece(at("h8"), Some(Piece::new(Player::Blue, PieceType::Pawn))); // live enemy
        b.recompute_zobrist();
        b.enter_dkw(Player::Red);

        let moves = generate_legal(&mut b);
        assert!(
            moves.iter().all(|m| m.from == at("g7")),
            "only the king moves — non-king pieces are immovable"
        );
        assert!(
            moves.iter().any(|m| m.to == at("g8") && m.flags.capture),
            "the DKW king MAY capture its own frozen pawn (rule 2, corpus-observed)"
        );
        assert!(
            moves.iter().any(|m| m.to == at("h8") && m.flags.capture),
            "and it can capture a live enemy pawn"
        );
        assert_eq!(moves.len(), 8, "all 8 of g7's neighbours are reachable");
    }

    #[test]
    fn dkw_pieces_are_capturable_for_no_points_and_block() {
        // EXP-026 rule 2: a DKW player's pieces are capturable (chess.com: "capturing dead
        // pieces does not earn points") but still physically block rays while on the board.
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        b.side_to_move = Player::Blue; // a LIVE mover
        b.set_piece(at("g7"), Some(Piece::new(Player::Blue, PieceType::Rook)));
        b.set_piece(at("g9"), Some(Piece::new(Player::Red, PieceType::Pawn))); // Red is DKW
        b.set_piece(at("g10"), Some(Piece::new(Player::Yellow, PieceType::Pawn))); // live, behind
        b.recompute_zobrist();
        b.enter_dkw(Player::Red);

        let moves = generate_legal(&mut b);
        let cap = moves
            .iter()
            .find(|m| m.to == at("g9") && m.flags.capture)
            .copied()
            .expect("a DKW pawn IS capturable by a live player (rule 2)");
        assert!(
            !moves.iter().any(|m| m.to == at("g10")),
            "the DKW pawn still blocks the ray past it while on the board"
        );
        assert!(
            moves.iter().any(|m| m.to == at("g8")),
            "the rook still slides up to the square before it"
        );
        b.make_move(cap);
        assert_eq!(
            b.points[Player::Blue.index()],
            0,
            "capturing a DKW player's piece earns no points (chess.com Help Center)"
        );
    }

    #[test]
    fn dkw_king_capture_awards_no_points() {
        use crate::board::types::{Piece, PieceType};
        let at = |s: &str| Square::from_algebraic(s).unwrap();
        let mut b = Board::empty();
        b.side_to_move = Player::Red;
        b.set_piece(at("g7"), Some(Piece::new(Player::Red, PieceType::King)));
        b.set_piece(at("h8"), Some(Piece::new(Player::Blue, PieceType::Queen)));
        b.recompute_zobrist();
        b.enter_dkw(Player::Red);

        let cap = generate_legal(&mut b)
            .into_iter()
            .find(|m| m.to == at("h8"))
            .expect("DKW king can capture the adjacent queen");
        b.make_move(cap);
        assert_eq!(
            b.points[Player::Red.index()],
            0,
            "§1.7: a DKW king earns no points for captures"
        );
        assert_eq!(
            b.piece_at(at("h8")),
            Some(Piece::new(Player::Red, PieceType::King))
        );
    }
}
