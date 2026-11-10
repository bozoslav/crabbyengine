//! Alpha to coverage drawn offscreen: a cut-out material's edge gets partial
//! coverage (smooth) instead of a hard cut, and the prepass and the opaque
//! pass agree on it (no holes, no halo). Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
#[allow(unused_imports)]
use crate::anim::Blend;
#[allow(unused_imports)]
use crate::d3::Transform as Transform3D;
use crate::prelude::*;
#[allow(unused_imports)]
use crate::render::{MaterialDesc, Mesh, MeshData, Model, Shader};
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};

const SIZE: u32 = 128;

/// An offscreen renderer with a 1x1 white texture, and a `Ctx` to fill.
struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    output: wgpu::Texture,
    target: wgpu::TextureView,
    textures: Vec<GpuTexture>,
    renderer: Renderer3D,
    ctx: Ctx,
}

impl Rig {
    /// `None` without a GPU.
    fn new(samples: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            crate::render::render_test_support::no_gpu("no GPU adapter");
            return None;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &crate::render::render_test_support::forced(&wgpu::DeviceDescriptor::default()),
        )) else {
            crate::render::render_test_support::no_gpu("no GPU device");
            return None;
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let white = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });
        queue.write_texture(
            white.as_image_copy(),
            &[255; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            white.size(),
        );
        let sampler = device.create_sampler(&Default::default());
        let view = white.create_view(&Default::default());
        let textures = vec![GpuTexture {
            texture: white.clone(),
            nearest: false,
            mips: None,
            bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            }),
        }];
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = output.create_view(&Default::default());
        let renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, samples, true);
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(SIZE as f32, SIZE as f32));
        ctx.sky.atmosphere = false;
        Some(Self {
            device,
            queue,
            output,
            target,
            textures,
            renderer,
            ctx,
        })
    }

    /// A shader made from `source`, on the CPU and the GPU.
    fn shader(&mut self, source: &str) -> Shader {
        let shader = self.ctx.gfx.create_shader("test", source);
        self.renderer.create_shader(&self.device, "test", source).unwrap();
        shader
    }

    /// A mesh of `data` with `desc`, on the CPU and the GPU.
    fn mesh(&mut self, data: &MeshData, desc: MaterialDesc) -> Mesh {
        let material = self.ctx.gfx.create_material(desc);
        let mesh = self.ctx.gfx.create_mesh(data, material);
        self.renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], true);
        mesh
    }

    /// Draws the world and reads the pixels back (RGBA).
    fn draw(&mut self) -> Vec<u8> {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        frame.collect(
            &mut ctx.world,
            &ctx.gfx.assets3d,
            &SkinFrame::default(),
            &ctx.camera3d,
            &ctx.lighting,
        );
        let scene = Scene3D {
            frame: &frame,
            lighting: &ctx.lighting,
            sky: &ctx.sky,
            // Sunlight only in the shadow scene: elsewhere the cubes glow
            // on black.
            light: SkyLight {
                sun: if ctx.lighting.shadows {
                    Vec3::splat(ctx.lighting.sun_intensity)
                } else {
                    Vec3::ZERO
                },
                sky: Vec3::ZERO,
                ground: Vec3::ZERO,
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
            occlusion: false,
        };
        self.renderer.prepare(
            &self.device,
            &self.queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (SIZE, SIZE),
            },
        );
        let buffers = self
            .renderer
            .draw(&self.device, &self.target, wgpu::Color::BLACK, &self.textures, None);
        self.queue.submit(buffers);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(SIZE * SIZE * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: None,
                },
            },
            self.output.size(),
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        readback.get_mapped_range(..).unwrap().to_vec()
    }
}

/// A big white cut-out plane under the camera, its alpha running from -0.5
/// to 1.5 along a diagonal: one soft edge across the screen, and no edge of
/// the plane itself in view.
fn diagonal_cutout(rig: &mut Rig) {
    let shader =
        rig.shader("fn surface(s: Surface) -> Surface { var o = s; o.alpha = s.uv.x + s.uv.y - 0.5; return o; }");
    let desc = MaterialDesc {
        color: Color::rgb(0.0, 0.0, 0.0),
        emissive: Color::WHITE,
        roughness: 1.0,
        blend: Blend::Cutout,
        shader: Some(shader),
        double_sided: true,
        ..Default::default()
    };
    let plane = rig.mesh(&MeshData::plane(1.0), desc);
    rig.ctx.world.spawn((
        Transform3D {
            scale: Vec3::splat(40.0),
            ..Default::default()
        },
        Model::new(plane),
    ));
    rig.ctx.camera3d.position = vec3(0.0, 6.0, 0.001);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
}

/// (background, inside, between) pixel counts, by brightness against the
/// brightest pixel.
fn coverage_counts(pixels: &[u8]) -> (usize, usize, usize) {
    let brightness: Vec<u32> = pixels.as_chunks::<4>().0.iter().map(|p| u32::from(p[0])).collect();
    let full = *brightness.iter().max().unwrap();
    assert!(full > 100, "the plane shows: {full}");
    let (mut bg, mut inside, mut between) = (0, 0, 0);
    for &b in &brightness {
        if b < 12 {
            bg += 1;
        } else if b + 12 > full {
            inside += 1;
        } else {
            between += 1;
        }
    }
    (bg, inside, between)
}

#[test]
fn multisampled_cutouts_get_soft_edges_without_holes() {
    let Some(mut rig) = Rig::new(4) else { return };
    diagonal_cutout(&mut rig);
    let (bg, inside, between) = coverage_counts(&rig.draw());
    // About half the screen on each side of the diagonal.
    let half = (SIZE * SIZE / 2) as usize;
    assert!(bg > half * 8 / 10 && inside > half * 8 / 10, "{bg} {inside} {between}");
    // The edge crosses the screen once: about one pixel per row (the
    // diagonal's length in pixels, a little more or less).
    assert!(between >= SIZE as usize, "a soft edge: {bg} {inside} {between}");
    assert!(between < 4 * SIZE as usize, "only at the edge: {bg} {inside} {between}");
}
