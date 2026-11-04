//! The 3D passes: instanced meshes with a depth buffer and 4x multisampling,
//! drawn before sprites. Made the first time a game creates a mesh or
//! material.
//!
//! The frame, in order:
//! 0. **sky texture**: with an atmosphere, the sky in every direction;
//! 1. **shadows**: one pass per cascade of the sun's shadow map;
//! 2. **depth prepass**: the nearest depth of every pixel, so the next pass
//!    lights each pixel once, and later passes can read the depth;
//! 3. **opaque**: lit models into an HDR (16-bit float) target, keeping only
//!    the fragments the prepass found nearest, then the sky behind them and
//!    the transparent models over them;
//! 4. **bloom**: bright light is blurred through a chain of ever smaller
//!    images and back up (skipped when bloom is off);
//! 5. **post**: exposure, bloom, color grading and tone mapping, onto the
//!    screen.
//!
//! Sprites and UI are drawn on top afterwards, without tone mapping.

use std::ops::Range;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::gpu::GpuTexture;
use super::timer::GpuTimer;
use crate::Material;
use crate::three::Scene3D;
use crate::three::{
    GpuMaterialDesc, Instance3D, MAX_CASCADES, MaterialTextures, MeshData, PostUniform, SHADOW_MAP_SIZE, linear,
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The lit frame before tone mapping: light brighter than white survives.
pub(super) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Samples per pixel: smooth edges.
const MSAA: u32 = 4;

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
}

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

struct GpuMesh {
    /// Vertex and index buffers; `None` once freed (its parts are gone too).
    buffers: Option<(wgpu::Buffer, wgpu::Buffer)>,
    /// Index range and material of each part.
    parts: Vec<(Range<u32>, u32)>,
}

struct GpuMaterial {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    shader: u32,
    transparent: bool,
    /// Whether its depth passes cut out pixels by alpha ([`Blend::Cutout`]).
    cutout: bool,
    double_sided: bool,
}

/// One material shader's pipelines, for one-sided and double-sided surfaces.
struct ShaderPipelines {
    sides: [Pipelines; 2],
    /// Whether it reads the scene behind it (`scene_color`, `scene_position`,
    /// `scene_distance`): its transparent surfaces then need a copy of it.
    reads_scene: bool,
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
}

/// The built-in parts of every material shader.
const MESH_PRELUDE: &str = include_str!("mesh.wgsl");
const MESH_MAIN: &str = include_str!("mesh_main.wgsl");
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
    let mut source = String::with_capacity(MESH_PRELUDE.len() + user.len() + MESH_MAIN.len() + 200);
    source.push_str(MESH_PRELUDE);
    source.push_str("\n// ---- the game's shader ----\n");
    source.push_str(user);
    source.push_str("\n// ---- the engine's entry points ----\n");
    if !defines(user, "vertex") {
        source.push_str("fn vertex(v: Vertex) -> Vertex { return v; }\n");
    }
    if !defines(user, "surface") {
        source.push_str("fn surface(s: Surface) -> Surface { return s; }\n");
    }
    source.push_str(MESH_MAIN);
    source
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
    source.push_str(COMPUTE_PRELUDE);
    source.push_str("\n// ---- the game's compute pass ----\n");
    source.push_str(user);
    source.push_str("\n// ---- the engine's entry point ----\n");
    source.push_str(COMPUTE_MAIN);
    source
}

/// Checks a composed shader, with an error message that points at the
/// line, as WGSL compilers print them.
pub(crate) fn check_shader(name: &str, source: &str) -> Result<(), String> {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string_with_path(source, name))?;
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .map_err(|e| e.emit_to_string_with_path(source, name))?;
    Ok(())
}

/// Whether `device` runs compute passes: WebGL2 has no compute shaders
/// or storage textures.
fn can_compute(device: &wgpu::Device) -> bool {
    let limits = device.limits();
    limits.max_compute_workgroups_per_dimension > 0 && limits.max_storage_textures_per_shader_stage > 0
}

/// Most bloom steps: the smallest image is 1/128 of the screen.
const BLOOM_MIPS: u32 = 7;
/// How much each bloom step keeps of the wider glow below it.
const BLOOM_SCATTER: f64 = 0.7;

/// Window-sized targets.
struct Targets {
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

/// (mesh, first instance, count): one instanced draw of a mesh.
type Run = (u32, u32, u32);

/// The shared parts of every frame bind group: the shadow map and the sky.
struct FrameSources {
    layout: wgpu::BindGroupLayout,
    shadow_view: wgpu::TextureView,
    shadow_sampler: wgpu::Sampler,
    sky_view: wgpu::TextureView,
    sky_sampler: wgpu::Sampler,
}

impl FrameSources {
    fn bind_group(&self, device: &wgpu::Device, frame: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d frame"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.shadow_view),
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
            ],
        })
    }
}

/// A render target a target camera draws into: multisampled color and
/// depth of its size, resolved into the target's texture, and its own
/// frame uniform.
struct ViewTarget {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::TextureView,
    frame_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
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
    /// Pipeline layouts of the material shaders: lit passes, and the
    /// shadow pass (which can't read the shadow map it draws into).
    layout: wgpu::PipelineLayout,
    shadow_pipeline_layout: wgpu::PipelineLayout,
    /// Each shader's pipelines; 0 is the built-in shader.
    shaders: Vec<ShaderPipelines>,
    material_sampler: wgpu::Sampler,
    scene_layout: wgpu::BindGroupLayout,
    /// The scene bind group of passes that can't see the scene.
    no_scene: wgpu::BindGroup,
    post_pipeline: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    bloom_prefilter: wgpu::RenderPipeline,
    bloom_down: wgpu::RenderPipeline,
    bloom_up: wgpu::RenderPipeline,
    post_buffer: wgpu::Buffer,
    linear_clamp: wgpu::Sampler,
    /// Bound as the bloom input of the bloom steps themselves.
    black: wgpu::TextureView,
    /// This frame's lookup table texture (or white), and whether bloom runs.
    lut: u32,
    bloom: bool,
    frame_buffer: wgpu::Buffer,
    /// The physical sky: made into `sky_view` each frame, then drawn behind
    /// everything. Only with an atmosphere.
    sky_view: wgpu::TextureView,
    sky_lut: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    atmosphere: bool,
    frame_bind_group: wgpu::BindGroup,
    /// What frame bind groups are made of, for each target camera's own.
    frame_sources: FrameSources,
    /// Every render target's texture, by texture id.
    render_targets: Vec<(u32, wgpu::Texture)>,
    /// What target cameras need of the render targets they draw into,
    /// made the first time one does, by texture id.
    view_targets: Vec<(u32, ViewTarget)>,
    /// Compute pass pipelines, by compute id, and their layouts.
    compute_pipelines: Vec<wgpu::ComputePipeline>,
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
    /// instance offset already added to its runs, runs).
    views: Vec<(usize, Vec<Run>)>,
    /// The frame uniform alone, for the shadow pass (which can't also read
    /// the shadow map it draws into): one copy per cascade, each with its
    /// `caster_view_proj`, `shadow_stride` bytes apart.
    shadow_frames: wgpu::Buffer,
    shadow_stride: u64,
    shadow_bind_group: wgpu::BindGroup,
    /// Each cascade's layer of the shadow map, drawn into.
    shadow_layers: Vec<wgpu::TextureView>,
    cascades: usize,
    material_layout: wgpu::BindGroupLayout,
    meshes: Vec<GpuMesh>,
    materials: Vec<GpuMaterial>,
    instances: wgpu::Buffer,
    /// Capacity of `instances`, in instances.
    capacity: usize,
    targets: Option<Targets>,
    /// A 1x1 white texture, for materials without one.
    white: u32,
    /// Screen effect pipelines, by effect id.
    effect_pipelines: Vec<wgpu::RenderPipeline>,
    /// This frame's effects, in order: (effect, material).
    effects: Vec<(u32, u32)>,
    /// Texture arrays for materials (whole, every layer and mip level).
    arrays: Vec<wgpu::TextureView>,
    /// A 1x1 white one-layer array, for materials without one.
    white_array: wgpu::TextureView,
    /// This frame's runs: (mesh, first instance, count).
    runs: Vec<(u32, u32, u32)>,
    shadow_runs: [Vec<(u32, u32, u32)>; MAX_CASCADES],
}

impl Renderer3D {
    pub fn new(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        white: u32,
        white_array: &wgpu::Texture,
    ) -> Self {
        // The material shaders' bindings are seen by compute passes too
        // (they read the frame and their material), where there are any.
        let compute = can_compute(device);
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
        let frame_bindings = frame_entries(all_stages, false);
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d frame layout"),
            entries: &frame_bindings,
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d shadow layout"),
            entries: &frame_entries(all_stages, true),
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
                        multisampled: true,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d pipeline layout"),
            bind_group_layouts: &[Some(&frame_layout), Some(&material_layout), Some(&scene_layout)],
            immediate_size: 0,
        });
        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d shadow pipeline layout"),
            bind_group_layouts: &[Some(&shadow_layout), Some(&material_layout), Some(&scene_layout)],
            immediate_size: 0,
        });
        // Passes that can't see the scene bind a 1x1 stand-in (black, far).
        let no_scene = {
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
                MSAA,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
            scene_bind_group(device, &scene_layout, &color, &depth)
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
        let builtin = build_pipelines(
            device,
            &layout,
            &shadow_pipeline_layout,
            "built-in",
            &compose_shader(""),
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
            "3d bloom up",
            &bloom_pipeline_layout,
            "fs_bloom_up",
            HDR_FORMAT,
            Some(wgpu::BlendState {
                color: lerp,
                alpha: lerp,
            }),
        );
        let post_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d post"),
            size: size_of::<PostUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d frame"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d shadow map"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: MAX_CASCADES as u32,
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
        let shadow_layers = (0..MAX_CASCADES as u32)
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
        let shadow_frames = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d shadow frames"),
            size: shadow_stride * MAX_CASCADES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d shadow sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let sky_view = device
            .create_texture(&wgpu::TextureDescriptor {
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
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
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
            source: wgpu::ShaderSource::Wgsl(format!("{MESH_PRELUDE}\n{SKY}").into()),
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
            MSAA,
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let frame_sources = FrameSources {
            layout: frame_layout,
            shadow_view,
            shadow_sampler,
            sky_view: sky_view.clone(),
            sky_sampler,
        };
        let frame_bind_group = frame_sources.bind_group(device, &frame_buffer);
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
        let no_shadow = stand_in("3d no shadow", DEPTH_FORMAT, wgpu::TextureViewDimension::D2Array);
        let no_sky = stand_in("3d no sky", HDR_FORMAT, wgpu::TextureViewDimension::D2);
        let no_sky_sampler = device.create_sampler(&Default::default());
        let no_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d shadow"),
            layout: &shadow_layout,
            entries: &[
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
            ],
        });
        let capacity = 1024;
        Self {
            layout,
            shadow_pipeline_layout,
            shaders: vec![builtin],
            material_sampler,
            scene_layout,
            no_scene,
            post_pipeline,
            post_layout,
            bloom_prefilter,
            bloom_down,
            bloom_up,
            post_buffer,
            linear_clamp,
            black,
            lut: white,
            bloom: false,
            frame_buffer,
            sky_view,
            sky_lut,
            sky,
            atmosphere: false,
            frame_bind_group,
            frame_sources,
            render_targets: Vec::new(),
            view_targets: Vec::new(),
            compute_pipelines: Vec::new(),
            compute_layouts,
            compute_states: Vec::new(),
            computes: Vec::new(),
            views: Vec::new(),
            shadow_frames,
            shadow_stride,
            shadow_bind_group,
            shadow_layers,
            cascades: 0,
            material_layout,
            meshes: Vec::new(),
            materials: Vec::new(),
            instances: Self::instance_buffer(device, capacity),
            capacity,
            targets: None,
            white,
            effect_pipelines: Vec::new(),
            effects: Vec::new(),
            arrays: Vec::new(),
            white_array: array_view(white_array),
            runs: Vec::new(),
            shadow_runs: Default::default(),
        }
    }

    /// Compiles a screen effect from the game's WGSL; returns its id.
    pub fn create_effect(&mut self, device: &wgpu::Device, name: &str, user: &str) -> Result<u32, String> {
        let source = compose_effect(user);
        check_shader(name, &source)?;
        self.effect_pipelines
            .push(build_effect(device, &self.layout, name, &source));
        Ok(self.effect_pipelines.len() as u32 - 1)
    }

    /// Recompiles effect `id` (hot reload). On an error the old one stays.
    pub fn replace_effect(&mut self, device: &wgpu::Device, id: u32, name: &str, user: &str) -> Result<(), String> {
        let source = compose_effect(user);
        check_shader(name, &source)?;
        if let Some(pipeline) = self.effect_pipelines.get_mut(id as usize) {
            *pipeline = build_effect(device, &self.layout, name, &source);
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
                    sample_count: MSAA,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d target camera frame"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = self.frame_sources.bind_group(device, &frame_buffer);
        let view = ViewTarget {
            color: target("3d target camera color", HDR_FORMAT),
            depth: target("3d target camera depth", DEPTH_FORMAT),
            resolve: texture.create_view(&Default::default()),
            frame_buffer,
            frame_bind_group,
        };
        self.view_targets.push((id, view));
        Some(self.view_targets.len() - 1)
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

    /// Compiles a compute pass from the game's WGSL; returns its id.
    pub fn create_compute(&mut self, device: &wgpu::Device, name: &str, user: &str) -> Result<u32, String> {
        let source = compose_compute(user);
        check_shader(name, &source)?;
        let Some((_, layout)) = &self.compute_layouts else {
            return Err(format!(
                "{name}: this GPU can't run compute passes (WebGL2 has no compute shaders)"
            ));
        };
        let pipeline = build_compute(device, layout, name, &source);
        self.compute_pipelines.push(pipeline);
        Ok(self.compute_pipelines.len() as u32 - 1)
    }

    /// Recompiles compute pass `id` (hot reload). On an error the old one stays.
    pub fn replace_compute(&mut self, device: &wgpu::Device, id: u32, name: &str, user: &str) -> Result<(), String> {
        let source = compose_compute(user);
        check_shader(name, &source)?;
        let Some((_, layout)) = &self.compute_layouts else {
            return Ok(());
        };
        let pipeline = build_compute(device, layout, name, &source);
        if let Some(old) = self.compute_pipelines.get_mut(id as usize) {
            *old = pipeline;
        }
        Ok(())
    }

    /// Keeps a texture array for materials; returns its id.
    pub fn add_texture_array(&mut self, texture: &wgpu::Texture) -> u32 {
        self.arrays.push(array_view(texture));
        self.arrays.len() as u32 - 1
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d instances"),
            size: (capacity * size_of::<Instance3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Compiles a material shader from the game's WGSL; returns its index.
    pub fn create_shader(&mut self, device: &wgpu::Device, name: &str, user: &str) -> Result<u32, String> {
        let source = compose_shader(user);
        check_shader(name, &source)?;
        let mut pipelines = build_pipelines(device, &self.layout, &self.shadow_pipeline_layout, name, &source);
        pipelines.reads_scene = reads_scene(user);
        self.shaders.push(pipelines);
        Ok(self.shaders.len() as u32 - 1)
    }

    /// Recompiles shader `id` (hot reload). On an error the old one stays.
    pub fn replace_shader(&mut self, device: &wgpu::Device, id: u32, name: &str, user: &str) -> Result<(), String> {
        let source = compose_shader(user);
        check_shader(name, &source)?;
        let mut pipelines = build_pipelines(device, &self.layout, &self.shadow_pipeline_layout, name, &source);
        pipelines.reads_scene = reads_scene(user);
        self.shaders[id as usize] = pipelines;
        Ok(())
    }

    fn material_bind_group(
        &self,
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        maps: &MaterialTextures,
        arrays: &[Option<u32>; 3],
        textures: &[GpuTexture],
    ) -> wgpu::BindGroup {
        // Normal and ORM maps hold data, not colors: read them without sRGB decoding.
        let linear = [false, true, true, false, false, false];
        let views: Vec<wgpu::TextureView> = maps
            .iter()
            .zip(linear)
            .map(|(map, linear)| {
                let id = map.filter(|&t| (t as usize) < textures.len()).unwrap_or(self.white);
                let texture = &textures[id as usize].texture;
                // Render targets (HDR) are read as they are.
                let image = texture.format() == wgpu::TextureFormat::Rgba8UnormSrgb;
                texture.create_view(&wgpu::TextureViewDescriptor {
                    format: image.then_some(if linear {
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
        for (i, array) in arrays.iter().enumerate() {
            let view = array
                .and_then(|id| self.arrays.get(id as usize))
                .unwrap_or(&self.white_array);
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
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d material"),
            contents: bytemuck::bytes_of(&desc.uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = self.material_bind_group(device, &buffer, &desc.textures, &desc.arrays, textures);
        self.materials.push(GpuMaterial {
            buffer,
            bind_group,
            shader: desc.shader.min(self.shaders.len() as u32 - 1),
            transparent: desc.transparent,
            cutout: desc.cutout,
            double_sided: desc.double_sided,
        });
        self.materials.len() as u32 - 1
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
        queue.write_buffer(&material.buffer, 0, bytemuck::bytes_of(&desc.uniform));
        let bind_group = self.material_bind_group(device, &material.buffer, &desc.textures, &desc.arrays, textures);
        let shader = desc.shader.min(self.shaders.len() as u32 - 1);
        let material = &mut self.materials[id as usize];
        material.bind_group = bind_group;
        material.shader = shader;
        material.transparent = desc.transparent;
        material.cutout = desc.cutout;
        material.double_sided = desc.double_sided;
    }

    pub fn create_mesh(&mut self, device: &wgpu::Device, data: &MeshData, parts: &[(Range<u32>, Material)]) -> u32 {
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
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d indices"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.push(GpuMesh {
            buffers: Some((vertices, indices)),
            parts: parts.iter().map(|(r, m)| (r.clone(), m.id)).collect(),
        });
        self.meshes.len() as u32 - 1
    }

    /// Frees mesh `id`'s GPU memory; it draws nothing from now on.
    pub fn free_mesh(&mut self, id: u32) {
        if let Some(mesh) = self.meshes.get_mut(id as usize) {
            mesh.buffers = None;
            mesh.parts = Vec::new();
        }
    }

    /// Uploads this frame's instances, camera, light and post settings.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, input: &Prepare3D<'_>) {
        let &Prepare3D { scene, lut, size } = input;
        let &Scene3D {
            frame,
            lighting,
            sky,
            light,
            camera,
            post,
            time,
        } = scene;
        queue.write_buffer(
            &self.post_buffer,
            0,
            bytemuck::bytes_of(&PostUniform::new(post, lut.map(|(_, n)| n))),
        );
        self.lut = lut.map_or(self.white, |(id, _)| id);
        self.bloom = post.bloom > 0.0;
        self.atmosphere = sky.atmosphere;
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
        self.cascades = frame.cascades;
        for (mine, theirs) in self.shadow_runs.iter_mut().zip(&frame.shadow_runs) {
            mine.clear();
            mine.extend_from_slice(theirs);
        }
        self.computes.clear();
        for &(compute, target, material) in &frame.computes {
            if (compute as usize) < self.compute_pipelines.len()
                && (material as usize) < self.materials.len()
                && let Some(state) = self.compute_state(device, target)
            {
                self.computes.push((compute, state, material));
            }
        }
        // Target cameras' instances follow the main view's in one buffer.
        let total = frame.instances.len() + frame.views.iter().map(|v| v.frame.instances.len()).sum::<usize>();
        let mut views = std::mem::take(&mut self.views);
        let mut used = 0;
        let mut offset = frame.instances.len() as u32;
        for view in &frame.views {
            let Some(target) = self.view_target(device, view.target) else {
                continue;
            };
            if used == views.len() {
                views.push((0, Vec::new()));
            }
            let (index, runs) = &mut views[used];
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
        if total == 0 {
            return;
        }
        if total > self.capacity {
            self.capacity = total.next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&frame.instances));
        let mut at = frame.instances.len();
        for view in &frame.views {
            if !view.frame.instances.is_empty() {
                let bytes = (at * size_of::<Instance3D>()) as u64;
                queue.write_buffer(&self.instances, bytes, bytemuck::cast_slice(&view.frame.instances));
            }
            at += view.frame.instances.len();
        }
        let scaled = |c, k: f32| {
            let [r, g, b, _] = linear(c);
            [r * k, g * k, b * k, 0.0]
        };
        let ambient = |light: Vec3| (light * lighting.ambient_intensity).extend(0.0).to_array();
        let sun = (-lighting.sun_direction).try_normalize().unwrap_or(Vec3::Y);
        let mut uniform = FrameUniform {
            view_proj: frame.view_proj.to_cols_array_2d(),
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
                1.0 / SHADOW_MAP_SIZE as f32,
                1.5,
                lighting.shadow_distance,
            ],
            cascade_texel: frame.cascade_texel,
            time: [time.0, time.1, 0.0, 0.0],
            inv_view_proj: frame.view_proj.inverse().to_cols_array_2d(),
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
                0.0,
            ],
            fog_color: scaled(sky.fog_color, 1.0),
            night: scaled(sky.night_light, 1.0),
        };
        queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&uniform));
        // Each target camera's frame: the same, seen from it.
        for (view, &(target, _)) in frame
            .views
            .iter()
            .filter(|v| self.view_targets.iter().any(|&(id, _)| id == v.target))
            .zip(&self.views)
        {
            let mut theirs = uniform;
            theirs.view_proj = view.view_proj.to_cols_array_2d();
            theirs.inv_view_proj = view.view_proj.inverse().to_cols_array_2d();
            theirs.camera = view.camera.extend(1.0).to_array();
            theirs.screen = [
                view.size.0 as f32,
                view.size.1 as f32,
                1.0 / view.size.0.max(1) as f32,
                1.0 / view.size.1.max(1) as f32,
            ];
            theirs.sky_light[3] = (view.camera.y / 1000.0).max(0.0);
            queue.write_buffer(
                &self.view_targets[target].1.frame_buffer,
                0,
                bytemuck::bytes_of(&theirs),
            );
        }
        // Copy 0 also serves the sky texture's pass.
        for i in 0..frame.cascades.max(1) {
            uniform.caster_view_proj = uniform.sun_view_proj[i.min(MAX_CASCADES - 1)];
            queue.write_buffer(
                &self.shadow_frames,
                self.shadow_stride * i as u64,
                bytemuck::bytes_of(&uniform),
            );
        }
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
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
            let depth = target("3d depth", DEPTH_FORMAT, MSAA, wgpu::TextureUsages::TEXTURE_BINDING);
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
            let n = bloom_mips.len();
            self.targets = Some(Targets {
                color: target("3d color", HDR_FORMAT, MSAA, none),
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
    }

    /// Draw calls the frame's models take: two per mesh part (depth
    /// prepass and lit), one per mesh part in each shadow cascade, one per
    /// screen effect, the bloom steps and the post pass.
    pub fn draw_calls(&self) -> usize {
        let parts: usize = self
            .runs
            .iter()
            .map(|&(mesh, _, _)| self.meshes[mesh as usize].parts.len())
            .sum();
        let view_parts: usize = self
            .views
            .iter()
            .flat_map(|(_, runs)| runs)
            .map(|&(mesh, _, _)| self.meshes[mesh as usize].parts.len())
            .sum();
        self.computes.len()
            + (parts + view_parts) * 2
            + self.views.len() * usize::from(self.atmosphere)
            + self.effects.len()
            + if self.atmosphere { 2 } else { 0 }
            + self.shadow_runs[..self.cascades].iter().map(Vec::len).sum::<usize>()
            + 1
            + if self.bloom {
                self.targets.as_ref().map_or(0, |t| t.bloom_mips.len() * 2 - 1)
            } else {
                0
            }
    }

    /// Whether this frame has anything to draw.
    pub fn has_draws(&self) -> bool {
        !self.runs.is_empty()
    }

    /// Draws the frame's passes (see the module docs) and leaves the tone
    /// mapped result in `view`. `clear` is the background, in linear color.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear: wgpu::Color,
        textures: &[GpuTexture],
        timer: Option<&GpuTimer>,
    ) {
        let Some(targets) = &self.targets else { return };
        let time = || timer.and_then(GpuTimer::pass);
        // Compute passes first, so this frame's materials see their results.
        for &(compute, state, material) in &self.computes {
            let state = &self.compute_states[state].1;
            {
                let timestamps = time().map(|t| wgpu::ComputePassTimestampWrites {
                    query_set: t.query_set,
                    beginning_of_pass_write_index: t.beginning_of_pass_write_index,
                    end_of_pass_write_index: t.end_of_pass_write_index,
                });
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("3d compute"),
                    timestamp_writes: timestamps,
                });
                pass.set_pipeline(&self.compute_pipelines[compute as usize]);
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
        if self.atmosphere {
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
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sky_lut);
            pass.set_bind_group(0, &self.shadow_bind_group, &[0]);
            pass.draw(0..3, 0..1);
        }
        for cascade in 0..self.cascades {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_layers[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let offset = (self.shadow_stride * cascade as u64) as u32;
            pass.set_bind_group(0, &self.shadow_bind_group, &[offset]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            self.draw_models(&mut pass, &self.shadow_runs[cascade], Kind::Shadow);
        }
        // Target cameras, before the main view, whose materials may read them.
        for (target, runs) in &self.views {
            let target = &self.view_targets[*target].1;
            self.draw_view(encoder, target, runs, clear, &time);
        }
        {
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
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            self.draw_models(&mut pass, &self.runs, Kind::Prepass);
        }
        // Transparent shaders that read the scene behind them get a copy of
        // the opaque scene: the frame is then drawn in two passes.
        let split = self.needs_scene();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d opaque"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color,
                    depth_slice: None,
                    resolve_target: Some(&targets.hdr),
                    ops: wgpu::Operations {
                        // Alpha 0 marks the background for the post pass.
                        load: wgpu::LoadOp::Clear(wgpu::Color { a: 0.0, ..clear }),
                        store: if split {
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
                        store: if split {
                            wgpu::StoreOp::Store
                        } else {
                            wgpu::StoreOp::Discard
                        },
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            self.draw_models(&mut pass, &self.runs, Kind::Opaque);
            if self.atmosphere {
                pass.set_pipeline(&self.sky);
                pass.set_bind_group(0, &self.frame_bind_group, &[]);
                pass.draw(0..3, 0..1);
                pass.set_bind_group(2, &self.no_scene, &[]);
            }
            if !split {
                self.draw_models(&mut pass, &self.runs, Kind::Transparent);
            }
        }
        if split {
            encoder.copy_texture_to_texture(
                targets.hdr_texture.as_image_copy(),
                targets.scene_texture.as_image_copy(),
                targets.hdr_texture.size(),
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d transparent"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color,
                    depth_slice: None,
                    resolve_target: Some(&targets.hdr),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                // Read-only depth: the shaders sample it at the same time.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.set_bind_group(2, &targets.scene, &[]);
            self.draw_models(&mut pass, &self.runs, Kind::Transparent);
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
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.effect_pipelines[effect as usize]);
            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.set_bind_group(1, &self.materials[material as usize].bind_group, &[]);
            pass.set_bind_group(2, source, &[]);
            pass.draw(0..3, 0..1);
        }
        let (post, bloom_prefilter) = if in_hdr {
            (&targets.post, &targets.bloom_prefilter)
        } else {
            (&targets.post_of_scene, &targets.bloom_prefilter_of_scene)
        };
        if self.bloom {
            let step = |encoder: &mut wgpu::CommandEncoder,
                        label,
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
                    timestamp_writes: time(),
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
                "3d bloom",
                &self.bloom_prefilter,
                bloom_prefilter,
                &mips[0],
                clear,
            );
            for (i, group) in targets.bloom_down.iter().enumerate() {
                step(encoder, "3d bloom down", &self.bloom_down, group, &mips[i + 1], clear);
            }
            for (i, group) in targets.bloom_up.iter().enumerate().rev() {
                step(
                    encoder,
                    "3d bloom up",
                    &self.bloom_up,
                    group,
                    &mips[i],
                    wgpu::LoadOp::Load,
                );
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d post"),
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
            timestamp_writes: time(),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.post_pipeline);
        pass.set_bind_group(0, post, &[]);
        pass.set_bind_group(1, &textures[self.lut as usize].bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// One target camera's view: depth prepass, then the lit solid models
    /// and the sky, resolved into its render target.
    fn draw_view<'t>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ViewTarget,
        runs: &[(u32, u32, u32)],
        clear: wgpu::Color,
        time: &dyn Fn() -> Option<wgpu::RenderPassTimestampWrites<'t>>,
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
                timestamp_writes: time(),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &target.frame_bind_group, &[]);
            pass.set_bind_group(2, &self.no_scene, &[]);
            self.draw_models(&mut pass, runs, Kind::Prepass);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d target camera"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.color,
                depth_slice: None,
                resolve_target: Some(&target.resolve),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Discard,
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
            timestamp_writes: time(),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &target.frame_bind_group, &[]);
        pass.set_bind_group(2, &self.no_scene, &[]);
        self.draw_models(&mut pass, runs, Kind::Opaque);
        if self.atmosphere {
            pass.set_pipeline(&self.sky);
            pass.set_bind_group(0, &target.frame_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Whether a transparent surface this frame reads the scene behind it.
    fn needs_scene(&self) -> bool {
        self.runs.iter().any(|&(mesh, _, _)| {
            self.meshes[mesh as usize].parts.iter().any(|&(_, m)| {
                let m = &self.materials[m as usize];
                m.transparent && self.shaders[m.shader as usize].reads_scene
            })
        })
    }

    /// The mesh parts of `runs` that `kind` draws, each with its material
    /// and all its models at once.
    fn draw_models(&self, pass: &mut wgpu::RenderPass<'_>, runs: &[(u32, u32, u32)], kind: Kind) {
        pass.set_vertex_buffer(1, self.instances.slice(..));
        let mut bound = None;
        for &(mesh, start, count) in runs {
            let mesh = &self.meshes[mesh as usize];
            let Some((vertices, indices)) = &mesh.buffers else {
                continue;
            };
            let mut buffers_set = false;
            for (range, material) in &mesh.parts {
                let m = &self.materials[*material as usize];
                // Transparent surfaces are drawn last and cast no shadows.
                if m.transparent != (kind == Kind::Transparent) {
                    continue;
                }
                let pipelines = &self.shaders[m.shader as usize].sides[usize::from(m.double_sided)];
                let pipeline = match kind {
                    Kind::Shadow if m.cutout => &pipelines.shadow_cutout,
                    Kind::Shadow => &pipelines.shadow,
                    Kind::Prepass if m.cutout => &pipelines.prepass_cutout,
                    Kind::Prepass => &pipelines.prepass,
                    Kind::Opaque => &pipelines.opaque,
                    Kind::Transparent => &pipelines.transparent,
                };
                if bound != Some(pipeline as *const _) {
                    pass.set_pipeline(pipeline);
                    bound = Some(pipeline as *const _);
                }
                if !buffers_set {
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    buffers_set = true;
                }
                pass.set_bind_group(1, &m.bind_group, &[]);
                pass.draw_indexed(range.clone(), 0, start..start + count);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Shadow,
    Prepass,
    Opaque,
    Transparent,
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
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(name),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_effect"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_effect"),
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
fn frame_entries(stages: wgpu::ShaderStages, per_cascade: bool) -> Vec<wgpu::BindGroupLayoutEntry> {
    vec![
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
    ]
}

fn build_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shadow_layout: &wgpu::PipelineLayout,
    name: &str,
    source: &str,
) -> ShaderPipelines {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<Instance3D>() as u64,
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
        count: MSAA,
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
                module: &module,
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
                module: &module,
                entry_point: Some(fragment),
                compilation_options: Default::default(),
                targets,
            }),
            multiview_mask: None,
            cache: None,
        })
    };
    let shadow_bias = wgpu::DepthBiasState {
        constant: 2,
        slope_scale: 2.0,
        clamp: 0.0,
    };
    // Solid surfaces write depth with no fragment shader at all, so the
    // GPU can reject hidden ones early; cut-out ones run it for the alpha.
    let prepass = |fragment, cull| {
        pipeline(
            "3d depth prepass",
            layout,
            "vs_main",
            fragment,
            cull,
            depth(true, wgpu::CompareFunction::Less, Default::default()),
            msaa,
            &[],
        )
    };
    // Both faces cast: thin things (leaves, planes) too. The stored depth
    // is pushed back a little, against shadow acne.
    let shadow = |fragment| {
        pipeline(
            "3d shadow",
            shadow_layout,
            "vs_shadow",
            fragment,
            None,
            depth(true, wgpu::CompareFunction::Less, shadow_bias),
            wgpu::MultisampleState::default(),
            &[],
        )
    };
    let side = |cull: Option<wgpu::Face>| Pipelines {
        prepass: prepass(None, cull),
        prepass_cutout: prepass(Some("fs_depth"), cull),
        // Only the fragments the prepass found nearest: each pixel is lit once.
        opaque: pipeline(
            "3d opaque",
            layout,
            "vs_main",
            Some("fs_main"),
            cull,
            depth(false, wgpu::CompareFunction::Equal, Default::default()),
            msaa,
            &opaque_target,
        ),
        // Over the opaque frame, without hiding what's behind it.
        transparent: pipeline(
            "3d transparent",
            layout,
            "vs_main",
            Some("fs_transparent"),
            cull,
            depth(false, wgpu::CompareFunction::Less, Default::default()),
            msaa,
            &transparent_target,
        ),
        shadow: shadow(None),
        shadow_cutout: shadow(Some("fs_depth")),
    };
    ShaderPipelines {
        sides: [side(Some(wgpu::Face::Back)), side(None)],
        reads_scene: false,
    }
}

/// What transparent shaders read of the scene behind them.
fn scene_bind_group(
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
    fn a_shader_with_both_hooks_is_valid() {
        let user = "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.y += sin(frame.time.x); return o; }\n\
                    fn surface(s: Surface) -> Surface { var o = s; o.albedo *= noise2(s.uv * 8.0); return o; }";
        check_shader("both", &compose_shader(user)).unwrap();
    }

    #[test]
    fn shadow_passes_bind_everything_a_material_shader_reads() {
        let user = "fn surface(s: Surface) -> Surface { var o = s; o.emissive = sky_radiance(s.normal) * sunlight(s.world, s.normal); return o; }";
        let module = wgpu::naga::front::wgsl::parse_str(&compose_shader(user)).unwrap();
        let shadow: Vec<u32> = frame_entries(wgpu::ShaderStages::VERTEX_FRAGMENT, true)
            .iter()
            .map(|e| e.binding)
            .collect();
        let frame: Vec<u32> = frame_entries(wgpu::ShaderStages::VERTEX_FRAGMENT, false)
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
