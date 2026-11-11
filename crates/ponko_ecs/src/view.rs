//! Restricted views of a [`World`]: a declaration of the components and
//! resources a piece of code may touch, and a view that hands out only
//! those.
//!
//! A [`ViewAccess`] lists what may be read and written. A [`ViewState`]
//! owns one and prepares a world for it; a [`WorldView`] is the only way in.
//! It has no path to structural changes (spawn, despawn, insert, remove)
//! and checks every query, `get` and resource fetch against the
//! declaration, in every build, once per call (never per entity). Anything
//! undeclared panics, so the declaration is enforced, not trusted.
//!
//! That is what makes views safe to run side by side: two views whose
//! declarations don't overlap (no component or resource that one writes and
//! the other reads or writes) can use one world from two threads at once.
//! The world is shared (`&World`); written resources are moved out of it
//! into the view's state for the duration, and components are written
//! through the columns' own pointers, which the declarations keep apart.
//!
//! Writes through a view count for [change detection](crate::World::track_changes)
//! like any others: a query that fetches `&mut T` and [`WorldView::get_mut`]
//! stamp what they borrow. A view query takes the same typed filters as a
//! world query ([`ViewQuery::filter`]): [`Added<T>`](crate::Added) and
//! [`Changed<T>`](crate::Changed) need declared access to `T` (they read the
//! stamps its writers leave), and [`WorldView::removed`] lists removals.
//! A view can't start tracking a component (that reallocates its columns),
//! so track it beforehand with [`World::track_changes`] (in a game: in
//! setup, or from a `&mut Ctx` system); a change filter on an untracked
//! component panics.
//!
//! ```
//! use ponko_ecs::*;
//! struct Pos(f32);
//! struct Vel(f32);
//! struct Gravity(f32);
//!
//! let mut world = World::new();
//! world.insert_resource(Gravity(-1.0));
//! world.spawn((Pos(0.0), Vel(0.0)));
//!
//! let mut fall = ViewState::new(ViewAccess::new().write::<Vel>().read_resource::<Gravity>());
//! fall.run(&mut world, |mut view| {
//!     let g = view.resource::<Gravity>().0;
//!     view.query::<&mut Vel>().for_each(|v| v.0 += g);
//! });
//! assert_eq!(world.query::<&Vel>().iter().next().unwrap().0, -1.0);
//! ```

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::bundle::Component;
use crate::change::{self, Clock, Tick};
use crate::entity::Entity;
use crate::query::{Archetypes, Filters, QueryCache, QueryData, QueryFilter, QueryIter, check_aliasing, signature};
use crate::storage::ComponentId;
use crate::world::World;

pub(crate) fn next_world_uid() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// One declared component.
#[derive(Clone, Copy)]
struct ComponentDecl {
    id: TypeId,
    name: &'static str,
    write: bool,
    register: fn(&mut World) -> ComponentId,
}

/// One declared resource.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ResourceDecl {
    id: TypeId,
    name: &'static str,
    write: bool,
}

/// The components and resources a [`WorldView`] may read and write.
///
/// [`ViewAccess::new`] allows nothing; chain the builder methods. Declaring
/// a write also allows reading.
#[derive(Clone, Default)]
pub struct ViewAccess {
    components: Vec<ComponentDecl>,
    resources: Vec<ResourceDecl>,
}

impl PartialEq for ViewAccess {
    fn eq(&self, other: &Self) -> bool {
        let key = |c: &ComponentDecl| (c.id, c.write);
        self.components.len() == other.components.len()
            && self.components.iter().map(key).eq(other.components.iter().map(key))
            && self.resources == other.resources
    }
}

impl Eq for ViewAccess {}

impl std::fmt::Debug for ViewAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let list = |w: bool| if w { "write" } else { "read" };
        f.debug_struct("ViewAccess")
            .field(
                "components",
                &self
                    .components
                    .iter()
                    .map(|c| (c.name, list(c.write)))
                    .collect::<Vec<_>>(),
            )
            .field(
                "resources",
                &self
                    .resources
                    .iter()
                    .map(|r| (r.name, list(r.write)))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

fn register<T: Component>(world: &mut World) -> ComponentId {
    world.register::<T>()
}

impl ViewAccess {
    /// Allows nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn component<T: Component>(mut self, write: bool) -> Self {
        let id = TypeId::of::<T>();
        match self.components.iter_mut().find(|c| c.id == id) {
            Some(c) => c.write |= write,
            None => self.components.push(ComponentDecl {
                id,
                name: std::any::type_name::<T>(),
                write,
                register: register::<T>,
            }),
        }
        self
    }

    fn resource<R: 'static>(mut self, write: bool) -> Self {
        let id = TypeId::of::<R>();
        match self.resources.iter_mut().find(|r| r.id == id) {
            Some(r) => r.write |= write,
            None => self.resources.push(ResourceDecl {
                id,
                name: std::any::type_name::<R>(),
                write,
            }),
        }
        self
    }

    /// Allows reading component `T`.
    #[must_use]
    pub fn read<T: Component>(self) -> Self {
        self.component::<T>(false)
    }

    /// Allows reading and writing component `T`.
    #[must_use]
    pub fn write<T: Component>(self) -> Self {
        self.component::<T>(true)
    }

    /// Allows reading resource `R`.
    #[must_use]
    pub fn read_resource<R: Send + Sync + 'static>(self) -> Self {
        self.resource::<R>(false)
    }

    /// Allows reading, writing, inserting and removing resource `R`.
    #[must_use]
    pub fn write_resource<R: Send + Sync + 'static>(self) -> Self {
        self.resource::<R>(true)
    }

    /// The declared components: type, name, written.
    pub fn components(&self) -> impl Iterator<Item = (TypeId, &'static str, bool)> + '_ {
        self.components.iter().map(|c| (c.id, c.name, c.write))
    }

    /// The declared resources: type, name, written.
    pub fn resources(&self) -> impl Iterator<Item = (TypeId, &'static str, bool)> + '_ {
        self.resources.iter().map(|r| (r.id, r.name, r.write))
    }

    /// `true` if views of the two declarations must not exist at the same
    /// time: one writes a component or resource the other reads or writes.
    #[must_use]
    pub fn overlaps(&self, other: &ViewAccess) -> bool {
        let c = self
            .components
            .iter()
            .any(|a| other.components.iter().any(|b| a.id == b.id && (a.write || b.write)));
        let r = self
            .resources
            .iter()
            .any(|a| other.resources.iter().any(|b| a.id == b.id && (a.write || b.write)));
        c || r
    }

    fn resource_decl(&self, id: TypeId) -> Option<&ResourceDecl> {
        self.resources.iter().find(|r| r.id == id)
    }
}

impl Drop for ViewState {
    fn drop(&mut self) {
        // Not while unwinding: a panicking system's state is dropped then,
        // and a second panic would abort.
        if !std::thread::panicking() {
            debug_assert!(
                !self.is_prepared(),
                "ViewState dropped while prepared: {}'s lent resources are lost; call finish first",
                self.label
            );
        }
    }
}

/// A written resource, moved out of the world while it is viewed.
struct Lent {
    id: TypeId,
    value: Option<Box<dyn Any + Send + Sync>>,
}

/// A [`ViewAccess`] and what viewing a world with it needs: the world's ids
/// for its components and the written resources while they are lent out.
///
/// Keep one per system and reuse it: after the first run, preparing and
/// finishing allocate nothing.
pub struct ViewState {
    access: ViewAccess,
    /// Shown in panics: usually the system's name.
    label: &'static str,
    /// The prepared world's ids for each declared component (with `write`).
    ids: Vec<(ComponentId, bool)>,
    /// One per declared written resource.
    lent: Vec<Lent>,
    /// The world prepared for, 0 when not prepared.
    world: u64,
    /// Unique per state: how a [`ViewGroup`] recognizes the states it checked.
    id: u64,
    /// What change filters compare against, if not the world's
    /// [`changes_since`](World::changes_since).
    since: Option<Tick>,
    /// The tick this state's views write in and report, if not the
    /// world's current one.
    now: Option<Tick>,
}

fn next_state_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl ViewState {
    /// State for views with `access`.
    #[must_use]
    pub fn new(access: ViewAccess) -> Self {
        ViewState {
            access,
            label: "this view",
            ids: Vec::new(),
            lent: Vec::new(),
            world: 0,
            id: next_state_id(),
            since: None,
            now: None,
        }
    }

    /// Makes [`Added`](crate::Added) and [`Changed`](crate::Changed) filters
    /// and [`WorldView::removed`] in this state's views report what happened
    /// after `since`, instead of after the world's
    /// [`changes_since`](World::changes_since); `None` goes back to the
    /// world's. Views that share a world each need their own: the world has
    /// only one `changes_since`.
    pub fn set_changes_since(&mut self, since: Option<Tick>) {
        self.since = since;
    }

    /// Makes this state's views write in change tick `tick`: their writes
    /// are stamped with it and [`WorldView::change_tick`] returns it,
    /// instead of the world's current tick; `None` goes back to the
    /// world's. For views that share a world but stand for work that would
    /// otherwise happen in ticks of its own, one after another: advance the
    /// world past those ticks first, then give each view its tick. A tick
    /// after the world's current one counts as the current one.
    ///
    /// ```
    /// # use ponko_ecs::*;
    /// struct Pos(f32);
    /// let mut world = World::new();
    /// world.track_changes::<Pos>();
    /// world.spawn((Pos(0.0),));
    /// let first = world.advance_change_tick();
    /// let second = world.advance_change_tick();
    /// let mut state = ViewState::new(ViewAccess::new().write::<Pos>());
    /// // Write as if in the tick `second`, although the world is past it.
    /// state.set_change_tick(Some(second));
    /// state.run(&mut world, |mut view| {
    ///     assert_eq!(view.change_tick(), second);
    ///     view.query::<&mut Pos>().for_each(|p| p.0 += 1.0);
    /// });
    /// let changed = |world: &mut World, since| world.query::<&Pos>().filter::<Changed<Pos>>().since(since).count();
    /// assert_eq!(changed(&mut world, first), 1);
    /// assert_eq!(changed(&mut world, second), 0);
    /// ```
    pub fn set_change_tick(&mut self, tick: Option<Tick>) {
        self.now = tick;
    }

    /// Names the code using the view in panic messages (a system's name).
    #[must_use]
    pub fn labeled(mut self, label: &'static str) -> Self {
        self.label = label;
        self
    }

    /// The declaration.
    #[must_use]
    pub fn access(&self) -> &ViewAccess {
        &self.access
    }

    /// `true` between [`prepare`](Self::prepare) and [`finish`](Self::finish).
    #[must_use]
    pub fn is_prepared(&self) -> bool {
        self.world != 0
    }

    /// Gets `world` ready to be viewed: registers the declared components
    /// and moves the declared written resources out of it (a missing one
    /// stays missing). Until [`finish`](Self::finish), those resources are
    /// absent from the world itself: dropping a prepared state loses them
    /// (builds with debug assertions panic when that happens).
    /// [`run`](Self::run) always finishes.
    ///
    /// # Panics
    /// If the state is already prepared.
    pub fn prepare(&mut self, world: &mut World) {
        assert!(
            !self.is_prepared(),
            "ViewState::prepare: {} is already prepared; call finish first",
            self.label
        );
        // The world is held exclusively here: let its archetype index
        // take in what the last views' queries asked for.
        world.query_matches.settle_for_views(&world.archetypes);
        self.ids.clear();
        for c in &self.access.components {
            self.ids.push(((c.register)(world), c.write));
        }
        self.lent.clear();
        for r in self.access.resources.iter().filter(|r| r.write) {
            self.lent.push(Lent {
                id: r.id,
                value: world.lend_resource(r.id),
            });
        }
        self.world = world.uid;
    }

    /// Puts the written resources back into `world` (including ones the
    /// view inserted; ones it removed stay removed).
    ///
    /// # Panics
    /// If the state was prepared for another world. Does nothing if it
    /// isn't prepared.
    pub fn finish(&mut self, world: &mut World) {
        if !self.is_prepared() {
            return;
        }
        assert_eq!(
            self.world, world.uid,
            "ViewState::finish: {} was prepared for another world",
            self.label
        );
        for lent in &mut self.lent {
            if let Some(value) = lent.value.take() {
                world.return_resource(lent.id, value);
            }
        }
        self.world = 0;
    }

    /// Prepares `world`, runs `f` with a view of it, and finishes, also
    /// when `f` panics. The safe way to use a view on its own.
    pub fn run<T>(&mut self, world: &mut World, f: impl FnOnce(WorldView<'_>) -> T) -> T {
        struct Finish<'a> {
            state: &'a mut ViewState,
            world: &'a mut World,
        }
        impl Drop for Finish<'_> {
            fn drop(&mut self) {
                self.state.finish(self.world);
            }
        }
        self.prepare(world);
        let guard = Finish { state: self, world };
        let world: &World = guard.world;
        // SAFETY: the guard holds the only (mutable) borrow of the world;
        // this shared reborrow and the view are the only access until it drops.
        let view = unsafe { guard.state.view_unchecked_inner(world) };
        f(view)
    }

    /// A view of a prepared `world`, which may be shared with other views.
    ///
    /// # Safety
    /// While the view lives:
    /// - no other code may write a component this declaration reads or
    ///   writes, or read one it writes (other views of non-overlapping
    ///   declarations, see [`ViewAccess::overlaps`], are fine);
    /// - nothing may change the world through `&mut World`.
    ///
    /// Resources need no promise: written ones are moved out by
    /// [`prepare`](Self::prepare), so overlapping resource access only
    /// finds them missing.
    ///
    /// # Panics
    /// If the state isn't prepared, or was prepared for another world.
    pub unsafe fn view<'w>(&'w mut self, world: &'w World) -> WorldView<'w> {
        // SAFETY: forwarded caller contract.
        unsafe { self.view_unchecked_inner(world) }
    }

    /// # Safety
    /// As [`view`](Self::view).
    unsafe fn view_unchecked_inner<'w>(&'w mut self, world: &'w World) -> WorldView<'w> {
        assert!(
            self.is_prepared(),
            "ViewState::view: {} isn't prepared; call prepare first",
            self.label
        );
        assert_eq!(
            self.world, world.uid,
            "ViewState::view: {} was prepared for another world",
            self.label
        );
        WorldView {
            world,
            access: &self.access,
            label: self.label,
            ids: &self.ids,
            lent: &mut self.lent,
            since: self.since.unwrap_or(world.changes_since()),
            now: self.now.map_or(world.change_tick(), |t| t.min(world.change_tick())),
        }
    }
}

/// A restricted view of a [`World`]: only what its [`ViewAccess`] declared,
/// and no structural changes. From [`ViewState::run`] or [`ViewState::view`].
///
/// Every fetch checks the declaration once per call; anything undeclared
/// panics with the label and what was missing.
pub struct WorldView<'w> {
    world: &'w World,
    access: &'w ViewAccess,
    label: &'static str,
    ids: &'w [(ComponentId, bool)],
    lent: &'w mut [Lent],
    /// What change filters and `removed` compare against by default.
    since: Tick,
    /// The tick this view writes in.
    now: Tick,
}

/// Panics unless `decl` allows `write` (or reading) component `id`.
#[track_caller]
fn check_declared(world: &World, label: &str, decl: &[(ComponentId, bool)], id: ComponentId, write: bool) {
    if !decl.iter().any(|&(c, w)| c == id && (w || !write)) {
        let verb = if write { "write" } else { "read" };
        panic!(
            "{label} doesn't declare that it may {verb} component `{}`; add it to its declared access",
            world.component_info(id).name()
        );
    }
}

#[cold]
#[track_caller]
fn undeclared<T>(label: &str, what: &str, write: bool) -> ! {
    let verb = if write { "write" } else { "read" };
    panic!(
        "{label} doesn't declare that it may {verb} {what} `{}`; add it to its declared access",
        std::any::type_name::<T>()
    );
}

/// Builds a [`ViewQuery`]'s filters from a shared world: nothing is
/// registered or starts being tracked, and change filters are checked
/// against the view's declaration.
///
/// Public but not exported, like [`Filters`]: it seals the filter traits.
#[doc(hidden)]
pub struct SharedFilters<'a> {
    pub(crate) world: &'a World,
    label: &'static str,
    decl: &'a [(ComponentId, bool)],
    filters: Filters,
    /// A filter can never hold: a `with` component no entity ever had.
    empty: bool,
}

impl SharedFilters<'_> {
    pub(crate) fn with(&mut self, id: Option<ComponentId>) {
        match id {
            Some(id) => self.filters.add_with(id),
            None => self.empty = true,
        }
    }

    pub(crate) fn without(&mut self, id: Option<ComponentId>) {
        if let Some(id) = id {
            self.filters.add_without(id);
        }
    }

    /// `T`'s id for an `Added`/`Changed` filter named `filter`: declared
    /// (read is enough) and already tracked.
    #[track_caller]
    fn tracked<T: Component>(&self, filter: &str) -> ComponentId {
        let Some(id) = self.world.component_id::<T>() else {
            undeclared::<T>(self.label, "component", false)
        };
        check_declared(self.world, self.label, self.decl, id, false);
        if !self.world.component_info(id).tracks_changes() {
            let name = std::any::type_name::<T>();
            panic!(
                "{}: `{filter}<{name}>` needs `{name}`'s changes tracked, and a view can't start tracking; \
                 call `world.track_changes::<{name}>()` before (in a game: in setup or a `&mut Ctx` system)",
                self.label
            );
        }
        id
    }

    #[track_caller]
    pub(crate) fn tick<T: Component>(&mut self, filter: &str, added: bool) {
        let id = self.tracked::<T>(filter);
        self.filters.add_tick(id, added);
    }

    /// Resolve each arm independently: an unknown `With` makes only that
    /// arm false, while an unknown `Without` contributes no requirement.
    pub(crate) fn or_arm<F: QueryFilter>(&self) -> Filters {
        let mut arm = SharedFilters {
            world: self.world,
            label: self.label,
            decl: self.decl,
            filters: Filters::default(),
            empty: false,
        };
        F::add_shared(&mut arm);
        if arm.empty {
            Filters::never()
        } else {
            arm.filters.into_clauses()
        }
    }

    pub(crate) fn append_clauses(&mut self, filters: &Filters) {
        self.filters.append_clauses(filters);
    }
}

impl<'w> WorldView<'w> {
    #[track_caller]
    fn check(&self, id: ComponentId, write: bool) {
        check_declared(self.world, self.label, self.ids, id, write);
    }

    #[track_caller]
    fn undeclared<T>(&self, what: &str, write: bool) -> ! {
        undeclared::<T>(self.label, what, write)
    }

    /// The declared id of `T`, checked for `write`.
    #[track_caller]
    fn id_of<T: Component>(&self, write: bool) -> ComponentId {
        match self.world.component_id::<T>() {
            Some(id) => {
                self.check(id, write);
                id
            }
            None => self.undeclared::<T>("component", write),
        }
    }

    /// A query over the declared components: `Q` is what
    /// [`World::query`] takes. Panics if `Q` touches a component the
    /// declaration doesn't allow (with the mutability it asks for), or if
    /// it asks for one component twice and once mutably.
    #[track_caller]
    pub fn query<Q: QueryData>(&mut self) -> ViewQuery<'_, Q> {
        let Some(ids) = Q::ids_in(self.world) else {
            panic!(
                "{}: query `{}` uses a component its declared access doesn't name",
                self.label,
                std::any::type_name::<Q>()
            );
        };
        check_aliasing::<Q>(self.world, &ids);
        Q::access(&ids, &mut |id, write, _| self.check(id, write));
        ViewQuery {
            world: self.world,
            label: self.label,
            decl: self.ids,
            ids,
            filters: Filters::default(),
            empty: false,
            cache: QueryCache::Auto,
            since: None,
            default_since: self.since,
            now: self.now,
            _borrow: PhantomData,
        }
    }

    /// The entities that lost component `T` (declared read or write)
    /// since the world's [`changes_since`](World::changes_since) (or the
    /// state's own, see [`ViewState::set_changes_since`]), oldest first: [`World::removed`] through the view. In a game, that is since
    /// this system last ran.
    ///
    /// # Panics
    /// If `T` is undeclared or isn't tracked.
    #[track_caller]
    #[must_use]
    pub fn removed<T: Component>(&self) -> &'w [Entity] {
        self.removed_since::<T>(self.since)
    }

    /// [`removed`](Self::removed) after a tick the caller kept.
    ///
    /// # Panics
    /// If `T` is undeclared or isn't tracked.
    #[track_caller]
    #[must_use]
    pub fn removed_since<T: Component>(&self, since: Tick) -> &'w [Entity] {
        let world: &'w World = self.world;
        let _ = self.id_of::<T>(false);
        world.removed_since::<T>(since)
    }

    /// The change tick this view writes in: the world's current one (see
    /// [`World::change_tick`]) unless the state has its own
    /// ([`ViewState::set_change_tick`]). Keep it to pass to
    /// [`ViewQuery::since`] or [`removed_since`](Self::removed_since) next
    /// time.
    #[must_use]
    pub fn change_tick(&self) -> Tick {
        self.now
    }

    /// Component `T` of `e` (declared read or write), or `None` if `e` is
    /// dead or has none.
    #[track_caller]
    #[must_use]
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        let id = self.id_of::<T>(false);
        let ptr = self.world.locate(e, id)?;
        // SAFETY: `T` is declared, so no concurrent view writes it (the
        // `view` contract); the pointer is to a live `T` in its column.
        Some(unsafe { &*ptr.cast::<T>() })
    }

    /// Component `T` of `e` to change (declared write).
    #[track_caller]
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T> {
        let id = self.id_of::<T>(true);
        let loc = self.world.entities.get(e)?;
        let arch = &self.world.archetypes[loc.archetype as usize];
        let col = arch.column_index(id)?;
        if arch.tracked() {
            // A mutable borrow counts as a change, as with `World::get_mut`.
            // The stamps belong to the column this view may write.
            arch.stamp(col, loc.row as usize, self.now.0 as u32, false);
        }
        let ptr = arch.columns[col].ptr(loc.row as usize);
        // SAFETY: `T` is declared written, so nothing else touches it (the
        // `view` contract); `&mut self` keeps this the only borrow from
        // this view. Columns are raw allocations, so writing through their
        // pointer from a shared world is allowed.
        Some(unsafe { &mut *ptr.cast::<T>() })
    }

    /// `true` if `e` is alive and has component `T`. Needs no declaration:
    /// it reads which components an entity has, not their values.
    #[must_use]
    pub fn has<T: Component>(&self, e: Entity) -> bool {
        self.world.has::<T>(e)
    }

    /// `true` if `e` is alive.
    #[must_use]
    pub fn is_alive(&self, e: Entity) -> bool {
        self.world.is_alive(e)
    }

    /// Number of living entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.world.len()
    }

    /// `true` if there are no entities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.world.is_empty()
    }

    fn lent(&self, id: TypeId) -> Option<&Lent> {
        self.lent.iter().find(|l| l.id == id)
    }

    fn lent_mut(&mut self, id: TypeId) -> Option<&mut Lent> {
        self.lent.iter_mut().find(|l| l.id == id)
    }

    /// The resource `R` (declared read or write), if there is one.
    #[track_caller]
    #[must_use]
    pub fn get_resource<R: Send + Sync + 'static>(&self) -> Option<&R> {
        let id = TypeId::of::<R>();
        match self.access.resource_decl(id) {
            Some(d) if d.write => self.lent(id)?.value.as_ref().map(|v| v.downcast_ref().unwrap()),
            Some(_) => self.world.get_resource::<R>(),
            None => self.undeclared::<R>("resource", false),
        }
    }

    /// The resource `R` (declared read or write).
    ///
    /// # Panics
    /// If it is undeclared or missing.
    #[track_caller]
    #[must_use]
    pub fn resource<R: Send + Sync + 'static>(&self) -> &R {
        self.get_resource()
            .unwrap_or_else(|| panic!("{}: no resource of type `{}`", self.label, std::any::type_name::<R>()))
    }

    #[track_caller]
    fn written<R: 'static>(&mut self) -> &mut Lent {
        let id = TypeId::of::<R>();
        if !self.access.resource_decl(id).is_some_and(|d| d.write) {
            self.undeclared::<R>("resource", true);
        }
        self.lent_mut(id).expect("declared written resources are lent")
    }

    /// The resource `R` to change (declared write), if there is one.
    #[track_caller]
    pub fn get_resource_mut<R: Send + Sync + 'static>(&mut self) -> Option<&mut R> {
        self.written::<R>().value.as_mut().map(|v| v.downcast_mut().unwrap())
    }

    /// The resource `R` to change (declared write).
    ///
    /// # Panics
    /// If it is undeclared or missing.
    #[track_caller]
    pub fn resource_mut<R: Send + Sync + 'static>(&mut self) -> &mut R {
        let label = self.label;
        self.get_resource_mut()
            .unwrap_or_else(|| panic!("{label}: no resource of type `{}`", std::any::type_name::<R>()))
    }

    /// Stores resource `R` (declared write), replacing any existing one.
    #[track_caller]
    pub fn insert_resource<R: Send + Sync + 'static>(&mut self, value: R) {
        self.written::<R>().value = Some(Box::new(value));
    }

    /// Removes and returns resource `R` (declared write).
    #[track_caller]
    pub fn remove_resource<R: Send + Sync + 'static>(&mut self) -> Option<R> {
        self.written::<R>().value.take().map(|v| *v.downcast().unwrap())
    }
}

/// A query from a [`WorldView`]: [`for_each`](Self::for_each),
/// [`iter`](Self::iter), [`count`](Self::count) and [`get`](Self::get), with
/// [`with`](Self::with), [`without`](Self::without) and typed
/// [`filter`](Self::filter)s, [`Added`](crate::Added) and
/// [`Changed`](crate::Changed) included.
#[must_use = "a query does nothing until iterated"]
pub struct ViewQuery<'v, Q: QueryData> {
    world: &'v World,
    label: &'static str,
    /// The view's declared components (with `write`), for change filters.
    decl: &'v [(ComponentId, bool)],
    ids: Q::Ids,
    filters: Filters,
    /// A filter can never hold: it names a component no entity ever had.
    empty: bool,
    /// Whether this query uses the world's archetype index.
    cache: QueryCache,
    /// What `Added`/`Changed` filters compare against, if not the world's
    /// [`changes_since`](World::changes_since).
    since: Option<Tick>,
    /// The view's own default for `since`.
    default_since: Tick,
    /// The tick the view writes in.
    now: Tick,
    /// Borrows the view mutably: one query at a time, like `World::query`.
    _borrow: PhantomData<&'v mut ()>,
}

impl<'v, Q: QueryData> ViewQuery<'v, Q> {
    /// Only entities that also have `T`. Needs no declaration.
    pub fn with<T: Component>(mut self) -> Self {
        match self.world.component_id::<T>() {
            Some(id) => self.filters.add_with(id),
            None => self.empty = true,
        }
        self
    }

    /// Skips entities that have `T`. Needs no declaration.
    pub fn without<T: Component>(mut self) -> Self {
        if let Some(id) = self.world.component_id::<T>() {
            self.filters.add_without(id);
        }
        self
    }

    /// Only entities that pass the typed filter `F`, as
    /// [`Query::filter`](crate::Query::filter): [`With`](crate::With),
    /// [`Without`](crate::Without), [`Or`](crate::Or),
    /// [`Added`](crate::Added), [`Changed`](crate::Changed), or a tuple of
    /// them that must all hold.
    ///
    /// `With`, `Without` and `Or` need no declaration. `Added<T>` and
    /// `Changed<T>` read the stamps that writers of `T` leave, so they need
    /// `T` declared (read is enough), and `T` must already be tracked: a
    /// view can't start tracking it. They report what changed since the
    /// world's [`changes_since`](World::changes_since) (in a game: since
    /// this system last ran) unless [`since`](Self::since) says otherwise.
    ///
    /// ```
    /// use ponko_ecs::*;
    /// struct Health(i32);
    ///
    /// let mut world = World::new();
    /// world.track_changes::<Health>();
    /// world.spawn((Health(10),));
    /// let seen = world.advance_change_tick();
    /// let hurt = world.spawn((Health(10),));
    /// world.get_mut::<Health>(hurt).unwrap().0 = 5;
    /// world.set_changes_since(seen);
    ///
    /// let mut state = ViewState::new(ViewAccess::new().read::<Health>());
    /// let changed = state.run(&mut world, |mut view| {
    ///     view.query::<Entity>().filter::<Changed<Health>>().iter().collect::<Vec<_>>()
    /// });
    /// assert_eq!(changed, [hurt]);
    /// ```
    ///
    /// # Panics
    /// If `Added<T>`/`Changed<T>` names an undeclared or untracked `T`, and
    /// on the same limits as [`Query::filter`](crate::Query::filter).
    #[track_caller]
    pub fn filter<F: QueryFilter>(mut self) -> Self {
        let mut cx = SharedFilters {
            world: self.world,
            label: self.label,
            decl: self.decl,
            filters: self.filters,
            empty: self.empty,
        };
        F::add_shared(&mut cx);
        self.filters = cx.filters;
        self.empty = cx.empty;
        self
    }

    /// Makes [`Added`](crate::Added) and [`Changed`](crate::Changed)
    /// filters report what happened after `tick` (see
    /// [`Query::since`](crate::Query::since)).
    pub fn since(mut self, tick: Tick) -> Self {
        self.since = Some(tick);
        self
    }

    /// Index this query's signature in the world's archetype index, as
    /// [`Query::cached`](crate::Query::cached) asks. A view shares the
    /// world, so it can't build the index itself: this query scans, and the
    /// index is built the next time the world is held exclusively (before
    /// the next view runs).
    pub fn cached(mut self) -> Self {
        self.cache = QueryCache::Always;
        self
    }

    /// Never use the world's archetype index for this query, as
    /// [`Query::uncached`](crate::Query::uncached) asks.
    pub fn uncached(mut self) -> Self {
        self.cache = QueryCache::Never;
        self
    }

    /// The archetypes to visit: every one, or the candidates the world's
    /// index already holds for this query. The index is only read here
    /// (other views may be reading it on other threads); a signature it
    /// doesn't hold yet is scanned.
    fn archetypes(&self) -> Archetypes<'v> {
        let world = self.world;
        if self.empty {
            return Archetypes {
                all: &[],
                indices: None,
                next: 0,
            };
        }
        let index = &world.query_matches;
        let indices = if self.cache == QueryCache::Never || world.archetypes.len() < index.min_archetypes() {
            None
        } else {
            let key = signature::<Q>(&self.ids, &self.filters);
            index.shared_candidates(&world.archetypes, &key, self.cache)
        };
        Archetypes {
            all: &world.archetypes,
            indices: indices.map(<[u32]>::iter),
            next: 0,
        }
    }

    /// The window of this query's change stamps: writes are stamped with
    /// the view's tick.
    fn clock(&self) -> Clock {
        Clock::new(self.now, self.since.unwrap_or(self.default_since))
    }

    /// Runs `f` for every matching entity. As in [`World::query`], every
    /// entity visited counts as changed for the tracked components `Q`
    /// fetches mutably (only the rows a change filter lets through).
    #[inline]
    pub fn for_each(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        let filters = self.filters;
        let tracking = self.world.tracks_changes();
        let clock = self.clock();
        for arch in self.archetypes() {
            let len = arch.len();
            if len == 0 || !filters.matches(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &self.ids) else {
                continue;
            };
            // Stamps only go to the columns `Q` may write: columns this
            // view declared written, which no other view touches. Change
            // filters read the stamps of declared components, which no
            // other view writes.
            let mut runs = if tracking && arch.tracked() {
                change::visit::<Q>(arch, &self.ids, &filters, clock)
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
                for row in start..end {
                    // SAFETY: `row` is in bounds; the view checked that `Q`
                    // only touches declared components, with declared
                    // mutability.
                    f(unsafe { Q::fetch(state, row) });
                }
                if runs.is_none() {
                    break;
                }
            }
        }
    }

    /// An iterator over the matching entities.
    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        QueryIter {
            archetypes: self.archetypes(),
            ids: self.ids,
            filters: self.filters,
            state: None,
            row: 0,
            len: 0,
            clock: self.clock(),
            tracking: self.world.tracks_changes(),
            filtered: None,
            _q: PhantomData,
        }
    }

    /// Number of matching entities. Doesn't count as a change to anything.
    pub fn count(&mut self) -> usize {
        let filters = self.filters;
        let tracking = self.world.tracks_changes();
        let clock = self.clock();
        self.archetypes()
            .filter(|a| !a.is_empty() && filters.matches(a) && Q::state(a, &self.ids).is_some())
            .map(|a| {
                if tracking && a.tracked() {
                    change::count(a, &filters, clock)
                } else {
                    a.len()
                }
            })
            .sum()
    }

    /// The item for `e`, or `None` if it's dead or doesn't match.
    pub fn get(&mut self, e: Entity) -> Option<Q::Item<'_>> {
        if self.empty {
            return None;
        }
        let loc = self.world.entities.get(e)?;
        let arch = &self.world.archetypes[loc.archetype as usize];
        if !self.filters.matches(arch) {
            return None;
        }
        let state = Q::state(arch, &self.ids)?;
        if arch.tracked() && !change::visit_row::<Q>(arch, &self.ids, &self.filters, self.clock(), loc.row as usize) {
            return None;
        }
        // SAFETY: the row is the live entity's; access was checked.
        Some(unsafe { Q::fetch(state, loc.row as usize) })
    }
}

/// Proof that a list of [`ViewState`]s may view one world at the same time:
/// their declarations don't overlap pairwise ([`ViewAccess::overlaps`]).
///
/// Check once with [`new`](Self::new), then [`run`](Self::run) the same
/// states as often as needed, on several threads or one after another. This
/// is the safe way to share a world between views: `run` recognizes the
/// states it checked and refuses any other.
///
/// ```
/// use ponko_ecs::*;
/// struct Pos(f32);
/// struct Health(i32);
///
/// let mut world = World::new();
/// world.spawn((Pos(0.0), Health(3)));
///
/// // Each task: its view state, and whatever else it needs.
/// let mut tasks = vec![
///     (ViewState::new(ViewAccess::new().write::<Pos>()), 1.0f32),
///     (ViewState::new(ViewAccess::new().write::<Health>()), 2.0f32),
/// ];
/// let group = ViewGroup::new(tasks.iter().map(|t| &t.0)).unwrap();
/// group.run(
///     &mut world,
///     &mut tasks,
///     true,
///     |task| (&mut task.0, &mut task.1),
///     |amount, mut view| {
///         if *amount == 1.0 {
///             view.query::<&mut Pos>().for_each(|p| p.0 += *amount);
///         } else {
///             view.query::<&mut Health>().for_each(|h| h.0 -= *amount as i32);
///         }
///     },
/// );
/// assert_eq!(world.query::<&Pos>().iter().next().unwrap().0, 1.0);
/// assert_eq!(world.query::<&Health>().iter().next().unwrap().0, 1);
/// ```
#[derive(Clone, Debug)]
pub struct ViewGroup {
    /// The checked states, in order.
    ids: Vec<u64>,
}

impl ViewGroup {
    /// Checks `states`. `Err((i, j))` names the first two whose
    /// declarations overlap (or that are the same state twice).
    pub fn new<'a>(states: impl IntoIterator<Item = &'a ViewState>) -> Result<Self, (usize, usize)> {
        let states: Vec<&ViewState> = states.into_iter().collect();
        for (j, b) in states.iter().enumerate() {
            for (i, a) in states[..j].iter().enumerate() {
                if a.id == b.id || a.access.overlaps(&b.access) {
                    return Err((i, j));
                }
            }
        }
        Ok(ViewGroup {
            ids: states.iter().map(|s| s.id).collect(),
        })
    }

    /// How many states the group checked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// `true` for a group of no states.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Runs `f` once per item with a view of `world`, all sharing it.
    ///
    /// `split` gives each item's view state (the group's states, in the
    /// order they were checked) and the rest of the item, which `f` gets
    /// next to the view. With `parallel` (and the `parallel` feature, and
    /// more than one item) the calls run on rayon's threads; otherwise one
    /// after another on this thread, in order. Either way every state is
    /// prepared before the first call and finished after the last, in
    /// order: what the items see doesn't depend on which of the two it was.
    ///
    /// A panic in `f` doesn't stop the other calls, on threads or in order:
    /// every item still gets its call, every state is finished (lent
    /// resources go back to the world), and then the panic of the first
    /// item in order that panicked is passed on. So a panic leaves the same
    /// world and the same items behind whichever way the group ran, and
    /// which panic the caller sees doesn't depend on thread timing.
    ///
    /// # Panics
    /// If `items` aren't the states this group checked, in that order, or
    /// one of them is already prepared. With the first panic of `f`, as
    /// above.
    pub fn run<T, U>(
        &self,
        world: &mut World,
        items: &mut [T],
        parallel: bool,
        split: impl Fn(&mut T) -> (&mut ViewState, &mut U) + Sync,
        f: impl Fn(&mut U, WorldView<'_>) + Sync,
    ) where
        T: Send,
    {
        /// Finishes every state, also while unwinding.
        struct Finish<'a, T, U, S: Fn(&mut T) -> (&mut ViewState, &mut U)> {
            items: &'a mut [T],
            world: &'a mut World,
            split: &'a S,
        }
        impl<T, U, S: Fn(&mut T) -> (&mut ViewState, &mut U)> Drop for Finish<'_, T, U, S> {
            fn drop(&mut self) {
                for item in self.items.iter_mut() {
                    let state = (self.split)(item).0;
                    // Only what this run prepared: a state prepared for
                    // another world isn't ours to finish.
                    if state.world == self.world.uid {
                        state.finish(self.world);
                    }
                }
            }
        }

        assert_eq!(
            items.len(),
            self.ids.len(),
            "ViewGroup::run: the group checked {} states, not {}",
            self.ids.len(),
            items.len()
        );
        let guard = Finish {
            items,
            world,
            split: &split,
        };
        for (i, item) in guard.items.iter_mut().enumerate() {
            let state = split(item).0;
            assert_eq!(
                state.id, self.ids[i],
                "ViewGroup::run: item {i} isn't the state this group checked"
            );
            state.prepare(guard.world);
        }
        let world: &World = guard.world;
        let ids = &self.ids;
        // The panic of the first item in order that panicked. Only locked
        // when a call panics.
        let first_panic: Mutex<Option<(usize, Box<dyn Any + Send>)>> = Mutex::new(None);
        let one = |(i, item): (usize, &mut T)| {
            let (state, rest) = split(item);
            // Checked again: `split` decides what it returns on every call.
            assert_eq!(
                state.id, ids[i],
                "ViewGroup::run: item {i} isn't the state this group checked"
            );
            // SAFETY: `guard` holds the only (mutable) borrow of the world,
            // so nothing changes it through `&mut World` and only these
            // views read or write it. Each view comes from a different
            // state (ids are unique and each item must have its own), and
            // the states' declarations don't overlap pairwise (checked in
            // `new`; a state's declaration never changes), so no view
            // writes a component another reads or writes. `f` gets nothing
            // of the world but its view.
            let view = unsafe { state.view(world) };
            // The other items still run: see the docs. The item's state is
            // finished by `guard` like every other.
            if let Err(panic) = catch_unwind(AssertUnwindSafe(|| f(rest, view))) {
                let mut first = first_panic.lock().unwrap_or_else(PoisonError::into_inner);
                if first.as_ref().is_none_or(|(at, _)| i < *at) {
                    *first = Some((i, panic));
                }
            }
        };
        #[cfg(feature = "parallel")]
        let threads = parallel && guard.items.len() > 1;
        #[cfg(not(feature = "parallel"))]
        let threads = {
            let _ = parallel;
            false
        };
        if threads {
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                guard.items.par_iter_mut().enumerate().for_each(one);
            }
        } else {
            guard.items.iter_mut().enumerate().for_each(one);
        }
        // Finish the states first, then pass the panic on.
        drop(guard);
        let first = first_panic.into_inner().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, panic)) = first {
            resume_unwind(panic);
        }
    }
}
