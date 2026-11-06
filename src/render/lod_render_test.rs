//! Level-of-detail groups drawn offscreen: the right mesh for the camera's
//! distance, a crossfade drawing both levels through complementary dither
//! patterns without holes (depth prepass and main pass agreeing), and the
//! shadow maps following the level and its fade. Skips without a GPU.
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
    fn new(solid_prepass: bool) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return None;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) else {
            eprintln!("no GPU device: skipped");
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

    /// Draws the world and reads the pixels back (RGBA).
    fn draw(&mut self) -> Vec<u8> {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        frame.collect(
            &mut ctx.world,
            &ctx.assets3d,
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

/// A material that shows as a flat `color`, whatever the light.
fn glowing(color: Color) -> MaterialDesc {
    MaterialDesc {
        color: Color::rgb(0.0, 0.0, 0.0),
        emissive: color,
        roughness: 1.0,
        ..Default::default()
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
struct Count {
    red: usize,
    blue: usize,
    dark: usize,
}

fn count(pixels: &[u8]) -> Count {
    let mut c = Count::default();
    for p in pixels.as_chunks::<4>().0 {
        let (r, g, b) = (u32::from(p[0]), u32::from(p[1]), u32::from(p[2]));
        if r > 120 && g < 60 && b < 60 {
            c.red += 1;
        } else if b > 120 && g < 60 && r < 60 {
            c.blue += 1;
        } else if r + g + b < 40 {
            c.dark += 1;
        }
    }
    c
}

/// The camera `d` in front of the origin.
fn camera_at(ctx: &mut Ctx, d: f32) {
    ctx.camera3d.position = vec3(0.0, 0.0, d);
    ctx.camera3d.look_at(Vec3::ZERO);
}

/// Level 0 is a small red cube, level 1 a bigger, nearer blue one (so a
/// depth pass that ignored the fade would hide the red behind the blue).
fn two_level_scene(rig: &mut Rig) {
    let red = rig.mesh(&MeshData::cube(3.0), glowing(Color::rgb(1.0, 0.0, 0.0)));
    let blue = rig.mesh(&MeshData::cube(3.6), glowing(Color::rgb(0.0, 0.0, 1.0)));
    // Size 8: 8 / (2 d tan 30°) of the screen's height at distance d.
    // Level 0 from 30% up, fading into level 1 over 30% to 65%.
    let group = LodGroup::new(
        8.0,
        vec![
            LodLevel::new(0.30, vec![LodRenderer::new(red)]).with_fade(0.5),
            LodLevel::new(0.05, vec![LodRenderer::new(blue)]),
        ],
    );
    rig.ctx.world.spawn((Transform3D::default(), group));
}

/// A distance where the group covers `height` of the screen.
fn distance_for(height: f32) -> f32 {
    LodGroup::distance_at(8.0, height, 60f32.to_radians())
}

#[test]
fn the_level_for_the_distance_is_the_one_drawn() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        two_level_scene(&mut rig);
        camera_at(&mut rig.ctx, distance_for(0.9));
        let near = count(&rig.draw());
        assert!(near.red > 150 && near.blue == 0, "near: only the red cube: {near:?}");
        camera_at(&mut rig.ctx, distance_for(0.15));
        let far = count(&rig.draw());
        assert!(far.blue > 40 && far.red == 0, "far: only the blue cube: {far:?}");
        camera_at(&mut rig.ctx, distance_for(0.02));
        let gone = count(&rig.draw());
        assert!(
            gone.red == 0 && gone.blue == 0,
            "past the last level nothing is drawn: {gone:?}"
        );
        // The bias moves the switch: at the far distance, a bigger bias
        // brings the red cube back (5 x 0.15 is above the fade band).
        camera_at(&mut rig.ctx, distance_for(0.15));
        rig.ctx.lod.bias = 5.0;
        let biased = count(&rig.draw());
        assert!(biased.red > 20 && biased.blue == 0, "bias 4: {biased:?}");
    }
}

#[test]
fn a_crossfade_draws_both_levels_through_complementary_dither() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        two_level_scene(&mut rig);
        // Half way through the band: 0.475 of the screen.
        camera_at(&mut rig.ctx, distance_for(0.475));
        let mid = rig.draw();
        let faded = count(&mid);
        let drawn = faded.red + faded.blue;
        assert!(drawn > 150, "{faded:?}");
        assert!(
            faded.red * 4 > drawn && faded.blue * 4 > drawn,
            "half and half, give or take: {faded:?}"
        );
        // No holes: wherever the red cube is solid in an undithered frame
        // (crossfades off, level 0 only), the dithered one shows red or
        // blue, never the background.
        rig.ctx.lod.crossfade = false;
        camera_at(&mut rig.ctx, distance_for(0.475));
        let solid = rig.draw();
        let (red_only, blue_only) = (count(&solid).red, count(&solid).blue);
        assert!(red_only == 0 || blue_only == 0, "crossfade off draws one level");
        // Level 1 alone covers more than level 0 alone, so use the pixels
        // of the level-0 cube: shrink the check to its middle.
        let at = |p: &[u8], x: u32, y: u32| {
            let i = ((y * SIZE + x) * 4) as usize;
            u32::from(p[i]) + u32::from(p[i + 1]) + u32::from(p[i + 2])
        };
        let (mut holes, mut inside) = (0, 0);
        for y in 0..SIZE {
            for x in 0..SIZE {
                // The undithered red cube's interior, two pixels in.
                let covered = |dx: i32, dy: i32| {
                    let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                    (0..SIZE as i32).contains(&xx)
                        && (0..SIZE as i32).contains(&yy)
                        && at(&solid, xx as u32, yy as u32) > 100
                };
                if (-2..=2).all(|dy| (-2..=2).all(|dx| covered(dx, dy))) {
                    inside += 1;
                    if at(&mid, x, y) < 100 {
                        holes += 1;
                    }
                }
            }
        }
        assert!(inside > 50, "solid cube has an interior ({inside})");
        assert_eq!(
            holes, 0,
            "{holes} of {inside} interior pixels are holes (prepass {solid_prepass})"
        );
    }
}

#[test]
fn the_dither_follows_the_blend() {
    let Some(mut rig) = Rig::new(true) else {
        return;
    };
    two_level_scene(&mut rig);
    let share = |rig: &mut Rig, height: f32| {
        camera_at(&mut rig.ctx, distance_for(height));
        let c = count(&rig.draw());
        c.red as f32 / (c.red + c.blue).max(1) as f32
    };
    // Band 0.30 to 0.65: the red cube's share of the pixels rises with the
    // screen height from 0 to 1.
    let low = share(&mut rig, 0.36);
    let mid = share(&mut rig, 0.475);
    let high = share(&mut rig, 0.59);
    assert!(low < mid && mid < high, "{low} {mid} {high}");
    assert!(low < 0.5 && high > 0.5, "{low} {high}");
}

/// A glowing cube over a white ground, lit by a sun straight above: the
/// ground under the cube is in its shadow, but not if the level shown casts
/// none.
fn shadow_scene(rig: &mut Rig) -> impl Fn(&[u8]) -> u32 + use<> {
    rig.ctx.lighting.sun_direction = vec3(0.0, -1.0, 0.0001);
    rig.ctx.lighting.shadows = true;
    let matte = MaterialDesc {
        roughness: 1.0,
        ..Default::default()
    };
    let ground = rig.mesh(&MeshData::plane(30.0), matte);
    rig.ctx.world.spawn((Transform3D::default(), Model::new(ground)));
    let red = rig.mesh(&MeshData::cube(1.0), glowing(Color::rgb(1.0, 0.0, 0.0)));
    let blue = rig.mesh(&MeshData::cube(1.0), glowing(Color::rgb(0.0, 0.0, 1.0)));
    rig.ctx.camera3d.position = vec3(0.0, 6.0, 6.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    // 6.7 from the camera; size chosen so the group covers 60% of the
    // screen: half way through the fade band (50% to 70%).
    let distance = rig.ctx.camera3d.position.distance(vec3(0.0, 3.0, 0.0));
    let size = LodGroup::screen_height_at(1.0, distance, rig.ctx.camera3d.fov_y).recip() * 0.6;
    let group = LodGroup::new(
        size,
        vec![
            LodLevel::new(0.5, vec![LodRenderer::new(red)]).with_fade(0.4),
            // The far level shows a cube too but casts no shadow.
            LodLevel::new(0.1, vec![LodRenderer::new(blue)]).with_shadow(LodShadow::Casters(vec![])),
        ],
    )
    .with_center(Vec3::ZERO);
    rig.ctx.world.spawn((Transform3D::at(vec3(0.0, 3.0, 0.0)), group));
    let view_proj = rig.ctx.camera3d.view_proj();
    move |pixels: &[u8]| {
        let ndc = view_proj.project_point3(vec3(0.0, 0.0, 0.3));
        let x = ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32;
        let y = ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32;
        // A few pixels around, so the dither averages out.
        let mut sum = 0;
        for dy in 0..3 {
            for dx in 0..3 {
                let i = (((y + dy - 1) * SIZE + x + dx - 1) * 4) as usize;
                sum += u32::from(pixels[i]) + u32::from(pixels[i + 1]) + u32::from(pixels[i + 2]);
            }
        }
        sum
    }
}

#[test]
fn shadows_follow_the_level_and_its_fade() {
    let Some(mut rig) = Rig::new(true) else {
        return;
    };
    let ground_at = shadow_scene(&mut rig);
    rig.ctx.lod.bias = 2.0; // 120% of the screen: level 0 alone
    let near = ground_at(&rig.draw());
    rig.ctx.lod.bias = 0.5; // 30%: level 1 alone, which casts no shadow
    let far = ground_at(&rig.draw());
    rig.ctx.lod.bias = 1.0; // 60%: a crossfade of the two
    let mid = ground_at(&rig.draw());
    assert!(
        near * 3 < far,
        "level 0 shadows the ground, level 1 doesn't: {near} vs {far}"
    );
    assert!(
        near < mid && mid < far,
        "half way through the fade the shadow is half there: {near} < {mid} < {far}"
    );
}
