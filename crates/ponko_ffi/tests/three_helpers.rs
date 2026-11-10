//! Plain value helpers of the 3D C API (quaternions, transforms, mesh data,
//! tyre curves) and morph writing: pure functions, no window needed.
use ponko_ffi::*;
use std::f32::consts::FRAC_PI_2;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}
fn near(a: CrabbyVec3, b: CrabbyVec3) -> bool {
    (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5
}
fn view(data: *mut CrabbyMeshData) -> CrabbyMeshView {
    let mut view = std::mem::MaybeUninit::<CrabbyMeshView>::uninit();
    assert_eq!(unsafe { crabby_mesh_data_view(data, view.as_mut_ptr()) }, CRABBY_OK);
    unsafe { view.assume_init() }
}
fn positions(v: &CrabbyMeshView) -> &[CrabbyVec3] {
    unsafe { std::slice::from_raw_parts(v.positions, v.vertex_count) }
}

#[test]
fn rotating_a_vector_by_a_quarter_turn_about_y_sends_x_to_minus_z() {
    let quarter = crabby_quat_from_axis_angle(v3(0.0, 1.0, 0.0), FRAC_PI_2);
    assert!(near(crabby_quat_rotate(quarter, v3(1.0, 0.0, 0.0)), v3(0.0, 0.0, -1.0)));
    assert!(
        near(crabby_quat_rotate(quarter, v3(0.0, 5.0, 0.0)), v3(0.0, 5.0, 0.0)),
        "the axis stays"
    );
    assert!(near(crabby_quat_rotate(quarter, v3(0.0, 0.0, 1.0)), v3(1.0, 0.0, 0.0)));
    let identity = crabby_quat_identity();
    assert!(near(crabby_quat_rotate(identity, v3(1.0, 2.0, 3.0)), v3(1.0, 2.0, 3.0)));
    // Two quarter turns make a half turn: x goes to -x.
    let half = crabby_quat_mul(quarter, quarter);
    assert!(near(crabby_quat_rotate(half, v3(1.0, 0.0, 0.0)), v3(-1.0, 0.0, 0.0)));
}

#[test]
fn a_transform_faces_minus_z_and_looking_at_turns_it_towards_the_target() {
    let origin = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
    assert!(near(crabby_transform3d_forward(origin), v3(0.0, 0.0, -1.0)));

    let there = crabby_transform3d_at(v3(1.0, 2.0, 3.0));
    for (target, expected) in [
        (v3(1.0, 2.0, -10.0), v3(0.0, 0.0, -1.0)),
        (v3(11.0, 2.0, 3.0), v3(1.0, 0.0, 0.0)),
        (v3(-9.0, 2.0, 3.0), v3(-1.0, 0.0, 0.0)),
        (v3(1.0, 12.0, 3.0), v3(0.0, 1.0, 0.0)),
        (v3(1.0, 2.0, 13.0), v3(0.0, 0.0, 1.0)),
    ] {
        let turned = crabby_transform3d_looking_at(there, target);
        assert!(near(turned.position, there.position), "looking doesn't move");
        assert_eq!(turned.scale.x, there.scale.x);
        assert!(
            near(crabby_transform3d_forward(turned), expected),
            "{target:?}: {:?}",
            crabby_transform3d_forward(turned)
        );
    }
    // Looking diagonally: the forward is the unit vector to the target.
    let diagonal = crabby_transform3d_looking_at(origin, v3(3.0, 0.0, -4.0));
    assert!(near(crabby_transform3d_forward(diagonal), v3(0.6, 0.0, -0.8)));
}

#[test]
fn sphere_mesh_data_has_every_corner_on_the_sphere_and_unit_normals() {
    let sphere = crabby_mesh_data_sphere(2.5, 16);
    assert!(!sphere.is_null());
    let v = view(sphere);
    assert!(v.vertex_count > 16 && v.index_count > 0 && v.index_count.is_multiple_of(3));
    for (i, p) in positions(&v).iter().enumerate() {
        let r = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
        assert!((r - 2.5).abs() < 1e-4, "corner {i} is {r} from the center");
    }
    let normals = unsafe { std::slice::from_raw_parts(v.normals, v.vertex_count) };
    for (p, n) in positions(&v).iter().zip(normals) {
        assert!(near(v3(p.x / 2.5, p.y / 2.5, p.z / 2.5), *n), "normals point outwards");
    }
    let indices = unsafe { std::slice::from_raw_parts(v.indices, v.index_count) };
    assert!(indices.iter().all(|&i| (i as usize) < v.vertex_count));

    // More segments, more corners.
    let finer = crabby_mesh_data_sphere(2.5, 32);
    assert!(view(finer).vertex_count > v.vertex_count);
    // The smallest sphere still has triangles.
    let tiny = crabby_mesh_data_sphere(1.0, 0);
    assert!(view(tiny).index_count >= 3);
    unsafe {
        crabby_mesh_data_free(sphere);
        crabby_mesh_data_free(finer);
        crabby_mesh_data_free(tiny);
    }
}

#[test]
fn heightfield_mesh_data_puts_each_height_at_its_grid_corner() {
    // 3 x 3 heights over a 10-unit square: one peak in the middle.
    let heights = [0.0f32, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0];
    let hill = unsafe { crabby_mesh_data_heightfield(heights.as_ptr(), heights.len(), 10.0, 1) };
    assert!(!hill.is_null());
    let v = view(hill);
    assert!(positions(&v).contains(&v3(5.0, 2.0, 5.0)), "the peak is in the middle");
    let top = positions(&v).iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert_eq!(top, 2.0);
    let (min_x, max_x) = positions(&v)
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
    assert_eq!((min_x, max_x), (0.0, 10.0), "as wide as asked");
    // A coarser step uses fewer corners.
    let coarse = unsafe { crabby_mesh_data_heightfield(heights.as_ptr(), heights.len(), 10.0, 2) };
    assert!(view(coarse).vertex_count <= v.vertex_count);

    unsafe {
        assert!(
            crabby_mesh_data_heightfield(heights.as_ptr(), 8, 10.0, 1).is_null(),
            "not square"
        );
        assert!(
            crabby_mesh_data_heightfield(heights.as_ptr(), 1, 10.0, 1).is_null(),
            "too small"
        );
        assert!(
            crabby_mesh_data_heightfield(heights.as_ptr(), 9, 10.0, 0).is_null(),
            "step 0"
        );
        assert!(crabby_mesh_data_heightfield(null(), 9, 10.0, 1).is_null());
        crabby_mesh_data_free(hill);
        crabby_mesh_data_free(coarse);
    }
}

#[test]
fn mesh_data_colors_need_one_per_corner_or_none() {
    unsafe {
        let cube = crabby_mesh_data_cube(1.0);
        let corners = view(cube).vertex_count;
        let red = vec![
            CrabbyColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0
            };
            corners
        ];
        assert_eq!(crabby_mesh_data_set_colors(cube, red.as_ptr(), corners), CRABBY_OK);
        assert_eq!(
            crabby_mesh_data_set_colors(cube, red.as_ptr(), corners - 1),
            CRABBY_INVALID_ARGUMENT,
            "one short"
        );
        assert_eq!(
            crabby_mesh_data_set_colors(cube, red.as_ptr(), corners + 1),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_mesh_data_set_colors(cube, null(), corners),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_mesh_data_set_colors(cube, null(), 0),
            CRABBY_OK,
            "0 removes them"
        );
        assert_eq!(
            crabby_mesh_data_set_colors(null_mut(), red.as_ptr(), corners),
            CRABBY_INVALID_ARGUMENT
        );

        // The colored mesh data still makes a mesh.
        assert_eq!(crabby_mesh_data_set_colors(cube, red.as_ptr(), corners), CRABBY_OK);
        unsafe extern "C" fn make_mesh(ctx: *mut CrabbyCtx, user: *mut c_void) {
            unsafe {
                let (data, made) = &mut *user.cast::<(*mut CrabbyMeshData, bool)>();
                let desc = crabby_material_desc(CrabbyColor {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                });
                let mut material = CrabbyMaterial::default();
                assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut material), CRABBY_OK);
                let mut mesh = CrabbyMesh::default();
                assert_eq!(crabby_ctx_create_mesh(ctx, *data, material, &mut mesh), CRABBY_OK);
                assert_ne!(mesh.id, CRABBY_MESH_NONE);
                *made = true;
            }
        }
        let mut used = (cube, false);
        let game = crabby_game_new(c"colored".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 1, Some(make_mesh), (&raw mut used).cast(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
        assert!(used.1, "the setup ran and made a mesh from the colored data");
        crabby_mesh_data_free(cube);
    }
}

#[test]
fn the_forward_tyre_curve_peaks_at_its_extremum_then_slides_to_the_asymptote() {
    let forward = crabby_friction_curve_forward();
    let sideways = crabby_friction_curve_sideways();
    assert_ne!(forward, sideways, "rolling and sideways grip differ");
    assert_eq!(crabby_friction_curve_evaluate(forward, 0.0), 0.0);
    assert!((crabby_friction_curve_evaluate(forward, forward.extremum_slip) - forward.extremum_value).abs() < 1e-5);
    assert!((crabby_friction_curve_evaluate(forward, 5.0) - forward.asymptote_value).abs() < 1e-5);
    assert_eq!(
        crabby_friction_curve_evaluate(forward, -forward.extremum_slip),
        crabby_friction_curve_evaluate(forward, forward.extremum_slip),
        "either sign of slip"
    );
    let icy = CrabbyFrictionCurve {
        stiffness: 0.25,
        ..forward
    };
    assert!(
        (crabby_friction_curve_evaluate(icy, forward.extremum_slip) - 0.25 * forward.extremum_value).abs() < 1e-5,
        "stiffness scales the grip"
    );
}

/// A plane with one morph target ("lift") on a new entity with a model.
unsafe fn spawn_morph_model(ctx: *mut CrabbyCtx) -> u64 {
    unsafe {
        let w = crabby_ctx_world(ctx);
        let (mut transform, mut model) = (0u32, 0u32);
        assert_eq!(crabby_transform3d_component(w, &mut transform), CRABBY_OK);
        assert_eq!(crabby_model_component(w, &mut model), CRABBY_OK);
        let d = crabby_mesh_data_plane(1.0);
        let lift = [v3(0.0, 0.5, 0.0); 4];
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, c"lift".as_ptr(), lift.as_ptr(), null(), 4),
            CRABBY_OK
        );
        let desc = crabby_material_desc(CrabbyColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        let mut material = CrabbyMaterial::default();
        assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut material), CRABBY_OK);
        let mut mesh = CrabbyMesh::default();
        assert_eq!(crabby_ctx_create_mesh(ctx, d, material, &mut mesh), CRABBY_OK);
        crabby_mesh_data_free(d);
        let t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let m = crabby_model(mesh);
        let parts = [
            CrabbyValue {
                component: transform,
                data: (&t as *const CrabbyTransform3D).cast(),
                size: size_of::<CrabbyTransform3D>(),
            },
            CrabbyValue {
                component: model,
                data: (&m as *const CrabbyModel).cast(),
                size: size_of::<CrabbyModel>(),
            },
        ];
        let mut entity = u64::MAX;
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 2, &mut entity), CRABBY_OK);
        entity
    }
}

#[test]
fn writing_animated_morphs_keeps_the_weights_nothing_animates() {
    unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
        unsafe {
            let entity = &mut *user.cast::<u64>();
            *entity = spawn_morph_model(ctx);
            assert_eq!(crabby_ctx_set_morph_weight(ctx, *entity, 0, 0.75), CRABBY_OK);

            // No skeleton animates this model: its hand-set weight stays.
            assert_eq!(crabby_ctx_write_animated_morphs(ctx), CRABBY_OK);
            assert_eq!(crabby_ctx_write_animated_morphs(ctx), CRABBY_OK);
            let mut weight = f32::NAN;
            assert_eq!(crabby_ctx_morph_weight(ctx, *entity, 0, &mut weight), CRABBY_OK);
            assert_eq!(weight, 0.75);
        }
    }
    let mut entity = u64::MAX;
    unsafe {
        let game = crabby_game_new(c"morphs".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 1, Some(setup), (&raw mut entity).cast(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
        assert_eq!(crabby_ctx_write_animated_morphs(null_mut()), CRABBY_INVALID_ARGUMENT);
    }
    assert_ne!(entity, u64::MAX, "the setup ran");
}

/// The weight the skeleton's pose has for the "lift" target, then the weight
/// after setting it by hand and writing the animated morphs again.
#[derive(Default)]
struct Weights {
    posed: f32,
    after_hand_set: f32,
    written_again: f32,
    ran: bool,
}

#[test]
fn writing_animated_morphs_applies_the_pose_of_a_clips_morph_track_to_the_model() {
    unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
        unsafe {
            let seen = &mut *user.cast::<Weights>();
            let path = std::env::temp_dir().join("ponko-ffi-morph-rig.gltf");
            std::fs::write(
                &path,
                // Two joints, a triangle on the second and a clip that holds the
                // first still: the least a glTF needs to give a skeleton.
                concat!(
                    r#"{"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],"#,
                    r#" "nodes": [{"name": "root", "children": [1]}, {"name": "arm", "mesh": 0}],"#,
                    r#" "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],"#,
                    r#" "animations": [{"name": "hold", "samplers": [{"input": 1, "output": 2, "interpolation": "LINEAR"}],"#,
                    r#" "channels": [{"sampler": 0, "target": {"node": 0, "path": "translation"}}]}],"#,
                    r#" "buffers": [{"byteLength": 68, "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}],"#,
                    r#" "bufferViews": [{"buffer": 0, "byteLength": 36}, {"buffer": 0, "byteOffset": 36, "byteLength": 8},"#,
                    r#" {"buffer": 0, "byteOffset": 44, "byteLength": 24}],"#,
                    r#" "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3","#,
                    r#" "min": [0, 0, 0], "max": [1, 1, 0]},"#,
                    r#" {"bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0], "max": [1]},"#,
                    r#" {"bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3"}]}"#
                ),
            )
            .unwrap();
            let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
            let e = spawn_morph_model(ctx);
            assert_eq!(crabby_ctx_load_skeleton(ctx, e, path.as_ptr()), CRABBY_OK);
            let weight = |ctx| {
                let mut w = f32::NAN;
                assert_eq!(crabby_ctx_morph_weight(ctx, e, 0, &mut w), CRABBY_OK);
                w
            };
            assert_eq!(crabby_ctx_set_morph_weight(ctx, e, 0, 0.1), CRABBY_OK);
            assert_eq!(weight(ctx), 0.1, "nothing has posed the skeleton yet");

            // A clip that holds "lift" at 0.6 for a second, played by a one-state graph.
            let mut clip = null_mut();
            assert_eq!(crabby_clip_new(c"face".as_ptr(), &mut clip), CRABBY_OK);
            let (times, values) = ([0.0f32, 1.0], [0.6f32, 0.6]);
            assert_eq!(
                crabby_clip_set_morph_track(clip, c"lift".as_ptr(), times.as_ptr(), values.as_ptr(), 2, 1),
                CRABBY_OK
            );
            let mut motion = null_mut();
            assert_eq!(crabby_motion_clip(clip, &mut motion), CRABBY_OK);
            let builder = crabby_animation_graph_builder_new();
            assert_eq!(
                crabby_animation_graph_add_state(builder, motion, 1.0, -1, true, null_mut()),
                CRABBY_OK
            );
            let mut graph = null_mut();
            assert_eq!(crabby_animation_graph_build(builder, 0, &mut graph), CRABBY_OK);
            assert_eq!(crabby_ctx_set_animator(ctx, e, graph), CRABBY_OK);
            assert_eq!(crabby_ctx_animator_set_manual(ctx, e, true), CRABBY_OK);
            assert_eq!(crabby_ctx_animator_update(ctx, e, 0.1), CRABBY_OK);
            seen.posed = weight(ctx);

            // Hand-setting the weight, then writing the animated morphs again:
            // the pose's morph channel is applied to the model once more.
            assert_eq!(crabby_ctx_set_morph_weight(ctx, e, 0, 0.0), CRABBY_OK);
            seen.after_hand_set = weight(ctx);
            assert_eq!(crabby_ctx_write_animated_morphs(ctx), CRABBY_OK);
            seen.written_again = weight(ctx);
            seen.ran = true;
            crabby_animation_graph_free(graph);
            crabby_clip_free(clip);
        }
    }
    let mut seen = Weights::default();
    unsafe {
        let game = crabby_game_new(c"morph-clip".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 1, Some(setup), (&raw mut seen).cast(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert!(seen.ran);
    assert!(
        (seen.posed - 0.6).abs() < 1e-5,
        "the animator wrote the clip's weight: {}",
        seen.posed
    );
    assert_eq!(seen.after_hand_set, 0.0);
    assert!(
        (seen.written_again - 0.6).abs() < 1e-5,
        "writing again applies the pose: {}",
        seen.written_again
    );
}
