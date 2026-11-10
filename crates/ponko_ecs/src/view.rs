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
//! stamp what they borrow. A view query has no [`Added`](crate::Added) or
//! [`Changed`](crate::Changed) filters; code that needs them queries the
//! world itself.
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
use std::sync::atomic::{AtomicU64, Ordering};

use crate::bundle::Component;
use crate::change::{self, Clock};
use crate::entity::Entity;
use crate::query::{Archetypes, Filters, QueryData, QueryIter, check_aliasing};
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
        }
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
}

impl<'w> WorldView<'w> {
    #[track_caller]
    fn check(&self, id: ComponentId, write: bool) {
        if !self.ids.iter().any(|&(c, w)| c == id && (w || !write)) {
            let verb = if write { "write" } else { "read" };
            panic!(
                "{} doesn't declare that it may {verb} component `{}`; add it to its declared access",
                self.label,
                self.world.component_info(id).name()
            );
        }
    }

    #[track_caller]
    fn undeclared<T>(&self, what: &str, write: bool) -> ! {
        let verb = if write { "write" } else { "read" };
        panic!(
            "{} doesn't declare that it may {verb} {what} `{}`; add it to its declared access",
            self.label,
            std::any::type_name::<T>()
        );
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
            ids,
            filters: Filters::default(),
            empty: false,
            _borrow: PhantomData,
        }
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
            arch.stamp(col, loc.row as usize, self.world.changes.stamp(), false);
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
/// [`with`](Self::with) and [`without`](Self::without) filters.
#[must_use = "a query does nothing until iterated"]
pub struct ViewQuery<'v, Q: QueryData> {
    world: &'v World,
    ids: Q::Ids,
    filters: Filters,
    /// A `with` filter names a component no entity ever had.
    empty: bool,
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

    fn archetypes(&self) -> Archetypes<'v> {
        Archetypes {
            all: if self.empty { &[] } else { &self.world.archetypes },
            indices: None,
            next: 0,
        }
    }

    /// The window of this query's change stamps: writes are stamped with
    /// the world's current tick.
    fn clock(&self) -> Clock {
        Clock::new(self.world.change_tick(), self.world.changes_since())
    }

    /// Runs `f` for every matching entity. As in [`World::query`], every
    /// entity visited counts as changed for the tracked components `Q`
    /// fetches mutably.
    #[inline]
    pub fn for_each(&mut self, mut f: impl FnMut(Q::Item<'_>)) {
        let filters = self.filters;
        let tracking = self.world.tracks_changes();
        for arch in self.archetypes() {
            let len = arch.len();
            if len == 0 || !filters.matches(arch) {
                continue;
            }
            let Some(state) = Q::state(arch, &self.ids) else {
                continue;
            };
            if tracking && arch.tracked() {
                // A view query has no `Added`/`Changed` filters, so this
                // stamps every row of the columns `Q` may write: columns
                // this view declared, which no other view touches.
                change::visit::<Q>(arch, &self.ids, &filters, self.clock());
            }
            for row in 0..len {
                // SAFETY: `row` is in bounds; the view checked that `Q` only
                // touches declared components, with declared mutability.
                f(unsafe { Q::fetch(state, row) });
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

    /// Number of matching entities.
    pub fn count(&mut self) -> usize {
        let filters = self.filters;
        self.archetypes()
            .filter(|a| !a.is_empty() && filters.matches(a) && Q::state(a, &self.ids).is_some())
            .map(|a| a.len())
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
        if arch.tracked() {
            change::visit_row::<Q>(arch, &self.ids, &self.filters, self.clock(), loc.row as usize);
        }
        // SAFETY: the row is the live entity's; access was checked.
        Some(unsafe { Q::fetch(state, loc.row as usize) })
    }
}
