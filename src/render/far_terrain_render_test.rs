//! Far terrain drawn offscreen: the coarse ground shows beyond the loaded
//! tiles and is cut away under them, even where it lies higher than they
//! do. Skips without a GPU.
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

    /// A material of `desc`, on the CPU and the GPU.
    fn material(&mut self, desc: MaterialDesc) -> Material {
        self.renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        self.ctx.gfx.create_material(desc)
    }

    /// The GPU half of a mesh the `Ctx` made (meshes are numbered in the
    /// order they're made on both sides).
    fn upload(&mut self, data: &MeshData, material: Material) {
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], false);
    }

    /// A mesh of `data` with `material`, on the CPU and the GPU.
    fn mesh(&mut self, data: &MeshData, material: Material) -> Mesh {
        let mesh = self.ctx.gfx.create_mesh(data, material);
        self.upload(data, material);
        mesh
    }

    /// Builds `far`'s meshes and puts them on the GPU too.
    fn far(&mut self, far: &mut FarTerrain, material: Material) {
        far.update(&mut self.ctx);
        for (b, _, _) in far.shown().collect::<Vec<_>>() {
            self.upload(&far.block_data(b), material);
        }
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
            // Sunlight only: the fog glows with nothing else.
            light: SkyLight {
                sun: Vec3::splat(ctx.lighting.sun_intensity),
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
fn far_terrain_shows_beyond_the_loaded_tiles_and_is_cut_away_under_them() {
    let Some(mut rig) = Rig::new() else { return };
    let near = rig.material(MaterialDesc::color(Color::rgb(0.9, 0.05, 0.05)));
    let far_ground = rig.material(MaterialDesc::color(Color::rgb(0.05, 0.9, 0.05)));
    // One loaded tile, cell (0, 0): 32 m of flat ground at height 0.
    let heights = vec![0.0; 9 * 9];
    let lods = std::array::from_fn(|i| rig.mesh(&MeshData::heightfield(&heights, 32.0, 1 << i), near));
    rig.ctx.world.spawn((
        Transform3D::default(),
        TerrainTile {
            lods,
            lod_distance: 32.0,
        },
    ));
    // Far ground 2 m higher everywhere, every 8 m from (-32, -32) to (64, 64):
    // were it not cut, it would hide the tile.
    let coarse = vec![2.0; 13 * 13];
    let make = || FarTerrain::new(&coarse, 13, vec2(-32.0, -32.0), 8.0, 32.0, far_ground);

    rig.ctx.lighting.shadows = false;
    rig.ctx.lighting.sun_direction = vec3(0.2, -1.0, 0.1);
    rig.ctx.camera3d.position = vec3(16.0, 150.0, 16.01);
    rig.ctx.camera3d.look_at(vec3(16.0, 0.0, 16.0));
    let view_proj = rig.ctx.camera3d.view_proj();
    let color = |pixels: &[u8], p: Vec3| {
        let (x, y) = pixel(view_proj, p);
        let i = ((y * SIZE + x) * 4) as usize;
        (i32::from(pixels[i]), i32::from(pixels[i + 1]))
    };
    let is_near = |(r, g): (i32, i32)| r > g + 40;
    let is_far = |(r, g): (i32, i32)| g > r + 40;
    let tile_points = [vec3(16.0, 0.0, 16.0), vec3(1.5, 0.0, 16.0), vec3(30.5, 0.0, 30.5)];
    let beyond = [
        vec3(-1.5, 2.0, 16.0),
        vec3(-24.0, 2.0, -24.0),
        vec3(56.0, 2.0, 40.0),
        vec3(33.5, 2.0, 2.0),
    ];

    // Without the cut, the higher far ground covers the tile.
    let mut uncut = make();
    rig.far(&mut uncut, far_ground);
    let pixels = rig.draw();
    for p in tile_points {
        assert!(
            is_far(color(&pixels, p)),
            "uncut: far ground over {p}: {:?}",
            color(&pixels, p)
        );
    }
    for (_, entity, _) in uncut.shown() {
        rig.ctx.world.despawn(entity);
    }

    // Cut where the tile is: the tile shows there, the far ground all around.
    let mut far = make();
    far.cut((0, 0));
    rig.far(&mut far, far_ground);
    let pixels = rig.draw();
    for p in tile_points {
        assert!(
            is_near(color(&pixels, p)),
            "the tile shows at {p}: {:?}",
            color(&pixels, p)
        );
    }
    for p in beyond {
        assert!(is_far(color(&pixels, p)), "far ground at {p}: {:?}", color(&pixels, p));
    }
    // No crack where they meet: every pixel along the tile's edge is ground.
    for k in 0..=32 {
        for p in [
            vec3(k as f32, 1.0, -0.3),
            vec3(-0.3, 1.0, k as f32),
            vec3(32.3, 1.0, k as f32),
        ] {
            let (r, g) = color(&pixels, p);
            assert!(r.max(g) > 30, "a crack at {p}: {:?}", (r, g));
        }
    }
}
