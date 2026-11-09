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

use ponko_ecs::{AccessLog, Component, World};

#[cfg(debug_assertions)]
use crate::Ctx;
use crate::Phase;

/// One declared component, resource or event type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Item {
    id: TypeId,
    name: &'static str,
    /// Written (components, resources) or sent (events).
    write: bool,
}

fn item<T: 'static>(write: bool) -> Item {
    Item {
        id: TypeId::of::<T>(),
        name: std::any::type_name::<T>(),
        write,
    }
}

/// Adds `it`, a write replacing a read of the same type.
fn add(items: &mut Vec<Item>, it: Item) {
    match items.iter_mut().find(|i| i.id == it.id) {
        Some(i) => i.write |= it.write,
        None => items.push(it),
    }
}

/// Adds event right `it`: sending and reading are separate rights, kept
/// as separate entries.
fn add_event(items: &mut Vec<Item>, it: Item) {
    if !items.contains(&it) {
        items.push(it);
    }
}

/// What a system declares it reads and writes.
///
/// Two declared systems in the same [`Phase`](crate::Phase) conflict (and
/// keep their order) when either is exclusive, both touch the same
/// component or resource and one writes it, one sends an event type the
/// other reads or both send it, or either records commands while the other
/// touches anything. In builds with debug assertions, each declared system
/// runs with access recording on and panics on anything it used without
/// declaring it.
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
}

impl SystemAccess {
    /// A declaration of nothing: such a system touches no components,
    /// resources or events and records no commands.
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
        self
    }

    /// Reads and writes component `T` (`&mut T` in a query, `world.get_mut::<T>`).
    #[must_use]
    pub fn write<T: Component>(mut self) -> Self {
        add(&mut self.components, item::<T>(true));
        self
    }

    /// Reads resource `R`.
    #[must_use]
    pub fn read_resource<R: Send + Sync + 'static>(mut self) -> Self {
        add(&mut self.resources, item::<R>(false));
        self
    }

    /// Reads and writes resource `R`, including inserting or removing it.
    #[must_use]
    pub fn write_resource<R: Send + Sync + 'static>(mut self) -> Self {
        add(&mut self.resources, item::<R>(true));
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

    /// `true` for [`exclusive`](Self::exclusive) access.
    #[must_use]
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// Why this system and `other` can't run together; empty if they could.
    #[must_use]
    pub fn conflicts_with(&self, other: &SystemAccess) -> Vec<AccessConflict> {
        if self.exclusive || other.exclusive {
            return vec![AccessConflict::Exclusive];
        }
        let mut out = Vec::new();
        let data = |items: &[Item], others: &[Item], out: &mut Vec<AccessConflict>, make: fn(&'static str) -> _| {
            for a in items {
                if others.iter().any(|b| a.id == b.id && (a.write || b.write)) {
                    out.push(make(a.name));
                }
            }
        };
        data(&self.components, &other.components, &mut out, AccessConflict::Component);
        data(&self.resources, &other.resources, &mut out, AccessConflict::Resource);
        for a in &self.events {
            // Send/read or send/send of one type; read/read is fine.
            let clash = other.events.iter().any(|b| a.id == b.id && (a.write || b.write));
            let name = AccessConflict::Event(a.name);
            if clash && !out.contains(&name) {
                out.push(name);
            }
        }
        let touches_world = |s: &SystemAccess| s.commands || !s.components.is_empty() || !s.resources.is_empty();
        if (self.commands && touches_world(other)) || (other.commands && touches_world(self)) {
            out.push(AccessConflict::Commands);
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
        let covers = |items: &[Item], id: Option<TypeId>, write: bool| {
            id.is_some_and(|id| items.iter().any(|i| i.id == id && (i.write || !write)))
        };
        let verb = |write| if write { "writes" } else { "reads" };
        for &(c, write) in &used.components {
            let info = world.component_info(c);
            if !covers(&self.components, info.type_id(), write) {
                out.push(format!("{} component `{}`", verb(write), info.name()));
            }
        }
        for &(id, name, write) in &used.resources {
            if !covers(&self.resources, Some(id), write) {
                out.push(format!("{} resource `{name}`", verb(write)));
            }
        }
        for name in &used.dynamic_resources {
            out.push(format!("uses resource `{name}`, which has no Rust type"));
        }
        if used.structural {
            out.push("changes the world's structure directly (spawn, despawn, insert or remove); use commands".into());
        }
        for &(id, name, sent) in &events.used {
            if !self.events.iter().any(|i| i.id == id && i.write == sent) {
                let what = if sent { "sends" } else { "reads" };
                out.push(format!("{what} events `{name}`"));
            }
        }
        if events.raw {
            out.push("uses events without a Rust type".into());
        }
        if commands && !self.commands {
            out.push("records commands".into());
        }
        out
    }
}

/// Why two systems can't run together: from
/// [`SystemAccess::conflicts_with`] and [`AccessReport`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl fmt::Display for AccessConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessConflict::Exclusive => f.write_str("exclusive access"),
            AccessConflict::Component(n) => write!(f, "component `{n}`"),
            AccessConflict::Resource(n) => write!(f, "resource `{n}`"),
            AccessConflict::Event(n) => write!(f, "events `{n}`"),
            AccessConflict::Commands => f.write_str("commands"),
        }
    }
}

/// Event types a system used while its declaration was checked.
#[derive(Default)]
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub(crate) struct EventUse {
    /// Type, name, sent (else read). One entry per (type, sent).
    pub(crate) used: Vec<(TypeId, &'static str, bool)>,
    /// Used events without a Rust type (the C/C++ path).
    pub(crate) raw: bool,
}

#[cfg_attr(not(debug_assertions), allow(dead_code))]
impl EventUse {
    pub(crate) fn note(&mut self, id: TypeId, name: &'static str, sent: bool) {
        if !self.used.iter().any(|&(i, _, s)| i == id && s == sent) {
            self.used.push((id, name, sent));
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
}

/// Two systems in the same phase and whether they conflict.
#[derive(Clone, Debug)]
pub struct ReportedPair {
    /// Index into [`AccessReport::systems`] of the earlier system.
    pub first: usize,
    /// Index of the later system (same phase).
    pub second: usize,
    /// Why they can't run together; empty if they could.
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
                        conflicts: aa.conflicts_with(ba),
                    });
                }
            }
        }
        AccessReport { systems, pairs }
    }

    /// Names of the pairs that could run together.
    pub fn compatible_pairs(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.pairs
            .iter()
            .filter(|p| p.conflicts.is_empty())
            .map(|p| (self.systems[p.first].name, self.systems[p.second].name))
    }
}

impl fmt::Display for AccessReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in &self.pairs {
            let (a, b) = (self.systems[p.first].name, self.systems[p.second].name);
            if p.conflicts.is_empty() {
                writeln!(f, "{a} + {b}: could run together")?;
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
            raw: true,
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
