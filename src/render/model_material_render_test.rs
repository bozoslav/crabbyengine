//! `ModelMaterial` drawn offscreen: one mesh, uploaded once, drawn with
//! its own material and two others side by side, in both prepass modes.
//! Skips without a GPU (a skip is not a pass).
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
            crate::render::render_test_support::no_gpu("no GPU adapter");
            return None;
        };
        eprintln!("model material test adapter: {:?}", adapter.get_info());
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

    /// A material of `desc`, on the CPU and the GPU.
    fn material(&mut self, desc: MaterialDesc) -> Material {
        let material = self.ctx.gfx.create_material(desc);
        self.renderer
            .create_material(&self.device, &GpuMaterialDesc::new(&desc), &self.textures);
        material
    }

    /// A mesh of `data` with `desc`, on the CPU and the GPU.
    fn mesh(&mut self, data: &MeshData, desc: MaterialDesc) -> Mesh {
        let material = self.material(desc);
        let mesh = self.ctx.gfx.create_mesh(data, material);
        self.renderer
            .create_mesh(&self.device, data, &[(0..data.indices.len() as u32, material)], true);
        mesh
    }

    /// Draws the world and reads the pixels back (RGBA).
    fn draw(&mut self) -> (Vec<u8>, usize) {
        let ctx = &mut self.ctx;
        let mut frame = Frame3D::default();
        frame.lod = ctx.lod;
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
            // No light: the cubes glow on black.
            light: SkyLight {
                sun: Vec3::ZERO,
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
        (readback.get_mapped_range(..).unwrap().to_vec(), frame.runs.len())
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

/// Red, green and blue pixels in an RGBA image.
fn count(pixels: &[u8]) -> [usize; 3] {
    let mut c = [0; 3];
    for p in pixels.as_chunks::<4>().0 {
        let [r, g, b] = [p[0], p[1], p[2]].map(u32::from);
        for (i, (on, off)) in [(r, g + b), (g, r + b), (b, r + g)].into_iter().enumerate() {
            if on > 120 && off < 80 {
                c[i] += 1;
            }
        }
    }
    c
}

/// Red, green and blue pixels in a 9x9 box around where `at` shows.
fn around(pixels: &[u8], view_proj: Mat4, at: Vec3) -> [usize; 3] {
    let ndc = view_proj.project_point3(at);
    let x = ((ndc.x * 0.5 + 0.5) * SIZE as f32) as u32;
    let y = ((0.5 - ndc.y * 0.5) * SIZE as f32) as u32;
    let mut part = Vec::new();
    for yy in y - 4..=y + 4 {
        for xx in x - 4..=x + 4 {
            let i = ((yy * SIZE + xx) * 4) as usize;
            part.extend_from_slice(&pixels[i..i + 4]);
        }
    }
    count(&part)
}

#[test]
fn one_mesh_draws_with_each_models_material() {
    for solid_prepass in [true, false] {
        let Some(mut rig) = Rig::new(solid_prepass) else {
            return;
        };
        let cube = rig.mesh(&MeshData::cube(1.6), glowing(Color::rgb(0.0, 1.0, 0.0)));
        let red = rig.material(glowing(Color::rgb(1.0, 0.0, 0.0)));
        let blue = rig.material(glowing(Color::rgb(0.0, 0.0, 1.0)));
        // Shadow maps on: the overrides go through the shadow passes too.
        rig.ctx.lighting.shadows = true;
        rig.ctx.camera3d.position = vec3(0.0, 0.0, 8.0);
        rig.ctx.camera3d.look_at(Vec3::ZERO);
        let world = &mut rig.ctx.world;
        world.spawn((
            Transform3D::at(vec3(-2.4, 0.0, 0.0)),
            Model::new(cube),
            ModelMaterial(red),
        ));
        world.spawn((Transform3D::at(Vec3::ZERO), Model::new(cube)));
        world.spawn((
            Transform3D::at(vec3(2.4, 0.0, 0.0)),
            Model::new(cube),
            ModelMaterial(blue),
        ));
        world.spawn((
            Transform3D::at(vec3(2.4, 2.4, 0.0)),
            Model::new(cube),
            ModelMaterial(blue),
        ));
        let (pixels, runs) = rig.draw();
        let view_proj = rig.ctx.camera3d.view_proj();
        let at = |p: &[u8], x: f32| around(p, view_proj, vec3(x, 0.0, 0.0));
        let [left, middle, right] = [-2.4, 0.0, 2.4].map(|x| at(&pixels, x));
        eprintln!("prepass {solid_prepass}: {left:?} {middle:?} {right:?}, {runs} runs");
        assert!(
            left[0] == 81 && left[1] == 0 && left[2] == 0,
            "left: red only: {left:?}"
        );
        assert!(
            middle[1] == 81 && middle[0] == 0 && middle[2] == 0,
            "middle: its own green: {middle:?}"
        );
        assert!(
            right[2] == 81 && right[0] == 0 && right[1] == 0,
            "right: blue only: {right:?}"
        );
        // Plain, red, and both blue cubes in one run.
        assert_eq!(runs, 3);

        // Taking the override off draws the mesh's own material again.
        let blues: Vec<_> = rig
            .ctx
            .world
            .query::<(crate::Entity, &ModelMaterial)>()
            .iter()
            .filter(|(_, m)| m.0 == blue)
            .map(|(e, _)| e)
            .collect();
        for e in blues {
            rig.ctx.world.remove::<ModelMaterial>(e);
        }
        let (pixels, runs) = rig.draw();
        let right = at(&pixels, 2.4);
        assert!(right[1] == 81 && right[2] == 0, "right: green again: {right:?}");
        assert_eq!(runs, 2);
    }
}
