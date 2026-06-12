//! Hornet — a four-player chess engine.
//!
//! Architecture (see `HORNET-BUILD-SPEC.md`): per-piece BFS line projection feeds a
//! query engine that returns a per-player utility vector `V = <U1, U2, U3, U4>` to a
//! Max^n search. The eval contract is a vector, never a scalar.

pub mod board;
pub mod bounty;
pub mod eval;
pub mod game;
pub mod intent;
pub mod lines;
pub mod move_gen;
pub mod move_order;
pub mod nnue;
pub mod protocol;
pub mod queries;
pub mod replay;
pub mod search;
pub mod tt;
pub mod zones;
