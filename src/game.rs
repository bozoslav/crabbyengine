//! [`Game`]: window creation and the main loop.

use std::time::Duration;
use web_time::Instant;

use glam::Vec2;

use ponko_ecs::World;

#[cfg(feature = "window")]
mod window;

use crate::control::{self, Control};
use crate::ctx::Ctx;
use crate::perf::SystemTiming;
use crate::systems::{animate_sprites, draw_sprites, draw_tilemaps, rebuild_spatial_grid};
use crate::time::{Clock, DEFAULT_FIXED_HZ, FrameTiming};

/// Trackpads report scrolling in pixels; this many logical pixels count as one
/// wheel "notch" so `mouse_wheel()` means roughly the same on every device.
const PIXELS_PER_SCROLL_LINE: f32 = 20.0;

/// How often to check whether a hidden/minimized window is visible again.
const OCCLUDED_POLL: Duration = Duration::from_millis(100);

/// Sequential execution buckets, ordered independently of builder call order.
///
/// `Input` through `SimLast` run once per fixed tick (also headlessly).
/// `Frame` through `Draw` run once per rendered frame. Systems in the same
/// phase run in registration order, applying deferred commands after each one.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Fixed-tick input preparation, before all other simulation work.
    Input,
    /// Fixed-tick gameplay that feeds the physics step.
    BeforePhysics,
    /// Fixed-tick physics integration.
    Physics,
    /// Fixed-tick gameplay observing the current physics result.
    AfterPhysics,
    /// Final fixed-tick work, before advancing the tick counter.
    SimLast,
    /// Per-frame gameplay, after the due simulation ticks.
    Frame,
    /// Per-frame pose work after frame gameplay. With the `3d` feature, the
    /// built-in animator and particle updates ([`Systems::UPDATE_ANIMATORS`],
    /// [`Systems::UPDATE_PARTICLES`]) run at the end of this phase, after
    /// your systems in it.
    Animate,
    /// Per-frame work after animation and before drawing.
    Late,
    /// Per-frame drawing after final positions are available. With the `3d`
    /// feature, positional audio ([`Systems::UPDATE_AUDIO3D`]) runs at the
    /// end of this phase, after the drawing systems, so sound follows where
    /// everything ended up this frame.
    Draw,
}

/// A reusable module of systems and startup work.
///
/// `build` runs immediately during registration; `setup` runs once after runtime
/// initialization, interleaved with explicit setup callbacks in registration order.
/// Plugins need no registry, dependency graph, or additional runtime feature.
/// Each system names its [`Phase`], so the order in which plugins are added
/// only matters between systems of the same phase.
///
/// ```
/// use ponko::{Ctx, Game, Phase, Plugin};
///
/// struct Score;
/// impl Plugin for Score {
///     fn build(&self, game: Game) -> Game {
///         game.add(Phase::AfterPhysics, |ctx| *ctx.world.resource_mut::<u32>() += 1)
///     }
///     fn setup(&self, ctx: &mut Ctx) {
///         ctx.world.insert_resource(0u32);
///     }
/// }
///
/// let world = Game::new("score", 1, 1).plugin(Score).setup(|_| {}).run_headless(3);
/// assert_eq!(*world.resource::<u32>(), 3);
/// ```
pub trait Plugin {
    /// Registers systems and options on the existing builder.
    fn build(&self, game: Game) -> Game;
    /// Initializes this module before systems start. Commands apply afterward.
    fn setup(&self, _ctx: &mut Ctx) {}
}

/// An opaque registration handle. Registration IDs are unique across game builders
/// and never reused. Constants in [`Systems`] identify all registrations of a
/// built-in function. IDs are not persistence identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SystemId(u32);

/// Stable IDs for built-in systems. Feature-gated systems are only installed
/// when enabled/configured; removing an absent ID is a no-op.
pub struct Systems;

impl Systems {
    /// Fixed-tick 2D physics step.
    pub const PHYSICS: SystemId = SystemId(1);
    /// Fixed-tick 3D physics step.
    pub const PHYSICS3D: SystemId = SystemId(2);
    /// Per-frame interpolated 3D physics transforms.
    pub const INTERPOLATE_PHYSICS3D: SystemId = SystemId(3);
    /// Spatial grid rebuild.
    pub const REBUILD_SPATIAL_GRID: SystemId = SystemId(4);
    /// Sprite animation in the drawing bucket.
    pub const ANIMATE_SPRITES: SystemId = SystemId(5);
    /// Tilemap drawing.
    pub const DRAW_TILEMAPS: SystemId = SystemId(6);
    /// Sprite drawing.
    pub const DRAW_SPRITES: SystemId = SystemId(7);
    /// Attachment transforms, when explicitly registered.
    pub const FOLLOW_ATTACHMENTS: SystemId = SystemId(8);
    /// Navigation update, when explicitly registered.
    pub const STEP_NAVMESH3D: SystemId = SystemId(9);
    /// Advances every [`Animator`](crate::Animator) and poses its skeleton
    /// (feature `3d`). Stays at the end of [`Phase::Animate`].
    pub const UPDATE_ANIMATORS: SystemId = SystemId(10);
    /// Advances particle emitters and trails (feature `3d`). Stays at the
    /// end of [`Phase::Animate`], after the animators.
    pub const UPDATE_PARTICLES: SystemId = SystemId(11);
    /// Moves positional sounds to this frame's listener and emitters
    /// (feature `3d`). Stays at the end of [`Phase::Draw`], after the
    /// drawing systems.
    pub const UPDATE_AUDIO3D: SystemId = SystemId(12);
}

impl SystemId {
    /// The id as a plain number, for C/C++ and tools. The [`Systems`]
    /// constants keep their numbers; other ids are only valid in this run.
    #[must_use]
    pub fn to_bits(self) -> u32 {
        self.0
    }

    /// An id from its number, as returned by [`to_bits`](Self::to_bits).
    #[must_use]
    pub fn from_bits(bits: u32) -> Self {
        SystemId(bits)
    }
}

/// The id of built-in system function `name` (a type name), with the name
/// it is listed under: the old crate-root spellings count as the same system.
fn builtin_id(name: &str) -> Option<(SystemId, &'static str)> {
    /// The function in `ponko::systems` and its old crate-root spelling.
    macro_rules! both {
        ($name:ident) => {{
            #[allow(deprecated)]
            let root = std::any::type_name_of_val(&crate::systems::at_root::$name);
            [std::any::type_name_of_val(&crate::systems::$name), root]
        }};
    }
    #[cfg(feature = "3d")]
    let one = |name: &'static str| [name, name];
    let entries = [
        (both!(animate_sprites), Systems::ANIMATE_SPRITES),
        (both!(draw_tilemaps), Systems::DRAW_TILEMAPS),
        (both!(draw_sprites), Systems::DRAW_SPRITES),
        (both!(rebuild_spatial_grid), Systems::REBUILD_SPATIAL_GRID),
        (both!(follow_attachments), Systems::FOLLOW_ATTACHMENTS),
        #[cfg(feature = "physics")]
        (both!(step_physics), Systems::PHYSICS),
        #[cfg(feature = "physics3d")]
        (both!(step_physics3d), Systems::PHYSICS3D),
        #[cfg(feature = "physics3d")]
        (both!(interpolate_physics3d), Systems::INTERPOLATE_PHYSICS3D),
        #[cfg(feature = "navmesh")]
        (both!(step_navmesh3d), Systems::STEP_NAVMESH3D),
        #[cfg(feature = "3d")]
        (
            one(std::any::type_name_of_val(&crate::systems::update_animators)),
            Systems::UPDATE_ANIMATORS,
        ),
        #[cfg(feature = "3d")]
        (
            one(std::any::type_name_of_val(&crate::systems::update_particles)),
            Systems::UPDATE_PARTICLES,
        ),
        #[cfg(feature = "3d")]
        (
            one(std::any::type_name_of_val(&crate::systems::update_audio3d)),
            Systems::UPDATE_AUDIO3D,
        ),
    ];
    entries
        .into_iter()
        .find_map(|(names, id)| names.contains(&name).then_some((id, names[0])))
}

/// A game: a window plus the loop that drives your code.
///
/// ```no_run
/// use ponko::*;
///
/// Game::new("My Game", 800, 600).run(|ctx| {
///     // Setup: runs once, after the window exists.
///     let crab = ctx.load_texture("crab.png");
///     let mut pos = vec2(400.0, 300.0);
///
///     // Returned closure: runs every frame.
///     move |ctx: &mut Ctx| {
///         if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
///         if ctx.key_pressed(Key::Escape) { ctx.quit(); }
///         ctx.draw(&crab, pos);
///     }
/// });
/// ```
///
/// With the ECS, the game is entities plus systems:
///
/// ```no_run
/// use ponko::*;
///
/// struct Vel(Vec2);
///
/// fn movement(ctx: &mut Ctx) {
///     let dt = ctx.dt();
///     ctx.world.query::<(&mut Transform2D, &Vel)>().for_each(|(t, v)| t.position += v.0 * dt);
/// }
///
/// Game::new("Crabs", 800, 600).sim_system(movement).run(|ctx| {
///     let crab = ctx.load_texture("crab.png");
///     ctx.world.spawn((Transform2D::at(vec2(100.0, 100.0)), Sprite::new(crab), Vel(vec2(50.0, 0.0))));
/// });
/// ```
#[must_use = "a Game does nothing until `.run()` or `.run_headless()`"]
pub struct Game {
    setups: Vec<SetupFn>,
    title: String,
    width: u32,
    height: u32,
    fixed_hz: u32,
    /// `fixed_hz` was called: `physics3d` keeps that rate.
    fixed_hz_set: bool,
    seed: u64,
    vsync: bool,
    /// Samples per pixel of the 3D frame (see `msaa`).
    msaa: u32,
    fullscreen: bool,
    sim_systems: Vec<System>,
    systems: Vec<System>,
    late_systems: Vec<System>,
    /// Built-in drawing; runs after every other system so it sees final positions.
    draw_systems: Vec<System>,
    /// Cell size from `spatial_grid`, applied to `ctx.spatial` when the game starts.
    spatial_cell: Option<f32>,
    /// Gravity from `physics`, applied to `ctx.physics` when the game starts.
    #[cfg(feature = "physics")]
    gravity: Option<Vec2>,
    /// Gravity from `physics3d`, applied to `ctx.physics3d` when the game starts.
    #[cfg(feature = "physics3d")]
    gravity3d: Option<crate::Vec3>,
    /// The lockstep session from `lockstep`, moved into `ctx.net` at start.
    net: Option<crate::Net>,
    /// Open the control endpoint (also on with `PONKO_CONTROL=1`).
    control: bool,
    /// Reload asset and data files when they change (`PONKO_HOT_RELOAD` overrides).
    hot_reload: bool,
    inspector: bool,
    /// Scheduled input from `input_scenario`, fed at the start of sim ticks.
    input_scenario: Option<crate::InputScenario>,
    /// Whether tools may start input scenarios through the control endpoint.
    input_injection: bool,
}

type SetupFn = Box<dyn FnOnce(&mut Ctx)>;

/// A game with stored startup callbacks and closure-free run methods.
///
/// Returned by [`Game::setup`], or by [`Game::configured`] for a game with
/// no setup of its own. Configure systems and window options on `Game`
/// first, then register one or more setup callbacks. This separate type lets
/// existing `Game::run(setup)` calls keep compiling: Rust cannot overload
/// inherent methods by argument count.
#[must_use = "a configured game does nothing until run or run_headless"]
pub struct ConfiguredGame {
    game: Game,
}

fn run_setups(setups: Vec<SetupFn>, ctx: &mut Ctx) {
    for setup in setups {
        setup(ctx);
        ctx.apply_commands();
    }
}

impl ConfiguredGame {
    /// Appends a startup callback. Callbacks run once in registration order,
    /// after runtime initialization and before any systems. Deferred commands
    /// are applied after each callback, including when running zero ticks.
    pub fn setup(mut self, setup: impl FnOnce(&mut Ctx) + 'static) -> Self {
        self.game.setups.push(Box::new(setup));
        self
    }

    /// Registers a plugin after earlier startup callbacks. Its systems are
    /// configured immediately and its setup joins the same ordered queue.
    pub fn plugin(mut self, plugin: impl Plugin + 'static) -> Self {
        self.game = self.game.plugin(plugin);
        self
    }

    /// Opens the window and runs with the stored setup callbacks.
    /// Has the same platform requirements and panics as [`Game::run`].
    pub fn run(self) {
        self.game.run(|_| {});
    }

    /// Runs fixed ticks without a window or GPU, returning the world.
    /// Frame systems do not run; see [`Game::run_headless`].
    pub fn run_headless(self, ticks: u64) -> World {
        self.game.run_headless(ticks, |_| {})
    }

    /// Profiles fixed ticks using the stored setup callbacks.
    /// See [`Game::profile_headless`] for warmup and report semantics.
    pub fn profile_headless(self, warmup: u64, ticks: u64) -> (World, crate::PerfReport) {
        self.game.profile_headless(warmup, ticks, |_| {})
    }
}

type SystemFn = Box<dyn FnMut(&mut Ctx)>;

/// A system: a named function that runs over the world.
struct System {
    key: SystemId,
    builtin: Option<SystemId>,
    name: &'static str,
    stage: Phase,
    run: SystemFn,
    /// Simulation systems only: run on ticks where `tick % every == phase`.
    every: u32,
    phase: u32,
    /// Unique per system: its read position for events.
    id: u32,
    /// Declared access from `add_with_access` or `add_view`; `None` is exclusive.
    access: Option<crate::SystemAccess>,
    /// Stays at the end of its phase: systems added later go before it.
    /// Set for the built-ins that must see what the phase's other systems did.
    last: bool,
    /// Where this system last finished: its `Added` and `Changed` filters
    /// and `removed` report what happened after it.
    changes_seen: ChangesSeen,
    /// A view system (`add_view`): it can't reach anything undeclared.
    enforced: bool,
}

/// A system's place in the world's change history: the change tick at which
/// it last finished, and the world that tick belongs to.
#[derive(Clone, Copy, Default)]
struct ChangesSeen {
    /// [`World::id`] of the world the system last ran in; 0 before its first run.
    world: u64,
    tick: crate::Tick,
}

impl ChangesSeen {
    /// The tick to compare `world`'s changes against. A system that last
    /// ran in another world (the game replaced `ctx.world`) has seen
    /// nothing of this one: the new world's ticks start over, so the old
    /// tick would hide whatever was stamped before the new world reached it.
    fn in_world(self, world: &crate::World) -> crate::Tick {
        if self.world == world.id() {
            self.tick
        } else {
            crate::Tick::default()
        }
    }
}

/// Ids for [`System::id`]; 0 means "outside any system".
fn next_system_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1024);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    assert!(id >= 1024, "system ID space exhausted");
    id
}

/// This frame's timing: the clock at the current sim speed (0 while
/// paused), plus any single steps requested with `ctx.time.step()`.
fn frame_timing(clock: &mut Clock, ctx: &mut Ctx, frame_dt: f64) -> FrameTiming {
    let mut timing = clock.advance(frame_dt, ctx.effective_sim_speed());
    timing.fixed_steps += ctx.take_pending_steps();
    timing
}

fn end_timings(ctx: &mut Ctx) {
    for t in &mut ctx.perf.timings {
        t.avg_ms += (t.last_ms - t.avg_ms) * 0.05;
    }
}

/// Drops the removals of tracked components that every one of `systems` has
/// already been shown (those logged before the oldest of their last runs),
/// so the logs don't grow. A system that hasn't run in this world yet keeps
/// everything: while one never runs (the simulation is paused, say), the
/// removal logs of tracked components grow with every removal.
fn forget_seen_removals<'a>(ctx: &mut Ctx, systems: impl Iterator<Item = &'a System>) {
    if let Some(oldest) = systems.map(|s| s.changes_seen.in_world(&ctx.world)).min() {
        ctx.world.forget_removed(oldest);
    }
}

/// Runs one system, applies its commands, and records how long both took.
#[inline]
fn run_one(s: &mut System, ctx: &mut Ctx, timing_index: usize) {
    #[cfg(feature = "profiling")]
    puffin::profile_scope!("system", s.name);
    let allocs_before = ctx.perf.state.on.then(crate::alloc_counts);
    let start = Instant::now();
    ctx.system_index = timing_index;
    ctx.events.reader = s.id;
    // Change filters in this system report what happened since it last ran.
    ctx.world.set_changes_since(s.changes_seen.in_world(&ctx.world));
    ctx.sync_services();
    // View systems can't reach anything undeclared; only `Ctx` systems
    // with a declaration need the recorder.
    #[cfg(debug_assertions)]
    let check = s.access.as_ref().filter(|_| !s.enforced);
    #[cfg(debug_assertions)]
    if check.is_some() {
        crate::access::begin_check(ctx);
    }
    (s.run)(ctx);
    #[cfg(debug_assertions)]
    if let Some(access) = check {
        crate::access::end_check(ctx, s.name, access);
    }
    ctx.events.reader = 0;
    ctx.apply_commands();
    // The system's own changes (and its commands, just applied) belong to
    // the tick that ends here, so it won't be shown them next time; every
    // other system will. Code outside systems sees everything.
    // If the system replaced the world, this is the new world's tick.
    s.changes_seen = ChangesSeen {
        world: ctx.world.id(),
        tick: ctx.world.advance_change_tick(),
    };
    ctx.world.set_changes_since(crate::Tick::default());
    ctx.perf.timings[timing_index].last_ms += start.elapsed().as_secs_f32() * 1000.0;
    if let Some(before) = allocs_before {
        let made = crate::alloc_counts().count.saturating_sub(before.count);
        ctx.perf.state.add_system_allocs(timing_index, made);
    }
}

impl System {
    fn new<F: FnMut(&mut Ctx) + 'static>(f: F) -> Self {
        let id = next_system_id();
        let builtin = builtin_id(std::any::type_name::<F>());
        System {
            key: SystemId(id),
            builtin: builtin.map(|b| b.0),
            name: builtin.map_or(std::any::type_name::<F>(), |b| b.1),
            stage: Phase::AfterPhysics,
            run: hot(f),
            every: 1,
            phase: 0,
            id,
            access: None,
            last: false,
            changes_seen: ChangesSeen::default(),
            enforced: false,
        }
    }

    /// A built-in that stays at the end of its phase.
    #[cfg(feature = "3d")]
    fn last<F: FnMut(&mut Ctx) + 'static>(f: F) -> Self {
        let mut s = System::new(f);
        s.last = true;
        s
    }
}

/// Boxes system `f`. With the `hotpatch` feature, each call goes through
/// subsecond's jump table, so a patched version of the system (and of what
/// it calls) runs from the next call on.
#[cfg(not(feature = "hotpatch"))]
fn hot<F: FnMut(&mut Ctx) + 'static>(f: F) -> SystemFn {
    Box::new(f)
}

#[cfg(feature = "hotpatch")]
fn hot<F: FnMut(&mut Ctx) + 'static>(mut f: F) -> SystemFn {
    /// Calls the system. A zero-sized function item, instantiated in the
    /// game's crate, so subsecond finds its patched version in the jump
    /// table (a pointer-sized `&mut F` would be taken for a fn pointer).
    fn call<F: FnMut(&mut Ctx)>(f: &mut F, ctx: &mut Ctx) {
        f(ctx);
    }
    Box::new(move |ctx: &mut Ctx| {
        // A patch that changed code "above" this system (in its caller)
        // can't run in place; skip this call, the next one is current.
        let _ = subsecond::HotFn::current(call::<F>).try_call((&mut f, ctx));
    })
}

/// With the `hotpatch` feature: connects to `dx serve --hotpatch` (when the
/// game was started by it) so patches are applied as they arrive.
fn connect_hotpatch() {
    #[cfg(feature = "hotpatch")]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            // stdout: `dx` shows stderr lines as errors.
            subsecond::register_handler(std::sync::Arc::new(|| println!("ponko: code patched")));
            dioxus_devtools::connect_subsecond();
        });
    }
}

/// What `setup` may return: nothing, or a closure to run every frame.
///
/// Implemented for `()` and for every `FnMut(&mut Ctx)`.
#[doc(hidden)]
pub trait SetupResult {
    #[doc(hidden)]
    fn into_frame_fn(self) -> Option<SystemFn>;
}

impl SetupResult for () {
    fn into_frame_fn(self) -> Option<SystemFn> {
        None
    }
}

impl<F: FnMut(&mut Ctx) + 'static> SetupResult for F {
    fn into_frame_fn(self) -> Option<SystemFn> {
        Some(Box::new(self))
    }
}

impl Game {
    /// Describes a game window with a title and size in (logical) pixels.
    ///
    /// Nothing opens until you call [`run`](Self::run).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Space Crab", 1280, 720).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        #[cfg_attr(not(feature = "3d"), allow(unused_mut))]
        let mut game = Self {
            setups: Vec::new(),
            title: title.into(),
            width,
            height,
            fixed_hz: DEFAULT_FIXED_HZ,
            fixed_hz_set: false,
            seed: crate::random::DEFAULT_SEED,
            vsync: true,
            msaa: 4,
            fullscreen: false,
            sim_systems: Vec::new(),
            systems: Vec::new(),
            late_systems: Vec::new(),
            draw_systems: vec![
                System::new(animate_sprites),
                System::new(draw_tilemaps),
                System::new(draw_sprites),
            ],
            spatial_cell: None,
            #[cfg(feature = "physics")]
            gravity: None,
            #[cfg(feature = "physics3d")]
            gravity3d: None,
            net: None,
            control: control::env_flag(control::ENV_CONTROL),
            hot_reload: cfg!(debug_assertions),
            inspector: cfg!(debug_assertions),
            input_scenario: None,
            input_injection: false,
        };
        #[cfg(feature = "3d")]
        {
            game.insert_system(Phase::Animate, System::last(crate::systems::update_animators));
            game.insert_system(Phase::Animate, System::last(crate::systems::update_particles));
            game.insert_system(Phase::Draw, System::last(crate::systems::update_audio3d));
        }
        game
    }

    /// Registers a system in an explicit execution phase.
    ///
    /// Compatibility mappings: `sim_system` (including interval/named forms)
    /// uses `AfterPhysics`, `system` uses `Frame`, and `late_system` uses `Late`.
    /// Built-in physics always uses `Physics`; use `BeforePhysics` for logic
    /// that must affect the current physics step. This is sequential scheduling.
    ///
    /// ```
    /// use ponko::{Game, Phase};
    /// let world = Game::new("server", 1, 1)
    ///     .add(Phase::BeforePhysics, |_| {})
    ///     .add(Phase::AfterPhysics, |_| {})
    ///     .run_headless(1, |_| {});
    /// assert!(world.is_empty());
    /// ```
    pub fn add(mut self, phase: Phase, system: impl FnMut(&mut Ctx) + 'static) -> Self {
        self.add_system(phase, system);
        self
    }

    /// Registers a system and returns its removal handle. Unlike the chainable
    /// [`add`](Self::add), this method borrows the builder to return an ID.
    pub fn add_system(&mut self, phase: Phase, system: impl FnMut(&mut Ctx) + 'static) -> SystemId {
        self.insert_system(phase, System::new(system))
    }

    /// Like [`add`](Self::add), with a declaration of what the system
    /// reads and writes. Opt-in metadata for tools and a future scheduler:
    /// the system still runs in order with the whole `Ctx`. In debug builds,
    /// using anything beyond the declaration panics after the system runs.
    /// See [`SystemAccess`] and [`access_report`](Self::access_report).
    ///
    /// [`SystemAccess`]: crate::SystemAccess
    pub fn add_with_access(
        mut self,
        phase: Phase,
        access: crate::SystemAccess,
        system: impl FnMut(&mut Ctx) + 'static,
    ) -> Self {
        let mut s = System::new(system);
        s.access = Some(access);
        self.insert_system(phase, s);
        self
    }

    /// Registers a view system: it declares what it reads and writes, and
    /// gets a [`SystemView`](crate::SystemView) that offers exactly that,
    /// instead of the whole `Ctx`. Anything undeclared is unreachable (its
    /// accessors panic, in every build), so the declaration is enforced by
    /// the view, not trusted. Systems still run one at a time, in order;
    /// view systems are the only ones a future parallel executor may run
    /// together. See [`SystemAccess`](crate::SystemAccess) and the guide's
    /// "Declared system access" chapter.
    ///
    /// ```
    /// use ponko::*;
    /// struct Pos(f32);
    /// struct Vel(f32);
    ///
    /// fn movement(sys: &mut SystemView) {
    ///     let dt = sys.dt();
    ///     sys.world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0 * dt);
    /// }
    ///
    /// let world = Game::new("view", 1, 1)
    ///     .add_view(
    ///         Phase::AfterPhysics,
    ///         SystemAccess::new().write::<Pos>().read::<Vel>().read_service(Service::Time),
    ///         movement,
    ///     )
    ///     .run_headless(60, |ctx| {
    ///         ctx.world.spawn((Pos(0.0), Vel(1.0)));
    ///     });
    /// # let _ = world;
    /// ```
    ///
    /// # Panics
    /// If `access` can't be a view's: exclusive, items without a Rust type
    /// (the C/C++ path), or a service the view doesn't offer (`gfx`, `ui`,
    /// `assets`, `perf`, or writing `input`, `time` or `spatial`). Those
    /// systems take `&mut Ctx`: use [`add_with_access`](Self::add_with_access).
    #[track_caller]
    pub fn add_view(
        mut self,
        phase: Phase,
        access: crate::SystemAccess,
        system: impl FnMut(&mut crate::SystemView<'_>) + 'static,
    ) -> Self {
        let name = crate::system_view::name_of(&system);
        if let Some(problem) = access.view_problem() {
            panic!("Game::add_view: system `{name}` {problem}");
        }
        let mut s = System::new(crate::system_view::wrap(name, access.clone(), system));
        s.name = name;
        s.builtin = None;
        s.access = Some(access);
        s.enforced = true;
        self.insert_system(phase, s);
        self
    }

    /// For language bindings (the C API): `named_add` with a declaration
    /// (checked in debug builds, like [`add_with_access`](Self::add_with_access)).
    #[doc(hidden)]
    #[track_caller]
    pub fn named_add_with_access(
        mut self,
        phase: Phase,
        name: &'static str,
        ticks: u32,
        access: crate::SystemAccess,
        system: impl FnMut(&mut Ctx) + 'static,
    ) -> Self {
        let mut s = self.build_named_system(phase, name, ticks, system);
        s.access = Some(access);
        self.insert_system(phase, s);
        self
    }

    /// Every system in run order with its declared access, and which
    /// declared systems in the same phase could run together.
    #[must_use]
    pub fn access_report(&self) -> crate::AccessReport {
        let systems = (self.sim_systems.iter())
            .chain(&self.systems)
            .chain(&self.late_systems)
            .chain(&self.draw_systems)
            .map(|s| crate::access::ReportedSystem {
                name: s.name,
                phase: s.stage,
                access: s.access.clone(),
                enforced: s.enforced,
            })
            .collect();
        crate::AccessReport::new(systems)
    }

    /// For language bindings (the C API): `add` with a name for timings and
    /// tools, and an interval in ticks for fixed-tick phases (like
    /// `sim_system_every`). Frame phases need `ticks == 1`.
    #[doc(hidden)]
    #[track_caller]
    pub fn named_add(
        mut self,
        phase: Phase,
        name: &'static str,
        ticks: u32,
        system: impl FnMut(&mut Ctx) + 'static,
    ) -> Self {
        self.named_add_system(phase, name, ticks, system);
        self
    }

    /// For language bindings (the C API): [`named_add`](Self::named_add)
    /// that returns the system's removal handle, like `add_system`.
    #[doc(hidden)]
    #[track_caller]
    pub fn named_add_system(
        &mut self,
        phase: Phase,
        name: &'static str,
        ticks: u32,
        system: impl FnMut(&mut Ctx) + 'static,
    ) -> SystemId {
        let s = self.build_named_system(phase, name, ticks, system);
        self.insert_system(phase, s)
    }

    /// For language bindings (the C API): a startup callback on the builder
    /// itself. It joins the same ordered queue as [`setup`](Self::setup) and
    /// plugin setup, and runs before the closure passed to `run`.
    #[doc(hidden)]
    pub fn add_setup(mut self, setup: impl FnOnce(&mut Ctx) + 'static) -> Self {
        self.setups.push(Box::new(setup));
        self
    }

    #[track_caller]
    fn build_named_system(
        &self,
        phase: Phase,
        name: &'static str,
        ticks: u32,
        system: impl FnMut(&mut Ctx) + 'static,
    ) -> System {
        assert!(ticks > 0, "Game::named_add: the interval must be at least 1 tick");
        assert!(
            ticks == 1 || phase <= Phase::SimLast,
            "Game::named_add: only fixed-tick phases take an interval"
        );
        let mut s = System::new(system);
        s.name = name;
        if ticks > 1 {
            let same = self.sim_systems.iter().filter(|s| s.every == ticks).count() as u32;
            s.every = ticks;
            s.phase = same % ticks;
        }
        s
    }

    fn insert_system(&mut self, phase: Phase, mut s: System) -> SystemId {
        let id = s.key;
        s.stage = phase;
        let systems = match phase {
            Phase::Input | Phase::BeforePhysics | Phase::Physics | Phase::AfterPhysics | Phase::SimLast => {
                &mut self.sim_systems
            }
            Phase::Frame | Phase::Animate => &mut self.systems,
            Phase::Late => &mut self.late_systems,
            Phase::Draw => &mut self.draw_systems,
        };
        // At the end of the phase; before the built-ins that stay last in it.
        let last = s.last;
        let index = systems.partition_point(|x| x.stage < phase || (x.stage == phase && (last || !x.last)));
        systems.insert(index, s);
        id
    }

    /// Stores startup work on the builder. Configure systems and options first;
    /// the returned [`ConfiguredGame`] has plain `.run()` and
    /// `.run_headless(ticks)` methods, and accepts additional `.setup(...)` calls.
    /// Callbacks run once in order, after initialization and before systems.
    ///
    /// Runtime branches can choose different capturing closures without needing
    /// a common returned frame-closure type:
    ///
    /// ```
    /// use ponko::Game;
    /// let headless = true;
    /// let game = Game::new("Example", 800, 600);
    /// let game = if headless {
    ///     let label = String::from("server");
    ///     game.setup(move |ctx| ctx.world.insert_resource(label))
    /// } else {
    ///     let size = 42usize;
    ///     game.setup(move |ctx| ctx.world.insert_resource(size.to_string()))
    /// };
    /// let world = game.run_headless(0);
    /// assert_eq!(world.resource::<String>(), "server");
    /// ```
    pub fn setup(mut self, setup: impl FnOnce(&mut Ctx) + 'static) -> ConfiguredGame {
        self.setups.push(Box::new(setup));
        ConfiguredGame { game: self }
    }

    /// The game as it is, with the closure-free `.run()` and
    /// `.run_headless(ticks)` of [`ConfiguredGame`], for a game whose startup
    /// is all in its plugins and systems (or that needs none). It adds no
    /// setup; [`setup`](Self::setup) does the same with one.
    ///
    /// ```
    /// use ponko::{Ctx, Game, Plugin};
    /// struct Score(u32);
    /// struct Scoring;
    /// impl Plugin for Scoring {
    ///     fn build(&self, game: Game) -> Game {
    ///         game.sim_system(|ctx| ctx.world.resource_mut::<Score>().0 += 1)
    ///     }
    ///     fn setup(&self, ctx: &mut Ctx) {
    ///         ctx.world.insert_resource(Score(0));
    ///     }
    /// }
    /// let world = Game::new("Plugins only", 1, 1).plugin(Scoring).configured().run_headless(3);
    /// assert_eq!(world.resource::<Score>().0, 3);
    /// ```
    pub fn configured(self) -> ConfiguredGame {
        ConfiguredGame { game: self }
    }

    /// Composes an optional module. Its build runs now; setup runs before the
    /// legacy run closure or in registration order with [`Game::setup`].
    pub fn plugin(self, plugin: impl Plugin + 'static) -> Self {
        let mut game = plugin.build(self);
        game.setups.push(Box::new(move |ctx| plugin.setup(ctx)));
        game
    }

    /// Removes all registrations with this ID, including built-ins. An absent
    /// or already removed ID is a no-op. Runtime event readers remain distinct
    /// even when multiple registrations share a built-in ID.
    pub fn remove(mut self, id: SystemId) -> Self {
        self.remove_id(id);
        self
    }

    fn remove_id(&mut self, id: SystemId) {
        self.sim_systems.retain(|s| s.key != id && s.builtin != Some(id));
        self.systems.retain(|s| s.key != id && s.builtin != Some(id));
        self.late_systems.retain(|s| s.key != id && s.builtin != Some(id));
        self.draw_systems.retain(|s| s.key != id && s.builtin != Some(id));
    }

    /// Plays over the network in deterministic lockstep: `net` from
    /// [`Net::host`](crate::Net::host), [`Net::join`](crate::Net::join) or
    /// [`Net::loopback`](crate::Net::loopback). Ticks then run only once
    /// every player's input for them is in, and the setup and simulation
    /// systems must be deterministic (same seed, no wall clock, no frame
    /// systems changing the world). Without it, `ctx.net` is offline.
    ///
    /// ```
    /// use ponko::*;
    /// use std::thread;
    ///
    /// // Two players in one process; over the network it's Net::host / Net::join.
    /// let games: Vec<_> = Net::loopback(2)
    ///     .into_iter()
    ///     .map(|net| thread::spawn(move || {
    ///         let world = Game::new("t", 1, 1).lockstep(net).run_headless(30, |_| {});
    ///         world.len()
    ///     }))
    ///     .collect();
    /// for g in games {
    ///     assert_eq!(g.join().unwrap(), 0);
    /// }
    /// ```
    pub fn lockstep(mut self, net: crate::Net) -> Self {
        self.net = Some(net);
        self
    }

    /// Adds a per-frame system after automatic animation and before drawing.
    /// Runs in registration order, with frame dt and ordinary system timings.
    /// Like frame systems, it does not run in `run_headless`.
    ///
    /// ```no_run
    /// use ponko::*;
    /// Game::new("Pose", 800, 600).late_system(|ctx| { let _ = ctx.dt(); }).run(|_| {});
    /// ```
    pub fn late_system<F: FnMut(&mut Ctx) + 'static>(self, system: F) -> Self {
        self.add(Phase::Late, system)
    }

    /// Adds a simulation system: it runs at the fixed rate
    /// ([`fixed_hz`](Self::fixed_hz), default 60 per second), zero or more
    /// times per frame, and `ctx.dt()` inside it is the fixed step.
    /// Put game logic here so it behaves the same at any frame rate.
    ///
    /// Systems run in `Phase::AfterPhysics`, in registration order.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// fn gravity(ctx: &mut Ctx) { /* ... */ }
    /// fn movement(ctx: &mut Ctx) { /* ... */ }
    ///
    /// Game::new("Sim", 800, 600).sim_system(gravity).sim_system(movement).run(|_| {});
    /// ```
    pub fn sim_system<F: FnMut(&mut Ctx) + 'static>(self, system: F) -> Self {
        self.add(Phase::AfterPhysics, system)
    }

    /// Adds a simulation system that runs only every `ticks`-th tick, for
    /// work that doesn't need the full rate: AI that thinks a few times a
    /// second, slow growth, pathfinding, statistics. Inside it, `ctx.dt()`
    /// is `ticks` fixed steps, so rates stay right.
    ///
    /// Systems added with the same interval are spread over different ticks,
    /// so their cost doesn't land on the same frame.
    ///
    /// ```
    /// use ponko::*;
    ///
    /// struct Hunger(f32);
    ///
    /// fn get_hungry(ctx: &mut Ctx) {
    ///     let dt = ctx.dt(); // 10 ticks' worth of time
    ///     ctx.world.query::<&mut Hunger>().for_each(|h| h.0 += 1.0 * dt);
    /// }
    ///
    /// // 60 ticks at 60 Hz: runs 6 times, 1/6 s each.
    /// let mut world = Game::new("t", 1, 1)
    ///     .sim_system_every(10, get_hungry)
    ///     .run_headless(60, |ctx| { ctx.world.spawn((Hunger(0.0),)); });
    /// let h = world.query::<&Hunger>().iter().next().unwrap().0;
    /// assert!((h - 1.0).abs() < 1e-4);
    /// ```
    ///
    /// # Panics
    /// If `ticks` is 0.
    #[track_caller]
    pub fn sim_system_every<F: FnMut(&mut Ctx) + 'static>(self, ticks: u32, system: F) -> Self {
        assert!(
            ticks > 0,
            "Game::sim_system_every: the interval must be at least 1 tick"
        );
        self.push_sim_system(System::new(system), ticks)
    }

    fn push_sim_system(mut self, mut s: System, ticks: u32) -> Self {
        let same = self.sim_systems.iter().filter(|s| s.every == ticks).count() as u32;
        s.every = ticks;
        s.phase = same % ticks;
        let index = self.sim_systems.partition_point(|s| s.stage <= Phase::AfterPhysics);
        self.sim_systems.insert(index, s);
        self
    }

    /// For language bindings (the C API): `sim_system_every` with a name for
    /// timings and tools, since all foreign systems share one Rust type.
    #[doc(hidden)]
    #[track_caller]
    pub fn named_sim_system(self, name: &'static str, ticks: u32, system: impl FnMut(&mut Ctx) + 'static) -> Self {
        assert!(
            ticks > 0,
            "Game::named_sim_system: the interval must be at least 1 tick"
        );
        let mut s = System::new(system);
        s.name = name;
        self.push_sim_system(s, ticks)
    }

    /// For language bindings (the C API): `late_system` with a name.
    #[doc(hidden)]
    pub fn named_late_system(mut self, name: &'static str, system: impl FnMut(&mut Ctx) + 'static) -> Self {
        let mut s = System::new(system);
        s.name = name;
        self.insert_system(Phase::Late, s);
        self
    }

    /// For language bindings (the C API): `system` with a name.
    #[doc(hidden)]
    pub fn named_system(mut self, name: &'static str, system: impl FnMut(&mut Ctx) + 'static) -> Self {
        let mut s = System::new(system);
        s.name = name;
        self.insert_system(Phase::Frame, s);
        self
    }

    /// Removes every system named `name` (Rust systems are named by their
    /// path, e.g. `ponko::ecs::draw_sprites`). Returns how many were removed.
    #[doc(hidden)]
    #[deprecated(note = "use `Game::remove` with a `SystemId`: a `Systems` constant for a built-in, \
                         or the id `Game::add_system` returned")]
    pub fn remove_named_system(&mut self, name: &str) -> usize {
        self.remove_named(name)
    }

    fn remove_named(&mut self, name: &str) -> usize {
        let ids: Vec<_> = self
            .sim_systems
            .iter()
            .chain(&self.systems)
            .chain(&self.late_systems)
            .chain(&self.draw_systems)
            .filter(|s| s.name == name)
            .map(|s| s.key)
            .collect();
        let count = ids.len();
        for id in ids {
            self.remove_id(id);
        }
        count
    }

    /// How many registrations `id` names: 1 for an id from
    /// [`add_system`](Self::add_system) that is still registered, the number
    /// of instances for a [`Systems`] constant, 0 if there is none.
    ///
    /// ```
    /// use ponko::{Game, Systems};
    /// let game = Game::new("t", 1, 1);
    /// assert_eq!(game.system_count(Systems::DRAW_SPRITES), 1);
    /// assert_eq!(game.remove(Systems::DRAW_SPRITES).system_count(Systems::DRAW_SPRITES), 0);
    /// ```
    #[must_use]
    pub fn system_count(&self, id: SystemId) -> usize {
        (self.sim_systems.iter())
            .chain(&self.systems)
            .chain(&self.late_systems)
            .chain(&self.draw_systems)
            .filter(|s| s.key == id || s.builtin == Some(id))
            .count()
    }

    /// Turns on 2D physics (feature `physics`): every entity with a
    /// [`RigidBody`](crate::RigidBody), a [`Collider`](crate::Collider) and a
    /// [`Transform2D`](crate::Transform2D) is simulated each tick, pulled by
    /// `gravity` (pixels/s²; y is down). Collisions arrive as
    /// [`CollisionStarted`](crate::CollisionStarted) events; `ctx.physics`
    /// casts rays and finds overlaps.
    ///
    /// The step is the built-in [`step_physics`](crate::systems::step_physics) sim
    /// system in [`Phase::Physics`], independent of builder call order.
    /// Use [`Phase::BeforePhysics`] for work that must run before the step.
    ///
    /// ```
    /// use ponko::*;
    /// let mut world = Game::new("Drop", 800, 600)
    ///     .physics(vec2(0.0, 980.0))
    ///     .run_headless(60, |ctx| {
    ///         ctx.world.spawn((Transform2D::at(vec2(0.0, 0.0)), RigidBody::dynamic(), Collider::circle(8.0)));
    ///     });
    /// // One second of free fall: about 490 pixels down.
    /// let y = world.query::<&Transform2D>().iter().next().unwrap().position.y;
    /// assert!((y - 490.0).abs() < 10.0, "{y}");
    /// ```
    #[cfg(feature = "physics")]
    pub fn physics(mut self, gravity: Vec2) -> Self {
        self.gravity = Some(gravity);
        if !self.sim_systems.iter().any(|s| s.builtin == Some(Systems::PHYSICS)) {
            self = self.add(Phase::Physics, crate::systems::step_physics);
        }
        self
    }

    /// Turns on 3D physics (feature `physics3d`): every entity with a
    /// [`RigidBody3D`](crate::RigidBody3D), a [`Collider3D`](crate::Collider3D)
    /// and a [`Transform3D`](crate::Transform3D) is simulated each tick, and
    /// every [`Character3D`](crate::Character3D) walks, pulled by `gravity`
    /// (m/s²; +Y is up, so `vec3(0.0, -9.81, 0.0)`). Collisions arrive as
    /// [`CollisionStarted`](crate::CollisionStarted) events; `ctx.physics3d`
    /// casts rays and shapes and finds overlaps.
    ///
    /// The simulation runs at [`PHYSICS3D_HZ`](crate::PHYSICS3D_HZ) (50)
    /// ticks per second unless [`fixed_hz`](Self::fixed_hz) chose a rate;
    /// in a window, moving bodies are drawn blended between ticks
    /// ([`interpolate_physics3d`](crate::systems::interpolate_physics3d), a
    /// `Frame` system). The step is the built-in
    /// [`step_physics3d`](crate::systems::step_physics3d) sim system, removable and
    /// registered in [`Phase::Physics`], independent of builder call order.
    /// Use [`Phase::BeforePhysics`] for work that must run before the step.
    ///
    /// ```
    /// use ponko::*;
    /// let mut world = Game::new("Drop", 800, 600)
    ///     .physics3d(vec3(0.0, -9.81, 0.0))
    ///     .run_headless(50, |ctx| {
    ///         ctx.world.spawn((Transform3D::at(vec3(0.0, 10.0, 0.0)), RigidBody3D::dynamic(), Collider3D::sphere(0.5)));
    ///     });
    /// // One second of free fall: about 4.9 m down.
    /// let y = world.query::<&Transform3D>().iter().next().unwrap().position.y;
    /// assert!((y - 5.1).abs() < 0.2, "{y}");
    /// ```
    #[cfg(feature = "physics3d")]
    pub fn physics3d(mut self, gravity: crate::Vec3) -> Self {
        self.gravity3d = Some(gravity);
        if !self.fixed_hz_set {
            self.fixed_hz = crate::PHYSICS3D_HZ;
        }
        if !self.sim_systems.iter().any(|s| s.builtin == Some(Systems::PHYSICS3D)) {
            self = self.add(Phase::Physics, crate::systems::step_physics3d);
        }
        if !self
            .systems
            .iter()
            .any(|s| s.builtin == Some(Systems::INTERPOLATE_PHYSICS3D))
        {
            self = self.add(Phase::Frame, crate::systems::interpolate_physics3d);
        }
        self
    }

    /// Turns on the built-in [`SpatialGrid`](crate::SpatialGrid) with square
    /// cells of `cell_size` world units: at the start of every simulation
    /// tick, all entities with a [`Transform2D`](crate::Transform2D) are
    /// indexed into `ctx.spatial` for radius, rectangle and nearest queries.
    ///
    /// Choose a cell size near your usual query radius. The rebuild is the
    /// built-in [`rebuild_spatial_grid`] system, removable like any other.
    ///
    /// # Panics
    /// If `cell_size` isn't a positive, finite number.
    #[track_caller]
    pub fn spatial_grid(mut self, cell_size: f32) -> Self {
        self.spatial_cell = Some(crate::SpatialGrid::new(cell_size).cell_size());
        if !self
            .sim_systems
            .iter()
            .any(|s| s.builtin == Some(Systems::REBUILD_SPATIAL_GRID))
        {
            self = self.add(Phase::Input, rebuild_spatial_grid);
        }
        self
    }

    /// Adds a frame system: it runs once per rendered frame, after the
    /// simulation systems. Use it for input, camera and drawing.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// fn quit_on_escape(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::Escape) { ctx.quit(); }
    /// }
    ///
    /// Game::new("Sim", 800, 600).system(quit_on_escape).run(|_| {});
    /// ```
    pub fn system<F: FnMut(&mut Ctx) + 'static>(self, system: F) -> Self {
        self.add(Phase::Frame, system)
    }

    /// Removes a system by the function that was added.
    ///
    /// Deprecated: remove by id instead. [`Game::remove`] takes a [`Systems`]
    /// constant for a built-in (`Systems::DRAW_SPRITES`), or the
    /// [`SystemId`] that [`Game::add_system`] returned for one of yours.
    /// Unlike this method, it also tells apart two closures of the same
    /// type, and removing an absent system is not an error.
    ///
    /// # Panics
    /// If no such system was added.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// // A headless-style simulation that draws nothing by itself.
    /// Game::new("Sim", 800, 600).remove(Systems::DRAW_SPRITES).run(|_| {});
    /// ```
    #[deprecated(note = "use `Game::remove` with a `SystemId`: a `Systems` constant for a built-in \
                         (`Systems::DRAW_SPRITES`), or the id `Game::add_system` returned")]
    #[track_caller]
    pub fn remove_system<F: FnMut(&mut Ctx) + 'static>(mut self, system: F) -> Self {
        drop(system);
        let name = std::any::type_name::<F>();
        // A built-in is found by its id, whichever spelling named it.
        let removed = match builtin_id(name) {
            Some((id, _)) => {
                let n = self.system_count(id);
                self.remove_id(id);
                n
            }
            None => self.remove_named(name),
        };
        assert_ne!(removed, 0, "Game::remove_system: no system `{name}` was added");
        self
    }

    /// Feeds scheduled input into the game by simulation tick, for
    /// reproducible tests (off unless called). Before the simulation systems
    /// of each tick run, the scenario's steps for that tick go through
    /// [`Input::feed`](crate::Input::feed), so [named actions](crate::ActionMap) and raw input
    /// queries see them exactly like live input. Calling it again replaces
    /// the previous scenario.
    ///
    /// Check steps run after the tick's systems, and the outcome is the
    /// [`ScenarioReport`](crate::ScenarioReport) resource; see
    /// [`run_headless_scenario`](Self::run_headless_scenario) for a runner.
    ///
    /// Edges last until the end of the frame. Headless runs have one tick per
    /// frame, so a press is visible to exactly one tick. A windowed frame that
    /// runs several ticks shows it to the rest of that frame's ticks, as with
    /// live input. Live input is still read alongside the scenario.
    ///
    /// ```
    /// use ponko::*;
    ///
    /// struct Jumps(u32);
    ///
    /// fn jump(ctx: &mut Ctx) {
    ///     if ctx.input.action_pressed("jump") {
    ///         ctx.world.query::<&mut Jumps>().for_each(|j| j.0 += 1);
    ///     }
    /// }
    ///
    /// let scenario = InputScenario::new()
    ///     .at(3, InputEvent::Key(Key::Space, true))
    ///     .at(6, InputEvent::Key(Key::Space, false));
    /// let mut world = Game::new("test", 1, 1)
    ///     .sim_system(jump)
    ///     .input_scenario(scenario)
    ///     .run_headless(10, |ctx| {
    ///         let mut actions = ActionMap::default();
    ///         actions.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
    ///         ctx.input.set_actions(actions);
    ///         ctx.world.spawn((Jumps(0),));
    ///     });
    /// assert_eq!(world.query::<&Jumps>().iter().next().unwrap().0, 1);
    /// ```
    pub fn input_scenario(mut self, scenario: crate::InputScenario) -> Self {
        self.input_scenario = Some(scenario);
        self
    }

    /// Runs `scenario` headlessly for its [`ticks`](crate::InputScenario::ticks)
    /// and returns the world with the scenario's report: the bounded runner
    /// for tests and CI. A failed check ends the run at that tick and the
    /// report names its step and tick; [`ScenarioReport::assert_passed`](crate::ScenarioReport::assert_passed)
    /// turns that into a panic.
    ///
    /// ```
    /// use ponko::*;
    ///
    /// let scenario = InputScenario::new()
    ///     .at(2, InputEvent::Key(Key::Space, true))
    ///     .expect(2, ScenarioCheck::action("jump").pressed(true))
    ///     .expect(3, ScenarioCheck::action("jump").pressed(true)); // deliberately wrong
    /// let (_world, report) = Game::new("test", 1, 1).run_headless_scenario(scenario, |ctx| {
    ///     let mut actions = ActionMap::default();
    ///     actions.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
    ///     ctx.input.set_actions(actions);
    /// });
    /// let failure = report.failure.unwrap();
    /// assert_eq!((failure.step, failure.tick), (2, 3));
    /// ```
    pub fn run_headless_scenario<S, U>(self, scenario: crate::InputScenario, setup: S) -> (World, crate::ScenarioReport)
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        let ticks = scenario.ticks();
        let pending = crate::ScenarioReport::new(&scenario, 0);
        let world = self.input_scenario(scenario).run_headless(ticks, setup);
        let report = world.get_resource::<crate::ScenarioReport>().cloned();
        (world, report.unwrap_or(pending))
    }

    /// Lets tools start input scenarios through the [control endpoint](Self::control_endpoint)
    /// (the `input.scenario.*` methods and MCP's `run_input_scenario`). Off
    /// by default, so a game with the endpoint open for inspection still
    /// refuses injected input; setting `PONKO_INPUT_INJECTION=1` has the same
    /// effect without code changes (the MCP `launch` tool sets it). Scenarios
    /// installed with [`input_scenario`](Self::input_scenario) don't need it.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Dev build", 800, 600).control_endpoint(true).input_injection(true).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn input_injection(mut self, on: bool) -> Self {
        self.input_injection = on;
        self
    }

    /// Opens the control endpoint, so tools such as the `ponko-mcp` agent
    /// bridge can inspect and drive this game while it runs: list and edit
    /// entities (through [reflection](crate::WorldReflect)), pause, step,
    /// change speed, read system timings.
    ///
    /// It listens on `127.0.0.1` only, and every request must carry a random
    /// token written to the instance registry (`~/.crabby/instances`),
    /// which only your user can read. Off by default; setting the
    /// environment variable `PONKO_CONTROL=1` turns it on without code
    /// changes, and `PONKO_START_PAUSED=1` starts the simulation paused.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Colony", 1280, 720).control_endpoint(true).run(|_| {});
    /// ```
    pub fn control_endpoint(mut self, on: bool) -> Self {
        self.control = on;
        self
    }

    /// Reload textures, fonts, sounds and data files ([`Assets::load_resource`](crate::Assets::load_resource))
    /// from `assets/` when their files change, under the same handles: save an
    /// image or a JSON file in your editor and the running game shows it
    /// within a quarter of a second. On by default in debug builds, off in
    /// release builds; `PONKO_HOT_RELOAD=1` or `0` overrides both.
    ///
    /// A file is checked a few times a second (one `stat` per loaded
    /// texture). An image that changed size isn't reloaded (sprites store
    /// their texture's size): restart the game for that.
    /// [`Assets::reloaded_textures`](crate::Assets::reloaded_textures) lists what was reloaded this frame.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Art test", 1280, 720).hot_reload(true).run(|ctx| {
    ///     let crab = ctx.load_texture("crab.png"); // edit crab.png while it runs
    ///     ctx.world.spawn((Transform2D::at(vec2(640.0, 360.0)), Sprite::new(crab)));
    /// });
    /// ```
    pub fn hot_reload(mut self, on: bool) -> Self {
        self.hot_reload = on;
        self
    }

    /// Turns the runtime inspector on or off: an overlay that the backquote
    /// key (`` ` ``, left of 1) opens and closes,
    /// showing every entity and its components (reflected ones editable),
    /// reflected resources, system times, and pause / step / speed controls.
    /// Its "Pick" button selects the unit you click next.
    ///
    /// On by default in debug builds, off in release builds. The
    /// environment variable `PONKO_INSPECTOR=1` (or `0`) overrides this, and
    /// `PONKO_INSPECTOR=open` also starts the game with it open.
    /// Needs the `inspector` cargo feature (on by default); without it, this
    /// does nothing. While closed it costs nothing.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// // Ship a release build with the inspector, for playtesters.
    /// Game::new("Playtest", 1280, 720).inspector(true).run(|ctx| {
    ///     let _ = ctx;
    /// });
    /// ```
    pub fn inspector(mut self, on: bool) -> Self {
        self.inspector = on;
        self
    }

    /// Constructs the context both loops run: the services from the game's
    /// configuration (seed, hot reload, spatial grid, physics worlds, the
    /// lockstep session), before setup and before the control endpoint
    /// starts. `headless` is a run without a window or GPU.
    ///
    /// Window and headless runs differ only in what `ctx.gfx` and
    /// `ctx.audio` do with a missing device; every other service is built
    /// the same way in both.
    fn build_ctx(&mut self, headless: bool) -> Ctx {
        let fixed_dt = Clock::new(self.fixed_hz).fixed_dt();
        let mut ctx = Ctx::new(fixed_dt, Vec2::new(self.width as f32, self.height as f32));
        ctx.seed = self.seed;
        if headless {
            ctx.set_headless(true);
        }
        ctx.assets.watcher.set_on(crate::hot_reload::enabled(self.hot_reload));
        if let Some(cell) = self.spatial_cell {
            ctx.spatial = crate::SpatialGrid::new(cell);
        }
        #[cfg(feature = "physics")]
        if let Some(gravity) = self.gravity {
            crate::physics::start(&mut ctx, gravity);
        }
        #[cfg(feature = "physics3d")]
        if let Some(gravity) = self.gravity3d {
            crate::physics3d::start(&mut ctx, gravity);
        }
        if let Some(net) = self.net.take() {
            ctx.net = net;
        }
        ctx.net.begin(ctx.seed);
        ctx
    }

    /// Starts the control endpoint if it's enabled, and applies `PONKO_START_PAUSED`.
    fn start_control(&self, ctx: &mut Ctx, headless: bool) -> Option<Control> {
        if control::env_flag(control::ENV_START_PAUSED) {
            ctx.time.pause();
        }
        ctx.input.scenarios.injection = self.input_injection || control::env_flag(control::ENV_INPUT_INJECTION);
        if !self.control {
            return None;
        }
        if cfg!(target_arch = "wasm32") {
            eprintln!("ponko: the control endpoint isn't available in web builds");
            return None;
        }
        match Control::start(&self.title, headless) {
            Ok(c) => {
                ctx.control_addr = Some(c.addr());
                Some(c)
            }
            Err(e) => {
                eprintln!("ponko: could not open the control endpoint: {e}");
                None
            }
        }
    }

    /// Sets the seed for [`Ctx::random`]. Same seed, same simulation: runs
    /// are reproducible by default ([`DEFAULT_SEED`](crate::DEFAULT_SEED)).
    /// For a different world each run, seed from the clock.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    /// Game::new("Roguelike", 800, 600).seed(seed).run(|_| {});
    /// ```
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets how many fixed updates run per second (default 60).
    /// See [`Time::fixed_steps`](crate::Time::fixed_steps).
    ///
    /// # Panics
    /// If `hz` is 0.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Physics", 800, 600).fixed_hz(120).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn fixed_hz(mut self, hz: u32) -> Self {
        assert!(hz > 0, "Game::fixed_hz: the rate must be at least 1 Hz");
        self.fixed_hz = hz;
        self.fixed_hz_set = true;
        self
    }

    /// Turns vsync on (default) or off. With vsync the game runs at the
    /// display's refresh rate with no tearing; turn it off to measure how fast
    /// the game *could* run.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Benchmark", 800, 600).vsync(false).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn vsync(mut self, on: bool) -> Self {
        self.vsync = on;
        self
    }

    /// How many samples per pixel the 3D frame takes to smooth its edges:
    /// 4 (default) or 2. 2 costs about a third less GPU time on a large
    /// screen, with a little more stair-stepping on thin edges. GPUs that
    /// can't take 2 everywhere (WebGL2, some mobile ones) keep 4.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Open world", 1280, 720).msaa(2).run(|_| |_: &mut Ctx| {});
    /// ```
    ///
    /// # Panics
    ///
    /// If `samples` is not 2 or 4.
    pub fn msaa(mut self, samples: u32) -> Self {
        assert!(
            matches!(samples, 2 | 4),
            "the 3D frame takes 2 or 4 samples per pixel, not {samples}"
        );
        self.msaa = samples;
        self
    }

    /// Fills the screen with the game instead of opening a window (off by
    /// default). On macOS, windowed games are shown at most once per display
    /// refresh even with vsync off; fullscreen ones aren't. Web builds
    /// ignore it.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Benchmark", 1280, 720).fullscreen(true).vsync(false).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn fullscreen(mut self, on: bool) -> Self {
        self.fullscreen = on;
        self
    }

    /// One frame of game logic: simulation systems `fixed_steps` times, then
    /// frame, late and draw systems. The retained UI updates first
    /// (`Ctx::frame_begin_services`); every other engine step of a frame is
    /// a system: animators and particles end `Animate`, positional audio ends
    /// `Draw`. Commands are applied after every system.
    fn run_systems(&mut self, ctx: &mut Ctx, fixed_steps: u32) {
        ctx.frame_begin_services();
        self.begin_timings(ctx);
        ctx.set_in_sim(true);
        for _ in 0..fixed_steps {
            // Lockstep: a tick waits for everyone's input; the frame goes on.
            if !ctx.net.ready(ctx.time.sim_tick()) {
                break;
            }
            self.run_sim_tick(ctx);
        }
        ctx.set_in_sim(false);
        let n_sim = self.sim_systems.len();
        let n_frame = self.systems.len();
        for (i, s) in self.systems.iter_mut().enumerate() {
            run_one(s, ctx, n_sim + i);
        }
        for (i, s) in self.late_systems.iter_mut().enumerate() {
            run_one(s, ctx, n_sim + n_frame + i);
        }
        for (i, s) in self.draw_systems.iter_mut().enumerate() {
            run_one(s, ctx, n_sim + n_frame + self.late_systems.len() + i);
        }
        end_timings(ctx);
        let frame = self.systems.iter().chain(&self.late_systems).chain(&self.draw_systems);
        forget_seen_removals(ctx, self.sim_systems.iter().chain(frame));
    }

    /// One simulation tick: every sim system once, then the tick counter.
    fn run_sim_tick(&mut self, ctx: &mut Ctx) {
        let tick = ctx.time.sim_tick();
        if let Some(scenario) = self.input_scenario.take() {
            ctx.input.install_scenario(
                &mut ctx.world,
                &mut ctx.time,
                scenario,
                0,
                crate::scenario::RunOrigin::default(),
            );
        }
        ctx.scenario_begin_tick();
        for (i, s) in self.sim_systems.iter_mut().enumerate() {
            if tick % u64::from(s.every) != u64::from(s.phase) {
                continue;
            }
            ctx.set_sim_steps(s.every);
            run_one(s, ctx, i);
        }
        ctx.set_sim_steps(1);
        ctx.scenario_end_tick();
        ctx.net.end_tick(&ctx.world, tick);
        ctx.advance_tick();
    }

    /// Each system's phase and interval, in timing order, for diagnostics.
    /// Late and draw systems report their list's phase (bindings add them
    /// without setting `stage`).
    fn schedule_slots(&self) -> Vec<crate::diagnostics::ScheduleSlot> {
        let slot = |s: &System, phase| crate::diagnostics::ScheduleSlot {
            phase,
            every: s.every,
            offset: s.phase,
        };
        (self.sim_systems.iter().map(|s| slot(s, s.stage)))
            .chain(self.systems.iter().map(|s| slot(s, s.stage)))
            .chain(self.late_systems.iter().map(|s| slot(s, Phase::Late)))
            .chain(self.draw_systems.iter().map(|s| slot(s, Phase::Draw)))
            .collect()
    }

    /// Makes sure there's one timing slot per system, and zeroes this frame's times.
    fn begin_timings(&self, ctx: &mut Ctx) {
        if ctx.perf.timings.len()
            != self.sim_systems.len() + self.systems.len() + self.late_systems.len() + self.draw_systems.len()
        {
            ctx.perf.timings = (self.sim_systems.iter().map(|s| (s, true)))
                .chain(
                    self.systems
                        .iter()
                        .chain(&self.late_systems)
                        .chain(&self.draw_systems)
                        .map(|s| (s, false)),
                )
                .map(|(s, sim)| SystemTiming {
                    name: s.name,
                    sim,
                    last_ms: 0.0,
                    avg_ms: 0.0,
                })
                .collect();
            ctx.perf.schedule = self.schedule_slots();
        }
        for t in &mut ctx.perf.timings {
            t.last_ms = 0.0;
        }
        ctx.sync_profiling();
        ctx.perf.state.begin_frame(ctx.perf.timings.len());
        ctx.events.keep_ticks = self.sim_systems.iter().map(|s| s.every).max().unwrap_or(1);
    }

    /// Runs the simulation without a window or GPU, as fast as possible, and
    /// returns the world when done. For dedicated servers, batch runs,
    /// benchmarks, and testing game logic in plain `cargo test`.
    ///
    /// `setup` runs once, then `ticks` simulation ticks run (fewer if a
    /// system calls `ctx.quit()`). Only **simulation systems** run: frame
    /// systems, the closure returned by `setup`, and drawing need frames, and
    /// there are none. Sim speed is ignored, and so is pause unless the
    /// [control endpoint](Self::control_endpoint) is on: then a paused game
    /// waits for tools to step or resume it. Textures still load (for their
    /// sizes), but nothing is uploaded to a GPU.
    ///
    /// ```
    /// use ponko::*;
    ///
    /// struct Pos(f32);
    ///
    /// fn fall(ctx: &mut Ctx) {
    ///     let dt = ctx.dt();
    ///     ctx.world.query::<&mut Pos>().for_each(|p| p.0 -= 10.0 * dt);
    /// }
    ///
    /// let mut world = Game::new("test", 800, 600)
    ///     .sim_system(fall)
    ///     .run_headless(60, |ctx| {
    ///         ctx.world.spawn((Pos(100.0),));
    ///     });
    ///
    /// // One simulated second at 10 units/s.
    /// let y = world.query::<&Pos>().iter().next().unwrap().0;
    /// assert!((y - 90.0).abs() < 1e-3);
    /// ```
    ///
    /// For a game with systems, put the startup code on the builder with
    /// [`setup`](Self::setup) and call [`ConfiguredGame::run_headless`],
    /// which takes no closure. This form remains for short tests, and for a
    /// setup that borrows a local variable, which a stored callback can't.
    pub fn run_headless<S, U>(self, ticks: u64, setup: S) -> World
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        self.run_headless_inner(ticks, None, setup).0
    }

    /// Like [`run_headless`](Self::run_headless), but times everything and
    /// returns a [`PerfReport`](crate::PerfReport) next to the world: where
    /// the time goes (systems, [`profile_scope!`](crate::profile_scope)
    /// scopes), allocations, the slowest ticks, and warnings with advice.
    /// The first `warmup` ticks run but aren't measured (caches fill, buffers
    /// grow); the report covers the next `ticks` ticks (at most the last
    /// 50,000). Call it from a test or a CI job; see [`PerfReport::assert_within`](crate::PerfReport::assert_within).
    ///
    /// Counting allocations needs [`CountingAllocator`](crate::CountingAllocator)
    /// as the global allocator.
    ///
    /// ```
    /// use ponko::*;
    ///
    /// fn movement(ctx: &mut Ctx) {
    ///     profile_scope!("movement/integrate");
    ///     ctx.world.query::<&mut Transform2D>().for_each(|t| t.position.x += 1.0);
    /// }
    ///
    /// let (_world, report) = Game::new("bench", 1, 1)
    ///     .sim_system(movement)
    ///     .profile_headless(10, 100, |ctx| {
    ///         for _ in 0..1000 {
    ///             ctx.world.spawn((Transform2D::default(),));
    ///         }
    ///     });
    /// assert_eq!((report.frames, report.entities), (100, 1000));
    /// // Timings vary between machines: keep limits generous.
    /// report.assert_within(&Budget::new().frame_p99_ms(100.0));
    /// ```
    pub fn profile_headless<S, U>(self, warmup: u64, ticks: u64, setup: S) -> (World, crate::PerfReport)
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        let (world, report) = self.run_headless_inner(ticks, Some(warmup), setup);
        (world, report.unwrap_or_default())
    }

    fn run_headless_inner<S, U>(
        mut self,
        ticks: u64,
        warmup: Option<u64>,
        setup: S,
    ) -> (World, Option<crate::PerfReport>)
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        connect_hotpatch();
        let fixed_dt = Clock::new(self.fixed_hz).fixed_dt();
        let mut ctx = self.build_ctx(true);
        let control = self.start_control(&mut ctx, true);
        ctx.set_in_setup(true);
        run_setups(std::mem::take(&mut self.setups), &mut ctx);
        drop(setup(&mut ctx).into_frame_fn()); // no frames, so no frame closure
        ctx.set_in_setup(false);
        ctx.apply_commands();
        ctx.gfx.discard_draws();

        let timing = FrameTiming {
            dt: fixed_dt as f32,
            fixed_steps: 1,
            alpha: 0.0,
        };
        if warmup.is_some() {
            ctx.perf.state.collector = crate::perf_report::Collector::new(crate::perf_report::HEADLESS_HISTORY);
            ctx.perf.set_profiling(true);
        }
        let total = ticks + warmup.unwrap_or(0);
        let mut ran = 0;
        // Lists the systems before the first tick, for tools asking a paused game.
        self.begin_timings(&mut ctx);
        while ran < total && !ctx.quit_requested() {
            if let Some(control) = &control {
                control.poll(&mut ctx);
                if ctx.quit_requested() {
                    break;
                }
                // A paused game runs only the ticks tools ask for.
                if ctx.time.is_paused() && !ctx.take_one_step() {
                    control.wait(&mut ctx, Duration::from_millis(50));
                    continue;
                }
            }
            if !ctx.net.ready(ctx.time.sim_tick()) {
                if !ctx.net.wait() {
                    break;
                }
                continue;
            }
            ran += 1;
            let tick_start = Instant::now();
            ctx.begin_frame(timing);
            ctx.frame_prepare_services();
            self.begin_timings(&mut ctx);
            ctx.set_in_sim(true);
            self.run_sim_tick(&mut ctx);
            ctx.set_in_sim(false);
            end_timings(&mut ctx);
            // Only simulation systems run headless, so only they hold removals back.
            forget_seen_removals(&mut ctx, self.sim_systems.iter());
            ctx.gfx.discard_draws();
            ctx.input.end_frame();
            if ctx.perf.state.on {
                ctx.end_perf_frame(tick_start.elapsed().as_secs_f32() * 1000.0);
                if warmup == Some(ran) {
                    ctx.perf.state.collector.clear();
                }
            }
        }
        let report = warmup.map(|_| ctx.perf.report());
        (std::mem::take(&mut ctx.world), report)
    }

    /// Opens the window and runs the game until it is closed.
    ///
    /// `setup` runs once, after the window exists. It can spawn entities into
    /// `ctx.world`, and it can return a closure that runs every frame (after
    /// the systems). Variables created in `setup` and moved into that closure
    /// are your game state; no structs or traits required.
    ///
    /// Each frame runs: simulation systems (`ctx.time.fixed_steps()` times), then
    /// frame systems, then the closure from `setup`, then the
    /// [`Phase::Animate`], [`Phase::Late`] and [`Phase::Draw`] systems (the
    /// built-in [`draw_sprites`] is one of the last), then rendering.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Counter", 800, 600).run(|_ctx| {
    ///     let mut clicks = 0;
    ///     move |ctx: &mut Ctx| {
    ///         if ctx.input.mouse_pressed(MouseButton::Left) {
    ///             clicks += 1;
    ///             println!("clicks: {clicks}");
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// This form is for small programs. For a game with systems, put the
    /// startup code on the builder with [`setup`](Self::setup) and call
    /// [`ConfiguredGame::run`], which takes no closure: setup callbacks can
    /// be chosen at run time and are ordered with [`Plugin`] setups.
    ///
    /// # Panics
    /// If the operating system refuses to create the event loop or window,
    /// or no usable GPU is found, and always when the `window` feature is
    /// off.
    pub fn run<S, U>(self, setup: S)
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        #[cfg(feature = "window")]
        window::run(self, setup);
        #[cfg(not(feature = "window"))]
        {
            drop((self, setup));
            panic!(
                "ponko: Game::run needs a window, and this build has none \
                 (the `window` feature is off); use Game::run_headless"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Log(Vec<String>);
    struct Marker;

    fn ctx() -> Ctx {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(800.0, 600.0));
        ctx.world.insert_resource(Log(Vec::new()));
        ctx
    }

    fn log(ctx: &mut Ctx, s: &str) {
        let line = format!("{s} dt={:.4}", ctx.dt());
        ctx.world.resource_mut::<Log>().0.push(line);
    }

    fn sim_a(ctx: &mut Ctx) {
        log(ctx, "sim_a");
    }

    /// A game without the systems `Game::new` adds by itself.
    fn bare(game: Game) -> Game {
        game.remove(Systems::ANIMATE_SPRITES)
            .remove(Systems::DRAW_TILEMAPS)
            .remove(Systems::DRAW_SPRITES)
            .remove(Systems::UPDATE_ANIMATORS)
            .remove(Systems::UPDATE_PARTICLES)
            .remove(Systems::UPDATE_AUDIO3D)
    }
    fn frame_b(ctx: &mut Ctx) {
        log(ctx, "frame_b");
    }

    #[cfg(feature = "3d")]
    #[test]
    fn late_system_sees_automatic_animation_pose() {
        use crate::{AnimationClip, AnimationGraph, AnimationState, Animator, Joint, Motion, Skeleton, Transform3D};
        let skeleton = Skeleton::new(vec![Joint {
            name: "root".into(),
            parent: None,
            rest: Transform3D::default(),
        }]);
        let clip = AnimationClip::new("rest", &skeleton, vec![]);
        let animator = Animator::new(AnimationGraph::new(
            vec![],
            vec![AnimationState::new(Motion::Clip(clip))],
            vec![],
            0,
        ));
        let mut ctx = ctx();
        let entity = ctx.world.spawn((skeleton, animator));
        // Keeps the built-in animator system; it has nothing to draw.
        let mut game = Game::new("t", 1, 1)
            .system(move |ctx| ctx.world.get_mut::<Skeleton>(entity).unwrap().pose[0].position.x = 7.0)
            .add(Phase::Animate, move |ctx| {
                // Before the built-in animators, although it was added after them.
                assert_eq!(ctx.world.get::<Skeleton>(entity).unwrap().pose[0].position.x, 7.0);
            })
            .late_system(move |ctx| {
                let s = ctx.world.get_mut::<Skeleton>(entity).unwrap();
                assert_eq!(s.pose[0].position.x, 0.0);
                s.pose[0].position.x = 2.0;
            });
        game.run_systems(&mut ctx, 0);
        assert_eq!(ctx.world.get::<Skeleton>(entity).unwrap().pose[0].position.x, 2.0);
    }

    #[test]
    fn late_system_runs_after_frame_before_draw_and_is_timed() {
        fn late(ctx: &mut Ctx) {
            log(ctx, "late");
        }
        fn draw(ctx: &mut Ctx) {
            log(ctx, "draw");
        }
        let mut game = bare(Game::new("t", 1, 1)).system(frame_b).late_system(late);
        game.draw_systems.push(System::new(draw));
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 0);
        assert_eq!(
            ctx.world.resource::<Log>().0,
            ["frame_b dt=0.0000", "late dt=0.0000", "draw dt=0.0000"]
        );
        assert_eq!(ctx.perf.system_timings().len(), 3);
        assert!(ctx.perf.system_timings()[1].name.ends_with("::late"));
        #[allow(deprecated)]
        let game = game.remove_system(late);
        assert!(game.late_systems.is_empty());
    }

    #[test]
    fn ids_remove_from_every_phase_and_builtin_aliases_remove_all_instances() {
        let mut game = Game::new("ids", 1, 1);
        for phase in [
            Phase::Input,
            Phase::BeforePhysics,
            Phase::Physics,
            Phase::AfterPhysics,
            Phase::SimLast,
            Phase::Frame,
            Phase::Animate,
            Phase::Late,
            Phase::Draw,
        ] {
            let id = game.add_system(phase, |_| panic!("removed system ran"));
            game = game.remove(id);
            assert!(
                !game
                    .sim_systems
                    .iter()
                    .chain(&game.systems)
                    .chain(&game.late_systems)
                    .chain(&game.draw_systems)
                    .any(|s| s.key == id)
            );
        }
        game.add_system(Phase::Draw, draw_sprites);
        assert_eq!(game.system_count(Systems::DRAW_SPRITES), 2);
        game = game.remove(Systems::DRAW_SPRITES);
        assert_eq!(game.system_count(Systems::DRAW_SPRITES), 0);
        assert_eq!(game.system_count(Systems::ANIMATE_SPRITES), 1);
        assert_eq!(game.system_count(Systems::DRAW_TILEMAPS), 1);
        let game = game
            .spatial_grid(32.0)
            .remove(Systems::REBUILD_SPATIAL_GRID)
            .spatial_grid(64.0);
        assert_eq!(game.sim_systems.len(), 1);
    }

    #[test]
    fn explicit_phases_sort_without_changing_order_within_a_phase() {
        let mut game = bare(Game::new("t", 1, 1))
            .add(Phase::Draw, |ctx| log(ctx, "draw"))
            .add(Phase::Late, |ctx| log(ctx, "late"))
            .add(Phase::Animate, |ctx| log(ctx, "animate"))
            .add(Phase::Frame, |ctx| log(ctx, "frame1"))
            .system(|ctx| log(ctx, "frame2"))
            .add(Phase::SimLast, |ctx| log(ctx, "sim_last"))
            .add(Phase::AfterPhysics, |ctx| log(ctx, "after"))
            .add(Phase::Physics, |ctx| log(ctx, "physics"))
            .add(Phase::BeforePhysics, |ctx| log(ctx, "before"))
            .add(Phase::Input, |ctx| log(ctx, "input"));
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 1);
        let names: Vec<_> = ctx
            .world
            .resource::<Log>()
            .0
            .iter()
            .map(|s| s.split(' ').next().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "input", "before", "physics", "after", "sim_last", "frame1", "frame2", "animate", "late", "draw"
            ]
        );
    }

    #[test]
    fn order_and_dt() {
        let mut game = Game::new("t", 1, 1)
            .sim_system(sim_a)
            .system(frame_b)
            .remove(Systems::DRAW_SPRITES);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 2);
        assert_eq!(
            ctx.world.resource::<Log>().0,
            ["sim_a dt=0.0167", "sim_a dt=0.0167", "frame_b dt=0.0000"]
        );
    }

    #[test]
    fn commands_apply_between_systems() {
        fn spawner(ctx: &mut Ctx) {
            ctx.commands.spawn((Marker,));
        }
        fn counter(ctx: &mut Ctx) {
            let n = ctx.world.query::<&Marker>().count();
            ctx.world.resource_mut::<Log>().0.push(n.to_string());
        }
        let mut game = Game::new("t", 1, 1).system(spawner).system(counter);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 0);
        game.run_systems(&mut ctx, 0);
        assert_eq!(ctx.world.resource::<Log>().0, ["1", "2"]);
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_remove_system_still_removes_builtin_and_user_systems() {
        // Built-ins are found under their `ponko::systems` path and under
        // the old crate-root spelling.
        let game = Game::new("t", 1, 1)
            .sim_system(sim_a)
            .remove_system(sim_a)
            .remove_system(crate::systems::animate_sprites)
            .remove_system(crate::draw_tilemaps)
            .remove_system(crate::draw_sprites);
        assert!(game.sim_systems.is_empty());
        // Only positional audio (feature `3d`) is left in `Draw`.
        assert!(
            game.draw_systems
                .iter()
                .all(|s| s.builtin == Some(Systems::UPDATE_AUDIO3D))
        );
        let mut game = Game::new("t", 1, 1);
        assert_eq!(game.remove_named_system(std::any::type_name_of_val(&draw_sprites)), 1);
        assert_eq!(game.system_count(Systems::DRAW_SPRITES), 0);
    }

    #[test]
    #[allow(deprecated)]
    #[should_panic(expected = "no system")]
    fn removing_unknown_system_panics() {
        let _ = Game::new("t", 1, 1).remove_system(sim_a);
    }

    #[test]
    #[allow(deprecated)]
    fn a_builtin_added_by_its_old_root_name_is_the_same_system() {
        let game = Game::new("t", 1, 1)
            .remove(Systems::DRAW_SPRITES)
            .add(Phase::Draw, crate::draw_sprites);
        assert_eq!(game.system_count(Systems::DRAW_SPRITES), 1);
        let listed = game
            .draw_systems
            .iter()
            .find(|s| s.builtin == Some(Systems::DRAW_SPRITES));
        assert_eq!(
            listed.unwrap().name,
            std::any::type_name_of_val(&crate::systems::draw_sprites)
        );
    }

    /// The built-in frame steps are systems: listed, timed and removable.
    #[cfg(feature = "3d")]
    #[test]
    fn animators_particles_and_positional_audio_are_listed_as_systems() {
        fn user_draw(ctx: &mut Ctx) {
            log(ctx, "draw");
        }
        fn late(ctx: &mut Ctx) {
            log(ctx, "late");
        }
        // Added after the built-ins: they still end their phase.
        let mut game = Game::new("t", 1, 1)
            .late_system(late)
            .add(Phase::Animate, |ctx| log(ctx, "animate"))
            .add(Phase::Draw, user_draw);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 0);
        let listed: Vec<&str> = ctx
            .perf
            .system_timings()
            .iter()
            .map(|t| t.name.rsplit("::").next().unwrap())
            .collect();
        // User systems of a phase run before the built-ins that end it.
        let at = |name: &str| {
            listed
                .iter()
                .position(|n| *n == name)
                .unwrap_or_else(|| panic!("{name}: {listed:?}"))
        };
        let user_animate = listed.iter().position(|n| n.contains("closure")).unwrap();
        assert!(user_animate < at("update_animators"));
        assert!(at("update_animators") < at("update_particles"));
        assert!(at("update_particles") < at("late"), "{listed:?}");
        // Positional audio follows the late and the drawing systems.
        assert!(at("late") < at("update_audio3d"), "{listed:?}");
        assert!(at("draw_sprites") < at("update_audio3d"), "{listed:?}");
        assert!(at("user_draw") < at("update_audio3d"), "{listed:?}");
        assert_eq!(listed.last(), Some(&"update_audio3d"), "{listed:?}");
        let phases: Vec<_> = (game.systems.iter().chain(&game.late_systems))
            .map(|s| (s.stage, s.last))
            .collect();
        assert_eq!(
            phases,
            [
                (Phase::Animate, false),
                (Phase::Animate, true),
                (Phase::Animate, true),
                (Phase::Late, false),
            ]
        );
        let last_draw = game.draw_systems.last().unwrap();
        assert_eq!((last_draw.stage, last_draw.last), (Phase::Draw, true));
        assert_eq!(last_draw.builtin, Some(Systems::UPDATE_AUDIO3D));
        assert_eq!(
            ctx.world.resource::<Log>().0,
            ["animate dt=0.0000", "late dt=0.0000", "draw dt=0.0000"]
        );

        // Removed like any other system, and placed again where the game wants it.
        let game = game
            .remove(Systems::UPDATE_PARTICLES)
            .remove(Systems::UPDATE_AUDIO3D)
            .add(Phase::Late, crate::systems::update_audio3d);
        assert_eq!(game.system_count(Systems::UPDATE_PARTICLES), 0);
        assert_eq!(game.system_count(Systems::UPDATE_ANIMATORS), 1);
        assert_eq!(game.system_count(Systems::UPDATE_AUDIO3D), 1);
        assert_eq!(game.late_systems.last().unwrap().builtin, Some(Systems::UPDATE_AUDIO3D));
        assert_ne!(game.draw_systems.last().unwrap().builtin, Some(Systems::UPDATE_AUDIO3D));
    }

    #[test]
    fn pause_step_and_speed_shape_frame_timing() {
        let mut clock = Clock::new(60);
        let mut ctx = ctx();
        let frame = 1.0 / 60.0;
        ctx.time.pause();
        assert_eq!(frame_timing(&mut clock, &mut ctx, frame).fixed_steps, 0);
        ctx.time.step();
        ctx.time.step();
        assert_eq!(
            frame_timing(&mut clock, &mut ctx, frame).fixed_steps,
            2,
            "two single steps while paused"
        );
        assert_eq!(
            frame_timing(&mut clock, &mut ctx, frame).fixed_steps,
            0,
            "steps are used up"
        );
        ctx.time.resume();
        ctx.time.set_speed(4.0);
        let steps: u32 = (0..60)
            .map(|_| frame_timing(&mut clock, &mut ctx, frame).fixed_steps)
            .sum();
        assert!((239..=240).contains(&steps), "4x speed for one second, got {steps}");
    }

    #[test]
    fn records_system_timings() {
        fn slow(_: &mut Ctx) {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let mut game = bare(Game::new("t", 1, 1).sim_system(slow)).system(frame_b);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 3);
        let t = ctx.perf.system_timings();
        assert_eq!(t.len(), 2, "{t:?}");
        assert!(t[0].sim && !t[1].sim);
        assert!(t[0].name.ends_with("slow"), "{}", t[0].name);
        assert!(t[0].last_ms >= 6.0, "3 steps of 2 ms, got {}", t[0].last_ms);
        assert!(t[0].avg_ms > 0.0 && t[0].avg_ms < t[0].last_ms);
    }

    #[test]
    fn drawing_sees_positions_from_frame_systems() {
        fn move_right(ctx: &mut Ctx) {
            ctx.world
                .query::<&mut crate::Transform2D>()
                .for_each(|t| t.position.x += 1.0);
        }
        let mut game = Game::new("t", 1, 1).system(move_right);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 0);
        let order: Vec<_> = ctx.perf.system_timings().iter().map(|t| t.name).collect();
        let at = |name: &str| order.iter().position(|n| n.ends_with(name)).unwrap();
        assert!(order[0].ends_with("move_right"), "{order:?}");
        assert!(at("move_right") < at("draw_sprites"), "{order:?}");
    }

    #[test]
    fn draw_sprites_queues_world_sprites() {
        let mut game = Game::new("t", 1, 1);
        let mut ctx = ctx();
        let tex = crate::texture::Texture::new(0, 16, 16);
        for i in 0..10 {
            ctx.world
                .spawn((crate::Transform2D::at(Vec2::splat(i as f32)), crate::Sprite::new(tex)));
        }
        ctx.world.spawn((crate::Transform2D::default(),)); // no sprite: not drawn
        game.run_systems(&mut ctx, 0);
        assert_eq!(ctx.gfx.sprite_count(), 10);
    }
}

#[cfg(test)]
mod barrier_tests;
#[cfg(test)]
mod phase_order_tests;
#[cfg(test)]
mod scenario_tests;
