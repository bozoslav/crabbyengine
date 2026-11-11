//! The restricted view a declared system gets instead of `&mut Ctx`:
//! [`SystemView`], from [`Game::add_view`](crate::Game::add_view).
//!
//! A view system can reach only what its [`SystemAccess`] declares. The
//! world part is a [`WorldView`] (declared components and resources, no
//! structural changes); events, commands and services are offered through
//! methods that check the declaration on every call, in every build. There
//! is no path from a `SystemView` to the rest of `Ctx`, so a declaration
//! can't be wrong in a way that goes unnoticed: what isn't declared panics
//! when it is used, naming the system.
//!
//! A view system runs in one of two ways, with the same results. Alone, it
//! borrows what it declared from `Ctx`. In a parallel batch
//! ([`Game::parallel_systems`](crate::Game::parallel_systems)) it shares the
//! world with the batch's other systems and gets its own command buffer and
//! an outbox for its events, both delivered when the batch ends.

use std::any::TypeId;

use glam::Vec2;
use ponko_ecs::{Commands, ViewState, WorldView};
use web_time::Instant;

use crate::access::Service;
use crate::access::SystemAccess;
use crate::events::{EventSlot, Events, Outbox};
use crate::{Ctx, Key, MouseButton, Playing, Random, Sound, SoundParams};

/// The clocks, read once when the system starts.
#[derive(Clone, Copy, Default)]
pub(crate) struct Clock {
    dt: f32,
    fixed_dt: f32,
    sim_tick: u64,
    time: f64,
    frame: u64,
}

/// What a view system ([`Game::add_view`](crate::Game::add_view)) gets
/// instead of `&mut Ctx`: exactly what its [`SystemAccess`] declares.
///
/// - [`world`](Self::world): the declared components and resources (see
///   [`WorldView`]). Spawning, despawning, inserting and removing
///   components go through [`commands`](Self::commands).
/// - [`events`](Self::events): declared event types.
/// - Services, each behind its declaration: input and time (read), the
///   system's random stream, sound, the cameras, the spatial grid, the
///   physics worlds and networking. Drawing, UI, assets and profiling stay
///   on `Ctx` systems.
///
/// Every accessor checks the declaration and panics, naming the system and
/// what is missing, if it isn't declared.
///
/// ```
/// use ponko::prelude::*;
/// struct Health(i32);
/// struct Died(Entity);
///
/// fn hurt(sys: &mut ponko::system_view::SystemView) {
///     let mut died = Vec::new();
///     sys.world.query::<(Entity, &mut Health)>().for_each(|(e, h)| {
///         h.0 -= 1;
///         if h.0 <= 0 {
///             died.push(e);
///         }
///     });
///     for e in died {
///         sys.events().send(Died(e));
///         sys.commands().despawn(e);
///     }
/// }
///
/// let world = Game::new("view", 1, 1)
///     .add_view(
///         Phase::AfterPhysics,
///         ponko::access::SystemAccess::new().write::<Health>().send_events::<Died>().commands(),
///         hurt,
///     )
///     .run_headless(3, |ctx| {
///         ctx.world.spawn((Health(2),));
///     });
/// assert!(world.is_empty());
/// ```
pub struct SystemView<'a> {
    /// The declared components and resources.
    pub world: WorldView<'a>,
    name: &'static str,
    access: &'a SystemAccess,
    commands: &'a mut Commands,
    events: Port<'a>,
    input: &'a crate::input::Input,
    clock: Clock,
    random: Random,
    spatial: &'a crate::spatial::SpatialGrid,
    alone: Option<Alone<'a>>,
}

impl Clock {
    pub(crate) fn of(ctx: &Ctx) -> Self {
        Clock {
            dt: ctx.dt(),
            fixed_dt: ctx.time.fixed_dt(),
            sim_tick: ctx.time.sim_tick(),
            time: ctx.time.elapsed(),
            frame: ctx.time.frame(),
        }
    }
}

/// The services only one system at a time may hold: lent to a view system
/// that runs alone, absent in a parallel batch (a system that declares one
/// never joins a batch).
struct Alone<'a> {
    audio: &'a mut crate::audio::Audio,
    camera: &'a mut crate::d2::Camera,
    #[cfg(feature = "3d")]
    camera3d: &'a mut crate::d3::Camera,
    #[cfg(feature = "physics")]
    physics: &'a mut crate::d2::Physics,
    #[cfg(feature = "physics3d")]
    physics3d: &'a mut crate::d3::Physics,
    net: &'a mut crate::net::Net,
}

/// Where a view system's events come from and go to.
enum Port<'a> {
    /// Alone: the game's event store, directly.
    Direct(&'a mut Events),
    /// In a batch: the kept events of its declared types, shared and
    /// read-only; the types it has read in this run; its outbox.
    Batch {
        slots: &'a [EventSlot<'a>],
        read: &'a mut Vec<TypeId>,
        outbox: &'a mut Outbox,
    },
}

impl<'a> SystemView<'a> {
    /// The services a batch doesn't have. `need` passed, so the system
    /// declared one of them, and such a system always runs alone.
    fn alone(&mut self) -> &mut Alone<'a> {
        match &mut self.alone {
            Some(alone) => alone,
            None => unreachable!("a system that declares an exclusive service runs alone"),
        }
    }

    fn alone_ref(&self) -> &Alone<'a> {
        match &self.alone {
            Some(alone) => alone,
            None => unreachable!("a system that declares an exclusive service runs alone"),
        }
    }
}

impl SystemView<'_> {
    /// The system's name (its function path).
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The system's declaration.
    #[must_use]
    pub fn access(&self) -> &SystemAccess {
        self.access
    }

    #[track_caller]
    fn need(&self, service: Service, write: bool) {
        let ok = self.access.service(service).is_some_and(|w| w || !write);
        if !ok {
            let verb = if write { "write" } else { "read" };
            panic!(
                "system `{}` doesn't declare that it may {verb} service `{service}`; add `{}_service(Service::{service:?})` to its SystemAccess",
                self.name, verb
            );
        }
    }

    // ---- commands and events -------------------------------------------------------

    /// Commands to apply after the system (declared with
    /// [`SystemAccess::commands`]): spawning, despawning, inserting and
    /// removing components.
    #[track_caller]
    pub fn commands(&mut self) -> &mut Commands {
        if !self.access.records_commands() {
            panic!(
                "system `{}` doesn't declare that it records commands; add `.commands()` to its SystemAccess",
                self.name
            );
        }
        self.commands
    }

    /// The declared event types.
    pub fn events(&mut self) -> ViewEvents<'_> {
        ViewEvents {
            name: self.name,
            access: self.access,
            port: match &mut self.events {
                Port::Direct(events) => Port::Direct(events),
                Port::Batch { slots, read, outbox } => Port::Batch { slots, read, outbox },
            },
        }
    }

    // ---- time (read) ------------------------------------------------------------------

    /// As [`Ctx::dt`]: the fixed step (times the interval) in simulation
    /// phases, the frame time in frame phases. Needs `Service::Time`.
    #[track_caller]
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.need(Service::Time, false);
        self.clock.dt
    }

    /// As [`Time::fixed_dt`](crate::Time::fixed_dt). Needs `Service::Time`.
    #[track_caller]
    #[must_use]
    pub fn fixed_dt(&self) -> f32 {
        self.need(Service::Time, false);
        self.clock.fixed_dt
    }

    /// As [`Time::sim_tick`](crate::Time::sim_tick). Needs `Service::Time`.
    #[track_caller]
    #[must_use]
    pub fn sim_tick(&self) -> u64 {
        self.need(Service::Time, false);
        self.clock.sim_tick
    }

    /// As [`Time::elapsed`](crate::Time::elapsed). Needs `Service::Time`.
    #[track_caller]
    #[must_use]
    pub fn time(&self) -> f64 {
        self.need(Service::Time, false);
        self.clock.time
    }

    /// As [`Time::frame`](crate::Time::frame). Needs `Service::Time`.
    #[track_caller]
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.need(Service::Time, false);
        self.clock.frame
    }

    // ---- random (read) -----------------------------------------------------------------

    /// As [`Ctx::random`]: this system's stream for this tick. Needs
    /// `Service::Random`.
    #[track_caller]
    #[must_use]
    pub fn random(&self) -> Random {
        self.need(Service::Random, false);
        self.random
    }

    // ---- input (read) ------------------------------------------------------------------

    /// As [`Ctx::key_down`]. Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn key_down(&self, key: Key) -> bool {
        self.need(Service::Input, false);
        self.input.key_down(key)
    }

    /// As [`Ctx::key_pressed`]. Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn key_pressed(&self, key: Key) -> bool {
        self.need(Service::Input, false);
        self.input.key_pressed(key)
    }

    /// As [`Ctx::key_released`]. Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn key_released(&self, key: Key) -> bool {
        self.need(Service::Input, false);
        self.input.key_released(key)
    }

    /// As [`Input::mouse_down`](crate::input::Input::mouse_down). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.need(Service::Input, false);
        self.input.mouse_down(button)
    }

    /// As [`Input::mouse_pressed`](crate::input::Input::mouse_pressed). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.need(Service::Input, false);
        self.input.mouse_pressed(button)
    }

    /// As [`Input::mouse_released`](crate::input::Input::mouse_released). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.need(Service::Input, false);
        self.input.mouse_released(button)
    }

    /// As [`Ctx::mouse_pos`]. Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_pos(&self) -> Vec2 {
        self.need(Service::Input, false);
        self.input.mouse_pos()
    }

    /// As [`Input::action_down`](crate::input::Input::action_down). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_down(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.down(action)
    }

    /// As [`Input::action_pressed`](crate::input::Input::action_pressed). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_pressed(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.pressed(action)
    }

    /// As [`Input::action_released`](crate::input::Input::action_released). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_released(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.released(action)
    }

    /// As [`Input::action_axis`](crate::input::Input::action_axis). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_axis(&self, action: impl crate::ActionRef) -> f32 {
        self.need(Service::Input, false);
        self.input.actions.axis(action)
    }

    /// As [`Input::action_axis2`](crate::input::Input::action_axis2). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_axis2(&self, action: impl crate::ActionRef) -> Vec2 {
        self.need(Service::Input, false);
        self.input.actions.axis2(action)
    }

    // ---- audio (write) -----------------------------------------------------------------

    /// As [`Ctx::play_sound`]. Needs `Service::Audio` written.
    #[track_caller]
    pub fn play_sound(&mut self, sound: &Sound) -> Playing {
        self.play_sound_ex(sound, SoundParams::default())
    }

    /// As [`Audio::play_ex`](crate::audio::Audio::play_ex). Needs `Service::Audio` written.
    #[track_caller]
    pub fn play_sound_ex(&mut self, sound: &Sound, params: SoundParams) -> Playing {
        self.need(Service::Audio, true);
        self.alone().audio.play_ex(sound, params)
    }

    // ---- cameras, spatial grid, physics, net -----------------------------------------------

    /// The 2D camera ([`Ctx::camera`]). Needs `Service::Camera`.
    #[track_caller]
    #[must_use]
    pub fn camera(&self) -> &crate::d2::Camera {
        self.need(Service::Camera, false);
        self.alone_ref().camera
    }

    /// The 2D camera to move. Needs `Service::Camera` written.
    #[track_caller]
    pub fn camera_mut(&mut self) -> &mut crate::d2::Camera {
        self.need(Service::Camera, true);
        self.alone().camera
    }

    /// The 3D camera ([`Ctx::camera3d`], feature `3d`). Needs `Service::Camera`.
    #[cfg(feature = "3d")]
    #[track_caller]
    #[must_use]
    pub fn camera3d(&self) -> &crate::d3::Camera {
        self.need(Service::Camera, false);
        self.alone_ref().camera3d
    }

    /// The 3D camera to move (feature `3d`). Needs `Service::Camera` written.
    #[cfg(feature = "3d")]
    #[track_caller]
    pub fn camera3d_mut(&mut self) -> &mut crate::d3::Camera {
        self.need(Service::Camera, true);
        self.alone().camera3d
    }

    /// The spatial grid ([`Ctx::spatial`]). Needs `Service::Spatial`.
    #[track_caller]
    #[must_use]
    pub fn spatial(&self) -> &crate::spatial::SpatialGrid {
        self.need(Service::Spatial, false);
        self.spatial
    }

    /// The 2D physics world (feature `physics`): ray casts and overlap
    /// queries. Needs `Service::Physics`.
    #[cfg(feature = "physics")]
    #[track_caller]
    #[must_use]
    pub fn physics(&self) -> &crate::d2::Physics {
        self.need(Service::Physics, false);
        self.alone_ref().physics
    }

    /// The 2D physics world to change: impulses, gravity. Needs
    /// `Service::Physics` written.
    #[cfg(feature = "physics")]
    #[track_caller]
    pub fn physics_mut(&mut self) -> &mut crate::d2::Physics {
        self.need(Service::Physics, true);
        self.alone().physics
    }

    /// The 3D physics world (feature `physics3d`). Needs `Service::Physics3D`.
    #[cfg(feature = "physics3d")]
    #[track_caller]
    #[must_use]
    pub fn physics3d(&self) -> &crate::d3::Physics {
        self.need(Service::Physics3D, false);
        self.alone_ref().physics3d
    }

    /// The 3D physics world to change. Needs `Service::Physics3D` written.
    #[cfg(feature = "physics3d")]
    #[track_caller]
    pub fn physics3d_mut(&mut self) -> &mut crate::d3::Physics {
        self.need(Service::Physics3D, true);
        self.alone().physics3d
    }

    /// Lockstep networking ([`Ctx::net`]). Needs `Service::Net`.
    #[track_caller]
    #[must_use]
    pub fn net(&self) -> &crate::net::Net {
        self.need(Service::Net, false);
        self.alone_ref().net
    }

    /// Lockstep networking, to send input. Needs `Service::Net` written.
    #[track_caller]
    pub fn net_mut(&mut self) -> &mut crate::net::Net {
        self.need(Service::Net, true);
        self.alone().net
    }
}

/// The declared event types of a view system: from [`SystemView::events`].
///
/// Same visibility rules as [`Events`]: see the guide's "Systems and the
/// frame" chapter. In a parallel batch, sent events reach the store when
/// the batch ends, in registration order; no system of the batch could have
/// seen them earlier ([`SystemAccess::conflicts_before`]).
///
/// Event types are `Send + Sync`: a batch reads them from several threads.
pub struct ViewEvents<'a> {
    name: &'static str,
    access: &'a SystemAccess,
    port: Port<'a>,
}

impl ViewEvents<'_> {
    #[track_caller]
    fn need<E: 'static>(&self, sent: bool) {
        let id = TypeId::of::<E>();
        let ok = if sent {
            self.access.sends_events(id)
        } else {
            self.access.reads_events(id)
        };
        if !ok {
            let (verb, method) = if sent {
                ("send", "send_events")
            } else {
                ("read", "read_events")
            };
            panic!(
                "system `{}` doesn't declare that it may {verb} events `{}`; add `.{method}::<...>()` to its SystemAccess",
                self.name,
                std::any::type_name::<E>()
            );
        }
    }

    /// As [`Events::send`]. Needs [`SystemAccess::send_events`].
    #[track_caller]
    pub fn send<E: Send + Sync + 'static>(&mut self, event: E) {
        self.need::<E>(true);
        match &mut self.port {
            Port::Direct(events) => events.send(event),
            Port::Batch { outbox, .. } => outbox.send(event),
        }
    }

    /// As [`Events::read`]. Needs [`SystemAccess::read_events`].
    #[track_caller]
    pub fn read<E: Send + Sync + 'static>(&mut self) -> &[E] {
        self.need::<E>(false);
        match &mut self.port {
            Port::Direct(events) => events.read(),
            Port::Batch { slots, read, .. } => {
                let id = TypeId::of::<E>();
                // A second read in one run finds nothing new, as alone.
                if read.contains(&id) {
                    return &[];
                }
                read.push(id);
                match slots.iter().find(|s| s.id == id) {
                    Some(slot) => &items::<E>(slot)[slot.unread.clone()],
                    None => &[],
                }
            }
        }
    }

    /// As [`Events::peek`]. Needs [`SystemAccess::read_events`].
    #[track_caller]
    #[must_use]
    pub fn peek<E: Send + Sync + 'static>(&self) -> &[E] {
        self.need::<E>(false);
        match &self.port {
            Port::Direct(events) => events.peek(),
            Port::Batch { slots, .. } => match slots.iter().find(|s| s.id == TypeId::of::<E>()) {
                Some(slot) => items::<E>(slot),
                None => &[],
            },
        }
    }
}

/// The kept `E` events behind a batch's slot.
fn items<'a, E: 'static>(slot: &EventSlot<'a>) -> &'a [E] {
    slot.items.downcast_ref::<Vec<E>>().expect("event slot type")
}

pub(crate) fn name_of<F>(_: &F) -> &'static str {
    std::any::type_name::<F>()
}

/// A boxed view system.
pub(crate) type ViewFn = Box<dyn FnMut(&mut SystemView<'_>) + Send>;

/// A view system as the game keeps it: its view state, and everything else
/// it needs to run ([`ViewTask`]). Two parts, because in a batch the state
/// lends the world view to the task (`ViewGroup::run`).
pub(crate) struct ViewRunner {
    pub(crate) state: ViewState,
    pub(crate) task: ViewTask,
}

/// The part of a view system that runs: its function and declaration, and
/// what a batch gives it of its own.
pub(crate) struct ViewTask {
    name: &'static str,
    access: SystemAccess,
    f: ViewFn,
    /// In a batch: its own commands, applied when the batch ends.
    pub(crate) commands: Commands,
    /// In a batch: the events it sent, delivered when the batch ends.
    pub(crate) outbox: Outbox,
    /// In a batch: the event types it read in this run.
    pub(crate) read: Vec<TypeId>,
    /// In a batch: its part of the batch's event slots.
    pub(crate) slots: std::ops::Range<usize>,
    /// In a batch: the clocks and its random stream, taken before it starts.
    pub(crate) clock: Clock,
    pub(crate) random: Random,
    /// In a batch: how long its last run took, and the allocations made
    /// meanwhile (only counted when the batch runs in order).
    pub(crate) last_ms: f32,
    pub(crate) allocs: u64,
    /// Its last run in a batch returned (it didn't panic).
    pub(crate) finished: bool,
    /// Its usual run time in microseconds (a moving average), which decides
    /// whether a batch is worth sending to other threads. 0 before it ran.
    pub(crate) usual_us: f32,
}

/// What every system of a batch shares, read-only.
pub(crate) struct Shared<'a> {
    pub(crate) slots: &'a [EventSlot<'a>],
    pub(crate) input: &'a crate::input::Input,
    pub(crate) spatial: &'a crate::spatial::SpatialGrid,
    /// Count each system's allocations: only exact when they run in order.
    pub(crate) count_allocs: bool,
}

impl ViewRunner {
    pub(crate) fn new(name: &'static str, access: SystemAccess, f: ViewFn) -> Self {
        // Made once per registered system, for panic messages.
        let label: &'static str = Box::leak(format!("system `{name}`").into_boxed_str());
        ViewRunner {
            state: ViewState::new(access.world_access().clone()).labeled(label),
            task: ViewTask {
                name,
                access,
                f,
                commands: Commands::new(),
                outbox: Outbox::default(),
                read: Vec::new(),
                slots: 0..0,
                clock: Clock::default(),
                random: Random::new(0, 0, 0),
                last_ms: 0.0,
                allocs: 0,
                finished: false,
                usual_us: 0.0,
            },
        }
    }

    /// The declaration.
    pub(crate) fn access(&self) -> &SystemAccess {
        &self.task.access
    }

    /// Runs the system alone: lends the declared written resources out of
    /// the world, builds the view from separate `Ctx` fields, runs it and
    /// puts the resources back (also if it panics). Its commands go to
    /// `ctx.commands` and its events straight to `ctx.events`, like any
    /// system's.
    pub(crate) fn run_alone(&mut self, ctx: &mut Ctx) {
        let clock = Clock::of(ctx);
        let random = ctx.random();
        let commands = &mut ctx.commands;
        let events = &mut ctx.events;
        let input = &ctx.input;
        let spatial = &ctx.spatial;
        let alone = Alone {
            audio: &mut ctx.audio,
            camera: &mut ctx.camera,
            #[cfg(feature = "3d")]
            camera3d: &mut ctx.camera3d,
            #[cfg(feature = "physics")]
            physics: &mut ctx.physics,
            #[cfg(feature = "physics3d")]
            physics3d: &mut ctx.physics3d,
            net: &mut ctx.net.lockstep,
        };
        let ViewTask { name, access, f, .. } = &mut self.task;
        // The world's own change window, which the game set for this system.
        self.state.set_changes_since(None);
        self.state.set_change_tick(None);
        self.state.run(&mut ctx.world, |world| {
            let mut view = SystemView {
                world,
                name,
                access,
                commands,
                events: Port::Direct(events),
                input,
                clock,
                random,
                spatial,
                alone: Some(alone),
            };
            f(&mut view);
        });
    }
}

impl ViewTask {
    /// Runs the system in a batch, on any thread: `world` is its view of
    /// the shared world. Everything else is its own or read-only.
    pub(crate) fn run_shared(&mut self, world: WorldView<'_>, shared: &Shared<'_>) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("system", self.name);
        let allocs_before = shared.count_allocs.then(crate::perf::alloc_counts);
        let start = Instant::now();
        self.read.clear();
        self.finished = false;
        let mut view = SystemView {
            world,
            name: self.name,
            access: &self.access,
            commands: &mut self.commands,
            events: Port::Batch {
                slots: &shared.slots[self.slots.clone()],
                read: &mut self.read,
                outbox: &mut self.outbox,
            },
            input: shared.input,
            clock: self.clock,
            random: self.random,
            spatial: shared.spatial,
            alone: None,
        };
        (self.f)(&mut view);
        self.finished = true;
        self.last_ms = start.elapsed().as_secs_f32() * 1000.0;
        self.allocs = allocs_before.map_or(0, |before| {
            crate::perf::alloc_counts().count.saturating_sub(before.count)
        });
    }
}
