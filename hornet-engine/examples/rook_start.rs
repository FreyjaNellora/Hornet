//! Where do rooks START in 4PC? Tests whether Kimi's "rooks prefer edge files/ranks" is a real
//! preference or just an artifact of rooks barely developing off their starting squares.
//! Run: cargo run --release --example rook_start

use hornet_engine::board::fen4;
use hornet_engine::board::types::{PieceType, Square};

fn main() {
    let b = fen4::parse(fen4::START_FEN4).expect("start");
    print!("rook start squares: ");
    let mut files = std::collections::BTreeSet::new();
    let mut ranks = std::collections::BTreeSet::new();
    for i in 0u8..196 {
        let sq = Square::new(i);
        if !sq.is_valid() {
            continue;
        }
        if let Some(p) = b.piece_at(sq) {
            if p.piece_type == PieceType::Rook {
                print!("{} ", sq.to_algebraic());
                files.insert((b'a' + sq.file()) as char);
                ranks.insert(sq.rank() + 1);
            }
        }
    }
    println!();
    println!("start files: {files:?}");
    println!("start ranks: {ranks:?}");
}
