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
