//! Type-erased storage: component descriptions, columns and archetype tables.

use std::alloc::{self, Layout};
use std::any::TypeId;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::ptr::NonNull;

use crate::entity::Entity;

/// Identifies a component type inside one [`World`](crate::World).
///
/// Rust types get one automatically the first time they are used; other
/// languages get one from [`World::register_component`](crate::World::register_component).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Default)]
pub struct ComponentId(pub(crate) u32);

impl ComponentId {
    /// The id as a plain number (stable for the lifetime of the world).
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Destructor for one component value. `C-unwind` so C/C++ can supply one too.
pub type DropFn = unsafe extern "C-unwind" fn(*mut u8);

pub(crate) unsafe extern "C-unwind" fn drop_ptr<T>(p: *mut u8) {
    unsafe { p.cast::<T>().drop_in_place() }
}

/// What the world knows about a component type.
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub(crate) name: String,
    pub(crate) layout: Layout,
    pub(crate) drop: Option<DropFn>,
    pub(crate) type_id: Option<TypeId>,
    /// Change stamps are kept for this component (see `World::track_changes`).
    pub(crate) tracked: bool,
}

impl ComponentInfo {
    /// Type name for Rust components, the registered name otherwise.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Size and alignment of one value.
    #[must_use]
    pub fn layout(&self) -> Layout {
        self.layout
    }
    /// The Rust type, if this component was registered from Rust.
    #[must_use]
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }
    /// Whether values have a destructor. Values without one are plain bytes
    /// as far as the world is concerned (C/C++ may only touch those).
    #[must_use]
    pub fn needs_drop(&self) -> bool {
        self.drop.is_some()
    }
    /// Whether the world records when values of this component are added,
    /// changed and removed: [`World::track_changes`](crate::World::track_changes).
    #[must_use]
    pub fn tracks_changes(&self) -> bool {
        self.tracked
    }
}

/// One column of values of a single component type. Its capacity is owned by
/// the archetype (all columns of an archetype share it).
///
/// A column whose component is tracked ([`World::track_changes`]) keeps two
/// `u32` change stamps per row in the same buffer, after the values: when
/// each value was added, and when it last changed. Tracking adds nothing to
/// this struct, and an untracked column (every column, unless a game asks)
/// is laid out and handled exactly as if change detection didn't exist:
/// loops over many archetypes are sensitive to the size of this struct.
///
/// [`World::track_changes`]: crate::World::track_changes
pub(crate) struct Column {
    data: NonNull<u8>,
    /// Size of one value.
    size: usize,
    drop: Option<DropFn>,
    /// Alignment of one value.
    align: u32,
    /// Whether the buffer also holds the change stamps (see [`Column::stamps`]).
    tracked: bool,
}

/// Bytes of change stamps per row of a tracked column: `added` and `changed`.
const STAMP_BYTES: usize = 2 * std::mem::size_of::<u32>();

impl Column {
    fn new(item: Layout, drop: Option<DropFn>) -> Self {
        // Dangling but aligned: valid for zero-sized items and empty columns.
        let data = NonNull::new(std::ptr::without_provenance_mut(item.align())).unwrap();
        Column {
            data,
            size: item.size(),
            drop,
            align: u32::try_from(item.align()).expect("component alignment too large"),
            tracked: false,
        }
    }

    /// Bytes of change stamps per row: two `u32`s if tracked.
    fn stamp_bytes(&self) -> usize {
        if self.tracked { STAMP_BYTES } else { 0 }
    }

    /// Where the stamps start in a tracked column's buffer for `cap` rows:
    /// after the values, aligned for `u32`. [`Column::array`] checked that
    /// this doesn't overflow when the buffer was made.
    #[inline]
    fn stamps_offset(&self, cap: usize) -> usize {
        (self.size * cap).next_multiple_of(std::mem::align_of::<u32>())
    }

    /// The `added` and `changed` stamp arrays, `cap` stamps each, if this
    /// component is tracked. `cap` must be the column's capacity.
    #[inline]
    pub(crate) fn stamps(&self, cap: usize) -> Option<(*mut u32, *mut u32)> {
        if !self.tracked {
            return None;
        }
        // SAFETY: a tracked column's buffer for `cap` rows holds the values,
        // then both arrays. With no capacity the offset is 0 and the
        // (dangling) pointers are never read.
        unsafe {
            let added = self.data.as_ptr().add(self.stamps_offset(cap)).cast::<u32>();
            Some((added, added.add(cap)))
        }
    }

    /// Makes room for stamps in the buffer of `cap` rows, and stamps the
    /// first `len` rows.
    ///
    /// # Safety
    /// The column is untracked, and `len <= cap`, the archetype's capacity.
    unsafe fn start_tracking(&mut self, cap: usize, len: usize, stamp: u32) {
        let old = self.array(cap);
        self.tracked = true;
        let new = self.array(cap);
        if new.size() == 0 {
            return;
        }
        let p = unsafe {
            if old.size() == 0 {
                alloc::alloc(new)
            } else if old.align() == new.align() {
                alloc::realloc(self.data.as_ptr(), old, new.size())
            } else {
                // Stamps need a stricter alignment than the values: a new buffer.
                let p = alloc::alloc(new);
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(self.data.as_ptr(), p, old.size());
                    alloc::dealloc(self.data.as_ptr(), old);
                }
                p
            }
        };
        self.data = NonNull::new(p).unwrap_or_else(|| alloc::handle_alloc_error(new));
        let (added, changed) = self.stamps(cap).expect("tracked");
        for row in 0..len {
            // SAFETY: `row < cap`, the length of both arrays.
            unsafe {
                added.add(row).write(stamp);
                changed.add(row).write(stamp);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn base(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn ptr(&self, row: usize) -> *mut u8 {
        unsafe { self.data.as_ptr().add(row * self.size) }
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn drop_fn(&self) -> Option<DropFn> {
        self.drop
    }

    /// The buffer for `cap` rows: the values, then the stamps of a tracked
    /// column.
    fn array(&self, cap: usize) -> Layout {
        let values = self.size.checked_mul(cap).expect("column too large");
        let (size, align) = if self.tracked {
            let size = values
                .checked_next_multiple_of(std::mem::align_of::<u32>())
                .and_then(|values| values.checked_add(cap.checked_mul(STAMP_BYTES)?))
                .expect("column too large");
            (size, (self.align as usize).max(std::mem::align_of::<u32>()))
        } else {
            (values, self.align as usize)
        };
        Layout::from_size_align(size, align).expect("column too large")
    }

    unsafe fn grow(&mut self, old_cap: usize, new_cap: usize) {
        let (old, new) = (self.array(old_cap), self.array(new_cap));
        if new.size() == 0 {
            return;
        }
        let p = unsafe {
            if old.size() == 0 {
                alloc::alloc(new)
            } else {
                alloc::realloc(self.data.as_ptr(), old, new.size())
            }
        };
        self.data = NonNull::new(p).unwrap_or_else(|| alloc::handle_alloc_error(new));
        if self.tracked && old_cap != 0 {
            // The stamps still sit after `old_cap` values: move them behind
            // `new_cap` values. `changed` goes first; both arrays move up,
            // and it lands past where `added` still is.
            unsafe {
                let from = self.data.as_ptr().add(self.stamps_offset(old_cap)).cast::<u32>();
                let (added, changed) = self.stamps(new_cap).expect("tracked");
                std::ptr::copy(from.add(old_cap), changed, old_cap);
                std::ptr::copy(from, added, old_cap);
            }
        }
    }

    /// Moves the value at `last` into `row` without dropping `row`.
    #[inline]
    pub(crate) unsafe fn swap_remove_forget(&mut self, row: usize, last: usize) {
        if row != last {
            unsafe { std::ptr::copy_nonoverlapping(self.ptr(last), self.ptr(row), self.size) }
        }
    }

    /// Reorders the first `perm.len()` values (and their stamps): new row
    /// `i` gets old row `perm[i]`. `perm` must be a permutation of
    /// `0..perm.len()`, and `cap` the column's capacity.
    unsafe fn permute(&mut self, perm: &[u32], cap: usize) {
        let size = self.size;
        let layout = self.array(cap);
        if layout.size() == 0 || perm.is_empty() {
            return;
        }
        let new = unsafe { alloc::alloc(layout) };
        let new = NonNull::new(new).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        if size != 0 {
            for (i, &old) in perm.iter().enumerate() {
                unsafe { std::ptr::copy_nonoverlapping(self.ptr(old as usize), new.as_ptr().add(i * size), size) };
            }
        }
        if let Some((added, changed)) = self.stamps(cap) {
            // SAFETY: the first `perm.len()` stamps of both arrays are live,
            // every `perm` entry is below that, and the new buffer has the
            // same layout.
            unsafe {
                let to = new.as_ptr().add(self.stamps_offset(cap)).cast::<u32>();
                for (i, &old) in perm.iter().enumerate() {
                    to.add(i).write(added.add(old as usize).read());
                    to.add(cap + i).write(changed.add(old as usize).read());
                }
            }
        }
        unsafe { alloc::dealloc(self.data.as_ptr(), layout) };
        self.data = new;
    }

    unsafe fn dealloc(&mut self, cap: usize) {
        let layout = self.array(cap);
        if layout.size() != 0 {
            unsafe { alloc::dealloc(self.data.as_ptr(), layout) }
        }
    }
}

/// The bit of [`Archetype::cap`] that says the archetype keeps change stamps.
/// Capacities never get near it: rows are indexed by `u32`.
const TRACKED: usize = 1 << (usize::BITS - 1);

/// All entities that have exactly the same set of components, stored as one
/// column per component (struct of arrays).
pub struct Archetype {
    pub(crate) ids: Box<[ComponentId]>,
    pub(crate) columns: Box<[Column]>,
    pub(crate) entities: Vec<Entity>,
    /// How many rows every column has room for, and in the top bit
    /// ([`TRACKED`]) whether any column here keeps change stamps: `false`
    /// skips all stamp work. One word for both, so that change tracking
    /// doesn't grow the archetype: at 128 bytes each, a loop over many
    /// archetypes reads one cache line of every archetype.
    cap: usize,
    pub(crate) add_edges: EdgeMap,
    pub(crate) remove_edges: EdgeMap,
}

// SAFETY: columns are plain owned buffers. Rust components are required to be
// `Send + Sync`; components registered at runtime promise the same.
unsafe impl Send for Archetype {}
unsafe impl Sync for Archetype {}

impl Archetype {
    pub(crate) fn new(ids: Box<[ComponentId]>, infos: &[ComponentInfo]) -> Self {
        let mut tracked = false;
        let columns = ids
            .iter()
            .map(|id| {
                let info = &infos[id.index()];
                let mut column = Column::new(info.layout, info.drop);
                if info.tracked {
                    tracked = true;
                    // SAFETY: a new, untracked column of an archetype with no capacity.
                    unsafe { column.start_tracking(0, 0, 0) };
                }
                column
            })
            .collect();
        Archetype {
            ids,
            columns,
            entities: Vec::new(),
            cap: if tracked { TRACKED } else { 0 },
            add_edges: EdgeMap::default(),
            remove_edges: EdgeMap::default(),
        }
    }

    /// Number of entities in this archetype.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// `true` if no entity has this exact component set right now.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Bytes per entity in this archetype: the size of every component plus
    /// the entity handle, and 8 bytes of change stamps for each component
    /// whose changes are tracked
    /// ([`World::track_changes`](crate::World::track_changes)). What one more
    /// entity costs, with no slack.
    ///
    /// ```
    /// use ponko_ecs::World;
    ///
    /// struct Pos(f32, f32);
    /// let mut world = World::new();
    /// world.spawn((Pos(0.0, 0.0),));
    /// let a = world.archetypes().iter().find(|a| a.len() == 1).unwrap();
    /// assert_eq!(a.bytes_per_entity(), 8 + 8); // Pos + the entity handle
    ///
    /// world.track_changes::<Pos>();
    /// let a = world.archetypes().iter().find(|a| a.len() == 1).unwrap();
    /// assert_eq!(a.bytes_per_entity(), 8 + 8 + 8); // and two 4-byte stamps
    /// ```
    pub fn bytes_per_entity(&self) -> usize {
        self.columns.iter().map(|c| c.size() + c.stamp_bytes()).sum::<usize>() + std::mem::size_of::<Entity>()
    }

    /// Bytes this archetype has allocated for entity data (including spare
    /// capacity, which is why it can exceed `len * bytes_per_entity`).
    ///
    /// ```
    /// use ponko_ecs::World;
    ///
    /// struct Pos(f32, f32);
    /// let mut world = World::new();
    /// world.spawn_batch((0..100).map(|i| (Pos(i as f32, 0.0),)));
    /// let a = world.archetypes().iter().find(|a| a.len() == 100).unwrap();
    /// assert!(a.allocated_bytes() >= 100 * a.bytes_per_entity());
    /// ```
    #[must_use]
    pub fn allocated_bytes(&self) -> usize {
        self.columns
            .iter()
            .map(|c| (c.size() + c.stamp_bytes()) * self.cap())
            .sum::<usize>()
            + self.entities.capacity() * std::mem::size_of::<Entity>()
    }

    /// The components every entity here has, sorted by id.
    #[must_use]
    pub fn component_ids(&self) -> &[ComponentId] {
        &self.ids
    }

    /// The entities stored here; row `i` of every column belongs to `entities()[i]`.
    #[must_use]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// `true` if entities here have component `id`.
    #[inline]
    #[must_use]
    pub fn has(&self, id: ComponentId) -> bool {
        self.ids.contains(&id)
    }

    #[inline]
    pub(crate) fn column_index(&self, id: ComponentId) -> Option<usize> {
        // Archetypes have few components, so a linear scan is kept. Measured in isolation
        // (benches `membership/*`, Apple M4 Max, hot cache): 1-2 ns at 2 components, 3 ns at 8,
        // 5 ns at 16, 9 ns at 32, 14 ns at 64. A bitset or id-indexed table answers in under
        // 1 ns but costs 50-640 B per archetype and a second cache line; binary search over the
        // sorted ids is 2-7x faster than the scan from 8 components up, with no extra memory,
        // but is not yet shown to matter in a query-level benchmark (lookups happen once per
        // archetype and column, never per entity). Structural changes do not scan per column:
        // `move_row` merges the two sorted id lists.
        self.ids.iter().position(|&c| c == id)
    }

    /// Start of the column for component `id` (`len()` values, tightly packed),
    /// or `None` if entities here don't have it. This is what C/C++ systems use.
    #[inline]
    #[must_use]
    pub fn column_ptr(&self, id: ComponentId) -> Option<*mut u8> {
        self.column_index(id).map(|i| self.columns[i].base())
    }

    /// Reorders all rows: new row `i` gets old row `perm[i]`. `perm` must be
    /// a permutation of `0..len()`. The caller updates entity locations.
    pub(crate) fn permute(&mut self, perm: &[u32]) {
        assert_eq!(perm.len(), self.len());
        debug_assert!({
            let mut seen = vec![false; perm.len()];
            perm.iter().all(|&p| !std::mem::replace(&mut seen[p as usize], true))
        });
        let cap = self.cap();
        for c in self.columns.iter_mut() {
            unsafe { c.permute(perm, cap) };
        }
        let old = std::mem::take(&mut self.entities);
        self.entities = perm.iter().map(|&p| old[p as usize]).collect();
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        let needed = self.len().checked_add(additional).expect("too many entities");
        if needed <= self.cap() {
            return;
        }
        let cap = self.cap();
        let new_cap = needed.max(cap * 2).max(8);
        for c in self.columns.iter_mut() {
            unsafe { c.grow(cap, new_cap) }
        }
        self.cap = new_cap | (self.cap & TRACKED);
        self.entities.reserve(new_cap - self.entities.len());
    }

    /// Makes room for one more row and, if this archetype tracks changes,
    /// stamps that row as added at `changes`' current tick: what a spawn
    /// does before it writes the row.
    ///
    /// For an archetype that tracks nothing and has room, this is the one
    /// comparison `reserve(1)` makes: the tracked flag is the sign bit of
    /// `cap`, so a tracked archetype fails the signed comparison and takes
    /// the second path together with a full one. (A length can't reach the
    /// sign bit: a `Vec` holds at most `isize::MAX` bytes.)
    pub(crate) fn reserve_row(&mut self, changes: &crate::change::Changes) {
        if (self.len() + 1) as isize <= self.cap as isize {
            return;
        }
        self.reserve_row_slow(changes.stamp());
    }

    /// The rare half of [`reserve_row`](Self::reserve_row): the archetype is
    /// full, or it is tracked.
    #[inline(never)]
    fn reserve_row_slow(&mut self, stamp: u32) {
        self.reserve(1);
        if self.tracked() {
            self.stamp_row(self.len(), stamp);
        }
    }

    /// Removes `row` by moving the last row into it, dropping the removed
    /// values. Returns the entity that moved into `row`, if any.
    pub(crate) unsafe fn swap_remove_drop(&mut self, row: usize) -> Option<Entity> {
        let last = self.len() - 1;
        for c in self.columns.iter_mut() {
            if let Some(d) = c.drop {
                unsafe { d(c.ptr(row)) }
            }
            unsafe { c.swap_remove_forget(row, last) }
        }
        if self.tracked() {
            self.swap_remove_stamps(row, last);
        }
        self.entities.swap_remove(row);
        (row != last).then(|| self.entities[row])
    }

    // ---- change stamps (tracked components only) ---------------------------

    /// `true` if any component here keeps change stamps.
    #[inline(always)]
    pub(crate) fn tracked(&self) -> bool {
        self.cap & TRACKED != 0
    }

    /// Rows every column has room for.
    #[inline]
    fn cap(&self) -> usize {
        self.cap & !TRACKED
    }

    /// Starts keeping stamps for column `col`; rows already here count as
    /// added and changed at `stamp`.
    pub(crate) fn start_tracking(&mut self, col: usize, stamp: u32) {
        let (cap, len) = (self.cap(), self.len());
        let column = &mut self.columns[col];
        if !column.tracked {
            // SAFETY: untracked, and `len <= cap` rows are in use.
            unsafe { column.start_tracking(cap, len, stamp) };
            self.cap |= TRACKED;
        }
    }

    /// Stamps every tracked column of `row` as added and changed at `stamp`.
    /// `row` must be within the capacity (a row being spawned).
    pub(crate) fn stamp_row(&self, row: usize, stamp: u32) {
        let cap = self.cap();
        debug_assert!(row < cap);
        for c in self.columns.iter() {
            if let Some((added, changed)) = c.stamps(cap) {
                // SAFETY: `row < cap`, the length of both stamp arrays.
                unsafe {
                    added.add(row).write(stamp);
                    changed.add(row).write(stamp);
                }
            }
        }
    }

    /// Stamps column `col` of `row`: changed at `stamp`, and added too if
    /// `added`. Does nothing for an untracked column.
    #[inline]
    pub(crate) fn stamp(&self, col: usize, row: usize, stamp: u32, added: bool) {
        debug_assert!(row < self.cap());
        if let Some((a, c)) = self.columns[col].stamps(self.cap()) {
            // SAFETY: `row < cap`, the length of both stamp arrays.
            unsafe {
                c.add(row).write(stamp);
                if added {
                    a.add(row).write(stamp);
                }
            }
        }
    }

    /// The live `changed` stamps of component `id`, to mark rows as changed;
    /// `None` if it isn't here or isn't tracked.
    pub(crate) fn changed_stamps(&self, id: ComponentId) -> Option<*mut u32> {
        self.columns[self.column_index(id)?].stamps(self.cap()).map(|s| s.1)
    }

    /// The live stamps of component `id` for reading: its `added` array if
    /// `added`, else its `changed` array.
    pub(crate) fn read_stamps(&self, id: ComponentId, added: bool) -> Option<*const u32> {
        let (a, c) = self.columns[self.column_index(id)?].stamps(self.cap())?;
        Some(if added { a.cast_const() } else { c.cast_const() })
    }

    /// Moves the stamps of row `last` into `row` (the stamp half of a swap-remove).
    pub(crate) fn swap_remove_stamps(&self, row: usize, last: usize) {
        if row == last {
            return;
        }
        let cap = self.cap();
        for c in self.columns.iter() {
            if let Some((added, changed)) = c.stamps(cap) {
                // SAFETY: both rows are live rows of this archetype.
                unsafe {
                    added.add(row).write(added.add(last).read());
                    changed.add(row).write(changed.add(last).read());
                }
            }
        }
    }

    /// Copies the stamps of `row` to `dst_row` of `dst`, for every tracked
    /// component both archetypes have: moving an entity between archetypes
    /// isn't a change to the components it keeps. `dst_row` must be within
    /// `dst`'s capacity.
    pub(crate) fn copy_stamps_to(&self, row: usize, dst: &Archetype, dst_row: usize) {
        debug_assert!(row < self.len() && dst_row < dst.cap());
        let mut j = 0;
        let cap = self.cap();
        for (i, c) in self.columns.iter().enumerate() {
            let Some((added, changed)) = c.stamps(cap) else {
                continue;
            };
            let id = self.ids[i];
            while j < dst.ids.len() && dst.ids[j] < id {
                j += 1;
            }
            if j < dst.ids.len() && dst.ids[j] == id {
                // A component is tracked in every archetype or in none.
                let (dst_added, dst_changed) = dst.columns[j].stamps(dst.cap()).expect("tracked in both");
                // SAFETY: `row` is live here; `dst_row` is within `dst`'s capacity.
                unsafe {
                    dst_added.add(dst_row).write(added.add(row).read());
                    dst_changed.add(dst_row).write(changed.add(row).read());
                }
            }
        }
    }

    /// Ages stamps so they stay comparable after the tick counter's low 32
    /// bits wrap: any stamp older than `max_age` ticks becomes exactly
    /// `max_age` ticks old. `now` is the current tick's low 32 bits.
    pub(crate) fn age_stamps(&self, now: u32, max_age: u32) {
        let (len, cap) = (self.len(), self.cap());
        for c in self.columns.iter() {
            let Some((added, changed)) = c.stamps(cap) else {
                continue;
            };
            for stamps in [added, changed] {
                for row in 0..len {
                    // SAFETY: `row` is a live row of a tracked column.
                    unsafe {
                        let stamp = stamps.add(row);
                        if now.wrapping_sub(stamp.read()) > max_age {
                            stamp.write(now.wrapping_sub(max_age));
                        }
                    }
                }
            }
        }
    }
}

impl Drop for Archetype {
    fn drop(&mut self) {
        let (len, cap) = (self.len(), self.cap());
        for c in self.columns.iter_mut() {
            if let Some(d) = c.drop {
                for row in 0..len {
                    unsafe { d(c.ptr(row)) }
                }
            }
            unsafe { c.dealloc(cap) }
        }
    }
}

/// `TypeId` is already a good hash; skip SipHash for type lookups.
#[derive(Default)]
pub(crate) struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 ^= n;
    }
}

pub(crate) type TypeMap<V> = HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

/// Component ids are small dense integers: one multiply is a good hash, and
/// SipHash was most of the cost of a cached structural change.
#[derive(Default)]
pub(crate) struct ComponentIdHasher(u64);

impl Hasher for ComponentIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.0 = u64::from(n).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// Transition-edge cache of an archetype: component id -> destination archetype.
pub(crate) type EdgeMap = HashMap<ComponentId, usize, BuildHasherDefault<ComponentIdHasher>>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Loops over many archetypes read these structs once per archetype, so
    /// their size is part of the cost of every query. Change tracking keeps
    /// its flag in `Archetype::cap` and its stamps in the column buffers so
    /// that worlds which don't use it see the same structs as before.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn change_tracking_adds_no_bytes_to_columns_or_archetypes() {
        assert_eq!(std::mem::size_of::<Column>(), 32);
        assert_eq!(std::mem::size_of::<Archetype>(), 128);
    }

    #[test]
    fn a_tracked_buffer_holds_the_values_then_aligned_stamps() {
        // Three 1-byte values, then (after one byte of padding) 3 + 3 stamps.
        let mut column = Column::new(Layout::new::<u8>(), None);
        assert_eq!(column.array(3), Layout::from_size_align(3, 1).unwrap());
        assert_eq!(column.stamps(3), None);
        // SAFETY: untracked, and no rows are in use.
        unsafe { column.start_tracking(0, 0, 0) };
        assert_eq!(column.array(0).size(), 0);
        assert_eq!(column.array(3), Layout::from_size_align(4 + 3 * 8, 4).unwrap());
        assert_eq!(column.stamps_offset(3), 4);
        // A wider alignment is kept; zero-sized values leave only stamps.
        let mut wide = Column::new(Layout::new::<u128>(), None);
        let mut empty = Column::new(Layout::new::<()>(), None);
        // SAFETY: as above.
        unsafe {
            wide.start_tracking(0, 0, 0);
            empty.start_tracking(0, 0, 0);
        }
        assert_eq!(wide.array(2), Layout::from_size_align(32 + 16, 16).unwrap());
        assert_eq!(empty.array(5), Layout::from_size_align(40, 4).unwrap());
    }
}
