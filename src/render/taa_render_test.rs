//! Headless GPU tests of temporal antialiasing: converging edges, motion
//! vectors, ghosting, camera cuts and the fallback on devices without
//! compute. They skip without a GPU adapter.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

/// A renderer drawing a light box (or two) on black, read back as pixels.
struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer3D,
    textures: Vec<GpuTexture>,
    output: wgpu::Texture,
    ctx: Ctx,
    frame: Frame3D,
    cube: Mesh,
    size: u32,
}

impl Rig {
    fn new(size: u32, downlevel: bool) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return None;
        };
        let mut descriptor = wgpu::DeviceDescriptor::default();
        if downlevel {
            descriptor.required_limits.max_compute_workgroups_per_dimension = 0;
            descriptor.required_limits.max_storage_textures_per_shader_stage = 0;
        }
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&descriptor)) else {
            eprintln!("no GPU device: skipped");
            return None;
        };
        let texture_entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                texture_entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                texture_entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
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
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        // A box that shines white and lights nothing, on black.
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(size as f32, size as f32));
        let glow = MaterialDesc {
            color: Color::BLACK,
            roughness: 1.0,
            emissive: Color::WHITE,
            ..Default::default()
        };
        let material = ctx.create_material(glow);
        let cube_data = MeshData::cube(1.0);
        let cube = ctx.create_mesh(&cube_data, material);
        let mut renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, true);
        renderer.create_material(&device, &GpuMaterialDesc::new(&glow), &textures);
        renderer.create_mesh(
            &device,
            &cube_data,
            &[(0..cube_data.indices.len() as u32, material)],
            true,
        );
        ctx.camera3d.position = vec3(0.0, 0.0, 5.0);
        ctx.camera3d.look_at(Vec3::ZERO);
        ctx.lighting.shadows = false;
        ctx.lighting.sun_intensity = 0.0;
        ctx.lighting.ambient_intensity = 0.0;
        ctx.sky.atmosphere = false;
        ctx.post.bloom = 0.0;
        ctx.post.tonemap = Tonemap::None;
        Some(Self {
            device,
            queue,
            renderer,
            textures,
            output,
            ctx,
            frame: Frame3D::default(),
            cube,
            size,
        })
    }

    /// A slab turned `angle` about the view axis: a long diagonal edge.
    fn slab(&mut self, angle: f32) {
        self.ctx.world.clear();
        self.ctx.world.spawn((
            Transform3D {
                rotation: Quat::from_rotation_z(angle),
                scale: vec3(4.0, 1.5, 0.05),
                ..Default::default()
            },
            Model::new(self.cube),
        ));
    }

    /// Draws a frame and returns its red channel, linear, row by row.
    fn draw(&mut self) -> Vec<f32> {
        let size = self.size;
        let ctx = &mut self.ctx;
        self.frame.motion.enabled = ctx.post.anti_aliasing == AntiAliasing::Taa;
        self.frame.collect(
            &mut ctx.world,
            &ctx.assets3d,
            &SkinFrame::default(),
            &ctx.camera3d,
            &ctx.lighting,
        );
        let scene = Scene3D {
            frame: &self.frame,
            lighting: &ctx.lighting,
            sky: &ctx.sky,
            light: SkyLight {
                sun: Vec3::ZERO,
                sky: Vec3::ZERO,
                ground: Vec3::ZERO,
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
        };
        self.renderer.prepare(
            &self.device,
            &self.queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (size, size),
            },
        );
        self.frame.motion.reset = false;
        let target = self.output.create_view(&Default::default());
        let buffers = self
            .renderer
            .draw(&self.device, &target, wgpu::Color::BLACK, &self.textures, None);
        self.queue.submit(buffers);
        let bytes = self.read(&self.output, 4);
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                // sRGB to linear.
                let c = f32::from(p[0]) / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            })
            .collect()
    }

    fn read(&self, texture: &wgpu::Texture, bytes_per_pixel: u32) -> Vec<u8> {
        let row = self.size * bytes_per_pixel;
        assert_eq!(row % 256, 0, "pick a size whose rows are 256-byte aligned");
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * self.size),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        readback.get_mapped_range(..).unwrap().to_vec()
    }
}

/// Mean absolute difference from `reference` over the pixels either image
/// has between black and white (the edge).
fn edge_error(image: &[f32], reference: &[f32]) -> f32 {
    let edge = |v: f32| v > 0.004 && v < 0.996;
    let (mut sum, mut n) = (0.0, 0);
    for (&a, &b) in image.iter().zip(reference) {
        if edge(a) || edge(b) {
            sum += (a - b).abs();
            n += 1;
        }
    }
    assert!(n > 20, "the image has an edge: {n}");
    sum / n as f32
}

/// `image` (n x n) averaged down by `k` in each direction, in linear light.
fn downsample(image: &[f32], n: usize, k: usize) -> Vec<f32> {
    let m = n / k;
    (0..m * m)
        .map(|i| {
            let (x, y) = (i % m, i / m);
            let mut sum = 0.0;
            for dy in 0..k {
                for dx in 0..k {
                    sum += image[(y * k + dy) * n + x * k + dx];
                }
            }
            sum / (k * k) as f32
        })
        .collect()
}

const SIZE: u32 = 64;

#[test]
fn a_still_diagonal_edge_converges_smoother_than_without_it() {
    let Some(mut rig) = Rig::new(SIZE, false) else {
        return;
    };
    rig.slab(0.37);
    let none = rig.draw();
    // The truth: the same picture at 8 times the size, averaged down.
    let mut big = Rig::new(SIZE * 8, false).unwrap();
    big.slab(0.37);
    let truth = downsample(&big.draw(), (SIZE * 8) as usize, 8);
    drop(big);

    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    let mut frames = Vec::new();
    for _ in 0..40 {
        frames.push(rig.draw());
    }
    assert!(rig.renderer.taa_active());
    let (without, with) = (edge_error(&none, &truth), edge_error(&frames[39], &truth));
    let first = edge_error(&frames[0], &truth);
    eprintln!(
        "edge error against 8x8 supersampling: none {without:.4}, TAA first frame {first:.4}, after 40 frames {with:.4}"
    );
    assert!(with < without * 0.75, "TAA converges: {with} vs {without}");
    let levels = |image: &[f32]| {
        let mut v: Vec<u32> = image
            .iter()
            .filter(|&&v| v > 0.004 && v < 0.996)
            .map(|&v| (v * 255.0) as u32)
            .collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    assert!(levels(&frames[39]) > levels(&none), "more edge shades");
    // Still means still: the converged picture stays put frame to frame.
    let wobble = edge_error(&frames[39], &frames[38]);
    assert!(wobble < 0.05, "a settled edge does not shimmer: {wobble}");
}

/// Reads the motion vector at pixel (x, y): xy over alpha.
fn motion_at(rig: &Rig, x: u32, y: u32) -> Option<(f32, f32)> {
    let texture = rig.renderer.velocity_texture()?;
    let bytes = rig.read(texture, 8);
    let at = ((y * rig.size + x) * 8) as usize;
    let half = |i: usize| half::f16::from_le_bytes([bytes[at + i * 2], bytes[at + i * 2 + 1]]).to_f32();
    let alpha = half(3);
    (alpha > 0.5).then(|| (half(0) / alpha, half(1) / alpha))
}

#[test]
fn a_moved_model_writes_its_motion_and_a_still_one_does_not() {
    let Some(mut rig) = Rig::new(SIZE * 2, false) else {
        return;
    };
    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    // The world holds one entity; moving it keeps it the same entity.
    rig.ctx.world.spawn((Transform3D::at(Vec3::ZERO), Model::new(rig.cube)));
    rig.draw();
    rig.draw();
    let center = rig.size / 2;
    assert!(
        motion_at(&rig, center, center).is_none(),
        "a model that stays put writes no motion"
    );
    let step = vec3(0.3, 0.1, 0.0);
    rig.ctx
        .world
        .query::<&mut Transform3D>()
        .for_each(|t| t.position += step);
    rig.draw();
    let view_proj = rig.ctx.camera3d.view_proj();
    // The pixel at the middle of the box's front face (half a unit nearer).
    let face = vec3(0.0, 0.0, 0.5);
    let (before, now) = (view_proj.project_point3(face), view_proj.project_point3(face + step));
    let expected = ((before.x - now.x) * 0.5, (now.y - before.y) * 0.5);
    let to_pixel = |p: Vec3| ((p.x * 0.5 + 0.5) * rig.size as f32, (0.5 - p.y * 0.5) * rig.size as f32);
    let (px, py) = to_pixel(now);
    let (vx, vy) = motion_at(&rig, px as u32, py as u32).expect("the moved model covers its center");
    eprintln!("motion {vx:.5}, {vy:.5}; expected {:.5}, {:.5}", expected.0, expected.1);
    assert!((vx - expected.0).abs() < 0.0005 && (vy - expected.1).abs() < 0.0005);
    assert!(
        expected.0 < 0.0 && expected.1 > 0.0,
        "it moved right and up: it was left and down"
    );
}

#[test]
fn a_moving_box_leaves_no_ghost_behind() {
    let Some(mut rig) = Rig::new(SIZE, false) else {
        return;
    };
    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    rig.ctx
        .world
        .spawn((Transform3D::at(vec3(-2.0, 0.0, 0.0)), Model::new(rig.cube)));
    // A still start, so the history holds the box, then a fast drift to the right.
    for _ in 0..8 {
        rig.draw();
    }
    let mut last = Vec::new();
    for _ in 0..12 {
        rig.ctx
            .world
            .query::<&mut Transform3D>()
            .for_each(|t| t.position.x += 0.3);
        last = rig.draw();
    }
    let n = SIZE as usize;
    // Where the box is now, in pixels; its trail is to the left of it.
    let view_proj = rig.ctx.camera3d.view_proj();
    let x_of = |x: f32| (view_proj.project_point3(vec3(x, 0.0, 0.0)).x * 0.5 + 0.5) * SIZE as f32;
    let box_now = -2.0 + 12.0 * 0.3;
    let tail = x_of(box_now - 0.5);
    let (y0, y1) = (n / 2 - 3, n / 2 + 3);
    let mut ghost = 0.0f32;
    let mut inside = 0.0f32;
    for y in y0..y1 {
        for x in (tail as usize).saturating_sub(14)..(tail as usize).saturating_sub(2) {
            ghost = ghost.max(last[y * n + x]);
        }
        inside = inside.max(last[y * n + x_of(box_now) as usize]);
    }
    eprintln!("ghost {ghost:.4}, box {inside:.4}");

    assert!(inside > 0.9, "the box is drawn: {inside}");
    assert!(ghost < 0.03, "no trail behind it: {ghost}");
}

#[test]
fn resetting_the_history_starts_over_from_the_current_frame() {
    let Some(mut rig) = Rig::new(SIZE, false) else {
        return;
    };
    rig.slab(0.37);
    let mut big = Rig::new(SIZE * 8, false).unwrap();
    big.slab(0.37);
    let truth = downsample(&big.draw(), (SIZE * 8) as usize, 8);
    drop(big);
    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    for _ in 0..40 {
        rig.draw();
    }
    let settled = edge_error(&rig.draw(), &truth);
    // A camera cut: the history is dropped, so the next picture is a single
    // jittered sample again.
    rig.frame.motion.reset = true;
    let cut = edge_error(&rig.draw(), &truth);
    // And it settles again from there.
    let mut again = 0.0;
    for _ in 0..40 {
        again = edge_error(&rig.draw(), &truth);
    }
    eprintln!("edge error: settled {settled:.4}, right after the cut {cut:.4}, 40 frames later {again:.4}");
    assert!(cut > settled * 1.3, "the cut frame has no history: {cut} vs {settled}");
    assert!(again < cut * 0.8, "and it converges again: {again} vs {cut}");
}

#[test]
fn downlevel_devices_fall_back_to_fxaa() {
    let Some(mut rig) = Rig::new(SIZE, true) else {
        return;
    };
    rig.slab(0.37);
    rig.ctx.post.anti_aliasing = AntiAliasing::Fxaa;
    let fxaa = rig.draw();
    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    let taa = rig.draw();
    assert!(!rig.renderer.taa_active(), "no TAA without compute");
    assert_eq!(fxaa, taa, "TAA draws as FXAA there");
}
