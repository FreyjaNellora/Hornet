//! Protocol output formatting. Phase 8.

use crate::board::Move;
use crate::board::types::PieceType;

/// One `info` line for a ranked root candidate (UCI-style, 4PC-adapted). Per-seat scores use
/// `R/B/Y/G` keys; the full principal variation (`pv …`) is included only for the top candidate.
/// The schema is phase-agnostic — a UI parses key/value pairs and renders whatever is present, so
/// MCTS (`visits`/`winrate`) and NNUE fields can be appended later without breaking consumers.
pub fn format_info(
    rank: usize,
    depth: u32,
    score: [i16; 4],
    mv: &Move,
    piece: char,
    nodes: u64,
    nps: u64,
    time_ms: u128,
    pv: Option<&[Move]>,
) -> String {
    let mut s = format!(
        "info depth {depth} multipv {rank} score R {} B {} Y {} G {} nodes {nodes} nps {nps} time {time_ms} move {} piece {piece}",
        score[0],
        score[1],
        score[2],
        score[3],
        format_move(mv),
    );
    if let Some(pv) = pv {
        s.push_str(" pv");
        for m in pv {
            s.push(' ');
            s.push_str(&format_move(m));
        }
    }
    s
}

/// Format a move as chess.com-style long algebraic `from-to` (+ `=D`/`=R`/`=B`/`=N` for a
/// promotion). Castling is emitted as the king's `from-to` — unambiguous, and a driver re-decodes it
/// (the move generator self-syncs the mover and matches the castle by destination).
pub fn format_move(mv: &Move) -> String {
    let mut s = format!("{}-{}", mv.from.to_algebraic(), mv.to.to_algebraic());
    if let Some(p) = mv.promotion {
        let c = match p {
            PieceType::Queen | PieceType::PromotedQueen => 'D',
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            PieceType::Pawn | PieceType::King => '?',
        };
        s.push('=');
        s.push(c);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::fen4;
    use crate::move_gen::generate_legal;

    #[test]
    fn formats_from_to() {
        let mut b = fen4::parse(fen4::START_FEN4).unwrap();
        let mv = generate_legal(&mut b)[0];
        let s = format_move(&mv);
        let parts: Vec<&str> = s.split('-').collect();
        assert_eq!(parts.len(), 2, "expected from-to, got {s}");
        assert_eq!(parts[0], mv.from.to_algebraic());
        assert_eq!(parts[1], mv.to.to_algebraic());
    }

    #[test]
    fn info_line_schema() {
        let mut b = fen4::parse(fen4::START_FEN4).unwrap();
        let mv = generate_legal(&mut b)[0];
        let top = format_info(1, 8, [10, -5, 3, 0], &mv, 'P', 1234, 5000, 200, Some(&[mv]));
        assert!(
            top.starts_with(
                "info depth 8 multipv 1 score R 10 B -5 Y 3 G 0 nodes 1234 nps 5000 time 200 move "
            ),
            "schema: {top}"
        );
        assert!(top.contains(" piece P"), "carries the moving piece: {top}");
        assert!(top.contains(" pv "), "top candidate carries a pv: {top}");
        let rest = format_info(2, 8, [1, 2, 3, 4], &mv, 'N', 1234, 5000, 200, None);
        assert!(rest.contains("multipv 2"), "{rest}");
        assert!(!rest.contains(" pv"), "rank>1 has no pv: {rest}");
    }
}
