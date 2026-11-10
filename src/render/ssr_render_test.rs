//! Screen-space reflections: a mirror floor reflects what's on screen
//! above it. Skips without a GPU.
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
    /// Asks the next frame to forget the history (`reset_temporal_history`).
    reset_history: bool,
}

impl Rig {
    /// `None` without a GPU.
    fn new() -> Option<Self> {
        Self::sized(SIZE, SIZE)
    }

    fn sized(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            crate::render::render_test_support::no_gpu("no GPU adapter");
            return None;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &crate::render::render_test_support::forced(&wgpu::DeviceDescriptor {
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            }),
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
                width,
                height,
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
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(width as f32, height as f32));
        ctx.sky.atmosphere = false;
        Some(Self {
            device,
            queue,
            output,
            target,
            textures,
            renderer,
            ctx,
            reset_history: false,
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
        self.draw_timed(None)
    }

    fn draw_timed(&mut self, mut timer: Option<&mut super::timer::GpuTimer>) -> Vec<u8> {
        let width = self.output.width();
        let height = self.output.height();
        if let Some(t) = &mut timer {
            t.collect(&self.device);
            t.begin_frame();
        }
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        frame.motion.reset = std::mem::take(&mut self.reset_history);
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
                size: (width, height),
            },
        );
        let buffers = self.renderer.draw(
            &self.device,
            &self.target,
            wgpu::Color::BLACK,
            &self.textures,
            timer.as_deref(),
        );
        self.queue.submit(buffers);
        if let Some(t) = &mut timer {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            t.resolve(&mut encoder);
            self.queue.submit([encoder.finish()]);
            t.submitted();
        }
        // Rows padded to the copy alignment.
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
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
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.output.size(),
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = readback.get_mapped_range(..).unwrap();
        if row == width * 4 {
            return data.to_vec();
        }
        data.chunks(row as usize)
            .flat_map(|r| &r[..(width * 4) as usize])
            .copied()
            .collect()
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

/// A mirror floor and a glowing red box over it: with screen-space
/// reflections, the floor in front of the box shows it.
fn mirror_rig() -> Option<(Rig, Vec3, Vec3)> {
    floor_rig(0.0)
}

fn floor_rig(roughness: f32) -> Option<(Rig, Vec3, Vec3)> {
    let mut rig = Rig::new()?;
    if !crate::render::render_test_support::needs(&rig.device, crate::GpuFeature::ScreenSpaceReflections) {
        return None;
    }
    let mirror = MaterialDesc {
        color: Color::WHITE,
        roughness,
        metallic: 1.0,
        ..Default::default()
    };
    let glow = MaterialDesc {
        color: Color::BLACK,
        emissive: Color::rgb(4.0, 0.0, 0.0),
        ..Default::default()
    };
    let floor = rig.mesh(&MeshData::cube(1.0), mirror);
    let block = rig.mesh(&MeshData::cube(1.0), glow);
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, -0.5, 0.0),
            scale: vec3(20.0, 1.0, 20.0),
            ..Default::default()
        },
        Model::new(floor),
    ));
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Model::new(block),
    ));
    rig.ctx.lighting.shadows = false;
    rig.ctx.lighting.sun_intensity = 0.0;
    rig.ctx.post.bloom = 0.0;
    rig.ctx.camera3d.position = vec3(0.0, 2.0, 6.0);
    rig.ctx.camera3d.look_at(vec3(0.0, 0.5, 0.0));
    // The box's center mirrored is (0, -1, 0): seen from the camera through
    // the floor at (0, 0, 2). To the side, the floor reflects only the dark.
    Some((rig, vec3(0.0, 0.0, 2.0), vec3(2.5, 0.0, 2.0)))
}

#[test]
fn a_mirror_floor_reflects_the_box_above_it() {
    let Some((mut rig, reflection, aside)) = mirror_rig() else {
        return;
    };
    if !crate::render::render_test_support::needs(&rig.device, crate::GpuFeature::ScreenSpaceReflections) {
        return;
    }
    let view_proj = rig.ctx.camera3d.view_proj();
    let at = |pixels: &[u8], p: Vec3| {
        let (x, y) = pixel(view_proj, p);
        let i = ((y * SIZE + x) * 4) as usize;
        (i32::from(pixels[i]), i32::from(pixels[i + 1]))
    };
    let (red, _) = at(&rig.draw(), reflection);
    assert!(red < 40, "without SSR the floor reflects only the dark sky: {red}");

    rig.ctx.post.ssr = 1.0;
    // The first frame has no image of the last one to reflect.
    rig.draw();
    let pixels = rig.draw();
    let (red, green) = at(&pixels, reflection);
    assert!(red > 120 && green < 60, "the box's red, mirrored: ({red}, {green})");
    let (side, _) = at(&pixels, aside);
    assert!(side < 40, "nothing to reflect beside it: {side}");

    rig.ctx.post.ssr = 0.0;
    let (red, _) = at(&rig.draw(), reflection);
    assert!(red < 40, "off again: {red}");
}

#[test]
fn reflections_work_with_taa_and_fade_with_distance() {
    let Some((mut rig, reflection, _)) = mirror_rig() else {
        return;
    };
    if !crate::render::render_test_support::needs(&rig.device, crate::GpuFeature::ScreenSpaceReflections) {
        return;
    }
    let view_proj = rig.ctx.camera3d.view_proj();
    let red = |pixels: &[u8]| {
        let (x, y) = pixel(view_proj, reflection);
        i32::from(pixels[((y * SIZE + x) * 4) as usize])
    };
    rig.ctx.post.ssr = 1.0;
    rig.ctx.post.anti_aliasing = AntiAliasing::Taa;
    let mut pixels = Vec::new();
    for _ in 0..8 {
        pixels = rig.draw();
    }
    assert!(red(&pixels) > 120, "TAA keeps the reflection: {}", red(&pixels));

    // The box is about 1.7 units along the reflected ray: out of reach.
    rig.ctx.post.ssr_max_distance = 1.0;
    rig.ctx.post.anti_aliasing = AntiAliasing::None;
    rig.draw();
    let pixels = rig.draw();
    assert!(red(&pixels) < 40, "too far to reflect: {}", red(&pixels));
}

#[test]
fn reflections_and_ambient_occlusion_share_the_opaque_pass_image() {
    let Some((mut rig, reflection, _)) = mirror_rig() else {
        return;
    };
    if !crate::render::render_test_support::needs(&rig.device, crate::GpuFeature::ScreenSpaceReflections) {
        return;
    }
    let view_proj = rig.ctx.camera3d.view_proj();
    let red = |pixels: &[u8]| {
        let (x, y) = pixel(view_proj, reflection);
        i32::from(pixels[((y * SIZE + x) * 4) as usize])
    };
    rig.ctx.post.ambient_occlusion.enabled = true;
    rig.ctx.post.ao_half_resolution = true;
    rig.ctx.post.ao_temporal = true;
    let pixels = rig.draw();
    assert!(red(&pixels) < 40, "GTAO alone reflects nothing: {}", red(&pixels));
    rig.ctx.post.ssr = 1.0;
    rig.draw();
    let pixels = rig.draw();
    assert!(red(&pixels) > 120, "GTAO marks the reflections: {}", red(&pixels));
    rig.ctx.post.ambient_occlusion.enabled = false;
    rig.draw();
    let pixels = rig.draw();
    assert!(red(&pixels) > 120, "and so does a clear without it: {}", red(&pixels));
}

#[test]
fn a_glossy_floor_has_a_wider_lower_peak_reflection() {
    fn profile(roughness: f32) -> Option<(f32, f32)> {
        let (mut rig, reflection, _) = floor_rig(roughness)?;
        // Keep the bright source below display clipping so peak comparison
        // measures filtering rather than the tone mapper's saturation.
        rig.ctx.post.exposure = -4.0;
        let (cx, cy) = pixel(rig.ctx.camera3d.view_proj(), reflection);
        let off = rig.draw();
        rig.ctx.post.ssr = 1.0;
        rig.draw();
        let on = rig.draw();
        let mut energy = 0.0f32;
        let mut moment = 0.0f32;
        let mut peak = 0.0f32;
        for x in cx - 24..=cx + 24 {
            let at = ((cy * SIZE + x) * 4) as usize;
            assert!(off[at] < 40, "SSR off leaves the glossy floor dark");
            let value = (f32::from(on[at]) - f32::from(off[at])).max(0.0);
            energy += value;
            moment += value * (x as f32 - cx as f32).powi(2);
            peak = peak.max(value);
        }
        assert!(energy > 100.0, "the floor still reflects the box");
        rig.ctx.post.ssr = 0.0;
        let disabled = rig.draw();
        assert!(disabled[((cy * SIZE + cx) * 4) as usize] < 40);
        Some((peak, (moment / energy).sqrt()))
    }
    let Some(mirror) = profile(0.0) else { return };
    let Some(glossy) = profile(0.35) else { return };
    eprintln!("reflection peak / RMS width: mirror {mirror:?}, roughness 0.35 {glossy:?}");
    assert!(
        glossy.0 < mirror.0 * 0.95,
        "glossy peak falls: {mirror:?} -> {glossy:?}"
    );
    assert!(
        glossy.1 > mirror.1 * 1.1,
        "glossy reflection spreads: {mirror:?} -> {glossy:?}"
    );
}

#[test]
fn small_and_odd_sizes_filter_reflections() {
    // Too small for any filtered level, for some of them, and odd sizes.
    for (width, height) in [(1, 1), (3, 2), (5, 3), (37, 19), (130, 67)] {
        let Some(mut rig) = Rig::sized(width, height) else {
            return;
        };
        let floor = rig.mesh(
            &MeshData::cube(1.0),
            MaterialDesc {
                roughness: 0.6,
                metallic: 1.0,
                ..Default::default()
            },
        );
        rig.ctx.world.spawn((
            Transform3D {
                scale: vec3(20.0, 1.0, 20.0),
                ..Default::default()
            },
            Model::new(floor),
        ));
        rig.ctx.camera3d.position = vec3(0.0, 2.0, 6.0);
        rig.ctx.camera3d.look_at(Vec3::ZERO);
        rig.ctx.post.ssr = 1.0;
        for _ in 0..3 {
            assert_eq!(rig.draw().len(), (width * height * 4) as usize);
        }
    }
}

/// Offscreen timestamp measurement; run with --release --ignored --nocapture.
/// It prints the trace's span and the frame's GPU time with and without the
/// filtered levels, in alternating blocks of frames so both see the same
/// GPU clocks. (The filter's own span isn't printed: tile-based GPUs run
/// it alongside the start of the opaque pass, so the profiler, which
/// charges each pass from where the one before it ended, charges little or
/// none of it there.)
#[test]
#[ignore]
fn reflection_filter_gpu_cost() {
    let Some(mut rig) = Rig::sized(1920, 1080) else { return };
    let Some(mut timer) = super::timer::GpuTimer::new(&rig.device, &rig.queue) else {
        eprintln!("timestamp queries unavailable: skipped");
        return;
    };
    let floor = rig.mesh(
        &MeshData::cube(1.0),
        MaterialDesc {
            roughness: 0.35,
            metallic: 1.0,
            ..Default::default()
        },
    );
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, -0.5, 0.0),
            scale: vec3(20.0, 1.0, 20.0),
            ..Default::default()
        },
        Model::new(floor),
    ));
    let block = rig.mesh(
        &MeshData::cube(1.0),
        MaterialDesc {
            color: Color::BLACK,
            emissive: Color::rgb(4.0, 0.0, 0.0),
            ..Default::default()
        },
    );
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Model::new(block),
    ));
    rig.ctx.camera3d.position = vec3(0.0, 2.0, 6.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    rig.ctx.post.ssr = 1.0;
    rig.ctx.post.bloom = 0.0;
    rig.ctx.lighting.shadows = false;
    // [with the filter, without]
    let mut frames = [Vec::new(), Vec::new()];
    let mut trace = Vec::new();
    for frame in 0..1230 {
        let skip = (frame / 20) % 2 == 1;
        super::ssr::SKIP_FILTER.store(skip, std::sync::atomic::Ordering::Relaxed);
        rig.draw_timed(Some(&mut timer));
        // Results come back a few frames late: keep each block's last 15.
        if frame < 30 || frame % 20 < 5 {
            continue;
        }
        frames[usize::from(skip)].push(timer.last_frame.iter().map(|s| s.ms).sum::<f32>());
        if let Some(pass) = timer.last_frame.iter().find(|p| p.name == "screen reflections") {
            trace.push(pass.ms);
        }
    }
    super::ssr::SKIP_FILTER.store(false, std::sync::atomic::Ordering::Relaxed);
    assert!(!trace.is_empty(), "GPU timestamps must be collected");
    let median = |samples: &mut Vec<f32>| {
        samples.sort_by(f32::total_cmp);
        samples[samples.len() / 2]
    };
    let [mut with, mut without] = frames;
    let (with, without) = (median(&mut with), median(&mut without));
    eprintln!(
        "1920x1080: trace median {:.4} ms; frame median {with:.4} ms with the filtered levels, \
         {without:.4} ms without (+{:.4} ms)",
        median(&mut trace),
        with - without
    );
}

#[test]
fn a_history_reset_drops_the_last_frames_image_from_the_reflections() {
    let Some((mut rig, reflection, _)) = mirror_rig() else {
        return;
    };
    if !crate::render::render_test_support::needs(&rig.device, crate::GpuFeature::ScreenSpaceReflections) {
        return;
    }
    // A dark box far off to the side, to swap with the glowing one.
    let shade = rig.mesh(
        &MeshData::cube(1.0),
        MaterialDesc {
            color: Color::BLACK,
            roughness: 1.0,
            ..Default::default()
        },
    );
    rig.ctx
        .world
        .spawn((Transform3D::at(vec3(30.0, 1.0, 0.0)), Model::new(shade)));
    let view_proj = rig.ctx.camera3d.view_proj();
    let red = |pixels: &[u8]| {
        let (x, y) = pixel(view_proj, reflection);
        i32::from(pixels[((y * SIZE + x) * 4) as usize])
    };
    let swap = |rig: &mut Rig| {
        rig.ctx.world.query::<&mut Transform3D>().for_each(|t| {
            if t.position.y == 1.0 {
                t.position.x = if t.position.x == 0.0 { 30.0 } else { 0.0 };
            }
        });
    };
    rig.ctx.post.ssr = 1.0;
    rig.draw();
    let settled = red(&rig.draw());
    assert!(settled > 120, "the glowing box is reflected: {settled}");
    // Negative control: swapped without a reset, the dark box is reflected
    // in the colors the last frame had there.
    swap(&mut rig);
    let stale = red(&rig.draw());
    let next = red(&rig.draw());
    eprintln!("reflection: settled {settled}, swapped without a reset {stale}, a frame later {next}");
    assert!(stale > 120, "the stale image shows the old box: {stale}");
    assert!(next < 40, "and the next frame the dark one: {next}");
    swap(&mut rig);
    rig.draw();
    assert!(red(&rig.draw()) > 120, "the glowing box is back");
    // With a reset the swap frame reflects no old colors.
    swap(&mut rig);
    rig.reset_history = true;
    let reset = red(&rig.draw());
    let after = red(&rig.draw());
    eprintln!("reflection: swapped with a reset {reset}, a frame later {after}");
    assert!(reset < 40, "a reset frame reflects nothing stale: {reset}");
    assert!(after < 40, "and the next shows the dark box: {after}");
}
