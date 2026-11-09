//! [`Ctx`]: everything game code talks to each frame.

use glam::{Mat4, Vec2};
use ponko_ecs::{Commands, Entity, World};
use std::collections::HashMap;

use crate::assets;
use crate::audio::Audio;
use crate::camera::Camera2D;
use crate::camera::Rect;
use crate::color::Color;
use crate::ecs::{Sprite, Transform2D};
use crate::events::Events;
use crate::hot_reload::Asset;
use crate::input::{Input, Key};
use crate::reflect::WorldReflect;
use crate::render::batch::SpriteInstance;
use crate::render::batch::{FULL_UV, uv_region};
use crate::render::{FrameStatus, Renderer, Screenshot};
use crate::spatial::SpatialGrid;
use crate::text::Font;
use crate::texture::Texture;
use crate::time::{FrameTiming, Time};
use crate::ui::UiState;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Maps window pixels ((0, 0) top-left, y down) to wgpu clip space
/// (x right, y up, both -1..1).
pub(crate) fn screen_projection(size: Vec2) -> Mat4 {
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
/// Which loader a cached file went through.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AssetKind {
    Texture,
    Font,
    Sound,
}

/// Applies a data file's text to the world.
type DataLoader = Box<dyn Fn(&mut World, &str) -> Result<(), String>>;

/// What every system gets: the world, commands, events, input, drawing,
/// the clock, sound, the camera, physics and the network session. Systems are
/// `fn(&mut Ctx)`.
///
/// ```
/// use ponko::*;
/// fn spin(ctx: &mut Ctx) {
///     let dt = ctx.dt();
///     for t in ctx.world.query::<&mut Transform2D>().iter() {
///         t.rotation += dt;
///     }
/// }
/// Game::new("Spin", 800, 600).sim_system(spin).run_headless(10, |_| {});
/// ```
pub struct Ctx {
    /// All entities, components and resources.
    ///
    /// ```
    /// # use ponko::*;
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
    /// When they apply (systems run one at a time, each with all of `Ctx`):
    ///
    /// - Right after the recording system returns, before the next system
    ///   starts. Every system is a barrier, within a [`Phase`](crate::Phase)
    ///   and across phases; the recording system itself never sees them.
    /// - In recorded order. A [`Commands::add`](crate::Commands::add)
    ///   closure sees the commands before it; an `insert` after a `despawn`
    ///   of the same entity does nothing.
    /// - Commands from `setup` apply when it returns, before the first tick.
    /// - Engine steps are systems too: commands from
    ///   [`Phase::BeforePhysics`](crate::Phase::BeforePhysics) are in the
    ///   world when the physics step runs, and frame systems see the commands
    ///   of every tick in their frame.
    /// - A system skipped by [`sim_system_every`](crate::Game::sim_system_every)
    ///   doesn't run, so it records nothing.
    ///
    /// Changes made directly through [`world`](Self::world) are immediate.
    /// C and C++ systems follow the same rules. The guide's "Systems and the
    /// frame" chapter has the full contract with events.
    ///
    /// ```
    /// # use ponko::*;
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
    /// The physics world: ray casts, overlap queries, impulses, gravity.
    /// Simulated once enabled with [`Game::physics`](crate::Game::physics)
    /// (feature `physics`).
    #[cfg(feature = "physics")]
    pub physics: crate::Physics,
    /// The 3D physics world: ray and shape casts, overlaps, impulses,
    /// collision meshes, gravity. Simulated once enabled with
    /// [`Game::physics3d`](crate::Game::physics3d) (feature `physics3d`).
    #[cfg(feature = "physics3d")]
    pub physics3d: crate::Physics3D,
    /// Lockstep networking: this player's input out, everyone's in. Offline
    /// (one player) unless the game was given a session with
    /// [`Game::lockstep`](crate::Game::lockstep).
    pub net: crate::Net,
    /// Messages between systems: [`Events::send`] and [`Events::read`].
    /// A separate field, so you can read events while changing the world.
    ///
    /// ```
    /// # use ponko::*;
    /// struct Scored(u32);
    /// struct Score(u32);
    ///
    /// fn count_score(ctx: &mut Ctx) {
    ///     for s in ctx.events.read::<Scored>() {
    ///         ctx.world.resource_mut::<Score>().0 += s.0;
    ///     }
    /// }
    /// ```
    pub events: Events,
    /// Seed for `random()`.
    pub(crate) seed: u64,
    /// Index of the running system, so systems get different random streams.
    pub(crate) system_index: usize,
    /// No window or GPU (`Game::run_headless`): textures are size-only handles.
    pub(crate) headless: bool,
    /// Where the control endpoint listens, if it's on.
    pub(crate) control_addr: Option<std::net::SocketAddr>,
    /// Asset hot reload: which files textures came from.
    pub(crate) watcher: crate::hot_reload::SharedWatcher,
    /// Textures reloaded at the start of this frame.
    pub(crate) reloaded_textures: Vec<Texture>,
    /// Names of every file reloaded at the start of this frame.
    pub(crate) reloaded_files: Vec<String>,
    /// Scratch for the watcher's changes, kept so checks don't allocate.
    reload_pending: Vec<(Asset, PathBuf, String)>,
    /// How to apply each loaded data file's text (`load_resource`).
    data_loaders: Vec<DataLoader>,
    /// Parsed prefab files, by name (`spawn_prefab`).
    pub(crate) prefabs: HashMap<String, serde_json::Value>,
    /// Asset sources added with `add_asset_source`, asked before `assets/`.
    pub(crate) asset_sources: assets::SharedSources,
    /// Assets loaded under residency budgets (`resident_mesh`), by kind.
    pub(crate) residency: crate::residency::Residency,
    /// Keyboard, mouse, gamepads and named actions: see [`Input`].
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.input.mouse_pressed(MouseButton::Left) { /* ... */ }
    /// # }
    /// ```
    pub input: Input,
    /// The frame and simulation clocks: see [`Time`].
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let seconds = ctx.time.elapsed();
    /// ctx.time.set_speed(2.0);
    /// # }
    /// ```
    pub time: Time,
    window_size: Vec2,
    quit_requested: bool,
    /// Which part of the world the window shows. Move it, zoom it, or make
    /// it follow something; see [`Camera2D`].
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.camera.zoom = 2.0;
    /// ctx.camera.look_at(vec2(1000.0, 1000.0));
    /// # }
    /// ```
    pub camera: Camera2D,
    /// The 3D camera (feature `3d`): position, rotation, field of view.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.camera3d.position = vec3(0.0, 5.0, 10.0);
    /// ctx.camera3d.look_at(Vec3::ZERO);
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub camera3d: crate::Camera3D,
    /// The 3D world's sun and ambient light (feature `3d`).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.lighting.sun_intensity = 1.0; // overcast
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub lighting: crate::Lighting,
    /// Global settings for [`LodGroup`](crate::LodGroup)s (feature `3d`):
    /// the bias that a graphics quality setting drives, and crossfades on
    /// or off.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.lod.bias = 1.5; // keep detail up to 50% farther
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub lod: crate::LodSettings,
    /// What happens to the lit 3D frame on its way to the screen (feature
    /// `3d`): exposure, bloom, color grading, tone mapping.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.post.bloom = 0.2;
    /// ctx.post.vignette = 0.3;
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub post: crate::Post,
    /// Screen effects run over the lit 3D frame each frame, in order,
    /// before bloom and tone mapping (feature `3d`). Each reads what the
    /// one before it left.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let haze = ctx.gfx.load_effect("effects/haze.wgsl");
    /// ctx.effects.push(ScreenEffect::new(haze));
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub effects: Vec<crate::ScreenEffect>,
    /// Effects run over the lit 3D frame each frame, in order, that write
    /// into render targets instead of the frame (feature `3d`): after the
    /// transparent surfaces, before [`effects`](Self::effects). See
    /// [`TargetEffect`](crate::TargetEffect) for the ordering and resource
    /// rules.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, glow: Effect) {
    /// let mask = ctx.gfx.create_render_target(256, 256);
    /// ctx.target_effects.push(TargetEffect::new(glow, mask));
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub target_effects: Vec<crate::TargetEffect>,
    /// Cameras drawing the 3D world into render targets each frame, before
    /// the main view (feature `3d`): mirrors, reflections, security
    /// cameras. See [`TargetCamera`](crate::TargetCamera).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let lake = ctx.gfx.create_render_target(960, 540);
    /// ctx.target_cameras.push(TargetCamera::reflection(ctx.camera3d, lake, 0.0));
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub target_cameras: Vec<crate::TargetCamera>,
    /// Compute passes run on the GPU each frame, in order, before anything
    /// is drawn (feature `3d`): each updates a render target texel by
    /// texel. See [`ComputePass`](crate::ComputePass).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let ripples = ctx.gfx.load_compute("compute/ripples.wgsl");
    /// let water = ctx.gfx.create_render_target(256, 256);
    /// ctx.compute_passes.push(ComputePass::new(ripples, water));
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub compute_passes: Vec<crate::ComputePass>,
    /// The sky behind the 3D world and the fog in front of it (feature
    /// `3d`): a physical sky lit by the sun, by default.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.lighting.sun_direction = Sky::sun_direction_at(7.0); // morning
    /// ctx.sky.fog_density = 0.01;
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub sky: crate::Sky,
    /// Engine-side UI scratch: the spare tree, typed text, world panels.
    pub(crate) ui_state: UiState,
    /// The screen's UI: the retained widget tree and its screen stack
    /// (processed before the frame's systems), who has the pointer and the
    /// keyboard, and the immediate widgets (`ctx.ui.immediate`). See
    /// [`Ui`](crate::Ui).
    ///
    /// ```
    /// use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let root = ctx.ui.add(UiWidget::Panel, ui_layout::Style::default());
    /// ctx.ui.push_screen(root, false);
    /// # }
    /// ```
    pub ui: crate::Ui,
    /// Drawing and everything on the GPU: sprites, text, shapes, textures,
    /// fonts, and with the `3d` feature meshes, materials, shaders and the
    /// rest. See [`Graphics`](crate::Graphics).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, crab: Texture) {
    /// ctx.gfx.draw_circle(vec2(100.0, 100.0), 20.0, Color::WHITE);
    /// ctx.gfx.draw(&crab, vec2(400.0, 300.0));
    /// # }
    /// ```
    pub gfx: crate::Graphics,
    /// Sounds, playing handles, buses and the listener: see
    /// [`Audio`](crate::Audio).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let hit = ctx.audio.load("hit.ogg");
    /// ctx.audio.play(&hit);
    /// # }
    /// ```
    pub audio: Audio,
    /// Where the time goes: system timings, frame and render statistics,
    /// GPU passes, the HUD, traces and the report. See [`Perf`](crate::Perf).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.perf.set_hud(true);
    /// let draw_calls = ctx.perf.render_stats().draw_calls;
    /// # }
    /// ```
    pub perf: crate::Perf,
}

impl Ctx {
    /// `fixed_dt` in seconds, as f64 so `sim_time()` stays exact over long runs.
    pub(crate) fn new(fixed_dt: f64, window_size: Vec2) -> Self {
        // Every service that loads files shares these two.
        let asset_sources = assets::SharedSources::default();
        let watcher = crate::hot_reload::SharedWatcher::default();
        let mut world = World::new();
        crate::reflect::WorldReflect::reflect::<crate::Transform2D>(&mut world);
        crate::reflect::describe_transform2d(&mut world);
        crate::reflect::WorldReflect::reflect::<crate::Attached>(&mut world);
        #[cfg(feature = "3d")]
        crate::reflect::WorldReflect::reflect::<crate::Transform3D>(&mut world);
        #[cfg(feature = "3d")]
        crate::reflect::reflect_model(&mut world);
        #[cfg(feature = "3d")]
        {
            crate::reflect::WorldReflect::reflect::<crate::PointLight3D>(&mut world);
            crate::reflect::WorldReflect::reflect::<crate::SpotLight3D>(&mut world);
            crate::reflect::WorldReflect::reflect::<crate::Decal3D>(&mut world);
            crate::reflect::WorldReflect::reflect::<crate::ReceivesDecals>(&mut world);
            crate::reflect::WorldReflect::reflect::<crate::LodHysteresis>(&mut world);
        }
        crate::reflect::reflect_sprite(&mut world);
        // The file names behind sprites and models: the world saves by them,
        // the graphics service fills them in as files load.
        let gfx = crate::Graphics::new(
            asset_sources.clone(),
            watcher.clone(),
            world.resource::<crate::reflect::TextureNames>().clone(),
            #[cfg(feature = "3d")]
            world.resource::<crate::reflect::MeshNames>().clone(),
        );
        #[allow(unused_mut, clippy::let_and_return)]
        let mut ctx = Self {
            audio: Audio {
                asset_sources: asset_sources.clone(),
                watcher: watcher.clone(),
                ..Default::default()
            },
            gfx,
            world,
            commands: Commands::new(),
            spatial: SpatialGrid::default(),
            #[cfg(feature = "physics")]
            physics: crate::Physics::default(),
            #[cfg(feature = "physics3d")]
            physics3d: crate::Physics3D::default(),
            net: crate::Net::default(),
            events: Events::default(),
            seed: crate::random::DEFAULT_SEED,
            system_index: 0,
            headless: false,
            control_addr: None,
            watcher: watcher.clone(),
            reloaded_textures: Vec::new(),
            reloaded_files: Vec::new(),
            reload_pending: Vec::new(),
            data_loaders: Vec::new(),
            prefabs: HashMap::new(),
            asset_sources: asset_sources.clone(),
            residency: Default::default(),
            input: Input::new(),
            time: Time::new(fixed_dt),
            window_size,
            quit_requested: false,
            camera: Camera2D::new(window_size),
            #[cfg(feature = "3d")]
            camera3d: crate::Camera3D::new(window_size),
            #[cfg(feature = "3d")]
            lighting: crate::Lighting::default(),
            #[cfg(feature = "3d")]
            lod: crate::LodSettings::default(),
            #[cfg(feature = "3d")]
            post: crate::Post::default(),
            #[cfg(feature = "3d")]
            effects: Vec::new(),
            #[cfg(feature = "3d")]
            target_effects: Vec::new(),
            #[cfg(feature = "3d")]
            target_cameras: Vec::new(),
            #[cfg(feature = "3d")]
            compute_passes: Vec::new(),
            #[cfg(feature = "3d")]
            sky: crate::Sky::default(),
            ui_state: UiState::default(),
            ui: crate::Ui::default(),
            perf: crate::Perf::new(),
        };
        #[cfg(feature = "3d")]
        ctx.sync_audio_camera();
        ctx
    }

    /// Engine side: draw everything queued this frame, then reset the queue.
    pub(crate) fn render_frame(&mut self) -> FrameStatus {
        self.draw_retained_ui();
        let capture = !self.gfx.screenshots.is_empty();
        #[cfg(feature = "3d")]
        self.draw_ui_panels();
        #[cfg(feature = "3d")]
        self.prepare_3d();
        #[cfg(feature = "3d")]
        let panels = self.ui_state.panels.frame_draws();
        #[cfg(not(feature = "3d"))]
        let panels: &mut [crate::render::batch::PanelDraw] = &mut [];
        let status = match self.gfx.renderer.as_mut() {
            Some(renderer) => renderer.render(
                &mut self.gfx.draw_list,
                &mut self.gfx.ui_list,
                panels,
                self.gfx.clear_color,
                self.camera.view_proj(),
                screen_projection(self.window_size),
                capture,
            ),
            None => FrameStatus::Skipped,
        };
        #[cfg(feature = "3d")]
        // A frame that wasn't drawn hands its probe captures to the next one.
        self.gfx
            .assets3d
            .probes
            .settle(!matches!(status, FrameStatus::Occluded | FrameStatus::Skipped));
        self.perf.last_render_stats = self.gfx.renderer.as_ref().map(Renderer::stats).unwrap_or_default();
        self.perf.last_render_detail = self.gfx.renderer.as_ref().map(Renderer::detail);
        if let Some(renderer) = &self.gfx.renderer {
            self.perf.gpu_passes.clear();
            self.perf.gpu_passes.extend_from_slice(renderer.gpu_passes());
            self.perf.display_wait_ms = renderer.display_wait_ms();
        }
        self.gfx.draw_list.clear();
        self.gfx.ui_list.clear();
        if let Some(shot) = self.gfx.renderer.as_mut().and_then(Renderer::take_screenshot) {
            for req in self.gfx.screenshots.drain(..) {
                let result = save_png(&req.path, &shot, req.max_size);
                match req.reply {
                    Some(reply) => {
                        let _ = reply.send(result);
                    }
                    None => {
                        if let Err(e) = result {
                            eprintln!("ponko: {e}");
                        }
                    }
                }
            }
        }
        status
    }

    /// Engine side: start a new frame with fresh timing.
    pub(crate) fn begin_frame(&mut self, timing: FrameTiming) {
        self.time.begin_frame(timing);
        self.events.begin_frame(self.time.frame(), self.time.sim_tick());
        self.gfx.frame = self.time.frame();
        self.gfx.text.begin_frame();
        self.ui.immediate.begin_frame();
    }

    pub(crate) fn set_scale_factor(&mut self, scale: f32) {
        self.gfx.scale_factor = scale.max(0.25);
    }

    pub(crate) fn set_window_size(&mut self, size: Vec2) {
        self.window_size = size;
        self.camera.set_viewport(size);
        #[cfg(feature = "3d")]
        self.camera3d.set_viewport(size);
    }

    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// A snapshot of the ECS for tools and tests: every archetype with its
    /// components, entity count and storage bytes; every component type
    /// with its size and how many entities have it; and every system in run
    /// order with its [`Phase`](crate::Phase), interval and last timings.
    ///
    /// Built only when called (nothing is collected otherwise), so it costs
    /// nothing in a game that never asks. It allocates, so don't call it
    /// every frame. The control endpoint serves it as `ecs.diagnostics`.
    ///
    /// ```
    /// use ponko::*;
    /// struct Pos(f32);
    /// fn report(ctx: &mut Ctx) {
    ///     let d = ctx.ecs_diagnostics();
    ///     assert_eq!(d.entities, 2);
    ///     assert_eq!(d.systems[0].phase, "AfterPhysics");
    ///     println!("{}", serde_json::to_string_pretty(&d).unwrap());
    /// }
    /// Game::new("t", 1, 1).sim_system(report).run_headless(1, |ctx| {
    ///     ctx.world.spawn_batch([(Pos(0.0),), (Pos(1.0),)]);
    /// });
    /// ```
    pub fn ecs_diagnostics(&self) -> crate::EcsDiagnostics {
        crate::diagnostics::snapshot(self)
    }

    /// The address of this game's control endpoint, or `None` if it's off.
    /// Tools find it in the instance registry; this is for tests and logs.
    ///
    /// ```
    /// # use ponko::*;
    /// let mut addr = None;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| addr = ctx.control_addr());
    /// assert_eq!(addr, None); // off unless enabled
    /// ```
    pub fn control_addr(&self) -> Option<std::net::SocketAddr> {
        self.control_addr
    }

    /// Engine side: speed to feed the clock this frame (0 while paused).
    pub(crate) fn effective_sim_speed(&self) -> f64 {
        self.time.effective_sim_speed()
    }

    /// Engine side: uses up one requested single step; `false` if none was pending.
    pub(crate) fn take_one_step(&mut self) -> bool {
        self.time.take_one_step()
    }

    pub(crate) fn take_pending_steps(&mut self) -> u32 {
        self.time.take_pending_steps()
    }

    pub(crate) fn advance_tick(&mut self) {
        self.time.advance_tick();
        self.events.set_tick(self.time.sim_tick());
    }

    pub(crate) fn set_sim_steps(&mut self, steps: u32) {
        self.time.set_sim_steps(steps);
    }

    pub(crate) fn set_in_sim(&mut self, in_sim: bool) {
        self.time.set_in_sim(in_sim);
    }

    /// Random numbers for this system and tick: `ctx.random().of(entity)`
    /// for an entity's own stream (safe in `par_for_each`), or
    /// `.stream(n)` for anything else. A simulation run twice with the same
    /// [seed](crate::Game::seed) gives the same results.
    ///
    /// ```
    /// use ponko::*;
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
        crate::Random::new(self.seed, self.time.sim_tick(), self.system_index)
    }

    /// Engine side: apply commands recorded by the system that just ran.
    pub(crate) fn apply_commands(&mut self) {
        if !self.commands.is_empty() {
            self.world.apply(&mut self.commands);
        }
        self.perf.entities = self.world.len();
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
        let draw_list = &mut self.gfx.draw_list;
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

    /// Adds a place to load named assets from. Added sources are asked in
    /// the order they were added, all before the `assets/` folder.
    ///
    /// Every load by name goes through it: [`load_texture`](Self::load_texture),
    /// [`load_font`](crate::Graphics::load_font), [`load_resource`](Self::load_resource),
    /// [`load_scene`](Self::load_scene), [`spawn_prefab`](Self::spawn_prefab)
    /// (and the textures their scenes name), [`load_sound`](Self::load_sound),
    /// [`load_asset_with`](Self::load_asset_with) and, with the `3d` feature,
    /// meshes, skeletons and animation clips (and the files a glTF refers to),
    /// shaders, screen effects and compute passes.
    ///
    /// Not covered yet: [`load_stream`](crate::Audio::load_stream) (music streamed
    /// from a file), texture arrays and reflection probe images still come
    /// only from `assets/`.
    ///
    /// Assets from sources are never hot reloaded: only files found in
    /// `assets/` are watched. See [`AssetSource`](crate::AssetSource) for
    /// names, caching and hot reload.
    ///
    /// Add sources in setup, before loading what they hold: assets loaded
    /// earlier stay cached where they were found.
    ///
    /// ```
    /// # use ponko::*;
    /// #[derive(serde::Deserialize)]
    /// struct Rules {
    ///     speed: f32,
    /// }
    ///
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     ctx.add_asset_source(MemorySource::new("built-in").with("rules.json", r#"{"speed": 30}"#));
    ///     ctx.load_resource::<Rules>("rules.json");
    ///     assert_eq!(ctx.world.resource::<Rules>().speed, 30.0);
    /// });
    /// ```
    ///
    /// # Panics
    /// If the source's id is empty or a source with that id was already added.
    pub fn add_asset_source(&mut self, source: impl crate::AssetSource) {
        self.asset_sources.add(Box::new(source));
    }

    /// Reads asset `name` (from the added sources, then `assets/`) and
    /// decodes it with `decode` on the CPU: your own asset type, with no
    /// registration. Nothing is cached and nothing is watched: it reads and
    /// decodes each call, so keep the result.
    ///
    /// `decode` gets the bytes and returns your value, or an error message
    /// that is reported with the asset's name and source.
    ///
    /// ```
    /// # use ponko::*;
    /// /// A level in a tiny made-up format: one row of tiles per line.
    /// struct Tiles(Vec<Vec<u8>>);
    ///
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     ctx.add_asset_source(MemorySource::new("built-in").with("levels/one.tiles", "##.\n#..\n"));
    ///     let level = ctx
    ///         .load_asset_with("levels/one.tiles", |bytes| {
    ///             let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    ///             Ok(Tiles(text.lines().map(|row| row.bytes().collect()).collect()))
    ///         })
    ///         .unwrap();
    ///     assert_eq!(level.0.len(), 2);
    ///     assert_eq!(level.0[0], b"##.");
    /// });
    /// ```
    ///
    /// # Errors
    /// If no source has the asset (the message says where it looked), reading
    /// it fails, or `decode` returns an error.
    pub fn load_asset_with<T>(&self, name: &str, decode: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        let found = self.asset_sources.find(name)?;
        let origin = found.origin.clone();
        let bytes = found.into_bytes()?;
        decode(&bytes).map_err(|e| format!("{} could not be decoded: {e}", origin.describe(name)))
    }

    /// Reads asset `name` as UTF-8 text from the first source that has it.
    pub(crate) fn read_asset_text(&self, name: &str) -> Result<(assets::Origin, String), String> {
        let found = self.asset_sources.find(name)?;
        let origin = found.origin.clone();
        Ok((origin, found.into_text(name)?))
    }

    /// Loads the textures of sprites and meshes of models that `json` (a
    /// scene, a prefab, or components) names, so they can be spawned.
    pub(crate) fn load_scene_assets(&mut self, json: &serde_json::Value) -> Result<(), String> {
        let mut names = Vec::new();
        crate::reflect::asset_names(json, "Sprite", "texture", &mut names);
        for name in names.drain(..) {
            self.gfx.try_load_texture(&name)?;
        }
        #[cfg(feature = "3d")]
        {
            crate::reflect::asset_names(json, "Model", "mesh", &mut names);
            crate::reflect::asset_names(json, "ShadowMesh", "mesh", &mut names);
            crate::reflect::lod_mesh_names(json, &mut names);
            names.sort_unstable();
            names.dedup();
            for name in names {
                self.gfx.try_load_mesh(&name)?;
            }
        }
        #[cfg(feature = "physics3d")]
        self.load_collider_files(json)?;
        Ok(())
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
    /// # use ponko::*;
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
        loaded.unwrap_or_else(|e| panic!("ponko: {e}"));
    }

    /// Loads a JSON file into the reflected component `T` of `entity` (object
    /// fields merge), reloading it when the file changes. `T` must be
    /// registered with `World::reflect`; the name the file is applied under
    /// comes from the type, so it can't drift from it.
    /// ```
    /// # use ponko::*;
    /// # #[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
    /// # struct Stats { hp: f32 }
    /// # fn f(ctx: &mut Ctx, e: Entity) -> Result<(), String> {
    /// ctx.load_component_json::<Stats>("stats.json", e)?;
    /// # Ok(()) }
    /// ```
    pub fn load_component_json<T: 'static>(&mut self, path: &str, entity: Entity) -> Result<(), String> {
        self.load_component_json_named(path, entity, &crate::reflect::short_name(std::any::type_name::<T>()))
    }

    /// Loads a JSON file into the reflected component `component` of `entity`
    /// by name: the C and C++ way, and for components registered by name.
    /// Rust code that has the type uses
    /// [`load_component_json`](Self::load_component_json).
    #[doc(hidden)]
    pub fn load_component_json_named(&mut self, path: &str, entity: Entity, component: &str) -> Result<(), String> {
        let component = component.to_string();
        self.load_data(
            path,
            Box::new(move |world: &mut World, text: &str| {
                let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
                world.set_json(entity, &component, value).map_err(|e| e.to_string())
            }),
        )
    }

    /// Loads a JSON file into the reflected resource `T` (object fields
    /// merge; it's added if missing), reloading it when the file changes.
    /// `T` must be registered with `World::reflect_resource`; the name comes
    /// from the type. Unlike [`load_resource`](Self::load_resource) this
    /// merges into the existing value instead of replacing it.
    /// ```
    /// # use ponko::*;
    /// # #[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
    /// # struct Rules { speed: f32 }
    /// # fn f(ctx: &mut Ctx) -> Result<(), String> {
    /// ctx.load_resource_json::<Rules>("rules.json")?;
    /// # Ok(()) }
    /// ```
    pub fn load_resource_json<T: 'static>(&mut self, path: &str) -> Result<(), String> {
        self.load_resource_json_named(path, &crate::reflect::short_name(std::any::type_name::<T>()))
    }

    /// Loads a JSON file into the reflected resource `name` by name: the C
    /// and C++ way. Rust code that has the type uses
    /// [`load_resource_json`](Self::load_resource_json).
    #[doc(hidden)]
    pub fn load_resource_json_named(&mut self, path: &str, name: &str) -> Result<(), String> {
        let name = name.to_string();
        self.load_data(
            path,
            Box::new(move |world: &mut World, text: &str| {
                let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
                world.set_resource_json(&name, value).map_err(|e| e.to_string())
            }),
        )
    }

    fn load_data(&mut self, path: &str, apply: DataLoader) -> Result<(), String> {
        let (origin, text) = self.read_asset_text(path)?;
        apply(&mut self.world, &text).map_err(|e| format!("{path}: {e}"))?;
        self.data_loaders.push(apply);
        if let assets::Origin::File(file) = origin {
            self.watcher.watch(path, file, Asset::Data(self.data_loaders.len() - 1));
        }
        Ok(())
    }

    /// Names of the files (as you loaded them) that changed and were
    /// reloaded at the start of this frame, textures and data alike. Use it
    /// to redo work that depends on them.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.reloaded_files().iter().any(|f| f == "map.json") {
    ///     println!("map changed: rebuilding the flow fields");
    /// }
    /// # }
    /// ```
    pub fn reloaded_files(&self) -> &[String] {
        &self.reloaded_files
    }

    /// Textures whose files changed and were reloaded at the start of this
    /// frame (with [hot reload](crate::Game::hot_reload) on). They keep their
    /// handles, so sprites show the new image by themselves; use this to redo
    /// anything you computed from an image, such as a level read from its pixels.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, level: Texture) {
    /// if ctx.reloaded_textures().contains(&level) {
    ///     println!("level image changed: rebuilding the map");
    /// }
    /// # }
    /// ```
    pub fn reloaded_textures(&self) -> &[Texture] {
        &self.reloaded_textures
    }

    fn report_reload(&mut self, name: String, applied: Result<(), String>) {
        match applied {
            Ok(()) => {
                eprintln!("ponko: reloaded {name}");
                self.reloaded_files.push(name);
            }
            Err(e) => eprintln!("ponko: could not reload {name} (keeping the old one): {e}"),
        }
    }

    /// Reloads textures, fonts, sounds and data files that changed (checked a few times a second).
    pub(crate) fn reload_changed_assets(&mut self) {
        self.reloaded_textures.clear();
        self.reloaded_files.clear();
        if !self.watcher.on() {
            return;
        }
        let mut changed = std::mem::take(&mut self.reload_pending);
        self.watcher.changed(web_time::Instant::now(), &mut changed);
        for (asset, file, name) in changed.drain(..) {
            let texture = match asset {
                Asset::Texture(t) => t,
                Asset::Font(id) => {
                    let applied = assets::read(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| self.gfx.text.replace_font(Font(id), &bytes));
                    self.report_reload(name, applied);
                    continue;
                }
                Asset::Sound(id) => {
                    let applied = assets::read(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| self.audio.backend.replace(id, bytes));
                    self.report_reload(name, applied);
                    continue;
                }
                Asset::Data(i) => {
                    let applied =
                        assets::read_to_string(&file).and_then(|text| (self.data_loaders[i])(&mut self.world, &text));
                    self.report_reload(name, applied);
                    continue;
                }
                #[cfg(feature = "3d")]
                Asset::Shader(id) => {
                    let applied =
                        assets::read_to_string(&file).and_then(|text| self.gfx.reload_shader(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
                #[cfg(feature = "3d")]
                Asset::Effect(id) => {
                    let applied =
                        assets::read_to_string(&file).and_then(|text| self.gfx.reload_effect(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
                #[cfg(feature = "3d")]
                Asset::Compute(id) => {
                    let applied =
                        assets::read_to_string(&file).and_then(|text| self.gfx.reload_compute(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
            };
            match assets::load_image_at(&file, &name) {
                Ok(image) if (image.width, image.height) == (texture.width(), texture.height()) => {
                    self.gfx.update_texture(&texture, &image.rgba);
                    self.reloaded_textures.push(texture);
                    self.reloaded_files.push(name.clone());
                    eprintln!("ponko: reloaded {name}");
                }
                Ok(image) => eprintln!(
                    "ponko: {name} is now {} x {} (was {} x {}); restart the game to use the new size",
                    image.width,
                    image.height,
                    texture.width(),
                    texture.height()
                ),
                // Often a file caught halfway through saving; the next save reloads it.
                Err(e) => eprintln!("ponko: could not reload {name}: {e}"),
            }
        }
        self.reload_pending = changed;
    }

    /// Hands one frame's timings to the profiler. `true` when a benchmark
    /// run is done and the game should quit.
    pub(crate) fn record_frame(&mut self, sample: crate::profiler::FrameSample) -> bool {
        let gpu_frame = self.gfx.renderer.as_ref().and_then(Renderer::gpu_frame);
        self.perf.entities = self.world.len();
        self.perf.record_frame(sample, gpu_frame)
    }

    /// Starts or stops the deeper profiling to match what's wanted.
    pub(crate) fn sync_profiling(&mut self) {
        self.perf.sync_profiling();
    }

    /// Ends a frame (or headless tick) that took `ms` for the report.
    pub(crate) fn end_perf_frame(&mut self, ms: f32) {
        self.perf.entities = self.world.len();
        self.perf.end_frame(ms);
    }

    /// Engine side: hands the 3D camera to the audio service, whose default
    /// listener it is. Runs before every system and before the frame's
    /// positional sound update.
    /// Engine side, before each system: gives the services the state of
    /// their neighbours they answer from (the camera for the 3D listener,
    /// the mouse for the UI's pointer), so they need no borrow of `Ctx`.
    #[inline]
    pub(crate) fn sync_services(&mut self) {
        #[cfg(feature = "3d")]
        self.sync_audio_camera();
        self.ui.set_pointer(self.input.mouse_pos());
    }

    #[cfg(feature = "3d")]
    #[inline]
    pub(crate) fn sync_audio_camera(&mut self) {
        self.audio.backend.spatial.camera = crate::AudioListener3D {
            position: self.camera3d.position,
            rotation: self.camera3d.rotation,
        };
    }

    /// Draws `texture` centered at `pos`, in world units (window pixels until
    /// you move or zoom [`camera`](Self::camera)). A shortcut for
    /// [`ctx.gfx.draw`](crate::Graphics::draw), which has the details;
    /// [`draw_ex`](crate::Graphics::draw_ex) scales, rotates and tints.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, crab: Texture) {
    /// ctx.draw(&crab, vec2(400.0, 300.0));
    /// # }
    /// ```
    pub fn draw(&mut self, texture: &Texture, pos: Vec2) {
        self.gfx.draw(texture, pos);
    }

    /// Draws `text` with its top-left corner at `pos`, in window pixels (on
    /// top of the world, ignoring the camera). `size` is the line height in
    /// pixels. A shortcut for [`ctx.gfx.draw_text`](crate::Graphics::draw_text).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn hud(ctx: &mut Ctx) {
    /// let fps = ctx.time.fps();
    /// ctx.draw_text(&format!("{fps:.0} FPS"), vec2(10.0, 10.0), 18.0, Color::WHITE);
    /// # }
    /// ```
    pub fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        self.gfx.draw_text(text, pos, size, color);
    }

    /// Loads an image from an [added asset source](crate::Assets::add_source)
    /// or the `assets/` folder. Do this once in setup and keep the
    /// [`Texture`]; it's a cheap handle. A shortcut for
    /// [`ctx.gfx.load_texture`](crate::Graphics::load_texture), which has
    /// the details.
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a valid image.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Crabs", 800, 600).run(|ctx| {
    ///     let crab = ctx.load_texture("crab.png");
    ///     move |ctx: &mut Ctx| ctx.draw(&crab, vec2(400.0, 300.0))
    /// });
    /// ```
    pub fn load_texture(&mut self, path: &str) -> Texture {
        self.gfx.load_texture(path)
    }

    /// Loads a sound from an [added asset source](crate::Assets::add_source)
    /// or the `assets/` folder. A shortcut for
    /// [`ctx.audio.load`](crate::Audio::load), which has the details.
    ///
    /// # Panics
    /// With a clear message if the file is missing or, with the `audio`
    /// feature, can't be decoded.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let hit = ctx.load_sound("hit.ogg");
    /// ctx.play_sound(&hit);
    /// # }
    /// ```
    pub fn load_sound(&mut self, path: &str) -> crate::Sound {
        self.audio.load(path)
    }

    /// Plays a sound once at full volume. A shortcut for
    /// [`ctx.audio.play`](crate::Audio::play); the other ways to play
    /// (volume, pitch, looping, buses, positions) are on
    /// [`ctx.audio`](Self::audio).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, hit: Sound) {
    /// ctx.play_sound(&hit);
    /// # }
    /// ```
    pub fn play_sound(&mut self, sound: &crate::Sound) -> crate::Playing {
        self.audio.play(sound)
    }

    /// Engine side: this context has no window or GPU.
    pub(crate) fn set_headless(&mut self, headless: bool) {
        self.headless = headless;
        self.perf.headless = headless;
        self.gfx.headless = headless;
        self.audio.headless = headless;
    }

    /// Queues the HUD's drawing for this frame (the game loop calls it
    /// before rendering).
    pub(crate) fn draw_perf_hud(&mut self) {
        if !self.perf.profiler.hud || self.perf.profiler.hud_text().is_empty() {
            return;
        }
        if self.perf.state.on && self.perf.state.hud_due() {
            let before = crate::alloc_counts();
            self.perf.entities = self.world.len();
            let report = self.perf.report();
            crate::perf_report::hud_lines(&mut self.perf.state.hud_extra, &report);
            // Building the overlay isn't the game's allocation.
            self.perf.state.forgive_allocs(before);
        }
        let mut text = std::mem::take(self.perf.profiler.hud_text_mut());
        let base = text.len();
        text.push_str(&self.perf.state.hud_extra);
        let lines = text.lines().count() as f32;
        let (size, pad) = (14.0, 8.0);
        let panel = crate::Rect::new(4.0, 4.0, 420.0, lines * size * 1.25 + pad * 2.0);
        self.gfx.ui_fill(panel, Color::rgba(0.0, 0.0, 0.0, 0.6), 100.0);
        self.gfx.draw_text_ex(
            &text,
            panel.min + pad,
            crate::TextParams {
                size,
                color: Color::WHITE,
                z: 101.0,
                ..Default::default()
            },
        );
        text.truncate(base);
        *self.perf.profiler.hud_text_mut() = text;
    }

    // ---------------------------------------------------------------- timing

    /// Seconds since the previous frame. Multiply speeds by this for smooth,
    /// frame-rate independent movement. A shortcut for
    /// [`ctx.time.dt()`](crate::Time::dt).
    ///
    /// Clamped to at most 0.25 s so a hiccup never teleports objects.
    ///
    /// Inside a simulation system (added with
    /// [`Game::sim_system`](crate::Game::sim_system)) this is the fixed step
    /// instead, so simulation code just uses `dt()` too. In a system added
    /// with [`Game::sim_system_every`](crate::Game::sim_system_every) it is
    /// that many fixed steps.
    ///
    /// The meaning follows the running system. To say which clock you mean,
    /// use the explicit pair: [`ctx.time.frame_dt()`](crate::Time::frame_dt)
    /// is always the frame's delta, and
    /// [`ctx.time.sim_dt()`](crate::Time::sim_dt) is always the simulation's.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pos: &mut Vec2) {
    /// pos.x += 200.0 * ctx.dt(); // 200 pixels per second
    /// # }
    /// ```
    pub fn dt(&self) -> f32 {
        self.time.dt()
    }

    // ---------------------------------------------------------------- keyboard

    /// `true` while `key` is held down.
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, charging: &mut bool) {
    /// if ctx.key_released(Key::Space) { *charging = false; }
    /// # }
    /// ```
    pub fn key_released(&self, key: Key) -> bool {
        self.input.key_released(key)
    }

    // ---------------------------------------------------------------- mouse

    /// Mouse position in window pixels. `(0, 0)` is the top-left corner,
    /// `y` grows downward.
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.input.mouse_pressed(MouseButton::Left) {
    ///     println!("clicked the world at {}", ctx.mouse_world());
    /// }
    /// # }
    /// ```
    pub fn mouse_world(&self) -> Vec2 {
        self.camera.screen_to_world(self.mouse_pos())
    }

    // ---------------------------------------------------------------- gamepads

    // ---------------------------------------------------------------- window

    /// Window size in pixels (logical pixels, so it matches [`mouse_pos`](Self::mouse_pos)
    /// on high-DPI screens).
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx) {
    /// if ctx.key_pressed(Key::Escape) { ctx.quit(); }
    /// # }
    /// ```
    pub fn quit(&mut self) {
        self.quit_requested = true;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::DrawParams;

    fn timing(dt: f32) -> FrameTiming {
        FrameTiming {
            dt,
            fixed_steps: 1,
            alpha: 0.25,
        }
    }

    fn png(size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(size, size, image::Rgba([0, 0, 255, 255]))
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn source_assets_are_cached_by_source_and_never_watched() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        ctx.set_headless(true);
        ctx.watcher.set_on(true);
        // An absolute name resolves as itself in the default source.
        let dir = std::env::temp_dir().join(format!("ponko-source-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("crab.png");
        std::fs::write(&file, png(2)).unwrap();
        let name = file.to_str().unwrap().to_string();
        let from_folder = ctx.load_texture(&name);
        assert_eq!(ctx.watcher.watched(), 1, "default-source files are watched");
        ctx.add_asset_source(crate::MemorySource::new("mem").with(name.clone(), png(3)));
        let from_source = ctx.load_texture(&name);
        assert_ne!(from_source, from_folder, "same name, other source: another asset");
        assert_eq!(from_source.width(), 3);
        assert_eq!(ctx.watcher.watched(), 1, "source assets are not watched");
        let keys: Vec<_> = ctx.gfx.loaded.keys().map(|(origin, _)| origin.clone()).collect();
        assert!(keys.contains(&assets::Origin::Source {
            source: "mem".into(),
            name: name.clone()
        }));
        assert!(keys.iter().any(|o| matches!(o, assets::Origin::File(_))));
        assert_eq!(ctx.load_texture(&name), from_source, "the source's copy stays cached");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A WAV file of silence, so sounds load with and without the `audio` feature.
    fn wav() -> Vec<u8> {
        let samples = [0i16; 800];
        let data_len = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&8000u32.to_le_bytes());
        out.extend_from_slice(&16_000u32.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.resize(out.len() + data_len as usize, 0);
        out
    }

    #[test]
    fn sounds_from_sources_are_cached_by_source_and_never_watched() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        ctx.set_headless(true);
        ctx.watcher.set_on(true);
        let dir = std::env::temp_dir().join(format!("ponko-source-sound-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tone.wav");
        std::fs::write(&file, wav()).unwrap();
        let name = file.to_str().unwrap().to_string();
        let from_folder = ctx.load_sound(&name);
        assert_eq!(ctx.watcher.watched(), 1, "default-source sounds are watched");
        ctx.add_asset_source(crate::MemorySource::new("mem").with(name.clone(), wav()));
        let from_source = ctx.load_sound(&name);
        assert_ne!(from_source, from_folder, "same name, other source: another sound");
        assert_eq!(ctx.watcher.watched(), 1, "source sounds are not watched");
        assert_eq!(ctx.load_sound(&name), from_source, "the source's copy stays cached");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn shaders_from_sources_are_cached_by_source_and_never_watched() {
        let wgsl = "fn surface(s: Surface) -> Surface { return s; }";
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        ctx.set_headless(true);
        ctx.watcher.set_on(true);
        let dir = std::env::temp_dir().join(format!("ponko-source-shader-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("plain.wgsl");
        std::fs::write(&file, wgsl).unwrap();
        let name = file.to_str().unwrap().to_string();
        let from_folder = ctx.gfx.load_shader(&name);
        assert_eq!(ctx.watcher.watched(), 1, "default-source shaders are watched");
        ctx.add_asset_source(crate::MemorySource::new("mem").with(name.clone(), wgsl));
        let from_source = ctx.gfx.load_shader(&name);
        assert_ne!(from_source, from_folder, "same name, other source: another shader");
        assert_eq!(ctx.watcher.watched(), 1, "source shaders are not watched");
        assert_eq!(
            ctx.gfx.load_shader(&name),
            from_source,
            "the source's copy stays cached"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn begin_frame_updates_timing() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(800.0, 600.0));
        ctx.begin_frame(timing(0.5));
        ctx.begin_frame(timing(0.25));
        assert_eq!(ctx.time.frame(), 2);
        assert_eq!(ctx.dt(), 0.25);
        assert!((ctx.time.elapsed() - 0.75).abs() < 1e-9);
        assert_eq!(ctx.time.fixed_steps(), 1);
        assert_eq!(ctx.time.fixed_alpha(), 0.25);
    }

    #[test]
    fn fps_is_seeded_then_smoothed() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        ctx.begin_frame(timing(1.0 / 60.0));
        assert!((ctx.time.fps() - 60.0).abs() < 0.01);
        ctx.begin_frame(timing(1.0 / 30.0));
        assert!(ctx.time.fps() > 30.0 && ctx.time.fps() < 60.0);
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
        ctx.gfx.draw_ex(
            &tex,
            Vec2::ZERO,
            DrawParams {
                scale: Vec2::splat(2.0),
                ..Default::default()
            },
        );
        assert_eq!(ctx.gfx.sprite_count(), 2);
        assert_eq!(ctx.render_frame(), FrameStatus::Skipped);
        assert_eq!(ctx.gfx.sprite_count(), 0, "queue is cleared after each frame");
    }

    #[test]
    fn quit_sets_flag() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        assert!(!ctx.quit_requested());
        ctx.quit();
        assert!(ctx.quit_requested());
    }
}
