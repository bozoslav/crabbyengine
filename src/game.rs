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

use crate::ctx::Ctx;
use crate::input::{Key, MouseButton};
use crate::render::{FrameStatus, Renderer};
use crate::time::{Clock, DEFAULT_FIXED_HZ};

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
pub struct Game {
    title: String,
    width: u32,
    height: u32,
    fixed_hz: u32,
    vsync: bool,
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
            vsync: true,
        }
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

    /// Opens the window and runs the game until it is closed.
    ///
    /// `setup` runs once, after the window exists, and returns the closure
    /// that runs every frame. Variables created in `setup` and moved into that
    /// closure are your game state; no structs or traits required.
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
        U: FnMut(&mut Ctx) + 'static,
    {
        let event_loop = EventLoop::new()
            .unwrap_or_else(|e| panic!("crabbyengine: could not start the event loop: {e}"));

        let clock = Clock::new(self.fixed_hz);
        let ctx = Ctx::new(
            clock.fixed_dt(),
            Vec2::new(self.width as f32, self.height as f32),
        );
        let mut app = App {
            config: self,
            setup: Some(setup),
            update: None,
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

struct App<S, U> {
    config: Game,
    setup: Option<S>,
    update: Option<U>,
    window: Option<Arc<Window>>,
    ctx: Ctx,
    clock: Clock,
    last_frame: Instant,
    /// Window hidden/minimized: stop drawing, poll slowly instead of spinning.
    occluded: bool,
}

impl<S, U> App<S, U>
where
    S: FnOnce(&mut Ctx) -> U,
    U: FnMut(&mut Ctx),
{
    fn logical_window_size(window: &Window) -> Vec2 {
        let size: LogicalSize<f32> = window.inner_size().to_logical(window.scale_factor());
        Vec2::new(size.width, size.height)
    }

    fn run_frame(&mut self, event_loop: &ActiveEventLoop) {
        let Some(update) = self.update.as_mut() else {
            return;
        };

        let now = Instant::now();
        let frame_dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        self.ctx.begin_frame(self.clock.advance(frame_dt));
        update(&mut self.ctx);
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

impl<S, U> ApplicationHandler for App<S, U>
where
    S: FnOnce(&mut Ctx) -> U,
    U: FnMut(&mut Ctx),
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

        if let Some(setup) = self.setup.take() {
            self.update = Some(setup(&mut self.ctx));
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
                    self.ctx
                        .input
                        .set_key(key, event.state == ElementState::Pressed);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = MouseButton::from_winit(button) {
                    self.ctx
                        .input
                        .set_mouse_button(button, state == ElementState::Pressed);
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
