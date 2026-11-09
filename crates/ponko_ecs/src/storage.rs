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
pub(crate) struct Column {
    data: NonNull<u8>,
    item: Layout,
    drop: Option<DropFn>,
    /// Change stamps of a tracked component, one `u32` per row in each array
    /// (same capacity as `data`): when the value was added, and when it last
    /// changed. Both null for an untracked component, which is every
    /// component unless `World::track_changes` asked. All stamp work sits
    /// behind `Archetype::tracked`, so untracked archetypes never look here.
    added: *mut u32,
    changed: *mut u32,
}

impl Column {
    fn new(item: Layout, drop: Option<DropFn>) -> Self {
        // Dangling but aligned: valid for zero-sized items and empty columns.
        let data = NonNull::new(std::ptr::without_provenance_mut(item.align())).unwrap();
        Column {
            data,
            item,
            drop,
            added: std::ptr::null_mut(),
            changed: std::ptr::null_mut(),
        }
    }

    /// The `added` and `changed` stamp arrays, if this component is tracked.
    #[inline]
    /// Bytes of change stamps per row: two `u32`s if tracked.
    fn stamp_bytes(&self) -> usize {
        if self.changed.is_null() {
            0
        } else {
            2 * std::mem::size_of::<u32>()
        }
    }

    pub(crate) fn stamps(&self) -> Option<(*mut u32, *mut u32)> {
        (!self.changed.is_null()).then_some((self.added, self.changed))
    }

    /// Allocates stamp arrays for `cap` rows and stamps the first `len`.
    ///
    /// # Safety
    /// The column is untracked, and `len <= cap`, the archetype's capacity.
    unsafe fn start_tracking(&mut self, cap: usize, len: usize, stamp: u32) {
        // A dangling pointer marks "tracked, no rows yet": it is never read.
        self.added = NonNull::dangling().as_ptr();
        self.changed = NonNull::dangling().as_ptr();
        unsafe { self.grow_stamps(0, cap) };
        for row in 0..len {
            unsafe {
                self.added.add(row).write(stamp);
                self.changed.add(row).write(stamp);
            }
        }
    }

    /// Resizes both stamp arrays of a tracked column.
    unsafe fn grow_stamps(&mut self, old_cap: usize, new_cap: usize) {
        if new_cap == 0 {
            return;
        }
        let new = Layout::array::<u32>(new_cap).expect("column too large");
        for p in [&mut self.added, &mut self.changed] {
            let grown = unsafe {
                if old_cap == 0 {
                    alloc::alloc(new)
                } else {
                    alloc::realloc((*p).cast(), Layout::array::<u32>(old_cap).unwrap(), new.size())
                }
            };
            if grown.is_null() {
                alloc::handle_alloc_error(new);
            }
            *p = grown.cast();
        }
    }

    unsafe fn dealloc_stamps(&mut self, cap: usize) {
        if !self.changed.is_null() && cap != 0 {
            let layout = Layout::array::<u32>(cap).unwrap();
            unsafe {
                alloc::dealloc(self.added.cast(), layout);
                alloc::dealloc(self.changed.cast(), layout);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn base(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn ptr(&self, row: usize) -> *mut u8 {
        unsafe { self.data.as_ptr().add(row * self.item.size()) }
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.item.size()
    }

    pub(crate) fn drop_fn(&self) -> Option<DropFn> {
        self.drop
    }

    fn array(&self, cap: usize) -> Layout {
        Layout::from_size_align(
            self.item.size().checked_mul(cap).expect("column too large"),
            self.item.align(),
        )
        .expect("column too large")
    }

    unsafe fn grow(&mut self, old_cap: usize, new_cap: usize) {
        if self.item.size() == 0 {
            return;
        }
        let new = self.array(new_cap);
        let p = unsafe {
            if old_cap == 0 {
                alloc::alloc(new)
            } else {
                alloc::realloc(self.data.as_ptr(), self.array(old_cap), new.size())
            }
        };
        self.data = NonNull::new(p).unwrap_or_else(|| alloc::handle_alloc_error(new));
    }

    /// Moves the value at `last` into `row` without dropping `row`.
    #[inline]
    pub(crate) unsafe fn swap_remove_forget(&mut self, row: usize, last: usize) {
        if row != last {
            unsafe { std::ptr::copy_nonoverlapping(self.ptr(last), self.ptr(row), self.item.size()) }
        }
    }

    /// Reorders the first `perm.len()` values: new row `i` gets old row
    /// `perm[i]`. `perm` must be a permutation of `0..perm.len()`, and
    /// `cap` the column's capacity.
    unsafe fn permute(&mut self, perm: &[u32], cap: usize) {
        let size = self.item.size();
        if size == 0 || perm.is_empty() {
            return;
        }
        let layout = self.array(cap);
        let new = unsafe { alloc::alloc(layout) };
        let new = NonNull::new(new).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        for (i, &old) in perm.iter().enumerate() {
            unsafe { std::ptr::copy_nonoverlapping(self.ptr(old as usize), new.as_ptr().add(i * size), size) };
        }
        unsafe { alloc::dealloc(self.data.as_ptr(), layout) };
        self.data = new;
    }

    unsafe fn dealloc(&mut self, cap: usize) {
        if self.item.size() != 0 && cap != 0 {
            unsafe { alloc::dealloc(self.data.as_ptr(), self.array(cap)) }
        }
    }
}

/// All entities that have exactly the same set of components, stored as one
/// column per component (struct of arrays).
pub struct Archetype {
    pub(crate) ids: Box<[ComponentId]>,
    pub(crate) columns: Box<[Column]>,
    pub(crate) entities: Vec<Entity>,
    cap: usize,
    /// Whether any column here keeps change stamps. Checked once per
    /// structural change or query visit; `false` skips all stamp work.
    tracked: bool,
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
            cap: 0,
            tracked,
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
            .map(|c| (c.size() + c.stamp_bytes()) * self.cap)
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
        for c in self.columns.iter_mut() {
            unsafe { c.permute(perm, self.cap) };
        }
        if self.tracked {
            self.permute_stamps(perm);
        }
        let old = std::mem::take(&mut self.entities);
        self.entities = perm.iter().map(|&p| old[p as usize]).collect();
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        let needed = self.len().checked_add(additional).expect("too many entities");
        if needed <= self.cap {
            return;
        }
        let new_cap = needed.max(self.cap * 2).max(8);
        for c in self.columns.iter_mut() {
            unsafe { c.grow(self.cap, new_cap) }
        }
        if self.tracked {
            for c in self.columns.iter_mut().filter(|c| c.stamps().is_some()) {
                // SAFETY: a tracked column's stamp arrays hold `self.cap` rows.
                unsafe { c.grow_stamps(self.cap, new_cap) }
            }
        }
        self.cap = new_cap;
        self.entities.reserve(new_cap - self.entities.len());
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
        if self.tracked {
            self.swap_remove_stamps(row, last);
        }
        self.entities.swap_remove(row);
        (row != last).then(|| self.entities[row])
    }

    // ---- change stamps (tracked components only) ---------------------------

    /// `true` if any component here keeps change stamps.
    #[inline(always)]
    pub(crate) fn tracked(&self) -> bool {
        self.tracked
    }

    /// Starts keeping stamps for column `col`; rows already here count as
    /// added and changed at `stamp`.
    pub(crate) fn start_tracking(&mut self, col: usize, stamp: u32) {
        let (cap, len) = (self.cap, self.len());
        let column = &mut self.columns[col];
        if column.stamps().is_none() {
            // SAFETY: untracked, and `len <= cap` rows are in use.
            unsafe { column.start_tracking(cap, len, stamp) };
            self.tracked = true;
        }
    }

    /// Stamps every tracked column of `row` as added and changed at `stamp`.
    /// `row` must be within the capacity (a row being spawned).
    pub(crate) fn stamp_row(&self, row: usize, stamp: u32) {
        debug_assert!(row < self.cap);
        for c in self.columns.iter() {
            if let Some((added, changed)) = c.stamps() {
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
        debug_assert!(row < self.cap);
        if let Some((a, c)) = self.columns[col].stamps() {
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
        self.columns[self.column_index(id)?].stamps().map(|s| s.1)
    }

    /// The live stamps of component `id` for reading: its `added` array if
    /// `added`, else its `changed` array.
    pub(crate) fn read_stamps(&self, id: ComponentId, added: bool) -> Option<*const u32> {
        let (a, c) = self.columns[self.column_index(id)?].stamps()?;
        Some(if added { a.cast_const() } else { c.cast_const() })
    }

    /// Moves the stamps of row `last` into `row` (the stamp half of a swap-remove).
    pub(crate) fn swap_remove_stamps(&self, row: usize, last: usize) {
        if row == last {
            return;
        }
        for c in self.columns.iter() {
            if let Some((added, changed)) = c.stamps() {
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
        debug_assert!(row < self.len() && dst_row < dst.cap);
        let mut j = 0;
        for (i, c) in self.columns.iter().enumerate() {
            let Some((added, changed)) = c.stamps() else { continue };
            let id = self.ids[i];
            while j < dst.ids.len() && dst.ids[j] < id {
                j += 1;
            }
            if j < dst.ids.len() && dst.ids[j] == id {
                // A component is tracked in every archetype or in none.
                let (dst_added, dst_changed) = dst.columns[j].stamps().expect("tracked in both");
                // SAFETY: `row` is live here; `dst_row` is within `dst`'s capacity.
                unsafe {
                    dst_added.add(dst_row).write(added.add(row).read());
                    dst_changed.add(dst_row).write(changed.add(row).read());
                }
            }
        }
    }

    /// Reorders the stamps like `permute` reorders the values.
    fn permute_stamps(&mut self, perm: &[u32]) {
        let mut old = Vec::with_capacity(perm.len());
        for c in self.columns.iter() {
            let Some((added, changed)) = c.stamps() else { continue };
            for stamps in [added, changed] {
                old.clear();
                // SAFETY: the first `perm.len() == len()` stamps are live, and
                // every `perm` entry is below that.
                unsafe {
                    old.extend_from_slice(std::slice::from_raw_parts(stamps, perm.len()));
                    for (i, &from) in perm.iter().enumerate() {
                        stamps.add(i).write(old[from as usize]);
                    }
                }
            }
        }
    }

    /// Ages stamps so they stay comparable after the tick counter's low 32
    /// bits wrap: any stamp older than `max_age` ticks becomes exactly
    /// `max_age` ticks old. `now` is the current tick's low 32 bits.
    pub(crate) fn age_stamps(&self, now: u32, max_age: u32) {
        let len = self.len();
        for c in self.columns.iter() {
            let Some((added, changed)) = c.stamps() else { continue };
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
        let len = self.len();
        for c in self.columns.iter_mut() {
            if let Some(d) = c.drop {
                for row in 0..len {
                    unsafe { d(c.ptr(row)) }
                }
            }
            unsafe {
                c.dealloc(self.cap);
                c.dealloc_stamps(self.cap);
            }
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
