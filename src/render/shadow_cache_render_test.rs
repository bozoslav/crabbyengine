//! Shadow views kept from an earlier frame give the same image as drawing
//! them again. Two renderers draw the same scene offscreen, one keeping
//! unchanged shadow views and one drawing them all every frame, while
//! casters and lights move, appear and go, a material and the shadow
//! quality change and the camera moves; every frame must match exactly.
//! Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

const SIZE: u32 = 128;
/// Sun cascades and light views in the scene: 2 cascades, a spot light's
/// view and a point light's six.
const CASCADES: u32 = 2;
const LIGHT_VIEWS: u32 = 7;
const ALL: u32 = ((1 << CASCADES) - 1) | LIGHTS;
const LIGHTS: u32 = ((1 << LIGHT_VIEWS) - 1) << 4;

struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    textures: [GpuTexture; 1],
    layout: wgpu::BindGroupLayout,
    white: wgpu::Texture,
    output: wgpu::Texture,
    target: wgpu::TextureView,
}

impl Rig {
    fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&crate::render::render_test_support::forced(
            &wgpu::DeviceDescriptor::default(),
        )))
        .ok()?;
        eprintln!("adapter: {:?}", adapter.get_info());
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
        let textures = [GpuTexture {
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
        Some(Self {
            device,
            queue,
            textures,
            layout,
            white,
            output,
            target,
        })
    }

    fn renderer(&self) -> Renderer3D {
        Renderer3D::new(
            &self.device,
            &self.layout,
            self.output.format(),
            0,
            &self.white,
            4,
            true,
        )
    }

    /// Collects, prepares and draws one frame; returns its pixels.
    fn draw(&self, renderer: &mut Renderer3D, frame: &mut Frame3D, ctx: &mut Ctx) -> Vec<u8> {
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
                sun: Vec3::splat(2.0),
                sky: Vec3::splat(0.1),
                ground: Vec3::ZERO,
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
            occlusion: false,
        };
        renderer.prepare(
            &self.device,
            &self.queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (SIZE, SIZE),
            },
        );
        let buffers = renderer.draw(&self.device, &self.target, wgpu::Color::BLACK, &self.textures, None);
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

#[test]
fn kept_shadow_views_draw_the_same_image_as_redrawing_them() {
    let Some(rig) = Rig::new() else {
        crate::render::render_test_support::no_gpu("no GPU adapter or device");
        return;
    };
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(SIZE as f32, SIZE as f32));
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
    let mover = ctx
        .world
        .spawn((Transform3D::at(vec3(0.0, 1.0, 0.0)), Model::new(cube_mesh)));
    ctx.world
        .spawn((Transform3D::at(vec3(3.0, 1.0, 2.0)), Model::new(cube_mesh)));
    let spot = ctx.world.spawn((
        Transform3D::at(vec3(0.0, 6.0, 0.0)).looking_at(vec3(0.0, 0.0, 0.001)),
        SpotLight3D {
            intensity: 300.0,
            range: 20.0,
            inner_angle: 0.5,
            outer_angle: 0.8,
            shadows: true,
            ..Default::default()
        },
    ));
    ctx.world.spawn((
        Transform3D::at(vec3(-4.0, 3.0, -3.0)),
        PointLight3D {
            intensity: 200.0,
            range: 15.0,
            shadows: true,
            ..Default::default()
        },
    ));
    ctx.camera3d.position = vec3(0.0, 9.0, 9.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.lighting.shadows = true;
    ctx.lighting.shadow_cascades = CASCADES;
    ctx.lighting.shadow_resolution = 512;
    ctx.lighting.shadow_distance = 30.0;
    // Every cascade is drawn each frame unless the renderer keeps it.
    ctx.lighting.shadow_refresh = 1;
    ctx.lighting.light_shadow_atlas = 1024;
    ctx.sky.atmosphere = false;

    // `cached` keeps unchanged views; `reference` draws them all.
    let mut cached = rig.renderer();
    let mut reference = rig.renderer();
    for r in [&mut cached, &mut reference] {
        r.create_material(&rig.device, &GpuMaterialDesc::new(&matte), &rig.textures);
        r.create_mesh(
            &rig.device,
            &ground,
            &[(0..ground.indices.len() as u32, material)],
            true,
        );
        r.create_mesh(&rig.device, &cube, &[(0..cube.indices.len() as u32, material)], true);
    }
    let mut renderers = [cached, reference];
    let mut frames = [Frame3D::default(), Frame3D::default()];
    let mut last = Vec::new();
    // Draws a frame with both; they must match. Returns the views kept.
    let mut step = |renderers: &mut [Renderer3D; 2], ctx: &mut Ctx, what: &str, changes: bool| -> u32 {
        let [cached, reference] = renderers;
        let [cached_frame, reference_frame] = &mut frames;
        let image = rig.draw(cached, cached_frame, ctx);
        reference.forget_shadows();
        let expected = rig.draw(reference, reference_frame, ctx);
        assert_eq!(reference.shadow_views_kept(), 0, "{what}: the reference keeps nothing");
        let differing = image.chunks(4).zip(expected.chunks(4)).filter(|(a, b)| a != b).count();
        assert_eq!(
            differing, 0,
            "{what}: {differing} pixels differ from redrawing every view"
        );
        if changes {
            assert_ne!(image, last, "{what}: the change shows (the test can see stale shadows)");
        }
        last = image;
        cached.shadow_views_kept()
    };

    assert_eq!(step(&mut renderers, &mut ctx, "first frame", false), 0);
    let [(_, _), (casters, triangles)] = renderers[0].counts();
    assert!(casters > 0, "the first frame draws the casters");
    assert_eq!(step(&mut renderers, &mut ctx, "nothing changed", false), ALL);
    let [(_, _), kept_work] = renderers[0].counts();
    eprintln!("shadow work: first frame {casters} casters, {triangles} triangles; unchanged frame {kept_work:?}");
    assert_eq!(kept_work, (0, 0), "an unchanged frame draws no shadow casters");

    // A caster moves: the views that see it are drawn again, others kept.
    ctx.world.get_mut::<Transform3D>(mover).unwrap().position.x += 0.6;
    let kept = step(&mut renderers, &mut ctx, "caster moved", true);
    assert_ne!(kept & 0b11, 0b11, "the cascade that sees the caster is drawn: {kept:b}");
    assert!(kept & LIGHTS != 0, "light views that don't see it are kept: {kept:b}");
    assert_eq!(step(&mut renderers, &mut ctx, "settled", false), ALL);

    // The spot light moves: only its view is drawn again (its tile cleared
    // alone, the point light's tiles kept).
    ctx.world.get_mut::<Transform3D>(spot).unwrap().position.x += 0.5;
    let kept = step(&mut renderers, &mut ctx, "spot light moved", true);
    assert_eq!(kept & 0b11, 0b11, "the sun's cascades are kept: {kept:b}");
    assert_eq!(
        (ALL & !kept).count_ones(),
        1,
        "only the spot light's view is drawn: {kept:b}"
    );

    // A caster appears, then goes.
    let extra = ctx
        .world
        .spawn((Transform3D::at(vec3(-1.5, 1.0, 1.5)), Model::new(cube_mesh)));
    assert_ne!(step(&mut renderers, &mut ctx, "caster spawned", true), ALL);
    ctx.world.despawn(extra);
    assert_ne!(step(&mut renderers, &mut ctx, "caster despawned", true), ALL);
    assert_eq!(step(&mut renderers, &mut ctx, "settled again", false), ALL);

    // A material change draws every view again.
    let darker = MaterialDesc {
        color: Color::rgb(0.8, 0.8, 0.8),
        ..matte
    };
    for r in &mut renderers {
        r.update_material(
            &rig.device,
            &rig.queue,
            material.id,
            &GpuMaterialDesc::new(&darker),
            &rig.textures,
        );
    }
    assert_eq!(step(&mut renderers, &mut ctx, "material changed", true), 0);

    // New shadow quality: new sun shadow maps, every cascade drawn.
    ctx.lighting.shadow_resolution = 1024;
    let kept = step(&mut renderers, &mut ctx, "shadow resolution changed", false);
    assert_eq!(kept & 0b11, 0, "new maps are drawn: {kept:b}");

    // The camera moves: the cascades follow it; the lights' views stay.
    ctx.camera3d.position = vec3(1.0, 8.0, 10.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    let kept = step(&mut renderers, &mut ctx, "camera moved", true);
    assert_eq!(kept & 0b11, 0, "cascades follow the camera: {kept:b}");
    assert_eq!(kept & LIGHTS, LIGHTS, "light views are kept: {kept:b}");
    assert_eq!(step(&mut renderers, &mut ctx, "still", false), ALL);
}

/// Views drawn with a shader that moves vertices itself (it may read the
/// time or the camera) are drawn every frame; the others are still kept.
#[test]
fn views_with_shader_moved_vertices_are_always_drawn() {
    let Some(rig) = Rig::new() else {
        crate::render::render_test_support::no_gpu("no GPU adapter or device");
        return;
    };
    const WAVE: &str = "fn vertex(v: Vertex) -> Vertex { return v; }";
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(SIZE as f32, SIZE as f32));
    let matte = MaterialDesc {
        color: Color::WHITE,
        roughness: 1.0,
        ..Default::default()
    };
    let waving = MaterialDesc {
        shader: Some(ctx.gfx.create_shader("wave", WAVE)),
        ..matte
    };
    let material = ctx.gfx.create_material(matte);
    let wave_material = ctx.gfx.create_material(waving);
    let ground = MeshData::plane(20.0);
    let cube = MeshData::cube(1.0);
    let ground_mesh = ctx.gfx.create_mesh(&ground, material);
    let cube_mesh = ctx.gfx.create_mesh(&cube, wave_material);
    ctx.world.spawn((Transform3D::default(), Model::new(ground_mesh)));
    ctx.world
        .spawn((Transform3D::at(vec3(0.0, 1.0, 0.0)), Model::new(cube_mesh)));
    // A spot light away from the cube: its view sees only the ground.
    ctx.world.spawn((
        Transform3D::at(vec3(6.0, 4.0, 6.0)).looking_at(vec3(6.0, 0.0, 6.001)),
        SpotLight3D {
            intensity: 300.0,
            range: 6.0,
            inner_angle: 0.3,
            outer_angle: 0.4,
            shadows: true,
            ..Default::default()
        },
    ));
    ctx.camera3d.position = vec3(0.0, 9.0, 9.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.lighting.shadows = true;
    ctx.lighting.shadow_cascades = 1;
    ctx.lighting.shadow_resolution = 512;
    ctx.lighting.shadow_refresh = 1;
    ctx.lighting.light_shadow_atlas = 1024;
    ctx.sky.atmosphere = false;
    let mut renderer = rig.renderer();
    renderer.create_shader(&rig.device, "wave", WAVE).unwrap();
    renderer.create_material(&rig.device, &GpuMaterialDesc::new(&matte), &rig.textures);
    renderer.create_material(&rig.device, &GpuMaterialDesc::new(&waving), &rig.textures);
    renderer.create_mesh(
        &rig.device,
        &ground,
        &[(0..ground.indices.len() as u32, material)],
        true,
    );
    renderer.create_mesh(
        &rig.device,
        &cube,
        &[(0..cube.indices.len() as u32, wave_material)],
        true,
    );
    let mut frame = Frame3D::default();
    for _ in 0..3 {
        rig.draw(&mut renderer, &mut frame, &mut ctx);
    }
    // The cascade sees the cube; the spot light's view (view 4) doesn't.
    assert_eq!(renderer.shadow_views_kept(), 1 << 4);
}

/// A model's material override changing (here to glass, which casts no
/// shadow) redraws its shadow views, though the override's per-frame id
/// and the model's instance stay the same.
#[test]
fn a_changed_material_override_redraws_its_shadows() {
    let Some(rig) = Rig::new() else {
        crate::render::render_test_support::no_gpu("no GPU adapter or device");
        return;
    };
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(SIZE as f32, SIZE as f32));
    let matte = MaterialDesc {
        color: Color::WHITE,
        roughness: 1.0,
        ..Default::default()
    };
    let glass = MaterialDesc {
        blend: Blend::Transparent,
        color: Color::rgba(1.0, 1.0, 1.0, 0.3),
        ..matte
    };
    let material = ctx.gfx.create_material(matte);
    let solid = ctx.gfx.create_material(matte);
    let see_through = ctx.gfx.create_material(glass);
    let ground = MeshData::plane(20.0);
    let cube = MeshData::cube(1.0);
    let ground_mesh = ctx.gfx.create_mesh(&ground, material);
    let cube_mesh = ctx.gfx.create_mesh(&cube, material);
    ctx.world.spawn((Transform3D::default(), Model::new(ground_mesh)));
    let rock = ctx.world.spawn((
        Transform3D::at(vec3(0.0, 1.0, 0.0)),
        Model::new(cube_mesh),
        ModelMaterial(solid),
    ));
    ctx.camera3d.position = vec3(0.0, 9.0, 9.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.lighting.shadows = true;
    ctx.lighting.shadow_cascades = 1;
    ctx.lighting.shadow_resolution = 512;
    ctx.lighting.shadow_refresh = 1;
    ctx.sky.atmosphere = false;
    let mut renderers = [rig.renderer(), rig.renderer()];
    for r in &mut renderers {
        for desc in [&matte, &matte, &glass] {
            r.create_material(&rig.device, &GpuMaterialDesc::new(desc), &rig.textures);
        }
        r.create_mesh(
            &rig.device,
            &ground,
            &[(0..ground.indices.len() as u32, material)],
            true,
        );
        r.create_mesh(&rig.device, &cube, &[(0..cube.indices.len() as u32, material)], true);
    }
    let mut frames = [Frame3D::default(), Frame3D::default()];
    let mut draw = |ctx: &mut Ctx| {
        let [cached, reference] = &mut renderers;
        let [cached_frame, reference_frame] = &mut frames;
        let image = rig.draw(cached, cached_frame, ctx);
        reference.forget_shadows();
        let expected = rig.draw(reference, reference_frame, ctx);
        let differing = image.chunks(4).zip(expected.chunks(4)).filter(|(a, b)| a != b).count();
        assert_eq!(differing, 0, "{differing} pixels differ from redrawing every view");
        (image, cached.shadow_views_kept())
    };
    draw(&mut ctx);
    let (solid_image, kept) = draw(&mut ctx);
    assert_eq!(kept, 1, "unchanged: the cascade is kept");
    ctx.world.get_mut::<ModelMaterial>(rock).unwrap().0 = see_through;
    let (glass_image, kept) = draw(&mut ctx);
    assert_eq!(kept, 0, "the override changed: the cascade is drawn");
    assert_ne!(solid_image, glass_image);
}

#[test]
fn instance_levels_refresh_cached_sun_cascades_across_shadow_bands() {
    let Some(rig) = Rig::new() else {
        crate::render::render_test_support::no_gpu("no GPU adapter or device");
        return;
    };
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::splat(SIZE as f32));
    ctx.camera3d.position = vec3(0.0, 6.0, 6.0);
    ctx.camera3d.fov_y = 90.0_f32.to_radians();
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.sky.atmosphere = false;
    ctx.lighting.shadow_cascades = 4;
    ctx.lighting.shadow_resolution = 512;
    ctx.lighting.shadow_refresh = 32;
    ctx.lighting.sun_direction = -Vec3::Y;
    let desc = MaterialDesc {
        color: Color::WHITE,
        ..Default::default()
    };
    let material = ctx.gfx.create_material(desc);
    let ground = MeshData::plane(120.0);
    let cube = MeshData::cube(2.0);
    let floor = ctx.gfx.create_mesh(&ground, material);
    let proxy = ctx.gfx.create_mesh(&cube, material);
    ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    let b = ctx.gfx.create_instance_batch_levels(
        &[
            InstanceLevel {
                mesh: proxy,
                min_distance: 0.0,
                max_distance: 7.0,
                camera: false,
                shadows: true,
            },
            InstanceLevel {
                mesh: proxy,
                min_distance: 7.0,
                max_distance: 20.0,
                camera: false,
                shadows: false,
            },
        ],
        &[Mat4::from_translation(vec3(0.0, 3.0, 0.0))],
        &[],
        Default::default(),
    );
    // This caster lies in a deferred far cascade and crosses its own band
    // during the same half-unit camera move as the nearby caster.
    let far_batch = ctx.gfx.create_instance_batch_levels(
        &[InstanceLevel {
            mesh: proxy,
            min_distance: 0.0,
            max_distance: 46.25,
            camera: false,
            shadows: true,
        }],
        &[Mat4::from_scale_rotation_translation(
            Vec3::splat(4.0),
            Quat::IDENTITY,
            vec3(0.0, 3.0, -40.0),
        )],
        &[],
        Default::default(),
    );
    let mut renderers = [rig.renderer(), rig.renderer()];
    for r in &mut renderers {
        r.create_material(&rig.device, &GpuMaterialDesc::new(&desc), &rig.textures);
        r.create_mesh(
            &rig.device,
            &ground,
            &[(0..ground.indices.len() as u32, material)],
            true,
        );
        r.create_mesh(&rig.device, &cube, &[(0..cube.indices.len() as u32, material)], true);
        for batch in [b, far_batch] {
            r.set_instance_batch(
                &rig.device,
                batch.id(),
                ctx.gfx.assets3d.instance_batches[batch.id() as usize].as_ref(),
            );
        }
    }
    let mut frames = [Frame3D::default(), Frame3D::default()];
    let mut compare = |ctx: &mut Ctx, equal: bool| {
        let [cached, reference] = &mut renderers;
        let [a, b] = &mut frames;
        let image = rig.draw(cached, a, ctx);
        // Bypass both cache layers in the reference, including deferred turns.
        *b = Frame3D::default();
        reference.forget_shadows();
        let expected = rig.draw(reference, b, ctx);
        assert_eq!(reference.shadow_cascades_redrawn(), 4);
        let matches = image == expected;
        if equal {
            assert!(matches, "cached shadow bands differ from a fresh draw");
        }
        (image, a.redrawn, cached.shadow_cascades_redrawn(), matches)
    };
    compare(&mut ctx, true);
    let (with_proxy, collected, redrawn, _) = compare(&mut ctx, true);
    assert_eq!(
        collected & 0b1100,
        0,
        "stationary banded casters retain deferred far cascades"
    );
    assert_eq!(
        redrawn, 0,
        "stationary content redraws no cascades in either cache layer"
    );
    ctx.gfx.set_mesh_shadows(proxy, false);
    let (without_proxy, _, redrawn, _) = compare(&mut ctx, true);
    assert_eq!(redrawn, 4, "mesh shadow flags invalidate cached cascades");
    assert_ne!(
        with_proxy, without_proxy,
        "shadow-only geometry must cast a visible sun shadow"
    );
    ctx.gfx.set_mesh_shadows(proxy, true);
    compare(&mut ctx, true);
    ctx.camera3d.position.z = 6.5;
    ctx.camera3d.look_at(Vec3::ZERO);
    let mut refreshed = 0;
    let mut stale_seen = false;
    let refresh = ctx.lighting.shadow_refresh;
    for turn in 0..refresh {
        let (_, collected, _, matches) = compare(&mut ctx, turn + 1 == refresh);
        assert!(
            (collected & 0b1100).count_ones() <= 1,
            "band crossings preserve far-cascade turns"
        );
        refreshed |= collected;
        stale_seen |= !matches;
    }
    assert!(
        stale_seen,
        "the far caster must exercise deferred shadow content before its turn"
    );
    assert_eq!(refreshed, 0b1111, "every cascade converges within one refresh cycle");
}
