//! [`Ctx`]: everything game code talks to each frame.

use glam::Vec2;

use crate::input::{Input, Key, MouseButton};
use crate::time::FrameTiming;

/// The game's handle to the engine, passed to your update closure every frame.
///
/// Use it to read input, timing and window info, and (from M2 on) to draw.
pub struct Ctx {
    pub(crate) input: Input,
    timing: FrameTiming,
    fixed_dt: f32,
    time: f64,
    frame: u64,
    fps: f32,
    window_size: Vec2,
    quit_requested: bool,
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
        }
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
    fn quit_sets_flag() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ZERO);
        assert!(!ctx.quit_requested());
        ctx.quit();
        assert!(ctx.quit_requested());
    }
}
