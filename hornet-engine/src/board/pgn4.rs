//! PGN4 parser / serializer — the chess.com 4PC dialect. See spec §10.3.
//!
//! Parsing is **structural** — header tag-pairs + raw per-ply tokens (`"h2-h3"`,
//! `"Qb7xg12+#"`, `"O-O-O"`, `"Kh13-i14R"`). [`decode_ply`] additionally decodes one ply
//! token into a [`DecodedMove`] (from/to squares + promotion, or castle); matching that to a
//! concrete legal move and applying it lives in the replay harness (it needs move generation).
//!
//! Grammar recap (corpus-derived): `[Key "Value"]` header lines, a blank line, then
//! rounds `N. ply .. ply .. ply .. ply`. Rounds may have fewer than four plies once
//! players are eliminated. `StartFen4 "4PC"` is shorthand for the canonical start (§1.3).

use super::Board;
use super::fen4::{self, START_FEN4};
use super::types::{PieceType, Square};
use std::fmt;

/// A parsed PGN4 game: ordered header tags plus the move stream as rounds of raw plies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pgn4Game {
    pub tags: Vec<(String, String)>,
    pub rounds: Vec<Pgn4Round>,
}

/// One numbered round and its plies (raw tokens, in board order R/B/Y/G as present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pgn4Round {
    pub number: u32,
    pub plies: Vec<String>,
}

/// Errors PGN4 parsing can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pgn4Error {
    /// A `[...]` header line was malformed.
    BadHeader(String),
    /// A ply token appeared before any round number.
    PlyBeforeRound(String),
    /// The game had no move rounds.
    NoMoves,
    /// `StartFen4` was an explicit FEN4 string that failed to parse.
    Fen4(fen4::Fen4Error),
}

impl fmt::Display for Pgn4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pgn4Error::BadHeader(s) => write!(f, "malformed header line {s:?}"),
            Pgn4Error::PlyBeforeRound(s) => write!(f, "ply {s:?} before any round number"),
            Pgn4Error::NoMoves => write!(f, "PGN4 had no move rounds"),
            Pgn4Error::Fen4(e) => write!(f, "StartFen4 parse failed: {e}"),
        }
    }
}

impl std::error::Error for Pgn4Error {}

impl Pgn4Game {
    /// Value of a header tag by key, if present.
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The `StartFen4` tag, defaulting to the `"4PC"` shorthand.
    pub fn start_fen4(&self) -> &str {
        self.tag("StartFen4").unwrap_or("4PC")
    }

    /// Resolve the initial position to a [`Board`] (`"4PC"` → canonical start, else an
    /// explicit FEN4 string).
    pub fn initial_board(&self) -> Result<Board, Pgn4Error> {
        let s = self.start_fen4();
        let fen = if s == "4PC" { START_FEN4 } else { s };
        fen4::parse(fen).map_err(Pgn4Error::Fen4)
    }

    /// Total plies across all rounds.
    pub fn ply_count(&self) -> usize {
        self.rounds.iter().map(|r| r.plies.len()).sum()
    }
}

/// Parse a PGN4 document into a [`Pgn4Game`] (structural — ply tokens are not decoded).
pub fn parse(s: &str) -> Result<Pgn4Game, Pgn4Error> {
    let mut tags = Vec::new();
    let mut move_text = String::new();
    let mut in_moves = false;

    for line in s.lines() {
        let t = line.trim();
        if !in_moves {
            if t.is_empty() {
                continue;
            }
            if t.starts_with('[') {
                tags.push(parse_header(t)?);
                continue;
            }
            // First non-empty, non-header line: the move stream begins here.
            in_moves = true;
        }
        if !t.is_empty() {
            move_text.push_str(t);
            move_text.push(' ');
        }
    }

    let rounds = parse_moves(&move_text)?;
    Ok(Pgn4Game { tags, rounds })
}

/// Serialize a [`Pgn4Game`] back to PGN4 text. Not byte-identical to arbitrary source
/// (line wrapping is normalized), but structurally stable: `parse(serialize(g)) == g`.
pub fn serialize(game: &Pgn4Game) -> String {
    let mut out = String::new();
    for (k, v) in &game.tags {
        out.push('[');
        out.push_str(k);
        out.push_str(" \"");
        out.push_str(v);
        out.push_str("\"]\n");
    }
    out.push('\n');
    for round in &game.rounds {
        out.push_str(&round.number.to_string());
        out.push_str(". ");
        out.push_str(&round.plies.join(" .. "));
        out.push('\n');
    }
    out
}

fn parse_header(line: &str) -> Result<(String, String), Pgn4Error> {
    let inner = line
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| Pgn4Error::BadHeader(line.to_string()))?;
    let sp = inner
        .find(' ')
        .ok_or_else(|| Pgn4Error::BadHeader(line.to_string()))?;
    let key = inner[..sp].to_string();
    let rest = inner[sp + 1..].trim();
    let val = rest
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .ok_or_else(|| Pgn4Error::BadHeader(line.to_string()))?;
    Ok((key, val.to_string()))
}

/// A round marker is digits followed by a dot, e.g. `12.` → `Some(12)`.
fn parse_round_marker(tok: &str) -> Option<u32> {
    let num = tok.strip_suffix('.')?;
    if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    num.parse().ok()
}

fn parse_moves(text: &str) -> Result<Vec<Pgn4Round>, Pgn4Error> {
    let mut rounds: Vec<Pgn4Round> = Vec::new();
    let mut current: Option<Pgn4Round> = None;

    for tok in text.split_whitespace() {
        if let Some(number) = parse_round_marker(tok) {
            if let Some(r) = current.take() {
                rounds.push(r);
            }
            current = Some(Pgn4Round {
                number,
                plies: Vec::new(),
            });
            continue;
        }
        if tok == ".." {
            continue; // ply separator
        }
        match current.as_mut() {
            Some(r) => r.plies.push(tok.to_string()),
            None => return Err(Pgn4Error::PlyBeforeRound(tok.to_string())),
        }
    }

    if let Some(r) = current.take() {
        rounds.push(r);
    }
    if rounds.is_empty() {
        return Err(Pgn4Error::NoMoves);
    }
    Ok(rounds)
}

/// A structurally-decoded ply: from/to squares (+ promotion) or a castle. Turning this
/// into a concrete legal [`crate::board::Move`] needs move generation (the replay harness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedMove {
    Normal {
        from: Square,
        to: Square,
        promotion: Option<PieceType>,
    },
    Castle {
        kingside: bool,
    },
}

/// Decode a single PGN4 ply token (chess.com notation) into a [`DecodedMove`].
///
/// Handles `O-O`/`O-O-O`, from-to (`d2-d4`), SAN-with-source (`Ne1-f3`), captures with an
/// embedded captured-piece letter (`Bn6xBg13`), promotion (`=D` queen, `=N/=B/=R`), and
/// trailing `+`/`#`/elimination markers. Returns `None` for non-move tokens (e.g. a bare
/// result marker). The move's squares are the first and last algebraic squares in the token.
pub fn decode_ply(token: &str) -> Option<DecodedMove> {
    let t = token.trim_end_matches(['+', '#']);
    match t {
        "O-O" | "0-0" => return Some(DecodedMove::Castle { kingside: true }),
        "O-O-O" | "0-0-0" => return Some(DecodedMove::Castle { kingside: false }),
        _ => {}
    }

    let promotion = t.split('=').nth(1).and_then(|p| match p.chars().next()? {
        'D' | 'Q' => Some(PieceType::Queen),
        'N' => Some(PieceType::Knight),
        'B' => Some(PieceType::Bishop),
        'R' => Some(PieceType::Rook),
        _ => None,
    });

    let squares = extract_squares(t);
    if squares.len() < 2 {
        return None;
    }
    Some(DecodedMove::Normal {
        from: squares[0],
        to: *squares.last().unwrap(),
        promotion,
    })
}

/// Extract every algebraic square (lowercase file `a..n` + rank `1..14`) from a token,
/// in order. Uppercase piece letters and `x`/`-`/`=` separators are skipped naturally.
fn extract_squares(s: &str) -> Vec<Square> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if (b'a'..=b'n').contains(&bytes[i]) {
            let start = i;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start + 1
                && let Some(sq) = Square::from_algebraic(&s[start..j])
            {
                out.push(sq);
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::types::Player;

    const SAMPLE: &str = "[GameNr \"123\"]\n[Variant \"FFA\"]\n[StartFen4 \"4PC\"]\n\n1. h2-h3 .. b7-c7 .. g13-g12 .. m8-l8\n2. Ne1-f3 .. Qa8-b7 .. O-O-O .. Qm8xh13+\n3. k2-k4 .. Qb7xg12+# .. Qh13xQg12";

    #[test]
    fn parses_headers_and_rounds() {
        let g = parse(SAMPLE).unwrap();
        assert_eq!(g.tag("GameNr"), Some("123"));
        assert_eq!(g.tag("Variant"), Some("FFA"));
        assert_eq!(g.start_fen4(), "4PC");
        assert_eq!(g.rounds.len(), 3);
        assert_eq!(g.rounds[0].number, 1);
        assert_eq!(
            g.rounds[0].plies,
            vec!["h2-h3", "b7-c7", "g13-g12", "m8-l8"]
        );
        // Reduced final round (Green eliminated): only 3 plies, with check/mate markers kept.
        assert_eq!(g.rounds[2].plies, vec!["k2-k4", "Qb7xg12+#", "Qh13xQg12"]);
        assert_eq!(g.ply_count(), 4 + 4 + 3);
    }

    #[test]
    fn start_fen4_resolves_to_canonical_board() {
        let g = parse(SAMPLE).unwrap();
        let b = g.initial_board().unwrap();
        assert_eq!(b.side_to_move, Player::Red);
        assert_eq!(b.piece_count(Player::Red), 16);
    }

    #[test]
    fn structural_round_trip_is_stable() {
        let g = parse(SAMPLE).unwrap();
        let s = serialize(&g);
        assert_eq!(
            parse(&s).unwrap(),
            g,
            "parse∘serialize should reproduce the game"
        );
    }

    #[test]
    fn rejects_ply_before_round() {
        assert!(matches!(
            parse("[A \"b\"]\n\nh2-h3"),
            Err(Pgn4Error::PlyBeforeRound(_))
        ));
    }
}
