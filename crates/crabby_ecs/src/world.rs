//! The [`World`]: owns every entity, component and resource.

use std::alloc::Layout;
use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::bundle::{Bundle, Component};
use crate::commands::Commands;
use crate::entity::{Entities, Entity, Location};
use crate::query::{Query, QueryData};
use crate::storage::{Archetype, ComponentId, ComponentInfo, DropFn, TypeMap, drop_ptr};

struct BundleInfo {
    archetype: usize,
    /// Column index in the archetype for each tuple element.
    columns: Box<[usize]>,
}

/// One chunk of entities handed to [`World::for_each_chunk`]: `len` entities
/// and one column pointer per requested component, in request order.
pub struct Chunk<'a> {
    /// The entities in this chunk; row `i` of each column belongs to `entities[i]`.
    pub entities: &'a [Entity],
    /// Column start pointers, each pointing at `entities.len()` packed values.
    pub columns: &'a [*mut u8],
}

impl Chunk<'_> {
    /// Number of entities (rows) in the chunk.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// `true` if the chunk has no rows (never true for chunks passed to callbacks).
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// Holds everything in the simulation: entities, their components, and
/// global resources.
///
/// ```
/// use crabby_ecs::*;
///
/// struct Pos(f32, f32);
/// struct Vel(f32, f32);
///
/// let mut world = World::new();
/// let crab = world.spawn((Pos(0.0, 0.0), Vel(1.0, 2.0)));
///
/// world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
///     p.0 += v.0;
///     p.1 += v.1;
/// });
///
/// assert_eq!(world.get::<Pos>(crab).unwrap().1, 2.0);
/// world.despawn(crab);
/// assert!(world.get::<Pos>(crab).is_none());
/// ```
pub struct World {
    infos: Vec<ComponentInfo>,
    by_type: TypeMap<ComponentId>,
    by_name: HashMap<String, ComponentId>,
    pub(crate) archetypes: Vec<Archetype>,
    archetype_by_ids: HashMap<Box<[ComponentId]>, usize>,
    pub(crate) entities: Entities,
    bundles: TypeMap<BundleInfo>,
    resources: TypeMap<Box<dyn Any + Send + Sync>>,
    scratch_ids: Vec<ComponentId>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// An empty world.
    pub fn new() -> Self {
        let mut w = World {
            infos: Vec::new(),
            by_type: TypeMap::default(),
            by_name: HashMap::new(),
            archetypes: Vec::new(),
            archetype_by_ids: HashMap::new(),
            entities: Entities::default(),
            bundles: TypeMap::default(),
            resources: TypeMap::default(),
            scratch_ids: Vec::new(),
        };
        w.archetype_for(&[]); // archetype 0: entities with no components
        w
    }

    // ---- entities ----------------------------------------------------------

    /// Creates an entity with the given components and returns its handle.
    ///
    /// Components come as a tuple; note the trailing comma for one component:
    /// `world.spawn((Health(10),))`.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let a = self.bundle_archetype::<B>();
        let e = self.entities.alloc();
        self.spawn_in::<B>(a, bundle, e)
    }

    /// Spawns many entities of the same shape. Faster than calling `spawn` in a loop.
    pub fn spawn_batch<B: Bundle>(&mut self, bundles: impl IntoIterator<Item = B>) {
        let iter = bundles.into_iter();
        let a = self.bundle_archetype::<B>();
        self.archetypes[a].reserve(iter.size_hint().0);
        for b in iter {
            let e = self.entities.alloc();
            self.spawn_in::<B>(a, b, e);
        }
    }

    /// Spawns an entity with exactly the handle `e`, for restoring saved
    /// worlds so that components referring to entities stay valid. Returns
    /// `false` (dropping `bundle`) if `e` is taken, or was despawned in this
    /// world already (old handles stay dead).
    ///
    /// ```
    /// use crabby_ecs::{Entity, World};
    ///
    /// let mut world = World::new();
    /// let saved = Entity::from_bits(7 | 3 << 32); // slot 7, generation 3
    /// assert!(world.spawn_at(saved, ("hello",)));
    /// assert_eq!(world.get::<&str>(saved), Some(&"hello"));
    /// assert!(!world.spawn_at(saved, ("again",))); // taken
    /// let fresh = world.spawn(("new",)); // fresh handles never collide
    /// assert_ne!(fresh, saved);
    /// ```
    pub fn spawn_at<B: Bundle>(&mut self, e: Entity, bundle: B) -> bool {
        if !self.entities.alloc_at(e) {
            return false;
        }
        let a = self.bundle_archetype::<B>();
        self.spawn_in::<B>(a, bundle, e);
        true
    }

    /// Memory held for entity data: component columns and handles of every
    /// archetype, including spare capacity. Doesn't count resources or the
    /// world's bookkeeping (per-slot generation, archetype index).
    ///
    /// ```
    /// use crabby_ecs::World;
    ///
    /// struct Pos(f32, f32);
    /// let mut world = World::new();
    /// world.spawn_batch((0..1000).map(|i| (Pos(i as f32, 0.0),)));
    /// // 8 bytes of Pos + 8 bytes of entity handle each, at least.
    /// assert!(world.allocated_bytes() >= 1000 * 16);
    /// ```
    pub fn allocated_bytes(&self) -> usize {
        self.archetypes.iter().map(Archetype::allocated_bytes).sum()
    }

    /// Despawns every entity. Resources stay.
    ///
    /// ```
    /// use crabby_ecs::World;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn((1u8,));
    /// world.clear();
    /// assert!(world.is_empty() && !world.is_alive(e));
    /// ```
    pub fn clear(&mut self) {
        let all: Vec<Entity> = self
            .archetypes
            .iter()
            .flat_map(|a| a.entities.iter().copied())
            .collect();
        for e in all {
            self.despawn(e);
        }
    }

    /// Despawns every entity **and forgets every entity id**, as if the world
    /// were new; resources and registered components stay. Afterwards, old
    /// handles may match new entities, so use this only when replacing the
    /// whole world, such as when loading a save (whose ids then come back
    /// with [`spawn_at`](Self::spawn_at)). For everything else use
    /// [`clear`](Self::clear).
    ///
    /// ```
    /// use crabby_ecs::World;
    ///
    /// let mut world = World::new();
    /// let e = world.spawn((1u8,));
    /// world.reset();
    /// assert!(world.spawn_at(e, (2u8,))); // the id is free again
    /// ```
    pub fn reset(&mut self) {
        self.clear();
        self.entities = Default::default();
    }

    #[inline]
    fn spawn_in<B: Bundle>(&mut self, a: usize, bundle: B, e: Entity) -> Entity {
        let cols = &self.bundles[&TypeId::of::<B>()].columns;
        let arch = &mut self.archetypes[a];
        arch.reserve(1);
        let row = arch.len();
        unsafe {
            bundle.write(&mut |i, src| {
                let c = &arch.columns[cols[i]];
                std::ptr::copy_nonoverlapping(src, c.ptr(row), c.size());
            });
        }
        arch.entities.push(e);
        self.entities.set(
            e,
            Location {
                archetype: a as u32,
                row: row as u32,
            },
        );
        e
    }

    /// Removes an entity and drops its components. Returns `false` if it was
    /// already gone.
    pub fn despawn(&mut self, e: Entity) -> bool {
        let Some(loc) = self.entities.get(e) else { return false };
        // Free the slot first: if a component's drop panics, the entity is already gone.
        self.entities.free(e);
        let moved = unsafe { self.archetypes[loc.archetype as usize].swap_remove_drop(loc.row as usize) };
        if let Some(moved) = moved {
            self.entities.set_row(moved, loc.row as usize);
        }
        true
    }

    /// `true` if `e` hasn't been despawned.
    pub fn is_alive(&self, e: Entity) -> bool {
        self.entities.get(e).is_some()
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// `true` if there are no live entities.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ---- single-entity access -----------------------------------------------

    /// Component `T` of entity `e`, or `None` if `e` is dead or doesn't have one.
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        let ptr = self.get_dynamic(e, self.component_id::<T>()?)?;
        Some(unsafe { &*ptr.cast::<T>() })
    }

    /// Mutable version of [`get`](Self::get).
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T> {
        let ptr = self.get_dynamic(e, self.component_id::<T>()?)?;
        Some(unsafe { &mut *ptr.cast::<T>() })
    }

    /// `true` if `e` is alive and has component `T`.
    pub fn has<T: Component>(&self, e: Entity) -> bool {
        self.component_id::<T>()
            .is_some_and(|id| self.get_dynamic(e, id).is_some())
    }

    /// Adds component `value` to `e`, replacing any existing `T`.
    /// Returns `false` (and drops `value`) if `e` is dead.
    pub fn insert<T: Component>(&mut self, e: Entity, value: T) -> bool {
        let id = self.register::<T>();
        let value = std::mem::ManuallyDrop::new(value);
        let ok = unsafe { self.insert_dynamic(e, id, (&*value as *const T).cast()) };
        if !ok {
            drop(std::mem::ManuallyDrop::into_inner(value));
        }
        ok
    }

    /// Takes component `T` off `e` and returns it.
    pub fn remove<T: Component>(&mut self, e: Entity) -> Option<T> {
        let id = self.component_id::<T>()?;
        let loc = self.entities.get(e)?;
        let arch = &self.archetypes[loc.archetype as usize];
        let col = arch.column_index(id)?;
        let value = unsafe { arch.columns[col].ptr(loc.row as usize).cast::<T>().read() };
        self.move_without(e, loc, id);
        Some(value)
    }

    // ---- queries -------------------------------------------------------------

    /// Loops over every entity that has the components in `Q`.
    ///
    /// `Q` is `&T`, `&mut T`, `Option<&T>`, `Option<&mut T>`, `Entity`, or a
    /// tuple of those. See [`Query`] for filters and iterators.
    ///
    /// # Panics
    /// If `Q` asks for the same component twice and at least once mutably.
    pub fn query<Q: QueryData>(&mut self) -> Query<'_, Q> {
        Query::new(self)
    }

    /// Reorders entities in memory so that, within each archetype that has
    /// component `T`, rows are sorted by `key(&T)` (ascending; ties keep no
    /// particular order). Handles and component values don't change, only
    /// the order queries visit them in.
    ///
    /// Use it to put entities that are processed together next to each
    /// other: sorting by a spatial cell key every few dozen ticks makes
    /// neighbor queries hit the CPU cache instead of main memory.
    ///
    /// ```
    /// # use crabby_ecs::*;
    /// struct Pos(f32);
    /// let mut world = World::new();
    /// for x in [3.0, 1.0, 2.0] {
    ///     world.spawn((Pos(x),));
    /// }
    /// world.sort_by::<Pos>(|p| p.0 as u64);
    /// let order: Vec<f32> = world.query::<&Pos>().iter().map(|p| p.0).collect();
    /// assert_eq!(order, [1.0, 2.0, 3.0]);
    /// ```
    pub fn sort_by<T: Component>(&mut self, key: impl Fn(&T) -> u64 + Sync) {
        let Some(id) = self.component_id::<T>() else { return };
        let mut keyed: Vec<(u64, u32)> = Vec::new();
        for a in 0..self.archetypes.len() {
            let arch = &self.archetypes[a];
            let Some(base) = arch.column_ptr(id) else { continue };
            let len = arch.len();
            if len < 2 {
                continue;
            }
            let values = unsafe { std::slice::from_raw_parts(base as *const T, len) };
            keyed.clear();
            keyed.extend(values.iter().enumerate().map(|(row, v)| (key(v), row as u32)));
            if keyed.windows(2).all(|w| w[0].0 <= w[1].0) {
                continue; // already in order
            }
            sort_keys(&mut keyed);
            let perm: Vec<u32> = keyed.iter().map(|&(_, row)| row).collect();
            let arch = &mut self.archetypes[a];
            arch.permute(&perm);
            for (row, &e) in arch.entities.iter().enumerate() {
                self.entities.set_row(e, row);
            }
        }
    }

    // ---- resources -----------------------------------------------------------

    /// Stores a global value (game settings, a random generator, the score...),
    /// replacing any existing value of the same type.
    pub fn insert_resource<R: Send + Sync + 'static>(&mut self, value: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(value));
    }

    /// The resource of type `R`.
    ///
    /// # Panics
    /// If there is none; use [`get_resource`](Self::get_resource) to check.
    #[track_caller]
    pub fn resource<R: Send + Sync + 'static>(&self) -> &R {
        self.get_resource().unwrap_or_else(|| missing_resource::<R>())
    }

    /// Mutable version of [`resource`](Self::resource).
    #[track_caller]
    pub fn resource_mut<R: Send + Sync + 'static>(&mut self) -> &mut R {
        self.get_resource_mut().unwrap_or_else(|| missing_resource::<R>())
    }

    /// The resource of type `R`, if there is one.
    pub fn get_resource<R: Send + Sync + 'static>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .map(|r| r.downcast_ref().unwrap())
    }

    /// Mutable version of [`get_resource`](Self::get_resource).
    pub fn get_resource_mut<R: Send + Sync + 'static>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .map(|r| r.downcast_mut().unwrap())
    }

    /// Removes and returns the resource of type `R`.
    pub fn remove_resource<R: Send + Sync + 'static>(&mut self) -> Option<R> {
        self.resources
            .remove(&TypeId::of::<R>())
            .map(|r| *r.downcast().unwrap())
    }

    // ---- commands --------------------------------------------------------------

    /// Applies commands recorded while the world was busy (e.g. during a query).
    /// Leaves `commands` empty and ready to reuse.
    pub fn apply(&mut self, commands: &mut Commands) {
        commands.apply(self);
    }

    // ---- components (also the C/C++ path) ---------------------------------------

    /// The id of Rust component `T`, registering it on first use.
    pub fn register<T: Component>(&mut self) -> ComponentId {
        if let Some(&id) = self.by_type.get(&TypeId::of::<T>()) {
            return id;
        }
        let drop = std::mem::needs_drop::<T>().then_some(drop_ptr::<T> as DropFn);
        let id = self.push_info(ComponentInfo {
            name: std::any::type_name::<T>().into(),
            layout: Layout::new::<T>(),
            drop,
            type_id: Some(TypeId::of::<T>()),
        });
        self.by_type.insert(TypeId::of::<T>(), id);
        id
    }

    /// Registers a component known only by name, size and alignment: the path
    /// for components defined in C/C++ or loaded at runtime.
    ///
    /// Registering the same name again with the same layout returns the same id.
    ///
    /// # Safety
    /// Values of this component must be safe to move with `memcpy`, and to send
    /// and share between threads. `drop`, if given, must accept any value
    /// written as this component.
    ///
    /// # Panics
    /// If the name is taken with a different layout, or the layout is invalid.
    pub unsafe fn register_component(
        &mut self,
        name: &str,
        size: usize,
        align: usize,
        drop: Option<DropFn>,
    ) -> ComponentId {
        let layout = Layout::from_size_align(size, align).expect("invalid component size/alignment");
        if let Some(&id) = self.by_name.get(name) {
            let info = &self.infos[id.index()];
            assert!(
                info.layout == layout && info.type_id.is_none(),
                "component name `{name}` is already registered with a different layout"
            );
            return id;
        }
        self.push_info(ComponentInfo {
            name: name.into(),
            layout,
            drop,
            type_id: None,
        })
    }

    fn push_info(&mut self, info: ComponentInfo) -> ComponentId {
        let id = ComponentId(u32::try_from(self.infos.len()).expect("too many component types"));
        self.by_name.entry(info.name.clone()).or_insert(id);
        self.infos.push(info);
        id
    }

    /// The id of Rust component `T`, if it has been used in this world.
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.by_type.get(&TypeId::of::<T>()).copied()
    }

    /// Looks a component up by name (registered name, or Rust type name).
    pub fn component_by_name(&self, name: &str) -> Option<ComponentId> {
        self.by_name.get(name).copied()
    }

    /// What the world knows about component `id`.
    ///
    /// # Panics
    /// If `id` came from a different world.
    pub fn component_info(&self, id: ComponentId) -> &ComponentInfo {
        &self.infos[id.index()]
    }

    /// Every component type registered so far, indexed by [`ComponentId::index`].
    pub fn components(&self) -> &[ComponentInfo] {
        &self.infos
    }

    /// The archetype tables, for tools such as the inspector.
    pub fn archetypes(&self) -> &[Archetype] {
        &self.archetypes
    }

    /// Pointer to component `id` of entity `e`, or `None`.
    pub fn get_dynamic(&self, e: Entity, id: ComponentId) -> Option<*mut u8> {
        let loc = self.entities.get(e)?;
        let arch = &self.archetypes[loc.archetype as usize];
        let col = arch.column_index(id)?;
        Some(arch.columns[col].ptr(loc.row as usize))
    }

    /// Spawns an entity from raw component values.
    ///
    /// # Safety
    /// Each pointer must point to a valid value of its component; the world
    /// takes ownership (the caller must not drop it). Ids must be from this world.
    ///
    /// # Panics
    /// If a component id appears twice.
    pub unsafe fn spawn_dynamic(&mut self, components: &[(ComponentId, *const u8)]) -> Entity {
        let mut ids = std::mem::take(&mut self.scratch_ids);
        ids.clear();
        ids.extend(components.iter().map(|c| c.0));
        ids.sort_unstable();
        let unique = ids.windows(2).all(|w| w[0] != w[1]);
        let a = if unique { Some(self.archetype_for(&ids)) } else { None };
        self.scratch_ids = ids;
        let a = a.expect("spawn_dynamic: the same component was given twice");

        let e = self.entities.alloc();
        let arch = &mut self.archetypes[a];
        arch.reserve(1);
        let row = arch.len();
        for &(id, src) in components {
            let c = &arch.columns[arch.column_index(id).unwrap()];
            unsafe { std::ptr::copy_nonoverlapping(src, c.ptr(row), c.size()) };
        }
        arch.entities.push(e);
        self.entities.set(
            e,
            Location {
                archetype: a as u32,
                row: row as u32,
            },
        );
        e
    }

    /// Adds (or replaces) component `id` on `e` from a raw value.
    /// Returns `false` if `e` is dead; the value is then still the caller's.
    ///
    /// # Safety
    /// `value` must point to a valid value of component `id`; on success the
    /// world takes ownership of it.
    pub unsafe fn insert_dynamic(&mut self, e: Entity, id: ComponentId, value: *const u8) -> bool {
        let Some(loc) = self.entities.get(e) else { return false };
        let src = loc.archetype as usize;
        if let Some(col) = self.archetypes[src].column_index(id) {
            let c = &self.archetypes[src].columns[col];
            let dst = c.ptr(loc.row as usize);
            unsafe {
                if let Some(d) = c.drop_fn() {
                    d(dst);
                }
                std::ptr::copy_nonoverlapping(value, dst, c.size());
            }
            return true;
        }
        let dst = match self.archetypes[src].add_edges.get(&id) {
            Some(&d) => d,
            None => {
                let mut ids = self.archetypes[src].ids.to_vec();
                ids.push(id);
                ids.sort_unstable();
                let d = self.archetype_for(&ids);
                self.archetypes[src].add_edges.insert(id, d);
                self.archetypes[d].remove_edges.insert(id, src);
                d
            }
        };
        let row = self.move_row(e, loc, dst);
        let arch = &self.archetypes[dst];
        let c = &arch.columns[arch.column_index(id).unwrap()];
        unsafe { std::ptr::copy_nonoverlapping(value, c.ptr(row), c.size()) };
        true
    }

    /// Removes component `id` from `e`, dropping the value. Returns `false` if
    /// `e` is dead or didn't have it.
    pub fn remove_dynamic(&mut self, e: Entity, id: ComponentId) -> bool {
        let Some(loc) = self.entities.get(e) else { return false };
        let arch = &self.archetypes[loc.archetype as usize];
        let Some(col) = arch.column_index(id) else { return false };
        let c = &arch.columns[col];
        let Some(drop) = c.drop_fn() else {
            self.move_without(e, loc, id);
            return true;
        };
        // Move the value out first, then drop it: if drop panics, the world is consistent.
        let ptr = c.ptr(loc.row as usize);
        let mut tmp = TempValue::new(self.infos[id.index()].layout);
        unsafe { std::ptr::copy_nonoverlapping(ptr, tmp.ptr(), c.size()) };
        self.move_without(e, loc, id);
        unsafe { drop(tmp.ptr()) };
        true
    }

    /// Calls `f` once per archetype that has all of `ids`, with raw column
    /// pointers in `ids` order. This is how C/C++ systems see the world: whole
    /// columns, never one call per entity.
    pub fn for_each_chunk(&mut self, ids: &[ComponentId], mut f: impl FnMut(Chunk<'_>)) {
        const MAX: usize = 32;
        assert!(ids.len() <= MAX, "for_each_chunk: at most {MAX} components");
        let mut ptrs = [std::ptr::null_mut(); MAX];
        'arch: for arch in self.archetypes.iter() {
            if arch.is_empty() {
                continue;
            }
            for (i, &id) in ids.iter().enumerate() {
                match arch.column_ptr(id) {
                    Some(p) => ptrs[i] = p,
                    None => continue 'arch,
                }
            }
            f(Chunk {
                entities: &arch.entities,
                columns: &ptrs[..ids.len()],
            });
        }
    }

    // ---- internals -------------------------------------------------------------

    /// `ids` must be sorted and unique.
    fn archetype_for(&mut self, ids: &[ComponentId]) -> usize {
        if let Some(&a) = self.archetype_by_ids.get(ids) {
            return a;
        }
        let a = self.archetypes.len();
        self.archetypes.push(Archetype::new(ids.into(), &self.infos));
        self.archetype_by_ids.insert(ids.into(), a);
        a
    }

    fn bundle_archetype<B: Bundle>(&mut self) -> usize {
        if let Some(b) = self.bundles.get(&TypeId::of::<B>()) {
            return b.archetype;
        }
        let mut ids = Vec::new();
        B::component_ids(self, &mut ids);
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            ids.len(),
            "bundle `{}` has the same component twice",
            std::any::type_name::<B>()
        );
        let a = self.archetype_for(&sorted);
        let columns = ids
            .iter()
            .map(|&id| self.archetypes[a].column_index(id).unwrap())
            .collect();
        self.bundles
            .insert(TypeId::of::<B>(), BundleInfo { archetype: a, columns });
        a
    }

    /// Moves `e` to the archetype without component `id`. The value of `id`
    /// is forgotten: the caller must have read or copied it out.
    fn move_without(&mut self, e: Entity, loc: Location, id: ComponentId) {
        let src = loc.archetype as usize;
        let dst = match self.archetypes[src].remove_edges.get(&id) {
            Some(&d) => d,
            None => {
                let ids: Vec<_> = self.archetypes[src].ids.iter().copied().filter(|&c| c != id).collect();
                let d = self.archetype_for(&ids);
                self.archetypes[src].remove_edges.insert(id, d);
                self.archetypes[d].add_edges.insert(id, src);
                d
            }
        };
        self.move_row(e, loc, dst);
    }

    /// Moves `e`'s row to archetype `dst`, copying shared columns and
    /// forgetting the rest. Returns the new row; columns of `dst` that the
    /// source lacks are left uninitialized for the caller to fill.
    fn move_row(&mut self, e: Entity, loc: Location, dst: usize) -> usize {
        let src = loc.archetype as usize;
        let row = loc.row as usize;
        let (s, d) = two_mut(&mut self.archetypes, src, dst);
        d.reserve(1);
        let new_row = d.len();
        let last = s.len() - 1;
        for (i, col) in s.columns.iter_mut().enumerate() {
            if let Some(j) = d.column_index(s.ids[i]) {
                unsafe { std::ptr::copy_nonoverlapping(col.ptr(row), d.columns[j].ptr(new_row), col.size()) };
            }
            unsafe { col.swap_remove_forget(row, last) };
        }
        s.entities.swap_remove(row);
        if row != last {
            let moved = s.entities[row];
            self.entities.set_row(moved, row);
        }
        d.entities.push(e);
        self.entities.set(
            e,
            Location {
                archetype: dst as u32,
                row: new_row as u32,
            },
        );
        new_row
    }
}

fn sort_keys(keyed: &mut [(u64, u32)]) {
    #[cfg(feature = "parallel")]
    if keyed.len() > 50_000 {
        use rayon::slice::ParallelSliceMut;
        keyed.par_sort_unstable_by_key(|k| k.0);
        return;
    }
    keyed.sort_unstable_by_key(|k| k.0);
}

#[cold]
#[track_caller]
fn missing_resource<R>() -> ! {
    panic!(
        "no resource of type `{0}`; add one first with `world.insert_resource(...)`",
        std::any::type_name::<R>()
    )
}

fn two_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (l, r) = v.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = v.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

/// A heap slot for one value of a runtime layout (freed without dropping).
struct TempValue {
    ptr: *mut u8,
    layout: Layout,
}

impl TempValue {
    fn new(layout: Layout) -> Self {
        let ptr = if layout.size() == 0 {
            std::ptr::without_provenance_mut(layout.align())
        } else {
            let p = unsafe { std::alloc::alloc(layout) };
            if p.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            p
        };
        TempValue { ptr, layout }
    }

    fn ptr(&mut self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for TempValue {
    fn drop(&mut self) {
        if self.layout.size() != 0 {
            unsafe { std::alloc::dealloc(self.ptr, self.layout) }
        }
    }
}
