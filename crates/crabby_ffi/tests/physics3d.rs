//! 3D physics from C: body, collider, velocity and character components,
//! collision meshes, the game setting, queries and impulses.
#![cfg(feature = "physics3d")]
use crabby_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyBodyKind::*;
use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    body: u32,
    collider: u32,
    velocity: u32,
    character: u32,
    ball: u64,
    ground: u64,
    walker: u64,
    hit: Option<CrabbyHit3D>,
    all_hits: usize,
    overlapping: Vec<u64>,
    mass: f32,
}

fn value<T>(component: u32, v: &T) -> CrabbyValue {
    CrabbyValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}
fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}
fn bits(e: CrabbyEntity) -> u64 {
    crabby_entity_bits(e)
}

unsafe fn ids_of(w: *mut CrabbyWorld, ids: &mut Ids) {
    unsafe {
        assert_eq!(crabby_transform3d_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut ids.body), CRABBY_OK);
        assert_eq!(crabby_collider3d_component(w, &mut ids.collider), CRABBY_OK);
        assert_eq!(crabby_velocity3d_component(w, &mut ids.velocity), CRABBY_OK);
        assert_eq!(crabby_character3d_component(w, &mut ids.character), CRABBY_OK);
    }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        ids_of(w, ids);
        // Flat 3 x 3 ground, 20 m across, its corner at (-10, 0, -10).
        let heights = [0.0f32; 9];
        let mut ground = CrabbyCollisionMesh::default();
        assert_eq!(
            crabby_ctx_physics3d_heightfield(ctx, heights.as_ptr(), 9, 20.0, &mut ground),
            CRABBY_OK
        );
        let (fixed, dynamic) = (
            crabby_rigid_body3d(CRABBY_BODY_FIXED as u32),
            crabby_rigid_body3d(CRABBY_BODY_DYNAMIC as u32),
        );
        let (ground_at, ground_shape) = (
            crabby_transform3d_at(v3(-10.0, 0.0, -10.0)),
            crabby_collider3d_mesh(ground),
        );
        let values = [
            value(ids.transform, &ground_at),
            value(ids.body, &fixed),
            value(ids.collider, &ground_shape),
        ];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 3, &mut ids.ground), CRABBY_OK);

        let (ball_at, ball_shape, still) = (
            crabby_transform3d_at(v3(0.0, 3.0, 0.0)),
            crabby_collider3d_sphere(0.5),
            CrabbyVelocity3D::default(),
        );
        let values = [
            value(ids.transform, &ball_at),
            value(ids.body, &dynamic),
            value(ids.collider, &ball_shape),
            value(ids.velocity, &still),
        ];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 4, &mut ids.ball), CRABBY_OK);

        let mut walker = crabby_character3d(1.8, 0.3);
        walker.velocity = v3(1.0, 0.0, 0.0);
        let walker_at = crabby_transform3d_at(v3(-5.0, 0.5, 0.0));
        let values = [value(ids.transform, &walker_at), value(ids.character, &walker)];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut ids.walker), CRABBY_OK);
    }
}

unsafe extern "C" fn ask(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        if crabby_ctx_sim_tick(ctx) != 1 {
            return;
        }
        // Bodies exist after the first step.
        let (mut found, mut hit) = (false, CrabbyHit3D::default());
        let mut filter = crabby_query_filter3d();
        filter.exclude = true;
        filter.exclude_entity = ids.ball;
        assert_eq!(
            crabby_ctx_cast_ray3d(
                ctx,
                v3(0.0, 10.0, 0.0),
                v3(0.0, -1.0, 0.0),
                100.0,
                filter,
                &mut found,
                &mut hit
            ),
            CRABBY_OK
        );
        ids.hit = found.then_some(hit);
        let mut buf = [CrabbyHit3D::default(); 1];
        assert_eq!(
            crabby_ctx_cast_ray3d_all(
                ctx,
                v3(0.0, 10.0, 0.0),
                v3(0.0, -1.0, 0.0),
                100.0,
                crabby_query_filter3d(),
                buf.as_mut_ptr(),
                1,
                &mut ids.all_hits
            ),
            CRABBY_OK
        );
        let mut found_s = false;
        assert_eq!(
            crabby_ctx_cast_sphere3d(
                ctx,
                v3(0.0, 10.0, 0.0),
                0.25,
                v3(0.0, -1.0, 0.0),
                100.0,
                filter,
                &mut found_s,
                &mut hit
            ),
            CRABBY_OK
        );
        assert!(found_s);
        let mut near = [CrabbyEntity::default(); 4];
        let mut n = 0;
        assert_eq!(
            crabby_ctx_overlap_sphere3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                0.5,
                crabby_query_filter3d(),
                near.as_mut_ptr(),
                4,
                &mut n
            ),
            CRABBY_OK
        );
        ids.overlapping = near[..n.min(4)].iter().map(|e| bits(*e)).collect();
        assert_eq!(crabby_ctx_body_mass3d(ctx, ids.ball, &mut ids.mass), CRABBY_OK);
        let mut none = 0.0;
        assert_eq!(crabby_ctx_body_mass3d(ctx, 12345, &mut none), CRABBY_NOT_FOUND);
        assert_eq!(
            crabby_ctx_apply_impulse3d(ctx, 12345, v3(1.0, 0.0, 0.0)),
            CRABBY_NOT_FOUND
        );
        assert_eq!(crabby_ctx_apply_impulse3d(ctx, ids.ball, v3(0.0, 0.0, 0.0)), CRABBY_OK);
    }
}

#[test]
fn c_bodies_land_on_a_heightfield_characters_walk_and_queries_answer() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"physics3d".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        assert_eq!(
            crabby_game_sim_system(g, c"ask".as_ptr(), 1, Some(ask), user),
            CRABBY_OK
        );
        assert_eq!(
            crabby_game_run_headless(g, 150, Some(setup), user, &mut world),
            CRABBY_OK
        );

        let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let size = size_of::<CrabbyTransform3D>();
        assert_eq!(
            crabby_get(world, ids.ball, ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        assert!(
            (t.position.y - 0.5).abs() < 0.05,
            "rests on the ground: {}",
            t.position.y
        );

        let mut walker = crabby_character3d(1.0, 1.0);
        let size = size_of::<CrabbyCharacter3D>();
        assert_eq!(
            crabby_get(world, ids.walker, ids.character, (&raw mut walker).cast(), size),
            CRABBY_OK
        );
        assert!(walker.grounded, "stands on the ground");
        assert_eq!(
            crabby_get(
                world,
                ids.walker,
                ids.transform,
                (&raw mut t).cast(),
                size_of::<CrabbyTransform3D>()
            ),
            CRABBY_OK
        );
        assert!(t.position.x > -2.5, "walked 3 s at 1 m/s: {}", t.position.x);
        assert!(t.position.y.abs() < 0.05, "feet on the ground: {}", t.position.y);

        let hit = ids.hit.expect("the ray hits the ground through the ball");
        assert_eq!(bits(hit.entity), ids.ground);
        assert!((hit.distance - 10.0).abs() < 1e-3, "{}", hit.distance);
        assert!((hit.normal.y - 1.0).abs() < 1e-3);
        assert_eq!(ids.all_hits, 2, "the ball and the ground; one written");
        assert_eq!(ids.overlapping, [ids.ground]);
        let ball_mass = 1000.0 * 4.0 / 3.0 * std::f32::consts::PI * 0.125;
        assert!((ids.mass - ball_mass).abs() < 1.0, "{}", ids.mass);
        crabby_world_free(world);
    }
}

#[test]
fn components_round_trip_and_are_checked() {
    let mut ids = Ids::default();
    unsafe {
        let w = crabby_world_new();
        ids_of(w, &mut ids);
        let mut body = crabby_rigid_body3d(CRABBY_BODY_KINEMATIC as u32);
        body.linear_damping = 0.5;
        body.ccd = true;
        let mut capsule = crabby_collider3d_capsule(1.2, 0.3);
        capsule.offset = v3(0.0, 0.9, 0.0);
        capsule.restitution = 0.25;
        capsule.sensor = true;
        capsule.groups = CrabbyCollisionGroups {
            memberships: 2,
            filter: 5,
        };
        let mut walker = crabby_character3d(1.7, 0.25);
        walker.step_height = 0.2;
        walker.velocity = v3(1.0, 2.0, 3.0);
        let values = [
            value(ids.body, &body),
            value(ids.collider, &capsule),
            value(ids.character, &walker),
        ];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 3, &mut e), CRABBY_OK);

        let mut back = crabby_rigid_body3d(0);
        let n = size_of::<CrabbyRigidBody3D>();
        assert_eq!(crabby_get(w, e, ids.body, (&raw mut back).cast(), n), CRABBY_OK);
        assert_eq!(back, body);
        let mut shape = crabby_collider3d_sphere(1.0);
        let n = size_of::<CrabbyCollider3D>();
        assert_eq!(crabby_get(w, e, ids.collider, (&raw mut shape).cast(), n), CRABBY_OK);
        assert_eq!(shape, capsule);
        let mut ch = crabby_character3d(1.0, 1.0);
        let n = size_of::<CrabbyCharacter3D>();
        assert_eq!(crabby_get(w, e, ids.character, (&raw mut ch).cast(), n), CRABBY_OK);
        assert_eq!(ch, walker);

        let bad = CrabbyRigidBody3D { kind: 7, ..body };
        let n = size_of::<CrabbyRigidBody3D>();
        assert_eq!(
            crabby_set(w, e, ids.body, (&raw const bad).cast(), n),
            CRABBY_INVALID_ARGUMENT
        );
        let flat = crabby_collider3d_box(v3(1.0, 0.0, 1.0));
        let n = size_of::<CrabbyCollider3D>();
        assert_eq!(
            crabby_set(w, e, ids.collider, (&raw const flat).cast(), n),
            CRABBY_INVALID_ARGUMENT
        );
        let tiny = crabby_character3d(0.0, 0.3);
        let n = size_of::<CrabbyCharacter3D>();
        assert_eq!(
            crabby_set(w, e, ids.character, (&raw const tiny).cast(), n),
            CRABBY_INVALID_ARGUMENT
        );

        // Bodies, colliders and characters need conversion, so they can't be
        // query columns; velocity can.
        unsafe extern "C" fn nothing(_: *const CrabbyChunk, _: *mut c_void) {}
        for id in [ids.body, ids.collider, ids.character] {
            let cols = [id];
            let s = crabby_query(w, cols.as_ptr(), 1, Some(nothing), null_mut());
            assert_eq!(s, CRABBY_INVALID_ARGUMENT);
        }
        let cols = [ids.velocity];
        assert_eq!(crabby_query(w, cols.as_ptr(), 1, Some(nothing), null_mut()), CRABBY_OK);
        crabby_world_free(w);
    }
}

unsafe extern "C" fn meshes(ctx: *mut CrabbyCtx, _: *mut c_void) {
    unsafe {
        let mut m = CrabbyCollisionMesh::default();
        // Not square, null, or too small.
        let h = [0.0f32; 6];
        assert_eq!(
            crabby_ctx_physics3d_heightfield(ctx, h.as_ptr(), 6, 1.0, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_physics3d_heightfield(ctx, null(), 4, 1.0, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_physics3d_heightfield(ctx, h.as_ptr(), 1, 1.0, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        let cube: Vec<CrabbyVec3> = (0..8)
            .map(|i| v3((i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32))
            .collect();
        assert_eq!(
            crabby_ctx_physics3d_convex_hull(ctx, cube.as_ptr(), 8, &mut m),
            CRABBY_OK
        );
        assert_eq!(crabby_ctx_physics3d_free_mesh(ctx, m), CRABBY_OK);
        let flat = [v3(0.0, 0.0, 0.0), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0)];
        assert_eq!(
            crabby_ctx_physics3d_convex_hull(ctx, flat.as_ptr(), 3, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        let tri = [0u32, 1, 2];
        assert_eq!(
            crabby_ctx_physics3d_trimesh(ctx, flat.as_ptr(), 3, tri.as_ptr(), 3, &mut m),
            CRABBY_OK
        );
        let out_of_range = [0u32, 1, 3];
        assert_eq!(
            crabby_ctx_physics3d_trimesh(ctx, flat.as_ptr(), 3, out_of_range.as_ptr(), 3, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_physics3d_trimesh(ctx, flat.as_ptr(), 3, tri.as_ptr(), 2, &mut m),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_set_gravity3d(ctx, v3(0.0, f32::NAN, 0.0)),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_ctx_set_gravity3d(ctx, v3(0.0, -1.0, 0.0)), CRABBY_OK);
    }
}

#[test]
fn collision_meshes_are_checked() {
    unsafe {
        let g = crabby_game_new(c"meshes".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(meshes), null_mut(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
}

unsafe extern "C" fn mouse(ctx: *mut CrabbyCtx, _: *mut c_void) {
    unsafe {
        let mut motion = CrabbyVec2 { x: 1.0, y: 1.0 };
        assert_eq!(crabby_ctx_mouse_motion(ctx, &mut motion), CRABBY_OK);
        assert_eq!((motion.x, motion.y), (0.0, 0.0), "no mouse in a headless run");
        let mut locked = true;
        assert_eq!(crabby_ctx_cursor_locked(ctx, &mut locked), CRABBY_OK);
        assert!(!locked);
        assert_eq!(crabby_ctx_set_cursor_locked(ctx, true), CRABBY_OK);
        assert_eq!(crabby_ctx_cursor_locked(ctx, &mut locked), CRABBY_OK);
        assert!(locked);
    }
}

#[test]
fn mouse_motion_and_cursor_lock_work_headless() {
    unsafe {
        let g = crabby_game_new(c"mouse".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(mouse), null_mut(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
}
