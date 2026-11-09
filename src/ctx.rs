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
use crate::input::{Gamepad, GamepadAxis, GamepadButton, GamepadStick, Input, Key, MouseButton};
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
    pub(crate) headless_textures: u32,
    /// Asset hot reload: which files textures came from.
    pub(crate) watcher: crate::hot_reload::Watcher,
    /// How to apply each loaded data file's text (`load_resource`).
    data_loaders: Vec<DataLoader>,
    /// Parsed prefab files, by name (`spawn_prefab`).
    pub(crate) prefabs: HashMap<String, serde_json::Value>,
    /// Asset sources added with `add_asset_source`, asked before `assets/`.
    pub(crate) asset_sources: assets::Sources,
    /// Assets already loaded, by origin (resolved path, or source and
    /// name): loading one again returns the same handle.
    pub(crate) loaded: HashMap<(assets::Origin, AssetKind), u64>,
    /// Textures and sounds loaded from files, indexed by `loaded` values.
    cached_textures: Vec<Texture>,
    pub(crate) cached_sounds: Vec<crate::Sound>,
    pub(crate) timings: Vec<SystemTiming>,
    pub(crate) input: Input,
    timing: FrameTiming,
    fixed_dt: f32,
    time: f64,
    frame: u64,
    fps: f32,
    window_size: Vec2,
    quit_requested: bool,
    /// The cursor is hidden and held in the window (`set_cursor_locked`).
    cursor_locked: bool,
    /// `cursor_locked` changed since the window last applied it.
    pub(crate) cursor_lock_changed: bool,
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
    /// let haze = ctx.load_effect("effects/haze.wgsl");
    /// ctx.effects.push(ScreenEffect::new(haze));
    /// # }
    /// ```
    #[cfg(feature = "3d")]
    pub effects: Vec<crate::ScreenEffect>,
    /// Cameras drawing the 3D world into render targets each frame, before
    /// the main view (feature `3d`): mirrors, reflections, security
    /// cameras. See [`TargetCamera`](crate::TargetCamera).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let lake = ctx.create_render_target(960, 540);
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
    /// let ripples = ctx.load_compute("compute/ripples.wgsl");
    /// let water = ctx.create_render_target(256, 256);
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
    #[cfg(feature = "3d")]
    pub(crate) assets3d: crate::three::Assets3D,
    #[cfg(feature = "3d")]
    pub(crate) frame3d: crate::three::Frame3D,
    pub(crate) text: Text,
    /// The glyph atlas, created on the first `draw_text`.
    font_atlas: Option<Texture>,
    /// Physical pixels per window pixel (2 on most Retina screens).
    pub(crate) scale_factor: f32,
    pub(crate) ui: UiState,
    /// Retained widget tree and screen stack. Processed before frame systems.
    /// ```
    /// use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let root = ctx.ui_retained.add(UiWidget::Panel, ui_layout::Style::default());
    /// ctx.ui_retained.push_screen(root, false);
    /// # }
    /// ```
    pub ui_retained: crate::Ui,
    /// Where the left mouse button last went down, for button clicks.
    pub(crate) ui_press_at: Vec2,
    /// Colors and sizes of [`ui_button`](Self::ui_button) and friends.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// ctx.ui_style.text_size = 20.0;
    /// # }
    /// ```
    pub ui_style: UiStyle,
    pub(crate) audio: Audio,
    pub(crate) last_render_stats: RenderStats,
    /// Frame statistics, the HUD, traces and benchmark runs.
    pub(crate) profiler: crate::profiler::Profiler,
    /// Scopes, allocation counts and the history behind `perf_report`.
    pub(crate) perf: crate::perf_report::PerfState,
    /// The renderer's per-pass GPU times, copied after every frame.
    gpu_passes: Vec<crate::GpuPass>,
}

impl Ctx {
    /// `fixed_dt` in seconds, as f64 so `sim_time()` stays exact over long runs.
    pub(crate) fn new(fixed_dt: f64, window_size: Vec2) -> Self {
        Self {
            world: {
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
                }
                crate::reflect::reflect_sprite(&mut world);
                world
            },
            commands: Commands::new(),
            spatial: SpatialGrid::default(),
            #[cfg(feature = "physics")]
            physics: crate::Physics::default(),
            #[cfg(feature = "physics3d")]
            physics3d: crate::Physics3D::default(),
            net: crate::Net::default(),
            events: Events::default(),
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
            prefabs: HashMap::new(),
            asset_sources: assets::Sources::default(),
            loaded: HashMap::new(),
            cached_textures: Vec::new(),
            cached_sounds: Vec::new(),
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
            cursor_locked: false,
            cursor_lock_changed: false,
            renderer: None,
            screenshots: Vec::new(),
            draw_list: DrawList::default(),
            ui_list: DrawList::default(),
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
            target_cameras: Vec::new(),
            #[cfg(feature = "3d")]
            compute_passes: Vec::new(),
            #[cfg(feature = "3d")]
            sky: crate::Sky::default(),
            #[cfg(feature = "3d")]
            assets3d: Default::default(),
            #[cfg(feature = "3d")]
            frame3d: Default::default(),
            text: Text::new(),
            font_atlas: None,
            scale_factor: 1.0,
            ui: UiState::default(),
            ui_retained: crate::Ui::default(),
            ui_press_at: Vec2::splat(f32::NAN),
            ui_style: UiStyle::default(),
            audio: Audio::default(),
            last_render_stats: RenderStats::default(),
            profiler: crate::profiler::Profiler::new(),
            perf: crate::perf_report::PerfState::new(),
            gpu_passes: Vec::new(),
            clear_color: Color::PONKO_DARK,
        }
    }

    /// Engine side: draw everything queued this frame, then reset the queue.
    pub(crate) fn render_frame(&mut self) -> FrameStatus {
        self.draw_retained_ui();
        let capture = !self.screenshots.is_empty();
        #[cfg(feature = "3d")]
        self.draw_ui_panels();
        #[cfg(feature = "3d")]
        self.prepare_3d();
        #[cfg(feature = "3d")]
        let panels = self.ui.panels.frame_draws();
        #[cfg(not(feature = "3d"))]
        let panels: &mut [crate::render::batch::PanelDraw] = &mut [];
        let status = match self.renderer.as_mut() {
            Some(renderer) => renderer.render(
                &mut self.draw_list,
                &mut self.ui_list,
                panels,
                self.clear_color,
                self.camera.view_proj(),
                screen_projection(self.window_size),
                capture,
            ),
            None => FrameStatus::Skipped,
        };
        #[cfg(feature = "3d")]
        // A frame that wasn't drawn hands its probe captures to the next one.
        self.assets3d
            .probes
            .settle(!matches!(status, FrameStatus::Occluded | FrameStatus::Skipped));
        self.last_render_stats = self.renderer.as_ref().map(Renderer::stats).unwrap_or_default();
        if let Some(renderer) = &self.renderer {
            self.gpu_passes.clear();
            self.gpu_passes.extend_from_slice(renderer.gpu_passes());
        }
        self.draw_list.clear();
        self.ui_list.clear();
        if let Some(shot) = self.renderer.as_mut().and_then(Renderer::take_screenshot) {
            for req in self.screenshots.drain(..) {
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

    pub(crate) fn queue_screenshot(
        &mut self,
        path: PathBuf,
        max_size: u32,
        reply: Option<Sender<Result<Value, String>>>,
    ) {
        self.screenshots.push(ScreenshotRequest { path, max_size, reply });
    }

    pub(crate) fn renderer(&mut self) -> &mut Renderer {
        self.renderer
            .as_mut()
            .expect("ponko: textures can only be created once the game is running (in setup or update)")
    }

    /// Current binding configuration.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.actions();
    /// # }
    /// ```
    pub fn actions(&self) -> &crate::ActionMap {
        &self.input.actions
    }

    /// Resolves an action name once into a `Copy` [`ActionId`](crate::ActionId)
    /// for the queries below, which then skip the string lookup. An unknown
    /// name logs an error (naming the known actions) and gives
    /// [`ActionId::NONE`](crate::ActionId::NONE), which reads false/zero.
    /// Resolve in setup, after installing the bindings with
    /// [`set_actions`](Self::set_actions); later rebinding keeps the id valid.
    ///
    /// The queries take an id or a `&str` name. They are meant to move to the
    /// `ctx.input` service of the API-friction work (#195).
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let jump = ctx.action_id("jump");
    /// if ctx.action_pressed(jump) { /* ... */ }
    /// # }
    /// ```
    pub fn action_id(&self, name: &str) -> crate::ActionId {
        match self.input.actions.resolve(name) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("ponko: {e}");
                crate::ActionId::NONE
            }
        }
    }

    /// Like [`action_id`](Self::action_id), but returns the error instead of
    /// logging it.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let jump = ctx.try_action_id("jump");
    /// # }
    /// ```
    pub fn try_action_id(&self, name: &str) -> Result<crate::ActionId, crate::UnknownAction> {
        self.input.actions.resolve(name)
    }

    /// Resolves several actions at once; see also [`actions!`](crate::actions!),
    /// which keeps the names in one declaration.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let [jump, fire] = ctx.action_ids(&["jump", "fire"]);
    /// # }
    /// ```
    pub fn action_ids<const N: usize>(&self, names: &[&str; N]) -> [crate::ActionId; N] {
        names.map(|n| self.action_id(n))
    }

    /// Whether a named button is held. Takes a name or an
    /// [`ActionId`](crate::ActionId) from [`action_id`](Self::action_id).
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.action_down("move");
    /// # }
    /// ```
    pub fn action_down(&self, action: impl crate::ActionRef) -> bool {
        self.input.actions.down(action)
    }

    /// Whether a named button became held this frame.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.action_pressed("move");
    /// # }
    /// ```
    pub fn action_pressed(&self, action: impl crate::ActionRef) -> bool {
        self.input.actions.pressed(action)
    }

    /// Whether all sources of a named button were released this frame.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.action_released("move");
    /// # }
    /// ```
    pub fn action_released(&self, action: impl crate::ActionRef) -> bool {
        self.input.actions.released(action)
    }

    /// A named scalar axis, or zero.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.action_axis("move");
    /// # }
    /// ```
    pub fn action_axis(&self, action: impl crate::ActionRef) -> f32 {
        self.input.actions.axis(action)
    }

    /// A named vector axis, or zero.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &Ctx) {
    /// let value = ctx.action_axis2("move");
    /// # }
    /// ```
    pub fn action_axis2(&self, action: impl crate::ActionRef) -> Vec2 {
        self.input.actions.axis2(action)
    }

    /// Replace bindings at runtime. Held sources establish a baseline;
    /// rebinding itself generates no pressed/released edges.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &mut Ctx) {
    /// ctx.set_actions(ActionMap::default());
    /// # }
    /// ```
    pub fn set_actions(&mut self, mut actions: crate::ActionMap) {
        actions.prepare_after(&self.input.actions);
        self.input.actions = actions;
        self.input.refresh_actions();
        self.input.actions.end_frame();
    }

    /// Feed input by hand for tests/replay; live input uses the same state.
    /// Edges clear after each headless or windowed frame.
    /// ```
    /// # use ponko::*;
    /// # fn example(ctx: &mut Ctx) {
    /// ctx.feed_input(InputEvent::Key(Key::Space, true));
    /// # }
    /// ```
    pub fn feed_input(&mut self, event: crate::InputEvent) {
        use crate::InputEvent;
        match event {
            InputEvent::Key(k, down) => self.input.set_key(k, down),
            InputEvent::Mouse(b, down) => self.input.set_mouse_button(b, down),
            InputEvent::GamepadButton(p, b, down) => self.input.set_gamepad_button(p, b, down),
            InputEvent::GamepadAxis(p, a, value) => self.input.set_gamepad_axis(p, a, value),
            InputEvent::FocusLost => self.input.release_all(),
            InputEvent::GamepadDisconnected(p) => self.input.disconnect_gamepad(p),
        }
    }

    /// Engine side: start a new frame with fresh timing.
    pub(crate) fn begin_frame(&mut self, timing: FrameTiming) {
        self.timing = timing;
        self.time += f64::from(timing.dt);
        self.frame += 1;
        self.events.begin_frame(self.frame, self.sim.tick);
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
        #[cfg(feature = "3d")]
        self.camera3d.set_viewport(size);
    }

    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// How long each system took, in the order they run (simulation systems
    /// first). For finding what's slow; the inspector will graph these.
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
        self.events.set_tick(self.sim.tick);
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
        crate::Random::new(self.seed, self.sim_tick(), self.system_index)
    }

    /// Splits per-entity work over `every` ticks: entities take turns, so
    /// each tick updates 1/`every` of them. For costly logic that's fine at
    /// a lower rate per entity (path planning, AI decisions). Take it before
    /// the query; it is `Copy`, so it works in `par_for_each` too.
    ///
    /// ```
    /// use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// Loading the same file again returns the same handle.
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a font.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let title = ctx.load_font("fonts/Title.ttf");
    /// # let _ = title;
    /// # }
    /// ```
    pub fn load_font(&mut self, path: &str) -> Font {
        self.try_load_font(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load_font`](Self::load_font) with errors as values:
    /// [`ErrorKind::NotFound`](crate::ErrorKind::NotFound),
    /// [`Io`](crate::ErrorKind::Io), or [`Decode`](crate::ErrorKind::Decode)
    /// if it isn't a font.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let title = ctx.try_load_font("fonts/Fancy.ttf").unwrap_or(Font::DEFAULT);
    ///     # let _ = title;
    /// });
    /// ```
    pub fn try_load_font(&mut self, path: &str) -> Result<Font, crate::Error> {
        if let Some((_, id)) = self.cached_in_sources(path, AssetKind::Font) {
            return Ok(Font(id as u32));
        }
        let found = self.asset_sources.find(path)?;
        let origin = found.origin.clone();
        if let Some(&id) = self.loaded.get(&(origin.clone(), AssetKind::Font)) {
            return Ok(Font(id as u32));
        }
        let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
        let font = self.text.add_font(&bytes).map_err(|e| {
            let what = match &origin {
                assets::Origin::File(file) => file.display().to_string(),
                assets::Origin::Source { .. } => origin.describe(path),
            };
            crate::Error::decode(path, format!("{what}: {e}"))
        })?;
        self.loaded.insert((origin.clone(), AssetKind::Font), u64::from(font.0));
        if let assets::Origin::File(file) = origin {
            self.watcher.watch(path, file, Asset::Font(font.0));
        }
        Ok(font)
    }

    /// Adds a place to load named assets from. Added sources are asked in
    /// the order they were added, all before the `assets/` folder.
    ///
    /// Every load by name goes through it: [`load_texture`](Self::load_texture),
    /// [`load_font`](Self::load_font), [`load_resource`](Self::load_resource),
    /// [`load_scene`](Self::load_scene), [`spawn_prefab`](Self::spawn_prefab)
    /// (and the textures their scenes name), [`load_sound`](Self::load_sound),
    /// [`load_asset_with`](Self::load_asset_with) and, with the `3d` feature,
    /// meshes, skeletons and animation clips (and the files a glTF refers to),
    /// shaders, screen effects and compute passes.
    ///
    /// Not covered yet: [`load_stream`](Self::load_stream) (music streamed
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

    /// A cached `kind` asset called `name` from an added source, searched
    /// in order. Earlier sources win, so this matches what a fresh
    /// [`assets::Sources::find`] would pick for anything still cached.
    fn cached_in_sources(&self, name: &str, kind: AssetKind) -> Option<(assets::Origin, u64)> {
        self.asset_sources
            .origins(name)
            .find_map(|origin| self.loaded.get(&(origin.clone(), kind)).map(|&v| (origin, v)))
    }

    /// Reads asset `name` as UTF-8 text from the first source that has it.
    pub(crate) fn read_asset_text(&self, name: &str) -> Result<(assets::Origin, String), String> {
        let found = self.asset_sources.find(name)?;
        let origin = found.origin.clone();
        Ok((origin, found.into_text(name)?))
    }

    /// Whether `font` is the built-in font or one this context loaded. Only
    /// foreign callers can hold a handle that isn't (C code passing a made-up
    /// id); [`Text3D`](crate::Text3D) would panic when drawn with one.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     assert!(ctx.has_font(Font::DEFAULT));
    /// });
    /// ```
    #[must_use]
    pub fn has_font(&self, font: Font) -> bool {
        self.text.has_font(font)
    }

    /// Loads an image from the `assets/` folder (PNG for now).
    ///
    /// Do this once in setup and keep the [`Texture`]; it's a cheap handle.
    /// Loading the same file again returns the same texture (use
    /// [`create_texture`](Self::create_texture) for a copy of your own).
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
        self.try_load_texture(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load_texture`](Self::load_texture) with errors as values, for
    /// optional assets: [`ErrorKind::NotFound`](crate::ErrorKind::NotFound)
    /// if no source has it, [`Io`](crate::ErrorKind::Io) if it can't be
    /// read, [`Decode`](crate::ErrorKind::Decode) if it isn't an image.
    /// Like `load_texture`, it also names the texture, so sprites showing it
    /// can be saved.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     // A missing optional skin is not a bug: fall back to no skin.
    ///     let skin: Option<Texture> = ctx.try_load_texture("skins/gold.png").ok();
    ///     assert!(skin.is_none());
    /// });
    /// ```
    pub fn try_load_texture(&mut self, path: &str) -> Result<Texture, crate::Error> {
        let cached = match self.cached_in_sources(path, AssetKind::Texture) {
            Some((_, i)) => Ok(i),
            None => {
                let found = self.asset_sources.find(path)?;
                match self.loaded.get(&(found.origin.clone(), AssetKind::Texture)) {
                    Some(&i) => Ok(i),
                    None => Err(found),
                }
            }
        };
        let texture = match cached {
            Ok(i) => self.cached_textures[i as usize],
            Err(found) => {
                // Acquire, decode (CPU only), then create on the GPU (game thread).
                let origin = found.origin.clone();
                let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
                let image = assets::decode_image(&bytes).map_err(|e| {
                    crate::Error::decode(path, format!("{} is not a valid image: {e}", origin.describe(path)))
                })?;
                let texture = self.create_texture(image.width, image.height, &image.rgba);
                self.loaded
                    .insert((origin.clone(), AssetKind::Texture), self.cached_textures.len() as u64);
                self.cached_textures.push(texture);
                if let assets::Origin::File(file) = origin {
                    self.watcher.watch(path, file, Asset::Texture(texture));
                }
                texture
            }
        };
        if let Some(names) = self.world.get_resource_mut::<crate::reflect::TextureNames>() {
            names.by_id.entry(texture.id).or_insert_with(|| path.to_string());
            names.by_name.entry(path.to_string()).or_insert(texture);
        }
        Ok(texture)
    }

    /// Loads the textures of sprites and meshes of models that `json` (a
    /// scene, a prefab, or components) names, so they can be spawned.
    pub(crate) fn load_scene_assets(&mut self, json: &serde_json::Value) -> Result<(), String> {
        let mut names = Vec::new();
        crate::reflect::asset_names(json, "Sprite", "texture", &mut names);
        for name in names.drain(..) {
            self.try_load_texture(&name)?;
        }
        #[cfg(feature = "3d")]
        {
            crate::reflect::asset_names(json, "Model", "mesh", &mut names);
            crate::reflect::asset_names(json, "ShadowMesh", "mesh", &mut names);
            crate::reflect::lod_mesh_names(json, &mut names);
            names.sort_unstable();
            names.dedup();
            for name in names {
                self.try_load_mesh(&name)?;
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
        &self.watcher.reloaded_files
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
        &self.watcher.reloaded
    }

    fn report_reload(&mut self, name: String, applied: Result<(), String>) {
        match applied {
            Ok(()) => {
                eprintln!("ponko: reloaded {name}");
                self.watcher.reloaded_files.push(name);
            }
            Err(e) => eprintln!("ponko: could not reload {name} (keeping the old one): {e}"),
        }
    }

    /// Reloads textures, fonts, sounds and data files that changed (checked a few times a second).
    pub(crate) fn reload_changed_assets(&mut self) {
        self.watcher.reloaded.clear();
        self.watcher.reloaded_files.clear();
        if !self.watcher.on {
            return;
        }
        let mut changed = std::mem::take(&mut self.watcher.pending);
        self.watcher.changed(web_time::Instant::now(), &mut changed);
        for (asset, file, name) in changed.drain(..) {
            let texture = match asset {
                Asset::Texture(t) => t,
                Asset::Font(id) => {
                    let applied = assets::read(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| self.text.replace_font(Font(id), &bytes));
                    self.report_reload(name, applied);
                    continue;
                }
                Asset::Sound(id) => {
                    let applied = assets::read(&file)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| self.audio.replace(id, bytes));
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
                    let applied = assets::read_to_string(&file).and_then(|text| self.reload_shader(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
                #[cfg(feature = "3d")]
                Asset::Effect(id) => {
                    let applied = assets::read_to_string(&file).and_then(|text| self.reload_effect(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
                #[cfg(feature = "3d")]
                Asset::Compute(id) => {
                    let applied = assets::read_to_string(&file).and_then(|text| self.reload_compute(id, &name, &text));
                    self.report_reload(name, applied);
                    continue;
                }
            };
            match assets::load_image_at(&file, &name) {
                Ok(image) if (image.width, image.height) == (texture.width(), texture.height()) => {
                    self.update_texture(&texture, &image.rgba);
                    self.watcher.reloaded.push(texture);
                    self.watcher.reloaded_files.push(name.clone());
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
    /// # use ponko::*;
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
            "ponko: texture size must not be zero (got {width} x {height})"
        );
        let expected = width as usize * height as usize * 4;
        assert_eq!(
            rgba.len(),
            expected,
            "ponko: a {width} x {height} texture needs {expected} bytes of RGBA, got {}",
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
            "ponko: {width} x {height} is larger than this GPU's maximum texture size ({max})"
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
    /// # use ponko::*;
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
            "ponko: a {w} x {h} texture needs {expected} bytes of RGBA, got {}",
            rgba.len()
        );
        #[cfg(feature = "3d")]
        assert!(
            !self.is_render_target(*texture),
            "ponko: a render target is drawn by its target camera, not updated with pixels"
        );
        if self.headless {
            return;
        }
        self.renderer().update_texture(texture.id, w, h, rgba);
    }

    /// How many sprites have been drawn so far this frame.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx) {
    /// println!("{} sprites", ctx.sprite_count());
    /// # }
    /// ```
    pub fn sprite_count(&self) -> usize {
        self.draw_list.len() + self.ui_list.len()
    }

    /// Saves the next frame as a PNG at `path`, once it has been drawn.
    /// Does nothing in headless runs (there is no frame). Errors are printed.
    ///
    /// ```
    /// # use ponko::*;
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

    /// How long the last frame waited for the display (ms): idle time, not
    /// work. For benchmarks that subtract it from the frame time.
    #[doc(hidden)]
    #[must_use]
    pub fn display_wait_ms(&self) -> f32 {
        self.renderer.as_ref().map_or(0.0, Renderer::display_wait_ms)
    }

    /// What drawing the previous frame cost: sprites, draw calls, CPU time
    /// and GPU time. All zero (and `gpu_ms` is `None`) in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn overlay(ctx: &mut Ctx) {
    ///     let stats = ctx.render_stats();
    ///     if let Some(gpu) = stats.gpu_ms {
    ///         println!("{} sprites: {:.2} ms CPU, {gpu:.2} ms GPU", stats.sprites, stats.cpu_ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(overlay).run_headless(1, |_| {});
    /// ```
    pub fn render_stats(&self) -> RenderStats {
        self.last_render_stats
    }

    /// Frame-time statistics over the last few seconds: frames per second,
    /// average and percentile frame times, and where the time went (systems,
    /// drawing on the CPU, waiting for the display, the GPU). Refreshed four
    /// times a second. All zero in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn report(ctx: &mut Ctx) {
    ///     let s = ctx.frame_stats();
    ///     if s.p99_ms > 2.0 * s.p50_ms {
    ///         println!("stutter: p99 {:.1} ms vs median {:.1} ms", s.p99_ms, s.p50_ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(report).run_headless(1, |_| {});
    /// ```
    pub fn frame_stats(&self) -> crate::FrameStats {
        self.profiler.stats()
    }

    /// The GPU time of each pass of the last frames (shadow cascades, depth
    /// prepass, opaque, transparent, bloom, post, ...), smoothed, in the
    /// order they run. Empty in headless runs and on GPUs without timestamp
    /// queries.
    ///
    /// ```
    /// # use ponko::*;
    /// fn slowest_pass(ctx: &mut Ctx) {
    ///     if let Some(p) = ctx.gpu_passes().iter().max_by(|a, b| a.ms.total_cmp(&b.ms)) {
    ///         println!("slowest GPU pass: {} ({:.2} ms)", p.name, p.ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(slowest_pass).run_headless(1, |_| {});
    /// ```
    pub fn gpu_passes(&self) -> &[crate::GpuPass] {
        &self.gpu_passes
    }

    /// Shows or hides the performance HUD: frame times, CPU and GPU time,
    /// the slowest GPU passes and systems, drawn in the top-left corner.
    /// `PONKO_HUD=1` shows it from the start.
    ///
    /// ```
    /// # use ponko::*;
    /// fn toggle_hud(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F3) {
    ///         let on = ctx.perf_hud();
    ///         ctx.set_perf_hud(!on);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(toggle_hud).run_headless(1, |_| {});
    /// ```
    pub fn set_perf_hud(&mut self, on: bool) {
        self.profiler.hud = on;
    }

    /// Whether the performance HUD is showing.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let on = ctx.perf_hud();
    /// # }
    /// ```
    pub fn perf_hud(&self) -> bool {
        self.profiler.hud
    }

    /// Records the next `frames` frames as a Chrome trace at `path`: each
    /// frame's systems, drawing and display wait on one track, and the GPU
    /// passes on another. Open it in <https://ui.perfetto.dev>. Does nothing
    /// in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn capture(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F4) {
    ///         ctx.record_trace("frames.json", 120);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(capture).run_headless(1, |_| {});
    /// ```
    pub fn record_trace(&mut self, path: impl Into<PathBuf>, frames: usize) {
        if !self.headless {
            self.profiler.start_trace(path.into(), frames.max(1));
        }
    }

    /// Turns on the deeper profiling: your [`profile_scope!`](crate::profile_scope)
    /// scopes and [`Ctx::profile`] calls are timed, allocations are counted
    /// (if [`CountingAllocator`](crate::CountingAllocator) is installed), and
    /// [`perf_report`](Self::perf_report) has numbers. The overlay
    /// (`set_perf_hud`) and `PONKO_BENCH` switch it on by themselves. While
    /// it is off, scopes cost one atomic load.
    ///
    /// ```
    /// # use ponko::*;
    /// fn debug_keys(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F5) {
    ///         let on = !ctx.profiling();
    ///         ctx.set_profiling(on);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(debug_keys).run_headless(1, |_| {});
    /// ```
    pub fn set_profiling(&mut self, on: bool) {
        self.perf.explicit = on;
        self.sync_profiling();
    }

    /// Whether scopes are being timed and allocations counted right now.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.profiling() { /* build an expensive debug string */ }
    /// # }
    /// ```
    pub fn profiling(&self) -> bool {
        self.perf.on
    }

    /// Times `f` as a scope called `name` (see
    /// [`profile_scope!`](crate::profile_scope), which is cheaper in hot
    /// code: this looks the name up each time). Times add up across calls.
    ///
    /// ```
    /// # use ponko::*;
    /// fn ai(ctx: &mut Ctx) {
    ///     let target = ctx.profile("ai/pathing", |_ctx| 42);
    ///     assert_eq!(target, 42);
    /// }
    /// # Game::new("t", 1, 1).sim_system(ai).run_headless(1, |_| {});
    /// ```
    pub fn profile<R>(&mut self, name: &str, f: impl FnOnce(&mut Ctx) -> R) -> R {
        if !crate::scope::active() {
            return f(self);
        }
        let scope = crate::scope_named(name);
        let token = scope.begin();
        let out = f(self);
        scope.end(token);
        out
    }

    /// Each profiling scope's time in the last frame, in the order they
    /// first ran. Empty while profiling is off.
    ///
    /// ```
    /// # use ponko::*;
    /// fn report(ctx: &mut Ctx) {
    ///     for s in ctx.scope_timings() {
    ///         println!("{}: {:.2} ms in {} calls", s.name, s.last_ms, s.calls);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(report).run_headless(1, |_| {});
    /// ```
    pub fn scope_timings(&self) -> &[crate::ScopeTiming] {
        &self.perf.scopes.timings
    }

    /// Where time and allocations went over the last frames (about half a
    /// minute in a window; every tick of a [`Game::profile_headless`](crate::Game::profile_headless) run):
    /// frame-time percentiles, systems, scopes, allocations, the slowest
    /// frames and what was in them, drawing stats, and warnings with advice.
    /// `println!("{report}")` prints it. Empty until profiling is on (see
    /// [`set_profiling`](Self::set_profiling)).
    ///
    /// ```
    /// # use ponko::*;
    /// fn dump(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F6) {
    ///         println!("{}", ctx.perf_report());
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(dump).run_headless(1, |_| {});
    /// ```
    pub fn perf_report(&self) -> crate::PerfReport {
        let mut report = self.perf.collector.report();
        report.entities = self.world.len();
        if !self.headless && report.frames > 0 {
            let f = self.profiler.stats();
            let r = self.last_render_stats;
            let mut passes: Vec<_> = self
                .gpu_passes
                .iter()
                .map(|p| crate::PassReport {
                    name: p.name.into(),
                    ms: p.ms,
                })
                .collect();
            passes.sort_by(|a, b| b.ms.total_cmp(&a.ms));
            report.render = Some(crate::RenderReport {
                systems_ms: f.systems_ms,
                render_cpu_ms: f.render_cpu_ms,
                display_wait_ms: f.display_wait_ms,
                gpu_ms: f.gpu_ms,
                draw_calls: r.draw_calls,
                sprites: r.sprites,
                models: r.instances,
                triangles: r.triangles,
                shadow_models: r.shadow_instances,
                shadow_triangles: r.shadow_triangles,
                occluded: r.occluded,
                gpu_passes: passes,
            });
        }
        crate::perf_report::analyze(&mut report);
        report
    }

    /// Starts or stops the deeper profiling to match what's wanted: the
    /// overlay, a benchmark run, or `set_profiling`.
    pub(crate) fn sync_profiling(&mut self) {
        let wanted = self.perf.explicit || self.profiler.hud || self.profiler.benchmarking();
        self.perf.sync(wanted);
    }

    /// Ends a frame (or headless tick) that took `ms` for the report.
    pub(crate) fn end_perf_frame(&mut self, ms: f32) {
        self.perf.end_frame(ms, &self.timings);
    }

    /// Hands one frame's timings to the profiler. `true` when a benchmark
    /// run is done and the game should quit.
    pub(crate) fn record_frame(&mut self, sample: crate::profiler::FrameSample) -> bool {
        let gpu_frame = self.renderer.as_ref().and_then(Renderer::gpu_frame);
        let quit = self.profiler.record(
            sample,
            &self.timings,
            &self.gpu_passes,
            gpu_frame,
            self.last_render_stats,
        );
        self.end_perf_frame(sample.frame_ms);
        if self.profiler.warmup_ends_now() {
            // A benchmark's report covers the measured frames only.
            self.perf.collector.clear();
        }
        if quit {
            let report = self.perf_report();
            eprint!("{report}");
            if let Some(path) = crate::environment::var_os("PONKO_BENCH_REPORT")
                && let Err(e) = report.save(&path)
            {
                eprintln!("ponko: could not write {}: {e}", std::path::Path::new(&path).display());
            }
        }
        quit
    }

    /// Queues the HUD's drawing for this frame (the game loop calls it
    /// before rendering).
    pub(crate) fn draw_perf_hud(&mut self) {
        if !self.profiler.hud || self.profiler.hud_text().is_empty() {
            return;
        }
        if self.perf.on && self.perf.hud_due() {
            let before = crate::alloc_counts();
            let report = self.perf_report();
            crate::perf_report::hud_lines(&mut self.perf.hud_extra, &report);
            // Building the overlay isn't the game's allocation.
            self.perf.forgive_allocs(before);
        }
        let mut text = std::mem::take(self.profiler.hud_text_mut());
        let base = text.len();
        text.push_str(&self.perf.hud_extra);
        let lines = text.lines().count() as f32;
        let (size, pad) = (14.0, 8.0);
        let panel = crate::Rect::new(4.0, 4.0, 420.0, lines * size * 1.25 + pad * 2.0);
        self.ui_fill(panel, Color::rgba(0.0, 0.0, 0.0, 0.6), 100.0);
        self.draw_text_ex(
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
        *self.profiler.hud_text_mut() = text;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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

    /// `true` while `button` is held down.
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, camera: &mut Vec2) {
    /// if ctx.mouse_down(MouseButton::Right) { *camera -= ctx.mouse_delta(); }
    /// # }
    /// ```
    pub fn mouse_delta(&self) -> Vec2 {
        self.input.mouse_delta()
    }

    /// How far the mouse itself moved this frame, in the system's raw
    /// mouse units (about a pixel each, without pointer acceleration). Unlike
    /// [`mouse_delta`](Self::mouse_delta) it keeps coming while the cursor is
    /// locked and at the window's edge: use it to turn a first-person camera.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn look(ctx: &mut Ctx, yaw: &mut f32, pitch: &mut f32) {
    /// let turn = ctx.mouse_motion() * 0.002; // radians per unit
    /// *yaw -= turn.x;
    /// *pitch = (*pitch - turn.y).clamp(-1.5, 1.5);
    /// # }
    /// ```
    pub fn mouse_motion(&self) -> Vec2 {
        self.input.mouse_motion()
    }

    /// Hides the cursor and holds it in the window (`true`), for mouse-look,
    /// or shows and frees it again (`false`). Applied after the frame; read
    /// the movement with [`mouse_motion`](Self::mouse_motion). The system
    /// frees the cursor while the window isn't focused, and it's held again
    /// when focus comes back. Headless runs only remember the setting.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(1, |ctx| {
    ///     ctx.set_cursor_locked(true);
    ///     assert!(ctx.cursor_locked());
    /// });
    /// ```
    pub fn set_cursor_locked(&mut self, locked: bool) {
        if self.cursor_locked != locked {
            self.cursor_locked = locked;
            self.cursor_lock_changed = true;
        }
    }

    /// Whether the cursor is locked, as set with
    /// [`set_cursor_locked`](Self::set_cursor_locked).
    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    /// Scroll wheel movement this frame. `y > 0` means scrolled up / away from
    /// the user. Roughly one unit per wheel notch.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, zoom: &mut f32) {
    /// *zoom *= 1.1_f32.powf(ctx.mouse_wheel().y);
    /// # }
    /// ```
    pub fn mouse_wheel(&self) -> Vec2 {
        self.input.mouse_wheel()
    }

    // ---------------------------------------------------------------- gamepads

    /// The connected gamepads, by slot (player one first). Empty when none
    /// are plugged in, in headless runs, and without the `gamepad` feature.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pos: &mut Vec2) {
    /// if let Some(&pad) = ctx.gamepads().first() {
    ///     *pos += ctx.gamepad_stick(pad, GamepadStick::Left) * 300.0 * ctx.dt();
    /// }
    /// # }
    /// ```
    pub fn gamepads(&self) -> &[Gamepad] {
        self.input.gamepads()
    }

    /// `true` while `button` is held on `pad`.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pad: Gamepad) {
    /// let running = ctx.gamepad_down(pad, GamepadButton::West);
    /// # }
    /// ```
    pub fn gamepad_down(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_down(pad, button)
    }

    /// `true` only on the frame `button` went down on `pad`.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pad: Gamepad) {
    /// if ctx.gamepad_pressed(pad, GamepadButton::South) { println!("jump"); }
    /// # }
    /// ```
    pub fn gamepad_pressed(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_pressed(pad, button)
    }

    /// `true` only on the frame `button` was let go on `pad` (or `pad` was
    /// unplugged while it was held).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pad: Gamepad) {
    /// if ctx.gamepad_released(pad, GamepadButton::RightTrigger) { println!("fire!"); }
    /// # }
    /// ```
    pub fn gamepad_released(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_released(pad, button)
    }

    /// One axis of `pad`: sticks from `-1.0` to `1.0` (y down is positive,
    /// like screen coordinates), triggers from `0.0` to `1.0`. Small
    /// movements around the center read as `0.0`.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pad: Gamepad) {
    /// let throttle = ctx.gamepad_axis(pad, GamepadAxis::RightTrigger);
    /// # }
    /// ```
    pub fn gamepad_axis(&self, pad: Gamepad, axis: GamepadAxis) -> f32 {
        self.input.gamepad_axis(pad, axis)
    }

    /// A stick of `pad` as a vector, each part from `-1.0` to `1.0`, y down:
    /// add it to a position to move that way.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, pad: Gamepad, aim: &mut Vec2) {
    /// let stick = ctx.gamepad_stick(pad, GamepadStick::Right);
    /// if stick.length() > 0.5 { *aim = stick.normalize(); }
    /// # }
    /// ```
    pub fn gamepad_stick(&self, pad: Gamepad, stick: GamepadStick) -> Vec2 {
        let (x, y) = match stick {
            GamepadStick::Left => (GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
            GamepadStick::Right => (GamepadAxis::RightStickX, GamepadAxis::RightStickY),
        };
        Vec2::new(self.gamepad_axis(pad, x), self.gamepad_axis(pad, y))
    }

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
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, e: Entity) {
    /// if ctx.turns(8).includes(e) { /* expensive update */ }
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn includes(self, entity: ponko_ecs::Entity) -> bool {
        entity.index() % self.every == self.now
    }

    /// Time since an entity's previous turn: `every` times `ctx.dt()`.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let turn = ctx.turns(4);
    /// assert_eq!(turn.dt(), ctx.dt() * 4.0);
    /// # }
    /// ```
    #[must_use]
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
        ctx.headless = true;
        ctx.watcher.on = true;
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
        let keys: Vec<_> = ctx.loaded.keys().map(|(origin, _)| origin.clone()).collect();
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
        ctx.headless = true;
        ctx.watcher.on = true;
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
        ctx.headless = true;
        ctx.watcher.on = true;
        let dir = std::env::temp_dir().join(format!("ponko-source-shader-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("plain.wgsl");
        std::fs::write(&file, wgsl).unwrap();
        let name = file.to_str().unwrap().to_string();
        let from_folder = ctx.load_shader(&name);
        assert_eq!(ctx.watcher.watched(), 1, "default-source shaders are watched");
        ctx.add_asset_source(crate::MemorySource::new("mem").with(name.clone(), wgsl));
        let from_source = ctx.load_shader(&name);
        assert_ne!(from_source, from_folder, "same name, other source: another shader");
        assert_eq!(ctx.watcher.watched(), 1, "source shaders are not watched");
        assert_eq!(ctx.load_shader(&name), from_source, "the source's copy stays cached");
        let _ = std::fs::remove_dir_all(dir);
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
