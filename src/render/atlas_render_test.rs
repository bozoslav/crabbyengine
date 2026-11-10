//! The light shadow atlas: a point light and a spot light with tiles of
//! very different sizes in it each shade the ground under their own box,
//! and only that. Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

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
    fn new() -> Option<Self> {
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
        let renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, true);
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

    /// Draws the world and reads the pixels back (RGBA), with the light
    /// shadow views' tiles in the atlas.
    fn draw(&mut self) -> (Vec<u8>, Vec<[u32; 3]>) {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        // As the engine does: shadow maps no larger than the device allows.
        frame.size_limit = self.device.limits().max_texture_dimension_2d;
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
            // No sun or sky: only the point and spot lights.
            light: SkyLight {
                sun: Vec3::ZERO,
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
        let pixels = readback.get_mapped_range(..).unwrap().to_vec();
        (pixels, frame.lights.shadow_tiles.clone())
    }
}

/// The world point under screen pixel `p`'s projection.
fn pixel(view_proj: Mat4, p: Vec3) -> (u32, u32) {
    let c = view_proj * p.extend(1.0);
    let ndc = c.truncate() / c.w;
    (
        ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32,
        ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32,
    )
}

#[test]
fn lights_with_small_and_large_atlas_tiles_both_cast_their_shadows() {
    let Some(mut rig) = Rig::new() else { return };
    let matte = MaterialDesc {
        color: Color::WHITE,
        roughness: 1.0,
        ..Default::default()
    };
    let ground = rig.mesh(&MeshData::plane(30.0), matte);
    let cube = rig.mesh(&MeshData::cube(1.0), matte);
    let ctx = &mut rig.ctx;
    ctx.world.spawn((Transform3D::default(), Model::new(ground)));
    // A box 2 up under each light, the spot's on the left, the point's on
    // the right, too far apart for either light to reach the other's.
    let (spot_at, point_at) = (vec3(-4.0, 0.0, 0.0), vec3(4.0, 0.0, 0.0));
    for at in [spot_at, point_at] {
        ctx.world.spawn((Transform3D::at(at + Vec3::Y * 2.0), Model::new(cube)));
    }
    let spot = ctx.world.spawn((
        Transform3D::at(spot_at + Vec3::Y * 4.0).looking_at(spot_at + vec3(0.0, 0.0, 0.001)),
        SpotLight3D {
            intensity: 300.0,
            range: 10.0,
            inner_angle: 0.5,
            outer_angle: 0.7,
            shadows: true,
            ..Default::default()
        },
    ));
    let point = ctx.world.spawn((
        Transform3D::at(point_at + Vec3::Y * 4.0),
        PointLight3D {
            intensity: 300.0,
            range: 6.0,
            shadows: true,
            ..Default::default()
        },
    ));
    ctx.camera3d.position = vec3(0.0, 9.0, 9.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.lighting.shadows = true;
    ctx.lighting.sun_intensity = 0.0;
    ctx.lighting.ambient_intensity = 0.0;
    let view_proj = ctx.camera3d.view_proj();
    let brightness = |pixels: &[u8], p: Vec3| -> u32 {
        let (x, y) = pixel(view_proj, p);
        let i = ((y * SIZE + x) * 4) as usize;
        u32::from(pixels[i]) + u32::from(pixels[i + 1]) + u32::from(pixels[i + 2])
    };
    // Each box shades the ground within about 1 of the middle under it;
    // 2 out is still lit, in the open.
    let under = |at: Vec3| at + vec3(0.0, 0.0, 0.5);
    let open = |at: Vec3| at + vec3(0.0, 0.0, 2.0);

    // (spot, point) hints: a large spot tile beside six of the smallest
    // point tiles, then the other way around (a point light asking for
    // more than fits shrinks to fit).
    // The point light's faces shrink to a quarter of the atlas, which is
    // 4096² or the largest texture the device allows.
    let quarter = 4096.min(rig.device.limits().max_texture_dimension_2d) / 4;
    for (spot_hint, point_hint, sides) in [(1024, 64, [64, 1024]), (64, 2048, [quarter.min(1024), 64])] {
        rig.ctx.world.get_mut::<SpotLight3D>(spot).unwrap().shadow_resolution = spot_hint;
        rig.ctx.world.get_mut::<PointLight3D>(point).unwrap().shadow_resolution = point_hint;
        for light in [spot, point] {
            set_shadows(&mut rig.ctx, light, true);
        }
        let (shadowed, tiles) = rig.draw();
        // The point light's six faces come first, then the spot's view.
        assert_eq!(tiles.len(), 7);
        assert!(tiles[..6].iter().all(|t| t[2] == sides[0]), "{tiles:?}");
        assert_eq!(tiles[6][2], sides[1], "{tiles:?}");
        for (i, a) in tiles.iter().enumerate() {
            for b in &tiles[i + 1..] {
                let apart = a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[2] <= b[1] || b[1] + b[2] <= a[1];
                assert!(apart, "tiles {a:?} and {b:?} overlap");
            }
        }
        for light in [spot, point] {
            set_shadows(&mut rig.ctx, light, false);
        }
        let (unshadowed, _) = rig.draw();
        for (name, at) in [("spot", spot_at), ("point", point_at)] {
            let hints = format!("{name} light, hints {spot_hint}/{point_hint}");
            assert!(
                brightness(&unshadowed, open(at)) > 60,
                "{hints}: lights the open ground"
            );
            assert!(
                brightness(&unshadowed, under(at)) > 60,
                "{hints}: passes the box unshadowed"
            );
            assert!(
                brightness(&shadowed, under(at)) * 4 < brightness(&unshadowed, under(at)),
                "{hints}: the box shadows the ground under it: {} vs {}",
                brightness(&shadowed, under(at)),
                brightness(&unshadowed, under(at))
            );
            assert!(
                brightness(&shadowed, open(at)) * 10 > brightness(&unshadowed, open(at)) * 8,
                "{hints}: open ground stays lit with shadows on: {} vs {}",
                brightness(&shadowed, open(at)),
                brightness(&unshadowed, open(at))
            );
        }
    }
}

fn set_shadows(ctx: &mut Ctx, light: Entity, on: bool) {
    if let Some(s) = ctx.world.get_mut::<SpotLight3D>(light) {
        s.shadows = on;
    }
    if let Some(p) = ctx.world.get_mut::<PointLight3D>(light) {
        p.shadows = on;
    }
}
