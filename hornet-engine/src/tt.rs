//! Transposition table keyed by the Zobrist hash (spec §6.4). Caches per-player Max^n
//! value vectors with a search depth + bound and the best move, so the search can reuse
//! previously-computed subtrees and order moves.
//!
//! Direct-mapped, power-of-two slot count, depth-preferred replacement (same key keeps the
//! deeper search; a colliding key always replaces). A two-tier / aged scheme is a later
//! refinement.

use crate::board::Move;

/// Whether a stored value is exact or a bound (for Max^n shallow pruning, added later).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

/// One table slot. `key == 0` marks an empty slot (a real position hashing to 0 simply
/// never caches — astronomically rare).
#[derive(Clone, Copy)]
pub struct TtEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub value: [i16; 4],
    pub depth: u8,
    pub bound: Bound,
}

impl Default for TtEntry {
    fn default() -> Self {
        TtEntry {
            key: 0,
            best_move: None,
            value: [0; 4],
            depth: 0,
            bound: Bound::Exact,
        }
    }
}

pub struct TranspositionTable {
    entries: Vec<TtEntry>,
    mask: usize,
}

impl TranspositionTable {
    /// Allocate ~`size_mb` MiB, rounded down to a power-of-two slot count.
    pub fn new(size_mb: usize) -> Self {
        let bytes = size_mb.max(1) * 1024 * 1024;
        let want = (bytes / std::mem::size_of::<TtEntry>()).max(1);
        let mut len = 1usize;
        while len * 2 <= want {
            len *= 2;
        }
        TranspositionTable {
            entries: vec![TtEntry::default(); len],
            mask: len - 1,
        }
    }

    /// Number of slots (always a power of two).
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    /// Wipe all entries.
    pub fn clear(&mut self) {
        self.entries
            .iter_mut()
            .for_each(|e| *e = TtEntry::default());
    }

    #[inline]
    fn slot(&self, key: u64) -> usize {
        (key as usize) & self.mask
    }

    /// Look up a position; `Some` only if a stored entry's key matches exactly.
    pub fn probe(&self, key: u64) -> Option<&TtEntry> {
        let e = &self.entries[self.slot(key)];
        (e.key == key && key != 0).then_some(e)
    }

    /// Store a result, replacing if the slot holds a different position or a shallower
    /// search of the same position.
    pub fn store(
        &mut self,
        key: u64,
        depth: u8,
        value: [i16; 4],
        bound: Bound,
        best_move: Option<Move>,
    ) {
        let i = self.slot(key);
        let e = &mut self.entries[i];
        if e.key != key || depth >= e.depth {
            *e = TtEntry {
                key,
                best_move,
                value,
                depth,
                bound,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_is_power_of_two() {
        let tt = TranspositionTable::new(16);
        assert!(tt.capacity().is_power_of_two());
        assert!(tt.capacity() >= 2);
    }

    #[test]
    fn store_then_probe_round_trips() {
        let mut tt = TranspositionTable::new(1);
        tt.store(0xABCD, 5, [1, 2, 3, 4], Bound::Exact, None);
        let e = tt.probe(0xABCD).expect("stored key found");
        assert_eq!(e.value, [1, 2, 3, 4]);
        assert_eq!(e.depth, 5);
        assert_eq!(e.bound, Bound::Exact);
        assert!(tt.probe(0x1234).is_none(), "unknown key misses");
    }

    #[test]
    fn depth_preferred_replacement_for_same_key() {
        let mut tt = TranspositionTable::new(1);
        tt.store(0x55, 6, [10; 4], Bound::Exact, None);
        tt.store(0x55, 3, [0; 4], Bound::Lower, None); // shallower: ignored
        assert_eq!(tt.probe(0x55).unwrap().depth, 6);
        tt.store(0x55, 8, [20; 4], Bound::Upper, None); // deeper: replaces
        assert_eq!(tt.probe(0x55).unwrap().depth, 8);
        assert_eq!(tt.probe(0x55).unwrap().value, [20; 4]);
    }

    #[test]
    fn clear_empties_the_table() {
        let mut tt = TranspositionTable::new(1);
        tt.store(0x99, 4, [5; 4], Bound::Exact, None);
        tt.clear();
        assert!(tt.probe(0x99).is_none());
    }
}
