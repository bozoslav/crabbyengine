//! Bookkeeping of the world's archetype slots that lives next to the
//! archetypes rather than in them: which archetypes hold entities.
//!
//! **Skipping empty archetypes (issue #348).** A world whose components
//! come and go keeps many empty archetypes, and a query that scans used to
//! read every archetype's 128-byte struct to find the ones with entities.
//! In such a world (more than one empty archetype per four with entities)
//! a scanning query visits the list of archetypes with entities instead,
//! built from the occupancy bits when it is first needed after a change.
//! The query loop itself is unchanged: the list is handed to it as its
//! candidates, as the archetype index's lists are. Testing a bit in that
//! loop instead, for every archetype, was measured: it slowed scans of
//! worlds without empty archetypes by about 15 % and changed how `Query::iter`
//! compiles (`crates/ponko_bench/SPARSE_SET.md`).

/// Per-slot state of a world's archetypes. Indexed like `World::archetypes`.
#[derive(Default)]
pub(crate) struct Slots {
    /// Bit `a` (word `a / 64`, bit `a % 64`) is set exactly when archetype
    /// `a` holds at least one entity. Every change of an archetype's length
    /// between zero and not zero goes through [`filled`](Self::filled) or
    /// [`emptied`](Self::emptied).
    occupied: Vec<u64>,
    /// Archetypes, and those with entities.
    slots: usize,
    live: usize,
    /// The archetypes with entities, ascending, while `listed`.
    list: Vec<u32>,
    listed: bool,
}

impl Slots {
    /// Makes room for one more archetype slot, `a` (empty).
    pub(crate) fn add(&mut self, a: usize) {
        self.slots += 1;
        if a / 64 >= self.occupied.len() {
            self.occupied.push(0);
        }
    }

    /// Archetype `a` went from no entities to one.
    #[inline]
    pub(crate) fn filled(&mut self, a: usize) {
        self.occupied[a / 64] |= 1 << (a % 64);
        self.live += 1;
        self.listed = false;
    }

    /// Archetype `a` lost its last entity.
    #[inline]
    pub(crate) fn emptied(&mut self, a: usize) {
        self.occupied[a / 64] &= !(1 << (a % 64));
        self.live -= 1;
        self.listed = false;
    }

    /// Whether more than one archetype in five is empty: then scanning
    /// queries visit [`list`](Self::list) instead of every archetype.
    #[inline]
    fn many_empty(&self) -> bool {
        (self.slots - self.live) * 4 > self.live
    }

    /// What a query that scans visits: the archetypes with entities, in a
    /// world with many empty ones (listed now if they changed since the
    /// last time), or `None` for all of them. Allocates only when the list
    /// grows past every earlier length.
    #[inline]
    pub(crate) fn scan(&mut self) -> Option<&[u32]> {
        if !self.many_empty() {
            return None;
        }
        if !self.listed {
            self.relist();
        }
        Some(&self.list)
    }

    /// [`scan`](Self::scan) for queries that hold the world shared: the list
    /// only if it is current. Views list it before they run.
    #[inline]
    pub(crate) fn shared_scan(&self) -> Option<&[u32]> {
        (self.listed && self.many_empty()).then_some(&self.list[..])
    }

    #[cold]
    #[inline(never)]
    fn relist(&mut self) {
        self.list.clear();
        for (word, &bits) in self.occupied.iter().enumerate() {
            let mut bits = bits;
            while bits != 0 {
                self.list.push((word * 64) as u32 + bits.trailing_zeros());
                bits &= bits - 1;
            }
        }
        self.listed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_list_the_filled_slots_once_most_are_empty() {
        let mut slots = Slots::default();
        for a in 0..130 {
            slots.add(a);
        }
        assert_eq!(slots.occupied.len(), 3);
        for a in 0..130 {
            slots.filled(a);
        }
        // None empty: scans visit everything.
        assert_eq!(slots.scan(), None);
        for a in (0..130).filter(|a| ![0, 63, 64, 129].contains(a)) {
            slots.emptied(a);
        }
        assert_eq!(slots.scan(), Some(&[0, 63, 64, 129][..]));
        assert_eq!(slots.shared_scan(), Some(&[0, 63, 64, 129][..]));
        slots.emptied(63);
        // Stale until listed again by a query that holds the world exclusively.
        assert_eq!(slots.shared_scan(), None);
        assert_eq!(slots.scan(), Some(&[0, 64, 129][..]));
        slots.filled(1);
        assert_eq!(slots.scan(), Some(&[0, 1, 64, 129][..]));
    }
}
