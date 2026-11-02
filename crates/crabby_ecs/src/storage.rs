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
}

impl ComponentInfo {
    /// Type name for Rust components, the registered name otherwise.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Size and alignment of one value.
    pub fn layout(&self) -> Layout {
        self.layout
    }
    /// The Rust type, if this component was registered from Rust.
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }
}

/// One column of values of a single component type. Its capacity is owned by
/// the archetype (all columns of an archetype share it).
pub(crate) struct Column {
    data: NonNull<u8>,
    item: Layout,
    drop: Option<DropFn>,
}

impl Column {
    fn new(item: Layout, drop: Option<DropFn>) -> Self {
        // Dangling but aligned: valid for zero-sized items and empty columns.
        let data = NonNull::new(std::ptr::without_provenance_mut(item.align())).unwrap();
        Column { data, item, drop }
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
        Layout::from_size_align(self.item.size().checked_mul(cap).expect("column too large"), self.item.align())
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
    pub(crate) add_edges: HashMap<ComponentId, usize>,
    pub(crate) remove_edges: HashMap<ComponentId, usize>,
}

// SAFETY: columns are plain owned buffers. Rust components are required to be
// `Send + Sync`; components registered at runtime promise the same.
unsafe impl Send for Archetype {}
unsafe impl Sync for Archetype {}

impl Archetype {
    pub(crate) fn new(ids: Box<[ComponentId]>, infos: &[ComponentInfo]) -> Self {
        let columns = ids
            .iter()
            .map(|id| {
                let info = &infos[id.index()];
                Column::new(info.layout, info.drop)
            })
            .collect();
        Archetype {
            ids,
            columns,
            entities: Vec::new(),
            cap: 0,
            add_edges: HashMap::new(),
            remove_edges: HashMap::new(),
        }
    }

    /// Number of entities in this archetype.
    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// `true` if no entity has this exact component set right now.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// The components every entity here has, sorted by id.
    pub fn component_ids(&self) -> &[ComponentId] {
        &self.ids
    }

    /// The entities stored here; row `i` of every column belongs to `entities()[i]`.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// `true` if entities here have component `id`.
    #[inline]
    pub fn has(&self, id: ComponentId) -> bool {
        self.ids.contains(&id)
    }

    #[inline]
    pub(crate) fn column_index(&self, id: ComponentId) -> Option<usize> {
        // Archetypes have few components; a linear scan beats binary search here.
        self.ids.iter().position(|&c| c == id)
    }

    /// Start of the column for component `id` (`len()` values, tightly packed),
    /// or `None` if entities here don't have it. This is what C/C++ systems use.
    #[inline]
    pub fn column_ptr(&self, id: ComponentId) -> Option<*mut u8> {
        self.column_index(id).map(|i| self.columns[i].base())
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
        self.entities.swap_remove(row);
        (row != last).then(|| self.entities[row])
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
            unsafe { c.dealloc(self.cap) }
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
