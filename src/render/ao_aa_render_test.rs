//! Headless GPU regression tests for contact AO and post AA.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

const SIZE: u32 = 64;

#[test]
fn gtao_darkens_contact_more_than_open_ground_and_aa_smooths_edges() {
    render(false);
}
#[test]
fn downlevel_devices_skip_ao_and_keep_post_aa() {
    render(true);
}
fn render(downlevel: bool) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let mut descriptor = wgpu::DeviceDescriptor::default();
    if downlevel {
        descriptor.required_limits.max_compute_workgroups_per_dimension = 0;
        descriptor.required_limits.max_storage_textures_per_shader_stage = 0;
    }
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&descriptor)) else {
        eprintln!("no GPU device: skipped");
        return;
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

    // A white ground, a box 3 up, and a spot light 6 up pointing down.
    let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(SIZE as f32, SIZE as f32));
    let matte = MaterialDesc {
        color: Color::WHITE,
        roughness: 1.0,
        ..Default::default()
    };
    let material = ctx.create_material(matte);
    let ground = MeshData::plane(20.0);
    let cube = MeshData::cube(1.0);
    let ground_mesh = ctx.create_mesh(&ground, material);
    let cube_mesh = ctx.create_mesh(&cube, material);
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

    let mut draw = |ctx: &mut Ctx| -> Vec<u8> {
        let mut frame = Frame3D::default();
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
            light: SkyLight {
                sun: Vec3::splat(ctx.lighting.sun_intensity),
                sky: Vec3::ONE,
                ground: Vec3::ONE,
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
        };
        renderer.prepare(
            &device,
            &queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (SIZE, SIZE),
            },
        );
        let buffers = renderer.draw(&device, &target, wgpu::Color::BLACK, &textures, None);
        queue.submit(buffers);
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
    let baseline = draw(&mut ctx);
    ctx.post.ambient_occlusion = AmbientOcclusion {
        enabled: true,
        radius: 2.0,
        intensity: 2.0,
        slices: 6,
        steps: 12,
    };
    let occluded = draw(&mut ctx);
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
    ctx.post.ambient_occlusion.enabled = false;
    ctx.lighting.ambient_intensity = 0.0;
    ctx.lighting.sun_intensity = 3.0;
    let direct = draw(&mut ctx);
    ctx.post.ambient_occlusion.enabled = true;
    let direct_ao = draw(&mut ctx);
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
    let no_aa = draw(&mut ctx);
    for mode in [AntiAliasing::Fxaa, AntiAliasing::Smaa] {
        ctx.post.anti_aliasing = mode;
        let aa = draw(&mut ctx);
        eprintln!("{mode:?}: {} intermediate pixels vs {}", count(&aa), count(&no_aa));
        assert!(
            count(&aa) > count(&no_aa),
            "{mode:?}: {} intermediate pixels vs {}",
            count(&aa),
            count(&no_aa)
        );
    }
}
