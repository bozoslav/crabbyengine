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
}

impl Rig {
    /// `None` without a GPU.
    fn new() -> Option<Self> {
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

/// A mirror floor and a glowing red box over it: with screen-space
/// reflections, the floor in front of the box shows it.
fn mirror_rig() -> Option<(Rig, Vec3, Vec3)> {
    let mut rig = Rig::new()?;
    let mirror = MaterialDesc {
        color: Color::WHITE,
        roughness: 0.0,
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
