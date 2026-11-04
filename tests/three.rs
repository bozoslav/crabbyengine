//! 3D without a window: meshes, materials and models in headless runs,
//! glTF loading, and transforms as JSON.
#![cfg(feature = "3d")]

use crabbyengine::*;

// ANCHOR: three_d
/// Turns around its own Y axis, this many radians a second.
struct Spin(f32);

fn village(ctx: &mut Ctx) {
    let grass = ctx.create_material(MaterialDesc {
        roughness: 0.9,
        ..MaterialDesc::color(Color::hex(0x5E8C4A))
    });
    let ground = ctx.create_mesh(&MeshData::plane(100.0), grass);
    ctx.world.spawn((Transform3D::default(), Model::new(ground)));

    let house = ctx.load_mesh("models/house.glb"); // its walls and roof keep their materials
    for i in 0..5 {
        let at = vec3(i as f32 * 4.0 - 8.0, 0.0, -6.0);
        ctx.world.spawn((Transform3D::at(at), Model::new(house), Spin(0.5)));
    }

    ctx.camera3d.position = vec3(0.0, 6.0, 12.0);
    ctx.camera3d.look_at(vec3(0.0, 0.0, -4.0));
    ctx.lighting.sun_direction = vec3(-0.8, -0.6, -0.4); // late afternoon: long shadows
}

fn spin(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world.query::<(&mut Transform3D, &Spin)>().for_each(|(t, s)| {
        t.rotation *= Quat::from_rotation_y(s.0 * dt);
    });
}
// ANCHOR_END: three_d

#[test]
fn the_guide_village_has_five_spinning_houses() {
    let mut world = Game::new("village", 640, 480)
        .fixed_hz(60)
        .sim_system(spin)
        .run_headless(60, village);
    let mut houses = 0;
    world.query::<(&Transform3D, &Spin)>().for_each(|(t, _)| {
        houses += 1;
        // One second at half a radian a second.
        let (axis, angle) = t.rotation.to_axis_angle();
        assert!((angle - 0.5).abs() < 1e-3 && axis.y > 0.99, "{axis} {angle}");
    });
    assert_eq!(houses, 5);
}

// ANCHOR: sky
/// A misty morning: the sun low in the east, fog lying in the valleys.
fn misty_morning(ctx: &mut Ctx) {
    ctx.lighting.sun_direction = Sky::sun_direction_at(7.5);
    ctx.sky.haze = 2.0; // a paler sky, a bigger glow around the sun
    ctx.sky.fog_density = 0.03; // thick near the ground
    ctx.sky.fog_height = 0.0;
    ctx.sky.fog_falloff = 10.0; // and thin ten meters up
}
// ANCHOR_END: sky

#[test]
fn sky_settings_stay_as_set() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        assert_eq!(ctx.sky, Sky::default());
        assert!(ctx.sky.atmosphere);
        misty_morning(ctx);
        assert_eq!(ctx.sky.fog_density, 0.03);
        let to_sun = -ctx.lighting.sun_direction;
        assert!(to_sun.x > 0.0 && to_sun.y > 0.0, "morning: low in the east");
    });
}

// ANCHOR: post
/// A warm, glowing evening look.
fn golden_hour(ctx: &mut Ctx) {
    ctx.post.exposure = 0.3; // a third of a stop brighter
    ctx.post.bloom = 0.2; // highlights glow
    ctx.post.temperature = 0.4; // warmer
    ctx.post.saturation = 1.1;
    ctx.post.vignette = 0.3; // darker corners
    ctx.post.tonemap = Tonemap::AgX;
}
// ANCHOR_END: post

#[test]
fn post_settings_stay_as_set() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        assert_eq!(ctx.post, Post::default());
        golden_hour(ctx);
        assert_eq!(ctx.post.tonemap, Tonemap::AgX);
        let (w, h, pixels) = Post::neutral_lut(16);
        ctx.post.lut = Some(ctx.create_texture(w, h, &pixels));
        assert_eq!(ctx.post.lut.map(|t| (t.width(), t.height())), Some((256, 16)));
    });
}

#[test]
#[should_panic(expected = "2 to 64 steps")]
fn lookup_tables_have_two_to_64_steps() {
    let _ = Post::neutral_lut(1);
}

// ANCHOR: shader
/// Grass that sways in the wind: the higher a vertex, the more it moves.
const WIND: &str = "
fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let sway = sin(frame.time.x * 2.0 + v.position.x) * material.custom[0].x;
    out.position.x += sway * v.uv.y * v.uv.y;
    return out;
}";

fn meadow(ctx: &mut Ctx) {
    let wind = ctx.create_shader("wind", WIND);
    let grass = ctx.create_material(MaterialDesc {
        shader: Some(wind),
        double_sided: true,
        custom: [vec4(0.2, 0.0, 0.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO],
        ..MaterialDesc::color(Color::hex(0x7FA83C))
    });
    let blade = ctx.create_mesh(&MeshData::plane(0.5), grass);
    ctx.world.spawn((Transform3D::default(), Model::new(blade)));
}
// ANCHOR_END: shader

#[test]
fn shaders_and_their_materials_are_made() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        meadow(ctx);
        let water = ctx.load_shader("shaders/water.wgsl");
        assert_eq!(ctx.load_shader("shaders/water.wgsl"), water, "loaded once");
        let lake = ctx.create_material(MaterialDesc {
            shader: Some(water),
            blend: Blend::Transparent,
            ..Default::default()
        });
        ctx.update_material(
            lake,
            MaterialDesc {
                roughness: 0.1,
                ..Default::default()
            },
        );
    });
}

#[test]
fn transparent_shaders_can_read_the_scene_behind_them() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        let glass = ctx.create_shader(
            "glass",
            "fn surface(s: Surface) -> Surface {
                var out = s;
                let deep = scene_distance(s.screen) - distance(s.world, frame.camera.xyz);
                out.emissive = scene_color(s.screen + vec2<f32>(4.0, 0.0)) * exp(-deep);
                out.albedo = scene_position(s.screen) * 0.0 + s.albedo;
                return out;
            }",
        );
        ctx.create_material(MaterialDesc {
            shader: Some(glass),
            blend: Blend::Transparent,
            ..Default::default()
        });
    });
}

#[test]
#[should_panic(expected = "fix.wgsl")]
fn shader_errors_say_which_shader() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.create_shader("fix.wgsl", "fn surface(s: Surface) -> Surface { return 1.0; }");
    });
}

#[test]
#[should_panic(expected = "wasn't made by this game")]
fn materials_only_take_this_games_shaders() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let mut shader = None;
        Game::new("v", 1, 1).run_headless(0, |c| shader = Some(c.create_shader("s", "")));
        ctx.create_material(MaterialDesc {
            shader,
            ..Default::default()
        });
    });
}

#[test]
fn loading_a_model_twice_gives_the_same_mesh() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let a = ctx.load_mesh("models/house.glb");
        let b = ctx.load_mesh("models/house.glb");
        assert_eq!(a, b);
        let white = ctx.default_material();
        let cube = ctx.create_mesh(&MeshData::cube(1.0), white);
        assert_ne!(a, cube);
    });
}

#[test]
#[should_panic(expected = "models/no-such-model.glb")]
fn a_missing_model_names_the_file() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let _ = ctx.load_mesh("models/no-such-model.glb");
    });
}

#[test]
#[should_panic(expected = "roughness and metallic go from 0 to 1")]
fn materials_check_their_numbers() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let _ = ctx.create_material(MaterialDesc {
            metallic: 1.5,
            ..Default::default()
        });
    });
}

#[test]
#[should_panic(expected = "2 colors for 3 corners")]
fn vertex_colors_come_one_per_corner() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let tri = MeshData {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            indices: vec![0, 1, 2],
            colors: vec![Color::RED, Color::WHITE],
            ..Default::default()
        };
        let white = ctx.default_material();
        let _ = ctx.create_mesh(&tri, white);
    });
}

#[test]
#[should_panic(expected = "3 per triangle")]
fn meshes_check_their_triangles() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let broken = MeshData {
            positions: vec![Vec3::ZERO; 3],
            indices: vec![0, 1],
            ..Default::default()
        };
        let white = ctx.default_material();
        let _ = ctx.create_mesh(&broken, white);
    });
}

// ANCHOR: terrain
/// Rolling hills: 3 x 3 tiles of 128 m, each from 65 x 65 heights 2 m
/// apart, sharing their edges, with a pond-sized hole in the middle tile.
/// Grass in the valleys, dirt higher up and rock on the tops.
fn hills(ctx: &mut Ctx) {
    let height = |x: f32, z: f32| (x * 0.02).sin() * 6.0 + (z * 0.03).cos() * 4.0;
    let ground = hill_ground(ctx, height);
    let (n, size) = (65, 128.0);
    for tz in -1..=1 {
        for tx in -1..=1 {
            let corner = vec3(tx as f32 * size, 0.0, tz as f32 * size);
            let heights: Vec<f32> = (0..n * n)
                .map(|i| {
                    let (x, z) = ((i % n) as f32 * 2.0 + corner.x, (i / n) as f32 * 2.0 + corner.z);
                    let hole = tx == 0 && tz == 0 && (vec2(x, z) - vec2(64.0, 64.0)).length() < 6.0;
                    if hole { f32::NAN } else { height(x, z) }
                })
                .collect();
            let tile = ctx.create_terrain_tile(&heights, size, ground);
            ctx.world.spawn((Transform3D::at(corner), tile));
        }
    }
    ctx.camera3d.position = vec3(64.0, 30.0, 220.0);
    ctx.camera3d.look_at(vec3(64.0, 0.0, 64.0));
}
// ANCHOR_END: terrain

// ANCHOR: terrain_material
/// Three ground layers, speckled so they don't look flat (in a game,
/// `ctx.load_texture_array` from image files), and one weight map over
/// all nine tiles from the heights: red grass, green dirt, blue rock.
fn hill_ground(ctx: &mut Ctx, height: impl Fn(f32, f32) -> f32) -> Material {
    let speckled = |base: [u8; 3]| -> Vec<u8> {
        (0..64 * 64u32)
            .flat_map(|i| {
                let shade = (i.wrapping_mul(2_654_435_761) >> 27) as u8; // 0..31
                [
                    base[0].saturating_add(shade),
                    base[1].saturating_add(shade),
                    base[2].saturating_add(shade),
                    255,
                ]
            })
            .collect()
    };
    let (grass, dirt, rock) = (
        speckled([70, 120, 50]),
        speckled([110, 85, 60]),
        speckled([120, 120, 115]),
    );
    let layers = ctx.create_texture_array(64, 64, &[&grass, &dirt, &rock], TextureKind::Color);
    let (cells, area) = (96, 384.0); // 4 m a pixel, from (-128, -128)
    let weights: Vec<u8> = (0..cells * cells)
        .flat_map(|i| {
            let x = (i % cells) as f32 * area / cells as f32 - 128.0;
            let z = (i / cells) as f32 * area / cells as f32 - 128.0;
            let up = ((height(x, z) + 10.0) / 20.0).clamp(0.0, 1.0); // 0 low, 1 high
            let rock = ((up - 0.7) * 4.0).clamp(0.0, 1.0);
            let dirt = (1.0 - (up - 0.5).abs() * 4.0).clamp(0.0, 1.0);
            let grass = 1.0 - rock.max(dirt);
            [grass, dirt, rock, 0.0].map(|w| (w * 255.0) as u8)
        })
        .collect();
    let weights = ctx.create_texture_array(cells, cells, &[&weights], TextureKind::Data);
    let desc = TerrainMaterialDesc {
        layer_size: [6.0, 4.0, 8.0, 4.0, 4.0, 4.0, 4.0, 4.0],
        ..TerrainMaterialDesc::new(layers, weights, vec2(-128.0, -128.0), vec2(area, area))
    };
    ctx.create_terrain_material(&desc)
}
// ANCHOR_END: terrain_material

#[test]
fn the_guide_hills_are_nine_tiles_of_four_details() {
    let mut world = Game::new("hills", 640, 480).run_headless(1, hills);
    let mut meshes = std::collections::HashSet::new();
    world.query::<&TerrainTile>().for_each(|tile| {
        assert_eq!(tile.lod_distance, 128.0);
        meshes.extend(tile.lods.map(|m| m));
    });
    assert_eq!(meshes.len(), 9 * TERRAIN_LODS, "every tile and level has its own mesh");
}

// ANCHOR: effect
/// Under water, light fades to deep blue with distance (and the sky above
/// is gone). `material.custom[0].x` is the water's height.
const UNDERWATER: &str = "
fn effect(p: Pixel) -> vec3<f32> {
    if frame.camera.y > material.custom[0].x {
        return p.color; // above the water: nothing changes
    }
    let deep = vec3<f32>(0.02, 0.12, 0.18);
    let fade = 1.0 - exp(-p.distance * 0.08);
    return mix(p.color * vec3<f32>(0.6, 0.9, 1.0), deep, fade);
}";

fn dive(ctx: &mut Ctx) {
    let underwater = ctx.create_effect("underwater.wgsl", UNDERWATER);
    let lake = ctx.create_material(MaterialDesc {
        custom: [vec4(2.0, 0.0, 0.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO], // water at y = 2
        ..Default::default()
    });
    ctx.effects.push(ScreenEffect::new(underwater).with_material(lake));
}
// ANCHOR_END: effect

// ANCHOR: mirror
/// A still pond: its color is the world mirrored in it, which a target
/// camera below the surface draws into `custom_texture0`, upside down.
const MIRROR: &str = "
fn surface(s: Surface) -> Surface {
    var out = s;
    let uv = s.screen * frame.screen.zw;
    out.emissive = textureSample(custom_texture0, material_sampler, vec2<f32>(uv.x, 1.0 - uv.y)).rgb * 0.9;
    out.albedo = vec3<f32>(0.0);
    return out;
}";

fn pond(ctx: &mut Ctx) {
    let size = ctx.window_size();
    let reflection = ctx.create_render_target(size.x as u32 / 2, size.y as u32 / 2);
    let mirror = ctx.create_shader("mirror", MIRROR);
    let water = ctx.create_material(MaterialDesc {
        shader: Some(mirror),
        custom_textures: [Some(reflection), None],
        ..Default::default()
    });
    let surface = ctx.create_mesh(&MeshData::plane(20.0), water);
    ctx.world.spawn((Transform3D::default(), Model::new(surface)));
    ctx.target_cameras
        .push(TargetCamera::reflection(ctx.camera3d, reflection, 0.0));
}

/// Every frame: the mirrored camera follows the real one.
fn follow_reflections(ctx: &mut Ctx) {
    for t in &mut ctx.target_cameras {
        *t = TargetCamera::reflection(ctx.camera3d, t.target, 0.0);
    }
}
// ANCHOR_END: mirror

// ANCHOR: compute
/// Ripples: each texel moves toward the average of its neighbours,
/// overshooting like water. Red is the height now, green the height a
/// frame ago; `material.custom[0]` drops a stone (uv x, uv y, strength).
const RIPPLES: &str = "
fn compute(c: Cell) -> vec4<f32> {
    let around = cell_at(c.pixel + vec2<i32>(1, 0)).x + cell_at(c.pixel - vec2<i32>(1, 0)).x
        + cell_at(c.pixel + vec2<i32>(0, 1)).x + cell_at(c.pixel - vec2<i32>(0, 1)).x;
    var height = (around * 0.5 - c.value.y) * 0.985;
    let drop = material.custom[0];
    if drop.z > 0.0 && distance(c.uv, drop.xy) * f32(c.size.x) < 3.0 {
        height -= drop.z;
    }
    return vec4<f32>(height, c.value.x, 0.0, 0.0);
}";

/// The simulated heights, for a water material to read, and the material
/// that drops stones into them.
fn ripples(ctx: &mut Ctx) -> (Texture, Material) {
    let heights = ctx.create_render_target(256, 256);
    let ripples = ctx.create_compute("ripples", RIPPLES);
    let stones = ctx.create_material(MaterialDesc::default());
    ctx.compute_passes
        .push(ComputePass::new(ripples, heights).with_material(stones));
    (heights, stones)
}

/// Drops a stone at `uv` (0 to 1 across the water) this frame; `strength`
/// 0 drops none.
fn drop_stone(ctx: &mut Ctx, stones: Material, uv: Vec2, strength: f32) {
    let stone = uv.extend(strength).extend(0.0);
    ctx.update_material(
        stones,
        MaterialDesc {
            custom: [stone, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO],
            ..Default::default()
        },
    );
}
// ANCHOR_END: compute

#[test]
fn the_guide_ripples_run_on_a_render_target() {
    Game::new("ripples", 64, 64).run_headless(1, |ctx| {
        let (heights, stones) = ripples(ctx);
        drop_stone(ctx, stones, vec2(0.5, 0.5), 1.0);
        assert!(ctx.is_render_target(heights));
        assert_eq!(ctx.compute_passes.len(), 1);
        assert_eq!(ctx.compute_passes[0].material, Some(stones));
    });
}

#[test]
#[should_panic(expected = "ripples.wgsl")]
fn compute_errors_say_which_pass() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.create_compute("ripples.wgsl", "fn compute(c: Cell) -> f32 { return 1.0; }");
    });
}

#[test]
fn the_guide_pond_mirrors_the_camera() {
    Game::new("pond", 64, 64).run_headless(0, |ctx| {
        pond(ctx);
        ctx.camera3d.position = vec3(3.0, 5.0, 1.0);
        follow_reflections(ctx);
        let below = ctx.target_cameras[0];
        assert!(ctx.is_render_target(below.target));
        assert_eq!(below.target.width(), 32);
        assert_eq!(below.camera.position, vec3(3.0, -5.0, 1.0));
        assert_eq!(below.clip_plane, Some(vec4(0.0, 1.0, 0.0, 0.0)));
    });
}

#[test]
#[should_panic(expected = "render target size must not be zero")]
fn render_targets_need_a_size() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.create_render_target(0, 16);
    });
}

#[test]
#[should_panic(expected = "not updated with pixels")]
fn render_targets_take_no_pixels() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let target = ctx.create_render_target(1, 1);
        ctx.update_texture(&target, &[0; 4]);
    });
}

#[test]
fn the_guide_effect_is_added_once() {
    let mut effects = Vec::new();
    Game::new("dive", 64, 64).run_headless(2, |ctx| {
        dive(ctx);
        effects = ctx.effects.clone();
    });
    assert_eq!(effects.len(), 1);
    assert!(effects[0].material.is_some());
}

#[test]
fn effect_errors_say_which_effect() {
    let err = std::panic::catch_unwind(|| {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            let _ = ctx.create_effect("broken.wgsl", "fn effect(p: Pixel) -> vec3<f32> { return p; }");
        });
    })
    .unwrap_err();
    let message = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("broken.wgsl"), "{message}");
}

#[test]
fn effects_read_every_pixel_field() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let all = ctx.create_effect(
            "all.wgsl",
            "fn effect(p: Pixel) -> vec3<f32> {
                let far = select(p.distance, 0.0, p.sky);
                let near = scene_color(p.screen + vec2<f32>(1.0, 0.0));
                return p.color + near * 0.1 + p.world * 0.0 + vec3<f32>(p.uv, far * material.custom[0].x);
            }",
        );
        assert_ne!(
            all,
            ctx.create_effect("other.wgsl", "fn effect(p: Pixel) -> vec3<f32> { return p.color; }")
        );
    });
}

#[test]
fn texture_arrays_hold_one_image_per_layer() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let red = [255, 0, 0, 255].repeat(4);
        let blue = [0, 0, 255, 255].repeat(4);
        let tints = ctx.create_texture_array(2, 2, &[&red, &blue], TextureKind::Color);
        assert_eq!((tints.width(), tints.height(), tints.layers()), (2, 2, 2));
        let wind = ctx.create_shader(
            "layers.wgsl",
            "fn surface(s: Surface) -> Surface { var o = s; o.albedo = textureSample(custom_array0, material_sampler, s.uv, 1).rgb; return o; }",
        );
        let mut custom_arrays = [None; 3];
        custom_arrays[0] = Some(tints);
        let _ = ctx.create_material(MaterialDesc { shader: Some(wind), custom_arrays, ..Default::default() });
    });
}

#[test]
#[should_panic(expected = "layer 1 of a 2 x 2 texture array needs 16 bytes")]
fn texture_array_layers_are_all_one_size() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let _ = ctx.create_texture_array(2, 2, &[&[0; 16], &[0; 12]], TextureKind::Data);
    });
}

#[test]
fn terrain_materials_check_their_layers() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let image = [128u8; 16];
        let five = ctx.create_texture_array(2, 2, &[&image[..]; 5], TextureKind::Color);
        let one_weight = ctx.create_texture_array(2, 2, &[&image], TextureKind::Data);
        let two_weights = ctx.create_texture_array(2, 2, &[&image[..]; 2], TextureKind::Data);
        let nine = ctx.create_texture_array(2, 2, &[&image[..]; 9], TextureKind::Color);
        let desc = |layers, weights| TerrainMaterialDesc::new(layers, weights, Vec2::ZERO, vec2(10.0, 10.0));
        let err = |ctx: &mut Ctx, d: TerrainMaterialDesc| ctx.try_create_terrain_material(&d).unwrap_err();
        assert!(err(ctx, desc(five, one_weight)).contains("need 2 weight images"));
        assert!(err(ctx, desc(nine, two_weights)).contains("at most 8 layers"));
        let no_size = TerrainMaterialDesc {
            weights_size: Vec2::ZERO,
            ..desc(five, two_weights)
        };
        assert!(err(ctx, no_size).contains("size must be above 0"));
        let few_normals = TerrainMaterialDesc {
            normals: Some(one_weight),
            ..desc(five, two_weights)
        };
        assert!(err(ctx, few_normals).contains("need 5 normal maps"));
        // Two terrain materials share the one built-in shader.
        let a = ctx.create_terrain_material(&desc(five, two_weights));
        let b = ctx.create_terrain_material(&desc(five, two_weights));
        assert_ne!(a, b);
    });
}

#[test]
#[should_panic(expected = "size must be above 0")]
fn terrain_tiles_need_a_size() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let white = ctx.default_material();
        let _ = ctx.create_terrain_tile(&[0.0; 4], 0.0, white);
    });
}

#[test]
fn transforms_are_reflected_as_json() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let e = ctx.world.spawn((Transform3D::at(vec3(1.0, 2.0, 3.0)).scaled(2.0),));
        let json = ctx.world.get_json(e, "Transform3D").unwrap();
        assert_eq!(json["position"], serde_json::json!([1.0, 2.0, 3.0]));
        assert_eq!(json["scale"], serde_json::json!([2.0, 2.0, 2.0]));
        ctx.world
            .set_json(e, "Transform3D", serde_json::json!({"position": [0.0, 5.0, 0.0]}))
            .unwrap();
        assert_eq!(ctx.world.get::<Transform3D>(e).unwrap().position, vec3(0.0, 5.0, 0.0));
    });
}
