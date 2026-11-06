//! Global illumination: light bounced off a sunlit red wall tints the
//! floor beside it in a closed room. Skips without a GPU that has it.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
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
    /// `None` without a GPU that can draw global illumination. `slots`
    /// false: a device at the default limits (16 texture slots, as WebGL2),
    /// which can't.
    fn new(slots: bool) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return None;
        };
        // As the engine asks: more texture slots where the adapter has them.
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_sampled_textures_per_shader_stage: if slots {
                    adapter.limits().max_sampled_textures_per_shader_stage.clamp(16, 24)
                } else {
                    16
                },
                ..wgpu::Limits::default()
            },
            ..Default::default()
        })) else {
            eprintln!("no GPU device: skipped");
            return None;
        };
        if supports_gi(&device) != slots {
            eprintln!("this GPU can't be set up for this test: skipped");
            return None;
        }
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
        ctx.assets3d.decals_supported = supports_decals(&device);
        ctx.assets3d.gi_supported = true;
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
        let material = self.ctx.create_material(desc);
        let mesh = self.ctx.create_mesh(data, material);
        self.renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], true);
        mesh
    }

    /// Draws `frames` frames (the probes update a few each) and reads the
    /// last one back (RGBA).
    fn draw(&mut self, frames: usize) -> Vec<u8> {
        for _ in 0..frames {
            let ctx = &mut self.ctx;
            ctx.frame3d.lod = ctx.lod;
            ctx.frame3d.collect(
                &mut ctx.world,
                &ctx.assets3d,
                &SkinFrame::default(),
                &ctx.camera3d,
                &ctx.lighting,
            );
            let rgb = |c| {
                let [r, g, b, _] = crate::three::linear(c);
                Vec3::new(r, g, b)
            };
            let scene = Scene3D {
                frame: &ctx.frame3d,
                lighting: &ctx.lighting,
                sky: &ctx.sky,
                light: SkyLight {
                    sun: Vec3::splat(ctx.lighting.sun_intensity),
                    sky: rgb(ctx.lighting.sky_color),
                    ground: rgb(ctx.lighting.ground_color),
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
        }
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

    /// A box from `lo` to `hi`.
    fn block(&mut self, mesh: Mesh, lo: Vec3, hi: Vec3) {
        self.ctx.world.spawn((
            Transform3D {
                position: (lo + hi) * 0.5,
                scale: hi - lo,
                ..Default::default()
            },
            Model::new(mesh),
        ));
    }

    /// A closed white room (the inside from -3.5 to 3.5 across, -0.5 to
    /// 3.5 high) whose west wall is red, with the sun shining in through
    /// an opening in the roof onto that wall (not the floor), and the
    /// camera inside, looking west.
    fn red_wall_room(gi: bool, slots: bool) -> Option<Self> {
        let mut rig = Self::new(slots)?;
        let white = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::WHITE));
        let red = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::rgb(0.9, 0.04, 0.04)));
        let (inner, outer) = (3.5, 3.9);
        rig.block(white, vec3(-outer, -0.9, -outer), vec3(outer, -0.5, outer)); // floor
        rig.block(red, vec3(-outer, -0.5, -outer), vec3(-inner, 3.5, outer)); // west
        rig.block(white, vec3(inner, -0.5, -outer), vec3(outer, 3.5, outer)); // east
        rig.block(white, vec3(-inner, -0.5, -outer), vec3(inner, 3.5, -inner)); // north
        rig.block(white, vec3(-inner, -0.5, inner), vec3(inner, 3.5, outer)); // south
        // The roof, with an opening from x -2 to 0 and z -1.5 to 1.5.
        rig.block(white, vec3(-outer, 3.5, -outer), vec3(-2.0, 3.9, outer));
        rig.block(white, vec3(0.0, 3.5, -outer), vec3(outer, 3.9, outer));
        rig.block(white, vec3(-2.0, 3.5, -outer), vec3(0.0, 3.9, -1.5));
        rig.block(white, vec3(-2.0, 3.5, 1.5), vec3(0.0, 3.9, outer));
        let lighting = &mut rig.ctx.lighting;
        // From the upper east: through the opening onto the west wall,
        // between 0 and 2 high.
        lighting.sun_direction = vec3(-1.0, -1.0, 0.0);
        lighting.sun_color = Color::WHITE;
        lighting.sun_intensity = 4.0;
        lighting.sky_color = Color::rgb(0.5, 0.5, 0.5);
        lighting.ground_color = Color::rgb(0.5, 0.5, 0.5);
        lighting.ambient_intensity = 0.3;
        lighting.shadow_distance = 30.0;
        lighting.gi = GlobalIllumination {
            enabled: gi,
            extent: vec3(12.0, 4.0, 8.0),
            spacing: 1.0,
            probes_per_frame: MAX_GI_PROBES_PER_FRAME,
            intensity: 1.0,
        };
        rig.ctx.camera3d.position = vec3(2.0, 1.3, 0.3);
        rig.ctx.camera3d.look_at(vec3(-3.5, -0.2, 0.0));
        Some(rig)
    }

    /// The color of the world point `p` in `pixels`.
    fn at(&self, pixels: &[u8], p: Vec3) -> [i32; 3] {
        let c = self.ctx.camera3d.view_proj() * p.extend(1.0);
        let ndc = c.truncate() / c.w;
        let x = ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32;
        let y = ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32;
        let i = ((y * SIZE + x) * 4) as usize;
        [0, 1, 2].map(|k| i32::from(pixels[i + k]))
    }
}

#[test]
fn a_sunlit_red_wall_tints_the_floor_beside_it_red() {
    let Some(mut off) = Rig::red_wall_room(false, true) else {
        return;
    };
    let Some(mut on) = Rig::red_wall_room(true, true) else {
        return;
    };
    // Enough frames for every probe to look around twice (two bounces).
    let (dark, lit) = (off.draw(1), on.draw(80));
    let near_red = vec3(-3.0, -0.5, -0.8);
    let far_side = vec3(-0.5, -0.5, -0.8);
    let sunlit_wall = vec3(-3.5, 1.0, 0.0);
    let (a, b) = (off.at(&dark, near_red), on.at(&lit, near_red));
    let far = on.at(&lit, far_side);
    eprintln!(
        "floor by the red wall: {a:?} off, {b:?} on; further away {:?} off, {far:?} on; sunlit wall {:?}",
        off.at(&dark, far_side),
        on.at(&lit, sunlit_wall)
    );
    // The sun lights the wall, not the floor.
    assert!(on.at(&lit, sunlit_wall)[0] > 150, "the wall is in the sun");
    // Without bounced light, the floor takes only the grey ambient light.
    assert!((a[0] - a[1]).abs() <= 4, "no red without bounced light: {a:?}");
    // With it, red from the wall, most beside it.
    assert!(b[0] > b[1] + 20, "red bounce light on the floor: {b:?}");
    assert!(
        b[0] - b[1] > far[0] - far[1],
        "redder by the red wall: {b:?} vs {far:?}"
    );
}

#[test]
fn without_the_texture_slot_bounced_light_is_skipped() {
    // As on WebGL2: the frame is the same as with it off.
    let Some(mut off) = Rig::red_wall_room(false, false) else {
        return;
    };
    let Some(mut on) = Rig::red_wall_room(true, false) else {
        return;
    };
    on.ctx.assets3d.gi_supported = true; // even if the engine asked for it
    assert_eq!(off.draw(1), on.draw(3));
}
