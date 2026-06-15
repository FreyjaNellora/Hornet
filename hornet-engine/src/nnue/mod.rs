//! NNUE evaluator: a dense MLP over structured query outputs (not canonical
//! sparse-binary NNUE, which does not scale to 14x14). Phase 7.

pub mod network;
pub mod weights;
