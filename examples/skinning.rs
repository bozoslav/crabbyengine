//! Procedural tentacles: `cargo run --example skinning --features 3d`.
//! Escape quits. All visible geometry is built in code.
use crabbyengine::*;

const JOINTS: usize = 8;
const STEP: f32 = 0.5;
struct Sway(f32);

// ANCHOR: skin_chain
fn chain() -> Skeleton {
    Skeleton::new(
        (0..JOINTS)
            .map(|i| Joint {
                name: format!("joint_{i}"),
                parent: i.checked_sub(1),
                rest: Transform3D::at(Vec3::Y * if i == 0 { 0.0 } else { STEP }),
            })
            .collect(),
    )
}
// ANCHOR_END: skin_chain

// A cylinder (including caps), subdivided along +Y. Also used for the band.
// ANCHOR: skin_geometry
fn tube(bottom: f32, top: f32, radius: f32) -> MeshData {
    const SLICES: usize = 12;
    const ROWS: usize = 28;
    let mut data = MeshData::default();
    for row in 0..=ROWS {
        let y = bottom + (top - bottom) * row as f32 / ROWS as f32;
        let along = (y / STEP).clamp(0.0, (JOINTS - 1) as f32);
        let lower = (along.floor() as usize).min(JOINTS - 2);
        let blend = along - lower as f32;
        for slice in 0..SLICES {
            let angle = std::f32::consts::TAU * slice as f32 / SLICES as f32;
            data.positions.push(vec3(radius * angle.cos(), y, radius * angle.sin()));
            data.joints.push([lower as u16, (lower + 1) as u16, 0, 0]);
            data.weights.push([1.0 - blend, blend, 0.0, 0.0]);
        }
    }
    for row in 0..ROWS {
        for slice in 0..SLICES {
            let a = (row * SLICES + slice) as u32;
            let b = (row * SLICES + (slice + 1) % SLICES) as u32;
            let c = a + SLICES as u32;
            let d = b + SLICES as u32;
            data.indices.extend([a, c, b, b, c, d]);
        }
    }
    for slice in 1..SLICES - 1 {
        let last = (ROWS * SLICES) as u32;
        data.indices.extend([0, slice as u32, (slice + 1) as u32]);
        data.indices
            .extend([last, last + (slice + 1) as u32, last + slice as u32]);
    }
    data
}
// ANCHOR_END: skin_geometry

// ANCHOR: skin_pose
fn sway(ctx: &mut Ctx) {
    let time = ctx.time() as f32;
    ctx.world.query::<(&mut Skeleton, &Sway)>().for_each(|(s, phase)| {
        for (i, pose) in s.pose.iter_mut().enumerate() {
            pose.rotation = Quat::from_rotation_z((time * 1.5 + phase.0 - i as f32 * 0.45).sin() * 0.16);
        }
    });
    if ctx.key_pressed(Key::Escape) {
        ctx.quit();
    }
}
// ANCHOR_END: skin_pose

// Compiled book example; the procedural demo never loads asset files.
#[allow(dead_code)]
fn load_character(ctx: &mut Ctx) {
    // ANCHOR: skin_load
    let mesh = ctx.load_mesh("models/character.glb");
    let skeleton = ctx.load_skeleton("models/character.glb");
    let hand = skeleton.find("hand_r").expect("model needs hand_r");
    let character = ctx.world.spawn((Transform3D::default(), Model::new(mesh), skeleton));
    let skeleton = ctx.world.get_mut::<Skeleton>(character).unwrap();
    skeleton.pose[hand].rotation = Quat::from_rotation_y(0.3);
    let local_hand = skeleton.joint_matrix(hand);
    let transform = ctx.world.get::<Transform3D>(character).unwrap();
    let world_hand = (transform.matrix() * local_hand).transform_point3(Vec3::ZERO);
    println!("Hand at {world_hand}");
    // ANCHOR_END: skin_load
}

fn main() {
    Game::new("Crabby: skinned meshes", 1000, 700).system(sway).run(|ctx| {
        ctx.clear(Color::hex(0xABCBDD));
        ctx.camera3d.position = vec3(6.0, 4.0, 9.0);
        ctx.camera3d.look_at(vec3(0.0, 1.7, 0.0));
        let green = ctx.create_material(MaterialDesc::color(Color::hex(0x398F79)));
        let gold = ctx.create_material(MaterialDesc::color(Color::hex(0xF2BD50)));
        let white = ctx.default_material();
        let ground = ctx.create_mesh(&MeshData::plane(20.0), white);
        ctx.world.spawn((Transform3D::default(), Model::new(ground)));
        // ANCHOR: skin_create
        let skeleton = chain();
        let mesh = ctx.create_skinned_mesh(&tube(0.0, 3.5, 0.22), green, &skeleton);
        // A separate skeleton with matching names demonstrates name binding.
        let band = ctx.create_skinned_mesh(&tube(2.2, 2.5, 0.25), gold, &chain());
        // ANCHOR_END: skin_create
        let ball = ctx.create_mesh(&MeshData::sphere(0.18, 16), gold);
        for i in 0..3 {
            let tentacle = ctx.world.spawn((
                Transform3D::at(vec3((i as f32 - 1.0) * 2.5, 0.0, 0.0)),
                Model::new(mesh),
                skeleton.clone(),
                Sway(i as f32 * 1.8),
            ));
            if i == 0 {
                // ANCHOR: skin_parts
                ctx.world.spawn((Model::new(band), SkinOf(tentacle)));
                let tip = skeleton.find("joint_7").unwrap();
                ctx.world.spawn((
                    Transform3D::default(),
                    Model::new(ball),
                    AttachToJoint {
                        skeleton: tentacle,
                        joint: tip,
                        offset: Transform3D::at(vec3(0.0, 0.12, 0.0)),
                    },
                ));
                // ANCHOR_END: skin_parts
            }
        }
    });
}
