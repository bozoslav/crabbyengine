//! The windowed game loop (winit + wgpu), behind the `window` feature.

use std::sync::Arc;
use std::time::Instant;

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use super::*;
use crate::input::{Key, MouseButton};
use crate::render::{FrameStatus, Renderer};

/// `Game::run`: opens the window and runs until it closes.
pub(super) fn run<S, U>(game: Game, setup: S)
where
    S: FnOnce(&mut Ctx) -> U,
    U: SetupResult,
{
    #[cfg(feature = "profiling")]
    let _puffin_server = start_profiler();

    let event_loop = EventLoop::new().unwrap_or_else(|e| panic!("crabbyengine: could not start the event loop: {e}"));

    let clock = Clock::new(game.fixed_hz);
    let mut ctx = Ctx::new(clock.fixed_dt(), Vec2::new(game.width as f32, game.height as f32));
    ctx.seed = game.seed;
    if let Some(cell) = game.spatial_cell {
        ctx.spatial = crate::SpatialGrid::new(cell);
    }
    let control = game.start_control(&mut ctx, false);
    let mut app = App {
        control,
        config: game,
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
                    id: next_system_id(),
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
