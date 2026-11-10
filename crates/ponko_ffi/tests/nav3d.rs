//! 3D navigation meshes from C: gathering walkable triangles, baking,
//! position and path queries, and agents walking a baked mesh headless.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

const OK: PonkoStatus = PONKO_OK;
const BAD: PonkoStatus = PONKO_INVALID_ARGUMENT;

fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}

fn near(p: PonkoVec3, x: f32, z: f32, tolerance: f32) {
    assert!(
        (p.x - x).abs() <= tolerance && (p.z - z).abs() <= tolerance && p.y.abs() < 0.5,
        "{:?} is not near ({x}, 0, {z})",
        (p.x, p.y, p.z)
    );
}

/// Where the entities end up, and the ids to read them with.
#[derive(Default)]
struct Run {
    wall: bool,
    transform: u32,
    walker: u64,
    stopped: u64,
    removed: u64,
    idle: u64,
    queries_done: bool,
}

/// Two triangles, a 4 x 4 m square at x 30..34, z 0..4, facing up.
const ISLAND: [[f32; 3]; 6] = [
    [30.0, 0.0, 0.0],
    [30.0, 0.0, 4.0],
    [34.0, 0.0, 0.0],
    [34.0, 0.0, 0.0],
    [30.0, 0.0, 4.0],
    [34.0, 0.0, 4.0],
];

unsafe fn bake_floor(ctx: *mut PonkoCtx) {
    unsafe {
        // Nothing baked yet: queries say so, they don't crash.
        let mut found = true;
        let mut point = v3(0.0, 0.0, 0.0);
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(0.0, 0.0, 0.0), 1.0, &mut point, &mut found),
            PONKO_NOT_FOUND
        );
        let mut len = 7;
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, v3(0.0, 0.0, 0.0), v3(1.0, 0.0, 0.0), null_mut(), 0, &mut len),
            PONKO_NOT_FOUND
        );

        // Nothing to bake.
        let none = ponko_nav_sources_new();
        assert_eq!(ponko_ctx_bake_navmesh(ctx, none, null()), BAD, "empty geometry");
        assert_eq!(ponko_ctx_bake_navmesh(ctx, null(), null()), BAD);
        assert_eq!(ponko_ctx_bake_navmesh(null_mut(), none, null()), BAD);
        assert_eq!(
            ponko_nav_sources_add_triangles(none, null(), 0),
            OK,
            "no triangles is fine"
        );
        assert_eq!(ponko_nav_sources_add_triangles(none, null(), 2), BAD);
        assert_eq!(ponko_nav_sources_add_triangles(null_mut(), null(), 0), BAD);
        ponko_nav_sources_free(none);
        ponko_nav_sources_free(null_mut());

        // A 20 m floor from mesh data, plus a separate island from raw triangles.
        let sources = ponko_nav_sources_new();
        let floor = ponko_mesh_data_plane(20.0);
        let at = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
        assert_eq!(ponko_nav_sources_add_mesh(sources, floor, at), OK);
        assert_eq!(ponko_nav_sources_add_mesh(sources, null(), at), BAD);
        assert_eq!(ponko_nav_sources_add_mesh(null_mut(), floor, at), BAD);
        ponko_mesh_data_free(floor);
        let corners = ISLAND.map(|[x, y, z]| v3(x, y, z));
        assert_eq!(ponko_nav_sources_add_triangles(sources, corners.as_ptr(), 2), OK);

        // A bake with other settings (a fatter agent) works too.
        let mut fat = ponko_nav_bake_settings_default();
        fat.agent_radius = 0.6;
        assert_eq!(ponko_ctx_bake_navmesh(ctx, sources, &fat), OK);
        let mut settings = ponko_nav_bake_settings_default();
        settings.cell_size = 0.0; // not a usable sampling size
        assert_eq!(
            ponko_ctx_bake_navmesh(ctx, sources, &settings),
            BAD,
            "bad settings name the reason"
        );
        assert_eq!(
            ponko_ctx_bake_navmesh(ctx, sources, null()),
            OK,
            "null settings: the defaults"
        );
        ponko_nav_sources_free(sources);
    }
}

unsafe fn queries(ctx: *mut PonkoCtx) {
    unsafe {
        // The nearest walkable point: snapped to the floor, or none in range.
        let (mut p, mut found) = (v3(9.0, 9.0, 9.0), false);
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(1.0, 1.0, 2.0), 2.0, &mut p, &mut found),
            OK
        );
        assert!(found);
        near(p, 1.0, 2.0, 0.25);
        assert!(p.y.abs() < 0.25, "on the floor: {}", p.y);
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(100.0, 100.0, 100.0), 1.0, &mut p, &mut found),
            OK
        );
        assert!(!found);
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(0.0, 0.0, 0.0), 1.0, null_mut(), &mut found),
            BAD
        );
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(0.0, 0.0, 0.0), 1.0, &mut p, null_mut()),
            BAD
        );
        // The island is walkable too.
        assert_eq!(
            ponko_ctx_navmesh_sample_position(ctx, v3(32.0, 0.5, 2.0), 2.0, &mut p, &mut found),
            OK
        );
        assert!(found);
        near(p, 32.0, 2.0, 0.5);

        // A path across the floor: both ends included, in order.
        let (from, to) = (v3(-7.0, 0.0, -3.0), v3(6.0, 0.0, 4.0));
        let mut corners = [v3(0.0, 0.0, 0.0); 16];
        let mut len = 0;
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, to, corners.as_mut_ptr(), 16, &mut len),
            OK
        );
        assert!((2..=16).contains(&len), "{len} corners");
        near(corners[0], -7.0, -3.0, 0.5);
        near(corners[len - 1], 6.0, 4.0, 0.5);
        // Too little room: the full count comes back, only what fits is written.
        let mut small = [v3(9.0, 9.0, 9.0); 2];
        let mut total = 0;
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, to, small.as_mut_ptr(), 1, &mut total),
            OK
        );
        assert_eq!(total, len);
        near(small[0], -7.0, -3.0, 0.5);
        assert_eq!(small[1].x, 9.0, "nothing written past the capacity");
        // Capacity 0 with no buffer just counts.
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, to, null_mut(), 0, &mut total),
            OK
        );
        assert_eq!(total, len);
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, to, null_mut(), 4, &mut total),
            BAD
        );
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, to, corners.as_mut_ptr(), 16, null_mut()),
            BAD
        );

        // No way from the floor to the island: no corners.
        total = 99;
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, v3(32.0, 0.0, 2.0), corners.as_mut_ptr(), 16, &mut total),
            OK
        );
        assert_eq!(total, 0);
        // Off the mesh altogether: no path either.
        assert_eq!(
            ponko_ctx_navmesh_find_path(ctx, from, v3(500.0, 0.0, 0.0), corners.as_mut_ptr(), 16, &mut total),
            OK
        );
        assert_eq!(total, 0);
    }
}

unsafe fn spawn_at(ctx: *mut PonkoCtx, transform: u32, p: PonkoVec3) -> u64 {
    unsafe {
        let t = ponko_transform3d_at(p);
        let part = PonkoValue {
            component: transform,
            data: (&raw const t).cast(),
            size: size_of::<PonkoTransform3D>(),
        };
        let mut e = 0;
        assert_eq!(ponko_spawn(ponko_ctx_world(ctx), &part, 1, &mut e), OK);
        e
    }
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        bake_floor(ctx);
        queries(ctx);
        run.queries_done = true;

        let w = ponko_ctx_world(ctx);
        assert_eq!(ponko_transform3d_component(w, &mut run.transform), OK);
        let t = run.transform;
        // Four agents on four rows of the floor, all bound for x = 5.
        let dest = |z| v3(5.0, 0.0, z);
        run.walker = spawn_at(ctx, t, v3(-5.0, 0.0, -6.0));
        run.stopped = spawn_at(ctx, t, v3(-5.0, 0.0, -2.0));
        run.removed = spawn_at(ctx, t, v3(-5.0, 0.0, 2.0));
        run.idle = spawn_at(ctx, t, v3(-5.0, 0.0, 6.0));
        for (e, z) in [(run.walker, -6.0), (run.stopped, -2.0), (run.removed, 2.0)] {
            assert_eq!(ponko_ctx_set_nav_agent(ctx, e, 4.0, 0.3, 40.0), OK);
            assert_eq!(ponko_ctx_set_nav_destination(ctx, e, &dest(z)), OK);
        }
        // Taken back: an agent told to stop, one taken off, and one with no
        // destination at all stay where they are.
        assert_eq!(ponko_ctx_set_nav_destination(ctx, run.stopped, null()), OK);
        assert_eq!(ponko_ctx_remove_nav(ctx, run.removed), OK);
        assert_eq!(ponko_ctx_remove_nav(ctx, run.removed), OK, "removing twice is fine");
        assert_eq!(ponko_ctx_set_nav_agent(ctx, run.idle, 4.0, 0.3, 40.0), OK);

        if run.wall {
            // A wall across the whole floor, where only the first agent's
            // path would cross it.
            let wall = spawn_at(ctx, t, v3(0.0, 0.0, 0.0));
            assert_eq!(
                ponko_ctx_set_nav_obstacle(ctx, wall, v3(0.0, 1.0, 0.0), v3(0.5, 2.0, 20.0)),
                OK
            );
            // Setting it again moves it; removing agents and obstacles works on it.
            assert_eq!(
                ponko_ctx_set_nav_obstacle(ctx, wall, v3(0.0, 1.0, 0.0), v3(0.5, 2.0, 20.0)),
                OK
            );
        }

        // Errors.
        let nobody = 12345;
        assert_eq!(ponko_ctx_set_nav_agent(ctx, nobody, 4.0, 0.3, 40.0), BAD);
        assert_eq!(
            ponko_ctx_set_nav_agent(ctx, run.idle, 0.0, 0.3, 40.0),
            BAD,
            "speed must be positive"
        );
        assert_eq!(
            ponko_ctx_set_nav_agent(ctx, run.idle, 4.0, -1.0, 40.0),
            BAD,
            "radius not negative"
        );
        assert_eq!(ponko_ctx_set_nav_destination(ctx, nobody, &dest(0.0)), BAD);
        let plain = spawn_at(ctx, t, v3(0.0, 0.0, 0.0));
        assert_eq!(
            ponko_ctx_set_nav_destination(ctx, plain, &dest(0.0)),
            PONKO_NOT_FOUND,
            "not an agent"
        );
        assert_eq!(
            ponko_ctx_set_nav_obstacle(ctx, nobody, v3(0.0, 0.0, 0.0), v3(1.0, 1.0, 1.0)),
            BAD
        );
        assert_eq!(ponko_ctx_remove_nav(ctx, nobody), BAD);
        assert_eq!(ponko_ctx_set_nav_agent(null_mut(), run.idle, 4.0, 0.3, 40.0), BAD);
        assert_eq!(ponko_ctx_set_nav_destination(null_mut(), run.idle, &dest(0.0)), BAD);
        assert_eq!(
            ponko_ctx_set_nav_obstacle(null_mut(), run.idle, v3(0.0, 0.0, 0.0), v3(1.0, 1.0, 1.0)),
            BAD
        );
        assert_eq!(ponko_ctx_remove_nav(null_mut(), run.idle), BAD);
    }
}

/// Runs 360 ticks with agents moving, and returns the four agents' positions
/// (walker, stopped, removed, idle).
fn run(wall: bool) -> [PonkoVec3; 4] {
    let mut run = Run { wall, ..Run::default() };
    unsafe {
        let g = ponko_game_new(c"nav3d".as_ptr(), 1, 1);
        assert_eq!(ponko_game_fixed_hz(g, 60), OK);
        assert_eq!(ponko_game_navmesh3d(g), OK);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(ponko_game_run_headless(g, 360, Some(setup), user, &mut world), OK);
        assert!(run.queries_done);
        let at = |e| {
            let mut t = std::mem::MaybeUninit::<PonkoTransform3D>::uninit();
            let size = size_of::<PonkoTransform3D>();
            assert_eq!(ponko_get(world, e, run.transform, t.as_mut_ptr().cast(), size), OK);
            t.assume_init().position
        };
        let result = [at(run.walker), at(run.stopped), at(run.removed), at(run.idle)];
        ponko_world_free(world);
        result
    }
}

#[test]
fn navmesh_bakes_answers_queries_and_walks_agents_from_c() {
    let [walker, stopped, removed, idle] = run(false);
    near(walker, 5.0, -6.0, 0.3);
    near(stopped, -5.0, -2.0, 0.01);
    near(removed, -5.0, 2.0, 0.01);
    near(idle, -5.0, 6.0, 0.01);
}

#[test]
fn obstacles_stop_agents_from_crossing() {
    let [walker, stopped, removed, idle] = run(true);
    assert!(walker.x < 0.0, "the wall holds the walker back: x = {}", walker.x);
    near(stopped, -5.0, -2.0, 0.01);
    near(removed, -5.0, 2.0, 0.01);
    near(idle, -5.0, 6.0, 0.01);
}

#[test]
fn a_null_game_cannot_get_the_agent_system() {
    unsafe {
        assert_eq!(ponko_game_navmesh3d(null_mut()), BAD);
    }
}
