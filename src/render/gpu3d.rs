//! The 3D pass: instanced meshes with a depth buffer and 4x multisampling,
//! drawn before sprites, with the sun's shadow map drawn first. Made the
//! first time a game creates a mesh or material.

use std::ops::Range;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::gpu::GpuTexture;
use crate::three::{Frame3D, Instance3D, MaterialUniform, MeshData, SHADOW_MAP_SIZE, linear};
use crate::{Lighting, Material};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
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

/// Window-sized targets: multisampled color and depth.
struct Targets {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    size: (u32, u32),
}

pub(crate) struct Renderer3D {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
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
    format: wgpu::TextureFormat,
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
        let depth = |bias| {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias,
            })
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: depth(Default::default()),
            multisample: wgpu::MultisampleState {
                count: MSAA,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
            depth_stencil: depth(wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
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
            shadow_pipeline,
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
            format,
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

    /// Uploads this frame's instances, camera and light.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame3D,
        lighting: &Lighting,
        camera: Vec3,
        size: (u32, u32),
    ) {
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
            let target = |label, format| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: MSAA,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            };
            self.targets = Some(Targets {
                color: target("3d color", self.format),
                depth: target("3d depth", DEPTH_FORMAT),
                size,
            });
        }
    }

    /// Draw calls the frame's models take: one per mesh part, and one per
    /// mesh in the shadow map.
    pub fn draw_calls(&self) -> usize {
        let parts: usize = self
            .runs
            .iter()
            .map(|&(mesh, _, _)| self.meshes[mesh as usize].parts.len())
            .sum();
        parts + self.shadow_runs.len()
    }

    /// Whether this frame has anything to draw.
    pub fn has_draws(&self) -> bool {
        !self.runs.is_empty()
    }

    /// Draws the shadow map, then the frame's models into `view` (cleared
    /// to `clear` first).
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        clear: wgpu::Color,
        textures: &[GpuTexture],
    ) {
        let Some(targets) = &self.targets else { return };
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
                timestamp_writes: None,
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
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.color,
                depth_slice: None,
                resolve_target: Some(view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
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
