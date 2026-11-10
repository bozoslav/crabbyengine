//! Global illumination: light bounced off a sunlit red wall tints the
//! floor beside it in a closed room. Skips without a GPU that has it.
use super::gpu::GpuTexture;
use super::gpu3d::*;
#[allow(unused_imports)]
use crate::d3::Transform as Transform3D;
use crate::prelude::*;
#[allow(unused_imports)]
use crate::render::{GlobalIllumination, MaterialDesc, Mesh, MeshData, Model};
use crate::three::{GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};

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
    size: (u32, u32),
}

impl Rig {
    /// `None` without a GPU that can draw global illumination. `slots`
    /// false: a device at the default limits (16 texture slots, as WebGL2),
    /// which can't.
    fn new(slots: bool) -> Option<Self> {
        Self::sized(slots, (SIZE, SIZE))
    }

    /// [`new`](Self::new), drawing `size` pixels, with GPU timestamps
    /// where the GPU has them.
    fn sized(slots: bool, size: (u32, u32)) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            crate::render::render_test_support::no_gpu("no GPU adapter");
            return None;
        };
        // As the engine asks: more texture slots where the adapter has them.
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &crate::render::render_test_support::forced(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits {
                    max_sampled_textures_per_shader_stage: if slots {
                        adapter.limits().max_sampled_textures_per_shader_stage.clamp(16, 24)
                    } else {
                        16
                    },
                    ..wgpu::Limits::default()
                },
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            }),
        )) else {
            crate::render::render_test_support::no_gpu("no GPU device");
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
                width: size.0,
                height: size.1,
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
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(size.0 as f32, size.1 as f32));
        ctx.sky.atmosphere = false;
        ctx.gfx.assets3d.decals_supported = supports_decals(&device);
        ctx.gfx.assets3d.gi_supported = true;
        Some(Self {
            device,
            queue,
            output,
            target,
            textures,
            renderer,
            ctx,
            size,
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

    /// Draws `frames` frames (the probes update a few each) and reads the
    /// last one back (RGBA).
    fn draw(&mut self, frames: usize) -> Vec<u8> {
        for _ in 0..frames {
            let _ = self.frame(None);
        }
        self.read_back()
    }

    /// Collects and draws one frame, timed by `timer`; returns how long the
    /// GPU took from the submit to finishing it, in milliseconds.
    fn frame(&mut self, mut timer: Option<&mut super::timer::GpuTimer>) -> f64 {
        if let Some(t) = timer.as_deref_mut() {
            t.collect(&self.device);
            t.begin_frame();
        }
        {
            let ctx = &mut self.ctx;
            ctx.gfx.frame3d.lod = ctx.lod;
            ctx.gfx.frame3d.collect(
                &mut ctx.world,
                &ctx.gfx.assets3d,
                &SkinFrame::default(),
                &ctx.camera3d,
                &ctx.lighting,
            );
            let rgb = |c| {
                let [r, g, b, _] = crate::three::linear(c);
                Vec3::new(r, g, b)
            };
            let scene = Scene3D {
                frame: &ctx.gfx.frame3d,
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
                    size: self.size,
                },
            );
            let mut buffers = self.renderer.draw(
                &self.device,
                &self.target,
                wgpu::Color::BLACK,
                &self.textures,
                timer.as_deref(),
            );
            if let Some(t) = timer.as_deref_mut() {
                let mut encoder = self.device.create_command_encoder(&Default::default());
                t.resolve(&mut encoder);
                buffers.push(encoder.finish());
            }
            let start = std::time::Instant::now();
            self.queue.submit(buffers);
            if let Some(t) = timer {
                t.submitted();
            }
            self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            start.elapsed().as_secs_f64() * 1000.0
        }
    }

    /// The last frame drawn (RGBA).
    fn read_back(&self) -> Vec<u8> {
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(self.size.0 * self.size.1 * 4),
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
                    bytes_per_row: Some(self.size.0 * 4),
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
            probes_per_frame: crate::render::GlobalIllumination::MAX_PROBES_PER_FRAME,
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
    on.ctx.gfx.assets3d.gi_supported = true; // even if the engine asked for it
    assert_eq!(off.draw(1), on.draw(3));
}

/// GPU cost of global illumination in a town-like scene at 1280x720: off,
/// at the defaults, and at the most probes a frame. Run in release
/// (`cargo test --release --lib --features 3d -- --ignored --nocapture
/// gi_cost`); prints the GPU frame time and the GI passes' times from the
/// engine's timestamp profiler, and the CPU time to collect a frame.
#[test]
#[ignore = "benchmark: run in release"]
fn gi_cost() {
    let Some(mut rig) = Rig::sized(true, (1280, 720)) else {
        return;
    };
    let Some(mut timer) = super::timer::GpuTimer::new(&rig.device, &rig.queue) else {
        eprintln!("no GPU timestamps: skipped");
        return;
    };
    let white = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::WHITE));
    let tinted = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::rgb(0.8, 0.5, 0.3)));
    rig.block(white, vec3(-60.0, -1.0, -60.0), vec3(60.0, 0.0, 60.0));
    // 30 x 30 buildings, 4 apart, 3 to 12 high.
    let mut seed = 7u32;
    let mut random = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1 << 24) as f32
    };
    for i in 0..30 {
        for j in 0..30 {
            let at = vec3(i as f32 * 4.0 - 58.0, 0.0, j as f32 * 4.0 - 58.0);
            let mesh = if (i + j) % 3 == 0 { tinted } else { white };
            let height = 3.0 + random() * 9.0;
            rig.block(mesh, at, at + vec3(2.5, height, 2.5));
        }
    }
    rig.ctx.lighting.sun_direction = vec3(-0.4, -1.0, -0.3);
    rig.ctx.camera3d.position = vec3(1.0, 1.7, 1.0);
    rig.ctx.camera3d.look_at(vec3(20.0, 3.0, 12.0));
    rig.ctx.camera3d.far = 200.0;
    // Rounds of every setting, each setting's best round kept: this runs
    // on machines doing other things too.
    let settings = [
        ("off", None),
        (
            "defaults (4 probes a frame)",
            Some(GlobalIllumination::default().probes_per_frame),
        ),
        (
            "16 probes a frame",
            Some(crate::render::GlobalIllumination::MAX_PROBES_PER_FRAME),
        ),
    ];
    let mut best = vec![(f32::MAX, f64::MAX, f64::MAX, String::new()); settings.len()];
    for _ in 0..5 {
        for (k, &(_, gi)) in settings.iter().enumerate() {
            rig.ctx.lighting.gi = GlobalIllumination {
                enabled: gi.is_some(),
                probes_per_frame: gi.unwrap_or(1),
                ..Default::default()
            };
            for _ in 0..20 {
                let _ = rig.frame(Some(&mut timer));
            }
            let (mut frames, mut total) = (0, 0.0);
            let mut parts = std::collections::BTreeMap::<&str, f32>::new();
            let mut read = timer.frames_read;
            let mut waits = Vec::with_capacity(60);
            for _ in 0..60 {
                waits.push(rig.frame(Some(&mut timer)));
                if timer.frames_read != read {
                    read = timer.frames_read;
                    frames += 1;
                    let spans = &timer.last_frame;
                    let end = spans.iter().map(|p| p.start_ms + p.ms).fold(0.0, f32::max);
                    total += end - spans.iter().map(|p| p.start_ms).fold(f32::MAX, f32::min);
                    for p in spans {
                        *parts.entry(p.name).or_default() += p.ms;
                    }
                }
            }
            let mut frame = crate::three::Frame3D::default();
            let cpu_start = std::time::Instant::now();
            for _ in 0..30 {
                frame.collect(
                    &mut rig.ctx.world,
                    &rig.ctx.gfx.assets3d,
                    &SkinFrame::default(),
                    &rig.ctx.camera3d,
                    &rig.ctx.lighting,
                );
            }
            let cpu = cpu_start.elapsed().as_secs_f64() * 1000.0 / 30.0;
            waits.sort_by(f64::total_cmp);
            let n = frames.max(1) as f32;
            let gpu = total / n;
            let b = &mut best[k];
            if gpu < b.0 {
                let shown: Vec<String> = parts
                    .iter()
                    .filter(|(name, _)| name.starts_with("gi") || **name == "opaque")
                    .map(|(name, ms)| format!("{name} {:.3}", ms / n))
                    .collect();
                b.0 = gpu;
                b.3 = shown.join(", ");
            }
            b.1 = b.1.min(waits[waits.len() / 2]);
            b.2 = b.2.min(cpu);
        }
    }
    for ((label, _), (gpu, wait, cpu, parts)) in settings.iter().zip(&best) {
        println!("gi {label}: GPU frame {gpu:.3} ms ({parts}); submit to done {wait:.3} ms; collect {cpu:.3} ms");
    }
}
