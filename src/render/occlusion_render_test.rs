//! Occlusion culling drawn offscreen: a box behind a wall stops being drawn
//! once the wall's depth comes back from the GPU, and is drawn again after
//! the wall moves away. Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
#[allow(unused_imports)]
use crate::d3::Transform as Transform3D;
use crate::prelude::*;
#[allow(unused_imports)]
use crate::render::{GpuFeature, MaterialDesc, Mesh, MeshData, Model};
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
    /// Kept between frames, like the engine's: it holds the depth grid.
    frame: Frame3D,
}

impl Rig {
    /// `None` without a GPU.
    fn new(solid_prepass: bool) -> Option<Self> {
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
        let renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, solid_prepass);
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
            frame: Frame3D::default(),
        })
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
        let frame = &mut self.frame;
        if let Some(grid) = self.renderer.take_occlusion() {
            frame.occlusion = Some(grid);
        }
        frame.collect(
            &mut ctx.world,
            &ctx.gfx.assets3d,
            &SkinFrame::default(),
            &ctx.camera3d,
            &ctx.lighting,
        );
        let scene = Scene3D {
            frame,
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
            occlusion: ctx.camera3d.occlusion_culling,
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
        self.renderer.submitted();
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

/// A material that shows as a flat `color`, whatever the light.
fn glowing(color: Color) -> MaterialDesc {
    MaterialDesc {
        color: Color::rgb(0.0, 0.0, 0.0),
        emissive: color,
        roughness: 1.0,
        ..Default::default()
    }
}

/// Red pixels: the box behind the wall.
fn red(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 120 && p[1] < 60 && p[2] < 60)
        .count()
}

/// A red box at the origin and a blue wall between it and the camera.
fn wall_scene(rig: &mut Rig) -> Entity {
    let red_box = rig.mesh(&MeshData::cube(1.0), glowing(Color::rgb(1.0, 0.0, 0.0)));
    let wall = rig.mesh(&MeshData::cube(1.0), glowing(Color::rgb(0.0, 0.0, 1.0)));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(red_box)));
    let wall = rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, 0.0, 4.0),
            scale: vec3(6.0, 6.0, 0.5),
            ..Default::default()
        },
        Model::new(wall),
    ));
    rig.ctx.camera3d.position = vec3(0.0, 0.0, 10.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    wall
}

#[test]
fn a_box_behind_a_wall_is_culled_and_comes_back_when_the_wall_moves() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        if !crate::render::render_test_support::needs(&rig.device, crate::render::GpuFeature::OcclusionCulling) {
            return;
        }
        let wall = wall_scene(&mut rig);
        rig.ctx.camera3d.occlusion_culling = true;
        // The first frame has no depth to go by: both are drawn.
        rig.draw();
        assert_eq!((rig.frame.visible, rig.frame.occluded), (2, 0));
        // The depth comes back a frame or two later (here at once: each
        // draw waits for the GPU). Then the box is culled.
        let mut culled = false;
        for _ in 0..4 {
            rig.draw();
            if rig.frame.occluded == 1 {
                culled = true;
                break;
            }
        }
        assert!(culled, "solid prepass {solid_prepass}: the box was never culled");
        assert_eq!(rig.frame.visible, 1, "only the wall is drawn");
        // Move the wall aside: within a few frames, the box is back.
        rig.ctx.world.get_mut::<Transform3D>(wall).unwrap().position.x = 20.0;
        let mut back = false;
        for _ in 0..4 {
            if red(&rig.draw()) > 50 {
                back = true;
                break;
            }
        }
        assert!(back, "solid prepass {solid_prepass}: the box never came back");
        // Turning culling off draws everything at once.
        rig.ctx.world.get_mut::<Transform3D>(wall).unwrap().position.x = 0.0;
        for _ in 0..3 {
            rig.draw();
        }
        assert_eq!(rig.frame.occluded, 1);
        rig.ctx.camera3d.occlusion_culling = false;
        rig.draw();
        assert_eq!((rig.frame.visible, rig.frame.occluded), (2, 0));
    }
}
