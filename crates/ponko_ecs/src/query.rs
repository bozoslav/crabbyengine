//! Queries: loop over every entity that has a set of components.

use std::marker::PhantomData;

use crate::bundle::Component;
use crate::entity::Entity;
use crate::storage::{Archetype, ComponentId};
use crate::world::World;

/// What a query fetches per entity: `&T`, `&mut T`, `Option<&T>`,
/// `Option<&mut T>`, [`Entity`], or a tuple of those (up to 8).
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

/// Filters added with [`Query::with`] / [`Query::without`]. Inline storage, so
/// running a filtered query doesn't allocate.
#[derive(Clone, Copy, Default)]
struct Filters {
    with: [ComponentId; 4],
    without: [ComponentId; 4],
    n_with: u8,
    n_without: u8,
}

impl Filters {
    #[inline]
    fn matches(&self, arch: &Archetype) -> bool {
        self.with[..self.n_with as usize].iter().all(|&id| arch.has(id))
            && !self.without[..self.n_without as usize].iter().any(|&id| arch.has(id))
    }
}

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
        }
    }

    /// Only match entities that also have component `T` (without fetching it).
    ///
    /// # Panics
    ///
    /// With more than 4 `with` filters on one query.
    pub fn with<T: Component>(mut self) -> Self {
        let id = self.world.register::<T>();
        let f = &mut self.filters;
        assert!((f.n_with as usize) < f.with.len(), "at most 4 `with` filters per query");
        f.with[f.n_with as usize] = id;
        f.n_with += 1;
        self
    }

    /// Skip entities that have component `T`.
    ///
    /// # Panics
    ///
    /// With more than 4 `without` filters on one query.
    pub fn without<T: Component>(mut self) -> Self {
        let id = self.world.register::<T>();
        let f = &mut self.filters;
        assert!(
            (f.n_without as usize) < f.without.len(),
            "at most 4 `without` filters per query"
        );
        f.without[f.n_without as usize] = id;
        f.n_without += 1;
        self
    }

    /// Runs `f` for every matching entity. The fastest way to loop.
    #[inline]
    pub fn for_each(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        for arch in self.world.archetypes.iter() {
            let len = arch.len();
            if len == 0 || !self.filters.matches(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &self.ids) else {
                continue;
            };
            for row in 0..len {
                f(unsafe { Q::fetch(state, row) });
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

        // Position of the archetype's first row in the whole query.
        let mut base = 0;
        for arch in self.world.archetypes.iter() {
            let len = arch.len();
            if len == 0 || !self.filters.matches(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &self.ids) else {
                continue;
            };
            let chunk = (len / (threads * 4)).clamp(MIN_CHUNK, MAX_CHUNK);
            if len <= chunk {
                for row in 0..len {
                    f(base + row, unsafe { Q::fetch(state, row) });
                }
            } else {
                let shared = Shared(state);
                (0..len.div_ceil(chunk)).into_par_iter().for_each(|c| {
                    let state = shared;
                    let end = ((c + 1) * chunk).min(len);
                    for row in c * chunk..end {
                        f(base + row, unsafe { Q::fetch(state.0, row) });
                    }
                });
            }
            base += len;
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
            for arch in self.world.archetypes.iter() {
                let len = arch.len();
                if len == 0 || !self.filters.matches(arch) {
                    continue;
                }
                let Some(state) = Q::state(arch, &self.ids) else {
                    continue;
                };
                let state = SharedState(state);
                let chunk = (len / (threads * 4)).clamp(min_chunk, max_chunk);
                let mut completed = fill(
                    Initialized {
                        slots: Slots(initialized.slots.at(initialized.len)),
                        len: 0,
                    },
                    0,
                    len,
                    chunk,
                    &|row| f(unsafe { Q::fetch(state.get(), row) }),
                );
                initialized.len += completed.len;
                completed.len = 0;
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
        QueryIter {
            archetypes: &self.world.archetypes,
            ids: self.ids,
            filters: self.filters,
            next_arch: 0,
            state: None,
            row: 0,
            len: 0,
            _q: PhantomData,
        }
    }

    /// Number of matching entities (doesn't visit them one by one).
    pub fn count(&mut self) -> usize {
        self.world
            .archetypes
            .iter()
            .filter(|a| !a.is_empty() && self.filters.matches(a) && Q::state(a, &self.ids).is_some())
            .map(|a| a.len())
            .sum()
    }

    /// The query's item for one entity, or `None` if it's dead or doesn't match.
    pub fn get(&mut self, e: Entity) -> Option<Q::Item<'_>> {
        let loc = self.world.entities.get(e)?;
        let arch = &self.world.archetypes[loc.archetype as usize];
        if !self.filters.matches(arch) {
            return None;
        }
        let state = Q::state(arch, &self.ids)?;
        Some(unsafe { Q::fetch(state, loc.row as usize) })
    }
}

/// Iterator returned by [`Query::iter`].
pub struct QueryIter<'a, Q: QueryData> {
    archetypes: &'a [Archetype],
    ids: Q::Ids,
    filters: Filters,
    next_arch: usize,
    state: Option<Q::State>,
    row: usize,
    len: usize,
    _q: PhantomData<&'a mut Q>,
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
            let arch = self.archetypes.get(self.next_arch)?;
            self.next_arch += 1;
            self.state = if arch.is_empty() || !self.filters.matches(arch) {
                None
            } else {
                Q::state(arch, &self.ids)
            };
            self.row = 0;
            self.len = arch.len();
        }
    }
}
