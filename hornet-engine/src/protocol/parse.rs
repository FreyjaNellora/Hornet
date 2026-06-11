//! Protocol command parsing (position fen4/pgn4 + moves, go, uci handshake). Phase 8.

/// The base position a `position` command sets, before any `moves` are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionBase {
    /// Canonical 4PC start position.
    Start,
    /// An explicit FEN4 string (native dialect).
    Fen4(String),
    /// A PGN4 file path, replayed to its final position.
    Pgn4(String),
}

/// A parsed protocol command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `uci` — identify + handshake.
    Uci,
    /// `isready` — readiness handshake.
    IsReady,
    /// `quit` / `exit` / EOF.
    Quit,
    /// `position <base> [moves <ply>...]` — set the board, then apply the ply list.
    Position {
        base: PositionBase,
        moves: Vec<String>,
    },
    /// `go [depth N]` — search the current position (depth rounds up to a multiple of 4).
    Go { depth: u32 },
    /// `d` / `display` — print the current side-to-move and points.
    Display,
    /// Anything unrecognized (echoed back as an info line).
    Unknown(String),
}

/// Default search depth for a bare `go` (one full rotation).
const DEFAULT_DEPTH: u32 = 4;

/// Parse one protocol line. Returns `None` for blank lines.
pub fn parse(line: &str) -> Option<Command> {
    // Strip a leading UTF-8 BOM (Rust's `trim` does not — U+FEFF isn't whitespace) before trimming.
    let line = line.trim_start_matches('\u{feff}').trim();
    if line.is_empty() {
        return None;
    }
    let mut it = line.split_whitespace();
    let head = it.next()?;
    let cmd = match head {
        "uci" => Command::Uci,
        "isready" => Command::IsReady,
        "quit" | "exit" => Command::Quit,
        "d" | "display" => Command::Display,
        "go" => {
            let rest: Vec<&str> = it.collect();
            let mut depth = DEFAULT_DEPTH;
            if let Some(p) = rest.iter().position(|&t| t == "depth") {
                if let Some(n) = rest.get(p + 1).and_then(|s| s.parse::<u32>().ok()) {
                    depth = n;
                }
            }
            Command::Go { depth }
        }
        "position" => {
            let parts: Vec<&str> = it.collect();
            let moves_at = parts.iter().position(|&t| t == "moves");
            let base_parts = match moves_at {
                Some(mi) => &parts[..mi],
                None => &parts[..],
            };
            let moves: Vec<String> = match moves_at {
                Some(mi) => parts[mi + 1..].iter().map(|s| s.to_string()).collect(),
                None => Vec::new(),
            };
            let base = match base_parts.first().copied() {
                Some("startpos") => PositionBase::Start,
                Some("fen4") => {
                    PositionBase::Fen4(base_parts.get(1).copied().unwrap_or("").to_string())
                }
                Some("pgn4") => PositionBase::Pgn4(base_parts[1..].join(" ")),
                _ => return Some(Command::Unknown(line.to_string())),
            };
            Command::Position { base, moves }
        }
        _ => Command::Unknown(line.to_string()),
    };
    Some(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_handshake_and_go() {
        assert_eq!(parse("uci"), Some(Command::Uci));
        assert_eq!(parse("isready"), Some(Command::IsReady));
        assert_eq!(parse("quit"), Some(Command::Quit));
        assert_eq!(
            parse("go"),
            Some(Command::Go {
                depth: DEFAULT_DEPTH
            })
        );
        assert_eq!(parse("go depth 8"), Some(Command::Go { depth: 8 }));
        assert_eq!(parse("   "), None);
    }

    #[test]
    fn parses_position_forms() {
        assert_eq!(
            parse("position startpos"),
            Some(Command::Position {
                base: PositionBase::Start,
                moves: vec![]
            })
        );
        assert_eq!(
            parse("position startpos moves h2-h3 b7-c7"),
            Some(Command::Position {
                base: PositionBase::Start,
                moves: vec!["h2-h3".into(), "b7-c7".into()],
            })
        );
        assert_eq!(
            parse("position fen4 R-0,0,0,0-x moves Ne1-f3"),
            Some(Command::Position {
                base: PositionBase::Fen4("R-0,0,0,0-x".into()),
                moves: vec!["Ne1-f3".into()],
            })
        );
        assert_eq!(
            parse("position pgn4 game.pgn4"),
            Some(Command::Position {
                base: PositionBase::Pgn4("game.pgn4".into()),
                moves: vec![]
            })
        );
    }
}
