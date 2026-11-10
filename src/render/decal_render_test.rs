//! Projected decals drawn offscreen: a decal paints the floor inside its box
//! and nowhere else, steep surfaces and opted-out models don't get it, it is
//! lit with the surface it lands on, and its image and normal map land where
//! they should. Skips without a GPU that has a spare texture slot.
use super::gpu::GpuTexture;
use super::gpu3d::*;
use super::render_test_support::*;
#[allow(unused_imports)]
use crate::d3::{Decal as Decal3D, Transform as Transform3D};
use crate::prelude::*;
#[allow(unused_imports)]
use crate::render::{Material, MaterialDesc, Mesh, MeshData, Model, ReceivesDecals};
#[allow(unused_imports)]
use crate::terrain::TextureKind;
use crate::three::{Frame3D, GpuMaterialDesc, Scene3D, SkinFrame, SkyLight, TextureArray, linear};

/// Width and height of the offscreen frames (a row of pixels is 512 bytes, as copies need).
const SIZE: u32 = 128;

struct Rig {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer3D,
    ctx: Ctx,
    output: wgpu::Texture,
    size: (u32, u32),
    textures: Vec<GpuTexture>,
    material: Material,
}

impl Rig {
    /// A device with room for decals, or `None` (the test skips).
    fn new() -> Option<Rig> {
        Self::with_slots(true)
    }

    /// `decals` false: a device at the default limits (WebGL2's too).
    fn with_slots(decals: bool) -> Option<Rig> {
        Self::sized(decals, (SIZE, SIZE))
    }

    fn sized(decals: bool, size: (u32, u32)) -> Option<Rig> {
        let TestGpu { device, queue, .. } = gpu_with(|adapter| {
            let wanted = if decals {
                adapter.limits().max_sampled_textures_per_shader_stage.clamp(16, 24)
            } else {
                16
            };
            Ok(wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits {
                    max_sampled_textures_per_shader_stage: wanted,
                    ..wgpu::Limits::default()
                },
                ..Default::default()
            })
        })?;
        if supports_decals(&device) != decals {
            skip(format_args!(
                "this GPU can't be set to {} texture slots",
                if decals { "17+" } else { "16" }
            ));
            return None;
        }
        let (layout, white, textures) = white_textures(&device, &queue);
        let output = target(&device, size, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::new(size.0 as f32, size.1 as f32));
        let matte = MaterialDesc {
            color: Color::WHITE,
            roughness: 1.0,
            ..Default::default()
        };
        let material = ctx.gfx.create_material(matte);
        let mut renderer = Renderer3D::new(&device, &layout, output.format(), 0, &white, 4, true);
        renderer.create_material(&device, &GpuMaterialDesc::new(&matte), &textures);
        ctx.gfx.assets3d.decals_supported = decals;
        ctx.sky.atmosphere = false;
        // A plain light: ambient and sun the same everywhere.
        ctx.lighting.shadows = false;
        ctx.lighting.sun_direction = vec3(0.0, -1.0, 0.0);
        Some(Rig {
            device,
            queue,
            renderer,
            ctx,
            output,
            size,
            textures,
            material,
        })
    }

    fn mesh(&mut self, data: &MeshData) -> Mesh {
        let mesh = self.ctx.gfx.create_mesh(data, self.material);
        self.renderer.create_mesh(
            &self.device,
            data,
            &[(0..data.indices.len() as u32, self.material)],
            true,
        );
        mesh
    }

    /// Decal textures from RGBA8 layers of `size` x `size`, as `TextureKind::Data`.
    fn decal_images(&mut self, size: u32, layers: &[&[u8]]) {
        let array = texture(
            &self.device,
            &self.queue,
            size,
            size,
            layers.len() as u32,
            wgpu::TextureFormat::Rgba8Unorm,
            &layers.concat(),
        );
        let id = self.renderer.add_texture_array(&array);
        let handle = TextureArray {
            id,
            width: size,
            height: size,
            layers: layers.len() as u32,
        };
        assert_eq!(id as usize, self.ctx.gfx.assets3d.arrays.len());
        self.ctx.gfx.assets3d.arrays.push(handle);
        self.ctx.gfx.assets3d.array_data.push(true);
        self.ctx.gfx.assets3d.arrays_live.insert(id);
        self.ctx.gfx.set_decal_textures(Some(handle));
    }

    /// Looks at the origin from `from`.
    fn camera(&mut self, from: Vec3) {
        self.ctx.camera3d.position = from;
        self.ctx.camera3d.look_at(Vec3::ZERO);
    }

    fn draw(&mut self) -> Vec<u8> {
        let (device, queue, ctx) = (&self.device, &self.queue, &mut self.ctx);
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
            light: {
                let rgb = |c| {
                    let [r, g, b, _] = linear(c);
                    vec3(r, g, b)
                };
                SkyLight {
                    sun: rgb(ctx.lighting.sun_color) * ctx.lighting.sun_intensity,
                    sky: rgb(ctx.lighting.sky_color),
                    ground: rgb(ctx.lighting.ground_color),
                }
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
                size: (self.size.0, self.size.1),
            },
        );
        let target = self.output.create_view(&Default::default());
        let buffers = self
            .renderer
            .draw(device, &target, wgpu::Color::BLACK, &self.textures, None);
        queue.submit(buffers);
        read_texture(device, queue, &self.output)
    }

    /// The color where world point `p` lands on screen.
    fn at(&self, pixels: &[u8], p: Vec3) -> [i32; 3] {
        let ndc = self.ctx.camera3d.view_proj().project_point3(p);
        let x = ((ndc.x * 0.5 + 0.5) * self.size.0 as f32) as u32;
        let y = ((0.5 - ndc.y * 0.5) * self.size.1 as f32) as u32;
        let i = ((y * self.size.0 + x) * 4) as usize;
        [pixels[i] as i32, pixels[i + 1] as i32, pixels[i + 2] as i32]
    }
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    layers: u32,
    format: wgpu::TextureFormat,
    data: &[u8],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });
    queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        texture.size(),
    );
    texture
}

fn red(c: [i32; 3]) -> bool {
    c[0] > c[1] + 60 && c[0] > c[2] + 60
}
fn neutral(c: [i32; 3]) -> bool {
    (c[0] - c[1]).abs() < 12 && (c[1] - c[2]).abs() < 12
}
/// Looking down: the box's +Z turned to point up.
fn down() -> Quat {
    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}
fn red_decal(size: Vec3) -> Decal3D {
    Decal3D {
        size,
        image: Decal3D::NO_IMAGE,
        color: Color::rgb(1.0, 0.0, 0.0),
        ..Default::default()
    }
}

#[test]
fn a_decal_paints_the_floor_inside_its_box_only() {
    let Some(mut rig) = Rig::new() else { return };
    let floor = rig.mesh(&MeshData::plane(20.0));
    let ground = rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    let decal = rig.ctx.world.spawn((
        Transform3D::at(vec3(0.0, 0.2, 0.0)).rotated(down()),
        red_decal(vec3(2.0, 2.0, 1.0)),
    ));
    rig.camera(vec3(0.0, 8.0, 5.0));
    let pixels = rig.draw();
    assert!(
        red(rig.at(&pixels, vec3(0.0, 0.0, 0.0))),
        "{:?}",
        rig.at(&pixels, Vec3::ZERO)
    );
    assert!(red(rig.at(&pixels, vec3(0.8, 0.0, 0.8))), "near the box's corner");
    for outside in [vec3(1.5, 0.0, 0.0), vec3(0.0, 0.0, 1.5), vec3(-3.0, 0.0, -2.0)] {
        assert!(
            neutral(rig.at(&pixels, outside)),
            "{outside}: {:?}",
            rig.at(&pixels, outside)
        );
    }
    // The box ends in depth: out of reach of the floor, nothing is drawn.
    rig.ctx.world.get_mut::<Transform3D>(decal).unwrap().position.y = 3.0;
    let in_air = rig.draw();
    assert!(
        neutral(rig.at(&in_air, Vec3::ZERO)),
        "a decal in the air paints nothing"
    );
    // It is drawn onto whatever is inside the box now: lift the floor into it.
    rig.ctx.world.get_mut::<Transform3D>(ground).unwrap().position.y = 3.0;
    let moved = rig.draw();
    let up = vec3(0.0, 3.0, 0.0);
    assert!(
        red(rig.at(&moved, up)),
        "the decal follows a moving model: {:?}",
        rig.at(&moved, up)
    );
    // Hidden decals (no opacity) and a full cell of nothing leave the floor alone.
    rig.ctx.world.get_mut::<Decal3D>(decal).unwrap().opacity = 0.0;
    let hidden = rig.draw();
    assert!(neutral(rig.at(&hidden, up)));
}

#[test]
fn the_image_lands_with_its_top_along_the_boxs_up() {
    let Some(mut rig) = Rig::new() else { return };
    let floor = rig.mesh(&MeshData::plane(20.0));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    // 2 x 2 pixels: the left column is opaque (red above, green below), the right transparent.
    let image: [u8; 16] = [
        255, 0, 0, 255, 0, 0, 0, 0, // top row
        0, 255, 0, 255, 0, 0, 0, 0, // bottom row
    ];
    rig.decal_images(2, &[&image]);
    rig.ctx.world.spawn((
        Transform3D::at(vec3(0.0, 0.2, 0.0)).rotated(down()),
        Decal3D {
            size: vec3(2.0, 2.0, 1.0),
            image: 0,
            ..Default::default()
        },
    ));
    rig.camera(vec3(0.0, 8.0, 5.0));
    let pixels = rig.draw();
    // The box's +Y points to world -Z here (away from this camera): the top row is the far one.
    let top_left = rig.at(&pixels, vec3(-0.5, 0.0, -0.5));
    let bottom_left = rig.at(&pixels, vec3(-0.5, 0.0, 0.5));
    let right = rig.at(&pixels, vec3(0.5, 0.0, 0.0));
    assert!(red(top_left), "{top_left:?}");
    assert!(
        bottom_left[1] > bottom_left[0] + 60 && bottom_left[1] > bottom_left[2] + 60,
        "{bottom_left:?}"
    );
    assert!(neutral(right), "transparent texels paint nothing: {right:?}");
}

#[test]
fn steep_surfaces_fade_the_decal_out() {
    let Some(mut rig) = Rig::new() else { return };
    // A wall facing the camera (+Z), through the origin.
    let wall = rig.mesh(&MeshData::plane(20.0));
    rig.ctx.world.spawn((
        Transform3D::default().rotated(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        Model::new(wall),
    ));
    rig.camera(vec3(0.0, 0.0, 9.0));
    let bare = rig.draw();
    let wall_color = rig.at(&bare, vec3(1.5, 0.0, 0.0));
    // One decal projects into the wall (from +Z), the other along it (downward).
    rig.ctx
        .world
        .spawn((Transform3D::at(vec3(-1.5, 0.0, 0.2)), red_decal(vec3(2.0, 2.0, 1.0))));
    rig.ctx.world.spawn((
        Transform3D::at(vec3(1.5, 0.0, 0.0)).rotated(down()),
        red_decal(vec3(2.0, 2.0, 1.0)),
    ));
    let pixels = rig.draw();
    let facing = rig.at(&pixels, vec3(-1.5, 0.0, 0.0));
    let along = rig.at(&pixels, vec3(1.5, 0.0, 0.0));
    assert!(red(facing), "{facing:?}");
    assert_eq!(along, wall_color, "a wall parallel to the projection shows nothing");
}

#[test]
fn models_can_opt_out_of_decals_by_layer() {
    let Some(mut rig) = Rig::new() else { return };
    let tile = rig.mesh(&MeshData::plane(2.0));
    let at = |x: f32| Transform3D::at(vec3(x, 0.0, 0.0));
    rig.ctx.world.spawn((at(-2.4), Model::new(tile), ReceivesDecals::NONE));
    rig.ctx.world.spawn((at(0.0), Model::new(tile)));
    rig.ctx.world.spawn((at(2.4), Model::new(tile), ReceivesDecals(0b10)));
    rig.ctx.world.spawn((
        Transform3D::at(vec3(0.0, 0.2, 0.0)).rotated(down()),
        Decal3D {
            layers: 0b01,
            ..red_decal(vec3(10.0, 10.0, 1.0))
        },
    ));
    rig.camera(vec3(0.0, 9.0, 4.0));
    let pixels = rig.draw();
    let (opted_out, all, other_layer) = (
        rig.at(&pixels, vec3(-2.4, 0.0, 0.0)),
        rig.at(&pixels, vec3(0.0, 0.0, 0.0)),
        rig.at(&pixels, vec3(2.4, 0.0, 0.0)),
    );
    assert!(
        neutral(opted_out),
        "a model with no layers takes no decals: {opted_out:?}"
    );
    assert!(red(all), "{all:?}");
    assert!(
        neutral(other_layer),
        "the decal's layer isn't the model's: {other_layer:?}"
    );
}

#[test]
fn a_decal_is_lit_and_shadowed_with_its_surface() {
    let Some(mut rig) = Rig::new() else { return };
    let floor = rig.mesh(&MeshData::plane(20.0));
    let cube = rig.mesh(&MeshData::cube(2.0));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    rig.ctx
        .world
        .spawn((Transform3D::at(vec3(0.0, 3.0, 0.0)), Model::new(cube)));
    rig.ctx.world.spawn((
        Transform3D::at(vec3(0.0, 0.2, 0.0)).rotated(down()),
        red_decal(vec3(14.0, 14.0, 1.0)),
    ));
    rig.ctx.lighting.shadows = true;
    rig.ctx.lighting.ambient_intensity = 0.1;
    rig.camera(vec3(0.0, 9.0, 8.0));
    let pixels = rig.draw();
    let shadowed = rig.at(&pixels, vec3(0.0, 0.0, 0.0));
    let lit = rig.at(&pixels, vec3(4.0, 0.0, 0.0));
    assert!(red(lit), "{lit:?}");
    assert!(
        shadowed[0] * 2 < lit[0],
        "the cube's shadow darkens the decal: {shadowed:?} vs {lit:?}"
    );
}

#[test]
fn a_normal_map_tilts_the_surface_towards_the_light() {
    let Some(mut rig) = Rig::new() else { return };
    let floor = rig.mesh(&MeshData::plane(20.0));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    // Normals leaning along +x and -x of the decal (which is +X / -X in the world).
    let lean = |sign: i32| {
        let x = (128.0 + sign as f32 * 90.0) as u8;
        [x, 128, 190, 255].repeat(4)
    };
    let (towards, away) = (lean(1), lean(-1));
    rig.decal_images(2, &[&towards, &away]);
    rig.ctx.lighting.sun_direction = vec3(-1.0, -0.6, 0.0); // the sun is on the +X side
    rig.ctx.lighting.ambient_intensity = 0.0;
    let flat = Decal3D {
        image: Decal3D::NO_IMAGE,
        color: Color::rgb(0.5, 0.5, 0.5),
        ..Default::default()
    };
    for (x, normal) in [(-4.0, Decal3D::NO_IMAGE), (0.0, 0), (4.0, 1)] {
        rig.ctx.world.spawn((
            Transform3D::at(vec3(x, 0.2, 0.0)).rotated(down()),
            Decal3D {
                size: vec3(3.0, 3.0, 1.0),
                normal_image: normal,
                ..flat
            },
        ));
    }
    rig.camera(vec3(0.0, 9.0, 4.0));
    let pixels = rig.draw();
    let level = rig.at(&pixels, vec3(-4.0, 0.0, 0.0))[0];
    let toward = rig.at(&pixels, vec3(0.0, 0.0, 0.0))[0];
    let aside = rig.at(&pixels, vec3(4.0, 0.0, 0.0))[0];
    assert!(
        toward > level + 8,
        "leaning to the sun is brighter: {toward} vs {level}"
    );
    assert!(aside + 8 < level, "leaning away is darker: {aside} vs {level}");
}

#[test]
fn on_a_device_without_a_spare_texture_slot_decals_are_skipped() {
    let Some(mut rig) = Rig::with_slots(false) else { return };
    let floor = rig.mesh(&MeshData::plane(20.0));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    rig.ctx.world.spawn((
        Transform3D::at(vec3(0.0, 0.2, 0.0)).rotated(down()),
        red_decal(vec3(2.0, 2.0, 1.0)),
    ));
    rig.camera(vec3(0.0, 8.0, 5.0));
    let pixels = rig.draw();
    let floor_color = rig.at(&pixels, vec3(0.0, 0.0, 0.0));
    assert!(
        neutral(floor_color) && floor_color[0] > 40,
        "the floor still draws: {floor_color:?}"
    );
}

#[test]
fn without_a_texture_slot_decals_are_skipped_not_broken() {
    // The stand-in code keeps the shader and its entry points valid.
    check_shader("no decals", &compose_shader_with("", false)).unwrap();
    assert!(!compose_shader_with("", false).contains("decal_array"));
    assert!(compose_shader_with("", true).contains("decal_array"));
}

/// Frame time with many decals, offscreen at 1280 x 720 (`cargo test --release
/// --features 3d -- --ignored --nocapture decal_cost`). Prints CPU collection
/// and GPU frame times.
#[test]
#[ignore = "benchmark: run in release"]
fn decal_cost() {
    let Some(mut rig) = Rig::sized(true, (1280, 720)) else {
        return;
    };
    let floor = rig.mesh(&MeshData::plane(60.0));
    rig.ctx.world.spawn((Transform3D::default(), Model::new(floor)));
    rig.ctx.lighting.shadows = false;
    rig.camera(vec3(0.0, 14.0, 12.0));
    let mut seed = 7u32;
    let mut random = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1 << 24) as f32
    };
    let mut ids = Vec::new();
    // Spread over 40 x 40 m, then piled on 5 x 5 m (cells hit their cap).
    for (count, spread) in [(0usize, 40.0), (256, 40.0), (1024, 40.0), (4096, 40.0), (4096, 5.0)] {
        if spread < 40.0 {
            for id in ids.drain(..) {
                rig.ctx.world.despawn(id);
            }
        }
        while ids.len() < count {
            let at = vec3(random() * spread - spread / 2.0, 0.2, random() * spread - spread / 2.0);
            ids.push(
                rig.ctx
                    .world
                    .spawn((Transform3D::at(at).rotated(down()), red_decal(vec3(2.0, 2.0, 1.0)))),
            );
        }
        for _ in 0..3 {
            rig.draw();
        }
        let start = std::time::Instant::now();
        let frames = 20;
        for _ in 0..frames {
            rig.draw();
        }
        let total = start.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        let mut frame = Frame3D::default();
        let cpu_start = std::time::Instant::now();
        for _ in 0..frames {
            frame.collect(
                &mut rig.ctx.world,
                &rig.ctx.gfx.assets3d,
                &SkinFrame::default(),
                &rig.ctx.camera3d,
                &rig.ctx.lighting,
            );
        }
        let cpu = cpu_start.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        println!(
            "{count:5} decals over {spread} m: collect {cpu:.3} ms, whole offscreen frame (collect, upload, draw, readback) {total:.2} ms, {} reach the screen",
            frame.decals.records.len()
        );
    }
}
