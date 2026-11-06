//! Screen-space reflections (see `ssr.wgsl`): a half-resolution trace after
//! the depth prepass, read by the opaque pass, and a copy of the opaque
//! image after it, which the next frame's trace takes its colors from.
//!
//! Material shaders have no texture slot to spare, so the opaque pass reads
//! the reflections where it reads GTAO: its scene color slot holds a
//! two-level image, GTAO and contact shadows in the first level (full size,
//! red and green), the reflections in the second (half size). Blue 1 in the
//! first level says the second is there: GTAO writes it, or without GTAO a
//! clear of the new image. (GTAO's own RG8 image and the opaque pass's
//! black stand-in read 0 there; GLSL ES can't count a texture's levels.)
use std::sync::atomic::{AtomicBool, Ordering};

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Settings {
    inverse: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    previous: [[f32; 4]; 4],
    camera: [f32; 4],
    params: [f32; 4],
}

/// The image the opaque pass reads GTAO and the reflections from.
pub(super) const SCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub(super) struct Ssr {
    layout: wgpu::BindGroupLayout,
    trace_pipeline: wgpu::RenderPipeline,
    copy_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    /// The camera the image in the history was drawn with, while it holds
    /// the last frame's.
    previous: Option<Mat4>,
    frame: u32,
}

pub(super) struct SsrTargets {
    /// The last frame's opaque image.
    history: wgpu::TextureView,
    /// The two-level image the opaque pass reads (see the module docs).
    pub screen: wgpu::TextureView,
    /// Its first level, GTAO's target.
    pub ao: wgpu::TextureView,
    /// Its second level, the trace's target.
    output: wgpu::TextureView,
    /// Without GTAO, until the first level has its blue mark.
    unmarked: AtomicBool,
    trace_group: wgpu::BindGroup,
    copy_group: wgpu::BindGroup,
}

impl Ssr {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = |binding, sample_type, multisampled| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen reflections"),
            entries: &[
                texture(0, wgpu::TextureSampleType::Depth, true),
                texture(1, wgpu::TextureSampleType::Float { filterable: true }, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("ssr.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen reflections"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry, format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let trace_pipeline = pipeline("fs_trace", SCREEN_FORMAT);
        let copy_pipeline = pipeline("fs_copy", super::gpu3d::HDR_FORMAT);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen reflections"),
            contents: bytemuck::bytes_of(&<Settings as bytemuck::Zeroable>::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("screen reflections"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layout,
            trace_pipeline,
            copy_pipeline,
            uniform,
            sampler,
            previous: None,
            frame: 0,
        }
    }

    /// Forgets the last frame's image (reflections are off, or it can't be
    /// reprojected): the next frame reflects only the environment.
    pub fn forget(&mut self) {
        self.previous = None;
    }

    /// This frame's settings: the camera it's drawn with (jittered, with
    /// TAA), `strength` 0 to 1 and the longest ray in world units. Call
    /// only for frames that are drawn; their image is the next frame's
    /// history.
    pub fn prepare(&mut self, queue: &wgpu::Queue, view_proj: Mat4, camera: Vec3, strength: f32, longest: f32) {
        let previous = self.previous.replace(view_proj);
        self.frame = self.frame.wrapping_add(1);
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Settings {
                inverse: view_proj.inverse().to_cols_array_2d(),
                projection: view_proj.to_cols_array_2d(),
                previous: previous.unwrap_or(view_proj).to_cols_array_2d(),
                camera: camera.extend(1.0).to_array(),
                params: [
                    if previous.is_some() { strength } else { 0.0 },
                    longest,
                    (self.frame % 64) as f32,
                    0.0,
                ],
            }),
        );
    }

    /// The camera the history was drawn with, if it holds the last frame.
    pub fn previous(&self) -> Option<Mat4> {
        self.previous
    }

    /// Targets for frames of `size`: `depth` the prepass's, `lit` the image
    /// the opaque pass resolves into; `ao` whether GTAO writes `ao`.
    pub fn targets(
        &self,
        device: &wgpu::Device,
        size: (u32, u32),
        depth: &wgpu::TextureView,
        lit: &wgpu::TextureView,
        ao: bool,
    ) -> SsrTargets {
        let texture = |label, size: (u32, u32), levels, format| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: levels,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let history =
            texture("screen reflections history", size, 1, super::gpu3d::HDR_FORMAT).create_view(&Default::default());
        // At least 2 pixels across, for the second level.
        let screen = texture("screen reflections", (size.0.max(2), size.1.max(2)), 2, SCREEN_FORMAT);
        let level = |level| {
            screen.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let group = |image: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("screen reflections"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(image),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            })
        };
        SsrTargets {
            trace_group: group(&history),
            copy_group: group(lit),
            history,
            ao: level(0),
            output: level(1),
            unmarked: AtomicBool::new(!ao),
            screen: screen.create_view(&Default::default()),
        }
    }

    /// The trace, after the depth prepass.
    pub fn trace(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &SsrTargets,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        if targets.unmarked.swap(false, Ordering::Relaxed) {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d screen reflections mark"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.ao,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            b: 1.0,
                            ..wgpu::Color::TRANSPARENT
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        self.full_screen(
            encoder,
            "3d screen reflections",
            &self.trace_pipeline,
            &targets.trace_group,
            &targets.output,
            timestamp_writes,
        );
    }

    /// Keeps the opaque image for the next frame, after the opaque pass.
    pub fn keep(&self, encoder: &mut wgpu::CommandEncoder, targets: &SsrTargets) {
        self.full_screen(
            encoder,
            "3d screen reflections history",
            &self.copy_pipeline,
            &targets.copy_group,
            &targets.history,
            None,
        );
    }

    fn full_screen(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        pipeline: &wgpu::RenderPipeline,
        group: &wgpu::BindGroup,
        target: &wgpu::TextureView,
        timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            timestamp_writes,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}
