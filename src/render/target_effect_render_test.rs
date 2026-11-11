//! Target effects: an effect shader run over the lit 3D frame that writes
//! a render target of its own, at a fixed point of the frame (after the
//! transparent surfaces, before screen effects). Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
#[allow(unused_imports)]
use crate::d3::Transform as Transform3D;
use crate::prelude::*;
#[allow(unused_imports)]
use crate::render::{MaterialDesc, Mesh, MeshData, Model};
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
    /// Screen effects to run: (effect, material).
    effects: Vec<(u32, u32)>,
    /// Target effects to run: (effect, render target id, material).
    target_effects: Vec<(u32, u32, u32)>,
    /// Texture compute passes to run: (compute, render target id, material).
    computes: Vec<(u32, u32, u32)>,
    /// Materials made on the GPU so far (ids follow this count).
    materials: u32,
    /// The bind group layout of sprite textures (for render targets).
    layout: wgpu::BindGroupLayout,
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
            effects: Vec::new(),
            target_effects: Vec::new(),
            computes: Vec::new(),
            materials: 0,
            layout,
        })
    }

    /// A mesh of `data` with `desc`, on the CPU and the GPU.
    fn mesh(&mut self, data: &MeshData, desc: MaterialDesc) -> Mesh {
        let material = self.ctx.gfx.create_material(desc);
        let mesh = self.ctx.gfx.create_mesh(data, material);
        self.gpu_material(desc);
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], true);
        mesh
    }

    /// A material on the GPU only (for effects), returning its id.
    fn gpu_material(&mut self, desc: MaterialDesc) -> u32 {
        let id = self
            .renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        assert_eq!(id, self.materials);
        self.materials += 1;
        id
    }

    /// A `width` x `height` render target, made as the engine makes them:
    /// a texture id and a texture the renderer knows as a render target.
    fn render_target(&mut self, width: u32, height: u32) -> (Texture, wgpu::Texture) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = self.device.create_sampler(&Default::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
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
        });
        let id = self.textures.len() as u32;
        self.renderer.add_render_target(id, &texture);
        self.textures.push(GpuTexture {
            texture: texture.clone(),
            nearest: false,
            mips: Some(false),
            bind_group,
        });
        (Texture::new(id, width, height), texture)
    }

    /// Reads a render target back: RGB per texel, row by row from the top.
    fn read_target(&self, texture: &wgpu::Texture) -> Vec<[f32; 3]> {
        let size = texture.size();
        let row = (size.width * 8).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * size.height),
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
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = readback.get_mapped_range(..).unwrap().to_vec();
        let mut texels = Vec::with_capacity((size.width * size.height) as usize);
        for y in 0..size.height {
            for x in 0..size.width {
                let at = (y * row + x * 8) as usize;
                let channel = |c: usize| half::f16::from_le_bytes([bytes[at + c * 2], bytes[at + c * 2 + 1]]).to_f32();
                texels.push([channel(0), channel(1), channel(2)]);
            }
        }
        texels
    }

    /// Draws the world and reads the pixels back (RGBA).
    fn draw(&mut self) -> Vec<u8> {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        frame.effects.clone_from(&self.effects);
        frame.target_effects.clone_from(&self.target_effects);
        frame.computes.clone_from(&self.computes);
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
            // Sunlight only.
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

/// A ground block under one half of the view (the right one on screen),
/// sky (nothing) under the other, seen from straight above.
fn half_ground(rig: &mut Rig) {
    let ground = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::WHITE));
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(-10.0, -0.5, 0.0),
            scale: vec3(20.0, 1.0, 40.0),
            ..Default::default()
        },
        Model::new(ground),
    ));
    rig.ctx.camera3d.position = vec3(0.0, 12.0, 0.01);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
}

/// Whether the device reported a validation error while running `f`.
fn validation_error(rig: &mut Rig, f: impl FnOnce(&mut Rig)) -> Option<wgpu::Error> {
    let scope = rig.device.push_error_scope(wgpu::ErrorFilter::Validation);
    f(rig);
    pollster::block_on(scope.pop())
}

#[test]
fn target_effects_write_their_target_and_leave_the_frame_alone() {
    let Some(mut rig) = Rig::new() else { return };
    half_ground(&mut rig);
    let before = rig.draw();
    // A target of another size and shape than the frame: each texel reads
    // the frame at the matching point.
    let (target, texture) = rig.render_target(32, 16);
    let effect = rig
        .renderer
        .create_effect(
            &rig.device,
            "mark",
            "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(select(1.0, 0.0, p.sky), p.uv.x, p.uv.y); }",
        )
        .unwrap();
    rig.target_effects.push((effect, target.id, 0));
    let mut after = Vec::new();
    assert!(validation_error(&mut rig, |rig| after = rig.draw()).is_none());
    assert_eq!(rig.renderer.target_effect_count(), 1);
    assert!(before == after, "the frame is unchanged");
    let texels = rig.read_target(&texture);
    for y in 0..16 {
        for x in 0..32 {
            let [ground, u, v] = texels[(y * 32 + x) as usize];
            // The frame pixel under the texel's centre, and its centre in uv.
            // A centre on the line between two pixels (here every one: 4 frame
            // pixels a texel) may read either.
            let under = |t: u32, n: u32| ((t as f32 + 0.5) * SIZE as f32 / n as f32).floor() + 0.5;
            let (px, py) = (under(x, 32), under(y, 16));
            let near = 1.0 / SIZE as f32 + 2e-3;
            assert!((u - px / SIZE as f32).abs() < near, "texel {x},{y}: u {u}");
            assert!((v - py / SIZE as f32).abs() < near, "texel {x},{y}: v {v}");
            // Away from the edge between ground and sky.
            if x < 14 {
                assert_eq!(ground, 0.0, "texel {x},{y} sees the sky");
            } else if x > 17 {
                assert_eq!(ground, 1.0, "texel {x},{y} sees the ground");
            }
        }
    }
}

#[test]
fn screen_effects_see_this_frames_target_effects() {
    let Some(mut rig) = Rig::new() else { return };
    half_ground(&mut rig);
    let (target, _) = rig.render_target(4, 4);
    let fill = rig
        .renderer
        .create_effect(
            &rig.device,
            "fill",
            "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(0.0, 1.0, 0.0); }",
        )
        .unwrap();
    // A screen effect showing what the target holds, everywhere.
    let show = rig
        .renderer
        .create_effect(
            &rig.device,
            "show",
            "fn effect(p: Pixel) -> vec3<f32> { return textureLoad(custom_texture0, vec2<i32>(1, 1), 0).rgb; }",
        )
        .unwrap();
    let reads_target = rig.gpu_material(MaterialDesc {
        custom_textures: [Some(target), None],
        ..Default::default()
    });
    rig.effects.push((show, reads_target));
    // Without the target effect the target holds 0: a black frame.
    let black = rig.draw();
    assert!(black.chunks(4).all(|p| p[..3] == [0, 0, 0]), "the target starts at 0");
    rig.target_effects.push((fill, target.id, 0));
    let shown = rig.draw();
    let centre = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    let [r, g, b] = [shown[centre], shown[centre + 1], shown[centre + 2]];
    assert!(g > 150 && r < 20 && b < 20, "this frame's green: {r} {g} {b}");
}

#[test]
fn target_effects_sampling_their_own_target_are_skipped() {
    let Some(mut rig) = Rig::new() else { return };
    half_ground(&mut rig);
    let before = rig.draw();
    let (target, texture) = rig.render_target(8, 8);
    let fill = rig
        .renderer
        .create_effect(
            &rig.device,
            "fill",
            "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(1.0); }",
        )
        .unwrap();
    let reads_itself = rig.gpu_material(MaterialDesc {
        custom_textures: [None, Some(target)],
        ..Default::default()
    });
    rig.target_effects.push((fill, target.id, reads_itself));
    let mut after = Vec::new();
    assert!(
        validation_error(&mut rig, |rig| after = rig.draw()).is_none(),
        "no read-write hazard reaches the GPU"
    );
    assert_eq!(rig.renderer.target_effect_count(), 0, "skipped");
    assert!(before == after);
    assert!(
        rig.read_target(&texture).iter().all(|t| *t == [0.0; 3]),
        "never written"
    );
}

#[test]
fn target_effects_follow_hot_reloaded_effects() {
    let Some(mut rig) = Rig::new() else { return };
    half_ground(&mut rig);
    let (target, texture) = rig.render_target(4, 4);
    let effect = rig
        .renderer
        .create_effect(
            &rig.device,
            "e",
            "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(0.25); }",
        )
        .unwrap();
    rig.target_effects.push((effect, target.id, 0));
    rig.draw();
    assert_eq!(rig.read_target(&texture)[0], [0.25; 3]);
    rig.renderer
        .replace_effect(
            &rig.device,
            effect,
            "e",
            "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(0.75); }",
        )
        .unwrap();
    rig.draw();
    assert_eq!(rig.read_target(&texture)[0], [0.75; 3]);
}

/// The pixel in the middle of a drawn frame.
fn centre(frame: &[u8]) -> [u8; 3] {
    let at = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    [frame[at], frame[at + 1], frame[at + 2]]
}

#[test]
fn two_screen_effects_run_in_order_swapping_between_two_images() {
    let Some(mut rig) = Rig::new() else { return };
    half_ground(&mut rig);
    let effect = |rig: &mut Rig, name, source| rig.renderer.create_effect(&rig.device, name, source).unwrap();
    // The first leaves red everywhere; the second moves red into green.
    let red = effect(
        &mut rig,
        "red",
        "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(1.0, 0.0, 0.0); }",
    );
    let turn = effect(
        &mut rig,
        "turn",
        "fn effect(p: Pixel) -> vec3<f32> { return p.color.grb; }",
    );
    let material = rig.gpu_material(MaterialDesc::default());
    rig.effects.push((red, material));
    let [r, g, b] = centre(&rig.draw());
    assert!(r > 150 && g < 20 && b < 20, "one effect: red: {r} {g} {b}");
    let one = last_features().unwrap();
    assert!(one.contains(FrameFeatures::EFFECTS.with(FrameFeatures::EFFECTS_ODD)));
    assert!(!one.contains(FrameFeatures::EFFECTS_BACK));
    // The second reads what the first wrote and writes back into the lit
    // frame, where bloom and tone mapping then read it.
    rig.effects.push((turn, material));
    let [r, g, b] = centre(&rig.draw());
    assert!(g > 150 && r < 20 && b < 20, "two effects: green: {r} {g} {b}");
    let two = last_features().unwrap();
    assert!(two.contains(FrameFeatures::EFFECTS.with(FrameFeatures::EFFECTS_BACK)));
    assert!(!two.contains(FrameFeatures::EFFECTS_ODD));
    // A third ends in the other image again.
    rig.effects.push((turn, material));
    let [r, g, b] = centre(&rig.draw());
    assert!(r > 150 && g < 20 && b < 20, "three effects: red again: {r} {g} {b}");
    let three = FrameFeatures::EFFECTS_BACK.with(FrameFeatures::EFFECTS_ODD);
    assert!(last_features().unwrap().contains(three));
}

#[test]
fn a_texture_compute_pass_fills_its_target_before_the_frame_reads_it() {
    let Some(mut rig) = Rig::new() else { return };
    if rig.device.limits().max_storage_textures_per_shader_stage == 0 {
        return;
    }
    half_ground(&mut rig);
    let (target, texture) = rig.render_target(4, 4);
    let fill = rig
        .renderer
        .create_compute(
            &rig.device,
            "fill",
            "fn compute(c: Cell) -> vec4<f32> { return vec4<f32>(0.0, 1.0, 0.0, 1.0); }",
        )
        .unwrap();
    let show = rig
        .renderer
        .create_effect(
            &rig.device,
            "show",
            "fn effect(p: Pixel) -> vec3<f32> { return textureLoad(custom_texture0, vec2<i32>(1, 1), 0).rgb; }",
        )
        .unwrap();
    let reads_target = rig.gpu_material(MaterialDesc {
        custom_textures: [Some(target), None],
        ..Default::default()
    });
    let plain = rig.gpu_material(MaterialDesc::default());
    rig.effects.push((show, reads_target));
    rig.computes.push((fill, target.id, plain));
    let [r, g, b] = centre(&rig.draw());
    assert!(last_features().unwrap().contains(FrameFeatures::TEXTURE_COMPUTE));
    assert!(
        g > 150 && r < 20 && b < 20,
        "the frame shows this frame's fill: {r} {g} {b}"
    );
    let texels = rig.read_target(&texture);
    assert!(texels.iter().all(|t| t[1] > 0.9 && t[0] < 0.1), "{:?}", texels[0]);
}
