//! Offscreen GPU buffer compute and instanced material reads.
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
    fn new() -> Option<Self> {
        Self::with_compute(true)
    }
    fn with_compute(compute: bool) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return None;
        };
        let limits = if compute {
            wgpu::Limits::default()
        } else {
            wgpu::Limits::downlevel_webgl2_defaults()
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: limits,
            ..Default::default()
        })) else {
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
        self.draw_with_grid(None)
    }
    fn draw_with_grid(&mut self, grid: Option<crate::three::occlusion::OcclusionGrid>) -> Vec<u8> {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
        frame.occlusion = grid;
        frame.collect(
            &mut ctx.world,
            &ctx.assets3d,
            &SkinFrame::default(),
            &ctx.camera3d,
            &ctx.lighting,
        );
        frame.computes.extend(
            ctx.compute_passes
                .iter()
                .map(|c| (c.compute.id, c.buffer.unwrap().id | (1 << 31), 0)),
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
            occlusion: ctx.camera3d.occlusion_culling,
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

/// The world point under screen pixel `p`'s projection.
fn pixel(view_proj: Mat4, p: Vec3) -> (u32, u32) {
    let c = view_proj * p.extend(1.0);
    let ndc = c.truncate() / c.w;
    (
        ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32,
        ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32,
    )
}

#[test]
fn buffer_compute_places_two_instances_before_drawing() {
    buffer_draw(true);
}
#[test]
fn buffer_draw_skips_devices_without_compute() {
    buffer_draw(false);
}
fn buffer_draw(compute: bool) {
    let rig = if compute { Rig::new() } else { Rig::with_compute(false) };
    let Some(mut rig) = rig else { return };
    if compute && rig.device.limits().max_storage_buffers_per_shader_stage == 0 {
        return;
    }
    let source = "fn vertex(v: Vertex) -> Vertex { var o = v; o.position += items[v.instance_index].xyz; return o; }";
    let shader = rig.ctx.create_shader("buffer draw", source);
    rig.renderer.create_shader(&rig.device, "buffer draw", source).unwrap();
    let mesh = rig.mesh(
        &MeshData::cube(0.4),
        MaterialDesc {
            shader: Some(shader),
            emissive: Color::WHITE,
            ..Default::default()
        },
    );
    let mut data = vec![vec4(99.0, -1.0, 0.0, 1.0); 65];
    data[0].x = -2.0;
    let b = rig.ctx.create_gpu_buffer(65, &data);
    rig.renderer.create_gpu_buffer(&rig.device, 65, &data);
    rig.renderer
        .write_gpu_buffer(&rig.queue, b.id, 64, &[vec4(0.0, -1.0, 0.0, 1.0)]);
    rig.renderer.set_mesh_gpu_buffer(mesh.id, b.id, 65);
    let first = "fn update(i: u32) { items[i].x += 1.0; }";
    let second = "fn update(i: u32) { items[i].y += 1.0 + frame.time.y * 0.0; }";
    for source in [first, second] {
        let c = rig.ctx.create_compute("positions", source);
        rig.renderer.create_compute(&rig.device, "positions", source).unwrap();
        rig.ctx.compute_passes.push(ComputePass::for_buffer(c, b));
    }
    rig.ctx.world.spawn((Transform3D::default(), Model::new(mesh)));
    rig.ctx.lighting.shadows = false;
    rig.ctx.camera3d.position = vec3(0.0, 0.0, 5.0);
    rig.ctx.camera3d.look_at(Vec3::ZERO);
    let view_proj = rig.ctx.camera3d.view_proj();
    let pixels = rig.draw();
    let red = |p| {
        let (x, y) = pixel(view_proj, p);
        pixels[((y * SIZE + x) * 4) as usize]
    };
    if !compute {
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[0] < 20 && p[1] < 20 && p[2] < 20)
        );
        return;
    }
    assert!(red(vec3(-1.0, 0.0, 0.0)) > 150, "first computed instance");
    assert!(red(vec3(1.0, 0.0, 0.0)) > 150, "second computed instance");
    assert!(red(Vec3::ZERO) < 20, "nothing at the identity origin");
    assert!(red(vec3(-1.0, -1.0, 0.0)) < 20, "ordered second pass moved it");
}

#[test]
fn persistent_batches_cull_on_gpu() {
    persistent_batch_draw(true);
}
#[test]
fn persistent_batches_fallback_draws_visible_cells() {
    persistent_batch_draw(false);
}
fn persistent_batch_draw(compute: bool) {
    let Some(mut rig) = Rig::with_compute(compute) else {
        return;
    };
    let mesh = rig.mesh(
        &MeshData::cube(0.5),
        MaterialDesc {
            emissive: Color::WHITE,
            ..Default::default()
        },
    );
    rig.ctx.camera3d.position = Vec3::ZERO;
    rig.ctx.camera3d.look_at(-Vec3::Z);
    rig.ctx.lighting.shadows = false;
    let options = InstanceBatchOptions {
        cell_size: if compute { 32.0 } else { 1.0 },
        max_distance: 10.0,
        ..Default::default()
    };
    let at = [vec3(-1.0, 0.0, -5.0), vec3(1.0, 0.0, 5.0), vec3(-8.0, 0.0, -20.0)];
    let transforms = at.map(Mat4::from_translation);
    let b = rig.ctx.create_instance_batch(mesh, &transforms, options);
    rig.renderer.set_instance_batch(
        &rig.device,
        b.id(),
        rig.ctx.assets3d.instance_batches[b.id() as usize].as_ref(),
    );
    let image = rig.draw();
    let vp = rig.ctx.camera3d.view_proj();
    let (x, y) = pixel(vp, at[0]);
    assert!(
        image[((y * SIZE + x) * 4) as usize] > 100,
        "visible batch absent ({compute})"
    );
    // The far instance projects inside the view, but its maximum distance excludes it.
    let (x, y) = pixel(vp, at[2]);
    assert!(
        image[((y * SIZE + x) * 4) as usize] < 50,
        "distant instance drawn ({compute})"
    );
    if compute {
        let view = &rig.renderer.static_batches[b.id() as usize].as_ref().unwrap().views[0];
        let read = rig.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 20,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = rig.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&view.args, 0, &read, 0, 20);
        rig.queue.submit([encoder.finish()]);
        read.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        rig.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = read.get_mapped_range(..).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            1,
            "behind/distant instances survived GPU culling"
        );
    }
    rig.ctx.camera3d.occlusion_culling = true;
    let wall = vp * vec3(0.0, 0.0, -2.0).extend(1.0);
    let grid = crate::three::occlusion::OcclusionGrid::new(vp, Vec3::ZERO, 16, 16, vec![wall.z / wall.w; 256]);
    let hidden = rig.draw_with_grid(Some(grid));
    let (x, y) = pixel(vp, at[0]);
    assert!(
        hidden[((y * SIZE + x) * 4) as usize] < 50,
        "occluded instance survived ({compute})"
    );
    rig.renderer.set_instance_batch(&rig.device, b.id(), None);
    // Cutout-only batches must trigger the coverage prepass even without model entities.
    let cutout = rig.mesh(
        &MeshData::cube(0.5),
        MaterialDesc {
            blend: Blend::Cutout,
            emissive: Color::WHITE,
            ..Default::default()
        },
    );
    let cb = rig
        .ctx
        .create_instance_batch(cutout, &[Mat4::from_translation(at[0])], options);
    rig.renderer.set_instance_batch(
        &rig.device,
        cb.id(),
        rig.ctx.assets3d.instance_batches[cb.id() as usize].as_ref(),
    );
    rig.ctx.lighting.shadows = true;
    rig.ctx.camera3d.occlusion_culling = false;
    let image = rig.draw();
    assert!(
        image[((y * SIZE + x) * 4) as usize] > 100,
        "cutout batch absent ({compute})"
    );
    rig.renderer.set_instance_batch(&rig.device, cb.id(), None);
    rig.renderer.set_instance_batch(&rig.device, b.id(), None);
    let image = rig.draw();
    let (x, y) = pixel(vp, at[0]);
    assert!(image[((y * SIZE + x) * 4) as usize] < 50, "freed batch still drawn");
}
