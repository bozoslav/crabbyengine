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
fn hills(ctx: &mut Ctx) {
    let grass = ctx.create_material(MaterialDesc {
        roughness: 0.95,
        ..MaterialDesc::color(Color::hex(0x5A8A3C))
    });
    let (n, size) = (65, 128.0);
    let height = |x: f32, z: f32| (x * 0.02).sin() * 6.0 + (z * 0.03).cos() * 4.0;
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
            let tile = ctx.create_terrain_tile(&heights, size, grass);
            ctx.world.spawn((Transform3D::at(corner), tile));
        }
    }
    ctx.camera3d.position = vec3(64.0, 30.0, 220.0);
    ctx.camera3d.look_at(vec3(64.0, 0.0, 64.0));
}
// ANCHOR_END: terrain

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
