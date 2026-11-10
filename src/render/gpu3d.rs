//! The 3D passes: instanced meshes with a depth buffer and 4x multisampling,
//! drawn before sprites. Made the first time a game creates a mesh or
//! material.
//!
//! The frame, in order:
//! 0. **sky texture**: with an atmosphere, the sky in every direction;
//! 1. **shadows**: one pass per cascade of the sun's shadow map;
//! 2. **depth prepass**: the nearest depth of every pixel, so the next pass
//!    lights each pixel once, and later passes can read the depth; GTAO and
//!    screen-space reflections (see `screen.rs`, `ssr.rs`) trace it here;
//! 3. **opaque**: lit models into an HDR (16-bit float) target, keeping only
//!    the fragments the prepass found nearest, then the sky behind them and
//!    the transparent models over them;
//! 4. **bloom**: bright light is blurred through a chain of ever smaller
//!    images and back up (skipped when bloom is off);
//! 5. **post**: exposure, bloom, color grading and tone mapping, onto the
//!    screen.
//!
//! With temporal antialiasing, the camera also jitters by a sub-pixel amount
//! each frame, the lit frame goes into its own image, models that moved
//! since the last frame are drawn again for their motion vectors (after the
//! depth prepass, shared with temporal AO), and the post pass starts by resolving the frame with the
//! history into the HDR target (see `taa.rs`).
//!
//! Sprites and UI are drawn on top afterwards, without tone mapping.

mod draw_order;
use crate::render::traffic::{self, Counted, MAX_WRITES, MIRRORED, Mirror, NewBuffer, Synced, Upload};
mod instances;
mod passes;
mod pipeline_cache;
mod shader_checks;
mod shadow_cache;
mod variants;
use super::capabilities::GpuLimits;
pub(crate) use draw_order::StateChanges;
use instances::{Culling, StaticBatch};
use pipeline_cache::{Features, MaterialCache, ModuleKey, TargetKey};
pub(crate) use shader_checks::LateShaderError;
use shader_checks::{Swap, end_scope, rejected};
use shadow_cache::ShadowCache;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::Ordering;
pub(crate) use variants::{Instancing, Pass, PipelineRole, ShaderVariant, Sides};

use glam::{Mat4, Vec3};

use super::gpu::GpuTexture;
use super::lights::LightTextures;
use super::taa::{self, Taa};
use super::timer::GpuTimer;

// Native command recording uses rayon; browser GPU handles stay on one thread.
#[cfg(not(target_arch = "wasm32"))]
type PassTimer<'a, 't> = dyn Fn(&'static str) -> Option<wgpu::RenderPassTimestampWrites<'t>> + Sync + 'a;
#[cfg(target_arch = "wasm32")]
type PassTimer<'a, 't> = dyn Fn(&'static str) -> Option<wgpu::RenderPassTimestampWrites<'t>> + 'a;

/// A part of a frame, recorded into a command buffer of its own (see
/// `Renderer3D::draw`).
#[derive(Clone, Copy, Debug)]
enum Part {
    /// Compute passes and the sky texture.
    Prelude,
    /// A shadow cascade's runs `first..end`: big cascades are drawn in
    /// several passes (the first clears the map), recorded on several cores.
    Shadow {
        cascade: usize,
        first: u32,
        end: u32,
    },
    /// Every target camera.
    Views,
    /// Global illumination's probe views and their update.
    Gi,
    Prepass,
    /// The opaque pass, with the transparent surfaces when none reads the scene.
    Opaque,
    /// A copy of the opaque scene, then the transparent surfaces.
    Transparent,
    /// Screen effects, bloom and tone mapping.
    Post,
}

impl Part {
    /// Its pass in [`passes::FRAME_PASSES`]: what it reads and writes.
    fn pass(self) -> usize {
        match self {
            Part::Prelude => passes::PRELUDE,
            Part::Shadow { .. } => passes::SHADOW,
            Part::Views => passes::VIEWS,
            Part::Gi => passes::GI,
            Part::Prepass => passes::PREPASS,
            Part::Opaque => passes::OPAQUE,
            Part::Transparent => passes::TRANSPARENT,
            Part::Post => passes::POST,
        }
    }

    fn label(self) -> &'static str {
        passes::FRAME_PASSES[self.pass()].name
    }
}

/// What every part of a frame is recorded with.
struct FrameParts<'a, 't> {
    view: &'a wgpu::TextureView,
    clear: wgpu::Color,
    textures: &'a [GpuTexture],
    targets: &'a Targets,
    split: bool,
    time: &'a PassTimer<'a, 't>,
}

/// Draws per shadow pass before a cascade is split into more passes, each
/// recorded on its own core. A pass costs little GPU time on a depth-only
/// map; recording a few hundred draws costs a fraction of a millisecond.
const SHADOW_DRAWS_PER_PASS: usize = 300;

/// GPU timer names of the shadow cascades' passes.
const SHADOW_PASSES: [&str; 8] = [
    "shadows 0",
    "shadows 1",
    "shadows 2",
    "shadows 3",
    "shadows 4",
    "shadows 5",
    "shadows 6",
    "shadows 7",
];
use crate::Material;
use crate::three::gi::GI_SHADOW_SIZE;
use crate::three::motion::MovedInstance;
use crate::three::{
    FADED, GpuMaterialDesc, INSTANCE_BRIDGE, Instance3D, MAX_CASCADES, MAX_LIGHT_SHADOW_VIEWS, MAX_SHADOW_VIEWS,
    MaterialTextures, MeshData, PostUniform, linear, scaled_size, shadow_views,
};
use crate::three::{Frame3D, MaterialOverride, Scene3D};
use crate::three::{ShaderDiagnostic, ShaderKind};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The lit frame before tone mapping: light brighter than white survives.
pub(super) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Samples per pixel: smooth edges.
/// Whether this GPU can multisample the 3D frame's color and depth `samples`
/// times. 4 always works (WebGPU guarantees it).
pub(crate) fn multisamples(adapter: &wgpu::Adapter, samples: u32) -> bool {
    [DEPTH_FORMAT, HDR_FORMAT].iter().all(|&f| {
        adapter
            .get_texture_format_features(f)
            .flags
            .sample_count_supported(samples)
    })
}

/// Whether `backend` is the browser's WebGL2, which wgpu can't give
/// multisampled textures (`texStorage2DMultisample` is missing from WebGL2, and
/// glow's browser binding panics on it). The 3D frame then uses one sample.
pub(crate) fn browser_gl_has_no_multisampled_textures(backend: wgpu::Backend) -> bool {
    cfg!(target_arch = "wasm32") && backend == wgpu::Backend::Gl
}

/// The shader source for a frame of `samples` per pixel. Sources are written
/// for the multisampled frame; with one sample the scene depth the mesh
/// prelude declares is an ordinary depth texture (`textureLoad` takes a mip
/// level there instead of a sample index, and the engine's shaders pass 0 for
/// both). A game's own shader must read the scene depth at index 0 (or through
/// `scene_position`), the only value that means the same in both: the book's
/// shader section says so.
fn for_samples(source: &str, samples: u32) -> std::borrow::Cow<'_, str> {
    if samples > 1 {
        std::borrow::Cow::Borrowed(source)
    } else {
        std::borrow::Cow::Owned(source.replace("texture_depth_multisampled_2d", "texture_depth_2d"))
    }
}

/// The frame's camera and light, as the shader reads them.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    /// World to each shadow cascade's clip space.
    sun_view_proj: [[[f32; 4]; 4]; MAX_CASCADES],
    /// In the shadow pass: the cascade being drawn.
    caster_view_proj: [[f32; 4]; 4],
    camera: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky: [f32; 4],
    ground: [f32; 4],
    /// x: how many cascades (0: no shadows), y: one shadow-map texel in
    /// uv, z: how far the shadow lookup moves along the normal (in
    /// texels), w: where shadows fade out (distance from the camera).
    shadow: [f32; 4],
    /// Each cascade's texel size, in world units.
    cascade_texel: [f32; 4],
    /// x: seconds since the game started, y: seconds since the last frame.
    time: [f32; 4],
    inv_view_proj: [[f32; 4]; 4],
    /// Width and height in pixels, then their inverses.
    screen: [f32; 4],
    /// x: a physical sky (1) or not, y: haze, z: cos of the sun's radius,
    /// w: how bright the stars are.
    atmosphere: [f32; 4],
    /// rgb: the sunlight the sky is lit by (before the air), w: camera
    /// height in km.
    sky_light: [f32; 4],
    /// x: fog density per unit, y: fog height, z: 1 / falloff.
    fog: [f32; 4],
    fog_color: [f32; 4],
    /// Light at night.
    night: [f32; 4],
    /// x: distance fog start, y: its end (0: none), z: sun glow, w: sun focus.
    fog_far: [f32; 4],
    cluster_depth: [f32; 4],
    cluster_view_proj: [[f32; 4]; 4],
    /// World to each point and spot light shadow view's clip space.
    light_view_proj: [[[f32; 4]; 4]; MAX_LIGHT_SHADOW_VIEWS],
    /// Each light shadow view's tile in the atlas, in uv: x, y, side.
    light_rect: [[f32; 4]; MAX_LIGHT_SHADOW_VIEWS],
    /// x: how many light shadow views, y: one atlas texel in uv, z: how
    /// far the lookup moves along the normal (in texels).
    light_shadow: [f32; 4],
    probes: [crate::three::probe::ProbeUniform; 4],
    /// The camera without the temporal antialiasing jitter, and last
    /// frame's (the same without TAA).
    unjittered_view_proj: [[f32; 4]; 4],
    previous_view_proj: [[f32; 4]; 4],
    /// xy: this frame's jitter in pixels.
    jitter: [f32; 4],
    /// Global illumination's probe grid (see `gi.wgsl`); all zero without.
    gi: [[f32; 4]; 3],
}

/// Writes a frame uniform at `at` in `buffer`, whose contents there
/// `mirror` knows: only the vec4s that changed since the last frame (the
/// time, the camera when it moved), not the whole uniform for every view.
fn sync_frame(queue: &wgpu::Queue, buffer: &wgpu::Buffer, at: u64, mirror: &mut Mirror, uniform: &FrameUniform) {
    mirror.upload(queue, Upload::Uniforms, buffer, at, bytemuck::bytes_of(uniform));
}

#[path = "skinning.rs"]
mod skinning;

/// One corner of a mesh on the GPU.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    /// Linear RGBA, 0..255.
    color: [u8; 4],
    /// The second texture coordinate set.
    uv1: [f32; 2],
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 8 => Unorm8x4, 9 => Float32x2];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, // model matrix
    7 => Float32x4, // tint
];

/// A moved model for the motion pass: the instance, then last frame's model
/// matrix.
const MOVED_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, // model matrix
    7 => Float32x4, // tint
    10 => Float32x4, 11 => Float32x4, 12 => Float32x4, 13 => Float32x4, // last frame's matrix
];

struct GpuMesh {
    /// Vertex and index buffers; `None` once freed (its parts are gone too).
    buffers: Option<(wgpu::Buffer, wgpu::Buffer)>,
    /// Index range and material of each part.
    parts: Vec<(Range<u32>, u32)>,
    /// What skinning it needs, if it's skinned.
    skin: Option<skinning::GpuSkin>,
    gpu_buffer: Option<(u32, u32)>,
}

struct StorageBuffer {
    buffer: wgpu::Buffer,
    read: wgpu::BindGroup,
    write: wgpu::BindGroup,
    len: u32,
}
struct GpuMaterial {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bindings: MaterialBindings,
    shader: u32,
    transparent: bool,
    /// Whether its depth passes cut out pixels by alpha ([`Blend::Cutout`]).
    cutout: bool,
    double_sided: bool,
}

// Keep the resource identities as well as the requested handles: mip generation
// and reloads can replace a texture in place. Comparing these fixed arrays does
// not allocate or create views on uniform-only updates.
struct MaterialBindings {
    maps: MaterialTextures,
    array_ids: [Option<u32>; 3],
    textures: [wgpu::Texture; 6],
    arrays: [wgpu::TextureView; 3],
}

/// One material shader's pipelines: a [`Pipelines`] set per
/// [`ShaderVariant`] (see `variants.rs` for the whole list).
struct ShaderPipelines {
    /// [`Instancing::Plain`], one-sided and double-sided: built with the shader.
    sides: [Pipelines; 2],
    /// [`Instancing::Crossfade`], built when first drawn (or all at once by
    /// [`Renderer3D::warm_pipelines`]).
    fade: [std::sync::OnceLock<Pipelines>; 2],
    /// [`Instancing::Shared`]: buffer-driven instances, which all share
    /// their entity's one instance record (a zero-stride instance buffer);
    /// built when first drawn (or by [`Renderer3D::warm_pipelines`]).
    shared: [std::sync::OnceLock<Pipelines>; 2],
    builder: SideBuilder,
    /// The cache's counters: builds, misses, and the sets this one drops.
    counters: Arc<pipeline_cache::Counters>,
    /// Whether shaders draw with it: sets built for code the GPU then
    /// rejected were never used, so dropping them invalidates nothing.
    in_use: bool,
    /// Whether it reads the scene behind it (`scene_color`, `scene_position`,
    /// `scene_distance`): its transparent surfaces then need a copy of it.
    reads_scene: bool,
}

impl ShaderPipelines {
    /// The pipelines of `variant`. One not built yet is built now, and
    /// counted in `built`.
    fn variant(&self, variant: ShaderVariant) -> &Pipelines {
        self.lookup(variant, true)
    }

    /// The pipelines of `variant`, built now if needed; `drawing`: a
    /// frame needs them (a miss), not warming.
    fn lookup(&self, variant: ShaderVariant, drawing: bool) -> &Pipelines {
        let side = variant.sides.index();
        let lazy = match variant.instancing {
            Instancing::Plain => return &self.sides[side],
            Instancing::Crossfade => &self.fade[side],
            Instancing::Shared => &self.shared[side],
        };
        lazy.get_or_init(|| {
            if drawing {
                self.counters.missed();
            }
            self.builder.timed_build(variant, &self.counters)
        })
    }

    /// Whether `variant` is built.
    fn built(&self, variant: ShaderVariant) -> bool {
        let side = variant.sides.index();
        match variant.instancing {
            Instancing::Plain => true,
            Instancing::Crossfade => self.fade[side].get().is_some(),
            Instancing::Shared => self.shared[side].get().is_some(),
        }
    }

    /// Builds every variant not built yet.
    fn warm(&self) {
        for variant in ShaderVariant::ALL {
            self.lookup(variant, false);
        }
    }

    /// How many of its variant sets are built.
    fn built_count(&self) -> usize {
        ShaderVariant::ALL.iter().filter(|&&v| self.built(v)).count()
    }

    /// Render pipelines in each of its sets: one per [`PipelineRole`] some
    /// pass draws with at its sample count.
    fn pipelines_per_set(&self) -> usize {
        PipelineRole::per_set(self.builder.samples > 1)
    }
}

impl Drop for ShaderPipelines {
    /// The last shader using these let go (a reload or a replaced shader):
    /// its built sets are invalidated, no other shader's.
    fn drop(&mut self) {
        if self.in_use {
            self.counters.invalidated(self.built_count() as u64);
        }
    }
}

struct Pipelines {
    prepass: wgpu::RenderPipeline,
    opaque: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    /// The depth passes of cut-out materials, which run the material's
    /// shader for its alpha. Solid ones write depth without any.
    prepass_cutout: wgpu::RenderPipeline,
    shadow_cutout: wgpu::RenderPipeline,
    /// Cut-out materials with multisampling: alpha to coverage in the
    /// prepass (with a color target it doesn't write) and the opaque pass.
    /// `None` without multisampling.
    coverage: Option<[wgpu::RenderPipeline; 2]>,
    /// Solid surfaces lit without a prepass: they write their own depth.
    opaque_solid: wgpu::RenderPipeline,
    /// Motion vectors of moved models, over the opaque pass's depth.
    velocity: wgpu::RenderPipeline,
}

impl Pipelines {
    /// The pipeline of `role`; `None` for the coverage roles without
    /// multisampling.
    fn role(&self, role: PipelineRole) -> Option<&wgpu::RenderPipeline> {
        Some(match role {
            PipelineRole::Prepass => &self.prepass,
            PipelineRole::PrepassCutout => &self.prepass_cutout,
            PipelineRole::PrepassCoverage => &self.coverage.as_ref()?[0],
            PipelineRole::Opaque => &self.opaque,
            PipelineRole::OpaqueCoverage => &self.coverage.as_ref()?[1],
            PipelineRole::OpaqueSolid => &self.opaque_solid,
            PipelineRole::Transparent => &self.transparent,
            PipelineRole::Shadow => &self.shadow,
            PipelineRole::ShadowCutout => &self.shadow_cutout,
            PipelineRole::Velocity => &self.velocity,
        })
    }
}

/// The built-in parts of every material shader.
const MESH_PRELUDE: &str = include_str!("mesh.wgsl");
const MESH_MAIN: &str = include_str!("mesh_main.wgsl");
/// Projected decals in the mesh shader (see `Decal3D`), or the stand-in
/// where the GPU has no texture slot for them.
const DECALS: &str = include_str!("decal.wgsl");
const NO_DECALS: &str = include_str!("decal_off.wgsl");

/// Whether `device` has a texture slot to spare for projected decals.
/// Native devices ask for more than the default when the adapter has it;
/// WebGL2 doesn't, and skips decals.
pub(crate) fn supports_decals(device: &wgpu::Device) -> bool {
    GpuLimits::of(device).decals()
}

/// Global illumination's probes in the mesh shader (see `gi.rs`), or the
/// stand-in where the GPU can't have them.
const GI: &str = include_str!("gi.wgsl");
const NO_GI: &str = include_str!("gi_off.wgsl");

/// Global illumination's probe update pass (see `gi.rs`).
const GI_UPDATE: &str = include_str!("gi_update.wgsl");

/// The probe update shader: the mesh prelude (the frame, the sky's light)
/// and the update.
pub(super) fn gi_update_source() -> String {
    format!("{MESH_PRELUDE}\n{GI_UPDATE}")
}

/// Whether `device` can draw global illumination: compute shaders, and a
/// texture slot past the decals' for the probes (the 18th).
pub(crate) fn supports_gi(device: &wgpu::Device) -> bool {
    GpuLimits::of(device).gi()
}
const EFFECT_PRELUDE: &str = include_str!("effect.wgsl");
const EFFECT_MAIN: &str = include_str!("effect_main.wgsl");
const COMPUTE_PRELUDE: &str = include_str!("compute.wgsl");
const COMPUTE_MAIN: &str = include_str!("compute_main.wgsl");
const SKY: &str = include_str!("sky.wgsl");
/// The sky texture: every direction around the camera.
const SKY_SIZE: (u32, u32) = (256, 128);

/// Whether WGSL `source` defines a function called `name`.
fn defines(source: &str, name: &str) -> bool {
    let mut rest = source;
    while let Some(at) = rest.find("fn") {
        let before_ok = at == 0 || !rest[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let after = rest[at + 2..].trim_start();
        if before_ok && rest[at + 2..].starts_with(char::is_whitespace) && after.starts_with(name) {
            let tail = after[name.len()..].trim_start();
            if tail.starts_with('(') {
                return true;
            }
        }
        rest = &rest[at + 2..];
    }
    false
}

/// Whether a shader reads the scene behind it.
fn reads_scene(user: &str) -> bool {
    ["scene_color", "scene_position", "scene_distance"]
        .iter()
        .any(|f| user.contains(f))
}

/// A whole material shader: the engine's prelude, the game's code (with
/// pass-through `vertex` and `surface` functions when it has none) and the
/// engine's entry points.
pub(crate) fn compose_shader(user: &str) -> String {
    compose_material(user, true, true)
}

/// [`compose_shader`] for a device with (`decals`) or without a texture
/// slot for projected decals (and none for global illumination either).
#[cfg(test)]
pub(crate) fn compose_shader_with(user: &str, decals: bool) -> String {
    compose_material(user, decals, decals)
}

/// [`compose_shader`] for a device with or without the texture slots for
/// projected decals and global illumination.
fn compose_material(user: &str, decals: bool, gi: bool) -> String {
    let decal_code = if decals { DECALS } else { NO_DECALS };
    let gi_code = if gi { GI } else { NO_GI };
    let mut source = String::with_capacity(
        MESH_PRELUDE.len() + decal_code.len() + gi_code.len() + user.len() + MESH_MAIN.len() + 200,
    );
    source.push_str(MESH_PRELUDE);
    source.push_str("@group(3) @binding(0) var<storage, read> items: array<vec4<f32>>;\n@group(3) @binding(1) var<storage, read> static_indices: array<u32>;\n");
    source.push_str(decal_code);
    source.push_str(gi_code);
    source.push_str("\n// ---- the game's shader ----\n");
    source.push_str(user);
    source.push_str("\n// ---- the engine's entry points ----\n");
    if !defines(user, "vertex") {
        source.push_str("fn vertex(v: Vertex) -> Vertex { return v; }\n");
    }
    if !defines(user, "surface") {
        source.push_str("fn surface(s: Surface) -> Surface { return s; }\n");
    }
    source.push_str(&MESH_MAIN.replace("// STATIC_INSTANCE_HELPER", include_str!("gpu3d/instance_vertex.wgsl")));
    source
}

fn compose_shader_storage(user: &str, decals: bool, gi: bool, storage: bool) -> String {
    let source = compose_material(user, decals, gi);
    if storage {
        source
    } else {
        source.replace(
            include_str!("gpu3d/instance_vertex.wgsl"),
            "fn static_instance(v: VertexIn) -> VertexIn { return v; }",
        )
    }
}

/// A whole screen effect shader from the game's part: the mesh prelude
/// (bindings and helpers), `Pixel`, the game's `effect`, the entry points.
pub(crate) fn compose_effect(user: &str) -> String {
    let mut source =
        String::with_capacity(MESH_PRELUDE.len() + EFFECT_PRELUDE.len() + user.len() + EFFECT_MAIN.len() + 100);
    source.push_str(MESH_PRELUDE);
    source.push_str(EFFECT_PRELUDE);
    source.push_str("\n// ---- the game's effect ----\n");
    source.push_str(user);
    source.push_str("\n// ---- the engine's entry points ----\n");
    source.push_str(EFFECT_MAIN);
    source
}

/// A whole compute pass shader from the game's part: the mesh prelude
/// (bindings and helpers), `Cell`, the game's `compute`, the entry point.
pub(crate) fn compose_compute(user: &str) -> String {
    let mut source =
        String::with_capacity(MESH_PRELUDE.len() + COMPUTE_PRELUDE.len() + user.len() + COMPUTE_MAIN.len() + 100);
    source.push_str(MESH_PRELUDE);
    if defines(user, "update") {
        source.push_str("@group(3) @binding(0) var<storage, read_write> items: array<vec4<f32>>;\n");
    } else {
        source.push_str(COMPUTE_PRELUDE);
    }
    source.push_str("\n// ---- the game's compute pass ----\n");
    source.push_str(user);
    source.push_str("\n// ---- the engine's entry point ----\n");
    if defines(user, "update") {
        source.push_str("@compute @workgroup_size(64) fn cs_compute(@builtin(global_invocation_id) id: vec3<u32>) { if id.x < arrayLength(&items) { update(id.x); } }");
    } else {
        source.push_str(COMPUTE_MAIN);
    }
    source
}

/// Checks a composed shader, with an error message that points at the
/// line, as WGSL compilers print them.
#[cfg(test)]
pub(crate) fn check_shader(name: &str, source: &str) -> Result<(), String> {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string_with_path(source, name))?;
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .map_err(|e| e.emit_to_string_with_path(source, name))?;
    Ok(())
}

/// The variant the engine checks every material shader against first:
/// every optional part on.
pub(crate) const FULL_MATERIAL_VARIANT: &str = "material shader with decals, global illumination and instance storage";
pub(crate) const EFFECT_VARIANT: &str = "screen effect";
pub(crate) const COMPUTE_VARIANT: &str = "compute pass";

/// The material shader variant this device compiles.
fn material_variant(decals: bool, gi: bool, storage: bool) -> String {
    let on = |b: bool| if b { "on" } else { "off" };
    format!(
        "material shader on this GPU: decals {}, global illumination {}, instance storage {}",
        on(decals),
        on(gi),
        on(storage)
    )
}

/// Whether `device` runs compute passes: WebGL2 has no compute shaders
/// or storage textures.
fn can_compute(device: &wgpu::Device) -> bool {
    GpuLimits::of(device).compute()
}

/// Most bloom steps: the smallest image is 1/128 of the screen.
const BLOOM_MIPS: u32 = 7;
/// How much each bloom step keeps of the wider glow below it.
const BLOOM_SCATTER: f64 = 0.7;

/// Scene-sized targets: the window's size times `Post::render_scale`.
struct Targets {
    ao: Option<super::screen::AoTargets>,
    ssr: Option<super::ssr::SsrTargets>,
    /// With GTAO or screen-space reflections: what the opaque pass reads
    /// them by (its scene bind group).
    opaque_scene: Option<wgpu::BindGroup>,
    /// With temporal antialiasing: the lit frame's image, history and motion.
    taa: Option<super::taa::TaaTargets>,
    velocity: Option<super::taa::VelocityTargets>,
    ldr: Option<wgpu::TextureView>,
    aa_group: Option<wgpu::BindGroup>,
    /// With a render scale other than 1: the tone mapped (and antialiased)
    /// frame at the scene's size, and the bind group the scaling pass reads
    /// it by. The scaling pass then draws it into the window-sized output.
    upscale: Option<wgpu::TextureView>,
    upscale_group: Option<wgpu::BindGroup>,
    /// Multisampled HDR color, resolved into `hdr`.
    color: wgpu::TextureView,
    hdr: wgpu::TextureView,
    hdr_texture: wgpu::Texture,
    /// A copy of the opaque scene, for transparent shaders that read it.
    scene_texture: wgpu::Texture,
    /// The scene bind group of the transparent pass: `scene_texture` and `depth`.
    scene: wgpu::BindGroup,
    /// `scene_texture` as a render target: screen effects swap between it
    /// and `hdr` (ping-pong), each reading one and writing the other.
    scene_view: wgpu::TextureView,
    /// The scene bind group reading `hdr` (and `depth`), for effects.
    scene_of_hdr: wgpu::BindGroup,
    /// `post` and `bloom_prefilter`, reading `scene_texture`: for when an
    /// odd number of effects ran and the frame ended up there.
    post_of_scene: wgpu::BindGroup,
    bloom_prefilter_of_scene: wgpu::BindGroup,
    /// Multisampled depth, written by the prepass.
    depth: wgpu::TextureView,
    /// The post pass's input: `hdr` and the bloom.
    post: wgpu::BindGroup,
    /// Bloom: render target of each mip, and the bind groups of its
    /// steps: the first downsample (reading `hdr`), the other downsamples
    /// (reading the mip above) and the upsamples (reading the mip below).
    bloom_mips: Vec<wgpu::TextureView>,
    bloom_prefilter: wgpu::BindGroup,
    bloom_down: Vec<wgpu::BindGroup>,
    bloom_up: Vec<wgpu::BindGroup>,
    size: (u32, u32),
}

impl Targets {
    /// What the 3D passes resolve the lit frame into: `hdr`, or with
    /// temporal antialiasing its input image (the resolve then writes `hdr`).
    fn lit_view(&self, taa: bool) -> &wgpu::TextureView {
        match &self.taa {
            Some(t) if taa => &t.input_view,
            _ => &self.hdr,
        }
    }

    fn lit_texture(&self, taa: bool) -> &wgpu::Texture {
        match &self.taa {
            Some(t) if taa => &t.input,
            _ => &self.hdr_texture,
        }
    }
}

/// (mesh, first instance, count): one instanced draw of a mesh.
type Run = (u32, u32, u32);

/// The shared parts of every frame bind group: the shadow map and the sky.
struct FrameSources {
    layout: wgpu::BindGroupLayout,
    shadow_view: wgpu::TextureView,
    /// Point and spot lights' shadow views (a 1x1 stand-in until a light
    /// first casts shadows).
    light_shadow_view: wgpu::TextureView,
    shadow_sampler: wgpu::Sampler,
    sky_view: wgpu::TextureView,
    sky_sampler: wgpu::Sampler,
    /// Whether the layout has the decal images (see `supports_decals`).
    decals: bool,
    /// The decal images: a stand-in until the game sets some.
    decal_view: wgpu::TextureView,
    decal_sampler: wgpu::Sampler,
    /// Whether the layout has global illumination's probes (see
    /// `supports_gi`), and their atlas: a stand-in while it's off.
    gi: bool,
    gi_view: wgpu::TextureView,
}

impl FrameSources {
    fn bind_group(&self, device: &wgpu::Device, frame: &wgpu::Buffer, lights: &LightTextures) -> wgpu::BindGroup {
        self.bind_group_shadowed(device, frame, lights, &self.shadow_view)
    }

    /// [`bind_group`](Self::bind_group) with another sun shadow map (global
    /// illumination's probe views have their own).
    fn bind_group_shadowed(
        &self,
        device: &wgpu::Device,
        frame: &wgpu::Buffer,
        lights: &LightTextures,
        shadow: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(shadow),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&self.sky_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&self.sky_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&lights.records_view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&lights.indices_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&self.light_shadow_view),
            },
        ];
        if self.decals {
            entries.extend(decal_entries(&self.decal_view, &self.decal_sampler));
        }
        if self.gi {
            entries.push(wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(&self.gi_view),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d frame"),
            layout: &self.layout,
            entries: &entries,
        })
    }
}

/// The decal images' bindings in a frame bind group.
fn decal_entries<'a>(view: &'a wgpu::TextureView, sampler: &'a wgpu::Sampler) -> [wgpu::BindGroupEntry<'a>; 2] {
    [
        wgpu::BindGroupEntry {
            binding: 8,
            resource: wgpu::BindingResource::TextureView(view),
        },
        wgpu::BindGroupEntry {
            binding: 9,
            resource: wgpu::BindingResource::Sampler(sampler),
        },
    ]
}

/// A render target a target camera draws into: multisampled color and
/// depth of its size, resolved into the target's texture, and its own
/// frame uniform.
struct ViewTarget {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::TextureView,
    frame_buffer: wgpu::Buffer,
    /// What `frame_buffer` holds (see `sync_frame`).
    frame_mirror: Mirror,
    frame_bind_group: wgpu::BindGroup,
    lights: LightTextures,
}

/// A render target compute passes run on: the scratch texture each pass
/// writes (then copied back into the target) and its bind group.
struct ComputeState {
    target: wgpu::Texture,
    scratch: wgpu::Texture,
    group: wgpu::BindGroup,
}

/// What `Renderer3D::prepare` needs for a frame.
pub(crate) struct Prepare3D<'a> {
    pub scene: &'a Scene3D<'a>,
    /// The lookup table's texture and its size, when there is one.
    pub lut: Option<(u32, u32)>,
    pub size: (u32, u32),
}

pub(crate) struct Renderer3D {
    /// Samples per pixel of the frame and target cameras (2 or 4).
    samples: u32,
    /// Whether solid surfaces go through the depth prepass too. GPUs that
    /// draw in tiles (Apple's) drop hidden solid fragments themselves, so
    /// there they skip it and write their depth while lit; only cut-out
    /// ones, which those GPUs can't drop early, are laid down first.
    solid_prepass: bool,
    /// Pipeline layouts of the material shaders: lit passes, and the
    /// shadow pass (which can't read the shadow map it draws into).
    layout: wgpu::PipelineLayout,
    shadow_pipeline_layout: wgpu::PipelineLayout,
    /// Each shader's pipelines; 0 is the built-in shader. Shaders with the
    /// same code share one set (see `pipeline_cache.rs`).
    shaders: Vec<Arc<ShaderPipelines>>,
    /// The sets by key, and the counters behind `PipelineStats`.
    cache: MaterialCache,
    /// Whether every shader builds all its variants up front (after
    /// [`Renderer3D::warm_pipelines`]), instead of when first drawn.
    warm_variants: bool,
    material_sampler: wgpu::Sampler,
    scene_layout: wgpu::BindGroupLayout,
    /// The scene bind group of passes that can't see the scene.
    no_scene: wgpu::BindGroup,
    post_pipeline: wgpu::RenderPipeline,
    fxaa_pipeline: wgpu::RenderPipeline,
    /// Scales the scene-sized LDR frame to the window (see `Targets::upscale`).
    upscale_pipeline: wgpu::RenderPipeline,
    /// The 1x1 light shadow map frame bind groups use without light shadows.
    no_light_shadow: wgpu::TextureView,
    output_format: wgpu::TextureFormat,
    aa: crate::AntiAliasing,
    smaa: Option<super::smaa::Smaa>,
    /// Temporal antialiasing: the resolve pass (made when first used), and
    /// whether this device can run it (WebGL2 can't: FXAA stands in).
    taa: Option<super::taa::Taa>,
    taa_supported: bool,
    /// Whether this frame is resolved by it.
    taa_on: bool,
    velocity_on: bool,
    /// Frames drawn with it in a row (the jitter's position).
    taa_frame: u32,
    /// Which history image the next frame reads, and the one this frame does.
    taa_read: usize,
    taa_frame_read: usize,
    /// The last frame's unjittered camera, when its image is in the history.
    taa_previous: Option<Mat4>,
    /// The unjittered camera of the last frame drawn: camera cuts are
    /// measured from it.
    history_camera: Option<Mat4>,
    /// Why this frame dropped the temporal history, if it did.
    #[cfg(test)]
    history_reset: HistoryReset,
    /// Models that moved: their instances (see `MovedInstance`) and runs.
    /// Written a vec4 at a time: a model that only moved (and didn't turn)
    /// changes two of its record's nine, the rest are as the last frame's.
    moved: Synced,
    moved_runs: Vec<Run>,
    ao: Option<super::screen::Ao>,
    ao_enabled: bool,
    /// Screen-space reflections: the passes (made when first used), and
    /// whether this frame has them.
    ssr: Option<super::ssr::Ssr>,
    ssr_on: bool,
    /// Global illumination: the probes (made when first used), the
    /// atlas's stand-in while it's off, what the probes' views draw (into
    /// `instances`), and each view's slice of `gi_face_runs`.
    gi: Option<super::gi::Gi>,
    no_gi: wgpu::TextureView,
    gi_runs: Vec<Run>,
    gi_face_runs: Vec<Run>,
    gi_faces: Vec<(u32, u32)>,
    dummy_depth: wgpu::TextureView,
    /// Whether a texture made for sprites can also be read through its
    /// linear (non-sRGB) format: not on WebGL2, which has no view formats.
    linear_views: bool,
    post_layout: wgpu::BindGroupLayout,
    bloom_prefilter: wgpu::RenderPipeline,
    bloom_down: wgpu::RenderPipeline,
    bloom_up: wgpu::RenderPipeline,
    post_buffer: wgpu::Buffer,
    post_mirror: Mirror,
    linear_clamp: wgpu::Sampler,
    /// Bound as the bloom input of the bloom steps themselves.
    black: wgpu::TextureView,
    /// This frame's lookup table texture (or white), and whether bloom runs.
    lut: u32,
    bloom: bool,
    frame_buffer: wgpu::Buffer,
    /// What `frame_buffer` holds (see `sync_frame`).
    frame_mirror: Mirror,
    /// The physical sky: made into `sky_view` each frame, then drawn behind
    /// everything. Only with an atmosphere.
    sky_view: wgpu::TextureView,
    sky_texture: wgpu::Texture,
    environment: super::environment::Environment,
    sky_lut: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    atmosphere: bool,
    frame_bind_group: wgpu::BindGroup,
    lights: LightTextures,
    /// What frame bind groups are made of, for each target camera's own.
    frame_sources: FrameSources,
    /// The decal texture array the frame bind groups hold (`None`: the stand-in).
    decal_array: Option<u32>,
    no_decal_view: wgpu::TextureView,
    /// Every render target's texture, by texture id.
    render_targets: Vec<(u32, wgpu::Texture)>,
    /// What target cameras need of the render targets they draw into,
    /// made the first time one does, by texture id.
    view_targets: Vec<(u32, ViewTarget)>,
    /// Compute pass pipelines, by compute id, and their layouts.
    compute_pipelines: Vec<Option<wgpu::ComputePipeline>>,
    pub(super) static_batches: Vec<Option<StaticBatch>>,
    static_shadowless: Vec<u32>,
    static_culling: Option<Culling>,
    storage_read: Option<wgpu::BindGroupLayout>,
    storage_write: Option<(wgpu::BindGroupLayout, wgpu::PipelineLayout)>,
    storage_buffers: Vec<Option<StorageBuffer>>,
    buffer_computes: Vec<bool>,
    storage_dummy: Option<wgpu::BindGroup>,
    /// Their bind group and pipeline layouts; `None` where the GPU can't
    /// run compute shaders (WebGL2).
    compute_layouts: Option<(wgpu::BindGroupLayout, wgpu::PipelineLayout)>,
    /// What compute passes need of the render targets they run on, made
    /// the first time one does, by texture id.
    compute_states: Vec<(u32, ComputeState)>,
    /// This frame's compute passes, in order: (compute, index into
    /// `compute_states`, material).
    computes: Vec<(u32, usize, u32)>,
    /// This frame's target cameras: (index into `view_targets`, first
    /// instance offset already added to its runs, runs, batch visibility slot).
    views: Vec<(usize, Vec<Run>, usize)>,
    /// The frame uniform alone, for the shadow pass (which can't also read
    /// the shadow map it draws into): one copy per cascade, each with its
    /// `caster_view_proj`, `shadow_stride` bytes apart.
    shadow_frames: wgpu::Buffer,
    /// What each copy in `shadow_frames` holds.
    shadow_mirrors: Vec<Mirror>,
    shadow_stride: u64,
    shadow_bind_group: wgpu::BindGroup,
    /// Each cascade's layer of the shadow map, drawn into.
    shadow_layers: Vec<wgpu::TextureView>,
    cascades: usize,
    /// Which cascades are drawn this frame (one bit each); the others keep
    /// an earlier frame's shadow map.
    redrawn: u32,
    /// The sun's shadow map as made: its width and its layers (a 1x1 one
    /// without shadows).
    shadow_dims: (u32, u32),
    /// The width of the point and spot lights' shadow atlas, once made.
    light_shadow_dims: Option<u32>,
    /// The atlas, drawn into (each view into its tile); `None` until a
    /// light first casts shadows.
    light_atlas: Option<wgpu::TextureView>,
    /// This frame's light shadow views' tiles in it.
    light_tiles: Vec<crate::three::shadow_atlas::Tile>,
    light_views: usize,
    material_layout: wgpu::BindGroupLayout,
    meshes: Vec<GpuMesh>,
    /// This frame's skinned models, drawn as mesh ids past `meshes`.
    skinning: skinning::Skinning,
    /// The depth grid occlusion culling tests against.
    occlusion: super::occlusion::Occlusion,
    /// How many models in view the frame culled as hidden.
    pub occluded: usize,
    particles: super::particles::Particles,
    materials: Vec<GpuMaterial>,
    instances: wgpu::Buffer,
    /// What `instances` holds, so each frame uploads only what changed.
    instance_mirror: Mirror,
    /// Capacity of `instances`, in instances.
    capacity: usize,
    /// This frame has more instances than the device's largest buffer
    /// holds (reported once): it isn't drawn.
    too_many: bool,
    targets: Option<Targets>,
    /// A 1x1 white texture, for materials without one.
    white: u32,
    /// Screen effect pipelines, by effect id.
    effect_pipelines: Vec<wgpu::RenderPipeline>,
    /// The game's shader pipelines in use before the GPU checked them (a
    /// browser's WebGPU answers later; see `shader_checks.rs`).
    shader_checks: Vec<shader_checks::PendingCheck>,
    /// How many times each game shader's pipelines were put in use, by
    /// kind and id: tells a late GPU rejection whether newer code runs.
    shader_versions: std::collections::HashMap<(ShaderKind, u32), u64>,
    /// Tests: treat every GPU check as one that answers later.
    #[cfg(test)]
    pub(crate) defer_checks: bool,
    /// This frame's effects, in order: (effect, material).
    effects: Vec<(u32, u32)>,
    /// Each effect's compiled shader, for building its target pipeline.
    effect_modules: Vec<wgpu::ShaderModule>,
    /// Target effect pipelines, by effect id: built when first used.
    effect_target_pipelines: Vec<Option<wgpu::RenderPipeline>>,
    /// Render targets target effects write: (texture id, texture, its view),
    /// made on first use.
    effect_targets: Vec<(u32, wgpu::Texture, wgpu::TextureView)>,
    /// This frame's target effects, in order: (effect, index in
    /// `effect_targets`, material).
    target_effects: Vec<(u32, usize, u32)>,
    /// Target effects already warned about (effect, target), so a skipped
    /// one is reported once rather than every frame.
    target_effect_warnings: Vec<(u32, u32)>,
    /// Texture arrays for materials (whole, every layer and mip level).
    arrays: Vec<wgpu::TextureView>,
    /// A 1x1 white one-layer array, for materials without one.
    white_array: wgpu::TextureView,
    /// This frame's runs: (mesh, first instance, count).
    runs: Vec<(u32, u32, u32)>,
    shadow_runs: [Vec<(u32, u32, u32)>; MAX_SHADOW_VIEWS],
    /// `runs` sorted by the state they bind, for the passes whose result
    /// doesn't depend on the order (see `draw_order.rs`).
    solid_runs: Vec<Run>,
    /// Scratch space for sorting runs.
    sort_keys: Vec<((u32, u32, u32), Run)>,
    /// The camera's runs that blend, each drawing a copy of its instances
    /// sorted far to near (see `sort_blended`): the copies, which follow
    /// every other instance in the instance buffer, and scratch space.
    blended_runs: Vec<Run>,
    blended: Vec<Instance3D>,
    blend_keys: Vec<(f32, u32)>,
    /// Whether order-independent passes draw their runs sorted by state
    /// (on; off draws them in mesh order, for comparing).
    pub(crate) group_draws: bool,
    /// The model passes' state changes since the last `take_state_changes`.
    state_counters: draw_order::StateCounters,
    /// Per skinned model this frame, whether any pass draws it.
    skin_drawn: Vec<bool>,
    /// This frame's material overrides (see `Frame3D::material_overrides`):
    /// mesh id `override_base + k` draws entry `k`'s mesh with its
    /// materials.
    material_overrides: Vec<MaterialOverride>,
    override_base: u32,
    /// Per skinned model this frame, the materials it is drawn with instead
    /// of its mesh's (the entry's mesh isn't used). Empty when none has any.
    skin_overrides: Vec<MaterialOverride>,
    /// The `(part, material)` lists the two above point into.
    override_parts: Vec<(u32, u32)>,
    /// Which shadow views keep what they hold (see [`ShadowCache`]).
    shadow_cache: ShadowCache,
    /// Clears one light atlas tile (made with the atlas).
    tile_clear: Option<wgpu::RenderPipeline>,
}

impl Renderer3D {
    pub fn new(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        white: u32,
        white_array: &wgpu::Texture,
        samples: u32,
        solid_prepass: bool,
    ) -> Self {
        debug_assert_eq!(passes::check_passes(&passes::FRAME_PASSES), Ok(()));
        // The material shaders' bindings are seen by compute passes too
        // (they read the frame and their material), where there are any.
        let compute = can_compute(device);
        let decals = supports_decals(device);
        let all_stages = if compute {
            wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE
        } else {
            wgpu::ShaderStages::VERTEX_FRAGMENT
        };
        let uniform_entry = |visibility| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let gi = supports_gi(device);
        let frame_bindings = frame_entries(all_stages, false, decals, gi);
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d frame layout"),
            entries: &frame_bindings,
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d shadow layout"),
            entries: &frame_entries(all_stages, true, decals, gi),
        });
        let material_texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: all_stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let material_array = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: all_stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        };
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d material layout"),
            entries: &[
                uniform_entry(all_stages),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: all_stages,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                material_texture(2),
                material_texture(3),
                material_texture(4),
                material_texture(5),
                material_texture(6),
                material_texture(7),
                material_array(8),
                material_array(9),
                material_array(10),
            ],
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d scene layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: all_stages,
                    ty: wgpu::BindingType::Texture {
                        // Filtered for the screen-space reflections (see `ssr.rs`).
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: all_stages,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: samples > 1,
                    },
                    count: None,
                },
            ],
        });
        let storage_layout = |read_only| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GPU buffer layout"),
                entries: &(0..if read_only { 2 } else { 1 })
                    .map(|binding| wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: if read_only {
                            wgpu::ShaderStages::VERTEX_FRAGMENT
                        } else {
                            wgpu::ShaderStages::COMPUTE
                        },
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect::<Vec<_>>(),
            })
        };
        let storage_supported = GpuLimits::of(device).storage_buffers();
        let storage_read = storage_supported.then(|| storage_layout(true));
        let storage_dummy = storage_read.as_ref().map(|layout| {
            let buffer = device.new_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 16,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &(0..2)
                    .map(|binding| wgpu::BindGroupEntry {
                        binding,
                        resource: buffer.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            })
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d pipeline layout"),
            bind_group_layouts: &if let Some(s) = &storage_read {
                vec![
                    Some(&frame_layout),
                    Some(&material_layout),
                    Some(&scene_layout),
                    Some(s),
                ]
            } else {
                vec![Some(&frame_layout), Some(&material_layout), Some(&scene_layout)]
            },
            immediate_size: 0,
        });
        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d shadow pipeline layout"),
            bind_group_layouts: &if let Some(s) = &storage_read {
                vec![
                    Some(&shadow_layout),
                    Some(&material_layout),
                    Some(&scene_layout),
                    Some(s),
                ]
            } else {
                vec![Some(&shadow_layout), Some(&material_layout), Some(&scene_layout)]
            },
            immediate_size: 0,
        });
        // Passes that can't see the scene bind a 1x1 stand-in (black, far).
        let (no_scene, dummy_depth) = {
            let tiny = |label, format, samples, usage| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: samples,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let color = tiny("3d no scene color", HDR_FORMAT, 1, wgpu::TextureUsages::TEXTURE_BINDING);
            let depth = tiny(
                "3d no scene depth",
                DEPTH_FORMAT,
                samples,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
            (scene_bind_group(device, &scene_layout, &color, &depth), depth)
        };
        let material_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d material sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let mut cache = MaterialCache::new(TargetKey {
            color: HDR_FORMAT,
            depth: DEPTH_FORMAT,
            velocity: taa::VELOCITY_FORMAT,
            samples,
        });
        let mut builtin = build_pipelines(
            device,
            &layout,
            &shadow_pipeline_layout,
            "built-in",
            &compose_shader_storage("", decals, gi, storage_supported),
            samples,
            &cache.counters,
        );
        builtin.in_use = true;
        // wgpu objects aren't `Send` on wasm (one thread): the `Arc` only shares them.
        #[allow(clippy::arc_with_non_send_sync)]
        let builtin = Arc::new(builtin);
        cache.insert(
            cache.key(ModuleKey {
                source: "".into(),
                features: Features {
                    decals,
                    gi,
                    storage: storage_supported,
                },
            }),
            &builtin,
        );
        let post_shader = device.create_shader_module(wgpu::include_wgsl!("post.wgsl"));
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d post layout"),
            entries: &[
                texture_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    ..uniform_entry(wgpu::ShaderStages::FRAGMENT)
                },
                texture_entry(3),
            ],
        });
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d post pipeline layout"),
            bind_group_layouts: &[Some(&post_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        let bloom_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d bloom pipeline layout"),
            bind_group_layouts: &[Some(&post_layout)],
            immediate_size: 0,
        });
        let full_screen = |label, layout, entry_point, format, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let post_pipeline = full_screen("3d post pipeline", &post_pipeline_layout, "fs_tonemap", format, None);
        let fxaa_pipeline = full_screen("FXAA", &bloom_pipeline_layout, "fs_fxaa", format, None);
        let upscale_pipeline = full_screen("3d upscale", &bloom_pipeline_layout, "fs_upscale", format, None);
        let bloom_prefilter = full_screen(
            "3d bloom prefilter",
            &bloom_pipeline_layout,
            "fs_bloom_prefilter",
            HDR_FORMAT,
            None,
        );
        let bloom_down = full_screen(
            "3d bloom down",
            &bloom_pipeline_layout,
            "fs_bloom_down",
            HDR_FORMAT,
            None,
        );
        // Each upsample is blended over the mip's own downsample:
        // lerp(mip, upsampled, scatter), the scatter set as blend constant.
        let lerp = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Constant,
            dst_factor: wgpu::BlendFactor::OneMinusConstant,
            operation: wgpu::BlendOperation::Add,
        };
        let bloom_up = full_screen(
            "bloom",
            &bloom_pipeline_layout,
            "fs_bloom_up",
            HDR_FORMAT,
            Some(wgpu::BlendState {
                color: lerp,
                alpha: lerp,
            }),
        );
        let post_buffer = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("3d post"),
            size: size_of::<PostUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | MIRRORED,
            mapped_at_creation: false,
        });
        let linear_clamp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let black = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("3d black"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let frame_buffer = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("3d frame"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | MIRRORED,
            mapped_at_creation: false,
        });
        let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d shadow map"),
            // A stand-in until a frame casts shadows (see `set_shadow_map`).
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_map.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let shadow_layers = (0..1)
            .map(|layer| {
                shadow_map.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("3d shadow cascade"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let align = device.limits().min_uniform_buffer_offset_alignment as u64;
        let shadow_stride = (size_of::<FrameUniform>() as u64).div_ceil(align) * align;
        let shadow_frames = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("3d shadow frames"),
            // And one for global illumination's shadow map (see `gi.rs`).
            size: shadow_stride * (MAX_SHADOW_VIEWS as u64 + 1),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | MIRRORED,
            mapped_at_creation: false,
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d shadow sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let sky_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d sky"),
            size: wgpu::Extent3d {
                width: SKY_SIZE.0,
                height: SKY_SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let sky_view = sky_texture.create_view(&Default::default());
        let environment = super::environment::Environment::new(device, &sky_view);
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d sky"),
            // Around the horizon it wraps; up and down it stops.
            address_mode_u: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sky_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3d sky"),
            source: wgpu::ShaderSource::Wgsl(
                for_samples(&format!("{MESH_PRELUDE}\n{SKY}"), samples)
                    .into_owned()
                    .into(),
            ),
        });
        let sky_pipeline = |label, layout: &wgpu::PipelineLayout, entry_point, samples, depth| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &sky_module,
                    entry_point: Some("vs_far"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: depth,
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sky_module,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(HDR_FORMAT.into())],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let sky_lut = sky_pipeline(
            "3d sky texture",
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("3d sky texture"),
                bind_group_layouts: &[Some(&shadow_layout)],
                immediate_size: 0,
            }),
            "fs_lut",
            1,
            None,
        );
        // Behind everything: only where the depth is still the far plane.
        let sky = sky_pipeline(
            "3d sky",
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("3d sky"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            }),
            "fs_sky",
            samples,
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let stand_in = |label, format, layers| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(layers),
                    ..Default::default()
                })
        };
        let no_light_shadow = stand_in("3d no light shadows", DEPTH_FORMAT, wgpu::TextureViewDimension::D2);
        let no_decal_view = stand_in(
            "3d no decal images",
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureViewDimension::D2Array,
        );
        let no_gi = stand_in("3d no gi", HDR_FORMAT, wgpu::TextureViewDimension::D2);
        let frame_sources = FrameSources {
            gi,
            gi_view: no_gi.clone(),
            layout: frame_layout,
            shadow_view,
            light_shadow_view: no_light_shadow.clone(),
            shadow_sampler,
            sky_view: environment.atlas.create_view(&Default::default()),
            sky_sampler,
            decals,
            // Never sampled: without images every decal reads as flat color.
            decal_view: no_decal_view.clone(),
            decal_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("3d decals"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            }),
        };
        let lights = LightTextures::new(device);
        let frame_bind_group = frame_sources.bind_group(device, &frame_buffer, &lights);
        // A compute pass reads its render target and writes a scratch copy.
        let compute_layouts = compute.then(|| {
            let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("3d compute layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: HDR_FORMAT,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
            let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("3d compute pipeline layout"),
                bind_group_layouts: &[
                    Some(&frame_sources.layout),
                    Some(&material_layout),
                    Some(&scene_layout),
                    Some(&compute_layout),
                ],
                immediate_size: 0,
            });
            (compute_layout, compute_pipeline_layout)
        });
        let storage_write = storage_supported.then(|| {
            let binding = storage_layout(false);
            let pipeline = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GPU buffer compute"),
                bind_group_layouts: &[
                    Some(&frame_sources.layout),
                    Some(&material_layout),
                    Some(&scene_layout),
                    Some(&binding),
                ],
                immediate_size: 0,
            });
            (binding, pipeline)
        });
        let no_shadow = stand_in("3d no shadow", DEPTH_FORMAT, wgpu::TextureViewDimension::D2Array);
        let no_sky = stand_in("3d no sky", HDR_FORMAT, wgpu::TextureViewDimension::D2);
        let no_sky_sampler = device.create_sampler(&Default::default());
        let no_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let no_lights = LightTextures::new(device);
        let mut shadow_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &shadow_frames,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<FrameUniform>() as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&no_shadow),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&no_shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&no_sky),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&no_sky_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&no_lights.records_view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&no_lights.indices_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&no_light_shadow),
            },
        ];
        if decals {
            shadow_entries.extend(decal_entries(&frame_sources.decal_view, &frame_sources.decal_sampler));
        }
        if gi {
            shadow_entries.push(wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(&no_gi),
            });
        }
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d shadow"),
            layout: &shadow_layout,
            entries: &shadow_entries,
        });
        let capacity = 1024;
        Self {
            samples,
            solid_prepass,
            layout,
            shadow_pipeline_layout,
            shaders: vec![builtin],
            cache,
            warm_variants: false,
            material_sampler,
            scene_layout,
            no_scene,
            post_pipeline,
            fxaa_pipeline,
            upscale_pipeline,
            no_light_shadow,
            output_format: format,
            aa: crate::AntiAliasing::None,
            smaa: None,
            taa: None,
            taa_supported: can_compute(device),
            taa_on: false,
            velocity_on: false,
            taa_frame: 0,
            taa_read: 0,
            taa_frame_read: 0,
            taa_previous: None,
            history_camera: None,
            #[cfg(test)]
            history_reset: HistoryReset::default(),
            moved: Synced::new(
                device,
                "3d moved instances",
                wgpu::BufferUsages::VERTEX,
                16 * size_of::<MovedInstance>(),
                Upload::Instances,
                (16, 0, MAX_WRITES),
            ),
            moved_runs: Vec::new(),
            ao: None,
            ao_enabled: false,
            ssr: None,
            ssr_on: false,
            gi: None,
            no_gi,
            gi_runs: Vec::new(),
            gi_face_runs: Vec::new(),
            gi_faces: Vec::new(),
            dummy_depth,
            linear_views: true,
            post_layout,
            bloom_prefilter,
            bloom_down,
            bloom_up,
            post_buffer,
            post_mirror: Mirror::default(),
            linear_clamp,
            black,
            lut: white,
            bloom: false,
            frame_buffer,
            sky_view,
            sky_texture,
            environment,
            sky_lut,
            sky,
            atmosphere: false,
            frame_bind_group,
            lights,
            frame_sources,
            decal_array: None,
            no_decal_view,
            render_targets: Vec::new(),
            view_targets: Vec::new(),
            compute_pipelines: Vec::new(),
            static_batches: Vec::new(),
            static_shadowless: Vec::new(),
            static_culling: None,
            storage_read,
            storage_write,
            storage_buffers: Vec::new(),
            buffer_computes: Vec::new(),
            storage_dummy,
            compute_layouts,
            skinning: skinning::Skinning::new(device, GpuLimits::of(device).gpu_skinning()),
            particles: super::particles::Particles::new(device, texture_layout, samples),
            occlusion: super::occlusion::Occlusion::new(device, compute),
            occluded: 0,
            compute_states: Vec::new(),
            computes: Vec::new(),
            views: Vec::new(),
            shadow_frames,
            shadow_mirrors: (0..=MAX_SHADOW_VIEWS).map(|_| Mirror::default()).collect(),
            shadow_stride,
            shadow_bind_group,
            shadow_layers,
            cascades: 0,
            redrawn: 0,
            shadow_dims: (1, 1),
            light_shadow_dims: None,
            light_atlas: None,
            light_tiles: Vec::new(),
            light_views: 0,
            material_layout,
            meshes: Vec::new(),
            materials: Vec::new(),
            instances: Self::instance_buffer(device, capacity),
            too_many: false,
            instance_mirror: Default::default(),
            frame_mirror: Mirror::default(),
            capacity,
            targets: None,
            white,
            effect_pipelines: Vec::new(),
            shader_checks: Vec::new(),
            shader_versions: std::collections::HashMap::new(),
            #[cfg(test)]
            defer_checks: false,
            effects: Vec::new(),
            effect_modules: Vec::new(),
            effect_target_pipelines: Vec::new(),
            effect_targets: Vec::new(),
            target_effects: Vec::new(),
            target_effect_warnings: Vec::new(),
            arrays: Vec::new(),
            white_array: array_view(white_array),
            runs: Vec::new(),
            shadow_runs: Default::default(),
            solid_runs: Vec::new(),
            sort_keys: Vec::new(),
            blended_runs: Vec::new(),
            blended: Vec::new(),
            blend_keys: Vec::new(),
            group_draws: true,
            state_counters: Default::default(),
            skin_drawn: Vec::new(),
            material_overrides: Vec::new(),
            override_base: u32::MAX,
            skin_overrides: Vec::new(),
            override_parts: Vec::new(),
            shadow_cache: ShadowCache::default(),
            tile_clear: None,
        }
    }

    /// Compiles a screen effect from the game's WGSL; returns its id.
    pub fn create_effect(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
    ) -> Result<u32, Box<ShaderDiagnostic>> {
        let start = crate::Instant::now();
        let made = self.compile_effect(device, name, user);
        self.cache.counters.compiled(start.elapsed());
        made
    }

    fn compile_effect(&mut self, device: &wgpu::Device, name: &str, user: &str) -> Result<u32, Box<ShaderDiagnostic>> {
        let (module, pipeline, verdict) = self.checked_effect(device, name, user)?;
        let id = self.effect_pipelines.len() as u32;
        self.installed(ShaderKind::Effect, id);
        if let Some(verdict) = verdict {
            let swap = Swap::Effect {
                new: pipeline.clone(),
                old: None,
            };
            self.wait_for_verdict(id, name, EFFECT_VARIANT, swap, verdict);
        }
        self.effect_pipelines.push(pipeline);
        self.effect_modules.push(module);
        self.effect_target_pipelines.push(None);
        Ok(id)
    }

    /// Checks an effect's WGSL (with its place in the game's code) and
    /// builds its pipeline in a GPU error scope; the verdict comes with it
    /// if the GPU answers later.
    fn checked_effect(
        &self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
    ) -> Result<(wgpu::ShaderModule, wgpu::RenderPipeline, Option<shader_checks::Verdict>), Box<ShaderDiagnostic>> {
        let source = compose_effect(user);
        super::diagnose(name, ShaderKind::Effect, EFFECT_VARIANT, &source)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let (module, pipeline) = build_effect(device, &self.layout, name, &for_samples(&source, self.samples));
        match end_scope(scope, self.defer_checks()) {
            Ok(None) => Ok((module, pipeline, None)),
            Ok(Some(error)) => Err(rejected(name, ShaderKind::Effect, EFFECT_VARIANT, &error)),
            Err(verdict) => Ok((module, pipeline, Some(verdict))),
        }
    }

    /// Recompiles effect `id` (hot reload). On an error the old one stays.
    pub fn replace_effect(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        name: &str,
        user: &str,
    ) -> Result<(), Box<ShaderDiagnostic>> {
        let start = crate::Instant::now();
        let made = self.recompile_effect(device, id, name, user);
        self.cache.counters.compiled(start.elapsed());
        made
    }

    fn recompile_effect(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        name: &str,
        user: &str,
    ) -> Result<(), Box<ShaderDiagnostic>> {
        let (module, new, verdict) = self.checked_effect(device, name, user)?;
        let i = id as usize;
        if i >= self.effect_pipelines.len() {
            return Ok(());
        }
        let old_pipeline = std::mem::replace(&mut self.effect_pipelines[i], new.clone());
        let old_module = std::mem::replace(&mut self.effect_modules[i], module);
        // Rebuilt from the new shader when next used.
        self.effect_target_pipelines[i] = None;
        self.installed(ShaderKind::Effect, id);
        if let Some(verdict) = verdict {
            let swap = Swap::Effect {
                new,
                old: Some((old_module, old_pipeline)),
            };
            self.wait_for_verdict(id, name, EFFECT_VARIANT, swap, verdict);
        }
        Ok(())
    }

    /// Keeps render target `id` (`texture`, in `HDR_FORMAT`), for target
    /// cameras and compute passes.
    pub fn add_render_target(&mut self, id: u32, texture: &wgpu::Texture) {
        self.render_targets.push((id, texture.clone()));
    }

    /// The index in `view_targets` of render target `id`, made on first use.
    fn view_target(&mut self, device: &wgpu::Device, id: u32) -> Option<usize> {
        if let Some(i) = self.view_targets.iter().position(|&(t, _)| t == id) {
            return Some(i);
        }
        let texture = &self.render_targets.iter().find(|&&(t, _)| t == id)?.1;
        let target = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: texture.size(),
                    mip_level_count: 1,
                    sample_count: self.samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let frame_buffer = device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("3d target camera frame"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | MIRRORED,
            mapped_at_creation: false,
        });
        let lights = LightTextures::new(device);
        let frame_bind_group = self.frame_sources.bind_group(device, &frame_buffer, &lights);
        let view = ViewTarget {
            color: target("3d target camera color", HDR_FORMAT),
            depth: target("3d target camera depth", DEPTH_FORMAT),
            resolve: texture.create_view(&Default::default()),
            frame_buffer,
            frame_mirror: Mirror::default(),
            frame_bind_group,
            lights,
        };
        self.view_targets.push((id, view));
        Some(self.view_targets.len() - 1)
    }

    /// Keeps this frame's target effects that can run: a known effect,
    /// material and render target, and a material that doesn't sample the
    /// target it writes (a pass can't read and write one texture; those are
    /// skipped with a warning). Builds their pipelines on first use.
    fn prepare_target_effects(&mut self, device: &wgpu::Device, wanted: &[(u32, u32, u32)]) {
        self.target_effects.clear();
        for &(effect, target, material) in wanted {
            if effect as usize >= self.effect_pipelines.len() || material as usize >= self.materials.len() {
                continue;
            }
            let index = match self.effect_targets.iter().position(|&(t, _, _)| t == target) {
                Some(i) => i,
                None => {
                    let Some((_, texture)) = self.render_targets.iter().find(|&&(t, _)| t == target) else {
                        continue;
                    };
                    let texture = texture.clone();
                    let view = texture.create_view(&Default::default());
                    self.effect_targets.push((target, texture, view));
                    self.effect_targets.len() - 1
                }
            };
            let texture = &self.effect_targets[index].1;
            if self.materials[material as usize].bindings.textures.contains(texture) {
                if !self.target_effect_warnings.contains(&(effect, target)) {
                    self.target_effect_warnings.push((effect, target));
                    eprintln!(
                        "ponko: target effect {effect} skipped: its material samples render target {target}, which it writes"
                    );
                }
                continue;
            }
            if self.effect_target_pipelines[effect as usize].is_none() {
                self.effect_target_pipelines[effect as usize] = Some(effect_pipeline(
                    device,
                    &self.layout,
                    "3d target effect",
                    &self.effect_modules[effect as usize],
                    "vs_effect_target",
                    "fs_effect_target",
                ));
            }
            self.target_effects.push((effect, index, material));
        }
    }

    /// How many target effects run this frame (for tests).
    #[cfg(test)]
    pub(crate) fn target_effect_count(&self) -> usize {
        self.target_effects.len()
    }

    /// The index in `compute_states` of render target `id`, made on first use.
    fn compute_state(&mut self, device: &wgpu::Device, id: u32) -> Option<usize> {
        if let Some(i) = self.compute_states.iter().position(|&(t, _)| t == id) {
            return Some(i);
        }
        let texture = &self.render_targets.iter().find(|&&(t, _)| t == id)?.1;
        let scratch = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d compute scratch"),
            size: texture.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d compute"),
            layout: &self.compute_layouts.as_ref()?.0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scratch.create_view(&Default::default())),
                },
            ],
        });
        let state = ComputeState {
            target: texture.clone(),
            scratch,
            group,
        };
        self.compute_states.push((id, state));
        Some(self.compute_states.len() - 1)
    }

    /// Makes GPU buffer `id`: a freed one's slot, or the next new one.
    pub fn create_gpu_buffer(&mut self, device: &wgpu::Device, id: u32, len: u32, data: &[crate::Vec4]) {
        let mut slot = None;
        if let Some(read_layout) = &self.storage_read {
            assert!(
                u64::from(len) * 16 <= device.limits().max_storage_buffer_binding_size,
                "GPU buffer exceeds device limit"
            );
            assert!(
                len.div_ceil(64) <= device.limits().max_compute_workgroups_per_dimension,
                "GPU buffer exceeds dispatch limit"
            );
            let zeros;
            let data = if data.is_empty() {
                zeros = vec![crate::Vec4::ZERO; len as usize];
                &zeros
            } else {
                data
            };
            let buffer = device
                .uploading(Upload::Meshes)
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GPU items"),
                    contents: bytemuck::cast_slice(&data.iter().map(|v| v.to_array()).collect::<Vec<_>>()),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });
            let indices = device.new_buffer(&wgpu::BufferDescriptor {
                label: Some("non-batch visibility"),
                size: 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let group = |layout, read| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout,
                    entries: &(0..if read { 2 } else { 1 })
                        .map(|binding| wgpu::BindGroupEntry {
                            binding,
                            resource: if binding == 0 {
                                buffer.as_entire_binding()
                            } else {
                                indices.as_entire_binding()
                            },
                        })
                        .collect::<Vec<_>>(),
                })
            };
            let read = group(read_layout, true);
            let write = group(&self.storage_write.as_ref().unwrap().0, false);
            slot = Some(StorageBuffer {
                buffer,
                read,
                write,
                len,
            });
        }
        match self.storage_buffers.get_mut(id as usize) {
            Some(old) => *old = slot,
            None => {
                // Ids are handed out in order: only the next one is new.
                debug_assert_eq!(id as usize, self.storage_buffers.len());
                self.storage_buffers.push(slot);
            }
        }
    }
    /// Frees GPU buffer `id`; passes and meshes using it do nothing from now on.
    pub fn free_gpu_buffer(&mut self, id: u32) {
        if let Some(slot) = self.storage_buffers.get_mut(id as usize) {
            *slot = None;
        }
        for mesh in &mut self.meshes {
            // Still buffer-driven but bound to no buffer: it draws nothing
            // (not once as a plain model), and the id's next buffer isn't
            // picked up until the mesh is bound to it again.
            if mesh.gpu_buffer.is_some_and(|(b, _)| b == id) {
                mesh.gpu_buffer = Some((u32::MAX, 0));
            }
        }
    }
    pub fn write_gpu_buffer(&self, queue: &wgpu::Queue, id: u32, offset: u32, data: &[crate::Vec4]) {
        if let Some(Some(b)) = self.storage_buffers.get(id as usize) {
            queue.uploading(Upload::Meshes).write_buffer(
                &b.buffer,
                u64::from(offset) * 16,
                bytemuck::cast_slice(&data.iter().map(|v| v.to_array()).collect::<Vec<_>>()),
            );
        }
    }
    pub fn set_mesh_gpu_buffer(&mut self, mesh: u32, b: u32, count: u32) {
        self.shadow_cache.invalidate();
        if let Some(mesh) = self.meshes.get_mut(mesh as usize) {
            mesh.gpu_buffer = Some((b, count));
        }
    }
    /// Compiles a compute pass from the game's WGSL; returns its id.
    pub fn create_compute(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
    ) -> Result<u32, Box<ShaderDiagnostic>> {
        let start = crate::Instant::now();
        let made = self.compile_compute(device, name, user);
        self.cache.counters.compiled(start.elapsed());
        made
    }

    fn compile_compute(&mut self, device: &wgpu::Device, name: &str, user: &str) -> Result<u32, Box<ShaderDiagnostic>> {
        let source = compose_compute(user);
        super::diagnose(name, ShaderKind::Compute, COMPUTE_VARIANT, &source)?;
        let layouts = if defines(user, "update") {
            &self.storage_write
        } else {
            &self.compute_layouts
        };
        if layouts.is_none() && defines(user, "update") {
            self.buffer_computes.push(true);
            self.compute_pipelines.push(None);
            return Ok(self.compute_pipelines.len() as u32 - 1);
        }
        let Some((_, layout)) = layouts else {
            return Err(super::without_location(
                name,
                ShaderKind::Compute,
                COMPUTE_VARIANT,
                "this GPU can't run compute passes (WebGL2 has no compute shaders)".to_string(),
            ));
        };
        let (pipeline, verdict) = self.checked_compute(device, layout, name, &source)?;
        let id = self.compute_pipelines.len() as u32;
        self.installed(ShaderKind::Compute, id);
        if let Some(verdict) = verdict {
            let swap = Swap::Compute {
                new: pipeline.clone(),
                old: None,
            };
            self.wait_for_verdict(id, name, COMPUTE_VARIANT, swap, verdict);
        }
        self.buffer_computes.push(defines(user, "update"));
        self.compute_pipelines.push(Some(pipeline));
        Ok(id)
    }

    /// Builds a compute pass's pipeline from its checked `source` in a GPU
    /// error scope; the verdict comes with it if the GPU answers later.
    fn checked_compute(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        name: &str,
        source: &str,
    ) -> Result<(wgpu::ComputePipeline, Option<shader_checks::Verdict>), Box<ShaderDiagnostic>> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = build_compute(device, layout, name, source);
        match end_scope(scope, self.defer_checks()) {
            Ok(None) => Ok((pipeline, None)),
            Ok(Some(error)) => Err(rejected(name, ShaderKind::Compute, COMPUTE_VARIANT, &error)),
            Err(verdict) => Ok((pipeline, Some(verdict))),
        }
    }

    /// Recompiles compute pass `id` (hot reload). On an error the old one stays.
    pub fn replace_compute(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        name: &str,
        user: &str,
    ) -> Result<(), Box<ShaderDiagnostic>> {
        let start = crate::Instant::now();
        let made = self.recompile_compute(device, id, name, user);
        self.cache.counters.compiled(start.elapsed());
        made
    }

    fn recompile_compute(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        name: &str,
        user: &str,
    ) -> Result<(), Box<ShaderDiagnostic>> {
        let source = compose_compute(user);
        super::diagnose(name, ShaderKind::Compute, COMPUTE_VARIANT, &source)?;
        let layouts = if defines(user, "update") {
            &self.storage_write
        } else {
            &self.compute_layouts
        };
        let Some((_, layout)) = layouts else {
            return Ok(());
        };
        let (pipeline, verdict) = self.checked_compute(device, layout, name, &source)?;
        let i = id as usize;
        if i >= self.compute_pipelines.len() {
            return Ok(());
        }
        let old = self.compute_pipelines[i].replace(pipeline.clone());
        let old_buffer = std::mem::replace(&mut self.buffer_computes[i], defines(user, "update"));
        self.installed(ShaderKind::Compute, id);
        if let Some(verdict) = verdict {
            let swap = Swap::Compute {
                new: pipeline,
                old: Some((old, old_buffer)),
            };
            self.wait_for_verdict(id, name, COMPUTE_VARIANT, swap, verdict);
        }
        Ok(())
    }

    /// Keeps a texture array for materials; returns its id.
    pub fn add_texture_array(&mut self, texture: &wgpu::Texture) -> u32 {
        self.arrays.push(array_view(texture));
        self.arrays.len() as u32 - 1
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.new_buffer(&wgpu::BufferDescriptor {
            label: Some("3d instances"),
            size: (capacity * size_of::<Instance3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | MIRRORED,
            mapped_at_creation: false,
        })
    }

    /// Compiles a material shader from the game's WGSL; returns its index.
    pub fn create_shader(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
    ) -> Result<u32, Box<ShaderDiagnostic>> {
        let (pipelines, verdict) = self.material_pipelines(device, name, user, None)?;
        let id = self.shaders.len() as u32;
        self.installed(ShaderKind::Material, id);
        if let Some((variant, verdict)) = verdict {
            let swap = Swap::Material {
                new: pipelines.clone(),
                old: None,
            };
            self.wait_for_verdict(id, name, &variant, swap, verdict);
        }
        self.shaders.push(pipelines);
        self.shadow_cache.shaders.push(ShadowCache::shader_traits(user));
        Ok(id)
    }

    /// Recompiles shader `id` (hot reload). On an error the old one stays.
    /// Only `id`'s pipelines change: its old set is dropped (unless another
    /// shader shares it), and every other shader keeps its own.
    pub fn replace_shader(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        name: &str,
        user: &str,
    ) -> Result<(), Box<ShaderDiagnostic>> {
        let (pipelines, verdict) = self.material_pipelines(device, name, user, Some(id))?;
        let old = std::mem::replace(&mut self.shaders[id as usize], pipelines.clone());
        let old_traits = std::mem::replace(
            &mut self.shadow_cache.shaders[id as usize],
            ShadowCache::shader_traits(user),
        );
        self.shadow_cache.invalidate();
        self.installed(ShaderKind::Material, id);
        if let Some((variant, verdict)) = verdict {
            let swap = Swap::Material {
                new: pipelines,
                old: Some((old, old_traits)),
            };
            self.wait_for_verdict(id, name, &variant, swap, verdict);
        }
        Ok(())
    }

    /// Checks and builds a material shader's pipelines for this device's
    /// variant, or shares the ones a shader with the same code already
    /// built (see `pipeline_cache.rs`). Pipeline errors (a binding the
    /// layout doesn't have) come back as an error instead of breaking the
    /// device.
    fn material_pipelines(
        &mut self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
        replacing: Option<u32>,
    ) -> Result<(Arc<ShaderPipelines>, shader_checks::LaterVerdict), Box<ShaderDiagnostic>> {
        let storage = self.storage_read.is_some();
        let (decals, gi) = (self.frame_sources.decals, self.frame_sources.gi);
        let key = self.cache.key(ModuleKey {
            source: user.into(),
            features: Features { decals, gi, storage },
        });
        if let Some(shared) = self.cache.get(&key) {
            // A reload with unchanged code finds its own set: nothing shared.
            let own = replacing
                .and_then(|id| self.shaders.get(id as usize))
                .is_some_and(|current| Arc::ptr_eq(current, &shared));
            if !own {
                self.cache.counters.shared();
            }
            if self.warm_variants {
                shared.warm();
            }
            return Ok((shared, None));
        }
        let start = crate::Instant::now();
        let built = self.compile_material(device, name, user);
        self.cache.counters.compiled(start.elapsed());
        let (mut built, verdict) = built?;
        // Its builds count once the GPU accepted it; rejected code's don't.
        self.cache.counters.add_builds(&built.counters);
        built.counters = self.cache.counters.clone();
        built.in_use = true;
        // wgpu objects aren't `Send` on wasm (one thread): the `Arc` only shares them.
        #[allow(clippy::arc_with_non_send_sync)]
        let built = Arc::new(built);
        self.cache.insert(key, &built);
        Ok((built, verdict))
    }

    /// Compiles a material shader that no other shader shares.
    fn compile_material(
        &self,
        device: &wgpu::Device,
        name: &str,
        user: &str,
    ) -> Result<(ShaderPipelines, shader_checks::LaterVerdict), Box<ShaderDiagnostic>> {
        let storage = self.storage_read.is_some();
        let (decals, gi) = (self.frame_sources.decals, self.frame_sources.gi);
        let variant = material_variant(decals, gi, storage);
        let source = if !storage && user.contains("items") {
            compose_shader_storage("", decals, gi, false)
        } else {
            compose_shader_storage(user, decals, gi, storage)
        };
        super::diagnose(name, ShaderKind::Material, &variant, &source)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut pipelines = build_pipelines(
            device,
            &self.layout,
            &self.shadow_pipeline_layout,
            name,
            &source,
            self.samples,
            // Its own until the GPU accepts it (see `material_pipelines`).
            &Arc::default(),
        );
        pipelines.reads_scene = reads_scene(user);
        if self.warm_variants {
            pipelines.warm();
        }
        match end_scope(scope, self.defer_checks()) {
            Ok(None) => Ok((pipelines, None)),
            Ok(Some(error)) => Err(rejected(name, ShaderKind::Material, &variant, &error)),
            Err(verdict) => Ok((pipelines, Some((variant, verdict)))),
        }
    }

    fn material_texture<'a>(&self, map: Option<u32>, textures: &'a [GpuTexture]) -> &'a wgpu::Texture {
        let id = map.filter(|&t| (t as usize) < textures.len()).unwrap_or(self.white);
        &textures[id as usize].texture
    }

    fn material_array(&self, id: Option<u32>) -> &wgpu::TextureView {
        id.and_then(|id| self.arrays.get(id as usize))
            .unwrap_or(&self.white_array)
    }

    fn material_bindings(&self, desc: &GpuMaterialDesc, textures: &[GpuTexture]) -> MaterialBindings {
        self.bindings_for(desc.textures, desc.arrays, textures)
    }

    fn bindings_for(
        &self,
        maps: crate::three::MaterialTextures,
        array_ids: [Option<u32>; 3],
        textures: &[GpuTexture],
    ) -> MaterialBindings {
        MaterialBindings {
            maps,
            array_ids,
            textures: std::array::from_fn(|i| self.material_texture(maps[i], textures).clone()),
            arrays: std::array::from_fn(|i| self.material_array(array_ids[i]).clone()),
        }
    }

    /// Material `id` was freed: it draws as the default material from now
    /// on, and lets go of its textures.
    pub fn free_material(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, id: u32, textures: &[GpuTexture]) {
        let default = GpuMaterialDesc::new(&crate::three::MaterialDesc::default());
        self.update_material(device, queue, id, &default, textures);
    }

    /// Material shader `id` (an index into `shaders`) was freed: its
    /// materials draw with the built-in shader from now on.
    pub fn free_shader(&mut self, id: u32) {
        if id == 0 {
            return;
        }
        for material in &mut self.materials {
            if material.shader == id {
                material.shader = 0;
            }
        }
        // Its pipelines go too, unless another shader shares them.
        let builtin = self.shaders[0].clone();
        if let Some(slot) = self.shaders.get_mut(id as usize) {
            *slot = builtin;
        }
    }

    /// Compute pass `id` was freed: its pipeline goes, and passes naming it
    /// are skipped.
    pub fn free_compute(&mut self, id: u32) {
        if let Some(pipeline) = self.compute_pipelines.get_mut(id as usize) {
            *pipeline = None;
        }
    }

    /// Render target `id` was freed: target cameras and compute passes let
    /// go of it and of what they made for it.
    pub fn free_render_target(&mut self, id: u32) {
        self.render_targets.retain(|&(t, _)| t != id);
        self.view_targets.retain(|&(t, _)| t != id);
        self.compute_states.retain(|&(t, _)| t != id);
    }

    /// Texture array `id` was freed: it reads as the white one from now on.
    pub fn free_texture_array(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, id: u32, textures: &[GpuTexture]) {
        let Some(view) = self.arrays.get_mut(id as usize) else {
            return;
        };
        *view = self.white_array.clone();
        if self.decal_array == Some(id) {
            self.decal_array = None;
            self.frame_sources.decal_view = self.no_decal_view.clone();
            self.rebind_frames(device);
        }
        self.rebind_materials(device, queue, textures, None, Some(id));
    }

    /// Binds again the materials that read texture `texture` (whose slot in
    /// `textures` now holds the white stand-in) or texture array `array`
    /// (now the white one). A freed normal, ORM or emissive map is switched
    /// off, so the material looks as it would without one.
    pub fn rebind_materials(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &[GpuTexture],
        texture: Option<u32>,
        array: Option<u32>,
    ) {
        for i in 0..self.materials.len() {
            let material = &self.materials[i];
            let maps = material.bindings.maps;
            let array_ids = material.bindings.array_ids;
            let reads_texture = texture.is_some_and(|t| maps.contains(&Some(t)));
            if !reads_texture && !array.is_some_and(|a| array_ids.contains(&Some(a))) {
                continue;
            }
            if reads_texture {
                // Slots 1 to 3 (normal, ORM, emissive) have flags in `maps`.
                for (slot, &map) in maps.iter().enumerate().take(4).skip(1) {
                    if map == texture {
                        let offset = std::mem::offset_of!(crate::three::MaterialUniform, maps) + 4 * (slot - 1);
                        queue.write_buffer(&material.buffer, offset as u64, &0u32.to_le_bytes());
                    }
                }
            }
            let bindings = self.bindings_for(maps, array_ids, textures);
            let bind_group = self.material_bind_group(device, &material.buffer, &bindings);
            let material = &mut self.materials[i];
            material.bind_group = bind_group;
            material.bindings = bindings;
        }
    }

    /// Binds the materials that read texture `texture` again, keeping their
    /// maps on: its pixels moved to a new GPU texture (a streamed texture
    /// at another resolution). `rebind_materials` is for freed ones.
    pub fn texture_replaced(&mut self, device: &wgpu::Device, textures: &[GpuTexture], texture: u32) {
        for i in 0..self.materials.len() {
            let material = &self.materials[i];
            let (maps, array_ids) = (material.bindings.maps, material.bindings.array_ids);
            if !maps.contains(&Some(texture)) {
                continue;
            }
            let bindings = self.bindings_for(maps, array_ids, textures);
            let bind_group = self.material_bind_group(device, &material.buffer, &bindings);
            let material = &mut self.materials[i];
            material.bind_group = bind_group;
            material.bindings = bindings;
        }
    }

    fn material_bindings_match(
        &self,
        bindings: &MaterialBindings,
        desc: &GpuMaterialDesc,
        textures: &[GpuTexture],
    ) -> bool {
        bindings.maps == desc.textures
            && bindings.array_ids == desc.arrays
            && bindings
                .textures
                .iter()
                .zip(desc.textures)
                .all(|(texture, id)| texture == self.material_texture(id, textures))
            && bindings
                .arrays
                .iter()
                .zip(desc.arrays)
                .all(|(view, id)| view == self.material_array(id))
    }

    fn material_bind_group(
        &self,
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        bindings: &MaterialBindings,
    ) -> wgpu::BindGroup {
        // Normal and ORM maps hold data, not colors: read them without sRGB decoding.
        let linear = [false, true, true, false, false, false];
        let views: Vec<wgpu::TextureView> = bindings
            .textures
            .iter()
            .zip(linear)
            .map(|(texture, linear)| {
                // Render targets (HDR) are read as they are.
                let image = texture.format() == wgpu::TextureFormat::Rgba8UnormSrgb;
                texture.create_view(&wgpu::TextureViewDescriptor {
                    format: image.then_some(if linear && self.linear_views {
                        wgpu::TextureFormat::Rgba8Unorm
                    } else {
                        wgpu::TextureFormat::Rgba8UnormSrgb
                    }),
                    ..Default::default()
                })
            })
            .collect();
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&self.material_sampler),
            },
        ];
        for (i, view) in views.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: 2 + i as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        for (i, view) in bindings.arrays.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: 8 + i as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d material"),
            layout: &self.material_layout,
            entries: &entries,
        })
    }

    pub fn create_material(&mut self, device: &wgpu::Device, desc: &GpuMaterialDesc, textures: &[GpuTexture]) -> u32 {
        let buffer = device
            .uploading(Upload::Materials)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d material"),
                contents: bytemuck::bytes_of(&desc.uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let bindings = self.material_bindings(desc, textures);
        let bind_group = self.material_bind_group(device, &buffer, &bindings);
        self.shadow_cache.invalidate();
        self.materials.push(GpuMaterial {
            buffer,
            bind_group,
            bindings,
            shader: desc.shader.min(self.shaders.len() as u32 - 1),
            transparent: desc.transparent,
            cutout: desc.cutout,
            double_sided: desc.double_sided,
        });
        self.materials.len() as u32 - 1
    }

    /// Builds every material shader's pipeline variants that would
    /// otherwise be built the first time a frame draws them (crossfading
    /// level-of-detail instances, buffer-driven instances), and from now on
    /// builds them with each new shader. Building one takes several
    /// milliseconds, a visible hitch inside a frame.
    pub fn warm_pipelines(&mut self) {
        self.warm_variants = true;
        for shader in &self.shaders {
            shader.warm();
        }
    }

    /// Whether every variant of shader `id` is built.
    #[cfg(test)]
    pub fn shader_warm(&self, id: u32) -> bool {
        let shader = &self.shaders[id as usize];
        ShaderVariant::ALL.iter().all(|&v| shader.built(v))
    }

    /// The pipeline shader `id` draws `role` of `variant` with, building
    /// the variant if needed (tests check every documented pair builds).
    #[cfg(test)]
    pub fn shader_pipeline(
        &self,
        id: u32,
        variant: ShaderVariant,
        role: PipelineRole,
    ) -> Option<&wgpu::RenderPipeline> {
        self.shaders[id as usize].lookup(variant, false).role(role)
    }

    /// How many pipeline variants frames built while drawing since the last
    /// call (each one a hitch; see [`Renderer3D::warm_pipelines`]).
    pub fn take_pipelines_built(&self) -> usize {
        self.cache.counters.frame_misses.swap(0, Ordering::Relaxed) as usize
    }

    /// The pipeline cache's counters and what it holds now.
    pub fn pipeline_stats(&self) -> crate::PipelineStats {
        self.cache.stats()
    }

    /// Panics unless every buffer the renderer writes only the changes of
    /// holds what its mirror says: the frame uniforms (every view's copy,
    /// shadow views' and probe faces'), the post, ambient occlusion,
    /// reflection, antialiasing, probe-update and environment settings,
    /// the instances, the motion records, particles, skinning, and the
    /// static batches' views and culling. Call after a frame was submitted.
    #[cfg(test)]
    pub(crate) fn check_mirrors(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.frame_mirror
            .check(device, queue, &self.frame_buffer, 0, "frame uniform");
        for (i, mirror) in self.shadow_mirrors.iter().enumerate() {
            let at = self.shadow_stride * i as u64;
            mirror.check(device, queue, &self.shadow_frames, at, "shadow view frame uniform");
        }
        for (_, target) in &self.view_targets {
            let buffer = &target.frame_buffer;
            target
                .frame_mirror
                .check(device, queue, buffer, 0, "target camera frame uniform");
        }
        self.post_mirror
            .check(device, queue, &self.post_buffer, 0, "post uniform");
        self.instance_mirror
            .check(device, queue, &self.instances, 0, "instances");
        self.moved.check(device, queue);
        self.particles.check_mirrors(device, queue);
        self.skinning.check_mirrors(device, queue);
        self.environment.check_mirrors(device, queue);
        if let Some(ao) = &self.ao {
            ao.check_mirrors(device, queue);
        }
        if let Some(ssr) = &self.ssr {
            ssr.check_mirrors(device, queue);
        }
        if let Some(taa) = &self.taa {
            taa.check_mirrors(device, queue);
        }
        if let Some(gi) = &self.gi {
            gi.check_mirrors(device, queue);
        }
        self.check_batch_mirrors(device, queue);
    }

    /// Every material pipeline key built so far (tests).
    #[cfg(test)]
    pub(crate) fn built_pipeline_keys(&self) -> Vec<pipeline_cache::PipelineKey> {
        self.cache.built_keys()
    }

    /// Changes material `id` in place: every model using it changes.
    pub fn update_material(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: u32,
        desc: &GpuMaterialDesc,
        textures: &[GpuTexture],
    ) {
        let Some(material) = self.materials.get(id as usize) else {
            return;
        };
        self.shadow_cache.invalidate();
        queue
            .uploading(Upload::Materials)
            .write_buffer(&material.buffer, 0, bytemuck::bytes_of(&desc.uniform));
        if !self.material_bindings_match(&material.bindings, desc, textures) {
            let bindings = self.material_bindings(desc, textures);
            let bind_group = self.material_bind_group(device, &material.buffer, &bindings);
            let material = &mut self.materials[id as usize];
            material.bind_group = bind_group;
            material.bindings = bindings;
        }
        let shader = desc.shader.min(self.shaders.len() as u32 - 1);
        let material = &mut self.materials[id as usize];
        material.shader = shader;
        material.transparent = desc.transparent;
        material.cutout = desc.cutout;
        material.double_sided = desc.double_sided;
    }

    /// Makes the next new mesh (tests make meshes this way).
    #[cfg(test)]
    pub fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        data: &MeshData,
        parts: &[(Range<u32>, Material)],
        skinned: bool,
    ) -> u32 {
        let id = self.meshes.len() as u32;
        self.create_mesh_at(device, id, data, parts, skinned);
        id
    }

    /// Makes mesh `id`: a freed one's slot, or the next new one.
    pub fn create_mesh_at(
        &mut self,
        device: &wgpu::Device,
        id: u32,
        data: &MeshData,
        parts: &[(Range<u32>, Material)],
        skinned: bool,
    ) {
        let normals = data.normals_or_computed();
        let vertices: Vec<Vertex> = (0..data.positions.len())
            .map(|i| Vertex {
                position: data.positions[i].to_array(),
                normal: normals[i].to_array(),
                uv: data.uvs.get(i).map_or([0.0; 2], |uv| uv.to_array()),
                color: data.colors.get(i).map_or([255; 4], |&c| {
                    linear(c).map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
                }),
                uv1: data.uvs1.get(i).map_or([0.0; 2], |uv| uv.to_array()),
            })
            .collect();
        let skin = (skinned && (!data.joints.is_empty() || !data.morph_targets.is_empty())).then(|| {
            let deltas = data
                .morph_targets
                .iter()
                .flat_map(|t| {
                    (0..vertices.len()).map(|i| skinning::Delta {
                        position: t.positions[i].to_array(),
                        normal: t.normals.get(i).map_or([0.0; 3], |n| n.to_array()),
                    })
                })
                .collect();
            self.skinning
                .create(device, &vertices, &data.joints, &data.weights, deltas)
        });
        let vertices = device
            .uploading(Upload::Meshes)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX
                    | if skin.is_some() {
                        self.skinning.rest_usage()
                    } else {
                        wgpu::BufferUsages::empty()
                    },
            });
        let indices = device
            .uploading(Upload::Meshes)
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d indices"),
                contents: bytemuck::cast_slice(&data.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.shadow_cache.invalidate();
        let mesh = GpuMesh {
            buffers: Some((vertices, indices)),
            parts: parts.iter().map(|(r, m)| (r.clone(), m.id)).collect(),
            skin,
            gpu_buffer: None,
        };
        match self.meshes.get_mut(id as usize) {
            Some(slot) => *slot = mesh,
            None => {
                // Ids are handed out in order: only the next one is new.
                debug_assert_eq!(id as usize, self.meshes.len());
                self.meshes.push(mesh);
            }
        }
    }

    /// Meshes with GPU buffers and their bytes, then GPU buffers and
    /// theirs (`Ctx::asset_inventory`).
    pub fn residency(&self) -> (usize, u64, usize, u64) {
        let (mut meshes, mut mesh_bytes) = (0, 0);
        for (vertices, indices) in self.meshes.iter().filter_map(|m| m.buffers.as_ref()) {
            meshes += 1;
            mesh_bytes += vertices.size() + indices.size();
        }
        let buffers = self.storage_buffers.iter().flatten();
        (
            meshes,
            mesh_bytes,
            buffers.clone().count(),
            buffers.map(|b| b.buffer.size()).sum(),
        )
    }

    /// Frees mesh `id`'s GPU memory; it draws nothing from now on.
    pub fn free_mesh(&mut self, id: u32) {
        self.shadow_cache.invalidate();
        if let Some(mesh) = self.meshes.get_mut(id as usize) {
            mesh.buffers = None;
            mesh.parts = Vec::new();
            mesh.skin = None;
            mesh.gpu_buffer = None;
        }
    }

    /// Uploads this frame's instances, camera, light and post settings.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, input: &Prepare3D<'_>) {
        let &Prepare3D {
            scene,
            lut,
            size: window,
        } = input;
        let &Scene3D {
            frame,
            lighting,
            sky,
            light,
            camera,
            post,
            time,
            occlusion,
        } = scene;
        // The scene is drawn at `size` and scaled to the window's `window`.
        let size = scaled_size(window, post.render_scale, device.limits().max_texture_dimension_2d);
        let upscaled = size != window;
        let post_uniform = PostUniform::new(post, lut.map(|(_, n)| n)).scaled(post, size, window);
        let post_uniform = bytemuck::bytes_of(&post_uniform);
        self.post_mirror
            .upload(queue, Upload::Uniforms, &self.post_buffer, 0, post_uniform);
        self.lut = lut.map_or(self.white, |(id, _)| id);
        self.bloom = post.bloom > 0.0;
        // Temporal antialiasing needs compute-class devices; the others get FXAA.
        self.taa_on = post.anti_aliasing == crate::AntiAliasing::Taa && self.taa_supported;
        self.aa = match post.anti_aliasing {
            crate::AntiAliasing::Taa if !self.taa_supported => crate::AntiAliasing::Fxaa,
            aa => aa,
        };
        if self.taa_on && self.taa.is_none() {
            self.taa = Some(Taa::new(device));
        }
        // The camera the frame is drawn with: jittered by a sub-pixel amount
        // with TAA (depth, GTAO and particles all use it), and the one that
        // moves are measured with.
        let unjittered = frame.view_proj;
        let jitter = if self.taa_on {
            taa::jitter(self.taa_frame)
        } else {
            [0.0; 2]
        };
        let view_proj = if self.taa_on {
            taa::jittered(unjittered, jitter, size)
        } else {
            unjittered
        };
        self.moved_runs.clear();
        self.particles.prepare(device, queue, &frame.particles, view_proj);
        let ao_on = post.ambient_occlusion.enabled
            && post.ambient_occlusion.radius.is_finite()
            && post.ambient_occlusion.intensity.is_finite()
            && post.ambient_occlusion.radius > 0.0
            && post.ambient_occlusion.intensity > 0.0;
        // Contact shadows block only the sun, and only while it's up.
        let contact = if post.contact_shadows.is_finite() && lighting.sun_intensity > 0.0 {
            post.contact_shadows.max(0.0)
        } else {
            0.0
        };
        // One full-screen pass makes both.
        self.ao_enabled = (ao_on || contact > 0.0) && can_compute(device);
        if self.ao_enabled && self.ao.is_none() {
            self.ao = Some(super::screen::Ao::new(device));
        }
        self.ssr_on = post.ssr.is_finite()
            && post.ssr > 0.0
            && post.ssr_max_distance.is_finite()
            && post.ssr_max_distance > 0.0
            && can_compute(device);
        if self.ssr_on && self.ssr.is_none() {
            self.ssr = Some(super::ssr::Ssr::new(device));
        }
        self.velocity_on = self.taa_on || (self.ao_enabled && post.ao_temporal && ao_on);
        // Global illumination: the probes are made when first on, and their
        // atlas bound to every frame (again when it's remade).
        let gi_frame = frame.gi.as_deref().filter(|_| self.frame_sources.gi);
        if let Some(g) = gi_frame {
            let gi = self
                .gi
                .get_or_insert_with(|| super::gi::Gi::new(device, self.samples, size_of::<FrameUniform>() as u64));
            let mut remade = gi.prepare(device, queue, g) || gi.face_groups.is_empty();
            remade |= gi.lights.upload(device, queue, &g.capture.lights, &g.capture.decals);
            if remade {
                self.frame_sources.gi_view = gi.atlas_view.clone();
                self.rebind_frames(device);
            }
        } else if self.gi.take().is_some() {
            self.frame_sources.gi_view = self.no_gi.clone();
            self.rebind_frames(device);
        }
        // Temporal history: one decision for every effect that reuses the
        // last frame (see `HistoryReset`), made before any of them prepares.
        let rebuild = self.targets_stale(size, post, ao_on, upscaled);
        let reset = HistoryReset {
            requested: frame.motion.reset,
            targets: rebuild,
            camera_cut: self.history_camera.is_some_and(|before| camera_cut(before, unjittered)),
        };
        if reset.any() {
            self.forget_history();
        }
        #[cfg(test)]
        {
            self.history_reset = reset;
        }
        // Camera motion is measured from the last frame's camera; after a
        // reset there is none (the last frame is no past of this one).
        let previous_view_proj = self.taa_previous.unwrap_or(unjittered);
        if let Some(ao) = &mut self.ao {
            let sun = (-lighting.sun_direction).try_normalize().unwrap_or(Vec3::Y);
            ao.prepare(
                queue,
                view_proj,
                camera,
                ao_on.then_some(&post.ambient_occlusion),
                (sun, contact),
                (post.ao_half_resolution, post.ao_temporal && ao_on),
            );
        }
        if self.targets.as_ref().is_some_and(|t| t.size != size) || self.aa != crate::AntiAliasing::Smaa {
            self.smaa = None;
        }
        if self.aa == crate::AntiAliasing::Smaa
            && (self.smaa.is_none() || self.targets.as_ref().is_none_or(|t| t.size != size))
        {
            self.smaa = Some(super::smaa::Smaa::new(
                device,
                queue,
                size.0,
                size.1,
                self.output_format,
            ));
        }
        self.atmosphere = sky.atmosphere;
        self.override_base = frame.override_base;
        self.material_overrides.clear();
        let meshes = self.meshes.len() as u32;
        self.material_overrides
            .extend(frame.material_overrides.list.iter().map(|o| MaterialOverride {
                mesh: if o.mesh < meshes { o.mesh } else { 0 },
                ..*o
            }));
        self.override_parts.clear();
        self.override_parts.extend_from_slice(&frame.material_overrides.parts);
        // Skinned models carry their own: listed only when one has any.
        self.skin_overrides.clear();
        let none = MaterialOverride::NONE;
        if frame.skins.draws.iter().any(|d| (d.material, d.parts) != none) {
            let offset = self.override_parts.len() as u32;
            self.override_parts.extend_from_slice(&frame.skins.part_materials);
            self.skin_overrides
                .extend(frame.skins.draws.iter().map(|d| MaterialOverride {
                    mesh: d.mesh,
                    material: d.material,
                    parts: (d.parts.0 + offset, d.parts.1),
                }));
        }
        self.prepare_static(device, queue, frame, camera, occlusion);
        self.runs.clear();
        self.runs.extend_from_slice(&frame.runs);
        self.effects.clear();
        let (effects, materials) = (self.effect_pipelines.len(), self.materials.len());
        self.effects.extend(
            frame
                .effects
                .iter()
                .filter(|&&(e, m)| (e as usize) < effects && (m as usize) < materials),
        );
        self.prepare_target_effects(device, &frame.target_effects);
        self.cascades = frame.cascades;
        self.redrawn = frame.redrawn;
        self.light_views = frame.light_views;
        self.light_tiles.clear();
        self.light_tiles.extend_from_slice(&frame.lights.shadow_tiles);
        for (mine, theirs) in self.shadow_runs.iter_mut().zip(&frame.shadow_runs) {
            mine.clear();
            mine.extend_from_slice(theirs);
        }
        self.computes.clear();
        for &(compute, target, material) in &frame.computes {
            if material as usize >= self.materials.len() {
                continue;
            }
            if self.buffer_computes.get(compute as usize).copied() != Some(target & (1 << 31) != 0) {
                continue;
            }
            if target & (1 << 31) != 0 {
                let id = target & !(1 << 31);
                if self
                    .compute_pipelines
                    .get(compute as usize)
                    .is_some_and(Option::is_some)
                    && self.storage_buffers.get(id as usize).is_some_and(Option::is_some)
                {
                    self.computes.push((compute, target as usize, material));
                }
                continue;
            }
            // A freed pass has no pipeline.
            if self
                .compute_pipelines
                .get(compute as usize)
                .is_some_and(Option::is_some)
                && (material as usize) < self.materials.len()
                && let Some(state) = self.compute_state(device, target)
            {
                self.computes.push((compute, state, material));
            }
        }
        // Target cameras' instances follow the main view's in one buffer,
        // then those of global illumination's probe views.
        let total = frame.instances.len()
            + frame.views.iter().map(|v| v.frame.instances.len()).sum::<usize>()
            + gi_frame.map_or(0, |g| g.capture.instances.len());
        let mut views = std::mem::take(&mut self.views);
        let mut used = 0;
        let mut offset = frame.instances.len() as u32;
        for (camera_index, view) in frame.views.iter().enumerate() {
            let Some(target) = self.view_target(device, view.target) else {
                continue;
            };
            if used == views.len() {
                views.push((0, Vec::new(), 0));
            }
            let (index, runs, batch_view) = &mut views[used];
            *batch_view = 1 + MAX_CASCADES + frame.light_views + camera_index;
            *index = target;
            runs.clear();
            runs.extend(
                view.frame
                    .runs
                    .iter()
                    .map(|&(mesh, start, count)| (mesh, start + offset, count)),
            );
            offset += view.frame.instances.len() as u32;
            used += 1;
        }
        views.truncate(used);
        self.views = views;
        self.gi_runs.clear();
        self.gi_face_runs.clear();
        self.gi_faces.clear();
        if let Some(g) = gi_frame {
            let moved = |&(mesh, start, count): &Run| (mesh, start + offset, count);
            self.gi_runs.extend(g.capture.runs.iter().map(moved));
            self.gi_face_runs.extend(g.face_runs.iter().map(moved));
            self.gi_faces.extend_from_slice(&g.faces);
        }
        // Skinned models: only those some pass draws are posed.
        let real = self.meshes.len() as u32;
        let mut drawn = std::mem::take(&mut self.skin_drawn);
        drawn.clear();
        drawn.resize(frame.skins.draws.len(), false);
        let runs = self
            .runs
            .iter()
            .chain(shadow_views(self.cascades, self.light_views).flat_map(|v| &self.shadow_runs[v]))
            .chain(self.views.iter().flat_map(|(_, r, _)| r))
            .chain(&self.gi_runs);
        for &(mesh, _, _) in runs {
            if let Some(d) = (mesh & !FADED)
                .checked_sub(real)
                .and_then(|k| drawn.get_mut(k as usize))
            {
                *d = true;
            }
        }
        let meshes = &self.meshes;
        self.skinning.prepare(
            device,
            queue,
            &frame.skins,
            |id| {
                let m = meshes.get(id as usize)?;
                Some((&m.buffers.as_ref()?.0, m.skin.as_ref()?))
            },
            |k| drawn[k],
        );
        self.skin_drawn = drawn;
        self.order_runs();
        // Blended models are drawn from sorted copies of their instances,
        // which follow all the others.
        self.sort_blended(&frame.instances, camera, total as u32);
        let total = total + self.blended.len();
        if self.static_batches.iter().all(Option::is_none)
            && total == 0
            && frame.particles.instances.is_empty()
            && frame.particles.strips.runs.is_empty()
        {
            self.too_many = false;
            return;
        }
        let record = size_of::<Instance3D>();
        let most = traffic::most_bytes(device, wgpu::BufferUsages::VERTEX) / record;
        if traffic::outgrown(&mut self.capacity, total, most) {
            self.instances = Self::instance_buffer(device, self.capacity);
            self.instance_mirror.forget();
        }
        let too_many = total > self.capacity;
        if too_many && !self.too_many {
            let then = "the 3D scene isn't drawn";
            traffic::too_big(
                "3d instances (models, every view's)",
                total * record,
                most * record,
                then,
            );
        }
        self.too_many = too_many;
        if too_many {
            return;
        }
        // Only the pages that changed since the last frame are uploaded:
        // models that stand still (and keep their place in the lists) cost
        // nothing.
        let uploads = queue.uploading(Upload::Instances);
        let buffer = &self.instances;
        let mut upload = |at: usize, bytes: &[u8]| uploads.write_buffer(buffer, at as u64, bytes);
        // A frame uploads the instances that changed, with the few
        // unchanged ones between two of them, in at most `MAX_WRITES`
        // writes a list.
        let page = size_of::<Instance3D>();
        let mut at = 0;
        let views = frame.views.iter().map(|v| &v.frame.instances);
        for instances in std::iter::once(&frame.instances)
            .chain(views)
            .chain(gi_frame.map(|g| &g.capture.instances))
            .chain(std::iter::once(&self.blended))
        {
            let bytes = bytemuck::cast_slice(instances);
            self.instance_mirror
                .sync_capped(at, bytes, (page, INSTANCE_BRIDGE), MAX_WRITES, &mut upload);
            at += bytes.len();
        }
        if self.velocity_on && !frame.motion.runs.is_empty() {
            let moved = &frame.motion.instances;
            // Bound when drawn: a new buffer needs no rebinding.
            let moved = bytemuck::cast_slice(moved);
            self.moved.reserve(device, moved.len());
            // Past the most the device holds (reported), no motion is drawn.
            if moved.len() <= self.moved.fits() {
                self.moved.write(queue, 0, moved);
                self.moved_runs.extend_from_slice(&frame.motion.runs);
            }
        }
        let scaled = |c, k: f32| {
            let [r, g, b, _] = linear(c);
            [r * k, g * k, b * k, 0.0]
        };
        let ambient = |light: Vec3| (light * lighting.ambient_intensity).extend(0.0).to_array();
        let sun = (-lighting.sun_direction).try_normalize().unwrap_or(Vec3::Y);
        self.set_shadow_map(device, frame);
        self.set_light_shadow_map(device, frame);
        // Shadow views whose maps already hold what they'd draw are kept.
        self.plan_shadows(frame, camera);
        let rebind = self.bind_decal_array(frame.decal_array);
        if self.lights.upload(device, queue, &frame.lights, &frame.decals) || rebind {
            self.frame_bind_group = self.frame_sources.bind_group(device, &self.frame_buffer, &self.lights);
        }
        if rebind {
            for (_, target) in &mut self.view_targets {
                target.frame_bind_group = self
                    .frame_sources
                    .bind_group(device, &target.frame_buffer, &target.lights);
            }
            self.rebind_gi(device);
        }
        let mut light_view_proj = [Mat4::IDENTITY.to_cols_array_2d(); MAX_LIGHT_SHADOW_VIEWS];
        for (mine, theirs) in light_view_proj.iter_mut().zip(&frame.lights.shadow_views) {
            *mine = theirs.to_cols_array_2d();
        }
        let atlas = frame.lights.atlas.max(1) as f32;
        let mut light_rect = [[0.0; 4]; MAX_LIGHT_SHADOW_VIEWS];
        for (mine, &[x, y, side]) in light_rect.iter_mut().zip(&frame.lights.shadow_tiles) {
            *mine = [x as f32 / atlas, y as f32 / atlas, side as f32 / atlas, 0.0];
        }
        let mut uniform = FrameUniform {
            light_view_proj,
            light_rect,
            probes: frame.probes,
            // Light shadows move out along the normal by 1.5 texels too; w
            // is the sun shadow's filter (`Lighting::shadow_filter`).
            light_shadow: [
                frame.light_views as f32,
                1.0 / atlas,
                1.5,
                lighting.shadow_filter.clamp(1, 3) as f32,
            ],
            cluster_depth: cluster_depth(frame),
            cluster_view_proj: frame.view_proj.to_cols_array_2d(),
            view_proj: view_proj.to_cols_array_2d(),
            unjittered_view_proj: unjittered.to_cols_array_2d(),
            previous_view_proj: previous_view_proj.to_cols_array_2d(),
            jitter: [jitter[0], jitter[1], 0.0, 0.0],
            sun_view_proj: frame.sun_view_proj.map(|m| m.to_cols_array_2d()),
            caster_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera: camera.extend(1.0).to_array(),
            sun_dir: sun.extend(0.0).to_array(),
            sun_color: light.sun.extend(0.0).to_array(),
            sky: ambient(light.sky),
            ground: ambient(light.ground),
            // The shadow lookup moves out along the normal by 1.5 texels,
            // against acne on surfaces facing away from the sun.
            shadow: [
                frame.cascades as f32,
                1.0 / frame.shadow_size.max(1) as f32,
                1.5,
                lighting.shadow_distance,
            ],
            cascade_texel: frame.cascade_texel,
            time: [time.0, time.1, 0.0, 0.0],
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            screen: [
                size.0 as f32,
                size.1 as f32,
                1.0 / size.0.max(1) as f32,
                1.0 / size.1.max(1) as f32,
            ],
            atmosphere: [
                if sky.atmosphere { 1.0 } else { 0.0 },
                sky.haze.max(0.0),
                (sky.sun_size.clamp(0.01, 20.0).to_radians() * 0.5).cos(),
                sky.stars.max(0.0),
            ],
            sky_light: {
                let [r, g, b, _] = scaled(lighting.sun_color, lighting.sun_intensity);
                [r, g, b, (camera.y / 1000.0).max(0.0)]
            },
            fog: [
                sky.fog_density.max(0.0),
                sky.fog_height,
                1.0 / sky.fog_falloff.max(1e-3),
                sky.light_shafts.clamp(0.0, 1.0),
            ],
            fog_color: scaled(sky.fog_color, 1.0),
            night: scaled(sky.night_light, 1.0),
            fog_far: [
                sky.fog_start.max(0.0),
                if sky.fog_end > sky.fog_start.max(0.0) {
                    sky.fog_end
                } else {
                    0.0
                },
                sky.fog_sun_glow.max(0.0),
                sky.fog_sun_focus.clamp(0.0, 0.99),
            ],
            gi: [[0.0; 4]; 3],
        };
        if let (Some(g), Some(gi)) = (gi_frame, &self.gi) {
            let (n, origin) = (g.grid.counts.as_vec3(), g.grid.origin.as_vec3());
            uniform.gi = [
                [0.0, 0.0, 0.0, g.grid.spacing],
                [n.x, n.y, n.z, g.intensity],
                [origin.x, origin.y, origin.z, gi.rows() as f32],
            ];
        }
        // A three-color ambient, without an atmosphere.
        let equator = match lighting.equator_color {
            Some(c) if !sky.atmosphere => {
                let [r, g, b, _] = linear(c);
                [
                    r * lighting.ambient_intensity,
                    g * lighting.ambient_intensity,
                    b * lighting.ambient_intensity,
                    1.0,
                ]
            }
            _ => [0.0; 4],
        };
        self.environment.prepare(
            queue,
            [
                sun.x,
                sun.y,
                sun.z,
                sky.haze,
                camera.y,
                lighting.ambient_intensity,
                light.sky.x,
                light.sky.y,
                light.sky.z,
                light.ground.x,
                light.ground.y,
                light.ground.z,
                uniform.sky_light[0],
                uniform.sky_light[1],
                uniform.sky_light[2],
                uniform.night[0],
                uniform.night[1],
                uniform.night[2],
                equator[0],
                equator[1],
                equator[2],
                equator[3],
            ],
            sky.atmosphere,
            lighting.ambient_intensity,
            [uniform.sky, uniform.ground, equator],
        );
        self.environment.upload(device, queue, &frame.probe_uploads);
        self.environment.bakes.clear();
        self.environment.bakes.extend_from_slice(&frame.probe_bakes);
        self.environment.prepare_bakes(device, &self.render_targets);
        sync_frame(queue, &self.frame_buffer, 0, &mut self.frame_mirror, &uniform);
        // Each target camera's frame: the same, seen from it.
        for view in &frame.views {
            let Some(target) = self.view_targets.iter().position(|&(id, _)| id == view.target) else {
                continue;
            };
            let target_lights = &mut self.view_targets[target].1;
            if target_lights
                .lights
                .upload(device, queue, &view.frame.lights, &view.frame.decals)
            {
                target_lights.frame_bind_group =
                    self.frame_sources
                        .bind_group(device, &target_lights.frame_buffer, &target_lights.lights);
            }
            let mut theirs = uniform;
            // Captures use sky IBL only, preventing recursive probe feedback.
            if frame.probe_bakes.iter().any(|(_, faces)| faces.contains(&view.target)) {
                theirs.probes.fill(Default::default());
            }
            theirs.cluster_depth = cluster_depth(&view.frame);
            theirs.cluster_view_proj = view.frame.view_proj.to_cols_array_2d();
            theirs.view_proj = view.view_proj.to_cols_array_2d();
            theirs.inv_view_proj = view.view_proj.inverse().to_cols_array_2d();
            // Target cameras are not resolved over time.
            theirs.unjittered_view_proj = theirs.view_proj;
            theirs.previous_view_proj = theirs.view_proj;
            theirs.jitter = [0.0; 4];
            theirs.camera = view.camera.extend(1.0).to_array();
            theirs.screen = [
                view.size.0 as f32,
                view.size.1 as f32,
                1.0 / view.size.0.max(1) as f32,
                1.0 / view.size.1.max(1) as f32,
            ];
            theirs.sky_light[3] = (view.camera.y / 1000.0).max(0.0);
            let target = &mut self.view_targets[target].1;
            sync_frame(queue, &target.frame_buffer, 0, &mut target.frame_mirror, &theirs);
        }
        // Global illumination's probe views: the same, from each probe, lit
        // by their own sun shadow map (one cascade around the grid, not
        // fading out), without point and spot light shadows.
        if let (Some(g), Some(gi)) = (gi_frame, &mut self.gi) {
            let mut theirs = uniform;
            theirs.cluster_depth = cluster_depth(&g.capture);
            theirs.cluster_view_proj = g.capture_view_proj.to_cols_array_2d();
            theirs.jitter = [0.0; 4];
            let size = super::gi::VIEW_SIZE as f32;
            theirs.screen = [size, size, 1.0 / size, 1.0 / size];
            theirs.shadow = [1.0, 1.0 / GI_SHADOW_SIZE as f32, 1.5, f32::MAX];
            theirs.sun_view_proj[0] = g.sun_view_proj.to_cols_array_2d();
            theirs.cascade_texel[0] = g.sun_texel;
            theirs.light_shadow[0] = 0.0;
            for (i, view_proj) in g.face_view_proj.iter().enumerate() {
                let probe = g.updates[i / 6].position;
                theirs.view_proj = view_proj.to_cols_array_2d();
                theirs.inv_view_proj = view_proj.inverse().to_cols_array_2d();
                theirs.unjittered_view_proj = theirs.view_proj;
                theirs.previous_view_proj = theirs.view_proj;
                theirs.camera = probe.extend(1.0).to_array();
                theirs.sky_light[3] = (probe.y / 1000.0).max(0.0);
                sync_frame(queue, &gi.face_buffers[i], 0, &mut gi.face_mirrors[i], &theirs);
            }
        }
        // Copy 0 also serves the sky texture's pass. `camera.w` 0 tells
        // material shaders they're drawing a shadow map (`drawing_shadows`).
        uniform.camera[3] = 0.0;
        for i in 0..frame.cascades.max(1) {
            uniform.caster_view_proj = uniform.sun_view_proj[i.min(MAX_CASCADES - 1)];
            let mirror = &mut self.shadow_mirrors[i];
            sync_frame(
                queue,
                &self.shadow_frames,
                self.shadow_stride * i as u64,
                mirror,
                &uniform,
            );
        }
        for i in 0..frame.light_views {
            uniform.caster_view_proj = uniform.light_view_proj[i];
            let (at, mirror) = (MAX_CASCADES + i, &mut self.shadow_mirrors[MAX_CASCADES + i]);
            sync_frame(
                queue,
                &self.shadow_frames,
                self.shadow_stride * at as u64,
                mirror,
                &uniform,
            );
        }
        if let Some(g) = gi_frame {
            uniform.caster_view_proj = g.sun_view_proj.to_cols_array_2d();
            let mirror = &mut self.shadow_mirrors[MAX_SHADOW_VIEWS];
            sync_frame(
                queue,
                &self.shadow_frames,
                self.shadow_stride * MAX_SHADOW_VIEWS as u64,
                mirror,
                &uniform,
            );
        }
        // Targets made for settings that have since changed (a different
        // size, or an effect turned on or off) are replaced; the old ones
        // are freed once the commands that used them have run.
        if rebuild {
            let texture = |label, format, samples, usage| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
            };
            let target = |label, format, samples, usage| {
                texture(label, format, samples, wgpu::TextureUsages::RENDER_ATTACHMENT | usage)
                    .create_view(&Default::default())
            };
            let none = wgpu::TextureUsages::empty();
            let hdr_texture = texture(
                "3d hdr",
                HDR_FORMAT,
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
            );
            let hdr = hdr_texture.create_view(&Default::default());
            let scene_texture = texture(
                "3d scene copy",
                HDR_FORMAT,
                1,
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
            let scene_view = scene_texture.create_view(&Default::default());
            let depth = target(
                "3d depth",
                DEPTH_FORMAT,
                self.samples,
                wgpu::TextureUsages::TEXTURE_BINDING,
            );
            let scene = scene_bind_group(device, &self.scene_layout, &scene_view, &depth);
            let scene_of_hdr = scene_bind_group(device, &self.scene_layout, &hdr, &depth);
            // Bloom starts at half size and halves until about 8 pixels.
            let half = (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1));
            let mips = (32 - half.0.min(half.1).max(1).leading_zeros())
                .saturating_sub(3)
                .clamp(1, BLOOM_MIPS);
            let bloom = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("3d bloom"),
                size: wgpu::Extent3d {
                    width: half.0,
                    height: half.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let bloom_mips: Vec<wgpu::TextureView> = (0..mips)
                .map(|mip| {
                    bloom.create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect();
            let group = |source: &wgpu::TextureView, bloom: &wgpu::TextureView| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("3d post"),
                    layout: &self.post_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.linear_clamp),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.post_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(bloom),
                        },
                    ],
                })
            };
            let ldr = (self.aa == crate::AntiAliasing::Fxaa).then(|| {
                target(
                    "3d LDR before AA",
                    self.output_format,
                    1,
                    wgpu::TextureUsages::TEXTURE_BINDING,
                )
            });
            let aa_group = ldr.as_ref().map(|ldr| group(ldr, &self.black));
            let upscale = upscaled.then(|| {
                target(
                    "3d scaled frame",
                    self.output_format,
                    1,
                    wgpu::TextureUsages::TEXTURE_BINDING,
                )
            });
            let upscale_group = upscale.as_ref().map(|u| group(u, &self.black));
            let n = bloom_mips.len();
            let velocity = self
                .velocity_on
                .then(|| taa::VelocityTargets::new(device, size, self.samples));
            let taa = self
                .taa
                .as_ref()
                .filter(|_| self.taa_on)
                .map(|t| t.targets(device, size, velocity.as_ref().unwrap().velocity(), &depth));
            let lit = match &taa {
                Some(t) => &t.input_view,
                None => &hdr,
            };
            let ssr = self
                .ssr
                .as_ref()
                .filter(|_| self.ssr_on)
                .map(|s| s.targets(device, size, &depth, lit, self.ao_enabled));
            // With reflections, GTAO writes the first level of their image.
            let ao = self.ao.as_ref().filter(|_| self.ao_enabled).map(|ao| {
                ao.targets(
                    device,
                    size,
                    &depth,
                    &self.black,
                    ssr.as_ref().map(|s| &s.ao),
                    velocity.as_ref().map(|v| v.velocity()).unwrap_or(&self.black),
                )
            });
            // The opaque pass reads both from its scene color slot (see `ssr.rs`).
            let screen = ssr.as_ref().map(|s| &s.screen).or(ao.as_ref().map(|a| &a.output));
            let opaque_scene =
                screen.map(|screen| scene_bind_group(device, &self.scene_layout, screen, &self.dummy_depth));
            self.targets = Some(Targets {
                ao,
                ssr,
                opaque_scene,
                taa,
                velocity,
                ldr,
                aa_group,
                upscale,
                upscale_group,
                color: target("3d color", HDR_FORMAT, self.samples, none),
                depth,
                hdr_texture,
                scene_texture,
                scene,
                post: group(&hdr, &bloom_mips[0]),
                bloom_prefilter: group(&hdr, &self.black),
                post_of_scene: group(&scene_view, &bloom_mips[0]),
                bloom_prefilter_of_scene: group(&scene_view, &self.black),
                scene_of_hdr,
                scene_view,
                bloom_down: (1..n).map(|i| group(&bloom_mips[i - 1], &self.black)).collect(),
                bloom_up: (0..n - 1).map(|i| group(&bloom_mips[i + 1], &self.black)).collect(),
                bloom_mips,
                hdr,
                size,
            });
        }
        // Temporal antialiasing: this frame's resolve, and what the next reads.
        if !self.taa_on {
            self.taa_previous = self.velocity_on.then_some(unjittered);
        } else if self.has_draws() {
            // No camera of the last frame: the first frame with TAA, or
            // the history was just forgotten.
            let reset = self.taa_previous.is_none();
            if let Some(taa) = &mut self.taa {
                taa.prepare(
                    queue,
                    &taa::Resolve {
                        jittered: view_proj,
                        current: unjittered,
                        previous: previous_view_proj,
                        input_size: size,
                        output_size: size,
                        sharpness: post.taa_sharpness,
                        reset,
                    },
                );
            }
            self.taa_previous = Some(unjittered);
            self.taa_frame = self.taa_frame.wrapping_add(1);
            self.taa_frame_read = self.taa_read;
            self.taa_read ^= 1;
        }
        // Screen-space reflections: the last frame's image is their colors,
        // unless they were off or the history was forgotten above.
        let drawn = self.has_draws();
        if let Some(ssr) = &mut self.ssr {
            if !self.ssr_on {
                ssr.forget();
            }
            if self.ssr_on && drawn {
                ssr.prepare(queue, view_proj, camera, post.ssr.min(1.0), post.ssr_max_distance);
            }
        }
        // Cuts are measured from the last frame that left an image behind.
        if drawn {
            self.history_camera = Some(unjittered);
        }
        self.occluded = frame.occluded;
        if let Some(targets) = &self.targets {
            self.occlusion.prepare(
                device,
                queue,
                &targets.depth,
                size,
                self.samples,
                frame.view_proj,
                camera,
                occlusion,
            );
        }
    }

    /// The newest depth grid read back for occlusion culling, if one came
    /// back since the last call.
    pub fn take_occlusion(&mut self) -> Option<crate::three::occlusion::OcclusionGrid> {
        self.occlusion.latest.take()
    }

    /// Starts reading this frame's occlusion depth back. Call after submitting.
    pub fn submitted(&mut self) {
        self.occlusion.submitted();
    }

    /// For a device without view formats (WebGL2): a sprite texture is read
    /// as it is, in its own (sRGB) format, in the material slots that
    /// otherwise read it linearly. Data textures (normal, ORM maps) aren't
    /// affected: they are remade as linear textures when a material first
    /// uses them (`Gpu::ensure_mips`). What is still read decoded there is the
    /// default white image (the same either way) and a texture that was first
    /// used as a color and then as data.
    pub(super) fn without_linear_views(mut self) -> Self {
        self.linear_views = false;
        self
    }

    /// Where the lit frame's color pass draws, and what it resolves into:
    /// the multisampled color target resolving into the lit image, or with
    /// one sample the lit image itself.
    fn frame_color<'a>(&self, targets: &'a Targets) -> (&'a wgpu::TextureView, Option<&'a wgpu::TextureView>) {
        let lit = targets.lit_view(self.taa_on);
        if self.samples > 1 {
            (&targets.color, Some(lit))
        } else {
            (lit, None)
        }
    }

    /// Records the occlusion depth grid's reduction, timed as a compute pass.
    fn record_occlusion(&self, encoder: &mut wgpu::CommandEncoder, frame: &FrameParts<'_, '_>) {
        let timestamps = (frame.time)("occlusion depth").map(|t| wgpu::ComputePassTimestampWrites {
            query_set: t.query_set,
            beginning_of_pass_write_index: t.beginning_of_pass_write_index,
            end_of_pass_write_index: t.end_of_pass_write_index,
        });
        self.occlusion.record(encoder, timestamps);
    }

    /// Makes the sun's shadow map for this frame's size and cascades, if it
    /// isn't the one made already. Without shadows it shrinks to a 1x1
    /// stand-in, freeing the memory. The shadow passes redraw every layer
    /// each frame, so a new map never shows what the old one held.
    fn set_shadow_map(&mut self, device: &wgpu::Device, frame: &Frame3D) {
        let wanted = if frame.cascades > 0 {
            (frame.shadow_size.max(1), frame.cascades as u32)
        } else {
            (1, 1)
        };
        if self.shadow_dims == wanted {
            return;
        }
        self.shadow_cache.invalidate();
        let map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d shadow map"),
            size: wgpu::Extent3d {
                width: wanted.0,
                height: wanted.0,
                depth_or_array_layers: wanted.1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.frame_sources.shadow_view = map.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        self.shadow_layers = (0..wanted.1)
            .map(|layer| {
                map.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("3d shadow cascade"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        self.shadow_dims = wanted;
        self.rebind_frames(device);
    }

    /// Binds decal texture array `id` (`None`: the stand-in) in the frame
    /// bind groups' source. True when it changed, so they must be remade.
    fn bind_decal_array(&mut self, id: Option<u32>) -> bool {
        let id = id.filter(|&id| (id as usize) < self.arrays.len());
        if !self.frame_sources.decals || id == self.decal_array {
            return false;
        }
        self.decal_array = id;
        if let Some(id) = id {
            self.frame_sources.decal_view = self.arrays[id as usize].clone();
        } else {
            self.frame_sources.decal_view = self.no_decal_view.clone();
        }
        true
    }

    /// Makes the point and spot lights' shadow atlas (the first time a light
    /// casts shadows) and binds it to every frame. Made again when its size
    /// changes, and freed when point and spot light shadows are turned off.
    fn set_light_shadow_map(&mut self, device: &wgpu::Device, frame: &Frame3D) {
        let wanted = if frame.light_budget == 0 {
            None
        } else if frame.light_views > 0 || self.light_shadow_dims.is_some() {
            Some(frame.light_atlas.max(1))
        } else {
            return; // not needed yet
        };
        if self.light_shadow_dims == wanted {
            return;
        }
        self.shadow_cache.invalidate();
        self.light_shadow_dims = wanted;
        let Some(size) = wanted else {
            // Back to the 1x1 stand-in the frame bind groups start with.
            self.light_atlas = None;
            self.frame_sources.light_shadow_view = self.no_light_shadow.clone();
            self.rebind_frames(device);
            return;
        };
        let map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d light shadow atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = map.create_view(&Default::default());
        if self.tile_clear.is_none() {
            self.tile_clear = Some(shadow_cache::tile_clear_pipeline(device));
        }
        self.frame_sources.light_shadow_view = view.clone();
        self.light_atlas = Some(view);
        self.rebind_frames(device);
    }

    /// Makes every frame bind group again (the main view's and each target
    /// camera's) from `frame_sources`, after a shadow map changed.
    fn rebind_frames(&mut self, device: &wgpu::Device) {
        self.frame_bind_group = self.frame_sources.bind_group(device, &self.frame_buffer, &self.lights);
        for (_, target) in &mut self.view_targets {
            target.frame_bind_group = self
                .frame_sources
                .bind_group(device, &target.frame_buffer, &target.lights);
        }
        self.rebind_gi(device);
    }

    /// Makes global illumination's probe views' frame bind groups again.
    fn rebind_gi(&mut self, device: &wgpu::Device) {
        if let Some(gi) = &mut self.gi {
            gi.set_sky(device, &self.frame_sources.sky_view, &self.frame_sources.sky_sampler);
            gi.face_groups = gi
                .face_buffers
                .iter()
                .map(|b| {
                    self.frame_sources
                        .bind_group_shadowed(device, b, &gi.lights, &gi.shadow_array)
                })
                .collect();
        }
    }

    /// The size the scene is drawn at (test hook).
    #[cfg(test)]
    pub(super) fn scene_size(&self) -> Option<(u32, u32)> {
        self.targets.as_ref().map(|t| t.size)
    }

    /// The sun's shadow map: width and layers (test hook).
    #[cfg(test)]
    pub(super) fn shadow_map(&self) -> (u32, u32) {
        self.shadow_dims
    }

    /// The point and spot lights' shadow atlas: its width, once made (test
    /// hook).
    #[cfg(test)]
    pub(super) fn light_shadow_map(&self) -> Option<u32> {
        self.light_shadow_dims
    }

    /// Whether the scaling pass has its target (test hook).
    #[cfg(test)]
    pub(super) fn scales_output(&self) -> bool {
        self.targets.as_ref().is_some_and(|t| t.upscale.is_some())
    }

    /// Draw calls the frame's models take: two per mesh part (depth
    /// prepass and lit), one per mesh part in each shadow cascade, one per
    /// screen effect, the bloom steps and the post pass.
    pub fn draw_calls(&self) -> usize {
        let parts: usize = self
            .runs
            .iter()
            .map(|&(mesh, _, _)| self.meshes[self.base(mesh) as usize].parts.len())
            .sum();
        let view_parts: usize = self
            .views
            .iter()
            .flat_map(|(_, runs, _)| runs)
            .map(|&(mesh, _, _)| self.meshes[self.base(mesh) as usize].parts.len())
            .sum();
        self.computes.len()
            + (parts + view_parts) * 2
            + self.views.len() * usize::from(self.atmosphere)
            + self.particles.draw_count()
            + self.effects.len()
            + self.target_effects.len()
            + usize::from(self.ao_enabled) * 2
            + usize::from(self.ssr_on) * 2
            + usize::from(self.taa_on)
            + if self.velocity_on {
                // The motion pass (one draw per moved part).
                self.moved_runs
                    .iter()
                    .map(|&(mesh, _, _)| self.meshes[self.base(mesh) as usize].parts.len())
                    .sum::<usize>()
            } else {
                0
            }
            + match self.aa {
                crate::AntiAliasing::None | crate::AntiAliasing::Taa => 0,
                crate::AntiAliasing::Fxaa => 1,
                crate::AntiAliasing::Smaa => 3,
            }
            + if self.atmosphere { 2 } else { 0 }
            + shadow_views(self.cascades, self.light_views)
                .map(|v| self.shadow_runs[v].len())
                .sum::<usize>()
            + 1
            + if self.bloom {
                self.targets.as_ref().map_or(0, |t| t.bloom_mips.len() * 2 - 1)
            } else {
                0
            }
    }

    /// (instances, triangles) the camera draws, then the same for the shadow maps.
    pub fn counts(&self) -> [(usize, usize); 2] {
        let count = |runs: &[(u32, u32, u32)]| {
            runs.iter().fold((0, 0), |(n, tris), &(mesh, _, k)| {
                let per: u32 = self.meshes[self.base(mesh) as usize]
                    .parts
                    .iter()
                    .map(|(range, _)| range.len() as u32 / 3)
                    .sum();
                (n + k as usize, tris + per as usize * k as usize)
            })
        };
        let shadows = shadow_views(self.cascades, self.light_views)
            .map(|v| count(&self.shadow_runs[v]))
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        [count(&self.runs), shadows]
    }

    /// Per mesh: (mesh, camera models, camera triangles, shadow models,
    /// shadow triangles), for meshes drawn this frame, in mesh order.
    pub fn mesh_costs(&self) -> Vec<(u32, usize, usize, usize, usize)> {
        let mut out: Vec<(u32, usize, usize, usize, usize)> = Vec::new();
        let tris = |mesh: u32| -> usize {
            self.meshes[self.base(mesh) as usize]
                .parts
                .iter()
                .map(|(range, _)| range.len() / 3)
                .sum()
        };
        let mut add = |mesh: u32, count: u32, shadow: bool| {
            // A material override counts towards its mesh.
            let mesh = self.material_override(mesh).map_or(mesh & !FADED, |o| o.mesh);
            let at = match out.binary_search_by_key(&mesh, |e| e.0) {
                Ok(i) => i,
                Err(i) => {
                    out.insert(i, (mesh, 0, 0, 0, 0));
                    i
                }
            };
            let (n, t) = (count as usize, tris(mesh) * count as usize);
            if shadow {
                out[at].3 += n;
                out[at].4 += t;
            } else {
                out[at].1 += n;
                out[at].2 += t;
            }
        };
        for &(mesh, _, count) in &self.runs {
            add(mesh, count, false);
        }
        for view in shadow_views(self.cascades, self.light_views) {
            for &(mesh, _, count) in &self.shadow_runs[view] {
                add(mesh, count, true);
            }
        }
        out
    }

    /// Whether this frame has anything to draw.
    pub fn has_draws(&self) -> bool {
        self.static_batches.iter().any(Option::is_some)
            || !self.runs.is_empty()
            || self.particles.has_draws()
            || !self.computes.is_empty()
    }

    /// Whether solid surfaces go through the depth prepass too: on GPUs
    /// that need it, and for the passes that read the complete depth
    /// before the opaque pass (GTAO, screen-space reflections).
    fn full_prepass(&self) -> bool {
        self.solid_prepass || self.ao_enabled || self.ssr_on || self.velocity_on
    }

    /// Draws the frame's passes (see the module docs) and leaves the tone
    /// mapped result in `view`. `clear` is the background, in linear color.
    ///
    /// Each part of the frame (each shadow cascade, the depth prepass, the
    /// opaque and transparent passes, ...) is recorded into its own command
    /// buffer, all on different cores: turning draw calls into GPU commands
    /// (wgpu's `finish`) is most of a busy frame's CPU time. The buffers come
    /// back in the order they must be submitted.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        clear: wgpu::Color,
        textures: &[GpuTexture],
        timer: Option<&GpuTimer>,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(targets) = self.targets.as_ref().filter(|_| !self.too_many) else {
            return Vec::new();
        };
        let time = |name| timer.and_then(|t| t.pass(name));
        let split = self.needs_scene();
        self.shadow_cache.recorded();
        let mut parts = Vec::with_capacity(8 + self.cascades + self.light_views);
        parts.push(Part::Prelude);
        for cascade in shadow_views(self.cascades, self.light_views) {
            if cascade < MAX_CASCADES && self.redrawn & (1 << cascade) == 0 {
                continue; // kept from an earlier frame: not even cleared
            }
            if cascade >= MAX_CASCADES && self.shadow_cache.light_kept & (1 << (cascade - MAX_CASCADES)) != 0 {
                continue; // its tile already holds this
            }
            let runs = &self.shadow_runs[cascade];
            let mut first = 0;
            let mut draws = 0;
            for (i, &(mesh, _, _)) in runs.iter().enumerate() {
                draws += self.meshes[self.base(mesh) as usize].parts.len();
                if draws >= SHADOW_DRAWS_PER_PASS {
                    parts.push(Part::Shadow {
                        cascade,
                        first,
                        end: i as u32 + 1,
                    });
                    (first, draws) = (i as u32 + 1, 0);
                }
            }
            if first == 0 || (first as usize) < runs.len() {
                parts.push(Part::Shadow {
                    cascade,
                    first,
                    end: runs.len() as u32,
                });
            }
        }
        if !self.views.is_empty() || self.environment.has_probes() {
            parts.push(Part::Views);
        }
        if self.gi.as_ref().is_some_and(|g| g.update_count() > 0) {
            parts.push(Part::Gi);
        }
        parts.extend([Part::Prepass, Part::Opaque]);
        if split {
            parts.push(Part::Transparent);
        }
        parts.push(Part::Post);
        // The order must keep what each pass declares it reads and writes
        // (see `passes`); checked in debug builds, so in every render test.
        if cfg!(debug_assertions)
            && let Err(error) = passes::check_order(&passes::FRAME_PASSES, parts.iter().map(|p| p.pass()))
        {
            panic!("invalid 3d frame order: {error}");
        }
        let frame = FrameParts {
            view,
            clear,
            textures,
            targets,
            split,
            time: &time,
        };
        let record = |part: &Part| {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(part.label()),
            });
            self.record(*part, &mut encoder, &frame);
            encoder.finish()
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rayon::prelude::*;
            parts.par_iter().map(record).collect()
        }
        #[cfg(target_arch = "wasm32")]
        {
            parts.iter().map(record).collect()
        }
    }

    /// Records one part of the frame (see [`draw`](Self::draw)).
    fn record(&self, part: Part, encoder: &mut wgpu::CommandEncoder, frame: &FrameParts<'_, '_>) {
        let FrameParts {
            view,
            clear,
            textures,
            targets,
            split,
            time,
        } = *frame;
        match part {
            Part::Prelude => {
                self.cull_static(encoder, time);
                // Skinned models first: every pass draws them posed.
                if self.skinning.has_work() {
                    let timestamps = time("skinning").map(|t| wgpu::ComputePassTimestampWrites {
                        query_set: t.query_set,
                        beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                        end_of_pass_write_index: t.end_of_pass_write_index,
                    });
                    self.skinning.record(encoder, timestamps);
                }
                // Compute passes first, so this frame's materials see their results.
                for &(compute, state, material) in &self.computes {
                    if state & (1 << 31) != 0 {
                        let buffer = self.storage_buffers[state & !(1 << 31)].as_ref().unwrap();
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("GPU buffer update"),
                            timestamp_writes: None,
                        });
                        pass.set_pipeline(self.compute_pipelines[compute as usize].as_ref().unwrap());
                        pass.set_bind_group(0, &self.frame_bind_group, &[]);
                        pass.set_bind_group(1, &self.materials[material as usize].bind_group, &[]);
                        pass.set_bind_group(2, &self.no_scene, &[]);
                        pass.set_bind_group(3, &buffer.write, &[]);
                        pass.dispatch_workgroups(buffer.len.div_ceil(64), 1, 1);
                        continue;
                    }
                    let state = &self.compute_states[state].1;
                    {
                        let timestamps = time("compute").map(|t| wgpu::ComputePassTimestampWrites {
                            query_set: t.query_set,
                            beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                            end_of_pass_write_index: t.end_of_pass_write_index,
                        });
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("3d compute"),
                            timestamp_writes: timestamps,
                        });
                        pass.set_pipeline(self.compute_pipelines[compute as usize].as_ref().unwrap());
                        pass.set_bind_group(0, &self.frame_bind_group, &[]);
                        pass.set_bind_group(1, &self.materials[material as usize].bind_group, &[]);
                        pass.set_bind_group(2, &self.no_scene, &[]);
                        pass.set_bind_group(3, &state.group, &[]);
                        let size = state.target.size();
                        pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
                    }
                    encoder.copy_texture_to_texture(
                        state.scratch.as_image_copy(),
                        state.target.as_image_copy(),
                        state.target.size(),
                    );
                }
                if self.atmosphere && self.environment.needs_sky() {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d sky texture"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.sky_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: time("sky texture"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_pipeline(&self.sky_lut);
                    pass.set_bind_group(0, &self.shadow_bind_group, &[0]);
                    pass.draw(0..3, 0..1);
                }
                self.environment.sky(encoder, &self.sky_texture);
            }
            Part::Shadow { cascade, first, end } => {
                // Light views share the atlas: the first one clears all of
                // it, unless some keep their tiles; then each drawn view
                // clears its own.
                let whole = self.shadow_cache.light_kept == 0;
                let (view, clear) = if cascade < MAX_CASCADES {
                    (&self.shadow_layers[cascade], first == 0)
                } else if let Some(atlas) = &self.light_atlas {
                    (atlas, whole && cascade == MAX_CASCADES && first == 0)
                } else {
                    return;
                };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("3d shadows"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: if clear {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: time(if cascade < MAX_CASCADES {
                        SHADOW_PASSES[cascade.min(SHADOW_PASSES.len() - 1)]
                    } else {
                        "light shadows"
                    }),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if cascade >= MAX_CASCADES
                    && let Some(&[x, y, side]) = self.light_tiles.get(cascade - MAX_CASCADES)
                {
                    pass.set_viewport(x as f32, y as f32, side as f32, side as f32, 0.0, 1.0);
                    pass.set_scissor_rect(x, y, side, side);
                    if !whole
                        && first == 0
                        && let Some(tile_clear) = &self.tile_clear
                    {
                        pass.set_pipeline(tile_clear);
                        pass.draw(0..3, 0..1);
                    }
                }
                let offset = (self.shadow_stride * cascade as u64) as u32;
                pass.set_bind_group(0, &self.shadow_bind_group, &[offset]);
                pass.set_bind_group(2, &self.no_scene, &[]);
                let runs = &self.shadow_runs[cascade][first as usize..end as usize];
                self.draw_model_runs(&mut pass, runs, Kind::Shadow(cascade), &self.instances, None);
                if first == 0 {
                    self.draw_models(&mut pass, &[], Kind::Shadow(cascade), &self.instances);
                }
            }
            Part::Views => {
                // Target cameras, before the main view, whose materials may read them.
                for (target, runs, batch_view) in &self.views {
                    let (id, target) = &self.view_targets[*target];
                    // Probe captures start empty (alpha 0): the sky fills in behind them.
                    let capture = self.environment.is_capture(*id);
                    let clear = if capture { wgpu::Color::TRANSPARENT } else { clear };
                    self.draw_view(encoder, target, runs, *batch_view, clear, time);
                }
                self.environment.probes(encoder);
            }
            Part::Gi => self.record_gi(encoder, time),
            Part::Prepass => {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("3d depth prepass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &targets.depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: time("depth prepass"),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_bind_group(0, &self.frame_bind_group, &[]);
                pass.set_bind_group(2, &self.no_scene, &[]);
                self.draw_models(&mut pass, &self.solid_runs, Kind::Prepass, &self.instances);
                drop(pass);
                self.coverage_prepass(
                    encoder,
                    &targets.color,
                    &targets.depth,
                    &self.frame_bind_group,
                    &self.solid_runs,
                    Some(0),
                );
                // With every solid surface in it, the depth is complete here.
                if self.solid_prepass {
                    self.record_occlusion(encoder, frame);
                }
                if self.velocity_on
                    && let Some(taa) = &targets.velocity
                {
                    // Motion of the models that moved, over the final depth
                    // (read-only); everything else moves with the camera.
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d motion vectors"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &taa.velocity_color,
                            depth_slice: None,
                            resolve_target: Some(taa.velocity()),
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Discard,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &targets.depth,
                            depth_ops: None,
                            stencil_ops: None,
                        }),
                        timestamp_writes: time("motion vectors"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(2, &self.no_scene, &[]);
                    self.draw_models(&mut pass, &self.moved_runs, Kind::Velocity, self.moved.buffer());
                }
                if self.ao_enabled {
                    self.ao
                        .as_ref()
                        .unwrap()
                        .draw(encoder, targets.ao.as_ref().unwrap(), time);
                }
                if let (Some(ssr), Some(images)) = (&self.ssr, &targets.ssr) {
                    ssr.trace(encoder, images, &time);
                }
            }
            Part::Opaque => {
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d opaque"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: self.frame_color(targets).0,
                            depth_slice: None,
                            resolve_target: self.frame_color(targets).1,
                            ops: wgpu::Operations {
                                // Alpha 0 marks the background for the post pass.
                                load: wgpu::LoadOp::Clear(wgpu::Color { a: 0.0, ..clear }),
                                // Without multisampling this is the lit frame itself.
                                store: if split || self.samples <= 1 {
                                    wgpu::StoreOp::Store
                                } else {
                                    wgpu::StoreOp::Discard
                                },
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &targets.depth,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                // Later passes read it (the motion pass and
                                // screen effects too); without a solid
                                // prepass, it's only complete here.
                                store: if split
                                    || self.taa_on
                                    || !self.effects.is_empty()
                                    || !self.target_effects.is_empty()
                                    || !self.full_prepass()
                                {
                                    wgpu::StoreOp::Store
                                } else {
                                    wgpu::StoreOp::Discard
                                },
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: time("opaque"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    let opaque_scene = targets.opaque_scene.as_ref().unwrap_or(&self.no_scene);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(2, opaque_scene, &[]);
                    self.draw_models(&mut pass, &self.solid_runs, Kind::Opaque, &self.instances);
                    if self.atmosphere {
                        pass.set_pipeline(&self.sky);
                        pass.set_bind_group(0, &self.frame_bind_group, &[]);
                        pass.draw(0..3, 0..1);
                        pass.set_bind_group(2, opaque_scene, &[]);
                    }
                    if !split {
                        self.draw_models(&mut pass, &self.blended_runs, Kind::Transparent, &self.instances);
                        self.particles.draw(&mut pass, textures, self.white);
                    }
                }
                if let (Some(ssr), Some(images)) = (&self.ssr, &targets.ssr) {
                    ssr.keep(encoder, images);
                }
                if !self.solid_prepass {
                    self.record_occlusion(encoder, frame);
                }
            }
            Part::Transparent => {
                if split {
                    let lit = targets.lit_texture(self.taa_on);
                    encoder.copy_texture_to_texture(
                        lit.as_image_copy(),
                        targets.scene_texture.as_image_copy(),
                        lit.size(),
                    );
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d transparent"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: self.frame_color(targets).0,
                            depth_slice: None,
                            resolve_target: self.frame_color(targets).1,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: if self.samples <= 1 {
                                    wgpu::StoreOp::Store
                                } else {
                                    wgpu::StoreOp::Discard
                                },
                            },
                        })],
                        // Read-only depth: the shaders sample it at the same time.
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &targets.depth,
                            depth_ops: None,
                            stencil_ops: None,
                        }),
                        timestamp_writes: time("transparent"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(2, &targets.scene, &[]);
                    self.draw_models(&mut pass, &self.blended_runs, Kind::Transparent, &self.instances);
                    self.particles.draw(&mut pass, textures, self.white);
                }
            }
            Part::Post => {
                // Temporal antialiasing first: effects and bloom then see the
                // settled image, in `hdr`.
                if self.taa_on
                    && let (Some(taa), Some(images)) = (&self.taa, &targets.taa)
                {
                    taa.draw(encoder, images, self.taa_frame_read, &targets.hdr, time("taa"));
                }
                // Target effects, in order: each reads the settled frame (in
                // `hdr`) and writes only its own render target, so the screen
                // effects below can sample this frame's results.
                for &(effect, index, material) in &self.target_effects {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d target effect"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.effect_targets[index].2,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: time("target effects"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_pipeline(self.effect_target_pipelines[effect as usize].as_ref().unwrap());
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(1, &self.materials[material as usize].bind_group, &[]);
                    pass.set_bind_group(2, &targets.scene_of_hdr, &[]);
                    if let Some(dummy) = &self.storage_dummy {
                        pass.set_bind_group(3, dummy, &[]);
                    }
                    pass.draw(0..3, 0..1);
                }
                // Screen effects, in order, swapping between `hdr` and the scene
                // texture (free once the transparents are drawn): each reads the
                // frame so far from one and writes the other, with no copies.
                let mut in_hdr = true;
                for &(effect, material) in &self.effects {
                    let (source, target) = if in_hdr {
                        (&targets.scene_of_hdr, &targets.scene_view)
                    } else {
                        (&targets.scene, &targets.hdr)
                    };
                    in_hdr = !in_hdr;
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d screen effect"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: target,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: time("screen effects"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_pipeline(&self.effect_pipelines[effect as usize]);
                    pass.set_bind_group(0, &self.frame_bind_group, &[]);
                    pass.set_bind_group(1, &self.materials[material as usize].bind_group, &[]);
                    pass.set_bind_group(2, source, &[]);
                    if let Some(dummy) = &self.storage_dummy {
                        pass.set_bind_group(3, dummy, &[]);
                    }
                    pass.draw(0..3, 0..1);
                }
                let (post, bloom_prefilter) = if in_hdr {
                    (&targets.post, &targets.bloom_prefilter)
                } else {
                    (&targets.post_of_scene, &targets.bloom_prefilter_of_scene)
                };
                if self.bloom {
                    let step = |encoder: &mut wgpu::CommandEncoder,
                                label: &'static str,
                                pipeline,
                                group: &wgpu::BindGroup,
                                target: &wgpu::TextureView,
                                load| {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(label),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: target,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: time(label),
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, group, &[]);
                        pass.set_blend_constant(wgpu::Color {
                            r: BLOOM_SCATTER,
                            g: BLOOM_SCATTER,
                            b: BLOOM_SCATTER,
                            a: BLOOM_SCATTER,
                        });
                        pass.draw(0..3, 0..1);
                    };
                    let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
                    let mips = &targets.bloom_mips;
                    step(
                        encoder,
                        "bloom",
                        &self.bloom_prefilter,
                        bloom_prefilter,
                        &mips[0],
                        clear,
                    );
                    for (i, group) in targets.bloom_down.iter().enumerate() {
                        step(encoder, "bloom", &self.bloom_down, group, &mips[i + 1], clear);
                    }
                    for (i, group) in targets.bloom_up.iter().enumerate().rev() {
                        step(encoder, "bloom", &self.bloom_up, group, &mips[i], wgpu::LoadOp::Load);
                    }
                }
                // With a render scale, everything up to antialiasing is drawn
                // at the scene's size, then scaled onto the window.
                let output = targets.upscale.as_ref().unwrap_or(view);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("3d post"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: match self.aa {
                            crate::AntiAliasing::None | crate::AntiAliasing::Taa => output,
                            crate::AntiAliasing::Fxaa => targets.ldr.as_ref().unwrap(),
                            crate::AntiAliasing::Smaa => self.smaa.as_ref().unwrap().input(),
                        },
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: time("post"),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.post_pipeline);
                pass.set_bind_group(0, post, &[]);
                pass.set_bind_group(1, &textures[self.lut as usize].bind_group, &[]);
                pass.draw(0..3, 0..1);
                drop(pass);
                match self.aa {
                    crate::AntiAliasing::None | crate::AntiAliasing::Taa => {}
                    crate::AntiAliasing::Smaa => self.smaa.as_ref().unwrap().draw(encoder, output),
                    crate::AntiAliasing::Fxaa => {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("FXAA"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: output,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&self.fxaa_pipeline);
                        pass.set_bind_group(0, targets.aa_group.as_ref().unwrap(), &[]);
                        pass.draw(0..3, 0..1);
                    }
                }
                if let Some(group) = &targets.upscale_group {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("3d upscale"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: time("upscale"),
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass.set_pipeline(&self.upscale_pipeline);
                    pass.set_bind_group(0, group, &[]);
                    pass.draw(0..3, 0..1);
                }
            }
        }
    }

    /// Global illumination's part of the frame (see `gi.rs`): the probe
    /// views' sun shadow map, the views (cut-outs' coverage first, then
    /// lit), and the probes' update from them.
    fn record_gi<'t>(&self, encoder: &mut wgpu::CommandEncoder, time: &PassTimer<'_, 't>) {
        let Some(gi) = &self.gi else {
            return;
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d gi shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gi.shadow,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: time("gi shadows"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let offset = (self.shadow_stride * MAX_SHADOW_VIEWS as u64) as u32;
            pass.set_bind_group(0, &self.shadow_bind_group, &[offset]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            // Models only: instance batches are culled for the main camera.
            self.draw_model_runs(&mut pass, &self.gi_runs, Kind::Shadow(0), &self.instances, None);
        }
        // Each view in its own square of the views image.
        let views = |pass: &mut wgpu::RenderPass<'_>, kind| {
            let size = super::gi::VIEW_SIZE;
            pass.set_bind_group(2, &self.no_scene, &[]);
            for (i, &(start, end)) in self.gi_faces.iter().enumerate() {
                let (x, y) = ((i % 6) as u32 * size, (i / 6) as u32 * size);
                pass.set_viewport(x as f32, y as f32, size as f32, size as f32, 0.0, 1.0);
                pass.set_scissor_rect(x, y, size, size);
                pass.set_bind_group(0, &gi.face_groups[i], &[]);
                self.draw_model_runs(
                    pass,
                    &self.gi_face_runs[start as usize..end as usize],
                    kind,
                    &self.instances,
                    None,
                );
            }
        };
        let pass = |encoder: &mut wgpu::CommandEncoder, label, first: bool, timestamps, kind| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &gi.views,
                    depth_slice: None,
                    resolve_target: (!first).then_some(&gi.resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gi.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: if first {
                            wgpu::LoadOp::Clear(1.0)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timestamps,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            views(&mut pass, kind);
        };
        pass(encoder, "3d gi cut-out views", true, None, Kind::PrepassCoverage);
        pass(encoder, "3d gi views", false, time("gi views"), Kind::Capture);
        let timestamps = time("gi update").map(|t| wgpu::ComputePassTimestampWrites {
            query_set: t.query_set,
            beginning_of_pass_write_index: t.beginning_of_pass_write_index,
            end_of_pass_write_index: t.end_of_pass_write_index,
        });
        gi.update(encoder, timestamps);
    }

    /// One target camera's view: depth prepass, then the lit solid models
    /// and the sky, resolved into its render target.
    fn draw_view<'t>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ViewTarget,
        runs: &[(u32, u32, u32)],
        batch_view: usize,
        clear: wgpu::Color,
        time: &PassTimer<'_, 't>,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d target camera prepass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: time("target cameras"),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &target.frame_bind_group, &[]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            self.draw_models_view(&mut pass, runs, Kind::Prepass, &self.instances, batch_view);
        }
        self.coverage_prepass(
            encoder,
            &target.color,
            &target.depth,
            &target.frame_bind_group,
            runs,
            Some(batch_view),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d target camera"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: if self.samples > 1 {
                    &target.color
                } else {
                    &target.resolve
                },
                depth_slice: None,
                resolve_target: (self.samples > 1).then_some(&target.resolve),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: if self.samples > 1 {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: time("target cameras"),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &target.frame_bind_group, &[]);
        pass.set_bind_group(2, &self.no_scene, &[]);
        self.draw_models_view(&mut pass, runs, Kind::Opaque, &self.instances, batch_view);
        if self.atmosphere {
            pass.set_pipeline(&self.sky);
            pass.set_bind_group(0, &target.frame_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// The motion vector image of the last frame (for tests).
    #[cfg(test)]
    pub(crate) fn velocity_texture(&self) -> Option<&wgpu::Texture> {
        self.targets.as_ref()?.velocity.as_ref().map(|t| &t.velocity_texture)
    }

    /// Why the last prepared frame dropped the temporal history (for tests).
    #[cfg(test)]
    pub(crate) fn history_reset(&self) -> HistoryReset {
        self.history_reset
    }

    /// Whether the targets were made for other settings than these: a
    /// different size (window or render scale), or an effect switched on or
    /// off. They are then made again, and the history with them.
    fn targets_stale(&self, size: (u32, u32), post: &crate::Post, ao_on: bool, upscaled: bool) -> bool {
        self.targets.as_ref().is_none_or(|t| {
            t.size != size
                || t.ao.is_some() != self.ao_enabled
                || t.ao
                    .as_ref()
                    .is_some_and(|a| a.half != (post.ao_half_resolution && ao_on))
                || t.ao.as_ref().is_some_and(|a| a.temporal != (post.ao_temporal && ao_on))
                || t.ssr.is_some() != self.ssr_on
                || t.taa.is_some() != self.taa_on
                || t.velocity.is_some() != self.velocity_on
                || t.ldr.is_some() != (self.aa == crate::AntiAliasing::Fxaa)
                || t.upscale.is_some() != upscaled
        })
    }

    /// Drops every effect's memory of the frames before this one: TAA's
    /// history and the camera motion is measured from, temporal AO's
    /// history and SSR's last image. Each starts over from the next frame
    /// it draws, so a reset asked for on a frame that draws nothing holds.
    fn forget_history(&mut self) {
        self.taa_previous = None;
        if let Some(ao) = &mut self.ao {
            ao.reset();
        }
        if let Some(ssr) = &mut self.ssr {
            ssr.forget();
        }
    }

    /// Whether this frame is resolved by temporal antialiasing (for tests).
    #[cfg(test)]
    pub(crate) fn taa_active(&self) -> bool {
        self.taa_on
    }

    /// Whether a transparent surface this frame reads the scene behind it.
    fn needs_scene(&self) -> bool {
        self.runs
            .iter()
            .map(|&(mesh, _, _)| mesh)
            .chain(
                self.static_batches
                    .iter()
                    .flatten()
                    .flat_map(|b| b.levels.iter().map(|l| l.mesh.id)),
            )
            .any(|mesh| {
                self.meshes[self.base(mesh) as usize]
                    .parts
                    .iter()
                    .enumerate()
                    .any(|(part, &(_, m))| {
                        let m = &self.materials[self.part_material(mesh, part, m) as usize];
                        m.transparent && self.shaders[m.shader as usize].reads_scene
                    })
            })
    }

    /// The mesh a run's mesh id draws: itself, or for a skinned model (ids
    /// past the meshes), the skinned mesh.
    fn base(&self, mesh: u32) -> u32 {
        let mesh = mesh & !FADED;
        if let Some(o) = self.material_override(mesh) {
            return o.mesh;
        }
        match (mesh as usize).checked_sub(self.meshes.len()) {
            None => mesh,
            Some(k) => self.skinning.draws.get(k).map_or(0, |d| d.0),
        }
    }

    /// The mesh and materials a run's mesh id stands for, if it's a
    /// material override (see `Frame3D::material_overrides`).
    fn material_override(&self, mesh: u32) -> Option<&MaterialOverride> {
        let k = (mesh & !FADED).checked_sub(self.override_base)?;
        self.material_overrides.get(k as usize)
    }

    /// The materials a run draws with instead of its mesh's: a material
    /// override's, or a skinned model's own.
    fn run_override(&self, run_mesh: u32) -> Option<&MaterialOverride> {
        let mesh = run_mesh & !FADED;
        match mesh.checked_sub(self.override_base) {
            Some(k) => self.material_overrides.get(k as usize),
            None => {
                let k = (mesh as usize).checked_sub(self.meshes.len())?;
                self.skin_overrides.get(k)
            }
        }
    }

    /// The `(part, material)` list of an override.
    fn override_parts(&self, o: &MaterialOverride) -> &[(u32, u32)] {
        let (start, len) = (o.parts.0 as usize, o.parts.1 as usize);
        self.override_parts.get(start..start + len).unwrap_or(&[])
    }

    /// The material a run draws mesh part `part` with: the part's own, or
    /// the run's override (a per-part one first, then the whole mesh's).
    fn part_material(&self, run_mesh: u32, part: usize, own: u32) -> u32 {
        match self.run_override(run_mesh) {
            Some(o) => MaterialOverride::resolve(
                o.material,
                self.override_parts(o),
                part,
                own,
                self.materials.len() as u32,
            ),
            None => own,
        }
    }

    /// The mesh parts of `runs` that `kind` draws, each with its material
    /// and all its models at once.
    /// With multisampling, the cut-out materials' prepass, with alpha to
    /// coverage: a pass of its own, as coverage needs a color target (`color`,
    /// attached but not written; the opaque pass clears it).
    fn coverage_prepass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame_bind_group: &wgpu::BindGroup,
        runs: &[(u32, u32, u32)],
        batches: Option<usize>,
    ) {
        let cutout = runs.iter().any(|&(mesh, _, _)| {
            self.meshes[self.base(mesh & !FADED) as usize]
                .parts
                .iter()
                .enumerate()
                .any(|(part, &(_, m))| self.materials[self.part_material(mesh, part, m) as usize].cutout)
        });
        let cutout = cutout
            || (batches.is_some()
                && self.static_batches.iter().flatten().any(|b| {
                    b.levels.iter().any(|l| {
                        self.meshes[l.mesh.id as usize]
                            .parts
                            .iter()
                            .any(|(_, m)| self.materials[*m as usize].cutout)
                    })
                }));
        if self.samples <= 1 || !cutout {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d cut-out prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, frame_bind_group, &[]);
        pass.set_bind_group(2, &self.no_scene, &[]);
        if let Some(view) = batches {
            self.draw_models_view(&mut pass, runs, Kind::PrepassCoverage, &self.instances, view);
        } else {
            self.draw_model_runs(&mut pass, runs, Kind::PrepassCoverage, &self.instances, None);
        }
    }

    fn draw_models(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        runs: &[(u32, u32, u32)],
        kind: Kind,
        instances: &wgpu::Buffer,
    ) {
        let view = match kind {
            Kind::Shadow(v) => v + 1,
            _ => 0,
        };
        self.draw_models_view(pass, runs, kind, instances, view);
    }
    fn draw_models_view(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        runs: &[(u32, u32, u32)],
        kind: Kind,
        instances: &wgpu::Buffer,
        view: usize,
    ) {
        self.draw_model_runs(pass, runs, kind, instances, None);
        if kind == Kind::Velocity {
            return;
        }
        for batch in self.static_batches.iter().flatten() {
            for (level, l) in batch.levels.iter().enumerate() {
                if self.meshes.get(l.mesh.id as usize).is_none_or(|m| m.buffers.is_none()) {
                    continue;
                }
                let visible = if batch.compute {
                    batch.view(view, level).is_some()
                } else {
                    batch
                        .visible
                        .get(view * batch.levels.len() + level)
                        .is_some_and(|v| !v.is_empty())
                };
                if !visible {
                    continue;
                }
                self.draw_model_runs(
                    pass,
                    &[(l.mesh.id, 0, batch.count)],
                    kind,
                    if batch.compute {
                        &self.static_culling.as_ref().unwrap().marker
                    } else {
                        &batch.vertices
                    },
                    Some((batch, view, level)),
                );
            }
        }
    }

    fn draw_model_runs(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        runs: &[(u32, u32, u32)],
        kind: Kind,
        instances: &wgpu::Buffer,
        batch: Option<(&StaticBatch, usize, usize)>,
    ) {
        let mut n = StateChanges::default();
        // Pipeline lookups, added to the cache's counters once at the end.
        let mut lookups = 0;
        pass.set_vertex_buffer(1, instances.slice(..));
        n.vertex_buffers += 1;
        if let Some(dummy) = &self.storage_dummy {
            pass.set_bind_group(3, dummy, &[]);
            n.bind_groups += 1;
        }
        // Whether group 3 holds the dummy and slot 1 the whole `instances`
        // (storage meshes and batches change them).
        let (mut dummy_bound, mut instances_bound) = (true, true);
        // Only what changes is set again: each `set_*` costs wgpu
        // validation and a Metal/Vulkan call per draw otherwise.
        let mut bound = None;
        let mut bound_material = u32::MAX;
        for &(mesh, start, models) in runs {
            let fading = mesh & FADED != 0;
            let mesh = mesh & !FADED;
            let run_mesh = mesh;
            let posed = (mesh as usize)
                .checked_sub(self.meshes.len())
                .filter(|_| self.material_override(mesh).is_none());
            let mesh = &self.meshes[self.base(mesh) as usize];
            let storage = if let Some((id, count)) = mesh.gpu_buffer {
                let Some(buffer) = self.storage_buffers.get(id as usize).and_then(Option::as_ref) else {
                    continue;
                };
                Some((buffer, count.min(buffer.len)))
            } else {
                None
            };
            let Some((rest, indices)) = &mesh.buffers else {
                continue;
            };
            let vertices = match posed {
                None => rest,
                Some(k) => match self.skinning.vertices(k) {
                    Some(v) => v,
                    None => continue,
                },
            };
            let mut buffers_set = false;
            for (part, (range, material)) in mesh.parts.iter().enumerate() {
                let material = &self.part_material(run_mesh, part, *material);
                let m = &self.materials[*material as usize];
                // The role for the material's blend in this pass (see
                // `PipelineRole::for_pass`): transparent surfaces are drawn
                // last and cast no shadows.
                let Some(role) = PipelineRole::for_pass(
                    kind.pass(),
                    m.cutout,
                    m.transparent,
                    self.samples > 1,
                    self.full_prepass(),
                ) else {
                    continue;
                };
                let shader = &self.shaders[m.shader as usize];
                // Crossfading instances dither in every pass (their
                // pipelines are built the first time they're drawn, unless
                // `warm_pipelines` built them while loading).
                let variant = ShaderVariant::new(
                    Sides::of(m.double_sided),
                    Instancing::of(fading, storage.is_some() || batch.is_some_and(|(b, _, _)| b.compute)),
                );
                lookups += 1;
                let Some(pipeline) = shader.variant(variant).role(role) else {
                    continue;
                };
                if bound != Some(pipeline as *const _) {
                    pass.set_pipeline(pipeline);
                    n.pipelines += 1;
                    bound = Some(pipeline as *const _);
                }
                if !buffers_set {
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    n.vertex_buffers += 1;
                    n.index_buffers += 1;
                    buffers_set = true;
                }
                if bound_material != *material {
                    pass.set_bind_group(1, &m.bind_group, &[]);
                    n.bind_groups += 1;
                    bound_material = *material;
                }
                if let Some((batch, view, level)) = batch {
                    if let Some(v) = batch.view(view, level) {
                        pass.set_bind_group(3, &v.read, &[]);
                        n.bind_groups += 1;
                        dummy_bound = false;
                        if mesh.parts.len() > 1 {
                            if part > 0 && mesh.parts[part - 1].1 == *material {
                                continue;
                            }
                            let parts = mesh.parts[part..].iter().take_while(|(_, m)| m == material).count();
                            pass.multi_draw_indexed_indirect(&v.args, (part * 20) as u64, parts as u32);
                        } else {
                            pass.draw_indexed_indirect(&v.args, (part * 20) as u64);
                        }
                        n.draws += 1;
                    } else {
                        if let Some(dummy) = self.storage_dummy.as_ref().filter(|_| !dummy_bound) {
                            pass.set_bind_group(3, dummy, &[]);
                            n.bind_groups += 1;
                            dummy_bound = true;
                        }
                        for &(start, count) in &batch.visible[view * batch.levels.len() + level] {
                            pass.draw_indexed(range.clone(), 0, start..start + count);
                            n.draws += 1;
                        }
                    }
                } else if let Some((b, count)) = storage {
                    // Each model gets its own draw: its one instance record
                    // (transform and tint) is shared by all `count` instances.
                    if count > 0 {
                        pass.set_bind_group(3, &b.read, &[]);
                        n.bind_groups += 1;
                        (dummy_bound, instances_bound) = (false, false);
                        let record = if kind == Kind::Velocity {
                            size_of::<MovedInstance>()
                        } else {
                            size_of::<Instance3D>()
                        } as u64;
                        for model in start..start + models {
                            pass.set_vertex_buffer(
                                1,
                                instances.slice(model as u64 * record..(model as u64 + 1) * record),
                            );
                            pass.draw_indexed(range.clone(), 0, 0..count);
                            n.vertex_buffers += 1;
                            n.draws += 1;
                        }
                    }
                } else {
                    if let Some(dummy) = self.storage_dummy.as_ref().filter(|_| !dummy_bound) {
                        pass.set_bind_group(3, dummy, &[]);
                        n.bind_groups += 1;
                        dummy_bound = true;
                    }
                    if !instances_bound {
                        pass.set_vertex_buffer(1, instances.slice(..));
                        n.vertex_buffers += 1;
                        instances_bound = true;
                    }
                    pass.draw_indexed(range.clone(), 0, start..start + models);
                    n.draws += 1;
                }
            }
        }
        self.state_counters.add(&n);
        self.cache.counters.looked_up(lookups);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Shadow(usize),
    Prepass,
    /// Cut-out materials' prepass with alpha to coverage (multisampling).
    PrepassCoverage,
    Opaque,
    Transparent,
    /// Motion vectors of the models that moved (temporal antialiasing).
    Velocity,
    /// Global illumination's probe views: lit, solid surfaces writing
    /// their own depth (cut-outs went through `PrepassCoverage`).
    Capture,
}

impl Kind {
    fn pass(self) -> Pass {
        match self {
            Kind::Shadow(_) => Pass::Shadow,
            Kind::Prepass => Pass::Prepass,
            Kind::PrepassCoverage => Pass::PrepassCoverage,
            Kind::Opaque => Pass::Opaque,
            Kind::Transparent => Pass::Transparent,
            Kind::Velocity => Pass::Velocity,
            Kind::Capture => Pass::Capture,
        }
    }
}

/// A material shader's pipelines: depth prepass, opaque, transparent and
/// shadow, each for one-sided and double-sided surfaces.
/// The pipeline of a compute pass.
fn build_compute(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    name: &str,
    source: &str,
) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(name),
        layout: Some(layout),
        module: &module,
        entry_point: Some("cs_compute"),
        compilation_options: Default::default(),
        cache: None,
    })
}

/// The pipeline of a screen effect: a full-screen triangle into the HDR
/// frame, with the material pipelines' bindings.
fn build_effect(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    name: &str,
    source: &str,
) -> (wgpu::ShaderModule, wgpu::RenderPipeline) {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = effect_pipeline(device, layout, name, &module, "vs_effect", "fs_effect");
    (module, pipeline)
}

/// A full-screen pipeline of an effect shader's entry points, writing `HDR_FORMAT`.
fn effect_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    name: &str,
    module: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(name),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(vertex),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(HDR_FORMAT.into())],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The frame's bindings (group 0) as material shaders see them. The
/// shadow and sky-texture passes bind the frame at a dynamic offset (one
/// per cascade) and stand-ins for the shadow map and sky, which they draw
/// into: a material shader that reads them (from `surface`, which runs
/// for cut-outs' alpha) must build there too.
fn frame_entries(
    stages: wgpu::ShaderStages,
    per_cascade: bool,
    decals: bool,
    gi: bool,
) -> Vec<wgpu::BindGroupLayoutEntry> {
    let mut entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: stages,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: per_cascade,
                min_binding_size: per_cascade
                    .then(|| wgpu::BufferSize::new(size_of::<FrameUniform>() as u64))
                    .flatten(),
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: stages,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: stages,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 6,
            visibility: stages,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Uint,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // Not in compute passes, which would go over WebGPU's 16 sampled
        // textures per stage; they light nothing.
        wgpu::BindGroupLayoutEntry {
            binding: 7,
            visibility: stages - wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
    ];
    if decals {
        // Projected decals' images: the 17th sampled texture of the mesh
        // pipelines, so only where the device has it. Fragment only.
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 8,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 9,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
    }
    if gi {
        // Global illumination's probes: the 18th sampled texture (see
        // `supports_gi`). Fragment only.
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 10,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
    }
    entries
}

fn build_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shadow_layout: &wgpu::PipelineLayout,
    name: &str,
    source: &str,
    samples: u32,
    counters: &Arc<pipeline_cache::Counters>,
) -> ShaderPipelines {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(for_samples(source, samples).into_owned().into()),
    });
    let builder = SideBuilder {
        device: device.clone(),
        module,
        layout: layout.clone(),
        shadow_layout: shadow_layout.clone(),
        samples,
    };
    ShaderPipelines {
        sides: Sides::ALL.map(|sides| builder.timed_build(ShaderVariant::new(sides, Instancing::Plain), counters)),
        fade: Default::default(),
        shared: Default::default(),
        builder,
        counters: counters.clone(),
        in_use: false,
        reads_scene: false,
    }
}

/// Why a frame drops the temporal history: everything an effect kept from
/// earlier frames (TAA's history, temporal AO's, SSR's last image, the
/// camera and models motion vectors are measured from). Decided once per
/// frame in `Renderer3D::prepare`, before any effect prepares, so they all
/// agree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HistoryReset {
    /// The game asked (`Graphics::reset_temporal_history`): a teleport or a new
    /// scene the camera can't tell apart from motion.
    pub requested: bool,
    /// The targets are made again (a resize, a render scale change or an
    /// effect switched): the old history images are gone or the wrong size.
    pub targets: bool,
    /// The camera jumped far or turned sharply (see `camera_cut`): the
    /// last frame shows somewhere else.
    pub camera_cut: bool,
}

impl HistoryReset {
    /// Whether the frame starts over.
    pub fn any(self) -> bool {
        self.requested || self.targets || self.camera_cut
    }
}

/// Whether the camera `now` is a cut from the one `before`: it turned a
/// lot or jumped far in one frame, so what was drawn then is no history.
fn camera_cut(before: Mat4, now: Mat4) -> bool {
    // Where the view starts and which way it looks.
    let eye = |m: Mat4| {
        let inverse = m.inverse();
        let near = inverse.project_point3(Vec3::ZERO);
        let far = inverse.project_point3(Vec3::Z);
        (near, (far - near).normalize_or_zero())
    };
    let (a, b) = (eye(before), eye(now));
    !(a.0.distance(b.0) < TAA_CUT_DISTANCE && a.1.dot(b.1) > TAA_CUT_ANGLE.cos())
}

/// A camera cut: this far apart (world units), or turned this much (radians).
const TAA_CUT_DISTANCE: f32 = 50.0;
const TAA_CUT_ANGLE: f32 = std::f32::consts::FRAC_PI_4;

/// A frame's cluster mapping for the shader: the lights' near plane and
/// slice scale, then how many decals there are and where their cell table
/// starts in the light lists (in words).
fn cluster_depth(frame: &Frame3D) -> [f32; 4] {
    let mut depth = frame.lights.depth;
    depth[2] = frame.decals.records.len() as f32;
    depth[3] = frame.lights.lists.len() as f32;
    depth
}

/// What it takes to build a material shader's pipelines, kept to build the
/// crossfade variants when a frame first needs them.
struct SideBuilder {
    device: wgpu::Device,
    module: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    shadow_layout: wgpu::PipelineLayout,
    samples: u32,
}

impl SideBuilder {
    /// [`SideBuilder::build`], timed into `counters`.
    fn timed_build(&self, variant: ShaderVariant, counters: &pipeline_cache::Counters) -> Pipelines {
        let start = crate::Instant::now();
        let pipelines = self.build(variant);
        counters.built(start.elapsed());
        pipelines
    }

    /// The pipeline set of `variant`: its sides pick the culling, its
    /// instancing the instance buffer's stride and (crossfading) the
    /// dithering fragment entry points; each role's entry points come from
    /// [`PipelineRole`].
    fn build(&self, variant: ShaderVariant) -> Pipelines {
        let cull = variant.sides.cull();
        let shared = variant.instancing.shared();
        let vs = PipelineRole::vertex_entry;
        let fs = |role: PipelineRole| role.fragment_entry(variant.instancing);
        let (device, module, layout, shadow_layout, samples) = (
            &self.device,
            &self.module,
            &self.layout,
            &self.shadow_layout,
            self.samples,
        );
        let buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRIBUTES,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: if shared { 0 } else { size_of::<Instance3D>() as u64 },
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &INSTANCE_ATTRIBUTES,
            }),
        ];
        let depth = |write, compare, bias| {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(write),
                depth_compare: Some(compare),
                stencil: Default::default(),
                bias,
            })
        };
        let msaa = wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        };
        let color = |blend| {
            [Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })]
        };
        let opaque_target = color(None);
        let transparent_target = color(Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING));
        let pipeline = |label: &str,
                        layout: &wgpu::PipelineLayout,
                        vertex: &str,
                        fragment: Option<&str>,
                        cull: Option<wgpu::Face>,
                        depth_stencil,
                        multisample,
                        targets: &[Option<wgpu::ColorTargetState>]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: cull,
                    ..Default::default()
                },
                depth_stencil,
                multisample,
                fragment: fragment.map(|fragment| wgpu::FragmentState {
                    module,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets,
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        // Moved models: only the motion, wherever the opaque pass drew them.
        let moved_buffers = [
            buffers[0].clone(),
            Some(wgpu::VertexBufferLayout {
                array_stride: if shared { 0 } else { size_of::<MovedInstance>() as u64 },
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &MOVED_ATTRIBUTES,
            }),
        ];
        let velocity_target = [Some(wgpu::ColorTargetState {
            format: super::taa::VELOCITY_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let velocity = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d motion vectors"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some(vs(PipelineRole::Velocity)),
                compilation_options: Default::default(),
                buffers: &moved_buffers,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: cull,
                ..Default::default()
            },
            depth_stencil: depth(false, wgpu::CompareFunction::Equal, Default::default()),
            multisample: msaa,
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: fs(PipelineRole::Velocity),
                compilation_options: Default::default(),
                targets: &velocity_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_bias = wgpu::DepthBiasState {
            constant: 2,
            slope_scale: 2.0,
            clamp: 0.0,
        };
        // Solid surfaces write depth with no fragment shader at all, so the
        // GPU can reject hidden ones early; cut-out ones run it for the alpha.
        let prepass = |role: PipelineRole| {
            pipeline(
                "3d depth prepass",
                layout,
                vs(role),
                fs(role),
                cull,
                depth(true, wgpu::CompareFunction::Less, Default::default()),
                msaa,
                &[],
            )
        };
        // Both faces cast: thin things (leaves, planes) too. The stored depth
        // is pushed back a little, against shadow acne.
        let shadow = |role: PipelineRole| {
            pipeline(
                "3d shadow",
                shadow_layout,
                vs(role),
                fs(role),
                None,
                depth(true, wgpu::CompareFunction::Less, shadow_bias),
                wgpu::MultisampleState::default(),
                &[],
            )
        };
        let coverage_msaa = wgpu::MultisampleState {
            alpha_to_coverage_enabled: true,
            ..msaa
        };
        let unwritten = [Some(wgpu::ColorTargetState {
            format: HDR_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::empty(),
        })];
        let coverage = (samples > 1).then(|| {
            [
                pipeline(
                    "3d depth prepass coverage",
                    layout,
                    vs(PipelineRole::PrepassCoverage),
                    fs(PipelineRole::PrepassCoverage),
                    cull,
                    depth(true, wgpu::CompareFunction::Less, Default::default()),
                    coverage_msaa,
                    &unwritten,
                ),
                pipeline(
                    "3d opaque coverage",
                    layout,
                    vs(PipelineRole::OpaqueCoverage),
                    fs(PipelineRole::OpaqueCoverage),
                    cull,
                    depth(false, wgpu::CompareFunction::Equal, Default::default()),
                    coverage_msaa,
                    &opaque_target,
                ),
            ]
        });
        Pipelines {
            coverage,
            prepass: prepass(PipelineRole::Prepass),
            prepass_cutout: prepass(PipelineRole::PrepassCutout),
            // Only the fragments the prepass found nearest: each pixel is lit once.
            opaque: pipeline(
                "3d opaque",
                layout,
                vs(PipelineRole::Opaque),
                fs(PipelineRole::Opaque),
                cull,
                depth(false, wgpu::CompareFunction::Equal, Default::default()),
                msaa,
                &opaque_target,
            ),
            opaque_solid: pipeline(
                "3d opaque solid",
                layout,
                vs(PipelineRole::OpaqueSolid),
                fs(PipelineRole::OpaqueSolid),
                cull,
                depth(true, wgpu::CompareFunction::Less, Default::default()),
                msaa,
                &opaque_target,
            ),
            // Over the opaque frame, without hiding what's behind it.
            transparent: pipeline(
                "3d transparent",
                layout,
                vs(PipelineRole::Transparent),
                fs(PipelineRole::Transparent),
                cull,
                depth(false, wgpu::CompareFunction::Less, Default::default()),
                msaa,
                &transparent_target,
            ),
            shadow: shadow(PipelineRole::Shadow),
            shadow_cutout: shadow(PipelineRole::ShadowCutout),
            velocity,
        }
    }
}

/// What transparent shaders read of the scene behind them.
pub(super) fn scene_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    color: &wgpu::TextureView,
    depth: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("3d scene"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(color),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(depth),
            },
        ],
    })
}

/// A view of a whole texture array, as materials read it.
fn array_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("3d texture array"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_uniform_matches_shader_layout() {
        let module = wgpu::naga::front::wgsl::parse_str(&compose_shader("")).unwrap();
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("Frame"))
            .unwrap();
        let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("Frame must be a struct");
        };
        assert_eq!(*span as usize, size_of::<FrameUniform>());
        for (name, offset) in [
            ("fog_far", std::mem::offset_of!(FrameUniform, fog_far)),
            ("probes", std::mem::offset_of!(FrameUniform, probes)),
            ("gi", std::mem::offset_of!(FrameUniform, gi)),
            (
                "unjittered_view_proj",
                std::mem::offset_of!(FrameUniform, unjittered_view_proj),
            ),
            (
                "previous_view_proj",
                std::mem::offset_of!(FrameUniform, previous_view_proj),
            ),
            ("jitter", std::mem::offset_of!(FrameUniform, jitter)),
            ("cluster_depth", std::mem::offset_of!(FrameUniform, cluster_depth)),
            (
                "cluster_view_proj",
                std::mem::offset_of!(FrameUniform, cluster_view_proj),
            ),
        ] {
            assert_eq!(
                members.iter().find(|m| m.name.as_deref() == Some(name)).unwrap().offset as usize,
                offset,
                "{name}"
            );
        }
    }

    #[test]
    fn custom_materials_can_evaluate_punctual_lighting() {
        check_shader(
            "punctual",
            &compose_shader(
                "fn surface(s: Surface) -> Surface { var o = s; o.emissive = punctual_light(s); return o; }",
            ),
        )
        .unwrap();
    }

    #[test]
    fn finds_defined_functions() {
        assert!(defines("fn vertex(v: Vertex) -> Vertex { return v; }", "vertex"));
        assert!(defines("fn  surface (s: Surface) -> Surface", "surface"));
        assert!(!defines("fn vertex_helper() {}", "vertex"));
        assert!(!defines("fn myvertex() {}", "vertex"));
        assert!(!defines("let fnvertex = 1;", "vertex"));
    }

    #[test]
    fn the_built_in_shader_is_valid() {
        check_shader("built-in", &compose_shader("")).unwrap();
    }

    #[test]
    fn compute_passes_compose_into_valid_shaders() {
        let blur = "fn compute(c: Cell) -> vec4<f32> {
            let around = cell_at(c.pixel + vec2<i32>(1, 0)) + cell_at(c.pixel - vec2<i32>(1, 0));
            return mix(c.value, around * 0.5, material.custom[0].x * frame.time.y);
        }";
        check_shader("blur", &compose_compute(blur)).unwrap();
        let err = check_shader(
            "bad.wgsl",
            &compose_compute("fn compute(c: Cell) -> f32 { return 1.0; }"),
        )
        .unwrap_err();
        assert!(err.contains("bad.wgsl"), "{err}");
    }

    #[test]
    fn every_built_in_shader_is_valid() {
        // Compiled only on a GPU, so naga checks them here (a reserved word
        // once crashed every textured material at startup).
        check_shader("mipmaps", super::super::mips::SHADER).unwrap();
        check_shader("post", include_str!("post.wgsl")).unwrap();
        check_shader("sprite", include_str!("sprite.wgsl")).unwrap();
    }

    #[test]
    fn the_sky_shader_is_valid() {
        check_shader("sky", &format!("{MESH_PRELUDE}\n{SKY}")).unwrap();
    }

    #[test]
    fn the_environment_shader_is_valid() {
        check_shader("environment", include_str!("environment.wgsl")).unwrap();
    }

    #[test]
    fn a_shader_with_both_hooks_is_valid() {
        let user = "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.y += sin(frame.time.x); return o; }\n\
                    fn surface(s: Surface) -> Surface { var o = s; o.albedo *= noise2(s.uv * 8.0); return o; }";
        check_shader("both", &compose_shader(user)).unwrap();
    }

    #[test]
    fn shadow_passes_bind_everything_a_material_shader_reads() {
        let user = "fn surface(s: Surface) -> Surface { var o = s; o.emissive = sky_radiance(s.normal) * sunlight(s.world, s.normal); return o; }";
        let module = wgpu::naga::front::wgsl::parse_str(&compose_shader(user)).unwrap();
        let shadow: Vec<u32> = frame_entries(wgpu::ShaderStages::VERTEX_FRAGMENT, true, true, true)
            .iter()
            .map(|e| e.binding)
            .collect();
        let frame: Vec<u32> = frame_entries(wgpu::ShaderStages::VERTEX_FRAGMENT, false, true, true)
            .iter()
            .map(|e| e.binding)
            .collect();
        for (_, g) in module.global_variables.iter() {
            if let Some(b) = &g.binding
                && b.group == 0
            {
                assert!(
                    shadow.contains(&b.binding),
                    "group 0 binding {} is not in the shadow passes",
                    b.binding
                );
                assert!(
                    frame.contains(&b.binding),
                    "group 0 binding {} is not in the frame",
                    b.binding
                );
            }
        }
    }

    #[test]
    fn shaders_read_the_second_uv_set() {
        let user = "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.y += v.uv1.x; return o; }\n\
                    fn surface(s: Surface) -> Surface { var o = s; o.albedo *= s.uv1.y; return o; }";
        check_shader("uv1", &compose_shader(user)).unwrap();
    }

    #[test]
    fn shaders_read_the_model_origin() {
        let user = "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.x += sin(v.origin.x); return o; }\n\
                    fn surface(s: Surface) -> Surface { var o = s; o.albedo *= fract(s.origin.x * 0.37); return o; }";
        check_shader("origin", &compose_shader(user)).unwrap();
    }

    #[test]
    fn shader_errors_name_the_file() {
        let e = check_shader(
            "water.wgsl",
            &compose_shader("fn surface(s: Surface) -> Surface { return 1; }"),
        )
        .unwrap_err();
        assert!(e.contains("water.wgsl"), "{e}");
    }
}

#[cfg(test)]
#[path = "material_render_test.rs"]
mod material_render_test;

#[cfg(test)]
#[path = "capabilities_render_test.rs"]
mod capabilities_render_test;
#[cfg(test)]
#[path = "shader_reload_render_test.rs"]
mod shader_reload_render_test;
