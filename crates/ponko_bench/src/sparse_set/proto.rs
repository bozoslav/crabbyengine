//! A small sparse-set store next to a `ponko_ecs::World`, for the `sparse-set/`
//! benchmarks only (issue #166). It is not part of the engine: it shows what
//! opt-in sparse storage would cost and save, measured through the same
//! public API a game uses.
//!
//! The layout is the classic one: per component, a `sparse` array indexed by
//! the entity's slot index holding a position in two packed arrays, `entities`
//! and `values`. Adding and removing a component never moves the entity's
//! other data (no archetype change); a query over the component walks the
//! packed arrays; a query that mixes it with archetype components joins the
//! two, driven from whichever side is smaller.
//!
//! It is safe Rust except [`dense_ref`] and [`dense_mut`], which cast the
//! raw pointers `RawWorldRef::get_dynamic` and `RawWorld::get_dynamic_mut` return to the component's type; they
//! are `unsafe fn`s because a [`DenseId`] from another world could name a
//! different type there. What a real integration would add (despawn hooks,
//! change stamps, commands, views, the scheduler's access sets, the C ABI,
//! saved worlds) is listed in `SPARSE_SET.md`, not built here.

use ponko_ecs::{Component, ComponentId, Entity, QueryData, World};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

/// The id of archetype component `D` in one world, looked up once: what an
/// integrated join would cache instead of hashing `D`'s type for every
/// element. It doesn't record which world it came from; component ids are
/// per world, so in another world it may name a different type.
pub struct DenseId<D>(ComponentId, PhantomData<fn() -> D>);

impl<D> Clone for DenseId<D> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<D> Copy for DenseId<D> {}

impl<D: Component> DenseId<D> {
    /// `D`'s id in `world`, if `D` is registered there.
    pub fn of(world: &World) -> Option<Self> {
        Some(DenseId(world.component_id::<D>()?, PhantomData))
    }
}

/// `e`'s archetype component `D`.
///
/// # Safety
/// `id` came from [`DenseId::of`] on this same `world`.
#[inline]
pub unsafe fn dense_ref<D: Component>(world: &World, id: DenseId<D>, e: Entity) -> Option<&D> {
    let p = world.raw_ref().get_dynamic(e, id.0)?;
    // SAFETY: by the caller's promise `id` is `D`'s id in this world, so `p`
    // points to `e`'s live `D`, which stays put while `world` is borrowed.
    Some(unsafe { &*p.cast::<D>() })
}

/// `e`'s archetype component `D`, mutably.
///
/// # Safety
/// `id` came from [`DenseId::of`] on this same `world`.
#[inline]
pub unsafe fn dense_mut<D: Component>(world: &mut World, id: DenseId<D>, e: Entity) -> Option<&mut D> {
    let p = world.raw().get_dynamic_mut(e, id.0)?;
    // SAFETY: as in `dense_ref`; `world` is borrowed mutably, so nothing
    // else refers to the value.
    Some(unsafe { &mut *p.cast::<D>() })
}

/// No packed slot for this entity index.
const EMPTY: u32 = u32::MAX;
/// The sparse array grows in steps of this many entity slots, with no
/// doubling slack: its size is then what the entity table dictates.
const SPARSE_STEP: usize = 4096;

/// One component's sparse set.
pub struct SparseSet<T> {
    /// Entity slot index -> position in `entities`/`values`, or [`EMPTY`].
    sparse: Vec<u32>,
    /// The entities that have the component, packed; `entities[i]` owns `values[i]`.
    entities: Vec<Entity>,
    values: Vec<T>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        SparseSet {
            sparse: Vec::new(),
            entities: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T> SparseSet<T> {
    /// The packed position of `e`, if it has the component. The generation
    /// is checked through the packed entity, so a recycled slot doesn't match.
    #[inline]
    fn slot(&self, e: Entity) -> Option<usize> {
        let i = *self.sparse.get(e.index() as usize)? as usize;
        (self.entities.get(i) == Some(&e)).then_some(i)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn contains(&self, e: Entity) -> bool {
        self.slot(e).is_some()
    }

    #[inline]
    pub fn get(&self, e: Entity) -> Option<&T> {
        self.slot(e).map(|i| &self.values[i])
    }

    #[cfg(test)]
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        self.slot(e).map(|i| &mut self.values[i])
    }

    /// Adds or replaces `e`'s value. Returns `true` if `e` didn't have one.
    pub fn insert(&mut self, e: Entity, value: T) -> bool {
        if let Some(i) = self.slot(e) {
            self.values[i] = value;
            return false;
        }
        let index = e.index() as usize;
        if index >= self.sparse.len() {
            let len = (index + 1).next_multiple_of(SPARSE_STEP);
            self.sparse.reserve_exact(len - self.sparse.len());
            self.sparse.resize(len, EMPTY);
        }
        self.sparse[index] = u32::try_from(self.entities.len()).expect("too many entities");
        self.entities.push(e);
        self.values.push(value);
        true
    }

    /// Takes `e`'s value out: the last packed entry moves into its place.
    pub fn remove(&mut self, e: Entity) -> Option<T> {
        let i = self.slot(e)?;
        self.sparse[e.index() as usize] = EMPTY;
        self.entities.swap_remove(i);
        let value = self.values.swap_remove(i);
        if let Some(&moved) = self.entities.get(i) {
            self.sparse[moved.index() as usize] = i as u32;
        }
        Some(value)
    }

    /// The packed entities and their values, in packed order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.entities.iter().copied().zip(self.values.iter())
    }

    /// The packed entities.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Bytes allocated (capacity, not length), like `World::allocated_bytes`.
    pub fn allocated_bytes(&self) -> usize {
        self.sparse.capacity() * size_of::<u32>()
            + self.entities.capacity() * size_of::<Entity>()
            + self.values.capacity() * size_of::<T>()
    }
}

/// What the store needs from a set without knowing its type.
trait ErasedSet: Any + Send + Sync {
    fn remove_entity(&mut self, e: Entity);
    fn bytes(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> ErasedSet for SparseSet<T> {
    fn remove_entity(&mut self, e: Entity) {
        self.remove(e);
    }
    fn bytes(&self) -> usize {
        self.allocated_bytes()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A world whose archetypes hold the dense components, with the sparse ones
/// in one sparse set per type beside it.
#[derive(Default)]
pub struct Hybrid {
    pub world: World,
    sets: HashMap<TypeId, Box<dyn ErasedSet>>,
}

impl Hybrid {
    /// `T`'s set, created empty on first use.
    pub fn set_mut<T: Component>(&mut self) -> &mut SparseSet<T> {
        self.sets
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SparseSet::<T>::default()))
            .as_any_mut()
            .downcast_mut()
            .expect("keyed by type")
    }

    /// `T`'s set, if anything ever stored a `T`.
    pub fn set<T: Component>(&self) -> Option<&SparseSet<T>> {
        self.sets.get(&TypeId::of::<T>())?.as_any().downcast_ref()
    }

    /// Adds sparse component `value` to live entity `e`.
    pub fn insert<T: Component>(&mut self, e: Entity, value: T) -> bool {
        if !self.world.is_alive(e) {
            return false;
        }
        self.set_mut::<T>().insert(e, value);
        true
    }

    pub fn remove<T: Component>(&mut self, e: Entity) -> Option<T> {
        self.set_mut::<T>().remove(e)
    }

    #[cfg(test)]
    pub fn has<T: Component>(&self, e: Entity) -> bool {
        self.set::<T>().is_some_and(|s| s.contains(e))
    }

    /// Despawns `e` and drops its sparse components: every set is visited,
    /// which is the cost a despawn hook would have in an integrated version.
    #[cfg_attr(not(test), allow(dead_code))] // no despawns in the workload
    pub fn despawn(&mut self, e: Entity) -> bool {
        for set in self.sets.values_mut() {
            set.remove_entity(e);
        }
        self.world.despawn(e)
    }

    /// Bytes of the world's columns plus every set.
    pub fn allocated_bytes(&self) -> usize {
        self.world.allocated_bytes() + self.sets.values().map(|s| s.bytes()).sum::<usize>()
    }

    /// Calls `f` for every entity with sparse `T` and archetype data `D`,
    /// driven from `T`'s packed array: one entity lookup per element.
    pub fn join_from_set<D: Component, T: Component>(&mut self, mut f: impl FnMut(&mut D, &T)) {
        let (Some(set), Some(id)) = (self.sets.get(&TypeId::of::<T>()), DenseId::<D>::of(&self.world)) else {
            return;
        };
        let set: &SparseSet<T> = set.as_any().downcast_ref().expect("keyed by type");
        for (e, value) in set.iter() {
            // SAFETY: `id` was taken from `self.world` above.
            if let Some(d) = unsafe { dense_mut(&mut self.world, id, e) } {
                f(d, value);
            }
        }
    }

    /// The same join driven from the archetype query `Q` (which must fetch the
    /// entity first): one sparse lookup per row of `Q`.
    pub fn join_from_rows<Q: QueryData, T: Component>(&mut self, mut f: impl FnMut(Q::Item<'_>, &T)) {
        let Some(set) = self.sets.get(&TypeId::of::<T>()) else {
            return;
        };
        let set: &SparseSet<T> = set.as_any().downcast_ref().expect("keyed by type");
        self.world.query::<(Entity, Q)>().for_each(|(e, q)| {
            if let Some(value) = set.get(e) {
                f(q, value);
            }
        });
    }

    /// Rows of archetype query `Q` whose entity has none of `T`.
    pub fn rows_without<Q: QueryData, T: Component>(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        let set = self
            .sets
            .get(&TypeId::of::<T>())
            .map(|s| s.as_any().downcast_ref::<SparseSet<T>>().expect("keyed by type"));
        self.world.query::<(Entity, Q)>().for_each(|(e, q)| {
            if !set.is_some_and(|s| s.contains(e)) {
                f(q);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, ponko_ecs::Component)]
    struct Hp(u32);
    #[derive(ponko_ecs::Component)]
    struct Pos(f32);

    #[test]
    fn insert_remove_keeps_the_packed_arrays_consistent() {
        let mut world = World::new();
        let es: Vec<Entity> = (0..10).map(|i| world.spawn((Pos(i as f32),))).collect();
        let mut set = SparseSet::default();
        for (i, &e) in es.iter().enumerate() {
            assert!(set.insert(e, Hp(i as u32)));
        }
        assert!(!set.insert(es[3], Hp(33)), "a second insert replaces");
        assert_eq!(set.get(es[3]), Some(&Hp(33)));
        // Removing from the middle moves the last entry into the hole.
        assert_eq!(set.remove(es[2]), Some(Hp(2)));
        assert_eq!(set.remove(es[2]), None);
        assert_eq!(set.len(), 9);
        assert_eq!(set.get(es[9]), Some(&Hp(9)));
        set.get_mut(es[9]).unwrap().0 = 99;
        for (i, &e) in es.iter().enumerate() {
            let expected = match i {
                2 => None,
                3 => Some(33),
                9 => Some(99),
                _ => Some(i as u32),
            };
            assert_eq!(set.get(e).map(|h| h.0), expected);
        }
        for (e, v) in set.iter() {
            assert_eq!(set.get(e), Some(v));
        }
    }

    #[test]
    fn a_recycled_entity_slot_does_not_inherit_sparse_components() {
        let mut h = Hybrid::default();
        let old = h.world.spawn((Pos(0.0),));
        h.insert(old, Hp(1));
        assert!(h.despawn(old));
        let new = h.world.spawn((Pos(1.0),));
        assert_eq!(new.index(), old.index(), "the slot is reused");
        assert!(!h.has::<Hp>(new));
        assert!(!h.insert(old, Hp(2)), "dead entities get nothing");
        assert_eq!(h.set::<Hp>().unwrap().len(), 0);
    }

    #[test]
    fn both_join_drivers_visit_the_same_entities() {
        let mut h = Hybrid::default();
        let es: Vec<Entity> = (0..100).map(|i| h.world.spawn((Pos(i as f32),))).collect();
        for &e in es.iter().step_by(3) {
            h.insert(e, Hp(e.index()));
        }
        let mut from_set = Vec::new();
        h.join_from_set::<Pos, Hp>(|p, hp| from_set.push((p.0 as u32, hp.0)));
        let mut from_rows = Vec::new();
        h.join_from_rows::<&Pos, Hp>(|p, hp| from_rows.push((p.0 as u32, hp.0)));
        from_set.sort_unstable();
        from_rows.sort_unstable();
        assert_eq!(from_set.len(), 34);
        assert_eq!(from_set, from_rows);
        let mut without = 0;
        h.rows_without::<&Pos, Hp>(|_| without += 1);
        assert_eq!(without, 66);
        // The cached-id lookups the set-driven joins use.
        let id = DenseId::<Pos>::of(&h.world).unwrap();
        // SAFETY (all three calls): `id` comes from `h.world`.
        unsafe { dense_mut(&mut h.world, id, es[7]) }.unwrap().0 = 70.0;
        assert_eq!(unsafe { dense_ref(&h.world, id, es[7]) }.unwrap().0, 70.0);
        assert!(
            DenseId::<Hp>::of(&h.world).is_none(),
            "sparse types aren't in the archetypes"
        );
        h.despawn(es[7]);
        assert!(unsafe { dense_ref(&h.world, id, es[7]) }.is_none());
    }
}
