//! Max^n search (spec §6). Each node maximizes the **moving player's own component** of the
//! per-player value vector `V = ⟨U1, U2, U3, U4⟩` — the vector is backed up whole, never
//! collapsed to a scalar (Hard Rule #3). Leaves are scored by P5's [`eval_4vec`].
//!
//! Baseline: beam Max^n — at internal nodes expand the top-`beam_width` ordered moves (§6.1),
//! at the **root** consider all legal moves (so a good move ordered past the beam is never
//! dropped). The transposition table is used for **move ordering only** (best-move hint); the
//! beam makes node values approximate, so there is no value cutoff until Max^n shallow pruning
//! adds real bounds. Proper checkmate/stalemate terminal scoring (§1.8) and iterative deepening
//! are further refinements.
//!
//! Depth should be a multiple of 4 (Hard Rule #1) so the perspective chain ends on a full
//! 4-player rotation; the recursion itself accepts any depth.

use crate::board::{Board, Move};
use crate::eval::eval_4vec;
use crate::lines::LineMap;
use crate::move_gen::generate_legal;
use crate::move_order;
use crate::tt::{Bound, TranspositionTable};

/// Default beam width (spec appendix `DEFAULT_BEAM_WIDTH`).
const DEFAULT_BEAM_WIDTH: usize = 30;

pub struct Searcher {
    tt: TranspositionTable,
    /// Reusable line buffer handed to the evaluator (always-recompute; one boxed buffer).
    lines: Box<LineMap>,
    /// Leaf evaluator. Defaults to [`eval_4vec`]; injectable so the Max^n backup can be tested
    /// with a controllable synthetic eval.
    eval: fn(&Board, &mut LineMap) -> [i16; 4],
    /// Per-node beam width: expand only the top-N ordered moves (§6.1).
    beam_width: usize,
    /// Nodes visited in the last `search` call.
    pub nodes: u64,
}

impl Searcher {
    pub fn new(tt_mb: usize) -> Self {
        Searcher {
            tt: TranspositionTable::new(tt_mb),
            lines: Box::new(LineMap::new()),
            eval: eval_4vec,
            beam_width: DEFAULT_BEAM_WIDTH,
            nodes: 0,
        }
    }

    /// Override the per-node beam width.
    pub fn with_beam_width(mut self, width: usize) -> Self {
        self.beam_width = width.max(1);
        self
    }

    /// Inject a leaf evaluator (used to test the Max^n backup with a controllable eval).
    #[cfg(test)]
    fn with_eval(mut self, eval: fn(&Board, &mut LineMap) -> [i16; 4]) -> Self {
        self.eval = eval;
        self
    }

    /// Search the position; return the best move for the side to move and its value vector,
    /// or `None` if there are no legal moves.
    pub fn search(&mut self, board: &mut Board, depth: u32) -> Option<(Move, [i16; 4])> {
        let depth = round_to_rotation(depth); // Hard Rule #1: search full 4-player rotations.
        self.nodes = 0;
        let mover = board.side_to_move.index();
        let mut moves = generate_legal(board);
        if moves.is_empty() {
            return None;
        }
        let tt_move = self.tt.probe(board.zobrist).and_then(|e| e.best_move);
        move_order::order(board, &mut moves, tt_move);

        // The root considers ALL legal moves (no beam) so a strong move is never dropped.
        let mut best: Option<(Move, [i16; 4])> = None;
        for mv in moves {
            let undo = board.make_move(mv);
            let child = self.maxn(board, depth.saturating_sub(1));
            board.unmake_move(undo);
            let take = best.is_none_or(|(_, bv)| child[mover] > bv[mover]);
            if take {
                best = Some((mv, child));
            }
        }
        if let Some((mv, v)) = best {
            let key = board.zobrist;
            self.tt
                .store(key, clamp_depth(depth), v, Bound::Exact, Some(mv));
        }
        best
    }

    fn maxn(&mut self, board: &mut Board, depth: u32) -> [i16; 4] {
        self.nodes += 1;

        if depth == 0 {
            return (self.eval)(board, &mut self.lines);
        }

        let mover = board.side_to_move.index();
        let mut moves = generate_legal(board);
        if moves.is_empty() {
            // Terminal (no legal moves): static score for now (§1.8 scoring is a refinement).
            return (self.eval)(board, &mut self.lines);
        }
        let tt_move = self.tt.probe(board.zobrist).and_then(|e| e.best_move);
        move_order::order(board, &mut moves, tt_move);

        let mut best = [i16::MIN; 4];
        let mut best_mover = i32::MIN;
        let mut best_move = None;
        for mv in moves.into_iter().take(self.beam_width) {
            let undo = board.make_move(mv);
            let child = self.maxn(board, depth - 1);
            board.unmake_move(undo);
            if i32::from(child[mover]) > best_mover {
                best_mover = i32::from(child[mover]);
                best = child;
                best_move = Some(mv);
            }
        }
        self.tt.store(
            board.zobrist,
            clamp_depth(depth),
            best,
            Bound::Exact,
            best_move,
        );
        best
    }
}

fn clamp_depth(depth: u32) -> u8 {
    depth.min(u8::MAX as u32) as u8
}

/// Round a requested depth up to the next positive multiple of 4, so the Max^n perspective
/// chain ends on a full 4-player rotation (Hard Rule #1: valid depths are 4, 8, 12, …).
fn round_to_rotation(depth: u32) -> u32 {
    depth.div_ceil(4).max(1) * 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::board::types::{Piece, PieceType, Player, Square};

    fn at(s: &str) -> Square {
        Square::from_algebraic(s).unwrap()
    }

    /// A sparse board with the four kings on their start squares + extra pieces.
    fn with_kings(extra: &[(&str, Player, PieceType)]) -> Board {
        let mut b = Board::empty();
        for (sq, pl) in [
            ("h1", Player::Red),
            ("a7", Player::Blue),
            ("g14", Player::Yellow),
            ("n8", Player::Green),
        ] {
            b.set_piece(at(sq), Some(Piece::new(pl, PieceType::King)));
        }
        for (sq, pl, pt) in extra {
            b.set_piece(at(sq), Some(Piece::new(*pl, *pt)));
        }
        b.recompute_zobrist();
        b
    }

    #[test]
    fn search_returns_a_legal_move_and_counts_nodes() {
        let mut b = with_kings(&[("g7", Player::Red, PieceType::Rook)]);
        let mut s = Searcher::new(8);
        let (mv, _v) = s.search(&mut b, 4).expect("has moves");
        assert!(s.nodes > 0);
        // The returned move must be one of the legal moves.
        assert!(generate_legal(&mut b).contains(&mv));
    }

    #[test]
    fn search_grabs_a_free_queen() {
        // Red rook on g7 can capture an undefended Blue queen on g10 up the file.
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("g10", Player::Blue, PieceType::Queen),
        ]);
        let mut s = Searcher::new(8);
        let (mv, v) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(mv.from, at("g7"));
        assert_eq!(mv.to, at("g10"), "Max^n should take the free queen");
        assert!(mv.flags.capture);
        // Red ends materially ahead of Blue.
        assert!(v[Player::Red.index()] > v[Player::Blue.index()]);
    }

    #[test]
    fn beam_keeps_the_best_capture() {
        // Even with a narrow beam, the MVV-LVA-ordered free-queen capture is expanded.
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("g10", Player::Blue, PieceType::Queen),
        ]);
        let mut s = Searcher::new(8).with_beam_width(3);
        let (mv, _) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(mv.to, at("g10"));
    }

    // --- Review/hardening: Max^n backup, root completeness, determinism, depth rounding ---

    fn find(board: &Board, player: Player, pt: PieceType) -> Option<Square> {
        (0..196u8)
            .map(Square::new)
            .find(|&sq| board.piece_at(sq) == Some(Piece::new(player, pt)))
    }

    /// Synthetic eval: Red's component = (Red king's file) × 100; others 0.
    fn red_king_file(board: &Board, _l: &mut LineMap) -> [i16; 4] {
        let f = find(board, Player::Red, PieceType::King).map_or(0, |s| s.file() as i16 * 100);
        [f, 0, 0, 0]
    }

    /// Synthetic eval: Blue's component = (Blue king's file) × 100; others 0.
    fn blue_king_file(board: &Board, _l: &mut LineMap) -> [i16; 4] {
        let f = find(board, Player::Blue, PieceType::King).map_or(0, |s| s.file() as i16 * 100);
        [0, f, 0, 0]
    }

    #[test]
    fn maxn_node_maximizes_the_movers_own_component() {
        // Four kings only. A Red node maximizes RED's own component (king file): from h1
        // (file 7) the best reachable file is 8 → 800.
        let mut b = with_kings(&[]);
        let mut s = Searcher::new(1).with_eval(red_king_file);
        assert_eq!(s.maxn(&mut b, 1), [800, 0, 0, 0]);

        // A Blue node maximizes BLUE's own component (not minimizing Red — the Max^n property,
        // vs paranoid minimax). Blue king a7 (file 0) reaches file 1 → 100.
        let mut bb = with_kings(&[]);
        bb.side_to_move = Player::Blue;
        bb.recompute_zobrist();
        let mut sb = Searcher::new(1).with_eval(blue_king_file);
        assert_eq!(sb.maxn(&mut bb, 1), [0, 100, 0, 0]);
    }

    #[test]
    fn root_considers_all_moves_not_just_the_beam() {
        // Red rook g7 can capture a Blue pawn on h7 (sorts first under MVV-LVA). With beam 1, a
        // beamed root would only try the capture; root-full-width still finds the king move that
        // maximizes Red's (synthetic) score — king h1 → file 8 (800) beats keeping it (700).
        let mut b = with_kings(&[
            ("g7", Player::Red, PieceType::Rook),
            ("h7", Player::Blue, PieceType::Pawn),
        ]);
        let mut s = Searcher::new(1).with_eval(red_king_file).with_beam_width(1);
        let (mv, v) = s.search(&mut b, 4).expect("has moves");
        assert_eq!(
            mv.from,
            at("h1"),
            "found the king move, not just the top-ordered capture"
        );
        assert_eq!(v[Player::Red.index()], 800);
    }

    #[test]
    fn fresh_searches_are_deterministic() {
        let mk = || {
            with_kings(&[
                ("g7", Player::Red, PieceType::Rook),
                ("g10", Player::Blue, PieceType::Queen),
            ])
        };
        let r1 = Searcher::new(8).with_beam_width(4).search(&mut mk(), 4);
        let r2 = Searcher::new(8).with_beam_width(4).search(&mut mk(), 4);
        assert_eq!(r1, r2, "two fresh searchers give identical results");
    }

    #[test]
    fn depth_rounds_up_to_a_full_rotation() {
        assert_eq!(round_to_rotation(0), 4);
        assert_eq!(round_to_rotation(1), 4);
        assert_eq!(round_to_rotation(4), 4);
        assert_eq!(round_to_rotation(5), 8);
        assert_eq!(round_to_rotation(8), 8);
    }
}
