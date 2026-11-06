//! Rendering: the GPU side (wgpu setup, textures, the instanced sprite pass)
//! lives in `gpu`, behind the `window` feature; without it, `none` stands in.

pub(crate) mod batch;
#[cfg(feature = "window")]
mod gpu;
#[cfg(all(feature = "window", feature = "3d"))]
mod gpu3d;
#[cfg(all(feature = "window", feature = "3d"))]
mod lights;
#[cfg(all(feature = "window", feature = "3d"))]
mod mips;
#[cfg(not(feature = "window"))]
mod none;
#[cfg(all(feature = "window", feature = "3d"))]
mod occlusion;
#[cfg(all(feature = "window", feature = "3d"))]
mod particles;
#[cfg(all(feature = "window", feature = "3d"))]
mod screen;
#[cfg(all(feature = "window", feature = "3d"))]
mod smaa;
#[cfg(all(feature = "window", feature = "3d"))]
mod ssr;
#[cfg(all(feature = "window", feature = "3d"))]
mod taa;
#[cfg(feature = "window")]
mod timer;

#[cfg(feature = "window")]
pub(crate) use gpu::Renderer;
#[cfg(all(feature = "window", feature = "3d"))]
pub(crate) use gpu3d::{check_shader, compose_compute, compose_effect, compose_shader};
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
    /// 3D: of `cpu_ms`, the time spent finding what the camera and the
    /// shadow maps see (culling every model), in ms.
    pub collect_ms: f32,
    /// Of `cpu_ms`, the time turning the frame's draws into GPU command
    /// buffers (recording, on all cores), in ms.
    pub record_ms: f32,
    /// Of `cpu_ms`, the time handing the command buffers to the GPU, in ms.
    pub submit_ms: f32,
    /// 3D models the camera draws (one per drawn [`Model`](crate::Model)).
    pub instances: usize,
    /// Triangles the camera's passes draw (each once, though the depth
    /// prepass draws the solid ones a second time).
    pub triangles: usize,
    /// 3D models drawn into the shadow maps, all cascades together.
    pub shadow_instances: usize,
    /// Triangles drawn into the shadow maps, all cascades together.
    pub shadow_triangles: usize,
    /// 3D models in the camera's view that weren't drawn because something
    /// hid them (see `Camera3D::occlusion_culling`).
    pub occluded: usize,
}

/// What happened when we tried to draw a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameStatus {
    Presented,
    /// The window is hidden or minimized; try again later.
    Occluded,
    /// Transient problem (timeout, outdated surface); just try the next frame.
    Skipped,
    /// The window couldn't take the frame, so it was drawn offscreen
    /// instead (benchmark runs, which must measure every frame).
    Offscreen,
}

/// A captured frame: RGBA8 sRGB pixels, row by row from the top.
pub(crate) struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[cfg(all(test, feature = "window", feature = "3d"))]
mod atlas_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod buffer_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod contact_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod coverage_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod effect_shadow_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod far_terrain_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod light_shadow_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod lights_bench;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod lod_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod occlusion_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod shafts_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod skin_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod ssr_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod text3d_render_test;

#[cfg(all(test, feature = "window", feature = "3d"))]
mod ao_aa_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod taa_render_test;

#[cfg(all(test, feature = "window", feature = "3d"))]
mod environment_render_test;

#[cfg(all(test, feature = "window", feature = "3d"))]
mod decal_render_test;

#[cfg(all(feature = "window", feature = "3d"))]
mod environment;

#[cfg(all(test, feature = "window", feature = "3d"))]
mod quality_render_test;
