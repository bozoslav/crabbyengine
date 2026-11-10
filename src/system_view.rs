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

use std::any::TypeId;

use glam::Vec2;
use ponko_ecs::{Commands, ViewState, WorldView};

use crate::events::Events;
use crate::{Ctx, Key, MouseButton, Playing, Random, Service, Sound, SoundParams, SystemAccess};

/// The clocks, read once when the system starts.
#[derive(Clone, Copy)]
struct Clock {
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
/// use ponko::*;
/// struct Health(i32);
/// struct Died(Entity);
///
/// fn hurt(sys: &mut SystemView) {
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
///         SystemAccess::new().write::<Health>().send_events::<Died>().commands(),
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
    events: &'a mut Events,
    input: &'a crate::input::Input,
    clock: Clock,
    random: Random,
    audio: &'a mut crate::audio::Audio,
    camera: &'a mut crate::Camera2D,
    #[cfg(feature = "3d")]
    camera3d: &'a mut crate::Camera3D,
    spatial: &'a crate::SpatialGrid,
    #[cfg(feature = "physics")]
    physics: &'a mut crate::Physics,
    #[cfg(feature = "physics3d")]
    physics3d: &'a mut crate::Physics3D,
    net: &'a mut crate::Net,
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
            events: self.events,
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

    /// As [`Input::mouse_down`](crate::Input::mouse_down). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.need(Service::Input, false);
        self.input.mouse_down(button)
    }

    /// As [`Input::mouse_pressed`](crate::Input::mouse_pressed). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.need(Service::Input, false);
        self.input.mouse_pressed(button)
    }

    /// As [`Input::mouse_released`](crate::Input::mouse_released). Needs `Service::Input`.
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

    /// As [`Input::action_down`](crate::Input::action_down). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_down(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.down(action)
    }

    /// As [`Input::action_pressed`](crate::Input::action_pressed). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_pressed(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.pressed(action)
    }

    /// As [`Input::action_released`](crate::Input::action_released). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_released(&self, action: impl crate::ActionRef) -> bool {
        self.need(Service::Input, false);
        self.input.actions.released(action)
    }

    /// As [`Input::action_axis`](crate::Input::action_axis). Needs `Service::Input`.
    #[track_caller]
    #[must_use]
    pub fn action_axis(&self, action: impl crate::ActionRef) -> f32 {
        self.need(Service::Input, false);
        self.input.actions.axis(action)
    }

    /// As [`Input::action_axis2`](crate::Input::action_axis2). Needs `Service::Input`.
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

    /// As [`Audio::play_ex`](crate::Audio::play_ex). Needs `Service::Audio` written.
    #[track_caller]
    pub fn play_sound_ex(&mut self, sound: &Sound, params: SoundParams) -> Playing {
        self.need(Service::Audio, true);
        self.audio.play_ex(sound, params)
    }

    // ---- cameras, spatial grid, physics, net -----------------------------------------------

    /// The 2D camera ([`Ctx::camera`]). Needs `Service::Camera`.
    #[track_caller]
    #[must_use]
    pub fn camera(&self) -> &crate::Camera2D {
        self.need(Service::Camera, false);
        self.camera
    }

    /// The 2D camera to move. Needs `Service::Camera` written.
    #[track_caller]
    pub fn camera_mut(&mut self) -> &mut crate::Camera2D {
        self.need(Service::Camera, true);
        self.camera
    }

    /// The 3D camera ([`Ctx::camera3d`], feature `3d`). Needs `Service::Camera`.
    #[cfg(feature = "3d")]
    #[track_caller]
    #[must_use]
    pub fn camera3d(&self) -> &crate::Camera3D {
        self.need(Service::Camera, false);
        self.camera3d
    }

    /// The 3D camera to move (feature `3d`). Needs `Service::Camera` written.
    #[cfg(feature = "3d")]
    #[track_caller]
    pub fn camera3d_mut(&mut self) -> &mut crate::Camera3D {
        self.need(Service::Camera, true);
        self.camera3d
    }

    /// The spatial grid ([`Ctx::spatial`]). Needs `Service::Spatial`.
    #[track_caller]
    #[must_use]
    pub fn spatial(&self) -> &crate::SpatialGrid {
        self.need(Service::Spatial, false);
        self.spatial
    }

    /// The 2D physics world (feature `physics`): ray casts and overlap
    /// queries. Needs `Service::Physics`.
    #[cfg(feature = "physics")]
    #[track_caller]
    #[must_use]
    pub fn physics(&self) -> &crate::Physics {
        self.need(Service::Physics, false);
        self.physics
    }

    /// The 2D physics world to change: impulses, gravity. Needs
    /// `Service::Physics` written.
    #[cfg(feature = "physics")]
    #[track_caller]
    pub fn physics_mut(&mut self) -> &mut crate::Physics {
        self.need(Service::Physics, true);
        self.physics
    }

    /// The 3D physics world (feature `physics3d`). Needs `Service::Physics3D`.
    #[cfg(feature = "physics3d")]
    #[track_caller]
    #[must_use]
    pub fn physics3d(&self) -> &crate::Physics3D {
        self.need(Service::Physics3D, false);
        self.physics3d
    }

    /// The 3D physics world to change. Needs `Service::Physics3D` written.
    #[cfg(feature = "physics3d")]
    #[track_caller]
    pub fn physics3d_mut(&mut self) -> &mut crate::Physics3D {
        self.need(Service::Physics3D, true);
        self.physics3d
    }

    /// Lockstep networking ([`Ctx::net`]). Needs `Service::Net`.
    #[track_caller]
    #[must_use]
    pub fn net(&self) -> &crate::Net {
        self.need(Service::Net, false);
        self.net
    }

    /// Lockstep networking, to send input. Needs `Service::Net` written.
    #[track_caller]
    pub fn net_mut(&mut self) -> &mut crate::Net {
        self.need(Service::Net, true);
        self.net
    }
}

/// The declared event types of a view system: from [`SystemView::events`].
///
/// Same visibility rules as [`Events`]: see the guide's "Systems and the
/// frame" chapter, and [`SystemAccess::conflicts_before`] for what a
/// parallel executor must keep.
pub struct ViewEvents<'a> {
    name: &'static str,
    access: &'a SystemAccess,
    events: &'a mut Events,
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
    pub fn send<E: 'static>(&mut self, event: E) {
        self.need::<E>(true);
        self.events.send(event);
    }

    /// As [`Events::read`]. Needs [`SystemAccess::read_events`].
    #[track_caller]
    pub fn read<E: 'static>(&mut self) -> &[E] {
        self.need::<E>(false);
        self.events.read()
    }

    /// As [`Events::peek`]. Needs [`SystemAccess::read_events`].
    #[track_caller]
    #[must_use]
    pub fn peek<E: 'static>(&self) -> &[E] {
        self.need::<E>(false);
        self.events.peek()
    }
}

pub(crate) fn name_of<F>(_: &F) -> &'static str {
    std::any::type_name::<F>()
}

/// The `&mut Ctx` system that runs view system `f`: lends the declared
/// written resources out of the world, builds the view from separate `Ctx`
/// fields, runs `f` and puts the resources back (also if `f` panics).
/// Commands apply after it, like after any system.
pub(crate) fn wrap<F>(name: &'static str, access: SystemAccess, mut f: F) -> impl FnMut(&mut Ctx) + 'static
where
    F: FnMut(&mut SystemView<'_>) + 'static,
{
    // Made once per registered system, for panic messages.
    let label: &'static str = Box::leak(format!("system `{name}`").into_boxed_str());
    let mut state = ViewState::new(access.world_access().clone()).labeled(label);
    move |ctx: &mut Ctx| {
        let clock = Clock {
            dt: ctx.dt(),
            fixed_dt: ctx.time.fixed_dt(),
            sim_tick: ctx.time.sim_tick(),
            time: ctx.time.elapsed(),
            frame: ctx.time.frame(),
        };
        let random = ctx.random();
        let commands = &mut ctx.commands;
        let events = &mut ctx.events;
        let input = &ctx.input;
        let audio = &mut ctx.audio;
        let camera = &mut ctx.camera;
        #[cfg(feature = "3d")]
        let camera3d = &mut ctx.camera3d;
        let spatial = &ctx.spatial;
        #[cfg(feature = "physics")]
        let physics = &mut ctx.physics;
        #[cfg(feature = "physics3d")]
        let physics3d = &mut ctx.physics3d;
        let net = &mut ctx.net;
        let access = &access;
        state.run(&mut ctx.world, |world| {
            let mut view = SystemView {
                world,
                name,
                access,
                commands,
                events,
                input,
                clock,
                random,
                audio,
                camera,
                #[cfg(feature = "3d")]
                camera3d,
                spatial,
                #[cfg(feature = "physics")]
                physics,
                #[cfg(feature = "physics3d")]
                physics3d,
                net,
            };
            f(&mut view);
        });
    }
}
