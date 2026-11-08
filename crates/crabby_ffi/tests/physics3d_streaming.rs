//! 3D physics for big worlds from C: collision mesh data built on another
//! thread, terrain colliders, and static colliders kept near a focus.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    body: u32,
    collider: u32,
    terrain: u32,
    focus: u32,
    ground: u64,
    far_box: u64,
    ball: u64,
    focus_entity: u64,
    /// After the first step: ground in, far box out.
    first: Option<(bool, bool)>,
    /// After the focus walked to the far box: box in.
    later: Option<bool>,
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

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform3d_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut ids.body), CRABBY_OK);
        assert_eq!(crabby_collider3d_component(w, &mut ids.collider), CRABBY_OK);
        assert_eq!(crabby_terrain_collider_component(w, &mut ids.terrain), CRABBY_OK);
        assert_eq!(crabby_collider_focus_component(w, &mut ids.focus), CRABBY_OK);
        assert_eq!(crabby_ctx_physics3d_stream_colliders(ctx, true, 8), CRABBY_OK);
        // Ground 2 m up, built on another thread, 20 m across from (-10, 0, -10).
        let data = std::thread::spawn(|| {
            let heights = [2.0f32; 9];
            crabby_collision_mesh_data_heightfield(heights.as_ptr(), 9, 20.0) as usize
        })
        .join()
        .unwrap() as *mut CrabbyCollisionMeshData;
        assert!(!data.is_null());
        let mut mesh = CrabbyCollisionMesh::default();
        assert_eq!(crabby_ctx_physics3d_add_mesh(ctx, data, &mut mesh), CRABBY_OK);
        crabby_collision_mesh_data_free(data);
        let (at, solid) = (
            crabby_transform3d_at(v3(-10.0, 0.0, -10.0)),
            crabby_terrain_collider(mesh),
        );
        let values = [value(ids.transform, &at), value(ids.terrain, &solid)];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut ids.ground), CRABBY_OK);
        let (at, body, shape) = (
            crabby_transform3d_at(v3(0.0, 4.0, 0.0)),
            crabby_rigid_body3d(CrabbyBodyKind::CRABBY_BODY_DYNAMIC as u32),
            crabby_collider3d_sphere(0.5),
        );
        let values = [
            value(ids.transform, &at),
            value(ids.body, &body),
            value(ids.collider, &shape),
        ];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 3, &mut ids.ball), CRABBY_OK);
        // A wall 300 m away, beyond the focus's reach.
        let (at, body, shape) = (
            crabby_transform3d_at(v3(300.0, 0.0, 0.0)),
            crabby_rigid_body3d(CrabbyBodyKind::CRABBY_BODY_FIXED as u32),
            crabby_collider3d_box(v3(1.0, 1.0, 1.0)),
        );
        let values = [
            value(ids.transform, &at),
            value(ids.body, &body),
            value(ids.collider, &shape),
        ];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 3, &mut ids.far_box), CRABBY_OK);
        let (at, focus) = (
            crabby_transform3d_at(v3(0.0, 0.0, 0.0)),
            crabby_collider_focus(50.0, 80.0),
        );
        let values = [value(ids.transform, &at), value(ids.focus, &focus)];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut ids.focus_entity), CRABBY_OK);
    }
}

unsafe extern "C" fn watch(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let active = |e: u64| {
            let mut a = false;
            assert_eq!(crabby_ctx_physics3d_is_collider_active(ctx, e, &mut a), CRABBY_OK);
            a
        };
        match crabby_ctx_sim_tick(ctx) {
            0 => {
                ids.first = Some((active(ids.ground), active(ids.far_box)));
                // The focus walks to the wall.
                let w = crabby_ctx_world(ctx);
                let at = crabby_transform3d_at(v3(290.0, 0.0, 0.0));
                let size = size_of::<CrabbyTransform3D>();
                assert_eq!(
                    crabby_set(w, ids.focus_entity, ids.transform, (&raw const at).cast(), size),
                    CRABBY_OK
                );
            }
            1 => ids.later = Some(active(ids.far_box)),
            _ => {}
        }
    }
}

#[test]
fn c_terrain_colliders_hold_a_ball_and_static_colliders_follow_the_focus() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"stream3d".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        assert_eq!(
            crabby_game_sim_system(g, c"watch".as_ptr(), 1, Some(watch), user),
            CRABBY_OK
        );
        assert_eq!(crabby_game_run_headless(g, 2, Some(setup), user, &mut world), CRABBY_OK);
        assert_eq!(ids.first, Some((true, false)), "the ground in, the far wall out");
        assert_eq!(ids.later, Some(true), "the wall came in as the focus reached it");
        // The components read back as written.
        let mut terrain = crabby_terrain_collider(CrabbyCollisionMesh::default());
        let size = size_of::<CrabbyTerrainCollider>();
        assert_eq!(
            crabby_get(world, ids.ground, ids.terrain, (&raw mut terrain).cast(), size),
            CRABBY_OK
        );
        assert_eq!(terrain.friction, 0.5);
        let mut focus = CrabbyColliderFocus::default();
        let size = size_of::<CrabbyColliderFocus>();
        assert_eq!(
            crabby_get(world, ids.focus_entity, ids.focus, (&raw mut focus).cast(), size),
            CRABBY_OK
        );
        assert_eq!(focus, crabby_collider_focus(50.0, 80.0));
        // Bad values are refused.
        terrain.friction = -1.0;
        let size = size_of::<CrabbyTerrainCollider>();
        assert_eq!(
            crabby_set(world, ids.ground, ids.terrain, (&raw const terrain).cast(), size),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_world_free(world);
    }
}

#[test]
fn ball_lands_on_terrain_built_from_c() {
    // The landing itself, longer: the ball rests on the 2 m ground.
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"land".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        assert_eq!(
            crabby_game_run_headless(g, 120, Some(setup), user, &mut world),
            CRABBY_OK
        );
        let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let size = size_of::<CrabbyTransform3D>();
        assert_eq!(
            crabby_get(world, ids.ball, ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        assert!(
            (t.position.y - 2.5).abs() < 0.05,
            "rests on the ground: {}",
            t.position.y
        );
        crabby_world_free(world);
    }
}

#[test]
fn collision_mesh_data_is_checked() {
    unsafe {
        let h = [0.0f32; 9];
        assert!(crabby_collision_mesh_data_heightfield(h.as_ptr(), 9, -1.0).is_null());
        assert!(crabby_collision_mesh_data_heightfield(null(), 9, 1.0).is_null());
        let v = [v3(0.0, 0.0, 0.0), v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0)];
        assert!(crabby_collision_mesh_data_trimesh(v.as_ptr(), 3, [0u32, 1, 7].as_ptr(), 3).is_null());
        assert!(crabby_collision_mesh_data_trimesh(v.as_ptr(), 3, [0u32, 1].as_ptr(), 2).is_null());
        let p = [v[0], v[1], v[2], v3(0.0, 1.0, 0.0)];
        let hull = crabby_collision_mesh_data_convex_hull(p.as_ptr(), 4);
        assert!(!hull.is_null());
        crabby_collision_mesh_data_free(hull);
        let mut mesh = CrabbyCollisionMesh::default();
        assert_eq!(
            crabby_ctx_physics3d_add_mesh(null_mut(), null(), &mut mesh),
            CRABBY_INVALID_ARGUMENT
        );
    }
}
