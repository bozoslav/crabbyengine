//! Light shafts: the fog's sun glow stops where the fog is in shadow.
//! Skips without a GPU (see `render_test_support`).
use super::gpu::GpuTexture;
use super::gpu3d::*;
use super::render_test_support::*;
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
        let TestGpu { device, queue, .. } = gpu()?;
        let (layout, white, textures) = white_textures(&device, &queue);
        let output = target(&device, (SIZE, SIZE), wgpu::TextureFormat::Rgba8UnormSrgb);
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
        let material = self.ctx.gfx.create_material(desc);
        let mesh = self.ctx.gfx.create_mesh(data, material);
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
            &ctx.gfx.assets3d,
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
        read_texture(&self.device, &self.queue, &self.output)
    }
}

#[test]
fn shadows_cut_the_fogs_sun_glow_into_shafts() {
    let Some(mut rig) = Rig::new() else { return };
    let black = rig.mesh(&MeshData::cube(1.0), MaterialDesc::color(Color::BLACK));
    // A roof over everything left of the camera, and a black wall ahead.
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(-20.0, 10.0, -20.0),
            scale: vec3(40.0, 1.0, 60.0),
            ..Default::default()
        },
        Model::new(black),
    ));
    rig.ctx.world.spawn((
        Transform3D {
            position: vec3(0.0, 0.0, -30.0),
            scale: vec3(100.0, 40.0, 1.0),
            ..Default::default()
        },
        Model::new(black),
    ));
    rig.ctx.lighting.shadows = true;
    rig.ctx.lighting.sun_direction = vec3(0.1, -1.0, 0.05);
    rig.ctx.camera3d.position = vec3(0.0, 2.0, 0.0);
    rig.ctx.camera3d.look_at(vec3(0.0, 2.0, -1.0));
    // Black fog, so all its light is the sun's glow, the same all around.
    rig.ctx.sky.fog_density = 0.05;
    rig.ctx.sky.fog_height = 50.0;
    rig.ctx.sky.fog_color = Color::BLACK;
    rig.ctx.sky.fog_sun_glow = 1.0;
    rig.ctx.sky.fog_sun_focus = 0.0;
    let (left, right) = ((SIZE / 4, SIZE / 2), (SIZE * 3 / 4, SIZE / 2));
    let red = |pixels: &[u8], (x, y): (u32, u32)| i32::from(pixels[((y * SIZE + x) * 4) as usize]);

    let pixels = rig.draw();
    let (shade, sun) = (red(&pixels, left), red(&pixels, right));
    assert!(
        shade > 100 && (shade - sun).abs() < 20,
        "without shafts, all glows: {shade} {sun}"
    );

    rig.ctx.sky.light_shafts = 1.0;
    let pixels = rig.draw();
    let (shade, sun) = (red(&pixels, left), red(&pixels, right));
    assert!(shade < 30, "the fog under the roof is dark: {shade}");
    assert!(sun > 100, "the fog in the sun glows: {sun}");
}
