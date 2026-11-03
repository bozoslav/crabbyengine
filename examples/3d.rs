//! 3D: meshes, materials, a sun, and thousands of instanced models.
//!
//!   `cargo run --release --example 3d --features 3d`
//!
//! - Drag with the left mouse button to orbit, wheel to zoom.
//! - Space: pause the spinning.  Escape: quit.
//! - `CUBES=100000` (environment) spins that many cubes in a field.

use crabbyengine::*;

/// Turns around its own Y axis.
struct Spin(f32);

/// The orbiting camera: angles around the target and distance.
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

fn spin(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world.query::<(&mut Transform3D, &Spin)>().for_each(|(t, s)| {
        t.rotation *= Quat::from_rotation_y(s.0 * dt);
    });
}

fn orbit(ctx: &mut Ctx) {
    if ctx.key_pressed(Key::Space) {
        if ctx.is_paused() { ctx.resume() } else { ctx.pause() }
    }
    if ctx.key_pressed(Key::Escape) {
        ctx.quit();
    }
    let drag = if ctx.mouse_down(MouseButton::Left) {
        ctx.mouse_delta()
    } else {
        Vec2::ZERO
    };
    let wheel = ctx.mouse_wheel().y;
    let o = ctx.world.resource_mut::<Orbit>();
    o.yaw -= drag.x * 0.005;
    o.pitch = (o.pitch + drag.y * 0.005).clamp(0.05, 1.5);
    o.distance = (o.distance * 0.9f32.powf(wheel)).clamp(3.0, 400.0);
    let (yaw, pitch, distance) = (o.yaw, o.pitch, o.distance);
    let offset = vec3(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()) * distance;
    ctx.camera3d.position = offset;
    ctx.camera3d.look_at(Vec3::ZERO);
}

fn main() {
    let cubes: usize = std::env::var("CUBES").ok().and_then(|n| n.parse().ok()).unwrap_or(400);
    Game::new("Crabby Engine: 3D", 1280, 720)
        .sim_system(spin)
        .system(orbit)
        .run(move |ctx| {
            ctx.clear(Color::hex(0x9CC4E4)); // sky
            ctx.camera3d.far = 2000.0;
            ctx.lighting.sun_direction = vec3(-0.8, -0.6, -0.4); // late afternoon: long shadows
            ctx.world.insert_resource(Orbit {
                yaw: 0.6,
                pitch: 0.45,
                distance: 18.0,
            });

            let grass = ctx.create_material(MaterialDesc {
                roughness: 0.95,
                ..MaterialDesc::color(Color::hex(0x5E8C4A))
            });
            let ground = ctx.create_mesh(&MeshData::plane(400.0), grass);
            ctx.world.spawn((Transform3D::default(), Model::new(ground)));

            // Spheres: rough to smooth left to right, plastic in front, metal behind.
            let ball = MeshData::sphere(0.45, 32);
            for row in 0..2 {
                for col in 0..6 {
                    let material = ctx.create_material(MaterialDesc {
                        roughness: 0.05 + col as f32 * 0.19,
                        metallic: row as f32,
                        ..MaterialDesc::color(if row == 0 {
                            Color::hex(0xD8453B)
                        } else {
                            Color::hex(0xE8C070)
                        })
                    });
                    let mesh = ctx.create_mesh(&ball, material);
                    let at = vec3(col as f32 * 1.1 - 2.75, 0.45, row as f32 * -1.2);
                    ctx.world.spawn((Transform3D::at(at), Model::new(mesh)));
                }
            }

            // A field of spinning cubes, one mesh, tinted per cube.
            let white = ctx.default_material();
            let cube = ctx.create_mesh(&MeshData::cube(0.6), white);
            let side = (cubes as f32).sqrt().ceil() as usize;
            let mut rng = Rng::new(3);
            for i in 0..cubes {
                let (x, z) = ((i % side) as f32, (i / side) as f32);
                let at = vec3((x - side as f32 / 2.0) * 1.6, 0.3, -6.0 - z * 1.6);
                let tint = Color::rgb(rng.range(0.3, 1.0), rng.range(0.3, 1.0), rng.range(0.3, 1.0));
                ctx.world.spawn((
                    Transform3D::at(at).rotated(Quat::from_rotation_y(rng.range(0.0, 6.3))),
                    Model::new(cube).with_color(tint),
                    Spin(rng.range(-2.0, 2.0)),
                ));
            }
        });
}
