//! Rendering: the GPU side (wgpu setup, textures, the instanced sprite pass)
//! lives in `gpu`, behind the `window` feature; without it, `none` stands in.

pub(crate) mod batch;
#[cfg(feature = "window")]
mod gpu;
#[cfg(all(feature = "window", feature = "3d"))]
mod gpu3d;
#[cfg(all(feature = "window", feature = "3d"))]
mod mips;
#[cfg(not(feature = "window"))]
mod none;
#[cfg(feature = "window")]
mod timer;

#[cfg(feature = "window")]
pub(crate) use gpu::Renderer;
#[cfg(all(feature = "window", feature = "3d"))]
pub(crate) use gpu3d::{check_shader, compose_shader};
#[cfg(not(feature = "window"))]
pub(crate) use none::Renderer;

/// What drawing the last frame cost, from [`Ctx::render_stats`](crate::Ctx::render_stats).
///
/// Compare `cpu_ms` and `gpu_ms` with the frame time to see what limits the
/// frame rate: if `gpu_ms` is close to the frame time, the GPU is the
/// bottleneck (too many pixels or sprites); if neither is, the time goes
/// into your systems or into waiting for the display.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RenderStats {
    /// Sprites drawn.
    pub sprites: usize,
    /// GPU draw calls: one per run of sprites sharing a texture, and with
    /// the `3d` feature, one per mesh part drawn (all its models at once).
    pub draw_calls: usize,
    /// CPU time spent sorting, batching, uploading and submitting, in ms.
    pub cpu_ms: f32,
    /// GPU time of the frame in ms (smoothed), from the start of its first
    /// pass to the end of its last, 3D and shadows included, measured with
    /// timestamp queries. It includes any time the GPU waits between
    /// passes, so it's an upper bound. `None` when the GPU doesn't support them, and in headless runs.
    pub gpu_ms: Option<f32>,
}

/// What happened when we tried to draw a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameStatus {
    Presented,
    /// The window is hidden or minimized; try again later.
    Occluded,
    /// Transient problem (timeout, outdated surface); just try the next frame.
    Skipped,
}

/// A captured frame: RGBA8 sRGB pixels, row by row from the top.
pub(crate) struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
