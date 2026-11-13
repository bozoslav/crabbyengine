//! Bookkeeping of the world's archetype slots that lives next to the
//! archetypes rather than in them: which archetypes hold entities.
//!
//! Queries walk the occupancy bits instead of the archetypes, so an empty
//! archetype costs a bit, not a read of its 128-byte struct: a world whose
//! components come and go keeps many empty archetypes, and a loop over all
//! of them used to read every one (issue #348).

/// Per-slot state of a world's archetypes. Indexed like `World::archetypes`.
#[derive(Default)]
pub(crate) struct Slots {
    /// Bit `a` (word `a / 64`, bit `a % 64`) is set exactly when archetype
    /// `a` holds at least one entity. Every change of an archetype's length
    /// between zero and not zero goes through [`filled`](Self::filled) or
    /// [`emptied`](Self::emptied), so this holds whenever a query runs.
    pub(crate) occupied: Vec<u64>,
}

impl Slots {
    /// Makes room for one more archetype slot, `a` (empty).
    pub(crate) fn add(&mut self, a: usize) {
        if a / 64 >= self.occupied.len() {
            self.occupied.push(0);
        }
    }

    /// Archetype `a` went from no entities to one.
    #[inline]
    pub(crate) fn filled(&mut self, a: usize) {
        self.occupied[a / 64] |= 1 << (a % 64);
    }

    /// Archetype `a` lost its last entity.
    #[inline]
    pub(crate) fn emptied(&mut self, a: usize) {
        self.occupied[a / 64] &= !(1 << (a % 64));
    }
}

/// Whether bit `a` of `occupied` is set: archetype `a` holds entities.
#[inline(always)]
pub(crate) fn is_occupied(occupied: &[u64], a: usize) -> bool {
    occupied.get(a / 64).is_some_and(|word| word >> (a % 64) & 1 != 0)
}

/// The first set bit of `occupied` at or after `from`.
#[inline(always)]
pub(crate) fn next_occupied(occupied: &[u64], from: usize) -> Option<usize> {
    let mut word = from / 64;
    let mut bits = *occupied.get(word)? & (u64::MAX << (from % 64));
    while bits == 0 {
        word += 1;
        bits = *occupied.get(word)?;
    }
    Some(word * 64 + bits.trailing_zeros() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bits_follow_filled_and_emptied_slots() {
        let mut slots = Slots::default();
        for a in 0..130 {
            slots.add(a);
        }
        assert_eq!(slots.occupied.len(), 3);
        for a in [0, 63, 64, 129] {
            slots.filled(a);
        }
        slots.emptied(63);
        let found: Vec<usize> = std::iter::successors(next_occupied(&slots.occupied, 0), |&a| {
            next_occupied(&slots.occupied, a + 1)
        })
        .collect();
        assert_eq!(found, [0, 64, 129]);
        assert!(is_occupied(&slots.occupied, 64) && !is_occupied(&slots.occupied, 63));
        assert!(!is_occupied(&slots.occupied, 10_000));
        assert_eq!(next_occupied(&slots.occupied, 130), None);
        assert_eq!(next_occupied(&slots.occupied, 1_000), None);
    }
}
