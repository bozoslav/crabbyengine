//! Draw order: passes whose result doesn't depend on the order draw their
//! runs sorted by the state they bind. On a scene of many meshes, materials
//! and shaders (cut-out, double-sided, transparent, shadowed), that draws
//! the same pixels with fewer pipeline and bind group changes, also with
//! per-model material overrides. Skips without a GPU.
use super::gpu::GpuTexture;
use super::gpu3d::StateChanges;
use super::gpu3d::*;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight};
use crate::*;

const SIZE: u32 = 256;

/// An offscreen renderer with a 1x1 white texture, and a `Ctx` to fill.
struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    output: wgpu::Texture,
    target: wgpu::TextureView,
    textures: Vec<GpuTexture>,
    renderer: Renderer3D,
    ctx: Ctx,
    frame: Frame3D,
    /// Uncaptured GPU errors and device loss, so a wrong frame names its cause.
    gpu_errors: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl Rig {
    /// `None` without a GPU.
    fn new(solid_prepass: bool) -> Option<Self> {
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
        let gpu_errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = gpu_errors.clone();
        device.on_uncaptured_error(std::sync::Arc::new(move |e: wgpu::Error| {
            sink.lock().unwrap().push(format!("GPU error: {e}"));
        }));
        let sink = gpu_errors.clone();
        device.set_device_lost_callback(move |reason, message| {
            sink.lock()
                .unwrap()
                .push(format!("device lost ({reason:?}): {message}"));
        });
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
        ctx.lighting.shadows = true;
        ctx.lighting.sun_direction = vec3(-0.4, -1.0, -0.3);
        Some(Self {
            device,
            queue,
            output,
            target,
            textures,
            renderer,
            ctx,
            frame: Frame3D::default(),
            gpu_errors,
        })
    }

    /// Panics naming `what` if the device reported an error or was lost.
    fn assert_healthy(&self, what: &str) {
        let errors = self.gpu_errors.lock().unwrap();
        assert!(errors.is_empty(), "{what}: the GPU failed: {errors:?}");
    }

    /// Asserts that two frames are identical; on a mismatch, says how many
    /// pixels differ and how many of each are black (a lost or failed
    /// frame), instead of printing every byte.
    fn assert_same_frame(&self, a: &[u8], b: &[u8], what: &str) {
        self.assert_healthy(what);
        if a == b {
            return;
        }
        let black = |p: &[u8]| p.as_chunks::<4>().0.iter().filter(|p| p[..3] == [0, 0, 0]).count();
        let differ = a
            .as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(p, q)| p != q)
            .count();
        panic!(
            "{what}: {differ} of {} pixels differ; black pixels: {} in the first frame, {} in the second",
            a.len() / 4,
            black(a),
            black(b)
        );
    }

    /// A material on the CPU and the GPU.
    fn material(&mut self, desc: MaterialDesc) -> Material {
        let material = self.ctx.gfx.create_material(desc);
        self.renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        material
    }

    /// A material shader on the CPU and the GPU.
    fn shader(&mut self, name: &str, source: &str) -> Shader {
        let shader = self.ctx.gfx.create_shader(name, source);
        self.renderer.create_shader(&self.device, name, source).unwrap();
        shader
    }

    /// A mesh of `data` with `material`, on the CPU and the GPU.
    fn mesh(&mut self, data: &MeshData, material: Material) -> Mesh {
        let mesh = self.ctx.gfx.create_mesh(data, material);
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], true);
        mesh
    }

    /// Draws the world and reads the pixels back (RGBA).
    fn draw(&mut self) -> Vec<u8> {
        self.render();
        self.read()
    }

    /// Collects, prepares and records a frame, and submits it. Returns the
    /// CPU time of the renderer's prepare (where runs are sorted) and of
    /// recording the passes.
    fn render(&mut self) -> [std::time::Duration; 2] {
        let ctx = &mut self.ctx;
        let frame = &mut self.frame;
        frame.lod = ctx.lod;
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
                sky: Vec3::splat(0.3),
                ground: Vec3::splat(0.1),
            },
            camera: ctx.camera3d.position,
            post: &ctx.post,
            time: (0.0, 1.0 / 60.0),
            occlusion: false,
        };
        let t = std::time::Instant::now();
        self.renderer.prepare(
            &self.device,
            &self.queue,
            &Prepare3D {
                scene: &scene,
                lut: None,
                size: (SIZE, SIZE),
            },
        );
        let prepared = t.elapsed();
        let t = std::time::Instant::now();
        let buffers = self
            .renderer
            .draw(&self.device, &self.target, wgpu::Color::BLACK, &self.textures, None);
        let recorded = t.elapsed();
        self.queue.submit(buffers);
        [prepared, recorded]
    }

    /// A frame once the scene has settled (cached shadow cascades, temporal
    /// effects), with the state changes it took.
    fn settled(&mut self, group_draws: bool) -> (Vec<u8>, StateChanges) {
        self.renderer.group_draws = group_draws;
        for _ in 0..3 {
            self.render();
        }
        self.renderer.take_state_changes();
        let pixels = self.draw();
        // A differing frame is not a skipped upload: the buffers hold what
        // their mirrors say.
        self.renderer.check_mirrors(&self.device, &self.queue);
        (pixels, self.renderer.take_state_changes())
    }

    fn read(&self) -> Vec<u8> {
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

/// Built-in and custom shaders, solid, cut-out, double-sided and
/// transparent materials, on `meshes` meshes whose ids go round the
/// materials so that neighbouring meshes rarely share one, each drawn a few
/// times over a lit, shadowed ground. Every material is solid at the camera
/// (cut-outs with alpha 1), so the pixels show any order mistake.
fn many_materials(rig: &mut Rig, meshes: usize) {
    let tints = ["0.8", "1.2", "0.6"];
    let mut shaders = vec![None];
    for (i, tint) in tints.iter().enumerate() {
        let source =
            format!("fn surface(s: Surface) -> Surface {{ var o = s; o.color = s.color * {tint}; return o; }}");
        shaders.push(Some(rig.shader(&format!("tint{i}"), &source)));
    }
    let mut materials = Vec::new();
    for i in 0..24usize {
        let hue = i as f32 / 24.0;
        materials.push(rig.material(MaterialDesc {
            color: Color::rgb(hue, 1.0 - hue, 0.5 + 0.5 * (hue * 6.0).sin()),
            roughness: 0.4 + 0.02 * i as f32,
            shader: shaders[i % shaders.len()],
            blend: if i % 5 == 3 { Blend::Cutout } else { Blend::Opaque },
            double_sided: i % 7 == 2,
            ..Default::default()
        }));
    }
    let glass = [
        rig.material(MaterialDesc {
            color: Color::rgba(0.2, 0.4, 1.0, 0.5),
            blend: Blend::Transparent,
            ..Default::default()
        }),
        rig.material(MaterialDesc {
            color: Color::rgba(1.0, 0.3, 0.2, 0.4),
            blend: Blend::Transparent,
            shader: shaders[1],
            ..Default::default()
        }),
    ];
    let ground_material = rig.material(MaterialDesc {
        roughness: 1.0,
        ..Default::default()
    });
    let ground = rig.mesh(&MeshData::plane(60.0), ground_material);
    rig.ctx.world.spawn((Transform3D::default(), Model::new(ground)));
    let side = (meshes as f32).sqrt().ceil() as usize;
    for i in 0..meshes {
        // Mesh i gets material 7i mod 24: neighbours differ in material
        // and mostly in shader too.
        let material = materials[(i * 7) % materials.len()];
        let size = 0.3 + 0.1 * (i % 4) as f32;
        let data = if i % 3 == 0 {
            MeshData::sphere(size * 0.6, 12)
        } else {
            MeshData::cube(size)
        };
        let mesh = rig.mesh(&data, material);
        let (x, z) = ((i % side) as f32, (i / side) as f32);
        let spacing = 40.0 / side as f32;
        for copy in 0..3 {
            let at = vec3(
                (x - side as f32 / 2.0) * spacing + copy as f32 * 0.25 * spacing,
                size * 0.5 + copy as f32 * 0.4,
                (z - side as f32 / 2.0) * spacing,
            );
            rig.ctx.world.spawn((Transform3D::at(at), Model::new(mesh)));
        }
    }
    // Overlapping transparent panes, whose blending depends on their order.
    for (k, &m) in glass.iter().enumerate() {
        let pane = rig.mesh(&MeshData::cube(1.0), m);
        for j in 0..4 {
            let at = vec3(-3.0 + j as f32 * 2.0 + k as f32 * 0.6, 2.0, 4.0 + k as f32 * 0.5);
            rig.ctx.world.spawn((
                Transform3D {
                    scale: vec3(3.0, 3.0, 0.2),
                    ..Transform3D::at(at)
                },
                Model::new(pane),
            ));
        }
    }
    rig.ctx.camera3d.position = vec3(0.0, 18.0, 26.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    rig.ctx.camera3d.far = 200.0;
    rig.ctx.lighting.shadow_distance = 80.0;
}

#[test]
fn sorted_runs_draw_the_same_pixels_with_fewer_state_changes() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        many_materials(&mut rig, 300);
        rig.ctx.lighting.shadow_refresh = 1;
        let (before, unsorted) = rig.settled(false);
        let (again, _) = rig.settled(false);
        rig.assert_same_frame(
            &before,
            &again,
            &format!("a settled frame repeats (prepass {solid_prepass})"),
        );
        let (after, sorted) = rig.settled(true);
        eprintln!("prepass {solid_prepass}: mesh order {unsorted:?}, sorted {sorted:?}");
        let lit = before
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 20 || p[1] > 20)
            .count();
        assert!(lit > (SIZE * SIZE / 2) as usize, "the scene fills the view ({lit} lit)");
        let differ = before.iter().zip(&after).filter(|(a, b)| a != b).count();
        rig.assert_healthy("sorted runs");
        assert_eq!(differ, 0, "the same pixels (prepass {solid_prepass})");
        assert_eq!(sorted.draws, unsorted.draws, "the same draws");
        assert!(
            sorted.pipelines * 2 < unsorted.pipelines,
            "fewer pipeline changes: {unsorted:?} -> {sorted:?}"
        );
        assert!(
            sorted.bind_groups < unsorted.bind_groups,
            "fewer bind group changes: {unsorted:?} -> {sorted:?}"
        );
    }
}

#[test]
fn a_one_material_scene_draws_as_before() {
    let Some(mut rig) = Rig::new(true) else {
        return;
    };
    let material = rig.material(MaterialDesc::default());
    let cube = rig.mesh(&MeshData::cube(1.0), material);
    for i in 0..10 {
        let at = vec3(i as f32 * 1.5 - 7.0, 0.5, 0.0);
        rig.ctx.world.spawn((Transform3D::at(at), Model::new(cube)));
    }
    rig.ctx.camera3d.position = vec3(0.0, 4.0, 12.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    let (before, unsorted) = rig.settled(false);
    let (after, sorted) = rig.settled(true);
    rig.assert_same_frame(&before, &after, "sorted and unsorted runs");
    assert_eq!(sorted, unsorted);
}

/// CPU time and state changes of the many-material scene, mesh order vs
/// sorted, alternating in rounds so that load on the machine hits both
/// alike. Run in release:
/// `cargo test --release --lib --features 3d bench_draw_order -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn bench_draw_order() {
    let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
    for meshes in [1, 300, 2000] {
        let Some(mut rig) = Rig::new(true) else {
            return;
        };
        if meshes == 1 {
            let material = rig.material(MaterialDesc::default());
            let cube = rig.mesh(&MeshData::cube(1.0), material);
            for i in 0..100 {
                let at = vec3((i % 10) as f32 * 1.5 - 7.0, 0.5, (i / 10) as f32 * -1.5);
                rig.ctx.world.spawn((Transform3D::at(at), Model::new(cube)));
            }
            rig.ctx.camera3d.position = vec3(0.0, 6.0, 12.0);
            rig.ctx.camera3d.look_at(Vec3::ZERO);
        } else {
            many_materials(&mut rig, meshes);
        }
        let mut times: [Vec<[f64; 2]>; 2] = Default::default();
        let mut changes = [StateChanges::default(); 2];
        let (rounds, frames) = (10, 50);
        for _ in 0..rounds {
            for (i, group) in [false, true].into_iter().enumerate() {
                rig.renderer.group_draws = group;
                for _ in 0..5 {
                    rig.render();
                }
                rig.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                rig.renderer.take_state_changes();
                for _ in 0..frames {
                    let [prepare, record] = rig.render();
                    times[i].push([ms(prepare), ms(record)]);
                    rig.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                }
                changes[i] = rig.renderer.take_state_changes();
            }
        }
        for (i, group) in [false, true].into_iter().enumerate() {
            let stats = |k: usize| {
                let mut t: Vec<f64> = times[i].iter().map(|t| t[k]).collect();
                t.sort_by(f64::total_cmp);
                (t[0], t[t.len() / 2], t[t.len() * 95 / 100])
            };
            let (c, per) = (changes[i], frames);
            eprintln!(
                "{meshes} meshes, sorted {group}: prepare {:.3?} ms, record {:.3?} ms (min, p50, p95 of {}); \
                 per frame: {} pipelines, {} bind groups, {} vertex buffers, {} index buffers, {} draws",
                stats(0),
                stats(1),
                times[i].len(),
                c.pipelines / per,
                c.bind_groups / per,
                c.vertex_buffers / per,
                c.index_buffers / per,
                c.draws / per,
            );
        }
    }
}

#[test]
fn sorted_runs_draw_model_material_overrides_the_same() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        many_materials(&mut rig, 120);
        let tint = rig.shader(
            "override_tint",
            "fn surface(s: Surface) -> Surface { var o = s; o.color = s.color * 0.7; return o; }",
        );
        // Overrides that change the pipeline as well as the material: solid
        // parts turned cut-out, double-sided, custom-shaded or transparent,
        // and transparent panes turned solid.
        let overrides = [
            MaterialDesc {
                color: Color::rgb(0.9, 0.9, 0.2),
                ..Default::default()
            },
            MaterialDesc {
                color: Color::rgb(0.2, 0.9, 0.9),
                blend: Blend::Cutout,
                ..Default::default()
            },
            MaterialDesc {
                color: Color::rgb(0.9, 0.2, 0.9),
                double_sided: true,
                ..Default::default()
            },
            MaterialDesc {
                color: Color::rgb(0.5, 0.8, 0.3),
                shader: Some(tint),
                ..Default::default()
            },
            MaterialDesc {
                color: Color::rgba(0.3, 1.0, 0.4, 0.5),
                blend: Blend::Transparent,
                ..Default::default()
            },
        ]
        .map(|desc| rig.material(desc));
        let mut models: Vec<_> = rig
            .ctx
            .world
            .query::<(crate::Entity, &Model)>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        models.sort_unstable();
        // Not the ground (spawned first); every other model after it.
        for (i, &e) in models.iter().skip(1).step_by(2).enumerate() {
            let m = overrides[i % overrides.len()];
            rig.ctx.world.insert(e, ModelMaterial(m));
        }
        let (before, unsorted) = rig.settled(false);
        let (after, sorted) = rig.settled(true);
        eprintln!("prepass {solid_prepass}: mesh order {unsorted:?}, sorted {sorted:?}");
        let differ = before.iter().zip(&after).filter(|(a, b)| a != b).count();
        rig.assert_healthy("sorted runs");
        assert_eq!(differ, 0, "the same pixels (prepass {solid_prepass})");
        assert_eq!(sorted.draws, unsorted.draws, "the same draws");
        // Overridden runs sort by the material they draw with, not the
        // mesh's own.
        assert!(
            sorted.pipelines * 4 < unsorted.pipelines,
            "far fewer pipeline changes: {unsorted:?} -> {sorted:?}"
        );

        // The overrides show: taking them off changes the picture.
        for &e in &models {
            rig.ctx.world.remove::<ModelMaterial>(e);
        }
        let (plain, _) = rig.settled(true);
        let changed = plain.iter().zip(&after).filter(|(a, b)| a != b).count();
        assert!(changed > 1000, "the overrides are drawn ({changed} bytes differ)");
    }
}
