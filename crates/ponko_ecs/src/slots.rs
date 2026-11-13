//! Bookkeeping of the world's archetype slots that lives next to the
//! archetypes rather than in them: which archetypes hold entities, which
//! are empty and since when, and the storage of reclaimed archetypes kept
//! for reuse.
//!
//! **Skipping empty archetypes (issue #348).** A world whose components
//! come and go keeps many empty archetypes, and a query that scans used to
//! read every archetype's 128-byte struct to find the ones with entities.
//! In such a world (more than one empty archetype per four with entities,
//! and more than a few)
//! a scanning query visits the list of archetypes with entities instead,
//! built from the occupancy bits when it is first needed after a change.
//! The query loop itself is unchanged: the list is handed to it as its
//! candidates, as the archetype index's lists are. Testing a bit in that
//! loop instead, for every archetype, was measured: it slowed scans of
//! worlds without empty archetypes by about 15 % and changed how `Query::iter`
//! compiles (`crates/ponko_bench/SPARSE_SET.md`).
//!
//! **Reclaiming them (issue #347).** Such a world would also keep an
//! archetype, with its buffers, for every combination it ever had. The
//! policy: a world keeps at most [`KEEP_EMPTY`] empty archetypes, or as
//! many as it has archetypes with entities if that is more. Past that, the
//! archetype that has been empty the longest is reclaimed. A combination
//! that empties and fills again soon is still among the newest empty ones
//! and keeps its archetype, so it doesn't thrash, and the number of
//! archetypes levels off at about twice the number with entities.
//!
//! A reclaimed archetype's slot stays in `World::archetypes` with no
//! components, rows or edges ("dead"), free for the next new component
//! set. Its storage (id lists, column headers, entity list, edge maps)
//! waits in a [`Shell`] and its column buffers in the world's
//! [`BufferPool`], so a world in a steady state of churn builds its new
//! archetypes without allocating. Archetype indices therefore name an
//! archetype only until the next structural change, and what keeps them
//! across changes is kept right:
//!
//! - Entity locations only name archetypes with entities, never an empty
//!   (reclaimable) one.
//! - Transition edges are symmetric (an add edge `a -> b` for a component
//!   is a remove edge `b -> a`), so reclaiming `a` removes the edges that
//!   point to it from its neighbours, and drops its own.
//! - The query index keeps lists of slot indices. A reclaimed slot can stay
//!   in a list (it is empty, and loops skip empty archetypes); when the
//!   slot is reused for another component set, the world tells the index,
//!   which adds the slot to or removes it from every list for that set
//!   (`MatchCache::reused`). Slots are never removed, so the slot count the
//!   index uses as its generation still only grows.
//! - The list of archetypes with entities is rebuilt after any change.
//! - Archetypes named by bundle types (the spawn targets the world caches
//!   per Rust bundle type) and archetype 0 (no components) are pinned:
//!   never reclaimed.
//! - Change detection keeps its stamps in the column buffers and its
//!   removal logs by entity, so it names no archetype.

use std::collections::VecDeque;

use crate::storage::{Archetype, BufferPool, Shell};

/// Empty archetypes a world keeps whatever its size; see the module docs.
pub(crate) const KEEP_EMPTY: usize = 64;
/// Empty archetypes a scan reads (their lengths) rather than keeping a list
/// of the others: for small worlds that change often.
const SCAN_LIST_AFTER: usize = 16;

#[derive(Clone, Copy, Default)]
struct Meta {
    /// Times this slot's archetype became empty (or was created), to tell
    /// a current entry of the empty queue from a stale one.
    emptied: u32,
    /// Never reclaimed.
    pinned: bool,
    /// Reclaimed: no components, free for reuse.
    dead: bool,
}

/// Per-slot state of a world's archetypes. Indexed like `World::archetypes`.
#[derive(Default)]
pub(crate) struct Slots {
    /// Bit `a` (word `a / 64`, bit `a % 64`) is set exactly when archetype
    /// `a` holds at least one entity. Every change of an archetype's length
    /// between zero and not zero goes through [`filled`](Self::filled) or
    /// [`emptied`](Self::emptied).
    occupied: Vec<u64>,
    meta: Vec<Meta>,
    /// Archetypes with entities.
    live: usize,
    /// The archetypes with entities, ascending, while `listed`.
    list: Vec<u32>,
    listed: bool,
    /// Archetypes as they became empty (or were created), oldest first, with
    /// their `Meta::emptied` then. An entry is stale once the archetype got
    /// entities or emptied again since.
    queue: VecDeque<(u32, u32)>,
    /// Empty archetypes that may be reclaimed: not pinned, not dead.
    empty: usize,
    /// Dead slots, and the storage of the archetypes reclaimed from them,
    /// by width (as many shells as dead slots).
    free: Vec<u32>,
    shells: Vec<Vec<Shell>>,
    /// Column buffers of reclaimed archetypes.
    pub(crate) buffers: BufferPool,
}

impl Slots {
    /// Archetype `a` was just created, empty: in a new slot (the next
    /// index), or in a dead one.
    pub(crate) fn created(&mut self, a: usize) {
        if a == self.meta.len() {
            self.meta.push(Meta::default());
            if a / 64 >= self.occupied.len() {
                self.occupied.push(0);
            }
        }
        let meta = &mut self.meta[a];
        meta.dead = false;
        meta.pinned = false;
        self.queue_empty(a);
    }

    /// Archetype `a` is never reclaimed.
    pub(crate) fn pin(&mut self, a: usize) {
        let meta = &mut self.meta[a];
        if !meta.pinned {
            meta.pinned = true;
            if !is_occupied(&self.occupied, a) {
                self.empty -= 1;
            }
        }
    }

    /// Archetype `a` went from no entities to one.
    #[inline]
    pub(crate) fn filled(&mut self, a: usize) {
        self.occupied[a / 64] |= 1 << (a % 64);
        self.live += 1;
        self.listed = false;
        if !self.meta[a].pinned {
            self.empty -= 1;
        }
    }

    /// Archetype `a` lost its last entity. The world then reclaims empty
    /// archetypes while [`over`](Self::over) says so.
    #[inline]
    pub(crate) fn emptied(&mut self, a: usize) {
        self.occupied[a / 64] &= !(1 << (a % 64));
        self.live -= 1;
        self.listed = false;
        if !self.meta[a].pinned {
            self.queue_empty(a);
        }
    }

    fn queue_empty(&mut self, a: usize) {
        let meta = &mut self.meta[a];
        meta.emptied = meta.emptied.wrapping_add(1);
        self.empty += 1;
        if self.queue.capacity() == 0 {
            // Room for what compaction (below) keeps in a world with up to
            // `KEEP_EMPTY` empty archetypes, so that one doesn't allocate.
            self.queue.reserve(4 * KEEP_EMPTY);
        }
        self.queue.push_back((a as u32, meta.emptied));
        if self.queue.len() > 2 * self.empty + KEEP_EMPTY {
            // Mostly stale entries of archetypes that filled again: drop
            // them in place, so the queue stays proportional to the empty
            // archetypes.
            let (meta, occupied) = (&self.meta, &self.occupied);
            self.queue.retain(|&(a, stamp)| Self::current(meta, occupied, a, stamp));
        }
    }

    fn current(meta: &[Meta], occupied: &[u64], a: u32, stamp: u32) -> bool {
        let m = meta[a as usize];
        m.emptied == stamp && !m.pinned && !m.dead && !is_occupied(occupied, a as usize)
    }

    /// Whether the world keeps more empty archetypes than the policy allows.
    #[inline]
    pub(crate) fn over(&self) -> bool {
        self.empty > KEEP_EMPTY.max(self.live)
    }

    /// The archetype that has been empty the longest, taken off the queue,
    /// if any may be reclaimed.
    pub(crate) fn oldest_empty(&mut self) -> Option<usize> {
        while let Some((a, stamp)) = self.queue.pop_front() {
            if Self::current(&self.meta, &self.occupied, a, stamp) {
                return Some(a as usize);
            }
        }
        None
    }

    /// Archetype `a` was reclaimed, leaving `shell`.
    pub(crate) fn reclaimed(&mut self, a: usize, shell: Shell) {
        self.meta[a].dead = true;
        self.empty -= 1;
        self.free.push(a as u32);
        let width = shell.width();
        if width >= self.shells.len() {
            self.shells.resize_with(width + 1, Vec::new);
        }
        self.shells[width].push(shell);
    }

    /// A dead slot to build an archetype of `width` components in, with the
    /// storage of a reclaimed archetype: one as wide if there is one.
    pub(crate) fn reuse(&mut self, width: usize) -> Option<(usize, Shell)> {
        let a = self.free.pop()? as usize;
        let shell = match self.shells.get_mut(width).and_then(Vec::pop) {
            Some(shell) => shell,
            // As many shells as dead slots: there is one of another width.
            None => self
                .shells
                .iter_mut()
                .rev()
                .find_map(Vec::pop)
                .expect("a shell for every dead slot"),
        };
        Some((a, shell))
    }

    /// Bytes of entity storage kept for reuse: pooled column buffers and the
    /// entity lists of shells.
    pub(crate) fn kept_bytes(&self) -> usize {
        self.buffers.bytes() + self.shells.iter().flatten().map(Archetype::shell_bytes).sum::<usize>()
    }

    /// Whether more than one archetype in five is empty (dead slots count),
    /// and more than [`SCAN_LIST_AFTER`]: then scanning queries visit
    /// [`scan`](Self::scan)'s list instead of every archetype.
    #[inline]
    fn many_empty(&self) -> bool {
        let empty = self.meta.len() - self.live;
        empty > SCAN_LIST_AFTER && empty * 4 > self.live
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

/// Whether bit `a` of `occupied` is set: archetype `a` holds entities.
#[inline]
fn is_occupied(occupied: &[u64], a: usize) -> bool {
    occupied.get(a / 64).is_some_and(|word| word >> (a % 64) & 1 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_list_the_filled_slots_once_most_are_empty() {
        let mut slots = Slots::default();
        for a in 0..130 {
            slots.created(a);
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
