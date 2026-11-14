//! Sprite blend modes and sprite materials, drawn offscreen through the
//! window's sprite pass: each blend mode combines with the background as
//! documented (the plain alpha draw is the control), a material's shader
//! reads its parameters and extra textures, hot reloads replace the
//! shader and a GPU rejection keeps the old one. Skips without a GPU.

use glam::{Vec2, Vec4};

use super::batch::{DrawList, SpriteInstance};
use super::gpu::{GpuTexture, Renderer, SpritePass, sprite_pipeline};
use super::render_test_support::{gpu, gpu_adjusted, read_texture, target, white_textures};
use super::sprite_pipelines::SpritePipelines;
use crate::color::Color;
use crate::sprite_material::{BlendMode, SpriteMaterial, compose_sprite_shader, style_key};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// One 8 x 8 cell per draw, side by side.
const CELL: u32 = 8;
/// The background, sRGB 0.5 grey.
const BACKGROUND: Color = Color::rgb(0.5, 0.5, 0.5);

struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture_layout: wgpu::BindGroupLayout,
    camera_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    looks: SpritePipelines,
    textures: Vec<GpuTexture>,
}

impl Rig {
    fn new(gpu: Option<super::render_test_support::TestGpu>) -> Option<Self> {
        let gpu = gpu?;
        let (device, queue) = (gpu.device, gpu.queue);
        let (texture_layout, _, textures) = white_textures(&device, &queue);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));
        let pipeline = sprite_pipeline(&device, &layout, &shader, FORMAT);
        let looks = SpritePipelines::new(&device, &queue, &shader, &layout, &camera_layout, &texture_layout);
        Some(Self {
            device,
            queue,
            texture_layout,
            camera_layout,
            pipeline,
            looks,
            textures,
        })
    }

    /// Adds a 1x1 texture of sRGB `rgba`; returns its id.
    fn texture(&mut self, rgba: [u8; 4]) -> u32 {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            texture.size(),
        );
        let view = texture.create_view(&Default::default());
        let sampler = self.device.create_sampler(&Default::default());
        self.textures.push(GpuTexture {
            bind_group: self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.texture_layout,
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
            texture,
            nearest: false,
            mips: None,
        });
        (self.textures.len() - 1) as u32
    }

    fn shader(&mut self, id: u32, user: &str) -> Result<(), Box<crate::render::ShaderDiagnostic>> {
        self.looks
            .set_shader(&self.device, id, "test.wgsl", &compose_sprite_shader(user), FORMAT)
    }

    /// Draws one white 8x8 sprite per `(style, tint)`, in cells left to
    /// right, over the grey background; returns each cell's center as
    /// linear RGB.
    fn draw(&mut self, draws: &[(u32, Color)]) -> Vec<[f32; 3]> {
        self.draw_textured(&draws.iter().map(|&(s, c)| (0, s, c)).collect::<Vec<_>>())
    }

    /// [`draw`](Self::draw) with a texture per sprite.
    fn draw_textured(&mut self, draws: &[(u32, u32, Color)]) -> Vec<[f32; 3]> {
        use wgpu::util::DeviceExt;
        let size = (CELL * draws.len() as u32, CELL);
        let mut list = DrawList::default();
        for (i, &(texture, style, tint)) in draws.iter().enumerate() {
            let center = [(i as u32 * CELL + CELL / 2) as f32, (CELL / 2) as f32];
            let sprite = SpriteInstance::new(center, [CELL as f32; 2], [0.5, 0.5], 0.0, tint.to_rgba8());
            list.push_styled(texture, style, 0.0, sprite);
        }
        let (instances, batches) = list.finish();
        self.looks.prepare(&self.device, &self.textures, batches, FORMAT);
        let device = &self.device;
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice::<SpriteInstance, u8>(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let projection = crate::ctx::screen_projection(Vec2::new(size.0 as f32, size.1 as f32));
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&projection.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let camera = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let output = target(device, size, FORMAT);
        let view = output.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        Renderer::sprite_pass_with(
            &mut encoder,
            &view,
            Some(BACKGROUND),
            None,
            &SpritePass {
                viewport: [size.0, size.1],
                scale: 1.0,
                pipeline: &self.pipeline,
                instances: &instances,
                textures: &self.textures,
                layers: &[(&camera, batches, 0)],
            },
            Some((&self.looks, FORMAT)),
        );
        self.queue.submit([encoder.finish()]);
        let rgba = read_texture(device, &self.queue, &output);
        (0..draws.len())
            .map(|i| {
                let at = ((CELL / 2 * size.0 + i as u32 * CELL + CELL / 2) * 4) as usize;
                [0, 1, 2].map(|k| to_linear(f32::from(rgba[at + k]) / 255.0))
            })
            .collect()
    }
}

fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// `c`'s color in linear light (as the shader sees an 8-bit tint).
fn linear(c: Color) -> [f32; 3] {
    let [r, g, b, _] = c.to_rgba8();
    [r, g, b].map(|v| to_linear(f32::from(v) / 255.0))
}

#[track_caller]
fn assert_near(what: &str, got: [f32; 3], want: [f32; 3]) {
    // 8-bit sRGB readback: a step is under 0.01 in linear light up here.
    let close = got.iter().zip(want).all(|(g, w)| (g - w).abs() < 0.02);
    assert!(close, "{what}: got {got:?}, expected {want:?}");
}

fn mix(a: [f32; 3], b: [f32; 3], f: impl Fn(f32, f32) -> f32) -> [f32; 3] {
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

fn blend_cases(rig: &mut Rig) {
    let tint = Color::rgba(1.0, 0.5, 0.0, 0.5);
    let (c, a, b) = (linear(tint), 0.5, linear(BACKGROUND));
    let keys: Vec<(u32, Color)> = BlendMode::ALL.iter().map(|&m| (style_key(m, None), tint)).collect();
    let got = rig.draw(&keys);
    let alpha = mix(c, b, |c, b| c * a + b * (1.0 - a));
    assert_near("alpha", got[0], alpha);
    assert_near("additive", got[1], mix(c, b, |c, b| (b + c * a).min(1.0)));
    assert_near("multiply", got[2], mix(c, b, |c, b| b * (c * a + 1.0 - a)));
    // The built-in look premultiplies the tint: the same as alpha for an
    // opaque texture.
    assert_near("premultiplied", got[3], alpha);
    // Control: the modes really differ from the plain draw.
    for (i, mode) in ["additive", "multiply"].iter().enumerate() {
        let d: f32 = got[i + 1].iter().zip(got[0]).map(|(x, y)| (x - y).abs()).sum();
        assert!(d > 0.1, "{mode} drew the same as alpha: {got:?}");
    }
    // Plain sprites (no look) still draw with the alpha pipeline.
    assert_near("plain", rig.draw(&[(0, tint)])[0], alpha);
}

#[test]
fn blend_modes_combine_with_whats_behind_as_documented() {
    let Some(mut rig) = Rig::new(gpu()) else { return };
    blend_cases(&mut rig);
}

/// A material's shader with its parameters and an extra texture; and the
/// control: the same material without the texture (white).
fn material_cases(rig: &mut Rig) {
    rig.shader(
        0,
        "fn fragment(p: SpritePixel) -> vec4<f32> {\n    return vec4<f32>(material.params[0].rgb * material_sample0(p.uv).rgb, p.tint.a);\n}",
    )
    .unwrap();
    let green = rig.texture([0, 255, 0, 255]);
    let params = [Vec4::new(1.0, 0.5, 1.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO];
    rig.looks
        .set_material(&rig.device, &rig.queue, 0, Some(0), &params, [Some(green), None]);
    rig.looks
        .set_material(&rig.device, &rig.queue, 1, Some(0), &params, [None, None]);
    let material = |id| style_key(BlendMode::Alpha, Some(SpriteMaterial { id }));
    let got = rig.draw(&[(material(0), Color::WHITE), (material(1), Color::WHITE)]);
    assert_near("params × extra texture", got[0], [0.0, 0.5, 0.0]);
    assert_near("params × white (no texture)", got[1], [1.0, 0.5, 1.0]);

    // New parameters show in the next frame.
    let brighter = [Vec4::new(0.25, 1.0, 1.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO];
    rig.looks
        .set_material(&rig.device, &rig.queue, 1, Some(0), &brighter, [None, None]);
    assert_near(
        "updated params",
        rig.draw(&[(material(1), Color::WHITE)])[0],
        [0.25, 1.0, 1.0],
    );

    // A texture replaced in its slot (resolution streaming, freeing) is
    // picked up: the material's bind group is made again.
    let red = rig.texture([255, 0, 0, 255]);
    rig.textures.swap(green as usize, red as usize);
    assert_near(
        "replaced texture",
        rig.draw(&[(material(0), Color::WHITE)])[0],
        [1.0, 0.0, 0.0],
    );

    // Blend modes apply to a material's shader too: additive over grey.
    let additive = style_key(BlendMode::Additive, Some(SpriteMaterial { id: 1 }));
    let b = linear(BACKGROUND);
    let tint = Color::rgba(1.0, 1.0, 1.0, 0.5);
    assert_near(
        "additive material",
        rig.draw(&[(additive, tint)])[0],
        mix([0.25, 1.0, 1.0], b, |c, b| (b + c * 0.5).min(1.0)),
    );

    // A freed material draws the built-in look: texture × tint.
    rig.looks.free_material(1);
    assert_near("freed material", rig.draw(&[(material(1), Color::WHITE)])[0], [1.0; 3]);
}

#[test]
fn sprite_materials_run_their_shader_with_params_and_textures() {
    let Some(mut rig) = Rig::new(gpu()) else { return };
    material_cases(&mut rig);
}

#[test]
fn a_hot_reload_replaces_the_shader_and_a_rejected_one_keeps_the_old() {
    let Some(mut rig) = Rig::new(gpu()) else { return };
    let red = "fn fragment(p: SpritePixel) -> vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }";
    let blue = "fn fragment(p: SpritePixel) -> vec4<f32> { return vec4<f32>(0.0, 0.0, 1.0, 1.0); }";
    rig.shader(0, red).unwrap();
    rig.looks
        .set_material(&rig.device, &rig.queue, 0, Some(0), &[Vec4::ZERO; 4], [None, None]);
    let material = style_key(BlendMode::Alpha, Some(SpriteMaterial { id: 0 }));
    assert_near(
        "first version",
        rig.draw(&[(material, Color::WHITE)])[0],
        [1.0, 0.0, 0.0],
    );
    rig.shader(0, blue).unwrap();
    assert_near("reloaded", rig.draw(&[(material, Color::WHITE)])[0], [0.0, 0.0, 1.0]);
    // Code the GPU refuses (here: never checked by the engine first).
    let error = rig.shader(0, "fn fragment(p: SpritePixel) -> vec4<f32> { return nope; }");
    let error = error.unwrap_err();
    assert_eq!(error.kind, crate::render::ShaderKind::Sprite);
    assert!(error.message.contains("rejected"), "{error}");
    assert_near("kept", rig.draw(&[(material, Color::WHITE)])[0], [0.0, 0.0, 1.0]);
    // A freed shader's materials draw the built-in look.
    rig.looks.free_shader(0);
    assert_near("freed shader", rig.draw(&[(material, Color::WHITE)])[0], [1.0; 3]);
}

#[test]
fn blend_modes_and_materials_draw_with_webgl2_limits() {
    let gpu = gpu_adjusted(
        |_| Ok(wgpu::DeviceDescriptor::default()),
        |d| d.required_limits = wgpu::Limits::downlevel_webgl2_defaults(),
    );
    let Some(mut rig) = Rig::new(gpu) else { return };
    blend_cases(&mut rig);
    material_cases(&mut rig);
}

#[test]
fn the_examples_dissolve_burns_away_where_its_noise_is_below_the_progress() {
    let Some(mut rig) = Rig::new(gpu()) else { return };
    rig.shader(0, include_str!("../../assets/shaders/sprite_dissolve.wgsl"))
        .unwrap();
    // Noise 0.5 in sRGB: about 0.21 sampled.
    let noise = rig.texture([128, 128, 128, 255]);
    let edge = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let material = style_key(BlendMode::Alpha, Some(SpriteMaterial { id: 0 }));
    let burned = |rig: &mut Rig, progress: f32| {
        let params = [Vec4::new(progress, 0.0, 0.0, 0.0), edge, Vec4::ZERO, Vec4::ZERO];
        rig.looks
            .set_material(&rig.device, &rig.queue, 0, Some(0), &params, [Some(noise), None]);
        rig.draw(&[(material, Color::WHITE)])[0]
    };
    assert_near("not burned", burned(&mut rig, 0.0), [1.0; 3]);
    assert_near("burned away", burned(&mut rig, 0.5), linear(BACKGROUND));
    let glowing = burned(&mut rig, 0.18);
    assert!(glowing[0] > 0.9 && glowing[1] < 0.9, "the edge glows red: {glowing:?}");
}
