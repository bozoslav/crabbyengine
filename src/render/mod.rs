//! Rendering: the GPU side (wgpu setup, textures, the instanced sprite pass)
//! lives in `gpu`, behind the `window` feature; without it, `none` stands in.

pub(crate) mod batch;
pub(crate) mod capabilities;
#[cfg(all(feature = "window", feature = "3d"))]
mod gi;
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
mod pipeline_stats;
#[cfg(all(feature = "window", feature = "3d"))]
mod screen;
#[cfg(all(feature = "window", feature = "3d"))]
mod shader_diagnostics;
#[cfg(all(feature = "window", feature = "3d"))]
mod smaa;
#[cfg(all(feature = "window", feature = "3d"))]
mod ssr;
#[cfg(all(feature = "window", feature = "3d"))]
mod taa;
#[cfg(feature = "window")]
mod timer;
#[cfg(feature = "window")]
pub(crate) mod traffic;

pub use capabilities::{GpuAdapter, GpuCapabilities, GpuFeature, GpuFeatureStatus, GpuLimits, GpuTier};
#[cfg(feature = "window")]
pub(crate) use gpu::Renderer;
#[cfg(all(feature = "window", feature = "3d"))]
pub(crate) use gpu3d::{
    COMPUTE_VARIANT, EFFECT_VARIANT, FULL_MATERIAL_VARIANT, compose_compute, compose_effect, compose_shader,
};
#[cfg(not(feature = "window"))]
pub(crate) use none::Renderer;
pub use pipeline_stats::PipelineStats;
#[cfg(all(feature = "window", feature = "3d"))]
pub(crate) use shader_diagnostics::{diagnose, without_location};

/// What drawing the last frame cost, from [`Perf::render_stats`](crate::Perf::render_stats).
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
    /// 3D pipeline variants built while drawing (crossfading level-of-detail
    /// models, buffer-driven instances), each the first time a frame needed
    /// it. Each one costs several milliseconds inside its frame; call
    /// [`Graphics::warm_render_pipelines`](crate::Graphics::warm_render_pipelines)
    /// while loading to build them up front. Always 0 without the `3d` feature.
    pub pipelines_built: usize,
    /// What the renderer sent to the GPU since the frame before: bytes
    /// uploaded, by kind, and buffers created. In a scene where nothing
    /// changes, only a few small per-frame uniforms should be uploaded and
    /// no buffers created.
    pub uploads: GpuUploads,
}

/// Bytes the renderer uploaded to the GPU, by kind, and the buffers it
/// created, over one frame: [`RenderStats::uploads`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuUploads {
    /// 3D models' per-frame records (transforms and colors), for the
    /// camera, target cameras and probe views.
    pub instances: u64,
    /// 2D sprites' records.
    pub sprites: u64,
    /// Instance batches (`Ctx::set_instance_batch`) and their culling data.
    pub static_batches: u64,
    /// Small per-frame uniforms: cameras, post-processing and effect settings.
    pub uniforms: u64,
    /// Lights and decals.
    pub lights: u64,
    /// Material parameters.
    pub materials: u64,
    /// Skinned models: joint matrices and weights.
    pub skins: u64,
    /// Mesh vertices and indices, and GPU buffers the game made.
    pub meshes: u64,
    /// Texture pixels.
    pub textures: u64,
    /// Everything else: particles, 3D text, occlusion grids.
    pub other: u64,
    /// Buffers created (a buffer outgrown is replaced by a new one).
    pub buffers_created: u32,
    /// The bytes of the buffers created.
    pub buffer_bytes_created: u64,
}

impl GpuUploads {
    pub(crate) const ZERO: Self = Self {
        instances: 0,
        sprites: 0,
        static_batches: 0,
        uniforms: 0,
        lights: 0,
        materials: 0,
        skins: 0,
        meshes: 0,
        textures: 0,
        other: 0,
        buffers_created: 0,
        buffer_bytes_created: 0,
    };

    /// The counts as JSON, for benchmark reports and the control endpoint.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "bytes": self.bytes(),
            "instances": self.instances,
            "sprites": self.sprites,
            "static_batches": self.static_batches,
            "uniforms": self.uniforms,
            "lights": self.lights,
            "materials": self.materials,
            "skins": self.skins,
            "meshes": self.meshes,
            "textures": self.textures,
            "other": self.other,
            "buffers_created": self.buffers_created,
            "buffer_bytes_created": self.buffer_bytes_created,
        })
    }

    /// Every byte uploaded, all kinds together.
    pub fn bytes(&self) -> u64 {
        self.instances
            + self.sprites
            + self.static_batches
            + self.uniforms
            + self.lights
            + self.materials
            + self.skins
            + self.meshes
            + self.textures
            + self.other
    }
}

/// Renderer facts behind the render-pass inspector (`perf.get`'s
/// `render_inspector`) that [`RenderStats`] doesn't carry. Read once per
/// drawn frame; a plain copy, so it costs nothing when nobody asks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RenderDetail {
    /// The device has timestamp queries, so GPU times can be measured.
    pub gpu_timestamps: bool,
    /// Frames whose GPU timestamps have come back so far.
    pub gpu_frames_timed: u64,
    /// CPU time preparing the 3D frame (collecting models and uploading
    /// their data) in ms; includes `RenderStats::collect_ms`.
    pub prepare_ms: f32,
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
mod draw_order_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod effect_shadow_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod far_terrain_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod gi_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod handle_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod light_shadow_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod lights_bench;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod lod_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod model_material_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod occlusion_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod render_test_support;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod shadow_cache_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod shafts_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod single_sample_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod skin_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod ssr_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod target_effect_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod text3d_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod upload_render_test;
#[cfg(all(test, feature = "window", feature = "3d"))]
mod variant_render_test;

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

#[cfg(all(test, feature = "window", feature = "3d"))]
mod residency_render_test;
