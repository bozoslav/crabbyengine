//! The 3D passes: instanced meshes with a depth buffer and 4x multisampling,
//! drawn before sprites. Made the first time a game creates a mesh or
//! material.
//!
//! The frame, in order:
//! 1. **shadows**: the sun's shadow map;
//! 2. **depth prepass**: the nearest depth of every pixel, so the next pass
//!    lights each pixel once, and later passes can read the depth;
//! 3. **opaque**: lit models into an HDR (16-bit float) target, keeping only
//!    the fragments the prepass found nearest;
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
use crate::three::{Frame3D, Instance3D, MaterialUniform, MeshData, PostUniform, SHADOW_MAP_SIZE, linear};
use crate::{Lighting, Material};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The lit frame before tone mapping: light brighter than white survives.
const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Samples per pixel: smooth edges.
const MSAA: u32 = 4;

/// The frame's camera and light, as the shader reads them.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    sun_view_proj: [[f32; 4]; 4],
    camera: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky: [f32; 4],
    ground: [f32; 4],
    /// x: shadows on (1) or off (0), y: one shadow-map texel in uv,
    /// z: how far the shadow lookup moves along the normal (world units),
    /// w: where shadows fade out (distance from the camera).
    shadow: [f32; 4],
}

/// One corner of a mesh on the GPU.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// Index range and material of each part.
    parts: Vec<(Range<u32>, u32)>,
    /// Every part's indices, for the shadow pass (materials don't matter there).
    all: Range<u32>,
}

struct GpuMaterial {
    bind_group: wgpu::BindGroup,
    /// The renderer's texture to sample (white when the material has none).
    texture: u32,
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

/// What `Renderer3D::prepare` needs for a frame.
pub(crate) struct Prepare3D<'a> {
    pub frame: &'a Frame3D,
    pub lighting: &'a Lighting,
    pub camera: Vec3,
    pub post: &'a crate::Post,
    /// The lookup table's texture and its size, when there is one.
    pub lut: Option<(u32, u32)>,
    pub size: (u32, u32),
}

pub(crate) struct Renderer3D {
    pipeline: wgpu::RenderPipeline,
    prepass_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
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
    frame_bind_group: wgpu::BindGroup,
    /// The frame uniform alone, for the shadow pass (which can't also read
    /// the shadow map it draws into).
    shadow_bind_group: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    material_layout: wgpu::BindGroupLayout,
    meshes: Vec<GpuMesh>,
    materials: Vec<GpuMaterial>,
    instances: wgpu::Buffer,
    /// Capacity of `instances`, in instances.
    capacity: usize,
    targets: Option<Targets>,
    /// A 1x1 white texture, for materials without one.
    white: u32,
    /// This frame's runs: (mesh, first instance, count).
    runs: Vec<(u32, u32, u32)>,
    shadow_runs: Vec<(u32, u32, u32)>,
}

impl Renderer3D {
    pub fn new(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        white: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));
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
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d frame layout"),
            entries: &[
                uniform_entry(wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d shadow layout"),
            entries: &[uniform_entry(wgpu::ShaderStages::VERTEX)],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d material layout"),
            entries: &[uniform_entry(wgpu::ShaderStages::FRAGMENT)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d pipeline layout"),
            bind_group_layouts: &[Some(&frame_layout), Some(texture_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let instance_attributes = wgpu::vertex_attr_array![
            3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, // model matrix
            7 => Float32x4, // tint
        ];
        let buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance3D>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attributes,
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
        let culled = wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        };
        let prepass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d depth prepass pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: culled,
            depth_stencil: depth(true, wgpu::CompareFunction::Less, Default::default()),
            multisample: msaa,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_prepass"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: culled,
            // Only the fragments the prepass found nearest: each pixel is lit once.
            depth_stencil: depth(false, wgpu::CompareFunction::Equal, Default::default()),
            multisample: msaa,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d shadow pipeline layout"),
            bind_group_layouts: &[Some(&shadow_layout)],
            immediate_size: 0,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d shadow pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            // Both faces: thin things (leaves, planes) cast too.
            primitive: wgpu::PrimitiveState::default(),
            // Pushes the stored depth back a little, against shadow acne.
            depth_stencil: depth(
                true,
                wgpu::CompareFunction::Less,
                wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            ),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
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
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_map.create_view(&Default::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d shadow sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d frame"),
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d shadow"),
            layout: &shadow_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buffer.as_entire_binding(),
            }],
        });
        let capacity = 1024;
        Self {
            pipeline,
            prepass_pipeline,
            shadow_pipeline,
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
            frame_bind_group,
            shadow_bind_group,
            shadow_view,
            material_layout,
            meshes: Vec::new(),
            materials: Vec::new(),
            instances: Self::instance_buffer(device, capacity),
            capacity,
            targets: None,
            white,
            runs: Vec::new(),
            shadow_runs: Vec::new(),
        }
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d instances"),
            size: (capacity * size_of::<Instance3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn create_material(&mut self, device: &wgpu::Device, uniform: &MaterialUniform, texture: Option<u32>) -> u32 {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d material"),
            contents: bytemuck::bytes_of(uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d material"),
            layout: &self.material_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        self.materials.push(GpuMaterial {
            bind_group,
            texture: texture.unwrap_or(self.white),
        });
        self.materials.len() as u32 - 1
    }

    pub fn create_mesh(&mut self, device: &wgpu::Device, data: &MeshData, parts: &[(Range<u32>, Material)]) -> u32 {
        let normals = data.normals_or_computed();
        let vertices: Vec<Vertex> = (0..data.positions.len())
            .map(|i| Vertex {
                position: data.positions[i].to_array(),
                normal: normals[i].to_array(),
                uv: data.uvs.get(i).map_or([0.0; 2], |uv| uv.to_array()),
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
            vertices,
            indices,
            parts: parts.iter().map(|(r, m)| (r.clone(), m.id)).collect(),
            all: 0..data.indices.len() as u32,
        });
        self.meshes.len() as u32 - 1
    }

    /// Uploads this frame's instances, camera, light and post settings.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, input: &Prepare3D<'_>) {
        let &Prepare3D {
            frame,
            lighting,
            camera,
            post,
            lut,
            size,
        } = input;
        queue.write_buffer(
            &self.post_buffer,
            0,
            bytemuck::bytes_of(&PostUniform::new(post, lut.map(|(_, n)| n))),
        );
        self.lut = lut.map_or(self.white, |(id, _)| id);
        self.bloom = post.bloom > 0.0;
        self.runs.clear();
        self.runs.extend_from_slice(&frame.runs);
        self.shadow_runs.clear();
        self.shadow_runs.extend_from_slice(&frame.shadow_runs);
        if frame.instances.is_empty() {
            return;
        }
        if frame.instances.len() > self.capacity {
            self.capacity = frame.instances.len().next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&frame.instances));
        let scaled = |c, k: f32| {
            let [r, g, b, _] = linear(c);
            [r * k, g * k, b * k, 0.0]
        };
        let sun = (-lighting.sun_direction).try_normalize().unwrap_or(Vec3::Y);
        // The shadow lookup moves out along the normal by about one texel,
        // against acne on surfaces facing away from the sun.
        let texel_world = lighting.shadow_distance * 2.0 / SHADOW_MAP_SIZE as f32;
        let uniform = FrameUniform {
            view_proj: frame.view_proj.to_cols_array_2d(),
            sun_view_proj: frame.sun_view_proj.unwrap_or(Mat4::IDENTITY).to_cols_array_2d(),
            camera: camera.extend(1.0).to_array(),
            sun_dir: sun.extend(0.0).to_array(),
            sun_color: scaled(lighting.sun_color, lighting.sun_intensity),
            sky: scaled(lighting.sky_color, lighting.ambient_intensity),
            ground: scaled(lighting.ground_color, lighting.ambient_intensity),
            shadow: [
                if frame.sun_view_proj.is_some() { 1.0 } else { 0.0 },
                1.0 / SHADOW_MAP_SIZE as f32,
                texel_world * 1.5,
                lighting.shadow_distance,
            ],
        };
        queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&uniform));
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            let target = |label, format, samples, usage| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
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
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | usage,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            let none = wgpu::TextureUsages::empty();
            let hdr = target("3d hdr", HDR_FORMAT, 1, wgpu::TextureUsages::TEXTURE_BINDING);
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
                depth: target("3d depth", DEPTH_FORMAT, MSAA, none),
                post: group(&hdr, &bloom_mips[0]),
                bloom_prefilter: group(&hdr, &self.black),
                bloom_down: (1..n).map(|i| group(&bloom_mips[i - 1], &self.black)).collect(),
                bloom_up: (0..n - 1).map(|i| group(&bloom_mips[i + 1], &self.black)).collect(),
                bloom_mips,
                hdr,
                size,
            });
        }
    }

    /// Draw calls the frame's models take: two per mesh part (depth
    /// prepass and lit), one per mesh in the shadow map, the bloom steps and
    /// the post pass.
    pub fn draw_calls(&self) -> usize {
        let parts: usize = self
            .runs
            .iter()
            .map(|&(mesh, _, _)| self.meshes[mesh as usize].parts.len())
            .sum();
        parts * 2
            + self.shadow_runs.len()
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
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
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
            if !self.shadow_runs.is_empty() {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, &self.shadow_bind_group, &[]);
                pass.set_vertex_buffer(1, self.instances.slice(..));
                for &(mesh, start, count) in &self.shadow_runs {
                    let mesh = &self.meshes[mesh as usize];
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(mesh.all.clone(), 0, start..start + count);
                }
            }
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
            pass.set_pipeline(&self.prepass_pipeline);
            self.draw_models(&mut pass, textures);
        }
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
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
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
            pass.set_pipeline(&self.pipeline);
            self.draw_models(&mut pass, textures);
        }
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
                &targets.bloom_prefilter,
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
        pass.set_bind_group(0, &targets.post, &[]);
        pass.set_bind_group(1, &textures[self.lut as usize].bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Every mesh part of the frame, with its material, all its models at once.
    fn draw_models(&self, pass: &mut wgpu::RenderPass<'_>, textures: &[GpuTexture]) {
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_vertex_buffer(1, self.instances.slice(..));
        for &(mesh, start, count) in &self.runs {
            let mesh = &self.meshes[mesh as usize];
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (range, material) in &mesh.parts {
                let material = &self.materials[*material as usize];
                pass.set_bind_group(1, &textures[material.texture as usize].bind_group, &[]);
                pass.set_bind_group(2, &material.bind_group, &[]);
                pass.draw_indexed(range.clone(), 0, start..start + count);
            }
        }
    }
}
