//! Hornet engine binary entry point — UCI-like protocol over stdin/stdout.
//!
//! Commands: `uci`, `isready`, `position startpos|fen4 <fen>|pgn4 <path> [moves <ply>...]`,
//! `go [depth N]`, `d`, `quit`. See `protocol/` and `HORNET-BUILD-SPEC.md`.

fn main() {
    hornet_engine::protocol::run();
}
