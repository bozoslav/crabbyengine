//! [`Ctx`]: everything game code talks to each frame.

use crabby_ecs::{Commands, Entity, World};
use glam::{Mat4, Vec2};

use crate::assets;
use crate::audio::Audio;
use crate::camera::Camera2D;
use crate::camera::Rect;
use crate::color::Color;
use crate::ecs::{Sprite, Transform2D};
use crate::hot_reload::Asset;
use crate::input::{Input, Key, MouseButton};
use crate::reflect::WorldReflect;
use crate::render::batch::{DrawList, SpriteInstance};
use crate::render::batch::{FULL_UV, uv_region};
use crate::render::{FrameStatus, RenderStats, Renderer, Screenshot};
use crate::spatial::SpatialGrid;
use crate::text::{Font, Text, TextParams};
use crate::texture::{DrawParams, Texture, TextureFilter};
use crate::time::FrameTiming;
use crate::ui::{UiState, UiStyle};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Simulation clock state: speed, pause, single steps, tick count.
struct SimClock {
    speed: f32,
    paused: bool,
    pending_steps: u32,
    tick: u64,
    /// Fixed step in seconds, full precision.
    step: f64,
}

/// How long one system took. See [`Ctx::system_timings`].
#[derive(Debug, Clone, PartialEq)]
pub struct SystemTiming {
    /// The system's function name, e.g. `my_game::movement`.
    pub name: &'static str,
    /// `true` for simulation systems (they may run several times per frame).
    pub sim: bool,
    /// Time spent in the system during the last frame (all fixed steps
    /// together), in milliseconds.
    pub last_ms: f32,
    /// Smoothed average of `last_ms`.
    pub avg_ms: f32,
}

/// Maps window pixels ((0, 0) top-left, y down) to wgpu clip space
/// (x right, y up, both -1..1).
fn screen_projection(size: Vec2) -> Mat4 {
    let size = size.max(Vec2::ONE);
    Mat4::from_cols(
        glam::vec4(2.0 / size.x, 0.0, 0.0, 0.0),
        glam::vec4(0.0, -2.0 / size.y, 0.0, 0.0),
        glam::vec4(0.0, 0.0, 1.0, 0.0),
        glam::vec4(-1.0, 1.0, 0.0, 1.0),
    )
}

/// The game's handle to the engine, passed to your systems and update closure.
///
/// Use it to draw, read input, check timing and window info, and reach the
/// ECS [`World`] through `ctx.world`.
/// Applies a data file's text to the world.
type DataLoader = Box<dyn Fn(&mut World, &str) -> Result<(), String>>;

pub struct Ctx {
    /// All entities, components and resources.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// struct Vel(Vec2);
    ///
    /// fn movement(ctx: &mut Ctx) {
    ///     let dt = ctx.dt();
    ///     ctx.world.query::<(&mut Transform2D, &Vel)>().for_each(|(t, v)| {
    ///         t.position += v.0 * dt;
    ///     });
    /// }
    /// ```
    pub world: World,
    /// Changes to make once the current system finishes, for spawning or
    /// despawning while a query is running. Applied automatically after
    /// every system.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// struct Health(i32);
    ///
    /// fn deaths(ctx: &mut Ctx) {
    ///     ctx.world.query::<(Entity, &Health)>().for_each(|(e, h)| {
    ///         if h.0 <= 0 {
    ///             ctx.commands.despawn(e);
    ///         }
    ///     });
    /// }
    /// ```
    pub commands: Commands,
    /// Neighbor queries over every entity's [`Transform2D`] position; filled
    /// at the start of each simulation tick once enabled with
    /// [`Game::spatial_grid`](crate::Game::spatial_grid) (empty otherwise).
    pub spatial: SpatialGrid,
    /// `true` while simulation systems run: `dt()` then returns the fixed step.
    in_sim: bool,
    /// Fixed steps the running sim system covers (`sim_system_every`).
    sim_steps: u32,
    /// Seed for `random()`.
    pub(crate) seed: u64,
    /// Index of the running system, so systems get different random streams.
    pub(crate) system_index: usize,
    sim: SimClock,
    /// No window or GPU (`Game::run_headless`): textures are size-only handles.
    pub(crate) headless: bool,
    /// Where the control endpoint listens, if it's on.
    pub(crate) control_addr: Option<std::net::SocketAddr>,
    headless_textures: u32,
    /// Asset hot reload: which files textures came from.
    pub(crate) watcher: crate::hot_reload::Watcher,
    /// How to apply each loaded data file's text (`load_resource`).
    data_loaders: Vec<DataLoader>,
    pub(crate) timings: Vec<SystemTiming>,
    pub(crate) input: Input,
    timing: FrameTiming,
    fixed_dt: f32,
    time: f64,
    frame: u64,
    fps: f32,
    window_size: Vec2,
    quit_requested: bool,
    pub(crate) renderer: Option<Renderer>,
    /// Screenshots waiting for the next drawn frame.
    screenshots: Vec<ScreenshotRequest>,
    /// World-space sprites (through the camera).
    pub(crate) draw_list: DrawList,
    /// Screen-space sprites and text, drawn on top.
    pub(crate) ui_list: DrawList,
    clear_color: Color,
    /// Which part of the world the window shows. Move it, zoom it, or make
    /// it follow something; see [`Camera2D`].
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.camera.zoom = 2.0;
    /// ctx.camera.look_at(vec2(1000.0, 1000.0));
    /// # }
    /// ```
    pub camera: Camera2D,
    text: Text,
    /// The glyph atlas, created on the first `draw_text`.
    font_atlas: Option<Texture>,
    /// Physical pixels per window pixel (2 on most Retina screens).
    scale_factor: f32,
    pub(crate) ui: UiState,
    /// Where the left mouse button last went down, for button clicks.
    pub(crate) ui_press_at: Vec2,
    /// Colors and sizes of [`ui_button`](Self::ui_button) and friends.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// ctx.ui_style.text_size = 20.0;
    /// # }
    /// ```
    pub ui_style: UiStyle,
    pub(crate) audio: Audio,
}

impl Ctx {
    /// `fixed_dt` in seconds, as f64 so `sim_time()` stays exact over long runs.
    pub(crate) fn new(fixed_dt: f64, window_size: Vec2) -> Self {
        Self {
            world: {
                let mut world = World::new();
                crate::reflect::WorldReflect::reflect::<crate::Transform2D>(&mut world);
                world
            },
            commands: Commands::new(),
            spatial: SpatialGrid::default(),
            in_sim: false,
            sim_steps: 1,
            seed: crate::random::DEFAULT_SEED,
            system_index: 0,
            sim: SimClock {
                speed: 1.0,
                paused: false,
                pending_steps: 0,
                tick: 0,
                step: fixed_dt,
            },
            headless: false,
            control_addr: None,
            headless_textures: 0,
            watcher: Default::default(),
            data_loaders: Vec::new(),
            timings: Vec::new(),
            input: Input::new(),
            timing: FrameTiming {
                dt: 0.0,
                fixed_steps: 0,
                alpha: 0.0,
            },
            fixed_dt: fixed_dt as f32,
            time: 0.0,
            frame: 0,
            fps: 0.0,
            window_size,
            quit_requested: false,
            renderer: None,
            screenshots: Vec::new(),
            draw_list: DrawList::default(),
            ui_list: DrawList::default(),
            camera: Camera2D::new(window_size),
            text: Text::new(),
            font_atlas: None,
            scale_factor: 1.0,
            ui: UiState::default(),
            ui_press_at: Vec2::splat(f32::NAN),
            ui_style: UiStyle::default(),
            audio: Audio::default(),
            clear_color: Color::CRABBY_DARK,
        }
    }

    /// Engine side: draw everything queued this frame, then reset the queue.
    pub(crate) fn render_frame(&mut self) -> FrameStatus {
        let capture = !self.screenshots.is_empty();
        let status = match self.renderer.as_mut() {
            Some(renderer) => renderer.render(
                &mut self.draw_list,
                &mut self.ui_list,
                self.clear_color,
                self.camera.view_proj(),
                screen_projection(self.window_size),
                capture,
            ),
            None => FrameStatus::Skipped,
        };
        self.draw_list.clear();
        self.ui_list.clear();
        if let Some(shot) = self.renderer.as_mut().and_then(|r| r.screenshot.take()) {
            for req in self.screenshots.drain(..) {
                let result = save_png(&req.path, &shot, req.max_size);
                match req.reply {
                    Some(reply) => {
                        let _ = reply.send(result);
                    }
                    None => {
                        if let Err(e) = result {
                            eprintln!("crabbyengine: {e}");
                        }
                    }
                }
            }
        }
        status
    }

    pub(crate) fn queue_screenshot(
        &mut self,
        path: PathBuf,
        max_size: u32,
        reply: Option<Sender<Result<Value, String>>>,
    ) {
        self.screenshots.push(ScreenshotRequest { path, max_size, reply });
    }

    fn renderer(&mut self) -> &mut Renderer {
        self.renderer
            .as_mut()
            .expect("crabbyengine: textures can only be created once the game is running (in setup or update)")
    }

    /// Engine side: start a new frame with fresh timing.
    pub(crate) fn begin_frame(&mut self, timing: FrameTiming) {
        self.timing = timing;
        self.time += f64::from(timing.dt);
        self.frame += 1;
        self.text.begin_frame();
        self.ui.begin_frame();
        if timing.dt > 0.0 {
            let instant = 1.0 / timing.dt;
            // Exponential smoothing so the number is readable, seeded by the first sample.
            self.fps = if self.fps == 0.0 {
                instant
            } else {
                self.fps + (instant - self.fps) * 0.05
            };
        }
    }

    pub(crate) fn set_scale_factor(&mut self, scale: f32) {
        self.scale_factor = scale.max(0.25);
    }

    pub(crate) fn set_window_size(&mut self, size: Vec2) {
        self.window_size = size;
        self.camera.set_viewport(size);
    }

    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// How long each system took, in the order they run (simulation systems
    /// first). For finding what's slow; the inspector will graph these.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn print_slow_systems(ctx: &mut Ctx) {
    ///     for t in ctx.system_timings() {
    ///         if t.avg_ms > 1.0 {
    ///             println!("{}: {:.2} ms", t.name, t.avg_ms);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn system_timings(&self) -> &[SystemTiming] {
        &self.timings
    }

    // ------------------------------------------------------- simulation clock

    /// Stops simulation systems from running (frame systems and drawing go on).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn controls(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::Space) {
    ///         if ctx.is_paused() { ctx.resume() } else { ctx.pause() }
    ///     }
    ///     if ctx.key_pressed(Key::Period) {
    ///         ctx.step(); // advance one tick while paused
    ///     }
    /// }
    /// ```
    pub fn pause(&mut self) {
        self.sim.paused = true;
    }

    /// Lets simulation systems run again after [`pause`](Self::pause).
    pub fn resume(&mut self) {
        self.sim.paused = false;
    }

    /// `true` while the simulation is paused.
    pub fn is_paused(&self) -> bool {
        self.sim.paused
    }

    /// Runs exactly one extra simulation tick next frame. Meant for stepping
    /// through a paused simulation; each call adds one tick.
    pub fn step(&mut self) {
        self.sim.pending_steps = self.sim.pending_steps.saturating_add(1);
    }

    /// Sets how fast simulated time runs: `1.0` real time, `4.0` four times
    /// as fast, `0.5` half speed. Ticks stay the same length (the fixed step),
    /// so results don't depend on speed; only how many ticks run per frame
    /// changes. If the simulation can't keep up, it runs as fast as it can.
    ///
    /// # Panics
    /// If `speed` is negative, NaN or infinite.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.set_sim_speed(16.0); // fast-forward
    /// # }
    /// ```
    #[track_caller]
    pub fn set_sim_speed(&mut self, speed: f32) {
        assert!(
            speed.is_finite() && speed >= 0.0,
            "Ctx::set_sim_speed: speed must be a finite number >= 0 (got {speed})"
        );
        self.sim.speed = speed;
    }

    /// Current simulation speed (see [`set_sim_speed`](Self::set_sim_speed)).
    pub fn sim_speed(&self) -> f32 {
        self.sim.speed
    }

    /// Number of simulation ticks run so far. Use it to do work every Nth
    /// tick, or to timestamp events deterministically.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn slow_ai(ctx: &mut Ctx) {
    ///     if ctx.sim_tick() % 10 != 0 {
    ///         return; // think 6 times per second instead of 60
    ///     }
    ///     // ...
    /// }
    /// ```
    pub fn sim_tick(&self) -> u64 {
        self.sim.tick
    }

    /// Simulated seconds so far: `sim_tick() * fixed_dt()`.
    pub fn sim_time(&self) -> f64 {
        self.sim.tick as f64 * self.sim.step
    }

    /// Engine side: speed to feed the clock this frame (0 while paused).
    pub(crate) fn effective_sim_speed(&self) -> f64 {
        if self.sim.paused {
            0.0
        } else {
            f64::from(self.sim.speed)
        }
    }

    /// Engine side: single steps requested since the last frame.
    /// Uses up one requested single step; `false` if none was pending.
    pub(crate) fn take_one_step(&mut self) -> bool {
        let had = self.sim.pending_steps > 0;
        self.sim.pending_steps = self.sim.pending_steps.saturating_sub(1);
        had
    }

    /// The address of this game's control endpoint, or `None` if it's off.
    /// Tools find it in the instance registry; this is for tests and logs.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut addr = None;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| addr = ctx.control_addr());
    /// assert_eq!(addr, None); // off unless enabled
    /// ```
    pub fn control_addr(&self) -> Option<std::net::SocketAddr> {
        self.control_addr
    }

    pub(crate) fn take_pending_steps(&mut self) -> u32 {
        std::mem::take(&mut self.sim.pending_steps)
    }

    pub(crate) fn advance_tick(&mut self) {
        self.sim.tick += 1;
    }

    /// Engine side: forget queued draws (headless runs never render).
    pub(crate) fn discard_draws(&mut self) {
        self.draw_list.clear();
        self.ui_list.clear();
    }

    pub(crate) fn set_sim_steps(&mut self, steps: u32) {
        self.sim_steps = steps;
    }

    /// Random numbers for this system and tick: `ctx.random().of(entity)`
    /// for an entity's own stream (safe in `par_for_each`), or
    /// `.stream(n)` for anything else. A simulation run twice with the same
    /// [seed](crate::Game::seed) gives the same results.
    ///
    /// ```
    /// use crabbyengine::*;
    ///
    /// struct Pos(f32);
    ///
    /// fn jitter(ctx: &mut Ctx) {
    ///     let random = ctx.random();
    ///     ctx.world.query::<(Entity, &mut Pos)>().par_for_each(|(e, p)| p.0 += random.of(e).range(-1.0, 1.0));
    /// }
    ///
    /// let run = || {
    ///     let mut w = Game::new("t", 1, 1).sim_system(jitter).run_headless(100, |ctx| {
    ///         ctx.world.spawn_batch((0..1000).map(|_| (Pos(0.0),)));
    ///     });
    ///     w.query::<&Pos>().iter().map(|p| p.0).sum::<f32>()
    /// };
    /// assert_eq!(run(), run()); // same seed, same result
    /// ```
    pub fn random(&self) -> crate::Random {
        crate::Random::new(self.seed, self.sim_tick(), self.system_index)
    }

    /// Splits per-entity work over `every` ticks: entities take turns, so
    /// each tick updates 1/`every` of them. For costly logic that's fine at
    /// a lower rate per entity (path planning, AI decisions). Take it before
    /// the query; it is `Copy`, so it works in `par_for_each` too.
    ///
    /// ```
    /// use crabbyengine::*;
    ///
    /// struct Brain { plans: u32 }
    ///
    /// fn plan(ctx: &mut Ctx) {
    ///     let turn = ctx.turns(4);
    ///     ctx.world.query::<(Entity, &mut Brain)>().par_for_each(|(e, b)| {
    ///         if turn.includes(e) {
    ///             b.plans += 1; // each entity plans every 4th tick
    ///         }
    ///     });
    /// }
    ///
    /// let mut world = Game::new("t", 1, 1).sim_system(plan).run_headless(40, |ctx| {
    ///     for _ in 0..10 { ctx.world.spawn((Brain { plans: 0 },)); }
    /// });
    /// assert!(world.query::<&Brain>().iter().all(|b| b.plans == 10));
    /// ```
    ///
    /// # Panics
    /// If `every` is 0.
    #[track_caller]
    pub fn turns(&self, every: u32) -> Turns {
        assert!(every > 0, "Ctx::turns: `every` must be at least 1");
        Turns {
            every,
            now: (self.sim_tick() % u64::from(every)) as u32,
            dt: self.dt() * every as f32,
        }
    }

    pub(crate) fn set_in_sim(&mut self, in_sim: bool) {
        self.in_sim = in_sim;
    }

    /// Engine side: apply commands recorded by the system that just ran.
    pub(crate) fn apply_commands(&mut self) {
        if !self.commands.is_empty() {
            self.world.apply(&mut self.commands);
        }
    }

    /// Queues every `(Transform2D, Sprite)` entity for drawing.
    pub(crate) fn draw_world_sprites(&mut self) {
        /// Below this many sprites, one thread is faster than many.
        const PARALLEL_MIN: usize = 20_000;
        fn instance(t: &Transform2D, s: &Sprite) -> SpriteInstance {
            let (size, uv) = region(&s.texture, s.region);
            SpriteInstance::new(
                t.position.into(),
                (size * t.scale).into(),
                s.pivot.into(),
                t.rotation,
                s.color.to_rgba8(),
            )
            .with_uv(uv)
        }
        let draw_list = &mut self.draw_list;
        let mut query = self.world.query::<(&Transform2D, &Sprite)>();
        let n = query.count();
        if n < PARALLEL_MIN {
            query.for_each(|(t, s)| draw_list.push(s.texture.id, s.z, instance(t, s)));
            return;
        }
        // SAFETY: `par_for_each_indexed` visits each of the query's `n`
        // entities exactly once, with indices `0..n`.
        unsafe {
            draw_list.extend(n, |slots| {
                query.par_for_each_indexed(|i, (t, s)| slots.write(i, s.texture.id, s.z, instance(t, s)));
            });
        }
    }

    // ---------------------------------------------------------------- drawing

    /// Sets the background color. It stays until you change it.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// ctx.clear(Color::hex(0x203040));
    /// # }
    /// ```
    pub fn clear(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Draws `texture` centered at `pos`, in world units (window pixels until
    /// you move or zoom [`camera`](Self::camera)).
    ///
    /// Drawing is cheap: calls are collected and sent to the GPU in big
    /// batches at the end of the frame. Call it every frame for everything
    /// you want visible.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, crab: Texture) {
    /// ctx.draw(&crab, vec2(400.0, 300.0));
    /// # }
    /// ```
    pub fn draw(&mut self, texture: &Texture, pos: Vec2) {
        push_sprite(
            &mut self.draw_list,
            texture,
            pos,
            Vec2::ONE,
            0.0,
            0.0,
            Vec2::splat(0.5),
            [255; 4],
            None,
        );
    }

    /// Draws `texture` with rotation, scale, tint, draw order or pivot.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, crab: Texture) {
    /// ctx.draw_ex(&crab, vec2(400.0, 300.0), DrawParams {
    ///     rotation: ctx.time() as f32,
    ///     color: Color::RED.with_alpha(0.5),
    ///     z: 1.0, // on top of everything at z = 0
    ///     ..Default::default()
    /// });
    /// # }
    /// ```
    pub fn draw_ex(&mut self, texture: &Texture, pos: Vec2, params: DrawParams) {
        push_sprite(
            if params.screen {
                &mut self.ui_list
            } else {
                &mut self.draw_list
            },
            texture,
            pos,
            params.scale,
            params.rotation,
            params.z,
            params.pivot,
            params.color.to_rgba8(),
            params.region,
        );
    }

    /// Draws `text` with its top-left corner at `pos`, in window pixels (on
    /// top of the world, ignoring the camera). `size` is the line height in
    /// pixels. `\n` starts a new line.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn hud(ctx: &mut Ctx) {
    /// let fps = ctx.fps();
    /// ctx.draw_text(&format!("{fps:.0} FPS"), vec2(10.0, 10.0), 18.0, Color::WHITE);
    /// # }
    /// ```
    pub fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        self.draw_text_ex(
            text,
            pos,
            TextParams {
                size,
                color,
                ..Default::default()
            },
        );
    }

    /// [`draw_text`](Self::draw_text) with a font and draw order.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn hud(ctx: &mut Ctx, title: Font) {
    /// ctx.draw_text_ex("Colony", vec2(20.0, 20.0), TextParams {
    ///     font: title,
    ///     size: 40.0,
    ///     z: 1.0,
    ///     ..Default::default()
    /// });
    /// # }
    /// ```
    pub fn draw_text_ex(&mut self, text: &str, pos: Vec2, params: TextParams) {
        if self.headless || text.is_empty() {
            return;
        }
        let atlas = match self.font_atlas {
            Some(t) => t,
            None => {
                let side = Text::ATLAS_SIZE;
                let t = self.create_texture(side, side, &vec![0; (side * side * 4) as usize]);
                self.font_atlas = Some(t);
                t
            }
        };
        let scale = self.scale_factor;
        self.text.layout(text, params.font, params.size * scale, true);
        if !self.text.uploads.is_empty()
            && let Some(renderer) = self.renderer.as_mut()
        {
            for (at, size, rgba) in self.text.uploads.drain(..) {
                renderer.update_texture_region(atlas.id, at, size, &rgba);
            }
        }
        let side = Text::ATLAS_SIZE as f32;
        let color = params.color.to_rgba8();
        for p in self.text.placed() {
            let Some(at) = p.glyph.at else { continue };
            let size = Vec2::new(p.glyph.size[0] as f32, p.glyph.size[1] as f32);
            let uv = uv_region([at[0] as f32 / side, at[1] as f32 / side], (size / side).into());
            self.ui_list.push(
                atlas.id,
                params.z,
                SpriteInstance::new(
                    (pos + p.pos / scale).into(),
                    (size / scale).into(),
                    [0.0, 0.0],
                    0.0,
                    color,
                )
                .with_uv(uv),
            );
        }
    }

    /// The size `text` would take with [`draw_text`](Self::draw_text) at
    /// `size`, in window pixels. Works headless too.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn centered(ctx: &mut Ctx) {
    /// let label = "Paused";
    /// let size = ctx.measure_text(label, 32.0);
    /// let at = (ctx.window_size() - size) / 2.0;
    /// ctx.draw_text(label, at, 32.0, Color::WHITE);
    /// # }
    /// ```
    pub fn measure_text(&mut self, text: &str, size: f32) -> Vec2 {
        self.measure_text_ex(text, Font::DEFAULT, size)
    }

    /// [`measure_text`](Self::measure_text) for another font.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, title: Font) {
    /// let width = ctx.measure_text_ex("Colony", title, 40.0).x;
    /// # }
    /// ```
    pub fn measure_text_ex(&mut self, text: &str, font: Font, size: f32) -> Vec2 {
        let scale = self.scale_factor;
        self.text.layout(text, font, size * scale, false) / scale
    }

    /// Sets how `texture` is sampled. Use [`TextureFilter::Nearest`] for pixel
    /// art and tilesets; the default is smooth.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let tiles = ctx.load_texture("crab.png");
    /// ctx.set_texture_filter(&tiles, TextureFilter::Nearest);
    /// # }
    /// ```
    pub fn set_texture_filter(&mut self, texture: &Texture, filter: TextureFilter) {
        if let Some(r) = self.renderer.as_mut() {
            r.set_filter(texture.id, filter == TextureFilter::Nearest);
        }
    }

    /// Loads a TrueType or OpenType font from the `assets/` folder.
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a font.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let title = ctx.load_font("fonts/Title.ttf");
    /// # let _ = title;
    /// # }
    /// ```
    pub fn load_font(&mut self, path: &str) -> Font {
        let file = assets::resolve(path).unwrap_or_else(|e| panic!("crabbyengine: {e}"));
        let bytes =
            std::fs::read(&file).unwrap_or_else(|e| panic!("crabbyengine: could not read {}: {e}", file.display()));
        self.text
            .add_font(&bytes)
            .unwrap_or_else(|e| panic!("crabbyengine: {}: {e}", file.display()))
    }

    /// Loads an image from the `assets/` folder (PNG for now).
    ///
    /// Do this once in setup and keep the [`Texture`]; it's a cheap handle.
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a valid image.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Crabs", 800, 600).run(|ctx| {
    ///     let crab = ctx.load_texture("crab.png");
    ///     move |ctx: &mut Ctx| ctx.draw(&crab, vec2(400.0, 300.0))
    /// });
    /// ```
    pub fn load_texture(&mut self, path: &str) -> Texture {
        let file = assets::resolve(path).unwrap_or_else(|e| panic!("crabbyengine: {e}"));
        let image = assets::load_image_at(&file, path).unwrap_or_else(|e| panic!("crabbyengine: {e}"));
        let texture = self.create_texture(image.width, image.height, &image.rgba);
        self.watcher.watch(path, file, Asset::Texture(texture));
        texture
    }

    /// Loads resource `R` from a JSON file in `assets/`. With [hot
    /// reload](crate::Game::hot_reload) on, editing the file replaces the
    /// resource while the game runs: tune numbers without restarting. A file
    /// that doesn't parse keeps the old values and prints why.
    ///
    /// # Panics
    /// If the file is missing or isn't valid JSON for `R` (with the line and
    /// column).
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// #[derive(serde::Deserialize)]
    /// struct Rules {
    ///     speed: f32,
    ///     damage: f32,
    /// }
    ///
    /// Game::new("Tuning", 800, 600).run(|ctx| {
    ///     ctx.load_resource::<Rules>("rules.json"); // {"speed": 30, "damage": 2}
    ///     |ctx: &mut Ctx| {
    ///         let speed = ctx.world.resource::<Rules>().speed;
    ///         // ...
    ///     }
    /// });
    /// ```
    pub fn load_resource<R: Send + Sync + 'static + serde::de::DeserializeOwned>(&mut self, path: &str) {
        let loaded = self.load_data(
            path,
            Box::new(|world: &mut World, text: &str| {
                let value: R = serde_json::from_str(text).map_err(|e| e.to_string())?;
                world.insert_resource(value);
                Ok(())
            }),
        );
        loaded.unwrap_or_else(|e| panic!("crabbyengine: {e}"));
    }

    /// Loads a JSON file into reflected component `component` of `entity`
    /// (object fields merge), reloading it when the file changes: the C and
    /// C++ way to [`load_resource`](Self::load_resource).
    #[doc(hidden)]
    pub fn load_component_json(&mut self, path: &str, entity: Entity, component: &str) -> Result<(), String> {
        let component = component.to_string();
        self.load_data(
            path,
            Box::new(move |world: &mut World, text: &str| {
                let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
                world.set_json(entity, &component, value).map_err(|e| e.to_string())
            }),
        )
    }

    fn load_data(&mut self, path: &str, apply: DataLoader) -> Result<(), String> {
        let file = assets::resolve(path)?;
        let text = std::fs::read_to_string(&file).map_err(|e| format!("could not read {}: {e}", file.display()))?;
        apply(&mut self.world, &text).map_err(|e| format!("{path}: {e}"))?;
        self.data_loaders.push(apply);
        self.watcher.watch(path, file, Asset::Data(self.data_loaders.len() - 1));
        Ok(())
    }

    /// Names of the files (as you loaded them) that changed and were
    /// reloaded at the start of this frame, textures and data alike. Use it
    /// to redo work that depends on them.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.reloaded_files().iter().any(|f| f == "map.json") {
    ///     println!("map changed: rebuilding the flow fields");
    /// }
    /// # }
    /// ```
    pub fn reloaded_files(&self) -> &[String] {
        &self.watcher.reloaded_files
    }

    /// Textures whose files changed and were reloaded at the start of this
    /// frame (with [hot reload](crate::Game::hot_reload) on). They keep their
    /// handles, so sprites show the new image by themselves; use this to redo
    /// anything you computed from an image, such as a level read from its pixels.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, level: Texture) {
    /// if ctx.reloaded_textures().contains(&level) {
    ///     println!("level image changed: rebuilding the map");
    /// }
    /// # }
    /// ```
    pub fn reloaded_textures(&self) -> &[Texture] {
        &self.watcher.reloaded
    }

    /// Reloads textures and data files that changed (checked a few times a second).
    pub(crate) fn reload_changed_assets(&mut self) {
        self.watcher.reloaded.clear();
        self.watcher.reloaded_files.clear();
        if !self.watcher.on {
            return;
        }
        let mut changed = std::mem::take(&mut self.watcher.pending);
        self.watcher.changed(std::time::Instant::now(), &mut changed);
        for (asset, file, name) in changed.drain(..) {
            let texture = match asset {
                Asset::Texture(t) => t,
                Asset::Data(i) => {
                    let applied = std::fs::read_to_string(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|text| (self.data_loaders[i])(&mut self.world, &text));
                    match applied {
                        Ok(()) => {
                            eprintln!("crabbyengine: reloaded {name}");
                            self.watcher.reloaded_files.push(name);
                        }
                        Err(e) => eprintln!("crabbyengine: could not reload {name} (keeping the old values): {e}"),
                    }
                    continue;
                }
            };
            match assets::load_image_at(&file, &name) {
                Ok(image) if (image.width, image.height) == (texture.width(), texture.height()) => {
                    self.update_texture(&texture, &image.rgba);
                    self.watcher.reloaded.push(texture);
                    self.watcher.reloaded_files.push(name.clone());
                    eprintln!("crabbyengine: reloaded {name}");
                }
                Ok(image) => eprintln!(
                    "crabbyengine: {name} is now {} x {} (was {} x {}); restart the game to use the new size",
                    image.width,
                    image.height,
                    texture.width(),
                    texture.height()
                ),
                // Often a file caught halfway through saving; the next save reloads it.
                Err(e) => eprintln!("crabbyengine: could not reload {name}: {e}"),
            }
        }
        self.watcher.pending = changed;
    }

    /// Creates a texture from raw RGBA pixels (4 bytes per pixel, row by row,
    /// top row first). Handy for generated images.
    ///
    /// # Panics
    /// If `rgba.len()` isn't `width * height * 4`, or the size is 0 or larger
    /// than the GPU supports.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// // A 2x2 checkerboard.
    /// let w = [255, 255, 255, 255];
    /// let b = [0, 0, 0, 255];
    /// let pixels = [w, b, b, w].concat();
    /// let checker = ctx.create_texture(2, 2, &pixels);
    /// # }
    /// ```
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        assert!(
            width > 0 && height > 0,
            "crabbyengine: texture size must not be zero (got {width} x {height})"
        );
        let expected = width as usize * height as usize * 4;
        assert_eq!(
            rgba.len(),
            expected,
            "crabbyengine: a {width} x {height} texture needs {expected} bytes of RGBA, got {}",
            rgba.len()
        );
        if self.headless {
            // No GPU: a handle that remembers its size, so game logic still works.
            self.headless_textures += 1;
            return Texture::new(self.headless_textures, width, height);
        }
        let renderer = self.renderer();
        let max = renderer.max_texture_size();
        assert!(
            width <= max && height <= max,
            "crabbyengine: {width} x {height} is larger than this GPU's maximum texture size ({max})"
        );
        let id = renderer.create_texture(width, height, rgba);
        Texture::new(id, width, height)
    }

    /// Replaces every pixel of `texture` with `rgba` (same size as it was
    /// created with, 4 bytes per pixel). For images that change while the
    /// game runs: a minimap, a heat map, a simulated field. Cheap enough to
    /// do every frame for textures up to a few megapixels.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let mut pixels = vec![0u8; 64 * 64 * 4];
    /// let heat = ctx.create_texture(64, 64, &pixels);
    /// pixels[0..4].copy_from_slice(&[255, 0, 0, 255]); // top-left pixel red
    /// ctx.update_texture(&heat, &pixels);
    /// # }
    /// ```
    ///
    /// # Panics
    /// If `rgba` isn't `width * height * 4` bytes.
    #[track_caller]
    pub fn update_texture(&mut self, texture: &Texture, rgba: &[u8]) {
        let (w, h) = (texture.width(), texture.height());
        let expected = w as usize * h as usize * 4;
        assert_eq!(
            rgba.len(),
            expected,
            "crabbyengine: a {w} x {h} texture needs {expected} bytes of RGBA, got {}",
            rgba.len()
        );
        if self.headless {
            return;
        }
        self.renderer().update_texture(texture.id, w, h, rgba);
    }

    /// How many sprites have been drawn so far this frame.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// println!("{} sprites", ctx.sprite_count());
    /// # }
    /// ```
    pub fn sprite_count(&self) -> usize {
        self.draw_list.len() + self.ui_list.len()
    }

    /// What drawing the previous frame cost: sprites, draw calls, CPU time
    /// and GPU time. All zero (and `gpu_ms` is `None`) in headless runs.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn overlay(ctx: &mut Ctx) {
    ///     let stats = ctx.render_stats();
    ///     if let Some(gpu) = stats.gpu_ms {
    ///         println!("{} sprites: {:.2} ms CPU, {gpu:.2} ms GPU", stats.sprites, stats.cpu_ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(overlay).run_headless(1, |_| {});
    /// ```
    /// Saves the next frame as a PNG at `path`, once it has been drawn.
    /// Does nothing in headless runs (there is no frame). Errors are printed.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn photo_mode(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F12) {
    ///         ctx.save_screenshot("screenshot.png");
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(photo_mode).run_headless(1, |_| {});
    /// ```
    pub fn save_screenshot(&mut self, path: impl Into<PathBuf>) {
        if !self.headless {
            self.queue_screenshot(path.into(), 0, None);
        }
    }

    pub fn render_stats(&self) -> RenderStats {
        self.renderer.as_ref().map(|r| r.stats).unwrap_or_default()
    }

    // ---------------------------------------------------------------- timing

    /// Seconds since the previous frame. Multiply speeds by this for smooth,
    /// frame-rate independent movement.
    ///
    /// Clamped to at most 0.25 s so a hiccup never teleports objects.
    ///
    /// Inside a simulation system (added with
    /// [`Game::sim_system`](crate::Game::sim_system)) this is the fixed step
    /// instead, so simulation code just uses `dt()` too. In a system added
    /// with [`Game::sim_system_every`](crate::Game::sim_system_every) it is
    /// that many fixed steps.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, pos: &mut Vec2) {
    /// pos.x += 200.0 * ctx.dt(); // 200 pixels per second
    /// # }
    /// ```
    pub fn dt(&self) -> f32 {
        if self.in_sim {
            self.fixed_dt * self.sim_steps as f32
        } else {
            self.timing.dt
        }
    }

    /// How many fixed-rate updates to run this frame (often 0, 1 or 2).
    ///
    /// Use this for logic that must behave identically on every machine,
    /// such as physics or deterministic simulations. Each step lasts exactly
    /// [`fixed_dt`](Self::fixed_dt) seconds. The rate is set with
    /// [`Game::fixed_hz`](crate::Game::fixed_hz) (default 60).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, vel: &mut Vec2, pos: &mut Vec2) {
    /// for _ in 0..ctx.fixed_steps() {
    ///     vel.y += 980.0 * ctx.fixed_dt(); // gravity
    ///     *pos += *vel * ctx.fixed_dt();
    /// }
    /// # }
    /// ```
    pub fn fixed_steps(&self) -> u32 {
        self.timing.fixed_steps
    }

    /// Length of one fixed step in seconds (e.g. `1.0 / 60.0`).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// let steps_per_second = 1.0 / ctx.fixed_dt();
    /// # }
    /// ```
    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    /// How far (0.0 to 1.0) the current frame is between the last fixed step
    /// and the next. Use it to interpolate positions for perfectly smooth
    /// rendering of fixed-step objects.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, prev: Vec2, curr: Vec2) {
    /// let draw_pos = prev.lerp(curr, ctx.fixed_alpha());
    /// # }
    /// ```
    pub fn fixed_alpha(&self) -> f32 {
        self.timing.alpha
    }

    /// Seconds of game time since the game started (sum of all `dt`s).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// let blink_on = (ctx.time() * 2.0) as u64 % 2 == 0;
    /// # }
    /// ```
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Number of frames since the game started. The first frame is `1`.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// if ctx.frame() % 60 == 0 { println!("fps: {:.0}", ctx.fps()); }
    /// # }
    /// ```
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Smoothed frames per second.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// println!("{:.0} fps", ctx.fps());
    /// # }
    /// ```
    pub fn fps(&self) -> f32 {
        self.fps
    }

    // ---------------------------------------------------------------- keyboard

    /// `true` while `key` is held down.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, pos: &mut Vec2) {
    /// if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
    /// # }
    /// ```
    pub fn key_down(&self, key: Key) -> bool {
        self.input.key_down(key)
    }

    /// `true` only on the frame `key` went down. Good for jumps, menus, toggles.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, paused: &mut bool) {
    /// if ctx.key_pressed(Key::P) { *paused = !*paused; }
    /// # }
    /// ```
    pub fn key_pressed(&self, key: Key) -> bool {
        self.input.key_pressed(key)
    }

    /// `true` only on the frame `key` was let go.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, charging: &mut bool) {
    /// if ctx.key_released(Key::Space) { *charging = false; }
    /// # }
    /// ```
    pub fn key_released(&self, key: Key) -> bool {
        self.input.key_released(key)
    }

    // ---------------------------------------------------------------- mouse

    /// `true` while `button` is held down.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// let dragging = ctx.mouse_down(MouseButton::Left);
    /// # }
    /// ```
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.input.mouse_down(button)
    }

    /// `true` only on the frame `button` went down.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// if ctx.mouse_pressed(MouseButton::Left) { println!("click at {}", ctx.mouse_pos()); }
    /// # }
    /// ```
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.input.mouse_pressed(button)
    }

    /// `true` only on the frame `button` was let go.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// if ctx.mouse_released(MouseButton::Left) { println!("drop"); }
    /// # }
    /// ```
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.input.mouse_released(button)
    }

    /// Mouse position in window pixels. `(0, 0)` is the top-left corner,
    /// `y` grows downward.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// let target = ctx.mouse_pos();
    /// # }
    /// ```
    pub fn mouse_pos(&self) -> Vec2 {
        self.input.mouse_pos()
    }

    /// The world point under the mouse: [`mouse_pos`](Self::mouse_pos)
    /// through the [`camera`](Self::camera).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.mouse_pressed(MouseButton::Left) {
    ///     println!("clicked the world at {}", ctx.mouse_world());
    /// }
    /// # }
    /// ```
    pub fn mouse_world(&self) -> Vec2 {
        self.camera.screen_to_world(self.mouse_pos())
    }

    /// How far the mouse moved since the previous frame, in window pixels.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, camera: &mut Vec2) {
    /// if ctx.mouse_down(MouseButton::Right) { *camera -= ctx.mouse_delta(); }
    /// # }
    /// ```
    pub fn mouse_delta(&self) -> Vec2 {
        self.input.mouse_delta()
    }

    /// Scroll wheel movement this frame. `y > 0` means scrolled up / away from
    /// the user. Roughly one unit per wheel notch.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, zoom: &mut f32) {
    /// *zoom *= 1.1_f32.powf(ctx.mouse_wheel().y);
    /// # }
    /// ```
    pub fn mouse_wheel(&self) -> Vec2 {
        self.input.mouse_wheel()
    }

    // ---------------------------------------------------------------- window

    /// Window size in pixels (logical pixels, so it matches [`mouse_pos`](Self::mouse_pos)
    /// on high-DPI screens).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// let center = ctx.window_size() / 2.0;
    /// # }
    /// ```
    pub fn window_size(&self) -> Vec2 {
        self.window_size
    }

    /// Closes the window and ends the game after this frame.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx) {
    /// if ctx.key_pressed(Key::Escape) { ctx.quit(); }
    /// # }
    /// ```
    pub fn quit(&mut self) {
        self.quit_requested = true;
    }
}

/// A screenshot someone asked for: where to save it, and who to tell.
pub(crate) struct ScreenshotRequest {
    path: PathBuf,
    /// Scale down so neither side exceeds this (0: full size).
    max_size: u32,
    reply: Option<Sender<Result<Value, String>>>,
}

fn save_png(path: &Path, shot: &Screenshot, max_size: u32) -> Result<Value, String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    let image = image::RgbaImage::from_raw(shot.width, shot.height, shot.rgba.clone())
        .ok_or("the captured frame has the wrong size")?;
    let longest = shot.width.max(shot.height);
    let image = if max_size > 0 && longest > max_size {
        let (w, h) = (
            (u64::from(shot.width) * u64::from(max_size) / u64::from(longest)).max(1) as u32,
            (u64::from(shot.height) * u64::from(max_size) / u64::from(longest)).max(1) as u32,
        );
        image::imageops::resize(&image, w, h, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    image
        .save(path)
        .map_err(|e| format!("could not save the screenshot to {}: {e}", path.display()))?;
    Ok(serde_json::json!({ "path": path, "width": image.width(), "height": image.height() }))
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn push_sprite(
    draw_list: &mut DrawList,
    texture: &Texture,
    pos: Vec2,
    scale: Vec2,
    rotation: f32,
    z: f32,
    pivot: Vec2,
    color: [u8; 4],
    region_px: Option<Rect>,
) {
    let (size, uv) = region(texture, region_px);
    draw_list.push(
        texture.id,
        z,
        SpriteInstance::new(pos.into(), (size * scale).into(), pivot.into(), rotation, color).with_uv(uv),
    );
}

/// The drawn size (texture pixels) and UV of `region` of `texture`.
#[inline]
pub(crate) fn region(texture: &Texture, region: Option<Rect>) -> (Vec2, [u16; 4]) {
    match region {
        None => (texture.size(), FULL_UV),
        Some(r) => {
            let t = texture.size().max(Vec2::ONE);
            (r.size(), uv_region((r.min / t).into(), (r.size() / t).into()))
        }
    }
}

/// Which entities update this tick, from [`Ctx::turns`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turns {
    every: u32,
    now: u32,
    dt: f32,
}

impl Turns {
    /// `true` if `entity`'s turn is this tick.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, e: Entity) {
    /// if ctx.turns(8).includes(e) { /* expensive update */ }
    /// # }
    /// ```
    #[inline]
    pub fn includes(self, entity: crabby_ecs::Entity) -> bool {
        entity.index() % self.every == self.now
    }

    /// Time since an entity's previous turn: `every` times `ctx.dt()`.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let turn = ctx.turns(4);
    /// assert_eq!(turn.dt(), ctx.dt() * 4.0);
    /// # }
    /// ```
    pub fn dt(self) -> f32 {
        self.dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(dt: f32) -> FrameTiming {
        FrameTiming {
            dt,
            fixed_steps: 1,
            alpha: 0.25,
        }
    }

    #[test]
    fn begin_frame_updates_timing() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(800.0, 600.0));
        ctx.begin_frame(timing(0.5));
        ctx.begin_frame(timing(0.25));
        assert_eq!(ctx.frame(), 2);
        assert_eq!(ctx.dt(), 0.25);
        assert!((ctx.time() - 0.75).abs() < 1e-9);
        assert_eq!(ctx.fixed_steps(), 1);
        assert_eq!(ctx.fixed_alpha(), 0.25);
    }

    #[test]
    fn fps_is_seeded_then_smoothed() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        ctx.begin_frame(timing(1.0 / 60.0));
        assert!((ctx.fps() - 60.0).abs() < 0.01);
        ctx.begin_frame(timing(1.0 / 30.0));
        assert!(ctx.fps() > 30.0 && ctx.fps() < 60.0);
    }

    #[test]
    fn screen_projection_maps_corners() {
        let p = screen_projection(Vec2::new(800.0, 600.0));
        let clip = |x: f32, y: f32| p.project_point3(glam::vec3(x, y, 0.0)).truncate();
        assert!(clip(0.0, 0.0).abs_diff_eq(Vec2::new(-1.0, 1.0), 1e-6));
        assert!(clip(800.0, 600.0).abs_diff_eq(Vec2::new(1.0, -1.0), 1e-6));
        assert!(clip(400.0, 300.0).abs_diff_eq(Vec2::ZERO, 1e-6));
    }

    #[test]
    fn draw_without_gpu_queues_sprites() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(800.0, 600.0));
        let tex = Texture::new(0, 16, 8);
        ctx.draw(&tex, Vec2::new(10.0, 20.0));
        ctx.draw_ex(
            &tex,
            Vec2::ZERO,
            DrawParams {
                scale: Vec2::splat(2.0),
                ..Default::default()
            },
        );
        assert_eq!(ctx.sprite_count(), 2);
        assert_eq!(ctx.render_frame(), FrameStatus::Skipped);
        assert_eq!(ctx.sprite_count(), 0, "queue is cleared after each frame");
    }

    #[test]
    fn quit_sets_flag() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        assert!(!ctx.quit_requested());
        ctx.quit();
        assert!(ctx.quit_requested());
    }
}
