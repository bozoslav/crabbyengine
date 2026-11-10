//! Explicit access to the dynamic ECS interface used by native bindings.
//!
//! The typed API (`spawn`, `query`, `get`) is the way to use a [`World`] from
//! Rust. What C, C++ and tools need on top of it (components by id and
//! pointer, resources by name and layout, whole archetype chunks) goes
//! through [`World::raw`] and [`World::raw_ref`]. The same methods on `World`
//! itself still work, deprecated, and forward here.
use crate::{Archetype, Chunk, ComponentId, Entity, World};
use std::alloc::Layout;
use std::ops::{Deref, DerefMut};

/// Exclusive view of a world's dynamic component, resource, chunk and archetype APIs.
///
/// This borrows the same world; it does not copy storage or change the safety
/// requirements of dynamic methods. In particular, raw pointers remain subject
/// to the documented layout, lifetime and structural-mutation requirements.
pub struct RawWorld<'a>(&'a mut World);
impl Deref for RawWorld<'_> {
    type Target = World;
    fn deref(&self) -> &World {
        self.0
    }
}
impl DerefMut for RawWorld<'_> {
    fn deref_mut(&mut self) -> &mut World {
        self.0
    }
}

/// Shared dynamic view, for inspecting component metadata and raw pointers.
#[derive(Clone, Copy)]
pub struct RawWorldRef<'a>(&'a World);
impl Deref for RawWorldRef<'_> {
    type Target = World;
    fn deref(&self) -> &World {
        self.0
    }
}
impl World {
    /// Borrows the dynamic/native interface without changing component identity.
    #[inline]
    pub fn raw(&mut self) -> RawWorld<'_> {
        RawWorld(self)
    }
    /// Borrows read-only dynamic metadata and pointer access.
    #[inline]
    pub fn raw_ref(&self) -> RawWorldRef<'_> {
        RawWorldRef(self)
    }
}

impl<'a> RawWorld<'a> {
    /// Pointer to component `id` of entity `e`, or `None`.
    #[must_use]
    #[inline]
    pub fn get_dynamic(&self, e: Entity, id: ComponentId) -> Option<*mut u8> {
        self.0.get_dynamic_inner(e, id)
    }

    /// Removes component `id` from `e`, dropping the value. Returns `false` if
    /// `e` is dead or didn't have it.
    #[inline]
    pub fn remove_dynamic(&mut self, e: Entity, id: ComponentId) -> bool {
        self.0.remove_dynamic_inner(e, id)
    }

    /// Adds (or replaces) component `id` on `e` from a raw value.
    /// Returns `false` if `e` is dead; the value is then still the caller's.
    ///
    /// # Safety
    /// `value` must point to a valid value of component `id`; on success the
    /// world takes ownership of it.
    #[inline]
    pub unsafe fn insert_dynamic(&mut self, e: Entity, id: ComponentId, value: *const u8) -> bool {
        // SAFETY: forwarded from the caller.
        unsafe { self.0.insert_dynamic_inner(e, id, value) }
    }

    /// Spawns an entity from raw component values.
    ///
    /// # Safety
    /// Each pointer must point to a valid value of its component; the world
    /// takes ownership (the caller must not drop it). Ids must be from this world.
    ///
    /// # Panics
    /// If a component id appears twice.
    #[inline]
    pub unsafe fn spawn_dynamic(&mut self, components: &[(ComponentId, *const u8)]) -> Entity {
        // SAFETY: forwarded from the caller.
        unsafe { self.0.spawn_dynamic_inner(components) }
    }

    /// [`spawn_dynamic`](Self::spawn_dynamic) with a given id, like
    /// [`spawn_at`](crate::World::spawn_at): returns `false` (and takes nothing) if
    /// that slot is taken.
    ///
    /// ```
    /// use ponko_ecs::{Entity, World};
    ///
    /// let mut world = World::new();
    /// // SAFETY: f32 is plain bytes.
    /// let id = unsafe { world.register_component("Speed", 4, 4, None) };
    /// let saved = Entity::from_bits(5 | 2 << 32);
    /// let speed = 1.5f32;
    /// // SAFETY: points to a valid f32, which needs no drop.
    /// assert!(unsafe { world.raw().spawn_at_dynamic(saved, &[(id, (&speed as *const f32).cast())]) });
    /// assert!(!unsafe { world.raw().spawn_at_dynamic(saved, &[(id, (&speed as *const f32).cast())]) });
    /// assert_eq!(world.len(), 1);
    /// ```
    ///
    /// # Safety
    /// As for [`spawn_dynamic`](Self::spawn_dynamic). When this returns
    /// `false`, the values stay the caller's.
    ///
    /// # Panics
    /// If a component id appears twice.
    #[inline]
    pub unsafe fn spawn_at_dynamic(&mut self, e: Entity, components: &[(ComponentId, *const u8)]) -> bool {
        // SAFETY: forwarded from the caller.
        unsafe { self.0.spawn_at_dynamic_inner(e, components) }
    }

    /// Pointer to resource `name` (one stored with
    /// [`insert_resource_dynamic`](Self::insert_resource_dynamic)), or `None`.
    /// It stays valid until the resource is replaced or removed.
    #[must_use]
    #[inline]
    pub fn get_resource_dynamic(&self, name: &str) -> Option<*mut u8> {
        self.0.get_resource_dynamic_inner(name)
    }

    /// Stores a resource that has no Rust type, by name: a copy of
    /// `layout.size()` bytes from `value`, replacing any resource of that
    /// name. This is how C and C++ games keep global values.
    ///
    /// ```
    /// use ponko_ecs::*;
    /// use std::alloc::Layout;
    ///
    /// let mut world = World::new();
    /// let score: [u32; 2] = [3, 1];
    /// // SAFETY: `score` is 8 readable bytes.
    /// unsafe { world.raw().insert_resource_dynamic("Score", Layout::new::<[u32; 2]>(), score.as_ptr().cast()) };
    /// let p = world.raw_ref().get_resource_dynamic("Score").unwrap() as *const [u32; 2];
    /// // SAFETY: the resource holds the bytes of a [u32; 2].
    /// assert_eq!(unsafe { *p }, [3, 1]);
    /// ```
    ///
    /// # Safety
    /// `value` points to `layout.size()` readable bytes (any pointer for size 0).
    ///
    /// # Panics
    /// If a resource of that name exists with a different layout.
    #[track_caller]
    #[inline]
    pub unsafe fn insert_resource_dynamic(&mut self, name: &str, layout: Layout, value: *const u8) {
        // SAFETY: forwarded from the caller.
        unsafe { self.0.insert_resource_dynamic_inner(name, layout, value) }
    }

    /// The size and alignment of resource `name`, if it exists.
    #[must_use]
    #[inline]
    pub fn resource_layout_dynamic(&self, name: &str) -> Option<Layout> {
        self.0.resource_layout_dynamic_inner(name)
    }

    /// Removes resource `name`; `false` if there was none.
    #[inline]
    pub fn remove_resource_dynamic(&mut self, name: &str) -> bool {
        self.0.remove_resource_dynamic_inner(name)
    }

    /// Calls `f` once per archetype that has all of `ids`, with raw column
    /// pointers in `ids` order. This is how C/C++ systems see the world: whole
    /// columns, never one call per entity.
    ///
    /// # Panics
    ///
    /// With more than 32 component ids.
    #[inline]
    pub fn for_each_chunk(&mut self, ids: &[ComponentId], f: impl FnMut(Chunk<'_>)) {
        self.0.for_each_chunk_inner(ids, f)
    }

    /// Looks a component up by name (registered name, or Rust type name).
    #[must_use]
    #[inline]
    pub fn component_by_name(&self, name: &str) -> Option<ComponentId> {
        self.0.component_by_name_inner(name)
    }

    /// The archetype tables, for tools such as the inspector.
    ///
    /// The slice borrows this view, so it can't outlive a temporary
    /// `world.raw()`. To keep it, take it from the shared view instead:
    /// `let tables = world.raw_ref().archetypes();` borrows the world itself.
    #[must_use]
    #[inline]
    pub fn archetypes(&self) -> &[Archetype] {
        self.0.archetypes_inner()
    }
}

impl<'a> RawWorldRef<'a> {
    /// Pointer to component `id` of entity `e`, or `None`.
    #[must_use]
    #[inline]
    pub fn get_dynamic(&self, e: Entity, id: ComponentId) -> Option<*mut u8> {
        self.0.get_dynamic_inner(e, id)
    }

    /// Pointer to resource `name` (one stored with
    /// [`insert_resource_dynamic`](RawWorld::insert_resource_dynamic)), or `None`.
    /// It stays valid until the resource is replaced or removed.
    #[must_use]
    #[inline]
    pub fn get_resource_dynamic(&self, name: &str) -> Option<*mut u8> {
        self.0.get_resource_dynamic_inner(name)
    }

    /// The size and alignment of resource `name`, if it exists.
    #[must_use]
    #[inline]
    pub fn resource_layout_dynamic(&self, name: &str) -> Option<Layout> {
        self.0.resource_layout_dynamic_inner(name)
    }

    /// Looks a component up by name (registered name, or Rust type name).
    #[must_use]
    #[inline]
    pub fn component_by_name(&self, name: &str) -> Option<ComponentId> {
        self.0.component_by_name_inner(name)
    }

    /// The archetype tables, for tools such as the inspector.
    #[must_use]
    #[inline]
    pub fn archetypes(&self) -> &'a [Archetype] {
        self.0.archetypes_inner()
    }
}

/// The dynamic methods' old spellings on `World`, kept until the next major release.
impl World {
    #[doc(hidden)]
    #[deprecated(note = "use `world.raw_ref().get_dynamic(..)`")]
    #[must_use]
    #[inline]
    pub fn get_dynamic(&self, e: Entity, id: ComponentId) -> Option<*mut u8> {
        self.get_dynamic_inner(e, id)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().remove_dynamic(..)`")]
    #[inline]
    pub fn remove_dynamic(&mut self, e: Entity, id: ComponentId) -> bool {
        self.remove_dynamic_inner(e, id)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().insert_dynamic(..)`")]
    #[inline]
    pub unsafe fn insert_dynamic(&mut self, e: Entity, id: ComponentId, value: *const u8) -> bool {
        // SAFETY: forwarded from the caller.
        unsafe { self.insert_dynamic_inner(e, id, value) }
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().spawn_dynamic(..)`")]
    #[inline]
    pub unsafe fn spawn_dynamic(&mut self, components: &[(ComponentId, *const u8)]) -> Entity {
        // SAFETY: forwarded from the caller.
        unsafe { self.spawn_dynamic_inner(components) }
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().spawn_at_dynamic(..)`")]
    #[inline]
    pub unsafe fn spawn_at_dynamic(&mut self, e: Entity, components: &[(ComponentId, *const u8)]) -> bool {
        // SAFETY: forwarded from the caller.
        unsafe { self.spawn_at_dynamic_inner(e, components) }
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw_ref().get_resource_dynamic(..)`")]
    #[must_use]
    #[inline]
    pub fn get_resource_dynamic(&self, name: &str) -> Option<*mut u8> {
        self.get_resource_dynamic_inner(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().insert_resource_dynamic(..)`")]
    #[track_caller]
    #[inline]
    pub unsafe fn insert_resource_dynamic(&mut self, name: &str, layout: Layout, value: *const u8) {
        // SAFETY: forwarded from the caller.
        unsafe { self.insert_resource_dynamic_inner(name, layout, value) }
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw_ref().resource_layout_dynamic(..)`")]
    #[must_use]
    #[inline]
    pub fn resource_layout_dynamic(&self, name: &str) -> Option<Layout> {
        self.resource_layout_dynamic_inner(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().remove_resource_dynamic(..)`")]
    #[inline]
    pub fn remove_resource_dynamic(&mut self, name: &str) -> bool {
        self.remove_resource_dynamic_inner(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw().for_each_chunk(..)`")]
    #[inline]
    pub fn for_each_chunk(&mut self, ids: &[ComponentId], f: impl FnMut(Chunk<'_>)) {
        self.for_each_chunk_inner(ids, f)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw_ref().component_by_name(..)`")]
    #[must_use]
    #[inline]
    pub fn component_by_name(&self, name: &str) -> Option<ComponentId> {
        self.component_by_name_inner(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `world.raw_ref().archetypes(..)`")]
    #[must_use]
    #[inline]
    pub fn archetypes(&self) -> &[Archetype] {
        self.archetypes_inner()
    }
}
