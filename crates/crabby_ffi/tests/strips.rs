//! Ribbons, lines and decals from C.
use crabby_ffi::*;
use std::ffi::{CString, c_void};
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

/// A 4 x 4 m floor facing up, as a glTF file.
const FLOOR: &str = r#"{"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}],
    "nodes": [{"mesh": 0}],
    "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
    "buffers": [{"byteLength": 60, "uri": "data:application/octet-stream;base64,AAAAwAAAAAAAAABAAAAAQAAAAAAAAABAAAAAQAAAAAAAAADAAAAAwAAAAAAAAADAAAABAAIAAAACAAMA"}],
    "bufferViews": [{"buffer": 0, "byteLength": 48}, {"buffer": 0, "byteOffset": 48, "byteLength": 12}],
    "accessors": [{"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [-2, 0, -2], "max": [2, 0, 2]},
        {"bufferView": 1, "componentType": 5123, "count": 6, "type": "SCALAR"}]}"#;

const WHITE: CrabbyColor = CrabbyColor {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

const NOBODY: u64 = u64::MAX;

fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}

/// A scale of (0.5, 0.5, 0.2) turning the box's +Z up, at (1, 0, 0).
const DOWN: [f32; 16] = [
    0.5, 0.0, 0.0, 0.0, 0.0, 0.0, -0.2, 0.0, 0.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
];

fn view(data: *const CrabbyMeshData) -> CrabbyMeshView {
    let mut v = std::mem::MaybeUninit::<CrabbyMeshView>::uninit();
    assert_eq!(unsafe { crabby_mesh_data_view(data, v.as_mut_ptr()) }, CRABBY_OK);
    unsafe { v.assume_init() }
}

#[derive(Default)]
struct Run {
    path: CString,
    /// What each call returned, in order.
    statuses: Vec<CrabbyStatus>,
    counts: Vec<u32>,
    loaded: Vec<(usize, usize)>,
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let mut transform = 0;
        crabby_transform3d_component(crabby_ctx_world(ctx), &mut transform);
        let at = crabby_transform3d_at(v3(0.0, 1.0, 0.0));
        let value = CrabbyValue {
            component: transform,
            data: (&raw const at).cast(),
            size: size_of::<CrabbyTransform3D>(),
        };
        let mut e = 0;
        assert_eq!(crabby_spawn(crabby_ctx_world(ctx), &value, 1, &mut e), CRABBY_OK);
        let s = &mut run.statuses;

        // A tracer: a thin, glowing ribbon with curves, set and replaced.
        let widths = [
            CrabbyParticleKey { t: 0.0, value: 0.1 },
            CrabbyParticleKey { t: 1.0, value: 0.0 },
        ];
        let colors = [
            CrabbyParticleColorKey { t: 0.0, color: WHITE },
            CrabbyParticleColorKey {
                t: 1.0,
                color: CrabbyColor { a: 0.0, ..WHITE },
            },
        ];
        let mut trail = crabby_trail3d_new(0.25, 0.05);
        trail.width_keys = widths.as_ptr();
        trail.width_key_count = 2;
        trail.color_keys = colors.as_ptr();
        trail.color_key_count = 2;
        trail.blend = CrabbyParticleBlend3D::CRABBY_PARTICLES_ADDITIVE as u32;
        s.push(crabby_ctx_set_trail3d(ctx, e, &trail));
        let mut n = 99;
        s.push(crabby_ctx_trail3d_point_count(ctx, e, &mut n));
        run.counts.push(n);
        s.push(crabby_ctx_set_trail3d_emitting(ctx, e, false));
        s.push(crabby_ctx_clear_trail3d(ctx, e));
        s.push(crabby_ctx_set_trail3d(ctx, e, &trail));

        // Bad trails are errors, and leave the old one alone.
        let bad = |change: &dyn Fn(&mut CrabbyTrail3D)| {
            let mut t = trail;
            change(&mut t);
            crabby_ctx_set_trail3d(ctx, e, &t)
        };
        s.push(bad(&|t| t.lifetime = 0.0));
        s.push(bad(&|t| t.lifetime = f32::NAN));
        s.push(bad(&|t| t.width = -1.0));
        s.push(bad(&|t| t.max_points = 1));
        s.push(bad(&|t| t.blend = 7));
        s.push(bad(&|t| t.width_key_count = 2)); // fine: the same keys
        s.push(bad(&|t| t.width_keys = null()));
        s.push(crabby_ctx_set_trail3d(ctx, e, null()));
        s.push(crabby_ctx_set_trail3d(ctx, NOBODY, &trail));
        s.push(crabby_ctx_trail3d_point_count(ctx, e, null_mut()));

        s.push(crabby_ctx_remove_trail3d(ctx, e));
        s.push(crabby_ctx_trail3d_point_count(ctx, e, &mut n));
        s.push(crabby_ctx_clear_trail3d(ctx, e));

        // A beam, set twice (the second replaces the first).
        let points = [v3(0.0, 0.0, 0.0), v3(0.0, 0.0, -20.0)];
        let mut line = crabby_line3d_new(
            0.02,
            CrabbyColor {
                g: 0.0,
                b: 0.0,
                ..WHITE
            },
        );
        line.points = points.as_ptr();
        line.point_count = 2;
        line.blend = CrabbyParticleBlend3D::CRABBY_PARTICLES_ADDITIVE as u32;
        s.push(crabby_ctx_set_line3d(ctx, e, &line));
        line.texture_length = 2.0;
        s.push(crabby_ctx_set_line3d(ctx, e, &line));
        let mut bad_line = line;
        bad_line.width = -1.0;
        s.push(crabby_ctx_set_line3d(ctx, e, &bad_line));
        bad_line = line;
        bad_line.blend = 2;
        s.push(crabby_ctx_set_line3d(ctx, e, &bad_line));
        bad_line = line;
        bad_line.points = null();
        s.push(crabby_ctx_set_line3d(ctx, e, &bad_line));
        let nan = [v3(0.0, f32::NAN, 0.0)];
        bad_line = line;
        bad_line.points = nan.as_ptr();
        bad_line.point_count = 1;
        s.push(crabby_ctx_set_line3d(ctx, e, &bad_line));
        s.push(crabby_ctx_set_line3d(ctx, e, null()));
        s.push(crabby_ctx_remove_line3d(ctx, e));
        s.push(crabby_ctx_remove_line3d(ctx, NOBODY));

        // A floor from a glTF file, then a decal cut from it.
        let mut floor = null_mut();
        s.push(crabby_ctx_load_mesh_data(ctx, run.path.as_ptr(), &mut floor));
        if !floor.is_null() {
            let v = view(floor);
            run.loaded.push((v.vertex_count, v.index_count));
            let mark = crabby_mesh_data_decal(floor, DOWN.as_ptr(), 0.002);
            run.loaded.push((view(mark).vertex_count, view(mark).index_count));
            crabby_mesh_data_free(mark);
            crabby_mesh_data_free(floor);
        }
        let mut missing = null_mut();
        s.push(crabby_ctx_load_mesh_data(ctx, c"no/such.glb".as_ptr(), &mut missing));
        s.push(crabby_ctx_load_mesh_data(ctx, null(), &mut missing));
        s.push(crabby_ctx_load_mesh_data(ctx, run.path.as_ptr(), null_mut()));
    }
}

#[test]
fn trails_lines_and_mesh_files_work_from_c() {
    let file = std::env::temp_dir().join("crabby-ffi-strips-floor.gltf");
    std::fs::write(&file, FLOOR).unwrap();
    let mut run = Run {
        path: CString::new(file.to_str().unwrap()).unwrap(),
        ..Run::default()
    };
    unsafe {
        let g = crabby_game_new(c"strips".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(crabby_game_run_headless(g, 1, Some(setup), user, &mut world), CRABBY_OK);
        crabby_world_free(world);
    }
    let _ = std::fs::remove_file(&file);
    let (ok, bad) = (CRABBY_OK, CRABBY_INVALID_ARGUMENT);
    assert_eq!(
        run.statuses,
        [
            // a trail: set, count, stop, clear, replace
            ok, ok, ok, ok, ok, //
            // bad trails: lifetime 0, lifetime NaN, width, max_points, blend
            bad, bad, bad, bad, bad, //
            // same keys with their count; keys null with a count; null trail; no such entity; no output
            ok, bad, bad, bad, bad, //
            // removed: remove, then count and clear fail
            ok, bad, bad, //
            // a line: set, replace, width, blend, null points, NaN point, null line
            ok, ok, bad, bad, bad, bad, bad, //
            // remove, no such entity
            ok, bad, //
            // a file: load, missing file, null path, null output
            ok, bad, bad, bad,
        ]
    );
    assert_eq!(run.counts, [0]);
    assert_eq!(run.loaded[0], (4, 6));
    assert!(run.loaded[1].1 >= 3, "the decal has triangles: {:?}", run.loaded);
}

#[test]
fn decals_are_cut_from_mesh_data() {
    unsafe {
        let floor = crabby_mesh_data_plane(4.0);
        let mark = crabby_mesh_data_decal(floor, DOWN.as_ptr(), 0.002);
        assert!(!mark.is_null());
        let v = view(mark);
        assert!(v.index_count >= 3 && v.index_count.is_multiple_of(3));
        let positions = std::slice::from_raw_parts(v.positions, v.vertex_count);
        for p in positions {
            assert!((p.x - 1.0).abs() <= 0.25 + 1e-4 && p.z.abs() <= 0.25 + 1e-4, "{p:?}");
            assert!((p.y - 0.002).abs() < 1e-5, "lifted along the normal: {p:?}");
        }
        crabby_mesh_data_free(mark);

        // A box that misses the floor gives an empty decal, not an error.
        let mut away = DOWN;
        away[12] = 50.0;
        let none = crabby_mesh_data_decal(floor, away.as_ptr(), 0.002);
        assert!(!none.is_null());
        assert_eq!(view(none).index_count, 0);
        crabby_mesh_data_free(none);

        // Bad arguments are errors.
        assert!(crabby_mesh_data_decal(floor, null(), 0.002).is_null());
        assert!(crabby_mesh_data_decal(null(), DOWN.as_ptr(), 0.002).is_null());
        let mut nan = DOWN;
        nan[0] = f32::NAN;
        assert!(crabby_mesh_data_decal(floor, nan.as_ptr(), 0.002).is_null());
        assert!(crabby_mesh_data_decal(floor, DOWN.as_ptr(), f32::INFINITY).is_null());
        crabby_mesh_data_free(floor);
    }
}
