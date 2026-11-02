//! [`Ctx`]: everything game code talks to each frame.

use glam::{Mat4, Vec2};

use crate::assets;
use crate::color::Color;
use crate::input::{Input, Key, MouseButton};
use crate::render::batch::{DrawList, SpriteInstance};
use crate::render::{FrameStatus, Renderer};
use crate::texture::{DrawParams, Texture};
use crate::time::FrameTiming;

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

/// The game's handle to the engine, passed to your update closure every frame.
///
/// Use it to draw, read input, and check timing and window info.
pub struct Ctx {
    pub(crate) input: Input,
    timing: FrameTiming,
    fixed_dt: f32,
    time: f64,
    frame: u64,
    fps: f32,
    window_size: Vec2,
    quit_requested: bool,
    pub(crate) renderer: Option<Renderer>,
    draw_list: DrawList,
    clear_color: Color,
}

impl Ctx {
    pub(crate) fn new(fixed_dt: f32, window_size: Vec2) -> Self {
        Self {
            input: Input::new(),
            timing: FrameTiming {
                dt: 0.0,
                fixed_steps: 0,
                alpha: 0.0,
            },
            fixed_dt,
            time: 0.0,
            frame: 0,
            fps: 0.0,
            window_size,
            quit_requested: false,
            renderer: None,
            draw_list: DrawList::default(),
            clear_color: Color::CRABBY_DARK,
        }
    }

    /// Engine side: draw everything queued this frame, then reset the queue.
    pub(crate) fn render_frame(&mut self) -> FrameStatus {
        let status = match self.renderer.as_mut() {
            Some(renderer) => {
                let view_proj = screen_projection(self.window_size);
                renderer.render(&mut self.draw_list, self.clear_color, view_proj)
            }
            None => FrameStatus::Skipped,
        };
        self.draw_list.clear();
        status
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

    pub(crate) fn set_window_size(&mut self, size: Vec2) {
        self.window_size = size;
    }

    pub(crate) fn quit_requested(&self) -> bool {
        self.quit_requested
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

    /// Draws `texture` centered at `pos` (window pixels, `(0, 0)` = top-left).
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
        self.push_sprite(
            texture,
            pos,
            texture.size(),
            0.0,
            0.0,
            Vec2::splat(0.5),
            [255; 4],
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
        self.push_sprite(
            texture,
            pos,
            texture.size() * params.scale,
            params.rotation,
            params.z,
            params.pivot,
            params.color.to_rgba8(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn push_sprite(
        &mut self,
        texture: &Texture,
        pos: Vec2,
        size: Vec2,
        rotation: f32,
        z: f32,
        pivot: Vec2,
        color: [u8; 4],
    ) {
        let (sin, cos) = if rotation == 0.0 {
            (0.0, 1.0)
        } else {
            rotation.sin_cos()
        };
        self.draw_list.push(
            texture.id,
            z,
            SpriteInstance {
                pos: pos.into(),
                size: size.into(),
                pivot: pivot.into(),
                rot: [cos, sin],
                uv: [0.0, 0.0, 1.0, 1.0],
                color,
            },
        );
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
        let image = assets::load_image(path).unwrap_or_else(|e| panic!("crabbyengine: {e}"));
        self.create_texture(image.width, image.height, &image.rgba)
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
        let renderer = self.renderer();
        let max = renderer.max_texture_size();
        assert!(
            width <= max && height <= max,
            "crabbyengine: {width} x {height} is larger than this GPU's maximum texture size ({max})"
        );
        let id = renderer.create_texture(width, height, rgba);
        Texture::new(id, width, height)
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
        self.draw_list.len()
    }

    // ---------------------------------------------------------------- timing

    /// Seconds since the previous frame. Multiply speeds by this for smooth,
    /// frame-rate independent movement.
    ///
    /// Clamped to at most 0.25 s so a hiccup never teleports objects.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn update(ctx: &mut Ctx, pos: &mut Vec2) {
    /// pos.x += 200.0 * ctx.dt(); // 200 pixels per second
    /// # }
    /// ```
    pub fn dt(&self) -> f32 {
        self.timing.dt
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
