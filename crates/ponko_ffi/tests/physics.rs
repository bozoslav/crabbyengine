//! Physics from C: body/collider/velocity components, the game setting,
//! queries, impulses and collision events.
#![cfg(feature = "physics")]
use ponko_ffi::*;
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
    ball: u64,
    floor: u64,
    sensor: u64,
    /// Collisions seen: (a, b, sensor, started).
    seen: Vec<(u64, u64, bool, bool)>,
    hit: Option<CrabbyRayHit>,
    at_floor: Vec<u64>,
    mass: f32,
}

fn value<T>(component: u32, v: &T) -> CrabbyValue {
    CrabbyValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}
fn at(x: f32, y: f32) -> CrabbyTransform2D {
    CrabbyTransform2D {
        position: CrabbyVec2 { x, y },
        ..Default::default()
    }
}
fn bits(e: CrabbyEntity) -> u64 {
    crabby_entity_bits(e)
}

unsafe fn ids_of(w: *mut CrabbyWorld, ids: &mut Ids) {
    unsafe {
        assert_eq!(crabby_transform_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_rigid_body_component(w, &mut ids.body), CRABBY_OK);
        assert_eq!(crabby_collider_component(w, &mut ids.collider), CRABBY_OK);
        assert_eq!(crabby_velocity_component(w, &mut ids.velocity), CRABBY_OK);
    }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        ids_of(w, ids);
        // Every value is a named local: `value` keeps only a raw pointer, so a
        // temporary would be gone before crabby_spawn reads it.
        let dynamic = crabby_rigid_body(CRABBY_BODY_DYNAMIC as u32);
        let fixed = crabby_rigid_body(CRABBY_BODY_FIXED as u32);
        let (ball_at, ball_shape, still) = (at(0.0, 0.0), crabby_collider_circle(10.0), CrabbyVelocity::default());
        let ball = [
            value(ids.transform, &ball_at),
            value(ids.body, &dynamic),
            value(ids.collider, &ball_shape),
            value(ids.velocity, &still),
        ];
        assert_eq!(crabby_spawn(w, ball.as_ptr(), 4, &mut ids.ball), CRABBY_OK);
        let mut trigger = crabby_collider_rect(CrabbyVec2 { x: 80.0, y: 20.0 });
        trigger.sensor = true;
        let sensor_at = at(0.0, 100.0);
        let sensor = [
            value(ids.transform, &sensor_at),
            value(ids.body, &fixed),
            value(ids.collider, &trigger),
        ];
        assert_eq!(crabby_spawn(w, sensor.as_ptr(), 3, &mut ids.sensor), CRABBY_OK);
        let (floor_at, floor_shape) = (at(0.0, 300.0), crabby_collider_rect(CrabbyVec2 { x: 400.0, y: 20.0 }));
        let floor = [
            value(ids.transform, &floor_at),
            value(ids.body, &fixed),
            value(ids.collider, &floor_shape),
        ];
        assert_eq!(crabby_spawn(w, floor.as_ptr(), 3, &mut ids.floor), CRABBY_OK);
    }
}

unsafe extern "C" fn watch(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        for started in [true, false] {
            let (mut data, mut count) = (null(), 0);
            assert_eq!(crabby_ctx_collisions(ctx, started, &mut data, &mut count), CRABBY_OK);
            for c in std::slice::from_raw_parts(data, count) {
                ids.seen.push((bits(c.a), bits(c.b), c.sensor, started));
            }
        }
        if crabby_ctx_sim_tick(ctx) == 1 {
            // Bodies exist after the first step: ask about them.
            let (mut found, mut hit) = (false, CrabbyRayHit::default());
            let down = CrabbyVec2 { x: 0.0, y: 1.0 };
            let from = CrabbyVec2 { x: 150.0, y: 0.0 };
            assert_eq!(
                crabby_ctx_cast_ray(ctx, from, down, 1000.0, &mut found, &mut hit),
                CRABBY_OK
            );
            ids.hit = found.then_some(hit);
            let mut buf = [CrabbyEntity::default(); 1];
            let mut n = 0;
            let p = CrabbyVec2 { x: 150.0, y: 300.0 };
            assert_eq!(crabby_ctx_physics_at(ctx, p, buf.as_mut_ptr(), 1, &mut n), CRABBY_OK);
            ids.at_floor = buf[..n.min(1)].iter().map(|e| bits(*e)).collect();
            assert_eq!(crabby_ctx_body_mass(ctx, ids.ball, &mut ids.mass), CRABBY_OK);
            let mut none = 0.0;
            assert_eq!(crabby_ctx_body_mass(ctx, 12345, &mut none), CRABBY_NOT_FOUND);
            assert_eq!(
                crabby_ctx_apply_impulse(ctx, 12345, CrabbyVec2 { x: 1.0, y: 0.0 }),
                CRABBY_NOT_FOUND
            );
        }
    }
}

#[test]
fn c_bodies_fall_collide_and_answer_queries() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"physics".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics(g, CrabbyVec2 { x: 0.0, y: 980.0 }), CRABBY_OK);
        assert_eq!(
            crabby_game_sim_system(g, c"watch".as_ptr(), 1, Some(watch), user),
            CRABBY_OK
        );
        assert_eq!(
            crabby_game_run_headless(g, 120, Some(setup), user, &mut world),
            CRABBY_OK
        );

        let mut t = CrabbyTransform2D::default();
        let size = size_of::<CrabbyTransform2D>();
        assert_eq!(
            crabby_get(world, ids.ball, ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        assert!(
            (t.position.y - 280.0).abs() < 2.0,
            "rests on the floor: {}",
            t.position.y
        );
        let mut v = CrabbyVelocity::default();
        let size = size_of::<CrabbyVelocity>();
        assert_eq!(
            crabby_get(world, ids.ball, ids.velocity, (&raw mut v).cast(), size),
            CRABBY_OK
        );
        assert!(v.linear.y.abs() < 5.0, "at rest: {:?}", v.linear);

        let pair = |x: u64, y: u64, started: bool| {
            ids.seen
                .iter()
                .find(|s| ((s.0 == x && s.1 == y) || (s.0 == y && s.1 == x)) && s.3 == started)
                .copied()
        };
        assert!(pair(ids.ball, ids.sensor, true).expect("entered the sensor").2);
        assert!(pair(ids.ball, ids.sensor, false).is_some(), "and left it");
        assert!(!pair(ids.ball, ids.floor, true).expect("hit the floor").2);

        let hit = ids.hit.expect("the ray hits the floor");
        assert_eq!(bits(hit.entity), ids.floor);
        assert!((hit.distance - 290.0).abs() < 1e-3, "{}", hit.distance);
        assert!((hit.normal.y + 1.0).abs() < 1e-3);
        assert_eq!(ids.at_floor, [ids.floor]);
        assert!((ids.mass - std::f32::consts::PI * 100.0).abs() < 1.0, "{}", ids.mass);
        crabby_world_free(world);
    }
}

#[test]
fn bodies_and_colliders_round_trip_and_are_checked() {
    let mut ids = Ids::default();
    unsafe {
        let w = crabby_world_new();
        ids_of(w, &mut ids);
        let mut body = crabby_rigid_body(CRABBY_BODY_KINEMATIC as u32);
        body.linear_damping = 0.5;
        body.lock_rotation = true;
        let mut capsule = crabby_collider_capsule(20.0, 6.0);
        capsule.offset = CrabbyVec2 { x: 1.0, y: 2.0 };
        capsule.restitution = 0.25;
        let values = [value(ids.body, &body), value(ids.collider, &capsule)];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut e), CRABBY_OK);

        let mut back = crabby_rigid_body(0);
        let n = size_of::<CrabbyRigidBody>();
        assert_eq!(crabby_get(w, e, ids.body, (&raw mut back).cast(), n), CRABBY_OK);
        assert_eq!(back, body);
        let mut shape = crabby_collider_circle(1.0);
        let n = size_of::<CrabbyCollider>();
        assert_eq!(crabby_get(w, e, ids.collider, (&raw mut shape).cast(), n), CRABBY_OK);
        assert_eq!(shape, capsule);

        let bad_kind = CrabbyRigidBody { kind: 7, ..body };
        let n = size_of::<CrabbyRigidBody>();
        assert_eq!(
            crabby_set(w, e, ids.body, (&raw const bad_kind).cast(), n),
            CRABBY_INVALID_ARGUMENT
        );
        let flat = crabby_collider_rect(CrabbyVec2 { x: 10.0, y: 0.0 });
        let n = size_of::<CrabbyCollider>();
        assert_eq!(
            crabby_set(w, e, ids.collider, (&raw const flat).cast(), n),
            CRABBY_INVALID_ARGUMENT
        );
        let values = [value(ids.body, &body), value(ids.collider, &flat)];
        let mut none = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut none), CRABBY_INVALID_ARGUMENT);
        let mut count = 0;
        assert_eq!(crabby_world_len(w, &mut count), CRABBY_OK);
        assert_eq!(count, 1, "a rejected spawn spawns nothing");
        assert_eq!(crabby_rigid_body(99).kind, CRABBY_BODY_DYNAMIC as u32);

        // Bodies need conversion, so they can't be query columns; velocity can.
        unsafe extern "C" fn nothing(_: *const CrabbyChunk, _: *mut c_void) {}
        let cols = [ids.body];
        let s = crabby_query(w, cols.as_ptr(), 1, Some(nothing), null_mut());
        assert_eq!(s, CRABBY_INVALID_ARGUMENT);
        let cols = [ids.velocity];
        assert_eq!(crabby_query(w, cols.as_ptr(), 1, Some(nothing), null_mut()), CRABBY_OK);
        crabby_world_free(w);
    }
}

#[test]
fn polygon_colliders_are_set_and_read_by_their_corners() {
    let mut ids = Ids::default();
    unsafe {
        let w = crabby_world_new();
        ids_of(w, &mut ids);
        let mut e = 0;
        assert_eq!(crabby_spawn(w, null(), 0, &mut e), CRABBY_OK);
        let v = |x, y| CrabbyVec2 { x, y };
        let wedge = [v(-30.0, 15.0), v(30.0, 15.0), v(-30.0, -15.0)];
        let mut settings = crabby_collider_circle(1.0);
        settings.friction = 0.9;
        assert_eq!(
            crabby_set_polygon_collider(w, e, wedge.as_ptr(), 3, settings),
            CRABBY_OK
        );

        let mut corners = [v(0.0, 0.0); 8];
        let mut n = 0;
        assert_eq!(crabby_collider_points(w, e, corners.as_mut_ptr(), &mut n), CRABBY_OK);
        assert_eq!(&corners[..n as usize], &wedge);

        // Read as a plain collider: the polygon's kind, bounding box and settings.
        let mut c = crabby_collider_circle(1.0);
        let size = size_of::<CrabbyCollider>();
        assert_eq!(crabby_get(w, e, ids.collider, (&raw mut c).cast(), size), CRABBY_OK);
        assert_eq!(c.shape, CrabbyShapeKind::CRABBY_SHAPE_POLYGON as u32);
        assert_eq!((c.size, c.friction), (v(60.0, 30.0), 0.9));
        // Written back as a plain collider it would lose its corners: refused.
        assert_eq!(
            crabby_set(w, e, ids.collider, (&raw const c).cast(), size),
            CRABBY_INVALID_ARGUMENT
        );

        assert_eq!(
            crabby_set_polygon_collider(w, e, wedge.as_ptr(), 2, settings),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_set_polygon_collider(w, e, wedge.as_ptr(), 9, settings),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_set_polygon_collider(w, 12345, wedge.as_ptr(), 3, settings),
            CRABBY_NOT_FOUND
        );
        // Other shapes have no corners.
        let ball = crabby_collider_circle(4.0);
        assert_eq!(
            crabby_set(w, e, ids.collider, (&raw const ball).cast(), size),
            CRABBY_OK
        );
        assert_eq!(crabby_collider_points(w, e, corners.as_mut_ptr(), &mut n), CRABBY_OK);
        assert_eq!(n, 0);
        crabby_world_free(w);
    }
}
