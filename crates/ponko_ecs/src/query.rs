//! Queries: loop over every entity that has a set of components.

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::bundle::Component;
use crate::change::{self, Clock, RowFilter, Tick};
use crate::entity::Entity;
use crate::storage::{Archetype, ComponentId};
use crate::world::World;

/// What a query fetches per entity: `&T`, `&mut T`, `Option<&T>`,
/// `Option<&mut T>`, [`Entity`], or a tuple of those (up to 12; nest tuples
/// for more).
///
/// # Safety
/// `state`/`fetch` may only create references into columns listed by `access`,
/// with the mutability listed there.
pub unsafe trait QueryData {
    /// What the loop body receives.
    type Item<'a>;
    #[doc(hidden)]
    type Ids: Copy;
    #[doc(hidden)]
    type State: Copy;
    #[doc(hidden)]
    fn ids(world: &mut World) -> Self::Ids;
    /// The ids without registering anything: `None` if a component isn't
    /// registered yet, or if this query data doesn't support shared lookup
    /// (what [`WorldView`](crate::WorldView) queries need).
    #[doc(hidden)]
    fn ids_in(world: &World) -> Option<Self::Ids> {
        let _ = world;
        None
    }
    /// Reports `(component, mutable, required)` for each component touched.
    #[doc(hidden)]
    fn access(ids: &Self::Ids, f: &mut impl FnMut(ComponentId, bool, bool));
    #[doc(hidden)]
    fn state(arch: &Archetype, ids: &Self::Ids) -> Option<Self::State>;
    #[doc(hidden)]
    unsafe fn fetch<'a>(state: Self::State, row: usize) -> Self::Item<'a>;
}

unsafe impl<T: Component> QueryData for &T {
    type Item<'a> = &'a T;
    type Ids = ComponentId;
    type State = *const T;
    fn ids(world: &mut World) -> ComponentId {
        world.register::<T>()
    }
    fn ids_in(world: &World) -> Option<ComponentId> {
        world.component_id::<T>()
    }
    fn access(id: &ComponentId, f: &mut impl FnMut(ComponentId, bool, bool)) {
        f(*id, false, true)
    }
    #[inline]
    fn state(arch: &Archetype, id: &ComponentId) -> Option<*const T> {
        arch.column_ptr(*id).map(|p| p as *const T)
    }
    #[inline(always)]
    unsafe fn fetch<'a>(state: *const T, row: usize) -> &'a T {
        unsafe { &*state.add(row) }
    }
}

unsafe impl<T: Component> QueryData for &mut T {
    type Item<'a> = &'a mut T;
    type Ids = ComponentId;
    type State = *mut T;
    fn ids(world: &mut World) -> ComponentId {
        world.register::<T>()
    }
    fn ids_in(world: &World) -> Option<ComponentId> {
        world.component_id::<T>()
    }
    fn access(id: &ComponentId, f: &mut impl FnMut(ComponentId, bool, bool)) {
        f(*id, true, true)
    }
    #[inline]
    fn state(arch: &Archetype, id: &ComponentId) -> Option<*mut T> {
        arch.column_ptr(*id).map(|p| p as *mut T)
    }
    #[inline(always)]
    unsafe fn fetch<'a>(state: *mut T, row: usize) -> &'a mut T {
        unsafe { &mut *state.add(row) }
    }
}

unsafe impl<T: Component> QueryData for Option<&T> {
    type Item<'a> = Option<&'a T>;
    type Ids = ComponentId;
    type State = Option<*const T>;
    fn ids(world: &mut World) -> ComponentId {
        world.register::<T>()
    }
    fn ids_in(world: &World) -> Option<ComponentId> {
        world.component_id::<T>()
    }
    fn access(id: &ComponentId, f: &mut impl FnMut(ComponentId, bool, bool)) {
        f(*id, false, false)
    }
    #[inline]
    fn state(arch: &Archetype, id: &ComponentId) -> Option<Option<*const T>> {
        Some(arch.column_ptr(*id).map(|p| p as *const T))
    }
    #[inline(always)]
    unsafe fn fetch<'a>(state: Option<*const T>, row: usize) -> Option<&'a T> {
        state.map(|p| unsafe { &*p.add(row) })
    }
}

unsafe impl<T: Component> QueryData for Option<&mut T> {
    type Item<'a> = Option<&'a mut T>;
    type Ids = ComponentId;
    type State = Option<*mut T>;
    fn ids(world: &mut World) -> ComponentId {
        world.register::<T>()
    }
    fn ids_in(world: &World) -> Option<ComponentId> {
        world.component_id::<T>()
    }
    fn access(id: &ComponentId, f: &mut impl FnMut(ComponentId, bool, bool)) {
        f(*id, true, false)
    }
    #[inline]
    fn state(arch: &Archetype, id: &ComponentId) -> Option<Option<*mut T>> {
        Some(arch.column_ptr(*id).map(|p| p as *mut T))
    }
    #[inline(always)]
    unsafe fn fetch<'a>(state: Option<*mut T>, row: usize) -> Option<&'a mut T> {
        state.map(|p| unsafe { &mut *p.add(row) })
    }
}

unsafe impl QueryData for Entity {
    type Item<'a> = Entity;
    type Ids = ();
    type State = *const Entity;
    fn ids(_: &mut World) {}
    fn ids_in(_: &World) -> Option<()> {
        Some(())
    }
    fn access(_: &(), _: &mut impl FnMut(ComponentId, bool, bool)) {}
    #[inline]
    fn state(arch: &Archetype, _: &()) -> Option<*const Entity> {
        Some(arch.entities.as_ptr())
    }
    #[inline(always)]
    unsafe fn fetch<'a>(state: *const Entity, row: usize) -> Self::Item<'a> {
        unsafe { *state.add(row) }
    }
}

macro_rules! query_tuple {
    ($($t:ident $i:tt),*) => {
        unsafe impl<$($t: QueryData),*> QueryData for ($($t,)*) {
            type Item<'a> = ($($t::Item<'a>,)*);
            type Ids = ($($t::Ids,)*);
            type State = ($($t::State,)*);
            fn ids(world: &mut World) -> Self::Ids {
                ($($t::ids(world),)*)
            }
            fn ids_in(world: &World) -> Option<Self::Ids> {
                Some(($($t::ids_in(world)?,)*))
            }
            fn access(ids: &Self::Ids, f: &mut impl FnMut(ComponentId, bool, bool)) {
                $($t::access(&ids.$i, f);)*
            }
            #[inline]
            fn state(arch: &Archetype, ids: &Self::Ids) -> Option<Self::State> {
                Some(($($t::state(arch, &ids.$i)?,)*))
            }
            #[inline(always)]
            unsafe fn fetch<'a>(state: Self::State, row: usize) -> Self::Item<'a> {
                unsafe { ($($t::fetch(state.$i, row),)*) }
            }
        }
    };
}
query_tuple!(A 0);
query_tuple!(A 0, B 1);
query_tuple!(A 0, B 1, C 2);
query_tuple!(A 0, B 1, C 2, D 3);
query_tuple!(A 0, B 1, C 2, D 3, E 4);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10);
query_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11);

/// Panics if a query asks for the same component twice with at least one `&mut`.
pub(crate) fn check_aliasing<Q: QueryData>(world: &World, ids: &Q::Ids) {
    const MAX: usize = 32;
    let mut seen = [(ComponentId(0), false); MAX];
    let mut n = 0;
    Q::access(ids, &mut |id, mutable, _| {
        for &(other, other_mut) in &seen[..n] {
            if other == id && (mutable || other_mut) {
                panic!(
                    "query `{}` accesses `{}` twice, once mutably; that would alias",
                    std::any::type_name::<Q>(),
                    world.component_info(id).name()
                );
            }
        }
        assert!(
            n < MAX,
            "query `{}` touches more than {MAX} components",
            std::any::type_name::<Q>()
        );
        seen[n] = (id, mutable);
        n += 1;
    });
}

/// Filters added with [`Query::with`], [`Query::without`] and
/// [`Query::filter`]. Inline storage, so running a filtered query doesn't
/// allocate.
///
/// An archetype matches when it has every `with` component, none of the
/// `without` components, and satisfies each `Or` clause: clause `c` holds
/// the terms `any[start..any_end[c]]`, and term `i` asks for presence when
/// bit `i` of `any_has` is set, absence otherwise.
///
/// `Added` and `Changed` filters also pick rows inside a matching archetype
/// (see [`Filters::row_filter`]). One that must hold is a `with` whose bit
/// is set in `with_added` or `with_changed`; inside an `Or`, bit `i` of
/// `any_added` or `any_changed` marks term `i` (which then asks for presence
/// too).
///
/// Public but not exported: it can't be named outside this crate, which
/// seals [`QueryFilter`], [`OrTerm`] and [`OrTerms`].
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct Filters {
    with: [ComponentId; MAX_WITH],
    without: [ComponentId; MAX_WITH],
    n_with: u8,
    n_without: u8,
    any: [ComponentId; MAX_OR_TERMS],
    any_has: u16,
    any_end: [u8; MAX_OR],
    n_any: u8,
    n_terms: u8,
    with_added: u8,
    with_changed: u8,
    any_added: u16,
    any_changed: u16,
}

const MAX_WITH: usize = 8;
const MAX_TICKS: usize = 4;
const MAX_OR: usize = 4;
const MAX_OR_TERMS: usize = 16;

impl Filters {
    /// Always inlined: the query loops rely on it to specialize their
    /// archetype loop for the filters in use.
    #[inline(always)]
    pub(crate) fn matches(&self, arch: &Archetype) -> bool {
        if !(self.with[..self.n_with as usize].iter().all(|&id| arch.has(id))
            && !self.without[..self.n_without as usize].iter().any(|&id| arch.has(id)))
        {
            return false;
        }
        let mut start = 0;
        for &end in &self.any_end[..self.n_any as usize] {
            let end = end as usize;
            let any = (start..end).any(|i| arch.has(self.any[i]) == (self.any_has >> i & 1 != 0));
            if !any {
                return false;
            }
            start = end;
        }
        true
    }

    /// [`matches`](Self::matches) as a call, for the loops of worlds that
    /// track changes: they exist next to the plain ones at every call site,
    /// so they stay small.
    #[inline(never)]
    fn matches_tracked(&self, arch: &Archetype) -> bool {
        self.matches(arch)
    }

    pub(crate) fn add_with(&mut self, id: ComponentId) {
        if self.with[..self.n_with as usize].contains(&id) {
            return;
        }
        assert!(
            (self.n_with as usize) < MAX_WITH,
            "at most {MAX_WITH} `with` filters per query"
        );
        self.with[self.n_with as usize] = id;
        self.n_with += 1;
    }

    pub(crate) fn add_without(&mut self, id: ComponentId) {
        if self.without[..self.n_without as usize].contains(&id) {
            return;
        }
        assert!(
            (self.n_without as usize) < MAX_WITH,
            "at most {MAX_WITH} `without` filters per query"
        );
        self.without[self.n_without as usize] = id;
        self.n_without += 1;
    }

    /// Adds a term to the `Or` clause being built: the terms after the last
    /// closed clause.
    fn add_or_term(&mut self, id: ComponentId, has: bool) {
        let n = self.n_terms as usize;
        assert!(
            n < MAX_OR_TERMS,
            "at most {MAX_OR_TERMS} components across a query's `Or` filters"
        );
        self.any[n] = id;
        if has {
            self.any_has |= 1 << n;
        }
        self.n_terms += 1;
    }

    /// Adds an `Added` (`added`) or `Changed` filter on component `id` that
    /// must hold; the component must be tracked.
    pub(crate) fn add_tick(&mut self, id: ComponentId, added: bool) {
        self.add_with(id);
        let n = self.n_with as usize;
        let i = self.with[..n].iter().position(|&with| with == id).expect("just added");
        let filters = self.with_added.count_ones() + self.with_changed.count_ones();
        let mask = if added {
            &mut self.with_added
        } else {
            &mut self.with_changed
        };
        if *mask >> i & 1 != 0 {
            return;
        }
        assert!(
            filters < MAX_TICKS as u32,
            "at most {MAX_TICKS} `Added`/`Changed` filters per query outside `Or`"
        );
        *mask |= 1 << i;
    }

    /// Adds an `Added` (`added`) or `Changed` term on component `id` to the
    /// `Or` clause being built; the component must be tracked.
    pub(crate) fn add_or_tick(&mut self, id: ComponentId, added: bool) {
        let n = self.n_terms as usize;
        self.add_or_term(id, true);
        if added {
            self.any_added |= 1 << n;
        } else {
            self.any_changed |= 1 << n;
        }
    }

    /// The row test `Added`/`Changed` filters apply inside matching archetype
    /// `arch`, or `None` if every row passes: no such filters, or each one
    /// sits in an `Or` that another term already satisfies for all of `arch`.
    #[inline]
    pub(crate) fn row_filter(&self, arch: &Archetype, clock: Clock) -> Option<RowFilter> {
        let in_or = self.any_added | self.any_changed;
        if self.with_added | self.with_changed == 0 && in_or == 0 {
            return None;
        }
        let mut rows = RowFilter::new(clock);
        for i in 0..self.n_with as usize {
            for (mask, added) in [(self.with_added, true), (self.with_changed, false)] {
                if mask >> i & 1 != 0 {
                    let stamps = arch.read_stamps(self.with[i], added);
                    rows.term(stamps.expect("an `Added`/`Changed` filter's component is present and tracked"));
                    rows.close();
                }
            }
        }
        let mut start = 0;
        for &end in &self.any_end[..self.n_any as usize] {
            let end = end as usize;
            let whole =
                (start..end).any(|i| in_or >> i & 1 == 0 && arch.has(self.any[i]) == (self.any_has >> i & 1 != 0));
            if !whole {
                for i in (start..end).filter(|i| in_or >> i & 1 != 0) {
                    if let Some(stamps) = arch.read_stamps(self.any[i], self.any_added >> i & 1 != 0) {
                        rows.term(stamps);
                    }
                }
                rows.close();
            }
            start = end;
        }
        (!rows.is_empty()).then_some(rows)
    }

    /// Closes the `Or` clause being built.
    fn close_or(&mut self) {
        assert!(
            (self.n_any as usize) < MAX_OR,
            "at most {MAX_OR} `Or` filters per query"
        );
        self.any_end[self.n_any as usize] = self.n_terms;
        self.n_any += 1;
    }
}

// Cache candidates, never QueryData::State: column buffers and the archetype
// Vec may move between queries. Q::state is still resolved under the live borrow.
// Bounds cap signature memory; saturated caches fall back without eviction churn.
const CACHE_MIN_ARCHETYPES: usize = 128;
const CACHE_MAX_SIGNATURES: usize = 128;

#[derive(PartialEq, Eq, Hash)]
struct MatchKey {
    required: [ComponentId; 32],
    n_required: usize,
    filters: Filters,
}

#[derive(Default)]
pub(crate) struct MatchCache(HashMap<MatchKey, MatchEntry>);

#[derive(Default)]
struct MatchEntry {
    // Archetypes are append-only: their count is a non-wrapping generation.
    // Empty layouts must stay cached: later spawns can populate them again.
    generation: usize,
    indices: Vec<usize>,
}

pub(crate) struct Archetypes<'a> {
    pub(crate) all: &'a [Archetype],
    pub(crate) indices: Option<std::slice::Iter<'a, usize>>,
    pub(crate) next: usize,
}

impl<'a> Iterator for Archetypes<'a> {
    type Item = &'a Archetype;
    fn next(&mut self) -> Option<Self::Item> {
        let index = if let Some(indices) = &mut self.indices {
            *indices.next()?
        } else {
            let index = self.next;
            self.next += 1;
            index
        };
        self.all.get(index)
    }
}

/// A typed query filter for [`Query::filter`]: [`With<T>`], [`Without<T>`],
/// [`Added<T>`](crate::Added), [`Changed<T>`](crate::Changed), [`Or`], or a
/// tuple of filters that must all hold (up to 8).
///
/// Name a filter you reuse across systems:
///
/// ```
/// # use ponko_ecs::*;
/// struct Health(i32);
/// struct Player;
/// struct Enemy;
/// struct Dead;
///
/// /// Living characters: players or enemies, not dead.
/// type Living = (Or<(With<Player>, With<Enemy>)>, Without<Dead>);
///
/// let mut world = World::new();
/// world.spawn((Health(10), Player));
/// world.spawn((Health(5), Enemy, Dead));
/// world.spawn((Health(1),));
///
/// assert_eq!(world.query::<&Health>().filter::<Living>().count(), 1);
/// ```
///
/// Implemented by this crate only: combine the provided filters.
pub trait QueryFilter {
    #[doc(hidden)]
    fn add(world: &mut World, filters: &mut Filters);
}

/// A term of an [`Or`] filter: [`With<T>`], [`Without<T>`],
/// [`Added<T>`](crate::Added), [`Changed<T>`](crate::Changed) or a nested `Or`.
pub trait OrTerm {
    #[doc(hidden)]
    fn add_term(world: &mut World, filters: &mut Filters);
}

/// The terms of an [`Or`] filter: a tuple of 1 to 8 [`OrTerm`]s.
pub trait OrTerms {
    #[doc(hidden)]
    fn add_terms(world: &mut World, filters: &mut Filters);
}

/// Filter: the entity has component `T` (it isn't fetched).
pub struct With<T>(PhantomData<fn() -> T>);

/// Filter: the entity doesn't have component `T`.
pub struct Without<T>(PhantomData<fn() -> T>);

/// Filter: at least one of the terms holds, for example
/// `Or<(With<Player>, With<Enemy>)>`. The terms are [`With`], [`Without`],
/// [`Added`](crate::Added), [`Changed`](crate::Changed) or nested `Or`s; for
/// "both A and B", put the filters in a tuple outside the `Or` instead.
///
/// A query takes at most 4 `Or` filters with 16 terms between them.
///
/// A tuple inside an `Or` doesn't compile:
///
/// ```compile_fail
/// # use ponko_ecs::*;
/// struct A;
/// struct B;
/// struct C;
/// let mut world = World::new();
/// let _ = world.query::<Entity>().filter::<Or<((With<A>, With<B>), With<C>)>>();
/// ```
pub struct Or<T>(PhantomData<fn() -> T>);

impl<T: Component> QueryFilter for With<T> {
    fn add(world: &mut World, filters: &mut Filters) {
        filters.add_with(world.register::<T>());
    }
}

impl<T: Component> QueryFilter for Without<T> {
    fn add(world: &mut World, filters: &mut Filters) {
        filters.add_without(world.register::<T>());
    }
}

impl<T: OrTerms> QueryFilter for Or<T> {
    fn add(world: &mut World, filters: &mut Filters) {
        T::add_terms(world, filters);
        filters.close_or();
    }
}

impl<T: Component> OrTerm for With<T> {
    fn add_term(world: &mut World, filters: &mut Filters) {
        filters.add_or_term(world.register::<T>(), true);
    }
}

impl<T: Component> OrTerm for Without<T> {
    fn add_term(world: &mut World, filters: &mut Filters) {
        filters.add_or_term(world.register::<T>(), false);
    }
}

impl<T: OrTerms> OrTerm for Or<T> {
    fn add_term(world: &mut World, filters: &mut Filters) {
        T::add_terms(world, filters);
    }
}

macro_rules! filter_tuple {
    ($($t:ident),*) => {
        impl<$($t: QueryFilter),*> QueryFilter for ($($t,)*) {
            fn add(world: &mut World, filters: &mut Filters) {
                $($t::add(world, filters);)*
            }
        }
        impl<$($t: OrTerm),*> OrTerms for ($($t,)*) {
            fn add_terms(world: &mut World, filters: &mut Filters) {
                $($t::add_term(world, filters);)*
            }
        }
    };
}
filter_tuple!(A);
filter_tuple!(A, B);
filter_tuple!(A, B, C);
filter_tuple!(A, B, C, D);
filter_tuple!(A, B, C, D, E);
filter_tuple!(A, B, C, D, E, F);
filter_tuple!(A, B, C, D, E, F, G);
filter_tuple!(A, B, C, D, E, F, G, H);

/// A query over a [`World`], created by [`World::query`].
///
/// ```
/// # use ponko_ecs::*;
/// struct Pos(f32);
/// struct Vel(f32);
/// struct Frozen;
///
/// let mut world = World::new();
/// world.spawn((Pos(0.0), Vel(1.0)));
/// world.spawn((Pos(0.0), Vel(1.0), Frozen));
///
/// world.query::<(&mut Pos, &Vel)>().without::<Frozen>().for_each(|(p, v)| p.0 += v.0);
///
/// let moved = world.query::<&Pos>().iter().filter(|p| p.0 > 0.0).count();
/// assert_eq!(moved, 1);
/// ```
#[must_use = "a query does nothing until iterated"]
pub struct Query<'w, Q: QueryData> {
    world: &'w mut World,
    ids: Q::Ids,
    filters: Filters,
    cached: bool,
    /// What `Added`/`Changed` filters compare against, if not the world's
    /// [`changes_since`](World::changes_since).
    since: Option<Tick>,
}

impl<'w, Q: QueryData> Query<'w, Q> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        let ids = Q::ids(world);
        check_aliasing::<Q>(world, &ids);
        world
            .recorder
            .note(|l| Q::access(&ids, &mut |id, write, _| l.component(id, write)));
        Query {
            world,
            ids,
            filters: Filters::default(),
            cached: false,
            since: None,
        }
    }

    /// The world's current tick and this query's `since`, for change stamps.
    #[inline]
    fn clock(&self) -> Clock {
        let since = self.since.unwrap_or(self.world.changes_since());
        Clock::new(self.world.change_tick(), since)
    }

    /// Whether any component of the world is tracked. The loops test this
    /// before an archetype's own flag, so a world that tracks nothing doesn't
    /// even read that flag.
    #[inline]
    fn tracking(&self) -> bool {
        self.world.tracks_changes()
    }

    fn archetypes(&mut self) -> Archetypes<'_> {
        let world = &mut self.world;
        if !self.cached || world.archetypes.len() < CACHE_MIN_ARCHETYPES {
            return Archetypes {
                all: &world.archetypes,
                indices: None,
                next: 0,
            };
        }
        let mut key = MatchKey {
            required: [ComponentId(0); 32],
            n_required: 0,
            filters: self.filters,
        };
        Q::access(&self.ids, &mut |id, _, required| {
            if required {
                key.required[key.n_required] = id;
                key.n_required += 1;
            }
        });
        key.required[..key.n_required].sort_unstable();
        key.filters.with[..key.filters.n_with as usize].sort_unstable();
        key.filters.without[..key.filters.n_without as usize].sort_unstable();
        let cache = &mut world.query_matches.0;
        if cache.len() >= CACHE_MAX_SIGNATURES && !cache.contains_key(&key) {
            return Archetypes {
                all: &world.archetypes,
                indices: None,
                next: 0,
            };
        }
        let required = key.required;
        let n_required = key.n_required;
        let entry = cache.entry(key).or_default();
        // Only newly created layouts need matching; existing membership cannot
        // change. Registration alone doesn't change any existing layout.
        for (index, arch) in world.archetypes.iter().enumerate().skip(entry.generation) {
            if self.filters.matches(arch) && required[..n_required].iter().all(|&id| arch.has(id)) {
                entry.indices.push(index);
            }
        }
        entry.generation = world.archetypes.len();
        Archetypes {
            all: &world.archetypes,
            indices: Some(entry.indices.iter()),
            next: 0,
        }
    }

    /// Reuse a world-owned matching-archetype list for repeated queries with
    /// this required-component and filter signature.
    ///
    /// This is opt-in: selective queries over many layouts benefit most; dense
    /// queries can lose to a direct scan. Worlds with fewer than 128 archetypes
    /// use the direct scan. At most 128 signatures are retained per world;
    /// further signatures also scan directly. Cold matching and newly created
    /// archetypes may allocate; steady-state repeated queries do not.
    ///
    /// Only archetype indices are retained, never component pointers. New
    /// layouts are matched before traversal; empty matching layouts remain
    /// eligible when populated again. Optional components and mutability do
    /// not affect the signature. Custom [`QueryData::state`] is still called
    /// for each candidate under the query's live world borrow.
    pub fn cached(mut self) -> Self {
        self.cached = true;
        self
    }

    /// Only match entities that also have component `T` (without fetching it).
    /// Shorthand for `.filter::<With<T>>()`.
    ///
    /// # Panics
    ///
    /// With more than 8 different `with` components on one query.
    pub fn with<T: Component>(mut self) -> Self {
        let id = self.world.register::<T>();
        self.filters.add_with(id);
        self
    }

    /// Skip entities that have component `T`. Shorthand for
    /// `.filter::<Without<T>>()`.
    ///
    /// # Panics
    ///
    /// With more than 8 different `without` components on one query.
    pub fn without<T: Component>(mut self) -> Self {
        let id = self.world.register::<T>();
        self.filters.add_without(id);
        self
    }

    /// Only match entities that pass the typed filter `F`: [`With`],
    /// [`Without`], [`Added`](crate::Added), [`Changed`](crate::Changed),
    /// [`Or`], or a tuple of them that must all hold. Calls combine with
    /// each other and with [`with`](Self::with) / [`without`](Self::without):
    /// every filter must hold. `Added` and `Changed` pick single entities
    /// (see [`World::track_changes`]); the others pick whole archetypes.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// struct Pos(f32);
    /// struct Player;
    /// struct Enemy;
    /// let mut world = World::new();
    /// world.spawn((Pos(0.0), Player));
    /// world.spawn((Pos(0.0), Enemy));
    /// world.spawn((Pos(0.0),));
    ///
    /// let n = world.query::<&Pos>().filter::<Or<(With<Player>, With<Enemy>)>>().count();
    /// assert_eq!(n, 2);
    /// ```
    ///
    /// # Panics
    ///
    /// With more than 8 different `With` or `Without` components, more than
    /// 4 `Or` filters, more than 16 terms across the `Or` filters, or more
    /// than 4 `Added`/`Changed` filters outside an `Or`.
    pub fn filter<F: QueryFilter>(mut self) -> Self {
        F::add(self.world, &mut self.filters);
        self
    }

    /// Makes [`Added`](crate::Added) and [`Changed`](crate::Changed) filters
    /// of this query report what happened after `tick`, instead of after the
    /// world's [`changes_since`](World::changes_since). Keep the
    /// [`Tick`] from [`World::advance_change_tick`] to continue where a
    /// reader left off; `Tick::default()` reports everything.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// struct Pos(f32);
    /// let mut world = World::new();
    /// world.track_changes::<Pos>();
    /// world.spawn((Pos(0.0),));
    /// let seen = world.advance_change_tick();
    /// let moved = world.spawn((Pos(0.0),));
    /// world.get_mut::<Pos>(moved).unwrap().0 = 1.0;
    ///
    /// let mut changed = world.query::<Entity>().filter::<Changed<Pos>>().since(seen);
    /// assert_eq!(changed.iter().collect::<Vec<_>>(), [moved]);
    /// ```
    pub fn since(mut self, tick: Tick) -> Self {
        self.since = Some(tick);
        self
    }

    /// Runs `f` for every matching entity. The fastest way to loop.
    #[inline]
    pub fn for_each(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        if !self.tracking() {
            // The loop of a world that tracks nothing: kept apart from the
            // tracked one, so it is compiled as it would be without change
            // detection.
            let ids = self.ids;
            let filters = self.filters;
            for arch in self.archetypes() {
                let len = arch.len();
                if len == 0 || !filters.matches(arch) {
                    continue;
                }
                let Some(state) = Q::state(arch, &ids) else {
                    continue;
                };
                for row in 0..len {
                    f(unsafe { Q::fetch(state, row) });
                }
            }
            return;
        }
        self.for_each_tracked(f);
    }

    /// [`for_each`](Self::for_each) in a world that tracks changes: a
    /// tracked archetype stamps what the query may write, and with an
    /// `Added`/`Changed` filter yields runs of rows instead of all of them.
    ///
    /// Always inlined: `f` must not be handed to a call the compiler can't
    /// see into, or the variables it captures stop living in registers in
    /// the caller's untracked loop too.
    #[inline(always)]
    fn for_each_tracked(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        let ids = self.ids;
        let filters = self.filters;
        let clock = self.clock();
        for arch in self.archetypes() {
            let len = arch.len();
            if len == 0 || !filters.matches_tracked(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &ids) else {
                continue;
            };
            let mut runs = if arch.tracked() {
                change::visit::<Q>(arch, &ids, &filters, clock)
            } else {
                None
            };
            let (mut start, mut end) = (0, len);
            loop {
                if let Some(runs) = &mut runs {
                    match runs.next::<Q>(&ids) {
                        Some(run) => (start, end) = run,
                        None => break,
                    }
                }
                for row in start..end {
                    f(unsafe { Q::fetch(state, row) });
                }
                if runs.is_none() {
                    break;
                }
            }
        }
    }

    /// Like [`for_each`](Self::for_each), but spread over all CPU cores.
    ///
    /// Each archetype is cut into chunks of rows, and the chunks run in
    /// parallel. Use it for heavy per-entity work over many entities; for a
    /// few thousand cheap updates, `for_each` is just as fast.
    ///
    /// `f` can't capture `&mut` state (it runs on many threads at once). To
    /// spawn or despawn, collect what you need and use commands afterwards.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// struct Pos(f32);
    /// struct Vel(f32);
    /// let mut world = World::new();
    /// world.spawn_batch((0..100_000).map(|i| (Pos(i as f32), Vel(1.0))));
    ///
    /// world.query::<(&mut Pos, &Vel)>().par_for_each(|(p, v)| p.0 += v.0);
    /// ```
    #[cfg(feature = "parallel")]
    pub fn par_for_each(&mut self, f: impl Fn(Q::Item<'_>) + Send + Sync) {
        self.par_for_each_indexed(|_, item| f(item));
    }

    /// Like [`par_for_each`](Self::par_for_each), but also passes each
    /// entity's position in the query, `0..count()`, in the same order as
    /// [`for_each`](Self::for_each) and [`iter`](Self::iter).
    ///
    /// Use it to fill a buffer from all cores: every entity writes its own
    /// slot, so the result comes out in a stable order.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// use std::sync::atomic::{AtomicU32, Ordering};
    ///
    /// struct Pos(f32);
    /// let mut world = World::new();
    /// world.spawn_batch((0..10_000).map(|i| (Pos(i as f32),)));
    ///
    /// let mut query = world.query::<&Pos>();
    /// let out: Vec<AtomicU32> = (0..query.count()).map(|_| AtomicU32::new(0)).collect();
    /// query.par_for_each_indexed(|i, p| out[i].store(p.0 as u32 * 2, Ordering::Relaxed));
    /// assert_eq!(out[1234].load(Ordering::Relaxed), 2468);
    /// ```
    #[cfg(feature = "parallel")]
    pub fn par_for_each_indexed(&mut self, f: impl Fn(usize, Q::Item<'_>) + Send + Sync) {
        use rayon::prelude::*;

        /// Rows per parallel task, chosen per archetype: about 4 tasks per
        /// thread so heavy per-entity work (neighbor search) uses every core
        /// even for a few thousand entities, but at least `MIN_CHUNK` rows so
        /// cheap loops stay vectorized and scheduling stays negligible.
        /// Tiny under Miri, so tests reach the multi-chunk path with few rows.
        const MIN_CHUNK: usize = if cfg!(miri) { 64 } else { 512 };
        const MAX_CHUNK: usize = if cfg!(miri) { 64 } else { 8192 };
        let threads = rayon::current_num_threads().max(1);

        /// Archetype column pointers, shared with worker threads.
        #[derive(Clone, Copy)]
        struct Shared<S>(S);
        // SAFETY: the pointers refer to component columns (`Send + Sync`
        // components); each task touches a disjoint row range.
        unsafe impl<S> Send for Shared<S> {}
        unsafe impl<S> Sync for Shared<S> {}

        // Position of the next run's first row in the whole query.
        let mut base = 0;
        let ids = self.ids;
        let filters = self.filters;
        let clock = self.clock();
        let tracking = self.tracking();
        for arch in self.archetypes() {
            let len = arch.len();
            if len == 0 || !filters.matches(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &ids) else {
                continue;
            };
            // As in `for_each`: stamps are written and row filters are read
            // here, on the calling thread, before a run's rows are shared.
            let mut runs = if tracking && arch.tracked() {
                change::visit::<Q>(arch, &ids, &filters, clock)
            } else {
                None
            };
            let (mut start, mut end) = (0, len);
            loop {
                if let Some(runs) = &mut runs {
                    match runs.next::<Q>(&ids) {
                        Some(run) => (start, end) = run,
                        None => break,
                    }
                }
                let len = end - start;
                let chunk = (len / (threads * 4)).clamp(MIN_CHUNK, MAX_CHUNK);
                if len <= chunk {
                    for row in start..end {
                        f(base + row - start, unsafe { Q::fetch(state, row) });
                    }
                } else {
                    let shared = Shared(state);
                    (0..len.div_ceil(chunk)).into_par_iter().for_each(|c| {
                        let state = shared;
                        let last = (start + (c + 1) * chunk).min(end);
                        for row in start + c * chunk..last {
                            f(base + row - start, unsafe { Q::fetch(state.0, row) });
                        }
                    });
                }
                base += len;
                if runs.is_none() {
                    break;
                }
            }
        }
    }

    /// Replaces the contents of `out` with `f` of every matching entity,
    /// computed on all cores, in the same order as [`for_each`](Self::for_each).
    /// `out` keeps its memory between calls, so collecting every tick
    /// allocates nothing once it's big enough. If `f` panics, completed results
    /// are dropped and `out` is left empty with its allocation available for reuse.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// struct Pos(f32);
    /// let mut world = World::new();
    /// world.spawn_batch((0..10_000).map(|i| (Pos(i as f32),)));
    ///
    /// let mut doubled = Vec::new();
    /// world.query::<&Pos>().par_collect_into(&mut doubled, |p| p.0 * 2.0);
    /// assert_eq!(doubled[1234], 2468.0);
    /// ```
    #[cfg(feature = "parallel")]
    pub fn par_collect_into<R: Send>(&mut self, out: &mut Vec<R>, f: impl Fn(Q::Item<'_>) -> R + Send + Sync) {
        /// `out`'s buffer, shared with worker threads.
        struct Slots<R>(*mut R);
        impl<R> Clone for Slots<R> {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<R> Copy for Slots<R> {}
        impl<R> Slots<R> {
            /// A method, so closures capture the whole `Slots` (which is
            /// `Sync`), not its raw pointer field.
            fn at(self, i: usize) -> *mut R {
                // SAFETY (of callers): `i` is within the reserved capacity.
                self.0.wrapping_add(i)
            }
        }
        // SAFETY: each query index is written by exactly one task.
        unsafe impl<R: Send> Send for Slots<R> {}
        unsafe impl<R: Send> Sync for Slots<R> {}

        out.clear();
        let n = self.count();
        out.reserve(n);
        let slots = Slots(out.as_mut_ptr());
        if !std::mem::needs_drop::<R>() {
            self.par_for_each_indexed(|i, item| {
                debug_assert!(i < n);
                // SAFETY: every index in the reserved buffer is written once.
                // On panic these values need no cleanup and `out.len()` stays 0.
                unsafe { slots.at(i).write(f(item)) };
            });
        } else {
            /// Owns only the initialized prefix of a disjoint output range.
            struct Initialized<R> {
                slots: Slots<R>,
                len: usize,
            }
            impl<R> Drop for Initialized<R> {
                fn drop(&mut self) {
                    // SAFETY: only this guard owns these initialized values;
                    // sibling tasks write disjoint ranges. Vec's length is 0.
                    unsafe { std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(self.slots.at(0), self.len)) };
                }
            }

            // Return ownership of completed ranges through `join`: if either
            // branch panics, rayon waits for its sibling and drops its result.
            // No shared counter, bitmap, or per-call allocation is needed.
            fn fill<R: Send>(
                mut initialized: Initialized<R>,
                start: usize,
                len: usize,
                chunk: usize,
                f: &(impl Fn(usize) -> R + Sync),
            ) -> Initialized<R> {
                if len > chunk {
                    let mid = (len.div_ceil(chunk) / 2) * chunk;
                    let right = Initialized {
                        slots: Slots(initialized.slots.at(mid)),
                        len: 0,
                    };
                    let (mut left, mut right) = rayon::join(
                        || fill(initialized, start, mid, chunk, f),
                        || fill(right, start + mid, len - mid, chunk, f),
                    );
                    // Both adjacent ranges are complete; transfer ownership.
                    left.len += right.len;
                    right.len = 0;
                    left
                } else {
                    for row in 0..len {
                        // SAFETY: this task exclusively owns these reserved
                        // slots. Record each write before calling `f` again.
                        unsafe { initialized.slots.at(row).write(f(start + row)) };
                        initialized.len += 1;
                    }
                    initialized
                }
            }

            struct SharedState<S>(S);
            impl<S: Copy> SharedState<S> {
                fn get(&self) -> S {
                    self.0
                }
            }
            // SAFETY: as in par_for_each_indexed, query access is validated and
            // workers fetch disjoint rows of Send + Sync component columns.
            unsafe impl<S> Send for SharedState<S> {}
            unsafe impl<S> Sync for SharedState<S> {}

            let threads = rayon::current_num_threads().max(1);
            let min_chunk = if cfg!(miri) { 64 } else { 512 };
            let max_chunk = if cfg!(miri) { 64 } else { 8192 };
            let mut initialized = Initialized { slots, len: 0 };
            let clock = self.clock();
            let tracking = self.tracking();
            for arch in self.world.archetypes.iter() {
                let len = arch.len();
                if len == 0 || !self.filters.matches(arch) {
                    continue;
                }
                let Some(state) = Q::state(arch, &self.ids) else {
                    continue;
                };
                let state = SharedState(state);
                // As in `for_each`: tracked archetypes are stamped, and may
                // yield runs of rows instead of every row.
                let mut runs = if tracking && arch.tracked() {
                    change::visit::<Q>(arch, &self.ids, &self.filters, clock)
                } else {
                    None
                };
                let (mut start, mut end) = (0, len);
                loop {
                    if let Some(runs) = &mut runs {
                        match runs.next::<Q>(&self.ids) {
                            Some(run) => (start, end) = run,
                            None => break,
                        }
                    }
                    let len = end - start;
                    let chunk = (len / (threads * 4)).clamp(min_chunk, max_chunk);
                    let mut completed = fill(
                        Initialized {
                            slots: Slots(initialized.slots.at(initialized.len)),
                            len: 0,
                        },
                        0,
                        len,
                        chunk,
                        &|row| f(unsafe { Q::fetch(state.get(), start + row) }),
                    );
                    initialized.len += completed.len;
                    completed.len = 0;
                    if runs.is_none() {
                        break;
                    }
                }
            }
            // All results now belong to `out`; disarm before setting its length.
            initialized.len = 0;
        }
        // SAFETY: both paths wrote all n slots; unwinding never reaches here.
        unsafe { out.set_len(n) };
    }

    /// An iterator over matching entities, for `map`/`filter`/`sum` and friends.
    ///
    /// Convenient, but several times slower than [`for_each`](Self::for_each)
    /// on tight math loops (the compiler can't vectorize across archetypes).
    /// Use `for_each` for per-tick simulation work.
    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        let ids = self.ids;
        let filters = self.filters;
        let clock = self.clock();
        let tracking = self.tracking();
        QueryIter {
            archetypes: self.archetypes(),
            ids,
            filters,
            state: None,
            row: 0,
            len: 0,
            clock,
            tracking,
            filtered: None,
            _q: PhantomData,
        }
    }

    /// Number of matching entities. It doesn't visit them one by one, except
    /// to test [`Added`](crate::Added) and [`Changed`](crate::Changed)
    /// filters, and it doesn't count as a change to anything.
    pub fn count(&mut self) -> usize {
        let ids = self.ids;
        let filters = self.filters;
        if !self.tracking() {
            return self
                .archetypes()
                .filter(|a| !a.is_empty() && filters.matches(a) && Q::state(a, &ids).is_some())
                .map(|a| a.len())
                .sum();
        }
        let clock = self.clock();
        self.archetypes()
            .filter(|a| !a.is_empty() && filters.matches(a) && Q::state(a, &ids).is_some())
            .map(|a| {
                if a.tracked() {
                    change::count(a, &filters, clock)
                } else {
                    a.len()
                }
            })
            .sum()
    }

    /// The query's item for one entity, or `None` if it's dead or doesn't match.
    pub fn get(&mut self, e: Entity) -> Option<Q::Item<'_>> {
        let ids = self.ids;
        let filters = self.filters;
        let loc = self.world.entities.get(e)?;
        let arch = &self.world.archetypes[loc.archetype as usize];
        if !filters.matches(arch) {
            return None;
        }
        let state = Q::state(arch, &ids)?;
        if self.tracking()
            && arch.tracked()
            && !change::visit_row::<Q>(arch, &ids, &filters, self.clock(), loc.row as usize)
        {
            return None;
        }
        Some(unsafe { Q::fetch(state, loc.row as usize) })
    }
}

/// Iterator returned by [`Query::iter`].
pub struct QueryIter<'a, Q: QueryData> {
    pub(crate) archetypes: Archetypes<'a>,
    pub(crate) ids: Q::Ids,
    pub(crate) filters: Filters,
    pub(crate) state: Option<Q::State>,
    pub(crate) row: usize,
    pub(crate) len: usize,
    pub(crate) clock: Clock,
    /// Whether the world tracks any component at all.
    pub(crate) tracking: bool,
    /// The current archetype, when an `Added`/`Changed` filter picks its
    /// rows: they then come in runs, and `len` is the current run's end.
    pub(crate) filtered: Option<&'a Archetype>,
    pub(crate) _q: PhantomData<&'a mut Q>,
}

impl<'a, Q: QueryData> Iterator for QueryIter<'a, Q> {
    type Item = Q::Item<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(state) = self.state
                && self.row < self.len
            {
                let row = self.row;
                self.row += 1;
                return Some(unsafe { Q::fetch(state, row) });
            }
            if let Some(arch) = self.filtered {
                // The next run starts at or after the end of this one.
                match change::next_run::<Q>(arch, self.ids, self.filters, self.clock, self.len) {
                    Some((start, end)) => {
                        self.row = start;
                        self.len = end;
                        continue;
                    }
                    None => self.filtered = None,
                }
            }
            let arch = self.archetypes.next()?;
            self.state = if arch.is_empty() || !self.filters.matches(arch) {
                None
            } else {
                Q::state(arch, &self.ids)
            };
            self.row = 0;
            self.len = arch.len();
            if self.tracking
                && arch.tracked()
                && self.state.is_some()
                && change::enter::<Q>(arch, self.ids, self.filters, self.clock)
            {
                // The rows come from runs.
                self.filtered = Some(arch);
                self.len = 0;
            }
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn signatures_are_bounded_and_filter_order_is_canonical() {
        let mut world = World::new();
        // Empty, immutable layouts isolate cache bookkeeping from entity storage.
        for _ in 0..CACHE_MIN_ARCHETYPES {
            world.archetypes.push(Archetype::new(Box::new([]), &[]));
        }
        world
            .query::<Entity>()
            .cached()
            .without::<u8>()
            .without::<u16>()
            .count();
        world
            .query::<Entity>()
            .without::<u16>()
            .cached()
            .without::<u8>()
            .count();
        assert_eq!(world.query_matches.0.len(), 1);
        for id in 0..CACHE_MAX_SIGNATURES as u32 {
            let mut q = world.query::<Entity>().cached();
            q.filters.without[0] = ComponentId(id);
            q.filters.n_without = 1;
            assert_eq!(q.count(), 0);
        }
        assert_eq!(world.query_matches.0.len(), CACHE_MAX_SIGNATURES);
        let mut q = world.query::<Entity>().cached();
        q.filters.with[0] = ComponentId(1000);
        q.filters.n_with = 1;
        assert!(q.archetypes().indices.is_none());
        assert_eq!(q.count(), 0);
        assert_eq!(world.query_matches.0.len(), CACHE_MAX_SIGNATURES);
    }
}
