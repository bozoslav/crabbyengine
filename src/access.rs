//! Declared system access: what a system reads and writes, and which
//! declared systems could run together.
//!
//! **Status: prototype.** Declarations are opt-in metadata. Systems still
//! run one after another, in phase and registration order, each with the
//! whole [`Ctx`]. Nothing here runs systems in parallel; a future scheduler
//! can use the metadata only once the rules below are enforced for it.
//!
//! # The model
//!
//! A [`SystemAccess`] lists:
//!
//! - **components** read or written (`&T` / `&mut T` in queries, `get`, `get_mut`),
//! - **resources** read or written,
//! - **events** read or sent ([`Events`](crate::Events)),
//! - whether the system records **commands** ([`Commands`](crate::Commands)).
//!
//! Or it is [`exclusive`](SystemAccess::exclusive): it may use anything, and
//! conflicts with every other system. A system registered without a
//! declaration (every `.sim_system(...)`, `.system(...)`, built-in, foreign
//! and hot-patched system) is exclusive, so existing games behave exactly
//! as before.
//!
//! # When two declarations conflict
//!
//! Two systems conflict, and so must keep their order, when:
//!
//! - either is exclusive;
//! - both touch the same component or resource and at least one writes it;
//! - one sends an event type the other reads, or both send it (the order
//!   of sends is visible to readers, so it must stay deterministic);
//! - either records commands and the other touches any component, resource
//!   or commands. Commands are applied after each system, so a later system
//!   sees them; running the two together would change what it sees. This is
//!   the conservative barrier; finer rules belong to the command and event
//!   barrier design.
//!
//! Systems that don't conflict *could* run together without changing what
//! either observes. Reading the same data, or reading one event type, never
//! conflicts. Only systems in the same [`Phase`] are compared: phases are
//! ordered barriers.
//!
//! # Soundness strategy
//!
//! A declaration is a promise; the engine must not trust it blindly.
//!
//! 1. **Debug validation now.** In builds with debug assertions, each
//!    system with a non-exclusive declaration runs with access recording on
//!    ([`World::record_access`](crate::World::record_access) and the same
//!    for events). Afterwards, any component, resource or event it used
//!    without declaring it, any structural change made directly on the
//!    world (spawn, despawn, insert, remove: those need exclusivity), any
//!    resource or event without a Rust type, and any undeclared command
//!    panics, naming the system and what it used.
//! 2. **No shared `Ctx` later.** A parallel executor must never hand
//!    `&mut Ctx` (or `&mut World`) to two systems at once. Declared systems
//!    that run in parallel will get a restricted view that exposes only what
//!    was declared, so the type system enforces the declaration. Until that
//!    view exists, systems keep `FnMut(&mut Ctx)` and run sequentially, so
//!    an undeclared access can't cause a data race, only a debug panic.
//!
//! # Scope and limits
//!
//! - **Other `Ctx` services** (input, drawing, audio, physics, navigation,
//!   random numbers, UI, assets, networking) are not part of a declaration
//!   and are not recorded. A system that uses them must stay exclusive; the
//!   debug check can't catch it yet. The restricted view (point 2) closes
//!   that gap by not offering them.
//! - **Physics** and other built-in systems stay exclusive.
//! - **Foreign (C/C++) systems** stay exclusive: their components and
//!   events have no Rust type to declare.
//! - **Hot patching** swaps a system's code, not its declaration. A patch
//!   that changes what a system touches is caught by the debug check on its
//!   next run.
//! - Release builds skip validation entirely; the declaration costs nothing
//!   at run time there.
//!
//! ```
//! use ponko::*;
//!
//! struct Pos(f32);
//! struct Vel(f32);
//! struct Health(i32);
//!
//! let game = Game::new("access", 320, 240)
//!     .add_with_access(
//!         Phase::AfterPhysics,
//!         SystemAccess::new().write::<Pos>().read::<Vel>(),
//!         |ctx| ctx.world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0),
//!     )
//!     .add_with_access(
//!         Phase::AfterPhysics,
//!         SystemAccess::new().write::<Health>(),
//!         |ctx| ctx.world.query::<&mut Health>().for_each(|h| h.0 -= 1),
//!     );
//! let report = game.access_report();
//! assert_eq!(report.compatible_pairs().count(), 1);
//! ```

use std::any::TypeId;
use std::fmt;

use ponko_ecs::{AccessLog, Component, ViewAccess, World};

#[cfg(debug_assertions)]
use crate::Ctx;
use crate::Phase;

/// An engine service on [`Ctx`](crate::Ctx) that a system can declare,
/// beside its components, resources, events and commands.
///
/// Names follow the service fields of `Ctx`. Two declarations conflict on
/// a service when both use it and at least one writes it. View systems
/// ([`Game::add_view`](crate::Game::add_view)) can use the services marked
/// "view" below, through [`SystemView`](crate::SystemView); the others
/// hold state that belongs to the window, renderer or asset tables, so a
/// system that needs them keeps `&mut Ctx` (declared with
/// [`Game::add_with_access`](crate::Game::add_with_access), or exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Service {
    /// Keys, mouse, gamepads and named actions (`ctx.input`). View: read.
    Input,
    /// Frame and simulation clocks: `dt`, ticks, pause and speed
    /// (`ctx.time`). View: read.
    Time,
    /// The system's random stream (`ctx.random()`). Only read: each
    /// system's stream depends on the seed, tick and system alone. View: read.
    Random,
    /// Drawing, textures, meshes, materials and render settings (`ctx.gfx`).
    Gfx,
    /// Sounds, buses and the listener (`ctx.audio`). View: write (play sounds).
    Audio,
    /// Hot reload, data files, prefabs and scenes (`ctx.assets`).
    Assets,
    /// The retained UI tree, style and input capture (`ctx.ui`).
    Ui,
    /// Profiling, statistics, the HUD and traces (`ctx.perf`).
    Perf,
    /// The 2D and 3D cameras (`ctx.camera`, `ctx.camera3d`). View: read and write.
    Camera,
    /// The spatial grid (`ctx.spatial`). View: read.
    Spatial,
    /// The 2D physics world (`ctx.physics`). View: read and write.
    Physics,
    /// The 3D physics world (`ctx.physics3d`). View: read and write.
    Physics3D,
    /// Lockstep networking (`ctx.net`). View: read and write.
    Net,
}

impl Service {
    /// Every service, in the order of their C numbers (`CRABBY_SERVICE_*`).
    pub const ALL: [Service; 13] = [
        Service::Input,
        Service::Time,
        Service::Random,
        Service::Gfx,
        Service::Audio,
        Service::Assets,
        Service::Ui,
        Service::Perf,
        Service::Camera,
        Service::Spatial,
        Service::Physics,
        Service::Physics3D,
        Service::Net,
    ];

    /// The service's field name on `Ctx`, e.g. `"input"`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Service::Input => "input",
            Service::Time => "time",
            Service::Random => "random",
            Service::Gfx => "gfx",
            Service::Audio => "audio",
            Service::Assets => "assets",
            Service::Ui => "ui",
            Service::Perf => "perf",
            Service::Camera => "camera",
            Service::Spatial => "spatial",
            Service::Physics => "physics",
            Service::Physics3D => "physics3d",
            Service::Net => "net",
        }
    }

    /// What a [`SystemView`](crate::SystemView) offers: `None` if nothing,
    /// else whether it can also write.
    pub(crate) fn in_view(self) -> Option<bool> {
        match self {
            Service::Input | Service::Time | Service::Random | Service::Spatial => Some(false),
            Service::Audio | Service::Camera | Service::Physics | Service::Physics3D | Service::Net => Some(true),
            Service::Gfx | Service::Assets | Service::Ui | Service::Perf => None,
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How a declared item is identified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    /// A Rust type.
    Type(TypeId),
    /// A component or resource registered by name (C and C++), matched by
    /// the item's name.
    Named,
    /// An event type without a Rust type (C and C++ use component ids).
    Raw(u32),
}

/// One declared component, resource or event type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Item {
    key: Key,
    name: &'static str,
    /// Written (components, resources) or sent (events).
    write: bool,
}

impl Item {
    fn same(&self, other: &Item) -> bool {
        match (self.key, other.key) {
            (Key::Named, Key::Named) => self.name == other.name,
            (a, b) => a == b,
        }
    }
}

fn item<T: 'static>(write: bool) -> Item {
    Item {
        key: Key::Type(TypeId::of::<T>()),
        name: std::any::type_name::<T>(),
        write,
    }
}

/// A name that lives as long as the program: declarations of C and C++
/// systems are made once, at registration.
fn named(name: &str, write: bool) -> Item {
    Item {
        key: Key::Named,
        name: Box::leak(name.to_owned().into_boxed_str()),
        write,
    }
}

/// Adds `it`, a write replacing a read of the same type.
fn add(items: &mut Vec<Item>, it: Item) {
    match items.iter_mut().find(|i| i.same(&it)) {
        Some(i) => i.write |= it.write,
        None => items.push(it),
    }
}

/// Adds event right `it`: sending and reading are separate rights, kept
/// as separate entries.
fn add_event(items: &mut Vec<Item>, it: Item) {
    if !items.iter().any(|i| i.same(&it) && i.write == it.write) {
        items.push(it);
    }
}

/// What a system declares it reads and writes.
///
/// Two declared systems in the same [`Phase`](crate::Phase) conflict (and
/// keep their order) when either is exclusive, both touch the same
/// component, resource or [`Service`] and one writes it, one sends an event
/// type the other reads or both send it, or either records commands while
/// the other touches anything. [`conflicts_before`](Self::conflicts_before)
/// has the finer rules for a known order. In builds with debug assertions,
/// each declared `Ctx` system runs with access recording on and panics on
/// any component, resource, event or command it used without declaring
/// it; a view system ([`Game::add_view`](crate::Game::add_view)) can't
/// reach anything undeclared at all.
///
/// [`SystemAccess::new`] declares nothing; chain the builder methods to add
/// to it. Declaring a write also covers reading the same type.
///
/// ```
/// use ponko::*;
/// struct Pos(f32);
/// struct Vel(f32);
/// struct Gravity(f32);
///
/// let fall = SystemAccess::new().write::<Vel>().read_resource::<Gravity>();
/// let movement = SystemAccess::new().write::<Pos>().read::<Vel>();
/// let names: Vec<String> = fall.conflicts_with(&movement).iter().map(|c| c.to_string()).collect();
/// assert_eq!(names.len(), 1);
/// assert!(names[0].contains("Vel"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemAccess {
    exclusive: bool,
    components: Vec<Item>,
    resources: Vec<Item>,
    events: Vec<Item>,
    commands: bool,
    /// Declared services and whether each is written.
    services: Vec<(Service, bool)>,
    /// The typed components and resources, as the ECS view takes them.
    world: ViewAccess,
}

impl SystemAccess {
    /// A declaration of nothing: such a system touches no components,
    /// resources, events or services and records no commands.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// May use anything; conflicts with every other system. What systems
    /// registered without a declaration get.
    #[must_use]
    pub fn exclusive() -> Self {
        SystemAccess {
            exclusive: true,
            ..Self::default()
        }
    }

    /// Reads component `T` (`&T` in a query, `world.get::<T>`).
    #[must_use]
    pub fn read<T: Component>(mut self) -> Self {
        add(&mut self.components, item::<T>(false));
        self.world = self.world.read::<T>();
        self
    }

    /// Reads and writes component `T` (`&mut T` in a query, `world.get_mut::<T>`).
    #[must_use]
    pub fn write<T: Component>(mut self) -> Self {
        add(&mut self.components, item::<T>(true));
        self.world = self.world.write::<T>();
        self
    }

    /// Reads resource `R`.
    #[must_use]
    pub fn read_resource<R: Send + Sync + 'static>(mut self) -> Self {
        add(&mut self.resources, item::<R>(false));
        self.world = self.world.read_resource::<R>();
        self
    }

    /// Reads and writes resource `R`, including inserting or removing it.
    #[must_use]
    pub fn write_resource<R: Send + Sync + 'static>(mut self) -> Self {
        add(&mut self.resources, item::<R>(true));
        self.world = self.world.write_resource::<R>();
        self
    }

    /// Reads `E` events ([`Events::read`](crate::Events::read) or `peek`).
    #[must_use]
    pub fn read_events<E: 'static>(mut self) -> Self {
        add_event(&mut self.events, item::<E>(false));
        self
    }

    /// Sends `E` events. Doesn't cover reading them; add
    /// [`read_events`](Self::read_events) for that.
    #[must_use]
    pub fn send_events<E: 'static>(mut self) -> Self {
        add_event(&mut self.events, item::<E>(true));
        self
    }

    /// Records commands ([`Ctx::commands`](crate::Ctx::commands)), applied
    /// after the system.
    #[must_use]
    pub fn commands(mut self) -> Self {
        self.commands = true;
        self
    }

    /// Reads `service` (asks it questions without changing it).
    #[must_use]
    pub fn read_service(mut self, service: Service) -> Self {
        self.add_service(service, false);
        self
    }

    /// Reads and changes `service`.
    #[must_use]
    pub fn write_service(mut self, service: Service) -> Self {
        self.add_service(service, true);
        self
    }

    fn add_service(&mut self, service: Service, write: bool) {
        match self.services.iter_mut().find(|(s, _)| *s == service) {
            Some((_, w)) => *w |= write,
            None => self.services.push((service, write)),
        }
    }

    /// Reads the component registered under `name` (C and C++ components,
    /// or any component looked up by name).
    #[must_use]
    pub fn read_named(mut self, name: &str) -> Self {
        add(&mut self.components, named(name, false));
        self
    }

    /// Reads and writes the component registered under `name`.
    #[must_use]
    pub fn write_named(mut self, name: &str) -> Self {
        add(&mut self.components, named(name, true));
        self
    }

    /// Reads the resource stored under `name` without a Rust type (the C
    /// and C++ path). Such resources are reached through raw pointers, so
    /// the debug check can't tell reads from writes: declare
    /// [`write_resource_named`](Self::write_resource_named) if the system
    /// changes it.
    #[must_use]
    pub fn read_resource_named(mut self, name: &str) -> Self {
        add(&mut self.resources, named(name, false));
        self
    }

    /// Reads and writes the resource stored under `name` without a Rust type.
    #[must_use]
    pub fn write_resource_named(mut self, name: &str) -> Self {
        add(&mut self.resources, named(name, true));
        self
    }

    /// Reads events of foreign type `id` (C and C++).
    #[doc(hidden)]
    #[must_use]
    pub fn read_raw_events(mut self, id: u32) -> Self {
        add_event(&mut self.events, raw_event(id, false));
        self
    }

    /// Sends events of foreign type `id` (C and C++).
    #[doc(hidden)]
    #[must_use]
    pub fn send_raw_events(mut self, id: u32) -> Self {
        add_event(&mut self.events, raw_event(id, true));
        self
    }

    /// `true` for [`exclusive`](Self::exclusive) access.
    #[must_use]
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// `Some(written)` if `service` is declared, `None` if not.
    #[must_use]
    pub fn service(&self, service: Service) -> Option<bool> {
        self.services.iter().find(|(s, _)| *s == service).map(|&(_, w)| w)
    }

    /// `true` if the system records commands.
    #[must_use]
    pub fn records_commands(&self) -> bool {
        self.commands
    }

    /// `true` if `E` events may be read.
    pub(crate) fn reads_events(&self, id: TypeId) -> bool {
        self.events.iter().any(|i| i.key == Key::Type(id) && !i.write)
    }

    /// `true` if `E` events may be sent.
    pub(crate) fn sends_events(&self, id: TypeId) -> bool {
        self.events.iter().any(|i| i.key == Key::Type(id) && i.write)
    }

    /// The typed component and resource part, for an ECS view.
    pub(crate) fn world_access(&self) -> &ViewAccess {
        &self.world
    }

    /// Why a view system can't have this declaration, if it can't.
    pub(crate) fn view_problem(&self) -> Option<String> {
        if self.exclusive {
            return Some("is exclusive; register it with `add` instead".into());
        }
        let untyped = |items: &[Item]| items.iter().find(|i| !matches!(i.key, Key::Type(_))).map(|i| i.name);
        if let Some(name) = untyped(&self.components)
            .or_else(|| untyped(&self.resources))
            .or_else(|| untyped(&self.events))
        {
            return Some(format!("declares `{name}`, which has no Rust type"));
        }
        for &(s, write) in &self.services {
            match s.in_view() {
                None => {
                    return Some(format!(
                        "declares service `{s}`, which a view doesn't offer; use `add_with_access`"
                    ));
                }
                Some(false) if write => {
                    return Some(format!(
                        "declares writing service `{s}`, which a view only reads; use `add_with_access`"
                    ));
                }
                _ => {}
            }
        }
        None
    }

    /// Why this system and `other` can't run together in either order;
    /// empty if they could. The conservative, symmetric rules: see
    /// [`conflicts_before`](Self::conflicts_before) for a known order.
    #[must_use]
    pub fn conflicts_with(&self, other: &SystemAccess) -> Vec<AccessConflict> {
        self.conflicts(other, false)
    }

    /// Why this system, registered (and so sequentially run) **before**
    /// `later` in the same phase, can't share a parallel batch with it;
    /// empty if it can.
    ///
    /// The finer barrier rules assume the batch contract a parallel
    /// executor must keep: commands recorded in a batch apply when the batch
    /// ends, in registration order, and events sent in it become visible
    /// when it ends, in registration order. Then, unlike
    /// [`conflicts_with`](Self::conflicts_with):
    ///
    /// - two systems that send the same event type don't conflict (their
    ///   events are published in the sequential order);
    /// - a reader before a sender doesn't conflict (sequentially it only
    ///   sees those events on its next run anyway); a sender before a
    ///   reader does;
    /// - a system that records commands conflicts only with a **later**
    ///   system that touches components or resources (which would see the
    ///   applied commands sequentially); two recorders don't conflict.
    ///
    /// Components, resources and services conflict as in `conflicts_with`.
    ///
    /// ```
    /// use ponko::*;
    /// struct Hit;
    /// let send = SystemAccess::new().send_events::<Hit>();
    /// let read = SystemAccess::new().read_events::<Hit>();
    /// assert!(!read.conflicts_before(&send).is_empty() == false); // reader first: fine
    /// assert_eq!(send.conflicts_before(&read).len(), 1); // sender first: the reader would see it
    /// assert!(send.conflicts_before(&send.clone()).is_empty()); // published in order
    /// ```
    #[must_use]
    pub fn conflicts_before(&self, later: &SystemAccess) -> Vec<AccessConflict> {
        self.conflicts(later, true)
    }

    fn conflicts(&self, other: &SystemAccess, ordered: bool) -> Vec<AccessConflict> {
        if self.exclusive || other.exclusive {
            return vec![AccessConflict::Exclusive];
        }
        let mut out = Vec::new();
        let data = |items: &[Item], others: &[Item], out: &mut Vec<AccessConflict>, make: fn(&'static str) -> _| {
            for a in items {
                if others.iter().any(|b| a.same(b) && (a.write || b.write)) {
                    out.push(make(a.name));
                }
            }
        };
        data(&self.components, &other.components, &mut out, AccessConflict::Component);
        data(&self.resources, &other.resources, &mut out, AccessConflict::Resource);
        for a in &self.events {
            let clash = other.events.iter().any(|b| {
                a.same(b)
                    && if ordered {
                        // Only an earlier send seen by a later read.
                        a.write && !b.write
                    } else {
                        // Send/read or send/send of one type; read/read is fine.
                        a.write || b.write
                    }
            });
            let name = AccessConflict::Event(a.name);
            if clash && !out.contains(&name) {
                out.push(name);
            }
        }
        let touches_world = |s: &SystemAccess| !s.components.is_empty() || !s.resources.is_empty();
        let commands = if ordered {
            self.commands && touches_world(other)
        } else {
            let uses = |s: &SystemAccess| s.commands || touches_world(s);
            (self.commands && uses(other)) || (other.commands && uses(self))
        };
        if commands {
            out.push(AccessConflict::Commands);
        }
        for &(s, w) in &self.services {
            if other.services.iter().any(|&(t, v)| s == t && (w || v)) {
                out.push(AccessConflict::Service(s));
            }
        }
        out
    }

    /// What `used` shows this system did beyond its declaration, as
    /// readable lines; empty if it stayed within it.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) fn undeclared(&self, world: &World, used: &AccessLog, events: &EventUse, commands: bool) -> Vec<String> {
        if self.exclusive {
            return Vec::new();
        }
        let mut out = Vec::new();
        let covers = |items: &[Item], id: Option<TypeId>, name: &str, write: bool| {
            items.iter().any(|i| {
                let same = match i.key {
                    Key::Type(t) => id == Some(t),
                    Key::Named => i.name == name,
                    Key::Raw(_) => false,
                };
                same && (i.write || !write)
            })
        };
        let verb = |write| if write { "writes" } else { "reads" };
        for &(c, write) in &used.components {
            let info = world.component_info(c);
            if !covers(&self.components, info.type_id(), info.name(), write) {
                out.push(format!("{} component `{}`", verb(write), info.name()));
            }
        }
        for &(id, name, write) in &used.resources {
            if !covers(&self.resources, Some(id), name, write) {
                out.push(format!("{} resource `{name}`", verb(write)));
            }
        }
        for name in &used.dynamic_resources {
            // Raw pointers can't tell reads from writes: any declaration covers use.
            if !covers(&self.resources, None, name, false) {
                out.push(format!("uses resource `{name}`, which has no Rust type"));
            }
        }
        if used.structural {
            out.push("changes the world's structure directly (spawn, despawn, insert or remove); use commands".into());
        }
        for &(id, name, sent) in &events.used {
            if !self.events.iter().any(|i| i.key == Key::Type(id) && i.write == sent) {
                let what = if sent { "sends" } else { "reads" };
                out.push(format!("{what} events `{name}`"));
            }
        }
        for &(id, sent) in &events.raw {
            if !self.events.iter().any(|i| i.key == Key::Raw(id) && i.write == sent) {
                let what = if sent { "sends" } else { "reads" };
                out.push(format!("{what} events of foreign type {id}"));
            }
        }
        if commands && !self.commands {
            out.push("records commands".into());
        }
        out
    }
}

fn raw_event(id: u32, write: bool) -> Item {
    Item {
        key: Key::Raw(id),
        name: Box::leak(format!("foreign event {id}").into_boxed_str()),
        write,
    }
}

/// Why two systems can't run together: from
/// [`SystemAccess::conflicts_with`] and [`AccessReport`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessConflict {
    /// At least one of them is exclusive (or undeclared).
    Exclusive,
    /// Both touch this component and at least one writes it.
    Component(&'static str),
    /// Both touch this resource and at least one writes it.
    Resource(&'static str),
    /// One sends this event type and the other reads or sends it.
    Event(&'static str),
    /// One records commands and the other touches components, resources or commands.
    Commands,
    /// Both use this service and at least one writes it.
    Service(Service),
}

impl fmt::Display for AccessConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessConflict::Exclusive => f.write_str("exclusive access"),
            AccessConflict::Component(n) => write!(f, "component `{n}`"),
            AccessConflict::Resource(n) => write!(f, "resource `{n}`"),
            AccessConflict::Event(n) => write!(f, "events `{n}`"),
            AccessConflict::Commands => f.write_str("commands"),
            AccessConflict::Service(s) => write!(f, "service `{s}`"),
        }
    }
}

/// Event types a system used while its declaration was checked.
#[derive(Default)]
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub(crate) struct EventUse {
    /// Type, name, sent (else read). One entry per (type, sent).
    pub(crate) used: Vec<(TypeId, &'static str, bool)>,
    /// Events without a Rust type (the C/C++ path): id, sent.
    pub(crate) raw: Vec<(u32, bool)>,
}

#[cfg_attr(not(debug_assertions), allow(dead_code))]
impl EventUse {
    pub(crate) fn note(&mut self, id: TypeId, name: &'static str, sent: bool) {
        if !self.used.iter().any(|&(i, _, s)| i == id && s == sent) {
            self.used.push((id, name, sent));
        }
    }

    pub(crate) fn note_raw(&mut self, id: u32, sent: bool) {
        if !self.raw.contains(&(id, sent)) {
            self.raw.push((id, sent));
        }
    }
}

/// Debug builds: starts recording what the system about to run uses.
#[cfg(debug_assertions)]
pub(crate) fn begin_check(ctx: &mut Ctx) {
    ctx.world.record_access();
    *ctx.events.used.get_mut() = Some(EventUse::default());
}

/// Debug builds: stops recording and panics if the system that just ran
/// (before its commands are applied) went beyond `access`.
#[cfg(debug_assertions)]
pub(crate) fn end_check(ctx: &mut Ctx, system: &str, access: &SystemAccess) {
    let used = ctx.world.take_access_log().unwrap_or_default();
    let events = ctx.events.used.get_mut().take().unwrap_or_default();
    let problems = access.undeclared(&ctx.world, &used, &events, !ctx.commands.is_empty());
    if !problems.is_empty() {
        panic!(
            "system `{system}` went beyond its declared access: it {}. Declare it in its SystemAccess, \
             or register the system without one (exclusive).",
            problems.join(", it ")
        );
    }
}

/// One registered system, as the report sees it.
#[derive(Clone, Debug)]
pub struct ReportedSystem {
    /// The system's name (its function path).
    pub name: &'static str,
    /// The phase it runs in.
    pub phase: Phase,
    /// Its declaration; `None` means undeclared, which counts as exclusive.
    pub access: Option<SystemAccess>,
    /// `true` for a view system ([`Game::add_view`](crate::Game::add_view)):
    /// it can reach nothing beyond its declaration, so a parallel executor
    /// may trust it. Declared `&mut Ctx` systems (`add_with_access`) are
    /// only checked in debug builds and must stay sequential.
    pub enforced: bool,
}

/// Two systems in the same phase and whether they conflict.
#[derive(Clone, Debug)]
pub struct ReportedPair {
    /// Index into [`AccessReport::systems`] of the earlier system.
    pub first: usize,
    /// Index of the later system (same phase).
    pub second: usize,
    /// Why they can't share a parallel batch, by the ordered rules of
    /// [`SystemAccess::conflicts_before`]; empty if they could.
    pub conflicts: Vec<AccessConflict>,
}

/// Which declared systems could run together: from
/// [`Game::access_report`](crate::Game::access_report).
///
/// Lists every system and, for each pair in the same phase where both have
/// a declaration, its conflicts. Undeclared systems are exclusive and are
/// left out of the pairs. Purely informational: systems still run in order.
#[derive(Clone, Debug, Default)]
pub struct AccessReport {
    /// Every registered system, in run order.
    pub systems: Vec<ReportedSystem>,
    /// Pairs of declared systems in the same phase.
    pub pairs: Vec<ReportedPair>,
}

impl AccessReport {
    /// Builds the report for systems in run order.
    pub(crate) fn new(systems: Vec<ReportedSystem>) -> Self {
        let mut pairs = Vec::new();
        for (i, a) in systems.iter().enumerate() {
            let Some(aa) = &a.access else { continue };
            for (j, b) in systems.iter().enumerate().skip(i + 1) {
                if let Some(ba) = &b.access
                    && a.phase == b.phase
                {
                    pairs.push(ReportedPair {
                        first: i,
                        second: j,
                        conflicts: aa.conflicts_before(ba),
                    });
                }
            }
        }
        AccessReport { systems, pairs }
    }

    /// Names of the pairs whose declarations don't conflict.
    pub fn compatible_pairs(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.pairs
            .iter()
            .filter(|p| p.conflicts.is_empty())
            .map(|p| (self.systems[p.first].name, self.systems[p.second].name))
    }

    /// Names of the compatible pairs where both are view systems, the only
    /// ones a parallel executor may run together.
    pub fn parallel_pairs(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.pairs
            .iter()
            .filter(|p| p.conflicts.is_empty() && self.systems[p.first].enforced && self.systems[p.second].enforced)
            .map(|p| (self.systems[p.first].name, self.systems[p.second].name))
    }
}

impl fmt::Display for AccessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in &self.pairs {
            let (a, b) = (self.systems[p.first].name, self.systems[p.second].name);
            let enforced = self.systems[p.first].enforced && self.systems[p.second].enforced;
            if p.conflicts.is_empty() && enforced {
                writeln!(f, "{a} + {b}: could run together")?;
            } else if p.conflicts.is_empty() {
                writeln!(f, "{a} + {b}: no conflict, but a `Ctx` system stays sequential")?;
            } else {
                let why: Vec<String> = p.conflicts.iter().map(ToString::to_string).collect();
                writeln!(f, "{a} + {b}: conflict on {}", why.join(", "))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pos;
    struct Hit;

    #[test]
    fn a_write_replaces_a_read_of_the_same_type() {
        let a = SystemAccess::new().read::<Pos>().write::<Pos>().read::<Pos>();
        assert_eq!(a.components.len(), 1);
        assert!(a.components[0].write);
        let n = SystemAccess::new().read_named("pos").write_named("pos");
        assert_eq!(n.components.len(), 1);
        assert!(n.components[0].write);
    }

    #[test]
    fn sending_and_reading_one_event_type_are_two_rights() {
        let a = SystemAccess::new()
            .send_events::<Hit>()
            .read_events::<Hit>()
            .send_events::<Hit>();
        assert_eq!(a.events.len(), 2);
        let b = SystemAccess::new()
            .read_events::<Hit>()
            .send_events::<Hit>()
            .read_events::<Hit>();
        assert_eq!(b.events.len(), 2);
    }

    #[test]
    fn exclusive_declares_nothing_else_and_covers_everything() {
        let mut world = World::new();
        let id = world.register::<Pos>();
        let used = AccessLog {
            components: vec![(id, true)],
            structural: true,
            ..AccessLog::default()
        };
        let events = EventUse {
            raw: vec![(7, true)],
            ..EventUse::default()
        };
        assert!(
            SystemAccess::exclusive()
                .undeclared(&world, &used, &events, true)
                .is_empty()
        );
        assert!(!SystemAccess::new().undeclared(&world, &used, &events, true).is_empty());
    }
}
