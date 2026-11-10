//! Headless GPU regression tests for contact AO and post AA.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

const SIZE: u32 = 64;

#[test]
fn gtao_darkens_contact_more_than_open_ground_and_aa_smooths_edges() {
    render(false, false);
}
#[test]
fn downlevel_devices_skip_ao_and_keep_post_aa() {
    render(true, false);
}
#[test]
#[ignore = "offscreen 1080p GPU timestamps; run in release with --ignored --nocapture"]
fn ao_gpu_timings_1080p() {
    render(false, true);
}
fn render(downlevel: bool, benchmark: bool) {
    let (width, height) = if benchmark { (1920, 1080) } else { (SIZE, SIZE) };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
        crate::render::render_test_support::no_gpu("no GPU adapter");
        return;
    };
    eprintln!("AO adapter: {:?}", adapter.get_info());
    let mut descriptor = wgpu::DeviceDescriptor::default();
    if benchmark {
        descriptor.required_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    }
    if downlevel {
        descriptor.required_limits.max_compute_workgroups_per_dimension = 0;
        descriptor.required_limits.max_storage_textures_per_shader_stage = 0;
    }
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&crate::render::render_test_support::forced(&descriptor)))
    else {
        crate::render::render_test_support::no_gpu("no GPU device");
        return;
    };
    if !downlevel && !crate::render::render_test_support::needs(&device, crate::GpuFeature::AmbientOcclusion) {
        return;
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

    // A white ground, a box 3 up, and a spot light 6 up pointing down.
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(width as f32, height as f32));
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
    ctx.camera3d.position = vec3(0.0, 6.0, 6.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    ctx.lighting.shadows = false;
    ctx.lighting.sun_intensity = 0.0;
    ctx.sky.atmosphere = false;
    ctx.post.bloom = 0.0;

    let mut renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, true);
    renderer.create_material(&device, &GpuMaterialDesc::new(&matte), &textures);
    renderer.create_mesh(&device, &ground, &[(0..ground.indices.len() as u32, material)], true);
    renderer.create_mesh(&device, &cube, &[(0..cube.indices.len() as u32, material)], true);

    if benchmark {
        for z in -25..25 {
            for x in -25..25 {
                ctx.world.spawn((
                    Transform3D {
                        position: vec3(x as f32 * 0.35, 0.15, z as f32 * 0.35),
                        scale: Vec3::splat(0.3),
                        ..Default::default()
                    },
                    Model::new(cube_mesh),
                ));
            }
        }
    }
    let mut timer = super::timer::GpuTimer::new(&device, &queue);
    let mut draw = |ctx: &mut Ctx, clear: wgpu::Color| -> Vec<u8> {
        let mut frame = Frame3D::default();
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
        renderer.prepare(
            &device,
            &queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (width, height),
            },
        );
        if let Some(timer) = &mut timer {
            timer.collect(&device);
            timer.begin_frame();
        }
        let buffers = renderer.draw(&device, &target, clear, &textures, timer.as_ref());
        queue.submit(buffers);
        if let Some(timer) = &mut timer {
            let mut encoder = device.create_command_encoder(&Default::default());
            timer.resolve(&mut encoder);
            queue.submit([encoder.finish()]);
            timer.submitted();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            timer.collect(&device);
        }
        if benchmark {
            if let Some(timer) = &timer {
                eprintln!(
                    "AO half={} temporal={} slices={} steps={} {:.4} ms",
                    ctx.post.ao_half_resolution,
                    ctx.post.ao_temporal,
                    ctx.post.ambient_occlusion.slices,
                    ctx.post.ambient_occlusion.steps,
                    timer
                        .last_frame
                        .iter()
                        .filter(|p| p.name.starts_with("GTAO"))
                        .map(|p| p.ms)
                        .sum::<f32>()
                );
            }
            return Vec::new();
        }
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(SIZE * SIZE * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: None,
                },
            },
            output.size(),
        );
        queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        readback.get_mapped_range(..).unwrap().to_vec()
    };
    if benchmark {
        for (label, half, temporal, slices, steps) in [
            ("full 4x8", false, false, 4, 8),
            ("half 4x8", true, false, 4, 8),
            ("half temporal 2x4", true, true, 2, 4),
        ] {
            ctx.post.ambient_occlusion.enabled = true;
            ctx.post.ambient_occlusion.slices = slices;
            ctx.post.ambient_occlusion.steps = steps;
            ctx.post.ao_half_resolution = half;
            ctx.post.ao_temporal = temporal;
            for _ in 0..80 {
                draw(&mut ctx, wgpu::Color::BLACK);
            }
            eprintln!("mode completed: {label}");
        }
        return;
    }
    // Where a ground point lands on screen, and how bright it is there.
    let view_proj = ctx.camera3d.view_proj();
    let brightness = |pixels: &[u8], p: Vec3| -> u32 {
        let ndc = view_proj.project_point3(p);
        let x = ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32;
        let y = ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32;
        let i = ((y * SIZE + x) * 4) as usize;
        u32::from(pixels[i]) + u32::from(pixels[i + 1]) + u32::from(pixels[i + 2])
    };

    let (under, open) = (vec3(0.0, 0.0, 0.7), vec3(0.0, 0.0, 2.5));
    let baseline = draw(&mut ctx, wgpu::Color::BLACK);
    ctx.post.ambient_occlusion = AmbientOcclusion {
        enabled: true,
        radius: 2.0,
        intensity: 2.0,
        slices: 6,
        steps: 12,
    };
    let occluded = draw(&mut ctx, wgpu::Color::BLACK);
    let contact_loss = brightness(&baseline, under) as i32 - brightness(&occluded, under) as i32;
    let open_loss = brightness(&baseline, open) as i32 - brightness(&occluded, open) as i32;
    eprintln!("AO contact loss {contact_loss}, open loss {open_loss} (summed RGB bytes)");
    if downlevel {
        assert_eq!(baseline, occluded, "unsupported AO preserves lighting");
    } else {
        assert!(contact_loss > 15, "contact AO darkens: {contact_loss}");
        assert!(
            contact_loss > open_loss + 10,
            "contact {contact_loss}, open {open_loss}"
        );
    }
    if !downlevel {
        ctx.post.ao_half_resolution = true;
        let half = draw(&mut ctx, wgpu::Color::BLACK);
        let half_loss = brightness(&baseline, under) as i32 - brightness(&half, under) as i32;
        eprintln!("half loss {half_loss}, full loss {contact_loss}");
        assert!(half_loss > 15 && (half_loss - contact_loss).abs() < 30);
        ctx.post.ao_temporal = true;
        for _ in 0..8 {
            let _ = draw(&mut ctx, wgpu::Color::BLACK);
        }
        let temporal = draw(&mut ctx, wgpu::Color::BLACK);
        assert!((brightness(&temporal, under) as i32 - brightness(&half, under) as i32).abs() < 40);
        // A camera cut exposes the sky: history cannot darken sky pixels.
        ctx.camera3d.look_at(vec3(0.0, 30.0, 0.0));
        let sky_ao = draw(&mut ctx, wgpu::Color::BLACK);
        ctx.post.ambient_occlusion.enabled = false;
        let sky_clear = draw(&mut ctx, wgpu::Color::BLACK);
        assert_eq!(sky_ao, sky_clear, "sky has no AO halo after a camera cut");
        ctx.camera3d.look_at(Vec3::ZERO);
        ctx.post.ao_half_resolution = false;
        ctx.post.ao_temporal = false;
    }
    ctx.post.ambient_occlusion.enabled = false;
    ctx.lighting.ambient_intensity = 0.0;
    ctx.lighting.sun_intensity = 3.0;
    let direct = draw(&mut ctx, wgpu::Color::BLACK);
    ctx.post.ambient_occlusion.enabled = true;
    let direct_ao = draw(&mut ctx, wgpu::Color::BLACK);
    assert_eq!(direct, direct_ao, "AO leaves direct sunlight unchanged");
    ctx.post.ambient_occlusion.enabled = false;
    ctx.lighting.ambient_intensity = 1.0;
    ctx.lighting.sun_intensity = 0.0;
    // A white emissive, rotated thin box against black supplies a diagonal edge.
    ctx.world.clear();
    ctx.world.spawn((
        Transform3D {
            rotation: Quat::from_rotation_z(0.37),
            scale: vec3(3.0, 0.05, 0.05),
            ..Default::default()
        },
        Model::new(cube_mesh),
    ));
    ctx.camera3d.position = vec3(0.0, 0.0, 5.0);
    ctx.camera3d.look_at(Vec3::ZERO);
    let count = |px: &[u8]| px.as_chunks::<4>().0.iter().filter(|p| p[0] > 10 && p[0] < 200).count();
    if !downlevel {
        let silhouette = draw(&mut ctx, wgpu::Color::WHITE);
        ctx.post.ambient_occlusion.enabled = true;
        ctx.post.ao_half_resolution = true;
        ctx.post.ao_temporal = true;
        let silhouette_ao = draw(&mut ctx, wgpu::Color::WHITE);
        for (clear, ao) in silhouette
            .as_chunks::<4>()
            .0
            .iter()
            .zip(silhouette_ao.as_chunks::<4>().0)
        {
            if clear[..3] == [255; 3] {
                assert_eq!(&ao[..3], &[255; 3], "sky silhouette halo");
            }
        }
        ctx.post.ambient_occlusion.enabled = false;
    }
    let no_aa = draw(&mut ctx, wgpu::Color::BLACK);
    for mode in [AntiAliasing::Fxaa, AntiAliasing::Smaa] {
        ctx.post.anti_aliasing = mode;
        let aa = draw(&mut ctx, wgpu::Color::BLACK);
        eprintln!("{mode:?}: {} intermediate pixels vs {}", count(&aa), count(&no_aa));
        assert!(
            count(&aa) > count(&no_aa),
            "{mode:?}: {} intermediate pixels vs {}",
            count(&aa),
            count(&no_aa)
        );
    }
}
