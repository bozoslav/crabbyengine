//! Headless GPU tests of graphics quality presets switched at run time:
//! every preset renders without validation errors, a scaled scene still
//! fills the whole window, and shadow maps are made again at the new size.
//! Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

const W: u32 = 64;
const H: u32 = 48;

struct Fixture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    textures: Vec<GpuTexture>,
    output: wgpu::Texture,
    target: wgpu::TextureView,
    renderer: Renderer3D,
    ctx: Ctx,
    /// Kept between frames, as the engine keeps it (far shadow cascades
    /// can be kept from an earlier frame).
    frame: Frame3D,
}

impl Fixture {
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
                width: W,
                height: H,
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

        // A ground, a box on it, a sun, and a shadowed spot light above.
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(W as f32, H as f32));
        let matte = MaterialDesc {
            color: Color::WHITE,
            roughness: 1.0,
            ..Default::default()
        };
        let material = ctx.gfx.create_material(matte);
        let ground = MeshData::plane(20.0);
        let cube = MeshData::cube(1.0);
        let ground_mesh = ctx.gfx.create_mesh(&ground, material);
        let cube_mesh = ctx.gfx.create_mesh(&cube, material);
        ctx.world.spawn((Transform3D::default(), Model::new(ground_mesh)));
        ctx.world
            .spawn((Transform3D::at(vec3(0.0, 0.5, 0.0)), Model::new(cube_mesh)));
        let lamp = Transform3D::at(vec3(1.5, 4.0, 1.0)).looking_at(vec3(0.0, 0.0, 0.0));
        ctx.world.spawn((
            lamp,
            SpotLight3D {
                intensity: 60.0,
                range: 12.0,
                shadows: true,
                ..Default::default()
            },
        ));
        ctx.camera3d.position = vec3(0.0, 5.0, 6.0);
        ctx.camera3d.look_at(Vec3::ZERO);
        ctx.sky.atmosphere = false;
        ctx.post.bloom = 0.0;

        let mut renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, true);
        renderer.create_material(&device, &GpuMaterialDesc::new(&matte), &textures);
        renderer.create_mesh(&device, &ground, &[(0..ground.indices.len() as u32, material)], true);
        renderer.create_mesh(&device, &cube, &[(0..cube.indices.len() as u32, material)], true);
        Some(Self {
            device,
            queue,
            textures,
            output,
            target,
            renderer,
            ctx,
            frame: Frame3D::default(),
        })
    }

    /// Draws one frame and reads the window back. Fails the test on a
    /// validation error.
    fn draw(&mut self) -> Vec<u8> {
        let (device, queue, ctx) = (&self.device, &self.queue, &mut self.ctx);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let frame = &mut self.frame;
        frame.size_limit = device.limits().max_texture_dimension_2d;
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
            light: SkyLight {
                sun: Vec3::splat(ctx.lighting.sun_intensity),
                sky: Vec3::ONE,
                ground: Vec3::ONE,
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
            occlusion: false,
        };
        self.renderer.prepare(
            device,
            queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (W, H),
            },
        );
        let buffers = self
            .renderer
            .draw(device, &self.target, wgpu::Color::BLACK, &self.textures, None);
        queue.submit(buffers);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(W * H * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(W * 4),
                    rows_per_image: None,
                },
            },
            self.output.size(),
        );
        queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = readback.get_mapped_range(..).unwrap().to_vec();
        if let Some(error) = pollster::block_on(scope.pop()) {
            panic!("validation error: {error}");
        }
        pixels
    }
}

/// The mean absolute difference per channel, 0 to 255.
fn difference(a: &[u8], b: &[u8]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(&a, &b)| a.abs_diff(b) as f32).sum::<f32>() / a.len() as f32
}

/// How many pixels of a quarter-size block in each corner of the window are
/// lit (not black).
fn lit_corners(pixels: &[u8]) -> [usize; 4] {
    let lit = |x0: u32, y0: u32| {
        let mut n = 0;
        for y in y0..y0 + H / 4 {
            for x in x0..x0 + W / 4 {
                let i = ((y * W + x) * 4) as usize;
                n += usize::from(pixels[i] > 8 || pixels[i + 1] > 8 || pixels[i + 2] > 8);
            }
        }
        n
    };
    [
        lit(0, 0),
        lit(W - W / 4, 0),
        lit(0, H - H / 4),
        lit(W - W / 4, H - H / 4),
    ]
}

#[test]
fn switching_presets_back_and_forth_renders_clean_full_size_frames() {
    let Some(mut f) = Fixture::new() else { return };
    // The ground fills the lower half of the picture; the first draw is
    // the reference at the engine's defaults.
    let reference = f.draw();
    let lit = lit_corners(&reference);
    assert!(
        lit[2] > 0 && lit[3] > 0,
        "the ground reaches the bottom corners: {lit:?}"
    );
    let levels = [
        GraphicsQuality::low(),
        GraphicsQuality::ultra(),
        GraphicsQuality::medium(),
        GraphicsQuality::high(),
        GraphicsQuality::low(),
        GraphicsQuality::default(),
        GraphicsQuality::ultra(),
        GraphicsQuality::medium(),
    ];
    for (round, quality) in levels.into_iter().enumerate() {
        f.ctx.gfx.set_quality(&mut f.ctx.post, &mut f.ctx.lighting, quality);
        let first = f.draw();
        let second = f.draw();
        assert_eq!(
            first.len(),
            (W * H * 4) as usize,
            "round {round}: the output is the window's size"
        );
        if quality.ao_temporal {
            // Rotating AO slices and history deliberately change between frames.
            let delta = difference(&first, &second);
            assert!(delta < 1.0, "round {round}: temporal AO difference {delta}");
        } else {
            assert_eq!(first, second, "round {round}: stable spatial rendering");
        }
        let lit = lit_corners(&first);
        assert!(
            lit[2] > 0 && lit[3] > 0,
            "round {round} ({:?}): a scaled scene still reaches the window's bottom corners: {lit:?}",
            quality.level()
        );
        let scene = f.renderer.scene_size().unwrap();
        assert_eq!(
            scene,
            ctx_scene_size(&f.ctx),
            "round {round}: the scene is drawn at the scaled size"
        );
        assert_eq!(f.renderer.scales_output(), quality.render_scale != 1.0);
    }
    // Back to where it started: the same picture as the first time.
    f.ctx
        .gfx
        .set_quality(&mut f.ctx.post, &mut f.ctx.lighting, GraphicsQuality::default());
    let again = f.draw();
    assert_eq!(again, reference, "switching away and back changes nothing");
}

fn ctx_scene_size(ctx: &Ctx) -> (u32, u32) {
    ctx.post.render_size((W, H))
}

#[test]
fn a_scaled_scene_is_the_same_picture_at_the_windows_size() {
    let Some(mut f) = Fixture::new() else { return };
    f.ctx.lighting.shadows = false;
    let full = f.draw();
    assert_eq!(f.renderer.scene_size(), Some((W, H)));
    assert!(!f.renderer.scales_output(), "no extra pass at scale 1");
    f.ctx.post.render_scale = 0.5;
    let soft = f.draw();
    f.ctx.post.sharpen = 1.0;
    let sharp = f.draw();
    assert_ne!(soft, sharp, "sharpening changes a scaled picture");
    f.ctx.post.sharpen = 0.0;
    f.ctx.post.render_scale = 1.0;
    for (scale, sharpen) in [(0.5, 0.0), (0.5, 1.0), (0.75, 0.5), (1.5, 0.0), (2.0, 0.5)] {
        f.ctx.post.render_scale = scale;
        f.ctx.post.sharpen = sharpen;
        let scaled = f.draw();
        let expect = ((W as f32 * scale).round() as u32, (H as f32 * scale).round() as u32);
        assert_eq!(f.renderer.scene_size(), Some(expect), "scale {scale}");
        assert!(f.renderer.scales_output());
        let d = difference(&full, &scaled);
        assert!(
            d < 14.0,
            "scale {scale}, sharpen {sharpen}: the same view, not shrunk or shifted ({d})"
        );
        let lit = lit_corners(&scaled);
        assert!(lit[2] > 0 && lit[3] > 0, "scale {scale}: {lit:?}");
    }
    // Antialiasing runs at the scene's size, then scales.
    for mode in [AntiAliasing::Fxaa, AntiAliasing::Smaa] {
        f.ctx.post.render_scale = 0.5;
        f.ctx.post.anti_aliasing = mode;
        let aa = f.draw();
        assert_eq!(f.renderer.scene_size(), Some((W / 2, H / 2)));
        assert!(difference(&full, &aa) < 14.0, "{mode:?}");
    }
    // And back to full size, with the extra target freed.
    f.ctx.post.render_scale = 1.0;
    f.ctx.post.anti_aliasing = AntiAliasing::None;
    let back = f.draw();
    assert_eq!(back, full);
    assert_eq!(f.renderer.scene_size(), Some((W, H)));
    assert!(!f.renderer.scales_output());
}

#[test]
fn shadow_maps_are_made_again_at_the_new_resolution_and_count() {
    let Some(mut f) = Fixture::new() else { return };
    f.draw();
    // The default 4096² atlas, or the largest texture the device allows.
    let atlas = Some(4096.min(f.device.limits().max_texture_dimension_2d));
    assert_eq!(f.renderer.shadow_map(), (2048, 4), "the engine's defaults");
    assert_eq!(f.renderer.light_shadow_map(), atlas, "one 4096² atlas");

    f.ctx.lighting.shadow_resolution = 512;
    f.draw();
    assert_eq!(f.renderer.shadow_map(), (512, 4));
    assert_eq!(
        f.renderer.light_shadow_map(),
        atlas,
        "the light shadow map is left alone"
    );

    f.ctx.lighting.shadow_cascades = 2;
    f.draw();
    assert_eq!(f.renderer.shadow_map(), (512, 2));

    // Fewer or smaller views share the same atlas; a new atlas size makes it again.
    f.ctx.lighting.light_shadow_resolution = 256;
    f.ctx.lighting.light_shadow_views = 4;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), atlas);
    f.ctx.lighting.light_shadow_atlas = 2048;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), Some(2048));
    assert_eq!(f.renderer.shadow_map(), (512, 2));

    // Point and spot light shadows off: the map is freed.
    f.ctx.lighting.light_shadow_views = 0;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), None);
    // ... and made again when they come back.
    f.ctx.lighting.light_shadow_views = 1;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), Some(2048));
    // Not a power of two: rounded down.
    f.ctx.lighting.light_shadow_atlas = 3000;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), Some(2048));
    f.ctx.lighting.light_shadow_atlas = 1;
    f.draw();
    assert_eq!(f.renderer.light_shadow_map(), Some(1024));

    // A resolution beyond the allowed range is clamped.
    f.ctx.lighting.shadow_resolution = 1 << 20;
    f.draw();
    assert_eq!(
        f.renderer.shadow_map().0,
        4096.min(f.device.limits().max_texture_dimension_2d)
    );
    f.ctx.lighting.shadow_resolution = 1;
    f.draw();
    assert_eq!(f.renderer.shadow_map().0, 256);

    // All shadows off: stand-ins of 1x1.
    f.ctx.lighting.shadows = false;
    f.draw();
    assert_eq!(f.renderer.shadow_map(), (1, 1));
    assert_eq!(f.renderer.light_shadow_map(), None);
    f.ctx.lighting.shadows = true;
    f.draw();
    assert_eq!(f.renderer.shadow_map(), (256, 2));
}

#[test]
fn a_bigger_shadow_map_changes_the_picture_less_than_it_changes_the_map() {
    // Presets differ in shadow detail but the lit ground and the box's
    // shadow stay: the picture is about the same.
    let Some(mut f) = Fixture::new() else { return };
    f.ctx.gfx.set_quality(
        &mut f.ctx.post,
        &mut f.ctx.lighting,
        GraphicsQuality {
            anti_aliasing: AntiAliasing::None,
            ambient_occlusion: false,
            ..GraphicsQuality::medium()
        },
    );
    let medium = f.draw();
    f.ctx.lighting.shadow_resolution = 4096;
    let sharp = f.draw();
    assert!(difference(&medium, &sharp) < 6.0, "{}", difference(&medium, &sharp));
}

#[test]
fn a_kept_far_cascade_still_shadows_the_frames_it_is_not_drawn_in() {
    let Some(mut f) = Fixture::new() else { return };
    // Two cascades, and the box far enough away to be in the far one.
    f.ctx.lighting.shadow_cascades = 2;
    f.ctx.lighting.light_shadow_views = 0;
    f.ctx.lighting.sun_intensity = 1.0;
    f.ctx.lighting.sun_direction = vec3(-1.0, -0.6, 0.0); // a long shadow
    f.ctx.lighting.ambient_intensity = 0.1;
    f.ctx.post.exposure = -1.0;
    f.ctx.world.query::<&mut Transform3D>().for_each(|t| {
        if t.position.y > 0.0 {
            t.scale = Vec3::splat(4.0); // the box: big enough to see
        }
    });
    f.ctx.camera3d.position = vec3(0.0, 24.0, 30.0);
    f.ctx.camera3d.look_at(Vec3::ZERO);
    f.ctx.camera3d.fov_y = 20f32.to_radians(); // zoomed in on the box
    f.ctx.post.anti_aliasing = AntiAliasing::None;
    f.ctx.post.ambient_occlusion.enabled = false;
    f.ctx.lighting.shadow_refresh = 1;
    let every_frame = f.draw();
    f.ctx.lighting.shadows = false;
    let unshadowed = f.draw();
    let shadow = difference(&every_frame, &unshadowed);
    assert!(shadow > 0.5, "the box casts a visible shadow: {shadow}");

    f.ctx.lighting.shadows = true;
    f.ctx.lighting.shadow_refresh = 2;
    let mut kept = 0;
    for _ in 0..4 {
        let picture = f.draw();
        if f.frame.redrawn & 0b10 == 0 {
            kept += 1;
        }
        let off = difference(&picture, &every_frame);
        assert!(off < shadow * 0.25, "{off} against a shadow of {shadow}");
    }
    assert!(kept >= 1, "the far cascade was kept on some frames");
}
