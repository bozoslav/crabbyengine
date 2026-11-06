//! Full-resolution, fragment-only GTAO and contact shadows, and bilateral
//! denoising: occlusion in red, the contact shadow in green. With
//! screen-space reflections, the denoised result goes into the first level
//! of their image instead (see `ssr.rs`).
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Settings {
    inverse: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    camera: [f32; 4],
    params: [f32; 4],
    sun: [f32; 4],
}
pub(super) struct Ao {
    layout: wgpu::BindGroupLayout,
    raw_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    /// Denoising into the screen-space reflections' image.
    blur_wide_pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
}
pub(super) struct AoTargets {
    raw: wgpu::TextureView,
    pub output: wgpu::TextureView,
    raw_group: wgpu::BindGroup,
    blur_group: wgpu::BindGroup,
    /// Whether `output` is the screen-space reflections' image.
    wide: bool,
}
impl Ao {
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
            label: Some("GTAO"),
            entries: &[
                texture(0, wgpu::TextureSampleType::Depth, true),
                texture(1, wgpu::TextureSampleType::Float { filterable: false }, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
        let shader = device.create_shader_module(wgpu::include_wgsl!("ao.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GTAO"),
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
        let raw_pipeline = pipeline("fs_ao", wgpu::TextureFormat::Rg8Unorm);
        let blur_pipeline = pipeline("fs_denoise", wgpu::TextureFormat::Rg8Unorm);
        let blur_wide_pipeline = pipeline("fs_denoise", super::ssr::SCREEN_FORMAT);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GTAO"),
            contents: bytemuck::bytes_of(&Settings::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            layout,
            raw_pipeline,
            blur_pipeline,
            blur_wide_pipeline,
            uniform,
        }
    }
    /// `ao`: `None` for contact shadows alone. `sun`: toward the sun, and
    /// the contact shadows' length (0: none).
    pub fn prepare(
        &self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        camera: Vec3,
        ao: Option<&crate::AmbientOcclusion>,
        sun: (Vec3, f32),
    ) {
        let off = crate::AmbientOcclusion {
            intensity: 0.0,
            ..Default::default()
        };
        let ao = ao.unwrap_or(&off);
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Settings {
                inverse: view_proj.inverse().to_cols_array_2d(),
                projection: view_proj.to_cols_array_2d(),
                camera: camera.extend(1.0).to_array(),
                params: [
                    ao.radius.max(0.001),
                    ao.intensity.max(0.0),
                    ao.slices.clamp(1, 8) as f32,
                    ao.steps.clamp(1, 16) as f32,
                ],
                sun: sun.0.normalize_or_zero().extend(sun.1).to_array(),
            }),
        );
    }
    pub fn targets(
        &self,
        device: &wgpu::Device,
        size: (u32, u32),
        depth: &wgpu::TextureView,
        dummy_color: &wgpu::TextureView,
        reflections: Option<&wgpu::TextureView>,
    ) -> AoTargets {
        let target = || {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("GTAO"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rg8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let raw = target();
        let output = reflections.cloned().unwrap_or_else(target);
        let group = |image: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GTAO"),
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
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            })
        };
        let raw_group = group(dummy_color);
        let blur_group = group(&raw);
        AoTargets {
            raw,
            output,
            raw_group,
            blur_group,
            wide: reflections.is_some(),
        }
    }
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, targets: &AoTargets) {
        for (label, pipeline, group, target) in [
            ("GTAO horizons", &self.raw_pipeline, &targets.raw_group, &targets.raw),
            (
                "GTAO bilateral",
                if targets.wide {
                    &self.blur_wide_pipeline
                } else {
                    &self.blur_pipeline
                },
                &targets.blur_group,
                &targets.output,
            ),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
use bytemuck::Zeroable;
