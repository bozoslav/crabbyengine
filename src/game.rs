//! [`Game`]: window creation and the main loop.

use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crabby_ecs::World;

use crate::control::{self, Control};
use crate::ctx::{Ctx, SystemTiming};
use crate::ecs::draw_sprites;
use crate::input::{Key, MouseButton};
use crate::render::{FrameStatus, Renderer};
use crate::spatial::rebuild_spatial_grid;
use crate::time::{Clock, DEFAULT_FIXED_HZ, FrameTiming};

/// Trackpads report scrolling in pixels; this many logical pixels count as one
/// wheel "notch" so `mouse_wheel()` means roughly the same on every device.
const PIXELS_PER_SCROLL_LINE: f32 = 20.0;

/// How often to check whether a hidden/minimized window is visible again.
const OCCLUDED_POLL: Duration = Duration::from_millis(100);

/// A game: a window plus the loop that drives your code.
///
/// ```no_run
/// use crabbyengine::*;
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
/// use crabbyengine::*;
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
pub struct Game {
    title: String,
    width: u32,
    height: u32,
    fixed_hz: u32,
    seed: u64,
    vsync: bool,
    sim_systems: Vec<System>,
    systems: Vec<System>,
    /// Built-in drawing; runs after every other system so it sees final positions.
    draw_systems: Vec<System>,
    /// Cell size from `spatial_grid`, applied to `ctx.spatial` when the game starts.
    spatial_cell: Option<f32>,
    /// Open the control endpoint (also on with `CRABBY_CONTROL=1`).
    control: bool,
}

type SystemFn = Box<dyn FnMut(&mut Ctx)>;

/// A system: a named function that runs over the world.
struct System {
    name: &'static str,
    run: SystemFn,
    /// Simulation systems only: run on ticks where `tick % every == phase`.
    every: u32,
    phase: u32,
}

/// This frame's timing: the clock at the current sim speed (0 while
/// paused), plus any single steps requested with `ctx.step()`.
fn frame_timing(clock: &mut Clock, ctx: &mut Ctx, frame_dt: f64) -> FrameTiming {
    let mut timing = clock.advance(frame_dt, ctx.effective_sim_speed());
    timing.fixed_steps += ctx.take_pending_steps();
    timing
}

fn end_timings(ctx: &mut Ctx) {
    for t in &mut ctx.timings {
        t.avg_ms += (t.last_ms - t.avg_ms) * 0.05;
    }
}

/// Runs one system, applies its commands, and records how long both took.
#[inline]
fn run_one(s: &mut System, ctx: &mut Ctx, timing_index: usize) {
    #[cfg(feature = "profiling")]
    puffin::profile_scope!("system", s.name);
    let start = Instant::now();
    ctx.system_index = timing_index;
    (s.run)(ctx);
    ctx.apply_commands();
    ctx.timings[timing_index].last_ms += start.elapsed().as_secs_f32() * 1000.0;
}

impl System {
    fn new<F: FnMut(&mut Ctx) + 'static>(f: F) -> Self {
        System {
            name: std::any::type_name::<F>(),
            run: Box::new(f),
            every: 1,
            phase: 0,
        }
    }
}

/// What `setup` may return: nothing, or a closure to run every frame.
///
/// Implemented for `()` and for every `FnMut(&mut Ctx)`.
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
    /// # use crabbyengine::*;
    /// Game::new("Space Crab", 1280, 720).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            fixed_hz: DEFAULT_FIXED_HZ,
            seed: crate::random::DEFAULT_SEED,
            vsync: true,
            sim_systems: Vec::new(),
            systems: Vec::new(),
            draw_systems: vec![System::new(draw_sprites)],
            spatial_cell: None,
            control: control::env_flag(control::ENV_CONTROL),
        }
    }

    /// Adds a simulation system: it runs at the fixed rate
    /// ([`fixed_hz`](Self::fixed_hz), default 60 per second), zero or more
    /// times per frame, and `ctx.dt()` inside it is the fixed step.
    /// Put game logic here so it behaves the same at any frame rate.
    ///
    /// Systems run in the order they were added.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// fn gravity(ctx: &mut Ctx) { /* ... */ }
    /// fn movement(ctx: &mut Ctx) { /* ... */ }
    ///
    /// Game::new("Sim", 800, 600).sim_system(gravity).sim_system(movement).run(|_| {});
    /// ```
    pub fn sim_system<F: FnMut(&mut Ctx) + 'static>(mut self, system: F) -> Self {
        self.sim_systems.push(System::new(system));
        self
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
    /// use crabbyengine::*;
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
    pub fn sim_system_every<F: FnMut(&mut Ctx) + 'static>(mut self, ticks: u32, system: F) -> Self {
        assert!(
            ticks > 0,
            "Game::sim_system_every: the interval must be at least 1 tick"
        );
        let same = self.sim_systems.iter().filter(|s| s.every == ticks).count() as u32;
        let mut s = System::new(system);
        s.every = ticks;
        s.phase = same % ticks;
        self.sim_systems.push(s);
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
        let name = std::any::type_name_of_val(&rebuild_spatial_grid);
        if !self.sim_systems.iter().any(|s| s.name == name) {
            self.sim_systems.insert(0, System::new(rebuild_spatial_grid));
        }
        self
    }

    /// Adds a frame system: it runs once per rendered frame, after the
    /// simulation systems. Use it for input, camera and drawing.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// fn quit_on_escape(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::Escape) { ctx.quit(); }
    /// }
    ///
    /// Game::new("Sim", 800, 600).system(quit_on_escape).run(|_| {});
    /// ```
    pub fn system<F: FnMut(&mut Ctx) + 'static>(mut self, system: F) -> Self {
        self.systems.push(System::new(system));
        self
    }

    /// Removes a system that was added before, including the built-in
    /// [`draw_sprites`]. Pass the same function you added.
    ///
    /// # Panics
    /// If no such system was added.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// // A headless-style simulation that draws nothing by itself.
    /// Game::new("Sim", 800, 600).remove_system(draw_sprites).run(|_| {});
    /// ```
    #[track_caller]
    pub fn remove_system<F: FnMut(&mut Ctx) + 'static>(mut self, system: F) -> Self {
        drop(system);
        let name = std::any::type_name::<F>();
        let count = |g: &Game| g.sim_systems.len() + g.systems.len() + g.draw_systems.len();
        let before = count(&self);
        self.sim_systems.retain(|s| s.name != name);
        self.systems.retain(|s| s.name != name);
        self.draw_systems.retain(|s| s.name != name);
        assert_ne!(
            before,
            count(&self),
            "Game::remove_system: no system `{name}` was added"
        );
        self
    }

    /// Opens the control endpoint, so tools such as the `crabby-mcp` agent
    /// bridge can inspect and drive this game while it runs: list and edit
    /// entities (through [reflection](crate::WorldReflect)), pause, step,
    /// change speed, read system timings.
    ///
    /// It listens on `127.0.0.1` only, and every request must carry a random
    /// token written to the instance registry (`~/.crabby/instances`),
    /// which only your user can read. Off by default; setting the
    /// environment variable `CRABBY_CONTROL=1` turns it on without code
    /// changes, and `CRABBY_START_PAUSED=1` starts the simulation paused.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Colony", 1280, 720).control_endpoint(true).run(|_| {});
    /// ```
    pub fn control_endpoint(mut self, on: bool) -> Self {
        self.control = on;
        self
    }

    /// Starts the control endpoint if it's enabled, and applies `CRABBY_START_PAUSED`.
    fn start_control(&self, ctx: &mut Ctx, headless: bool) -> Option<Control> {
        if control::env_flag(control::ENV_START_PAUSED) {
            ctx.pause();
        }
        if !self.control {
            return None;
        }
        match Control::start(&self.title, headless) {
            Ok(c) => {
                ctx.control_addr = Some(c.addr());
                Some(c)
            }
            Err(e) => {
                eprintln!("crabbyengine: could not open the control endpoint: {e}");
                None
            }
        }
    }

    /// Sets the seed for [`Ctx::random`]. Same seed, same simulation: runs
    /// are reproducible by default ([`DEFAULT_SEED`](crate::DEFAULT_SEED)).
    /// For a different world each run, seed from the clock.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    /// Game::new("Roguelike", 800, 600).seed(seed).run(|_| {});
    /// ```
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets how many fixed updates run per second (default 60).
    /// See [`Ctx::fixed_steps`].
    ///
    /// # Panics
    /// If `hz` is 0.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Physics", 800, 600).fixed_hz(120).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn fixed_hz(mut self, hz: u32) -> Self {
        assert!(hz > 0, "Game::fixed_hz: the rate must be at least 1 Hz");
        self.fixed_hz = hz;
        self
    }

    /// Turns vsync on (default) or off. With vsync the game runs at the
    /// display's refresh rate with no tearing; turn it off to measure how fast
    /// the game *could* run.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Benchmark", 800, 600).vsync(false).run(|_| |_: &mut Ctx| {});
    /// ```
    pub fn vsync(mut self, on: bool) -> Self {
        self.vsync = on;
        self
    }

    /// One frame of game logic: simulation systems `fixed_steps` times, then
    /// frame systems, then built-in drawing. Commands are applied after every system.
    fn run_systems(&mut self, ctx: &mut Ctx, fixed_steps: u32) {
        self.begin_timings(ctx);
        ctx.set_in_sim(true);
        for _ in 0..fixed_steps {
            self.run_sim_tick(ctx);
        }
        ctx.set_in_sim(false);
        let n_sim = self.sim_systems.len();
        let n_frame = self.systems.len();
        for (i, s) in self.systems.iter_mut().enumerate() {
            run_one(s, ctx, n_sim + i);
        }
        for (i, s) in self.draw_systems.iter_mut().enumerate() {
            run_one(s, ctx, n_sim + n_frame + i);
        }
        end_timings(ctx);
    }

    /// One simulation tick: every sim system once, then the tick counter.
    fn run_sim_tick(&mut self, ctx: &mut Ctx) {
        let tick = ctx.sim_tick();
        for (i, s) in self.sim_systems.iter_mut().enumerate() {
            if tick % u64::from(s.every) != u64::from(s.phase) {
                continue;
            }
            ctx.set_sim_steps(s.every);
            run_one(s, ctx, i);
        }
        ctx.set_sim_steps(1);
        ctx.advance_tick();
    }

    /// Makes sure there's one timing slot per system, and zeroes this frame's times.
    fn begin_timings(&self, ctx: &mut Ctx) {
        if ctx.timings.len() != self.sim_systems.len() + self.systems.len() + self.draw_systems.len() {
            ctx.timings = (self.sim_systems.iter().map(|s| (s, true)))
                .chain(self.systems.iter().chain(&self.draw_systems).map(|s| (s, false)))
                .map(|(s, sim)| SystemTiming {
                    name: s.name,
                    sim,
                    last_ms: 0.0,
                    avg_ms: 0.0,
                })
                .collect();
        }
        for t in &mut ctx.timings {
            t.last_ms = 0.0;
        }
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
    /// use crabbyengine::*;
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
    pub fn run_headless<S, U>(mut self, ticks: u64, setup: S) -> World
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        let fixed_dt = Clock::new(self.fixed_hz).fixed_dt();
        let mut ctx = Ctx::new(fixed_dt, Vec2::new(self.width as f32, self.height as f32));
        ctx.seed = self.seed;
        ctx.headless = true;
        if let Some(cell) = self.spatial_cell {
            ctx.spatial = crate::SpatialGrid::new(cell);
        }
        let control = self.start_control(&mut ctx, true);
        drop(setup(&mut ctx).into_frame_fn()); // no frames, so no frame closure
        ctx.apply_commands();
        ctx.discard_draws();

        let timing = FrameTiming {
            dt: fixed_dt as f32,
            fixed_steps: 1,
            alpha: 0.0,
        };
        let mut ran = 0;
        while ran < ticks && !ctx.quit_requested() {
            if let Some(control) = &control {
                control.poll(&mut ctx);
                if ctx.quit_requested() {
                    break;
                }
                // A paused game runs only the ticks tools ask for.
                if ctx.is_paused() && !ctx.take_one_step() {
                    control.wait(&mut ctx, Duration::from_millis(50));
                    continue;
                }
            }
            ran += 1;
            ctx.begin_frame(timing);
            self.begin_timings(&mut ctx);
            ctx.set_in_sim(true);
            self.run_sim_tick(&mut ctx);
            ctx.set_in_sim(false);
            end_timings(&mut ctx);
            ctx.discard_draws();
        }
        std::mem::take(&mut ctx.world)
    }

    /// Opens the window and runs the game until it is closed.
    ///
    /// `setup` runs once, after the window exists. It can spawn entities into
    /// `ctx.world`, and it can return a closure that runs every frame (after
    /// the systems). Variables created in `setup` and moved into that closure
    /// are your game state; no structs or traits required.
    ///
    /// Each frame runs: simulation systems (`ctx.fixed_steps()` times), then
    /// frame systems, then the closure from `setup`, then the built-in
    /// [`draw_sprites`], then rendering.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Counter", 800, 600).run(|_ctx| {
    ///     let mut clicks = 0;
    ///     move |ctx: &mut Ctx| {
    ///         if ctx.mouse_pressed(MouseButton::Left) {
    ///             clicks += 1;
    ///             println!("clicks: {clicks}");
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    /// If the operating system refuses to create the event loop or window,
    /// or no usable GPU is found.
    pub fn run<S, U>(self, setup: S)
    where
        S: FnOnce(&mut Ctx) -> U,
        U: SetupResult,
    {
        #[cfg(feature = "profiling")]
        let _puffin_server = start_profiler();

        let event_loop =
            EventLoop::new().unwrap_or_else(|e| panic!("crabbyengine: could not start the event loop: {e}"));

        let clock = Clock::new(self.fixed_hz);
        let mut ctx = Ctx::new(clock.fixed_dt(), Vec2::new(self.width as f32, self.height as f32));
        ctx.seed = self.seed;
        if let Some(cell) = self.spatial_cell {
            ctx.spatial = crate::SpatialGrid::new(cell);
        }
        let control = self.start_control(&mut ctx, false);
        let mut app = App {
            control,
            config: self,
            setup: Some(setup),
            started: false,
            window: None,
            ctx,
            clock,
            last_frame: Instant::now(),
            occluded: false,
        };

        event_loop
            .run_app(&mut app)
            .unwrap_or_else(|e| panic!("crabbyengine: the event loop stopped with an error: {e}"));
    }
}

/// Starts the puffin server; the returned handle keeps it running.
#[cfg(feature = "profiling")]
fn start_profiler() -> Option<puffin_http::Server> {
    const ADDR: &str = "127.0.0.1:8585";
    puffin::set_scopes_on(true);
    match puffin_http::Server::new(ADDR) {
        Ok(server) => {
            eprintln!("crabbyengine: profiler running; view it with `puffin_viewer --url {ADDR}`");
            Some(server)
        }
        Err(e) => {
            eprintln!("crabbyengine: could not start the profiler on {ADDR}: {e}");
            None
        }
    }
}

struct App<S> {
    /// Kept for the whole run: dropping it closes the endpoint.
    control: Option<Control>,
    config: Game,
    setup: Option<S>,
    /// Setup has run; frames may run.
    started: bool,
    window: Option<Arc<Window>>,
    ctx: Ctx,
    clock: Clock,
    last_frame: Instant,
    /// Window hidden/minimized: stop drawing, poll slowly instead of spinning.
    occluded: bool,
}

impl<S, U> App<S>
where
    S: FnOnce(&mut Ctx) -> U,
    U: SetupResult,
{
    fn logical_window_size(window: &Window) -> Vec2 {
        let size: LogicalSize<f32> = window.inner_size().to_logical(window.scale_factor());
        Vec2::new(size.width, size.height)
    }

    fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        if !self.started {
            return;
        }

        if let Some(control) = &self.control {
            control.poll(&mut self.ctx);
        }
        let now = Instant::now();
        let frame_dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        #[cfg(feature = "profiling")]
        puffin::GlobalProfiler::lock().new_frame();
        let timing = frame_timing(&mut self.clock, &mut self.ctx, frame_dt);
        self.ctx.begin_frame(timing);
        self.config.run_systems(&mut self.ctx, timing.fixed_steps);
        self.ctx.input.end_frame();

        if self.ctx.quit_requested() {
            event_loop.exit();
            return;
        }

        // With vsync, presenting blocks until the display is ready, which
        // paces the loop. So we can ask for the next frame right away.
        match self.ctx.render_frame() {
            FrameStatus::Presented | FrameStatus::Skipped => self.occluded = false,
            FrameStatus::Occluded => self.occluded = true,
        }
        if !self.occluded
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

impl<S, U> ApplicationHandler for App<S>
where
    S: FnOnce(&mut Ctx) -> U,
    U: SetupResult,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = event_loop
            .create_window(attributes)
            .unwrap_or_else(|e| panic!("crabbyengine: could not create the window: {e}"));
        let window = Arc::new(window);

        self.ctx.renderer = Some(Renderer::new(
            window.clone(),
            event_loop.owned_display_handle(),
            self.config.vsync,
        ));
        self.ctx.set_window_size(Self::logical_window_size(&window));
        self.ctx.set_scale_factor(window.scale_factor() as f32);

        if let Some(setup) = self.setup.take() {
            let frame_fn = setup(&mut self.ctx).into_frame_fn();
            self.ctx.apply_commands();
            if let Some(f) = frame_fn {
                self.config.systems.push(System {
                    name: "setup closure",
                    run: f,
                    every: 1,
                    phase: 0,
                });
            }
            self.started = true;
        }

        // Don't count setup time (e.g. asset loading) as the first frame's dt.
        self.last_frame = Instant::now();
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => self.run_frame(event_loop),

            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let size = window.inner_size();
                if let Some(renderer) = self.ctx.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
                self.ctx.set_window_size(Self::logical_window_size(&window));
                self.ctx.set_scale_factor(window.scale_factor() as f32);
                window.request_redraw();
            }

            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    window.request_redraw();
                }
            }

            WindowEvent::Focused(false) => self.ctx.input.release_all(),

            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(key) = Key::from_winit(code)
                {
                    self.ctx.input.set_key(key, event.state == ElementState::Pressed);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = MouseButton::from_winit(button) {
                    self.ctx.input.set_mouse_button(button, state == ElementState::Pressed);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let pos = position.to_logical::<f32>(window.scale_factor());
                self.ctx.input.set_mouse_pos(Vec2::new(pos.x, pos.y));
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        let p: PhysicalPosition<f64> = p;
                        let logical = p.to_logical::<f32>(window.scale_factor());
                        Vec2::new(logical.x, logical.y) / PIXELS_PER_SCROLL_LINE
                    }
                };
                self.ctx.input.add_mouse_wheel(lines);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.occluded {
            // Hidden: check back now and then instead of spinning a core.
            // The simulation pauses while hidden (dt is clamped on return).
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + OCCLUDED_POLL));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
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
    fn frame_b(ctx: &mut Ctx) {
        log(ctx, "frame_b");
    }

    #[test]
    fn order_and_dt() {
        let mut game = Game::new("t", 1, 1)
            .sim_system(sim_a)
            .system(frame_b)
            .remove_system(draw_sprites);
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
    fn remove_system_removes_builtin_and_user_systems() {
        let game = Game::new("t", 1, 1)
            .sim_system(sim_a)
            .remove_system(draw_sprites)
            .remove_system(sim_a);
        assert!(game.systems.is_empty() && game.sim_systems.is_empty() && game.draw_systems.is_empty());
    }

    #[test]
    #[should_panic(expected = "no system")]
    fn removing_unknown_system_panics() {
        let _ = Game::new("t", 1, 1).remove_system(sim_a);
    }

    #[test]
    fn pause_step_and_speed_shape_frame_timing() {
        let mut clock = Clock::new(60);
        let mut ctx = ctx();
        let frame = 1.0 / 60.0;
        ctx.pause();
        assert_eq!(frame_timing(&mut clock, &mut ctx, frame).fixed_steps, 0);
        ctx.step();
        ctx.step();
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
        ctx.resume();
        ctx.set_sim_speed(4.0);
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
        let mut game = Game::new("t", 1, 1)
            .sim_system(slow)
            .remove_system(draw_sprites)
            .system(frame_b);
        let mut ctx = ctx();
        game.run_systems(&mut ctx, 3);
        let t = ctx.system_timings();
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
        let order: Vec<_> = ctx.system_timings().iter().map(|t| t.name).collect();
        assert!(
            order[0].ends_with("move_right") && order[1].ends_with("draw_sprites"),
            "{order:?}"
        );
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
        assert_eq!(ctx.sprite_count(), 10);
    }
}
