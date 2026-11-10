//! Skeletons from C: building one joint by joint or loading it from a glTF
//! file, posing joints, joint matrices, IK and look-at, clips, skinned mesh
//! data, and attachments. All headless: no window or GPU is involved.
use ponko_ffi::*;
use std::ffi::{CString, c_void};
use std::ptr::{null, null_mut};

use PonkoStatus::*;

const OK: PonkoStatus = PONKO_OK;
const BAD: PonkoStatus = PONKO_INVALID_ARGUMENT;

fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} != {b}");
}

fn near3(p: PonkoVec3, x: f32, y: f32, z: f32) {
    assert!(
        (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3 && (p.z - z).abs() < 1e-3,
        "{:?} is not ({x}, {y}, {z})",
        (p.x, p.y, p.z)
    );
}

/// A skeleton of two joints (`root`, then `arm` under it, with a triangle on
/// it) and two clips of one second: `slide` moves `root` from x 0 to x 2,
/// `idle` holds it at x 5.
fn gltf() -> String {
    let mut bytes = Vec::new();
    let mut floats = |v: &[f32]| v.iter().for_each(|f| bytes.extend(f.to_le_bytes()));
    floats(&[0., 0., 0., 1., 0., 0., 0., 1., 0.]); // 0: the triangle
    floats(&[0., 1.]); // 36: key times
    floats(&[0., 0., 0., 2., 0., 0.]); // 44: slide
    floats(&[5., 0., 0., 5., 0., 0.]); // 68: idle
    format!(
        r#"{{"asset": {{"version": "2.0"}}, "scene": 0, "scenes": [{{"nodes": [0]}}],
        "nodes": [{{"name": "root", "children": [1]}}, {{"name": "arm", "mesh": 0, "translation": [0, 1, 0]}}],
        "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
        "animations": [
            {{"name": "slide", "samplers": [{{"input": 1, "output": 2, "interpolation": "LINEAR"}}],
              "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "translation"}}}}]}},
            {{"name": "idle", "samplers": [{{"input": 1, "output": 3, "interpolation": "LINEAR"}}],
              "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "translation"}}}}]}}],
        "buffers": [{{"byteLength": 92, "uri": "data:application/octet-stream;base64,{}"}}],
        "bufferViews": [{{"buffer": 0, "byteLength": 36}}, {{"buffer": 0, "byteOffset": 36, "byteLength": 8}},
            {{"buffer": 0, "byteOffset": 44, "byteLength": 24}}, {{"buffer": 0, "byteOffset": 68, "byteLength": 24}}],
        "accessors": [
            {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 1, 0]}},
            {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]}},
            {{"bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3"}},
            {{"bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3"}}]}}"#,
        base64(&bytes)
    )
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

struct Run {
    path: CString,
    done: bool,
}

unsafe fn spawn_at(ctx: *mut PonkoCtx, at: PonkoVec3) -> u64 {
    unsafe {
        let w = ponko_ctx_world(ctx);
        let mut transform = 0;
        assert_eq!(ponko_transform3d_component(w, &mut transform), OK);
        let t = ponko_transform3d_at(at);
        let part = PonkoValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<PonkoTransform3D>(),
        };
        let mut e = 0u64;
        assert_eq!(ponko_spawn(w, &part, 1, &mut e), OK);
        e
    }
}

unsafe fn pose_of(ctx: *mut PonkoCtx, e: u64, joint: usize) -> PonkoTransform3D {
    unsafe {
        let mut pose = ponko_transform3d_at(v3(9.0, 9.0, 9.0));
        assert_eq!(ponko_ctx_joint_pose(ctx, e, joint, &mut pose), OK);
        pose
    }
}

/// A joint's matrix, column-major: its axes, then where it is.
unsafe fn matrix_of(ctx: *mut PonkoCtx, e: u64, joint: usize) -> [f32; 16] {
    unsafe {
        let mut m = [0.0f32; 16];
        assert_eq!(ponko_ctx_joint_matrix(ctx, e, joint, m.as_mut_ptr()), OK);
        m
    }
}

fn column(m: &[f32; 16], i: usize) -> PonkoVec3 {
    v3(m[i * 4], m[i * 4 + 1], m[i * 4 + 2])
}

/// `root`, `arm` one meter up from it, `hand` one meter up from that.
unsafe fn arm(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        let joints = [
            PonkoJoint {
                name: c"root".as_ptr(),
                parent: -1,
                rest: ponko_transform3d_at(v3(0.0, 0.0, 0.0)),
            },
            PonkoJoint {
                name: c"arm".as_ptr(),
                parent: 0,
                rest: ponko_transform3d_at(v3(0.0, 1.0, 0.0)),
            },
            PonkoJoint {
                name: c"hand".as_ptr(),
                parent: 1,
                rest: ponko_transform3d_at(v3(0.0, 1.0, 0.0)),
            },
        ];
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, joints.as_ptr(), 3), OK);
    }
}

unsafe fn building_a_skeleton(ctx: *mut PonkoCtx) -> u64 {
    unsafe {
        let e = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        let one = |name, parent| PonkoJoint {
            name,
            parent,
            rest: ponko_transform3d_at(v3(0.0, 0.0, 0.0)),
        };

        // Bad skeletons are errors, and leave the entity without one.
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, null(), 2), BAD, "null joints");
        let forward = [one(c"a".as_ptr(), 1), one(c"b".as_ptr(), -1)];
        assert_eq!(
            ponko_ctx_set_skeleton(ctx, e, forward.as_ptr(), 2),
            BAD,
            "a joint must hang from an earlier one"
        );
        let itself = [one(c"a".as_ptr(), 0)];
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, itself.as_ptr(), 1), BAD);
        let nameless = [one(null(), -1)];
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, nameless.as_ptr(), 1), BAD);
        let bad_utf8 = CString::new(vec![0xff, 0xfe]).unwrap();
        let broken = [one(bad_utf8.as_ptr(), -1)];
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, broken.as_ptr(), 1), BAD);
        let ok = [one(c"a".as_ptr(), -1)];
        assert_eq!(
            ponko_ctx_set_skeleton(ctx, 12345, ok.as_ptr(), 1),
            BAD,
            "no such entity"
        );
        assert_eq!(ponko_ctx_set_skeleton(null_mut(), e, ok.as_ptr(), 1), BAD);
        let mut n = 7;
        assert_eq!(ponko_ctx_joint_count(ctx, e, &mut n), PONKO_NOT_FOUND);

        // No joints at all is a (useless) skeleton, not an error.
        assert_eq!(ponko_ctx_set_skeleton(ctx, e, null(), 0), OK);
        assert_eq!(ponko_ctx_joint_count(ctx, e, &mut n), OK);
        assert_eq!(n, 0);

        arm(ctx, e);
        assert_eq!(ponko_ctx_joint_count(ctx, e, &mut n), OK);
        assert_eq!(n, 3);
        assert_eq!(ponko_ctx_joint_count(ctx, e, null_mut()), BAD);
        assert_eq!(ponko_ctx_joint_count(ctx, 12345, &mut n), BAD);
        assert_eq!(ponko_ctx_joint_count(null_mut(), e, &mut n), BAD);
        e
    }
}

unsafe fn finding_joints(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        let (mut found, mut at) = (false, 99usize);
        assert_eq!(ponko_ctx_find_joint(ctx, e, c"hand".as_ptr(), &mut found, &mut at), OK);
        assert!(found);
        assert_eq!(at, 2);
        assert_eq!(ponko_ctx_find_joint(ctx, e, c"root".as_ptr(), &mut found, &mut at), OK);
        assert_eq!(at, 0);
        // Not there: a successful answer, and the index is left alone.
        at = 99;
        assert_eq!(ponko_ctx_find_joint(ctx, e, c"tail".as_ptr(), &mut found, &mut at), OK);
        assert!(!found);
        assert_eq!(at, 99);
        assert_eq!(ponko_ctx_find_joint(ctx, e, null(), &mut found, &mut at), BAD);
        assert_eq!(
            ponko_ctx_find_joint(ctx, e, c"hand".as_ptr(), &mut found, null_mut()),
            BAD
        );
        assert_eq!(ponko_ctx_find_joint(ctx, e, c"hand".as_ptr(), null_mut(), &mut at), BAD);
        assert_eq!(
            ponko_ctx_find_joint(ctx, 12345, c"hand".as_ptr(), &mut found, &mut at),
            BAD
        );
        let bare = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(
            ponko_ctx_find_joint(ctx, bare, c"hand".as_ptr(), &mut found, &mut at),
            PONKO_NOT_FOUND,
            "no skeleton"
        );
        assert_eq!(
            ponko_ctx_find_joint(null_mut(), e, c"hand".as_ptr(), &mut found, &mut at),
            BAD
        );
    }
}

unsafe fn posing_joints(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        // At rest a joint's pose is its rest transform, and the matrices stack up the chain.
        near3(pose_of(ctx, e, 1).position, 0.0, 1.0, 0.0);
        near3(pose_of(ctx, e, 2).position, 0.0, 1.0, 0.0);
        let m = matrix_of(ctx, e, 0);
        near3(column(&m, 3), 0.0, 0.0, 0.0);
        let m = matrix_of(ctx, e, 2);
        near3(column(&m, 3), 0.0, 2.0, 0.0);
        near(m[0], 1.0);
        near(m[5], 1.0);

        // Moving the root moves everything below it.
        let mut moved = ponko_transform3d_at(v3(3.0, 0.0, 0.0));
        assert_eq!(ponko_ctx_set_joint_pose(ctx, e, 0, &moved), OK);
        near3(pose_of(ctx, e, 0).position, 3.0, 0.0, 0.0);
        near3(pose_of(ctx, e, 1).position, 0.0, 1.0, 0.0);
        near3(column(&matrix_of(ctx, e, 2), 3), 3.0, 2.0, 0.0);

        // Turning the arm a quarter turn around z swings the hand to -x.
        let half = std::f32::consts::FRAC_1_SQRT_2;
        moved = ponko_transform3d_at(v3(0.0, 1.0, 0.0));
        moved.rotation = PonkoQuat {
            x: 0.0,
            y: 0.0,
            z: half,
            w: half,
        };
        assert_eq!(ponko_ctx_set_joint_pose(ctx, e, 1, &moved), OK);
        near3(column(&matrix_of(ctx, e, 2), 3), 2.0, 1.0, 0.0);

        // Posing a later joint first works too (the earlier ones stay at rest).
        let fresh = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        arm(ctx, fresh);
        let lift = ponko_transform3d_at(v3(0.0, 5.0, 0.0));
        assert_eq!(ponko_ctx_set_joint_pose(ctx, fresh, 2, &lift), OK);
        near3(pose_of(ctx, fresh, 2).position, 0.0, 5.0, 0.0);
        near3(pose_of(ctx, fresh, 1).position, 0.0, 1.0, 0.0);
        near3(column(&matrix_of(ctx, fresh, 2), 3), 0.0, 6.0, 0.0);

        // Back to rest.
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);
        near3(pose_of(ctx, e, 0).position, 0.0, 0.0, 0.0);
        near3(column(&matrix_of(ctx, e, 2), 3), 0.0, 2.0, 0.0);
        assert_eq!(ponko_ctx_reset_pose(ctx, 12345), BAD);
        let bare = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(ponko_ctx_reset_pose(ctx, bare), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_reset_pose(null_mut(), e), BAD);

        // Errors on every read and write.
        let mut pose = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
        let mut m = [0.0f32; 16];
        assert_eq!(ponko_ctx_joint_pose(ctx, e, 3, &mut pose), BAD, "past the last joint");
        assert_eq!(ponko_ctx_joint_pose(ctx, e, 0, null_mut()), BAD);
        assert_eq!(ponko_ctx_joint_pose(ctx, 12345, 0, &mut pose), BAD);
        assert_eq!(ponko_ctx_joint_pose(ctx, bare, 0, &mut pose), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_joint_pose(null_mut(), e, 0, &mut pose), BAD);
        assert_eq!(ponko_ctx_set_joint_pose(ctx, e, 3, &pose), BAD);
        assert_eq!(ponko_ctx_set_joint_pose(ctx, e, 0, null()), BAD);
        assert_eq!(ponko_ctx_set_joint_pose(ctx, 12345, 0, &pose), BAD);
        assert_eq!(ponko_ctx_set_joint_pose(ctx, bare, 0, &pose), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_set_joint_pose(null_mut(), e, 0, &pose), BAD);
        assert_eq!(ponko_ctx_joint_matrix(ctx, e, 3, m.as_mut_ptr()), BAD);
        assert_eq!(ponko_ctx_joint_matrix(ctx, e, 0, null_mut()), BAD);
        assert_eq!(ponko_ctx_joint_matrix(ctx, bare, 0, m.as_mut_ptr()), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_joint_matrix(null_mut(), e, 0, m.as_mut_ptr()), BAD);
    }
}

unsafe fn aiming(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        // Two-bone IK: root, arm and hand reach a target within their length (2 m).
        let target = v3(1.0, 1.0, 0.0);
        let pole = v3(0.0, 0.0, 1.0);
        assert_eq!(ponko_ctx_two_bone_ik(ctx, e, 0, 1, 2, target, pole, 1.0), OK);
        near3(column(&matrix_of(ctx, e, 2), 3), 1.0, 1.0, 0.0);
        // The root didn't move, only turned.
        near3(column(&matrix_of(ctx, e, 0), 3), 0.0, 0.0, 0.0);

        // With no weight the pose stays as it is.
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);
        assert_eq!(ponko_ctx_two_bone_ik(ctx, e, 0, 1, 2, target, pole, 0.0), OK);
        near3(column(&matrix_of(ctx, e, 2), 3), 0.0, 2.0, 0.0);

        // Out of reach: the arm stretches toward it as far as it can.
        assert_eq!(
            ponko_ctx_two_bone_ik(ctx, e, 0, 1, 2, v3(10.0, 0.0, 0.0), pole, 1.0),
            OK
        );
        let tip = column(&matrix_of(ctx, e, 2), 3);
        assert!(tip.x > 1.5, "reaching toward +x: {:?}", (tip.x, tip.y, tip.z));
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);

        assert_eq!(ponko_ctx_two_bone_ik(ctx, e, 0, 1, 3, target, pole, 1.0), BAD);
        assert_eq!(ponko_ctx_two_bone_ik(ctx, e, 4, 1, 2, target, pole, 1.0), BAD);
        assert_eq!(ponko_ctx_two_bone_ik(ctx, 12345, 0, 1, 2, target, pole, 1.0), BAD);
        assert_eq!(ponko_ctx_two_bone_ik(null_mut(), e, 0, 1, 2, target, pole, 1.0), BAD);
        let bare = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(
            ponko_ctx_two_bone_ik(ctx, bare, 0, 1, 2, target, pole, 1.0),
            PONKO_NOT_FOUND
        );

        // Look-at: the root's z axis turns to face a point on +x.
        let aim = |max_angle, weight| {
            ponko_ctx_look_at_joint(ctx, e, 0, v3(0.0, 0.0, 1.0), v3(5.0, 0.0, 0.0), max_angle, weight)
        };
        assert_eq!(aim(std::f32::consts::PI, 1.0), OK);
        let facing = column(&matrix_of(ctx, e, 0), 2);
        near3(facing, 1.0, 0.0, 0.0);
        // A limit on the turn holds it back: a quarter turn is the most it may give,
        // so a limit of a tenth leaves it well short of facing the target.
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);
        assert_eq!(aim(0.1, 1.0), OK);
        let facing = column(&matrix_of(ctx, e, 0), 2);
        assert!(
            facing.x > 0.05 && facing.x < 0.2,
            "{:?}",
            (facing.x, facing.y, facing.z)
        );
        // No weight, no turn.
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);
        assert_eq!(aim(std::f32::consts::PI, 0.0), OK);
        near3(column(&matrix_of(ctx, e, 0), 2), 0.0, 0.0, 1.0);
        assert_eq!(
            ponko_ctx_look_at_joint(ctx, e, 3, v3(0.0, 0.0, 1.0), v3(5.0, 0.0, 0.0), 1.0, 1.0),
            BAD
        );
        assert_eq!(
            ponko_ctx_look_at_joint(ctx, 12345, 0, v3(0.0, 0.0, 1.0), v3(5.0, 0.0, 0.0), 1.0, 1.0),
            BAD
        );
        assert_eq!(
            ponko_ctx_look_at_joint(null_mut(), e, 0, v3(0.0, 0.0, 1.0), v3(5.0, 0.0, 0.0), 1.0, 1.0),
            BAD
        );
        assert_eq!(ponko_ctx_reset_pose(ctx, e), OK);
    }
}

unsafe fn skinned_meshes(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        // A plane has four corners; each follows joint 1 fully.
        let joints: [u16; 16] = [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0];
        let weights: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        ];
        let data = ponko_mesh_data_plane(1.0);
        assert_eq!(
            ponko_mesh_data_set_skin(data, joints.as_ptr(), weights.as_ptr(), 3),
            BAD,
            "3 of 4 corners"
        );
        assert_eq!(ponko_mesh_data_set_skin(data, null(), weights.as_ptr(), 4), BAD);
        assert_eq!(ponko_mesh_data_set_skin(data, joints.as_ptr(), null(), 4), BAD);
        assert_eq!(
            ponko_mesh_data_set_skin(null_mut(), joints.as_ptr(), weights.as_ptr(), 4),
            BAD
        );

        let mut material = PonkoMaterial::default();
        let desc = ponko_material_desc(PonkoColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        assert_eq!(ponko_ctx_create_material(ctx, &desc, &mut material), OK);
        let mut mesh = PonkoMesh::default();
        assert_eq!(
            ponko_ctx_create_skinned_mesh(ctx, data, material, e, &mut mesh),
            BAD,
            "no skin set yet"
        );

        assert_eq!(ponko_mesh_data_set_skin(data, joints.as_ptr(), weights.as_ptr(), 4), OK);
        assert_eq!(ponko_ctx_create_skinned_mesh(ctx, data, material, e, &mut mesh), OK);
        assert_eq!(ponko_ctx_create_skinned_mesh(ctx, data, material, e, null_mut()), BAD);
        assert_eq!(ponko_ctx_create_skinned_mesh(ctx, null(), material, e, &mut mesh), BAD);
        assert_eq!(
            ponko_ctx_create_skinned_mesh(ctx, data, PonkoMaterial { id: 999 }, e, &mut mesh),
            BAD,
            "a material this game didn't make"
        );
        assert_eq!(
            ponko_ctx_create_skinned_mesh(ctx, data, material, 12345, &mut mesh),
            BAD
        );
        let bare = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(
            ponko_ctx_create_skinned_mesh(ctx, data, material, bare, &mut mesh),
            PONKO_NOT_FOUND,
            "no skeleton to bind to"
        );
        assert_eq!(
            ponko_ctx_create_skinned_mesh(null_mut(), data, material, e, &mut mesh),
            BAD
        );

        // A corner that follows joint 5 doesn't fit a three-joint skeleton.
        let far: [u16; 16] = [5, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0];
        assert_eq!(ponko_mesh_data_set_skin(data, far.as_ptr(), weights.as_ptr(), 4), OK);
        assert_eq!(ponko_ctx_create_skinned_mesh(ctx, data, material, e, &mut mesh), BAD);

        // Zero corners takes the skin off again.
        assert_eq!(ponko_mesh_data_set_skin(data, null(), null(), 0), OK);
        assert_eq!(ponko_ctx_create_skinned_mesh(ctx, data, material, e, &mut mesh), BAD);
        ponko_mesh_data_free(data);
    }
}

unsafe fn attaching(ctx: *mut PonkoCtx, e: u64) {
    unsafe {
        let hat = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        let offset = ponko_transform3d_at(v3(0.0, 0.5, 0.0));
        assert_eq!(ponko_ctx_attach_to_joint(ctx, hat, e, 2, &offset), OK);
        assert_eq!(
            ponko_ctx_attach_to_joint(ctx, hat, e, 0, &offset),
            OK,
            "attaching again replaces it"
        );
        assert_eq!(ponko_ctx_attach_to_joint(ctx, hat, e, 2, null()), BAD);
        assert_eq!(ponko_ctx_attach_to_joint(ctx, 12345, e, 2, &offset), BAD);
        assert_eq!(ponko_ctx_attach_to_joint(ctx, hat, 12345, 2, &offset), BAD);
        assert_eq!(ponko_ctx_attach_to_joint(null_mut(), hat, e, 2, &offset), BAD);

        assert_eq!(ponko_ctx_set_skin_of(ctx, hat, e), OK);
        assert_eq!(ponko_ctx_set_skin_of(ctx, hat, e), OK, "again is fine");
        assert_eq!(ponko_ctx_set_skin_of(ctx, 12345, e), BAD);
        assert_eq!(ponko_ctx_set_skin_of(ctx, hat, 12345), BAD);
        assert_eq!(ponko_ctx_set_skin_of(null_mut(), hat, e), BAD);

        // Debug lines for a skeleton are queued for the next frame, headless or not.
        let white = PonkoColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        assert_eq!(ponko_ctx_draw_skeleton(ctx, e, white), OK);
        assert_eq!(ponko_ctx_draw_skeleton(ctx, 12345, white), BAD);
        assert_eq!(ponko_ctx_draw_skeleton(ctx, hat, white), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_draw_skeleton(null_mut(), e, white), BAD);
    }
}

unsafe fn from_a_file(ctx: *mut PonkoCtx, path: &CString) {
    unsafe {
        let e = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(ponko_ctx_load_skeleton(ctx, e, path.as_ptr()), OK);
        let mut n = 0;
        assert_eq!(ponko_ctx_joint_count(ctx, e, &mut n), OK);
        assert_eq!(n, 2);
        let (mut found, mut at) = (false, 99usize);
        assert_eq!(ponko_ctx_find_joint(ctx, e, c"arm".as_ptr(), &mut found, &mut at), OK);
        assert!(found);
        assert_eq!(at, 1);
        near3(pose_of(ctx, e, 1).position, 0.0, 1.0, 0.0);

        // A missing file or a null path is an error.
        let missing = c"/nonexistent/ponko-ffi-skin/missing.gltf";
        assert_eq!(
            ponko_ctx_load_skeleton(ctx, e, missing.as_ptr()),
            BAD,
            "a missing file is an error, not a panic"
        );
        assert_eq!(ponko_ctx_joint_count(ctx, e, &mut n), OK);
        assert_eq!(n, 2, "and the entity keeps the skeleton it had");
        // A file without a skeleton: a mesh with no skins or animations.
        let plain = std::env::temp_dir().join("ponko-ffi-skin-plain.gltf");
        std::fs::write(
            &plain,
            r#"{"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"name": "lone"}]}"#,
        )
        .unwrap();
        let plain_path = CString::new(plain.to_str().unwrap()).unwrap();
        assert_eq!(ponko_ctx_load_skeleton(ctx, e, plain_path.as_ptr()), BAD, "no skins");
        let _ = std::fs::remove_file(plain);
        assert_eq!(ponko_ctx_load_skeleton(ctx, e, null()), BAD);
        assert_eq!(ponko_ctx_load_skeleton(ctx, 12345, path.as_ptr()), BAD);
        assert_eq!(ponko_ctx_load_skeleton(null_mut(), e, path.as_ptr()), BAD);

        // Clips: found by name, with their length.
        let (mut slide, mut idle) = (null_mut(), null_mut());
        assert_eq!(
            ponko_ctx_load_clip(ctx, path.as_ptr(), c"slide".as_ptr(), &mut slide),
            OK
        );
        assert_eq!(ponko_ctx_load_clip(ctx, path.as_ptr(), c"idle".as_ptr(), &mut idle), OK);
        near(ponko_clip_duration(slide), 1.0);
        let mut none = null_mut();
        assert_eq!(
            ponko_ctx_load_clip(ctx, path.as_ptr(), c"dance".as_ptr(), &mut none),
            PONKO_NOT_FOUND
        );
        assert!(none.is_null());
        assert_eq!(
            ponko_ctx_load_clip(ctx, missing.as_ptr(), c"slide".as_ptr(), &mut none),
            BAD
        );
        assert_eq!(ponko_ctx_load_clip(ctx, path.as_ptr(), null(), &mut none), BAD);
        assert_eq!(
            ponko_ctx_load_clip(ctx, path.as_ptr(), c"slide".as_ptr(), null_mut()),
            BAD
        );

        // Sampling poses the skeleton: slide runs x from 0 to 2 over a second.
        assert_eq!(ponko_ctx_sample_clip(ctx, slide, e, 0.5, 1.0), OK);
        near3(pose_of(ctx, e, 0).position, 1.0, 0.0, 0.0);
        assert_eq!(ponko_ctx_sample_clip(ctx, slide, e, 1.0, 1.0), OK);
        near3(pose_of(ctx, e, 0).position, 2.0, 0.0, 0.0);
        // Blending: half of idle (x = 5) over what the pose holds (x = 2).
        assert_eq!(ponko_ctx_sample_clip(ctx, idle, e, 0.0, 0.5), OK);
        near3(pose_of(ctx, e, 0).position, 3.5, 0.0, 0.0);
        assert_eq!(ponko_ctx_sample_clip(ctx, null(), e, 0.0, 1.0), BAD);
        assert_eq!(ponko_ctx_sample_clip(ctx, slide, 12345, 0.0, 1.0), BAD);
        let bare = spawn_at(ctx, v3(0.0, 0.0, 0.0));
        assert_eq!(ponko_ctx_sample_clip(ctx, slide, bare, 0.0, 1.0), PONKO_NOT_FOUND);
        assert_eq!(ponko_ctx_sample_clip(null_mut(), slide, e, 0.0, 1.0), BAD);
        assert_eq!(ponko_clip_duration(null()), 0.0);

        ponko_clip_free(slide);
        ponko_clip_free(idle);
        ponko_clip_free(null_mut());
    }
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let e = building_a_skeleton(ctx);
        finding_joints(ctx, e);
        posing_joints(ctx, e);
        aiming(ctx, e);
        skinned_meshes(ctx, e);
        attaching(ctx, e);
        from_a_file(ctx, &run.path);
        run.done = true;
    }
}

#[test]
fn skeletons_are_built_posed_aimed_and_loaded_from_c() {
    let file = std::env::temp_dir().join("ponko-ffi-skin-rig.gltf");
    std::fs::write(&file, gltf()).unwrap();
    let mut run = Run {
        path: CString::new(file.to_str().unwrap()).unwrap(),
        done: false,
    };
    unsafe {
        let g = ponko_game_new(c"skin".as_ptr(), 1, 1);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(ponko_game_run_headless(g, 0, Some(setup), user, &mut world), OK);
        ponko_world_free(world);
    }
    assert!(run.done);
    let _ = std::fs::remove_file(file);
}
