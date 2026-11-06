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

// ANCHOR: mesh_shadows
/// A meadow: thousands of grass tufts, too small for their shadows to show.
fn plant_meadow(ctx: &mut Ctx, tuft: Mesh) {
    ctx.set_mesh_shadows(tuft, false); // no shadow map work for grass
    for i in 0..1000 {
        let at = vec3((i % 40) as f32 * 0.5, 0.0, (i / 40) as f32 * 0.5);
        ctx.world.spawn((Transform3D::at(at), Model::new(tuft)));
    }
}
// ANCHOR_END: mesh_shadows

#[test]
fn grass_can_cast_no_shadow() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        let material = ctx.default_material();
        let tuft = ctx.create_mesh(&MeshData::cube(0.1), material);
        assert!(ctx.mesh_shadows(tuft), "meshes cast shadows at first");
        plant_meadow(ctx, tuft);
        assert!(!ctx.mesh_shadows(tuft));
        ctx.set_mesh_shadows(tuft, true);
        assert!(ctx.mesh_shadows(tuft));
    });
}

// ANCHOR: shadow_mesh
/// A detailed tree with a cheap shadow, a flat sign with none, and a stand-in
/// shadow for a row of fence pieces that cast none themselves.
fn yard(ctx: &mut Ctx, tree: Mesh, blob: Mesh, sign: Mesh, fence: Mesh, fence_shadow: Mesh) {
    ctx.world
        .spawn((Transform3D::default(), Model::new(tree), ShadowMesh(Some(blob))));
    ctx.world
        .spawn((Transform3D::at(vec3(4.0, 0.0, 0.0)), Model::new(sign), ShadowMesh(None)));
    for i in 0..10 {
        let at = vec3(i as f32, 0.0, 6.0);
        ctx.world
            .spawn((Transform3D::at(at), Model::new(fence), ShadowMesh(None)));
    }
    // Only a shadow: no Model.
    ctx.world
        .spawn((Transform3D::at(vec3(0.0, 0.0, 6.0)), ShadowMesh(Some(fence_shadow))));
}
// ANCHOR_END: shadow_mesh

#[test]
fn shadow_meshes_round_trip_through_scenes() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        let material = ctx.default_material();
        let mesh = ctx.create_mesh(&MeshData::cube(1.0), material);
        yard(ctx, mesh, mesh, mesh, mesh, mesh);
        let only_shadows = ctx.world.query::<&ShadowMesh>().without::<Model>().iter().count();
        assert_eq!(only_shadows, 1);
        let none = ctx
            .world
            .query::<&ShadowMesh>()
            .iter()
            .filter(|s| s.0.is_none())
            .count();
        assert_eq!(none, 11);
        // No shadow, as JSON and back.
        let sign = ctx.world.spawn((Transform3D::default(), ShadowMesh(None)));
        let json = ctx.world.entity_json(sign).unwrap();
        assert_eq!(json["ShadowMesh"], serde_json::json!({ "mesh": null }), "{json}");
        let other = ctx.world.spawn((Transform3D::default(),));
        ctx.world
            .set_json(other, "ShadowMesh", json["ShadowMesh"].clone())
            .unwrap();
        assert_eq!(ctx.world.get::<ShadowMesh>(other), Some(&ShadowMesh(None)));
        let unloaded = ctx
            .world
            .set_json(other, "ShadowMesh", serde_json::json!({ "mesh": "nope.glb" }))
            .unwrap_err();
        assert!(unloaded.to_string().contains("isn't loaded"), "{unloaded}");
    });
}

// ANCHOR: distance_fog
/// A big open world fading out far away, glowing toward the sun.
fn far_haze(ctx: &mut Ctx) {
    ctx.sky.fog_start = 500.0; // clear up to half a kilometre
    ctx.sky.fog_end = 1200.0; // and gone in the haze past 1.2 km
    ctx.sky.fog_sun_glow = 0.06; // the haze glows toward the sun
    ctx.sky.fog_sun_focus = 0.7; // most of all close around it
}
// ANCHOR_END: distance_fog

// ANCHOR: light_shafts
/// Beams of morning sun through the trees.
fn sunbeams(ctx: &mut Ctx) {
    ctx.sky.fog_density = 0.03; // low mist to light up
    ctx.sky.fog_sun_glow = 0.1; // glowing in the sun
    ctx.sky.light_shafts = 1.0; // but not in the shade
    ctx.lighting.shadows = true; // shafts follow the sun's shadows
    ctx.post.anti_aliasing = AntiAliasing::Taa; // and smooth out their grain
}
// ANCHOR_END: light_shafts

// ANCHOR: light_shadow_atlas
/// A street at night: a sharp shadow for the lamp beside the player,
/// rough ones for the street lights down the road.
fn street_lights(ctx: &mut Ctx) {
    ctx.lighting.light_shadow_atlas = 8192; // room for more shadow views
    ctx.lighting.light_shadow_views = 24; // as many as there can be
    let lamp = PointLight3D {
        color: Color::rgb(1.0, 0.8, 0.6),
        intensity: 40.0,
        range: 12.0,
        shadows: true,
        shadow_resolution: 2048, // always sharp
    };
    ctx.world.spawn((Transform3D::at(vec3(0.0, 3.0, 0.0)), lamp));
    for i in 0..18 {
        let x = -20.0 + 2.5 * i as f32;
        let at = Transform3D::at(vec3(x, 5.0, -30.0)).looking_at(vec3(x, 0.0, -29.9));
        // Sized by how big they look: smaller the farther they are.
        let street_light = SpotLight3D {
            intensity: 60.0,
            range: 8.0,
            shadows: true,
            ..Default::default()
        };
        ctx.world.spawn((at, street_light));
    }
}
// ANCHOR_END: light_shadow_atlas

#[test]
fn light_shadow_atlas_settings_stay_as_set() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        let l = ctx.lighting;
        assert_eq!(
            (l.light_shadow_atlas, l.light_shadow_views, l.light_shadow_resolution),
            (4096, 12, 1024)
        );
        street_lights(ctx);
        assert_eq!(ctx.lighting.light_shadow_atlas, 8192);
        let hinted: Vec<u32> = ctx
            .world
            .query::<&PointLight3D>()
            .iter()
            .map(|p| p.shadow_resolution)
            .collect();
        assert_eq!(hinted, [2048]);
        assert_eq!(ctx.world.query::<&SpotLight3D>().iter().count(), 18);
    });
}

#[test]
fn sky_settings_stay_as_set() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        assert_eq!(ctx.sky, Sky::default());
        assert!(ctx.sky.atmosphere);
        misty_morning(ctx);
        assert_eq!(ctx.sky.fog_density, 0.03);
        assert_eq!(
            (ctx.sky.fog_start, ctx.sky.fog_end),
            (0.0, 0.0),
            "no distance fog by default"
        );
        far_haze(ctx);
        assert_eq!(ctx.sky.fog_end, 1200.0);
        assert_eq!(ctx.sky.fog_density, 0.03, "both kinds of fog at once");
        assert_eq!(ctx.sky.light_shafts, 0.0, "no light shafts by default");
        sunbeams(ctx);
        assert_eq!(ctx.sky.light_shafts, 1.0);
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
fn loaded_materials_are_found_by_name_and_can_be_changed() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert!(ctx.loaded_materials("walls").is_empty(), "nothing loaded yet");
        let _ = ctx.load_mesh("models/house.glb");
        let walls = ctx.loaded_materials("walls");
        assert_eq!(walls.len(), 1);
        let roof = ctx.loaded_materials("roof");
        assert_eq!(roof.len(), 1);
        assert_ne!(walls[0].0, roof[0].0);
        // Loading the file again makes no new materials.
        let _ = ctx.load_mesh("models/house.glb");
        assert_eq!(ctx.loaded_materials("walls").len(), 1);
        // ANCHOR: loaded_materials
        // The house's walls, drawn with a shader of our own: keep what they
        // were made from (textures, colors) and change the shader.
        let weathered = ctx.create_shader(
            "weathered.wgsl",
            "fn surface(s: Surface) -> Surface { var o = s; o.albedo *= 0.8; o.roughness = 0.95; return o; }",
        );
        for (walls, desc) in ctx.loaded_materials("walls") {
            ctx.update_material(
                walls,
                MaterialDesc {
                    shader: Some(weathered),
                    ..desc
                },
            );
        }
        // ANCHOR_END: loaded_materials
        assert!(ctx.loaded_materials("no such material").is_empty());
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

// ANCHOR: ground_height
/// Trees standing on a 128 m tile: each at the ground's height under it,
/// none over a hole.
fn plant_trees(ctx: &mut Ctx, heights: &[f32], corner: Vec3, tree: Mesh) {
    let ground = Heightfield::new(heights, 128.0);
    for i in 0..20 {
        let (x, z) = ((i * 37 % 128) as f32, (i * 71 % 128) as f32);
        if let Some(y) = ground.height_at(x, z) {
            ctx.world
                .spawn((Transform3D::at(corner + vec3(x, y, z)), Model::new(tree)));
        }
    }
}
// ANCHOR_END: ground_height

#[test]
fn the_guide_trees_stand_on_the_ground() {
    let mut world = Game::new("t", 1, 1).run_headless(0, |ctx| {
        let material = ctx.default_material();
        let tree = ctx.create_mesh(&MeshData::cube(1.0), material);
        // Flat at 3 m, with the first cell's corner cut out.
        let mut heights = vec![3.0; 65 * 65];
        heights[0] = f32::NAN;
        plant_trees(ctx, &heights, vec3(128.0, 0.0, 0.0), tree);
    });
    let trees: Vec<Vec3> = world.query::<&Transform3D>().iter().map(|t| t.position).collect();
    assert_eq!(trees.len(), 19, "the tree at the corner is over the hole");
    assert!(trees.iter().all(|p| (p.y - 3.0).abs() < 1e-4 && p.x >= 128.0));
}

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

// ANCHOR: streaming
/// Ground that loads around the camera: tiles 128 m wide within 600 m,
/// built on worker threads, freed past 800 m.
struct Ground {
    cells: CellStreamer,
    tiles: std::collections::HashMap<(i32, i32), Tile>,
    material: Material,
}

enum Tile {
    Building(Job<TerrainTileData>),
    Live(Entity, TerrainTile),
}

fn hill(x: f32, z: f32) -> f32 {
    (x * 0.01).sin() * (z * 0.013).cos() * 20.0
}

impl Ground {
    fn new(material: Material) -> Self {
        Self {
            cells: CellStreamer::new(128.0, 600.0, 800.0),
            tiles: Default::default(),
            material,
        }
    }

    /// Every frame: start up to 4 tiles, drop the far ones, and put up to
    /// 2 finished ones in the world.
    fn update(&mut self, ctx: &mut Ctx) {
        self.cells.update(ctx.camera3d.position, 4);
        for &cell in self.cells.to_load() {
            let corner = self.cells.corner(cell);
            let job = Job::spawn(move || {
                // 65 x 65 heights, 2 m apart.
                let heights: Vec<f32> = (0..65 * 65)
                    .map(|i| hill(corner.x + (i % 65) as f32 * 2.0, corner.z + (i / 65) as f32 * 2.0))
                    .collect();
                TerrainTileData::new(&heights, 128.0)
            });
            self.tiles.insert(cell, Tile::Building(job));
        }
        for cell in self.cells.to_drop() {
            if let Some(Tile::Live(entity, tile)) = self.tiles.remove(cell) {
                ctx.world.despawn(entity);
                ctx.free_terrain_tile(tile);
            }
        }
        let mut uploads = 0;
        for (&cell, tile) in &mut self.tiles {
            if let Tile::Building(job) = tile
                && uploads < 2
                && let Some(data) = job.take()
            {
                let made = ctx.upload_terrain_tile(&data, self.material);
                let entity = ctx.world.spawn((Transform3D::at(self.cells.corner(cell)), made));
                *tile = Tile::Live(entity, made);
                uploads += 1;
            }
        }
    }
}
// ANCHOR_END: streaming

#[test]
fn the_guide_ground_streams_in_and_out() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let material = ctx.default_material();
        let mut ground = Ground::new(material);
        let live = |g: &Ground| g.tiles.values().filter(|t| matches!(t, Tile::Live(..))).count();
        // Until everything around the start is in.
        for _ in 0..10_000 {
            ground.update(ctx);
            if ground.tiles.len() == ground.cells.loaded_count()
                && live(&ground) == ground.tiles.len()
                && ground.cells.to_load().is_empty()
            {
                break;
            }
            std::thread::yield_now();
        }
        let first = live(&ground);
        assert!(first > 20, "{first} tiles");
        // Far away: the old tiles are freed.
        ctx.camera3d.position = vec3(10_000.0, 0.0, 0.0);
        ground.update(ctx);
        assert_eq!(ground.cells.to_drop().len(), first);
        assert!(ground.tiles.keys().all(|&(x, _)| x > 50));
    });
}

#[test]
fn models_of_loaded_meshes_save_and_load_by_file_name() {
    let file = "test-three-scene.json";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(file);
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let house = ctx.load_mesh("models/house.glb");
        let red = Color::rgb(1.0, 0.0, 0.0);
        let e = ctx
            .world
            .spawn((Transform3D::at(vec3(1.0, 2.0, 3.0)), Model::new(house).with_color(red)));
        assert_eq!(ctx.world.get_json(e, "Model").unwrap()["mesh"], "models/house.glb");
        let white = ctx.default_material();
        let cube = ctx.create_mesh(&MeshData::cube(1.0), white);
        ctx.world.spawn((Transform3D::default(), Model::new(cube)));
        ctx.save_scene(file).unwrap();
    });
    let result = std::panic::catch_unwind(|| {
        Game::new("t", 1, 1).run_headless(0, |ctx| {
            ctx.load_scene(file).unwrap();
            let house = ctx.load_mesh("models/house.glb");
            let models: Vec<(Transform3D, Model)> = ctx
                .world
                .query::<(&Transform3D, &Model)>()
                .iter()
                .map(|(t, m)| (*t, *m))
                .collect();
            assert_eq!(models.len(), 1, "the made-in-code cube has no file to name");
            assert_eq!(models[0].1.mesh, house, "loaded with the scene");
            assert_eq!(models[0].1.color, Color::rgb(1.0, 0.0, 0.0));
            assert_eq!(models[0].0.position, vec3(1.0, 2.0, 3.0));
        });
    });
    let _ = std::fs::remove_file(path);
    result.unwrap();
}

#[test]
fn models_name_meshes_that_must_be_files() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let e = ctx.world.spawn((Transform3D::default(),));
        let err = ctx
            .world
            .set_json(e, "Model", serde_json::json!({ "color": Color::WHITE }));
        assert!(err.unwrap_err().to_string().contains("\"mesh\""));
        let err = ctx
            .world
            .set_json(e, "Model", serde_json::json!({ "mesh": "nope.glb" }));
        assert!(err.unwrap_err().to_string().contains("ctx.load_mesh(\"nope.glb\")"));
    });
}

#[test]
fn occlusion_culling_is_off_until_turned_on() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert!(!ctx.camera3d.occlusion_culling);
        // ANCHOR: occlusion
        // A town or a forest: skip what houses, hills and trunks hide.
        ctx.camera3d.occlusion_culling = true;
        // ANCHOR_END: occlusion
        // Headless, there's no depth to cull with: everything in view is drawn.
        let white = ctx.default_material();
        let mesh = ctx.create_mesh(&MeshData::cube(1.0), white);
        ctx.world.spawn((Transform3D::default(), Model::new(mesh)));
        ctx.camera3d.position = vec3(0.0, 0.0, 5.0);
        ctx.camera3d.look_at(Vec3::ZERO);
        assert_eq!(ctx.render_stats().occluded, 0);
    });
}

#[test]
fn a_three_color_ambient_is_set_on_the_lighting() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert_eq!(ctx.lighting.equator_color, None);
        // ANCHOR: trilight
        // Without an atmosphere: a blue sky, a green-grey horizon and brown ground.
        ctx.sky.atmosphere = false;
        ctx.lighting.sky_color = Color::hex(0x8FB3D9);
        ctx.lighting.equator_color = Some(Color::hex(0x8C9A86));
        ctx.lighting.ground_color = Color::hex(0x4D4135);
        // ANCHOR_END: trilight
        assert!(ctx.lighting.equator_color.is_some());
    });
}

#[test]
fn lod_groups_pick_levels_by_screen_height() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let white = ctx.default_material();
        let high = ctx.create_mesh(&MeshData::sphere(1.0, 32), white);
        let low = ctx.create_mesh(&MeshData::cube(2.0), white);
        // ANCHOR: lod
        // A 2 m object: the sphere until it's 25% of the screen's height,
        // the cube down to 3%, then nothing. The sphere crossfades into the
        // cube over the top quarter of its range.
        let group = LodGroup::new(
            2.0,
            vec![
                LodLevel::new(0.25, vec![LodRenderer::new(high)]).with_fade(0.25),
                LodLevel::new(0.03, vec![LodRenderer::new(low)]),
            ],
        );
        let e = ctx.world.spawn((Transform3D::default(), group));
        // Lower quality: drop detail at 70% of the distance.
        ctx.lod.bias = 0.7;
        // ANCHOR_END: lod
        let group = ctx.world.get::<LodGroup>(e).unwrap().clone();
        let at = |ctx: &mut Ctx, d: f32| {
            ctx.camera3d.position = vec3(0.0, 0.0, d);
            ctx.camera3d.look_at(Vec3::ZERO);
            group.pick_for(&Transform3D::default(), &ctx.camera3d, &ctx.lod)
        };
        ctx.lod.bias = 1.0;
        assert_eq!(at(ctx, 2.0), LodPick::Level(0));
        assert_eq!(at(ctx, 10.0), LodPick::Level(1));
        assert_eq!(at(ctx, 100.0), LodPick::Culled);
        assert!(matches!(
            at(ctx, 6.4),
            LodPick::Fade {
                from: 0,
                to: Some(1),
                ..
            }
        ));
        // A distance to the screen height and back.
        let fov = ctx.camera3d.fov_y;
        let d = LodGroup::distance_at(2.0, 0.25, fov);
        assert!((LodGroup::screen_height_at(2.0, d, fov) - 0.25).abs() < 1e-5);
        ctx.lod.bias = 0.7;
        assert_eq!(at(ctx, 5.0), LodPick::Level(1), "the bias drops detail sooner");
    });
}

#[test]
fn lod_groups_of_loaded_meshes_save_and_load_by_file_name() {
    let file = "test-lod-scene.json";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(file);
    let group = |house: Mesh| {
        LodGroup::new(
            8.0,
            vec![
                LodLevel::new(0.3, vec![LodRenderer::new(house).with_color(Color::rgb(1.0, 0.0, 0.0))]).with_fade(0.2),
                LodLevel::new(
                    0.05,
                    vec![LodRenderer::new(house).with_offset(Transform3D::at(vec3(0.0, 1.0, 0.0)))],
                )
                .with_shadow(LodShadow::Casters(vec![])),
            ],
        )
        .with_center(vec3(0.0, 4.0, 0.0))
    };
    Game::new("t", 1, 1).run_headless(0, move |ctx| {
        let house = ctx.load_mesh("models/house.glb");
        ctx.world.spawn((Transform3D::at(vec3(1.0, 2.0, 3.0)), group(house)));
        // A group with a mesh made in code has no file to name: not saved.
        let white = ctx.default_material();
        let cube = ctx.create_mesh(&MeshData::cube(1.0), white);
        ctx.world.spawn((
            Transform3D::default(),
            LodGroup::new(1.0, vec![LodLevel::new(0.1, vec![LodRenderer::new(cube)])]),
        ));
        ctx.save_scene(file).unwrap();
    });
    let result = std::panic::catch_unwind(move || {
        Game::new("t", 1, 1).run_headless(0, move |ctx| {
            ctx.load_scene(file).unwrap();
            let house = ctx.load_mesh("models/house.glb");
            let groups: Vec<LodGroup> = ctx.world.query::<&LodGroup>().iter().cloned().collect();
            assert_eq!(groups, vec![group(house)], "loaded with the scene, its mesh with it");
        });
    });
    let _ = std::fs::remove_file(path);
    result.unwrap();
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let e = ctx.world.spawn((Transform3D::default(),));
        let level = serde_json::json!({ "levels": [{ "screen_height": 0.1, "renderers": [{ "mesh": "nope.glb" }] }] });
        let err = ctx.world.set_json(e, "LodGroup", level);
        assert!(err.unwrap_err().to_string().contains("ctx.load_mesh(\"nope.glb\")"));
    });
}

#[test]
fn temporal_antialiasing_is_a_setting_and_can_forget_its_history() {
    Game::new("taa settings", 64, 64).run_headless(1, |ctx| {
        assert_eq!(ctx.post.taa_sharpness, 0.25);
        // ANCHOR: taa
        ctx.post.anti_aliasing = AntiAliasing::Taa;
        ctx.post.taa_sharpness = 0.3; // it softens the image a little
        // After teleporting the camera (or an object), forget the old view:
        ctx.camera3d.position = vec3(400.0, 3.0, 0.0);
        ctx.reset_temporal_history();
        // ANCHOR_END: taa
        assert_eq!(ctx.post.anti_aliasing, AntiAliasing::Taa);
    });
}

#[test]
fn ao_and_aa_settings_preserve_existing_defaults() {
    Game::new("post settings", 64, 64).run_headless(0, |ctx| {
        assert_eq!(ctx.post.anti_aliasing, AntiAliasing::None);
        assert!(!ctx.post.ambient_occlusion.enabled);
        assert!(!ctx.post.ao_half_resolution && !ctx.post.ao_temporal);
        // ANCHOR: ao_aa
        ctx.post.ao_half_resolution = true;
        ctx.post.ao_temporal = true;
        ctx.post.ambient_occlusion = AmbientOcclusion {
            enabled: true,
            radius: 1.5,
            intensity: 1.0,
            slices: 3,
            steps: 6,
        };
        ctx.post.contact_shadows = 0.3; // small, sharp sun shadows under feet and props
        ctx.post.anti_aliasing = AntiAliasing::Smaa;
        // ANCHOR_END: ao_aa
        assert_eq!(ctx.post.ambient_occlusion.radius, 1.5);
        assert_eq!(ctx.post.contact_shadows, 0.3);
    });
}

#[test]
fn screen_space_reflections_are_off_until_turned_on() {
    Game::new("ssr settings", 64, 64).run_headless(0, |ctx| {
        assert_eq!((ctx.post.ssr, ctx.post.ssr_max_distance), (0.0, 30.0));
        // ANCHOR: ssr
        // Wet streets and polished floors reflect what's on screen.
        ctx.post.ssr = 1.0;
        ctx.post.ssr_max_distance = 40.0; // world units along the reflected ray
        ctx.post.anti_aliasing = AntiAliasing::Taa; // smooths their grain
        // Only smooth surfaces reflect it: roughness below 0.5, fully below 0.2.
        let wet = MaterialDesc {
            roughness: 0.1,
            ..MaterialDesc::color(Color::rgb(0.2, 0.2, 0.22))
        };
        // ANCHOR_END: ssr
        assert_eq!(
            (ctx.post.ssr, ctx.post.ssr_max_distance, wet.roughness),
            (1.0, 40.0, 0.1)
        );
        ctx.set_graphics_quality(GraphicsQuality::medium());
        assert_eq!(ctx.post.ssr, 0.0, "medium turns them off");
        ctx.set_graphics_quality(GraphicsQuality::high());
        assert_eq!(ctx.post.ssr, 1.0, "high turns them on");
    });
}

#[test]
fn quality_presets_switch_at_run_time() {
    Game::new("quality", 64, 64).run_headless(0, |ctx| {
        // ANCHOR: quality
        // An options menu: pick a preset ...
        ctx.set_graphics_quality(GraphicsQuality::medium());
        // ... then change single settings on top of it.
        let mut quality = ctx.graphics_quality();
        quality.render_scale = 0.8; // draw the 3D scene at 80% and scale it up
        quality.sharpen = 0.3;
        quality.shadow_resolution = 4096;
        ctx.set_graphics_quality(quality);
        // The settings are the ordinary ones in ctx.post and ctx.lighting.
        ctx.post.ambient_occlusion.radius = 2.0;
        let level = ctx.graphics_quality().level(); // QualityLevel::Custom
        // ANCHOR_END: quality
        assert_eq!(level, QualityLevel::Custom);
        assert_eq!(ctx.post.render_scale, 0.8);
        assert_eq!(ctx.lighting.shadow_resolution, 4096);
        assert_eq!(ctx.post.render_size((1000, 500)), (800, 400));
    });
}

/// The scene format the Unity importer writes for a LOD group loads.
#[test]
fn lod_groups_in_the_importers_json_shape_load() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.load_mesh("models/house.glb");
        let part = |y: f32| {
            serde_json::json!({
                "mesh": "models/house.glb",
                "offset": { "position": [0.0, y, -1.0], "rotation": [0.0, 0.0, 0.0, 1.0], "scale": [1.0, 1.0, 1.0] },
            })
        };
        let json = serde_json::json!({
            "size": 4.0,
            "center": [0.0, 1.0, -2.0],
            "levels": [
                { "screen_height": 0.6, "fade_width": 0.25, "renderers": [part(1.0)], "shadow": "Same" },
                { "screen_height": 0.3, "fade_width": 0.0, "renderers": [part(0.0)], "shadow": { "Casters": [part(2.0)] } },
                { "screen_height": 0.1, "fade_width": 0.5, "renderers": [], "shadow": { "Casters": [] } },
            ],
        });
        let e = ctx.world.spawn((Transform3D::default(),));
        ctx.world.set_json(e, "LodGroup", json).unwrap();
        let group = ctx.world.get::<LodGroup>(e).unwrap();
        assert_eq!(group.levels.len(), 3);
        assert_eq!(group.levels[0].renderers[0].offset.position, vec3(0.0, 1.0, -1.0));
        assert!(matches!(&group.levels[1].shadow, LodShadow::Casters(c) if c.len() == 1));
        assert_eq!(group.center, vec3(0.0, 1.0, -2.0));
    });
}

// ANCHOR: gpu_buffers
fn gpu_particles(ctx: &mut Ctx) -> GpuBuffer {
    let items = ctx.create_gpu_buffer(128, &[]);
    let update = ctx.create_compute("particles", "fn update(i: u32) { items[i].y += frame.time.y; }");
    ctx.compute_passes.push(ComputePass::for_buffer(update, items));
    let shader = ctx.create_shader(
        "particles",
        "fn vertex(v: Vertex) -> Vertex { var o = v; o.position += items[v.instance_index].xyz; return o; }",
    );
    let material = ctx.create_material(MaterialDesc {
        shader: Some(shader),
        ..Default::default()
    });
    let mesh = ctx.create_mesh(&MeshData::cube(0.05), material);
    ctx.set_mesh_gpu_buffer(mesh, items, 128);
    ctx.world.spawn((Transform3D::default(), Model::new(mesh)));
    items
}
// ANCHOR_END: gpu_buffers
#[test]
fn gpu_buffers_support_headless_setup_and_writes() {
    Game::new("buffers", 1, 1).run_headless(1, |ctx| {
        let b = gpu_particles(ctx);
        assert_eq!(ctx.gpu_buffer_len(b), Some(128));
        ctx.write_gpu_buffer(b, 127, &[Vec4::ONE]);
        ctx.write_gpu_buffer(b, 128, &[]);
        assert_eq!(ctx.compute_passes[0].buffer, Some(b));
    });
}
#[test]
#[should_panic(expected = "out of bounds")]
fn gpu_buffer_writes_validate_ranges() {
    Game::new("buffers", 1, 1).run_headless(0, |ctx| {
        let b = ctx.create_gpu_buffer(1, &[]);
        ctx.write_gpu_buffer(b, 1, &[Vec4::ONE]);
    });
}

// ANCHOR: gpu_instances
fn persistent_instances(ctx: &mut Ctx, mesh: Mesh) {
    let transforms = (0..100_000)
        .map(|i| Mat4::from_translation(vec3((i % 1000) as f32, 0.0, (i / 1000) as f32)))
        .collect::<Vec<_>>();
    let batch = ctx.create_instance_batch(
        mesh,
        &transforms,
        InstanceBatchOptions {
            max_distance: 500.0,
            ..Default::default()
        },
    );
    assert_eq!(ctx.instance_batch_len(batch), Some(100_000));
    assert!(ctx.update_instance_batch(batch, &[Mat4::IDENTITY]));
    ctx.free_instance_batch(batch);
    assert_eq!(ctx.instance_batch_len(batch), None);
    assert!(!ctx.update_instance_batch(batch, &[]));
    ctx.free_instance_batch(batch);
}
// ANCHOR_END: gpu_instances
#[test]
fn hundred_thousand_static_instances_need_no_entities() {
    Game::new("static instances", 1, 1).run_headless(0, |ctx| {
        let material = ctx.create_material(MaterialDesc::default());
        let mesh = ctx.create_mesh(&MeshData::cube(1.0), material);
        persistent_instances(ctx, mesh);
        assert_eq!(ctx.world.len(), 0);
    });
}
