//! FEN4 parser / serializer — the chess.com 4PC dialect (Hornet's native format).
//!
//! Grammar: `<turn>-<dead>-<castleK>-<castleQ>-<points>-<extra>-<board>`
//!
//! - `turn`     — one of `R B Y G`.
//! - `dead`, `castleK`, `castleQ` — four `0`/`1` flags, comma-separated, RBYG order.
//! - `points`   — four integers, comma-separated, RBYG order.
//! - `extra`    — a single field (the lone `0` in the start position); preserved raw
//!   (see [`super::Board::extra`]).
//! - `board`    — 14 ranks separated by `/`, listed from **display rank 14 (top) down
//!   to display rank 1 (bottom)**. Each rank is comma-separated tokens, each either a
//!   piece (`yR`) or a positive integer = that many consecutive empty cells. Empty runs
//!   **count the invalid corner cells** — every rank sums to exactly 14 columns.
//!
//! A legacy FEN4 dialect (`xxx` corner cells, space-separated trailer) is intentionally NOT
//! handled here; converting it is a separate concern for the strength-gate phase.

use super::Board;
use super::types::{BOARD_SIZE, Piece, Player, Square};
use std::fmt;

/// Canonical 4PC starting position (spec v0.2 §1.3, verified vs chess.com).
pub const START_FEN4: &str = "R-0,0,0,0-1,1,1,1-1,1,1,1-0,0,0,0-0-3,yR,yN,yB,yK,yQ,yB,yN,yR,3/3,yP,yP,yP,yP,yP,yP,yP,yP,3/14/bR,bP,10,gP,gR/bN,bP,10,gP,gN/bB,bP,10,gP,gB/bQ,bP,10,gP,gK/bK,bP,10,gP,gQ/bB,bP,10,gP,gB/bN,bP,10,gP,gN/bR,bP,10,gP,gR/14/3,rP,rP,rP,rP,rP,rP,rP,rP,3/3,rR,rN,rB,rQ,rK,rB,rN,rR,3";

/// The `StartFen4 "4PCo"` array (2026-06-12 export batch): Blue's and Green's king/queen home
/// squares are EXCHANGED relative to the canonical array (bQ a7 / bK a8, gQ n8 / gK n7);
/// Red/Yellow unchanged. Identified empirically, not from documentation: under the canonical
/// start these games fail replay on queen tokens that find a king on the from-square; under
/// this array they replay correctly, where the canonical start fails on those games.
pub const START_FEN4_4PCO: &str = "R-0,0,0,0-1,1,1,1-1,1,1,1-0,0,0,0-0-3,yR,yN,yB,yK,yQ,yB,yN,yR,3/3,yP,yP,yP,yP,yP,yP,yP,yP,3/14/bR,bP,10,gP,gR/bN,bP,10,gP,gN/bB,bP,10,gP,gB/bK,bP,10,gP,gQ/bQ,bP,10,gP,gK/bB,bP,10,gP,gB/bN,bP,10,gP,gN/bR,bP,10,gP,gR/14/3,rP,rP,rP,rP,rP,rP,rP,rP,3/3,rR,rN,rB,rQ,rK,rB,rN,rR,3";

/// Errors a malformed FEN4 string can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fen4Error {
    /// Fewer than the 7 dash-separated fields were present.
    MissingFields { got: usize },
    /// The turn field was not exactly one of `R B Y G`.
    BadTurn(String),
    /// A `0`/`1` flag group was malformed (wrong count or non-binary).
    BadFlagGroup(String),
    /// A points group was malformed (wrong count or non-numeric).
    BadPoints(String),
    /// The board did not have exactly 14 ranks.
    BadRankCount { got: usize },
    /// A rank's tokens summed to something other than 14 columns.
    BadFileCount { display_rank: u8, got: u16 },
    /// A token was neither a valid empty-run integer nor a valid piece.
    BadToken(String),
    /// A piece token was placed on an invalid corner square.
    PieceOnInvalidSquare { token: String, square: u8 },
}

impl fmt::Display for Fen4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fen4Error::MissingFields { got } => {
                write!(f, "FEN4 needs 7 dash-separated fields, got {got}")
            }
            Fen4Error::BadTurn(s) => write!(f, "bad turn field {s:?} (expected R/B/Y/G)"),
            Fen4Error::BadFlagGroup(s) => write!(f, "bad 0/1 flag group {s:?}"),
            Fen4Error::BadPoints(s) => write!(f, "bad points group {s:?}"),
            Fen4Error::BadRankCount { got } => write!(f, "board has {got} ranks, expected 14"),
            Fen4Error::BadFileCount { display_rank, got } => {
                write!(
                    f,
                    "display rank {display_rank} sums to {got} columns, expected 14"
                )
            }
            Fen4Error::BadToken(s) => write!(f, "bad board token {s:?}"),
            Fen4Error::PieceOnInvalidSquare { token, square } => {
                write!(f, "piece {token:?} on invalid corner square {square}")
            }
        }
    }
}

impl std::error::Error for Fen4Error {}

/// Parse a FEN4 string into a [`Board`].
pub fn parse(s: &str) -> Result<Board, Fen4Error> {
    let fields: Vec<&str> = s.splitn(7, '-').collect();
    if fields.len() < 7 {
        return Err(Fen4Error::MissingFields { got: fields.len() });
    }

    let side_to_move = parse_turn(fields[0])?;
    let dead = parse_flags(fields[1])?;
    let castle_kingside = parse_flags(fields[2])?;
    let castle_queenside = parse_flags(fields[3])?;
    let points = parse_points(fields[4])?;
    let extra = fields[5].to_string();

    let mut board = Board::empty();
    board.side_to_move = side_to_move;
    board.dead = dead;
    board.castle_kingside = castle_kingside;
    board.castle_queenside = castle_queenside;
    board.points = points;
    board.extra = extra;

    let ranks: Vec<&str> = fields[6].split('/').collect();
    if ranks.len() != BOARD_SIZE as usize {
        return Err(Fen4Error::BadRankCount { got: ranks.len() });
    }

    // ranks[0] is display rank 14 = internal rank 13; ranks[13] is display rank 1.
    for (i, rank_str) in ranks.iter().enumerate() {
        let internal_rank = (BOARD_SIZE - 1) - i as u8;
        let mut file: u16 = 0;
        for token in rank_str.split(',') {
            if let Ok(run) = token.parse::<u16>() {
                file += run;
            } else if let Some(piece) = Piece::from_token(token) {
                if file >= BOARD_SIZE as u16 {
                    return Err(Fen4Error::BadFileCount {
                        display_rank: internal_rank + 1,
                        got: file + 1,
                    });
                }
                let sq = Square::from_rank_file(internal_rank, file as u8);
                if !sq.is_valid() {
                    return Err(Fen4Error::PieceOnInvalidSquare {
                        token: token.to_string(),
                        square: sq.index(),
                    });
                }
                board.set_piece(sq, Some(piece));
                file += 1;
            } else {
                return Err(Fen4Error::BadToken(token.to_string()));
            }
        }
        if file != BOARD_SIZE as u16 {
            return Err(Fen4Error::BadFileCount {
                display_rank: internal_rank + 1,
                got: file,
            });
        }
    }

    board.recompute_zobrist();
    Ok(board)
}

/// Serialize a [`Board`] back to a FEN4 string. Inverse of [`parse`]; round-trips
/// byte-identically for any board that came from a well-formed FEN4.
pub fn serialize(board: &Board) -> String {
    let turn = board.side_to_move.to_char();
    let dead = flags_to_str(&board.dead);
    let ck = flags_to_str(&board.castle_kingside);
    let cq = flags_to_str(&board.castle_queenside);
    let points = points_to_str(&board.points);

    let mut ranks: Vec<String> = Vec::with_capacity(BOARD_SIZE as usize);
    // Emit display rank 14 (internal 13) first, down to display rank 1 (internal 0).
    for internal_rank in (0..BOARD_SIZE).rev() {
        let mut tokens: Vec<String> = Vec::new();
        let mut run: u16 = 0;
        for file in 0..BOARD_SIZE {
            let sq = Square::from_rank_file(internal_rank, file);
            match board.piece_at(sq) {
                Some(piece) => {
                    if run > 0 {
                        tokens.push(run.to_string());
                        run = 0;
                    }
                    tokens.push(piece.to_token());
                }
                None => run += 1,
            }
        }
        if run > 0 {
            tokens.push(run.to_string());
        }
        ranks.push(tokens.join(","));
    }

    format!(
        "{turn}-{dead}-{ck}-{cq}-{points}-{extra}-{board}",
        extra = board.extra,
        board = ranks.join("/"),
    )
}

fn parse_turn(field: &str) -> Result<Player, Fen4Error> {
    let mut chars = field.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            Player::from_char(c).ok_or_else(|| Fen4Error::BadTurn(field.to_string()))
        }
        _ => Err(Fen4Error::BadTurn(field.to_string())),
    }
}

fn parse_flags(field: &str) -> Result<[bool; 4], Fen4Error> {
    let parts: Vec<&str> = field.split(',').collect();
    if parts.len() != 4 {
        return Err(Fen4Error::BadFlagGroup(field.to_string()));
    }
    let mut out = [false; 4];
    for (i, p) in parts.iter().enumerate() {
        out[i] = match *p {
            "0" => false,
            "1" => true,
            _ => return Err(Fen4Error::BadFlagGroup(field.to_string())),
        };
    }
    Ok(out)
}

fn parse_points(field: &str) -> Result<[u16; 4], Fen4Error> {
    let parts: Vec<&str> = field.split(',').collect();
    if parts.len() != 4 {
        return Err(Fen4Error::BadPoints(field.to_string()));
    }
    let mut out = [0u16; 4];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse()
            .map_err(|_| Fen4Error::BadPoints(field.to_string()))?;
    }
    Ok(out)
}

fn flags_to_str(flags: &[bool; 4]) -> String {
    let mut parts = [""; 4];
    for (i, &b) in flags.iter().enumerate() {
        parts[i] = if b { "1" } else { "0" };
    }
    parts.join(",")
}

fn points_to_str(points: &[u16; 4]) -> String {
    points
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::types::PieceType;

    // ---- Stage 2.3: parse ----

    #[test]
    fn parses_start_position_kings() {
        let b = parse(START_FEN4).expect("start FEN4 parses");
        assert_eq!(b.side_to_move, Player::Red);
        assert_eq!(
            b.king_square(Player::Red),
            Some(Square::from_algebraic("h1").unwrap())
        );
        assert_eq!(
            b.king_square(Player::Blue),
            Some(Square::from_algebraic("a7").unwrap())
        );
        assert_eq!(
            b.king_square(Player::Yellow),
            Some(Square::from_algebraic("g14").unwrap())
        );
        assert_eq!(
            b.king_square(Player::Green),
            Some(Square::from_algebraic("n8").unwrap())
        );
    }

    #[test]
    fn parses_start_position_header_and_counts() {
        let b = parse(START_FEN4).unwrap();
        assert_eq!(b.dead, [false; 4]);
        assert_eq!(b.castle_kingside, [true; 4]);
        assert_eq!(b.castle_queenside, [true; 4]);
        assert_eq!(b.points, [0; 4]);
        assert_eq!(b.extra, "0");
        for p in Player::ALL {
            assert_eq!(b.piece_count(p), 16, "{p:?} should have 16 pieces");
        }
    }

    #[test]
    fn parses_specific_start_squares() {
        let b = parse(START_FEN4).unwrap();
        let at = |s: &str| b.piece_at(Square::from_algebraic(s).unwrap());
        assert_eq!(at("g1"), Some(Piece::new(Player::Red, PieceType::Queen)));
        assert_eq!(at("d1"), Some(Piece::new(Player::Red, PieceType::Rook)));
        assert_eq!(at("a8"), Some(Piece::new(Player::Blue, PieceType::Queen)));
        assert_eq!(at("n7"), Some(Piece::new(Player::Green, PieceType::Queen)));
        assert_eq!(
            at("h14"),
            Some(Piece::new(Player::Yellow, PieceType::Queen))
        );
        // An interior empty square and an invalid corner are both None.
        assert_eq!(at("g7"), None);
        assert_eq!(b.piece_at(Square::from_rank_file(0, 0)), None); // SW corner
    }

    // ---- Stage 2.4: serialize + byte-identical round-trip ----

    #[test]
    fn start_position_round_trips_byte_identical() {
        let b = parse(START_FEN4).unwrap();
        assert_eq!(serialize(&b), START_FEN4);
    }

    #[test]
    fn parse_serialize_is_identity_on_board() {
        let b1 = parse(START_FEN4).unwrap();
        let b2 = parse(&serialize(&b1)).unwrap();
        assert!(b1 == b2, "parse∘serialize∘parse should be stable");
    }

    // ---- Stage 2.5: edge cases ----

    #[test]
    fn empty_board_round_trips() {
        let b = Board::empty();
        let s = serialize(&b);
        // 14 all-empty ranks.
        assert_eq!(
            s,
            "R-0,0,0,0-0,0,0,0-0,0,0,0-0,0,0,0-0-14/14/14/14/14/14/14/14/14/14/14/14/14/14"
        );
        assert_eq!(parse(&s).unwrap(), b);
    }

    #[test]
    fn handbuilt_position_round_trips() {
        // Mixed flags/points + a few scattered pieces, incl. cells adjacent to corners.
        let mut b = Board::empty();
        b.side_to_move = Player::Yellow;
        b.dead = [false, true, false, false];
        b.castle_kingside = [true, false, true, false];
        b.castle_queenside = [false, false, true, true];
        b.points = [20, 0, 5, 41];
        b.set_piece(
            Square::from_algebraic("h1").unwrap(),
            Some(Piece::new(Player::Red, PieceType::King)),
        );
        b.set_piece(
            Square::from_algebraic("d3").unwrap(),
            Some(Piece::new(Player::Red, PieceType::Pawn)),
        );
        b.set_piece(
            Square::from_algebraic("n8").unwrap(),
            Some(Piece::new(Player::Green, PieceType::King)),
        );
        let s = serialize(&b);
        assert_eq!(parse(&s).unwrap(), b, "round-trip mismatch for {s}");
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(matches!(
            parse("R-0,0,0,0"),
            Err(Fen4Error::MissingFields { .. })
        ));
        // Bad turn.
        let bad_turn = START_FEN4.replacen("R-", "Z-", 1);
        assert!(matches!(parse(&bad_turn), Err(Fen4Error::BadTurn(_))));
        // Too few ranks.
        assert!(matches!(
            parse("R-0,0,0,0-0,0,0,0-0,0,0,0-0,0,0,0-0-14/14"),
            Err(Fen4Error::BadRankCount { .. })
        ));
        // A rank that sums to the wrong column count (13 instead of 14).
        let short_rank =
            "R-0,0,0,0-0,0,0,0-0,0,0,0-0,0,0,0-0-13/14/14/14/14/14/14/14/14/14/14/14/14/14";
        assert!(matches!(
            parse(short_rank),
            Err(Fen4Error::BadFileCount { .. })
        ));
        // Garbage token.
        let bad_tok =
            "R-0,0,0,0-0,0,0,0-0,0,0,0-0,0,0,0-0-zz,12/14/14/14/14/14/14/14/14/14/14/14/14/14";
        assert!(matches!(parse(bad_tok), Err(Fen4Error::BadToken(_))));
    }
}
