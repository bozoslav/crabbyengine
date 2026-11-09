//! 3D joints from C (one body pair per joint kind), the capsule cast and box
//! and capsule overlap queries, and heightfields for finding the ground on
//! the CPU. Headless, with 3D physics stepping.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyJointMotionKind::*;
use CrabbyStatus::*;

const OK: CrabbyStatus = CRABBY_OK;
const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;

fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}

fn value<T>(component: u32, v: &T) -> CrabbyValue {
    CrabbyValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}

fn kinds(j: &CrabbyJoint3D) -> [CrabbyJointMotionKind; 6] {
    j.axes.map(|a| a.kind)
}

#[test]
fn each_joint_kind_locks_the_axes_it_should() {
    let (a, b) = (3, 4);
    let fixed = crabby_joint3d_fixed(a, b);
    assert_eq!((fixed.body_a, fixed.body_b), (a, b));
    assert_eq!(kinds(&fixed), [CRABBY_JOINT_LOCKED; 6]);
    assert_eq!(
        kinds(&crabby_joint3d_spherical(a, b)),
        [
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_FREE,
            CRABBY_JOINT_FREE,
            CRABBY_JOINT_FREE
        ]
    );
    assert_eq!(
        kinds(&crabby_joint3d_revolute(a, b)),
        [
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_FREE,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED
        ]
    );
    assert_eq!(
        kinds(&crabby_joint3d_prismatic(a, b)),
        [
            CRABBY_JOINT_FREE,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED,
            CRABBY_JOINT_LOCKED
        ]
    );
    let generic = crabby_joint3d_generic(a, b);
    assert_eq!(kinds(&generic), [CRABBY_JOINT_FREE; 6]);
    // Every kind starts without motors, without contact between its bodies,
    // with no swing cone, and with its frames at the bodies' centers.
    for j in [fixed, generic, crabby_joint3d_revolute(a, b)] {
        assert!(j.drives.iter().all(|d| !d.enabled));
        assert!(!j.contacts_enabled);
        assert_eq!(j.swing_cone, -1.0);
        assert_eq!(j.frame_a.position, v3(0.0, 0.0, 0.0));
        assert_eq!(j.frame_b.rotation.w, 1.0);
    }
    let all = crabby_collision_groups_all();
    assert_eq!((all.memberships, all.filter), (u32::MAX, u32::MAX));
}

#[derive(Default)]
struct Joints {
    transform: u32,
    body: u32,
    collider: u32,
    velocity: u32,
    joint: u32,
    /// The jointed body of each pair, in the order of `NAMES`.
    bodies: [u64; 5],
    pivots: [CrabbyVec3; 5],
    /// The entity of the generic joint, despawned during the run.
    generic_joint: u64,
    counts: Vec<usize>,
}

const NAMES: [&str; 5] = ["fixed", "revolute", "prismatic", "spherical", "generic"];

unsafe fn ids(w: *mut CrabbyWorld, j: &mut Joints) {
    unsafe {
        assert_eq!(crabby_transform3d_component(w, &mut j.transform), OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut j.body), OK);
        assert_eq!(crabby_collider3d_component(w, &mut j.collider), OK);
        assert_eq!(crabby_velocity3d_component(w, &mut j.velocity), OK);
        assert_eq!(crabby_joint3d_component(w, &mut j.joint), OK);
    }
}

/// A pair: an anchor held in place at `anchor`, and a ball at `ball` with this velocity.
unsafe fn pair(
    ctx: *mut CrabbyCtx,
    j: &Joints,
    anchor: CrabbyVec3,
    ball: CrabbyVec3,
    velocity: CrabbyVec3,
) -> (u64, u64) {
    unsafe {
        let w = crabby_ctx_world(ctx);
        let shape = crabby_collider3d_sphere(0.2);
        let at = crabby_transform3d_at(anchor);
        let fixed = crabby_rigid_body3d(CrabbyBodyKind::CRABBY_BODY_FIXED as u32);
        let parts = [
            value(j.transform, &at),
            value(j.body, &fixed),
            value(j.collider, &shape),
        ];
        let mut a = 0u64;
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 3, &mut a), OK);

        let at = crabby_transform3d_at(ball);
        let dynamic = crabby_rigid_body3d(CrabbyBodyKind::CRABBY_BODY_DYNAMIC as u32);
        let moving = CrabbyVelocity3D {
            linear: velocity,
            angular: v3(0.0, 0.0, 0.0),
        };
        let parts = [
            value(j.transform, &at),
            value(j.body, &dynamic),
            value(j.collider, &shape),
            value(j.velocity, &moving),
        ];
        let mut b = 0u64;
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 4, &mut b), OK);
        (a, b)
    }
}

unsafe fn join(ctx: *mut CrabbyCtx, j: &Joints, joint: CrabbyJoint3D) -> u64 {
    unsafe {
        let part = value(j.joint, &joint);
        let mut e = 0u64;
        assert_eq!(crabby_spawn(crabby_ctx_world(ctx), &part, 1, &mut e), OK);
        e
    }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let j = &mut *user.cast::<Joints>();
        ids(crabby_ctx_world(ctx), j);
        let rest = v3(0.0, 0.0, 0.0);

        // Fixed: the ball hangs 1 m below its anchor and stays there, though
        // gravity pulls it.
        let (a, b) = pair(ctx, j, v3(0.0, 20.0, 0.0), v3(0.0, 19.0, 0.0), rest);
        let mut joint = crabby_joint3d_fixed(a, b);
        joint.frame_a = crabby_transform3d_at(v3(0.0, -1.0, 0.0));
        join(ctx, j, joint);
        (j.bodies[0], j.pivots[0]) = (b, v3(0.0, 20.0, 0.0));

        // Revolute: a hinge around x. The ball starts level with its pivot,
        // 1 m out along z, and swings down in the y-z plane.
        let (a, b) = pair(ctx, j, v3(10.0, 20.0, 0.0), v3(10.0, 20.0, 1.0), rest);
        let mut joint = crabby_joint3d_revolute(a, b);
        joint.frame_b = crabby_transform3d_at(v3(0.0, 0.0, -1.0));
        join(ctx, j, joint);
        (j.bodies[1], j.pivots[1]) = (b, v3(10.0, 20.0, 0.0));

        // Prismatic: a slider along x. Thrown along x it slides; gravity can't pull it down.
        let (a, b) = pair(ctx, j, v3(20.0, 20.0, 0.0), v3(20.0, 20.0, 0.0), v3(2.0, 0.0, 0.0));
        join(ctx, j, crabby_joint3d_prismatic(a, b));
        (j.bodies[2], j.pivots[2]) = (b, v3(20.0, 20.0, 0.0));

        // Spherical: a ball on a 1 m rope, thrown sideways, swings on a sphere around its anchor.
        let (a, b) = pair(ctx, j, v3(30.0, 20.0, 0.0), v3(30.0, 19.0, 0.0), v3(3.0, 0.0, 0.0));
        let mut joint = crabby_joint3d_spherical(a, b);
        joint.frame_b = crabby_transform3d_at(v3(0.0, 1.0, 0.0));
        join(ctx, j, joint);
        (j.bodies[3], j.pivots[3]) = (b, v3(30.0, 20.0, 0.0));

        // Generic with every axis free: no constraint at all, so the ball falls.
        let (a, b) = pair(ctx, j, v3(40.0, 20.0, 0.0), v3(40.0, 19.0, 0.0), rest);
        j.generic_joint = join(ctx, j, crabby_joint3d_generic(a, b));
        (j.bodies[4], j.pivots[4]) = (b, v3(40.0, 20.0, 0.0));
    }
}

unsafe extern "C" fn count(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let j = &mut *user.cast::<Joints>();
        let mut n = 99;
        assert_eq!(crabby_ctx_physics3d_joint_count(ctx, &mut n), OK);
        j.counts.push(n);
        // Despawning a joint removes it from the physics world at the next step.
        if crabby_ctx_sim_tick(ctx) == 5 {
            assert_eq!(crabby_despawn(crabby_ctx_world(ctx), j.generic_joint), OK);
        }
    }
}

fn position(world: *mut CrabbyWorld, j: &Joints, e: u64) -> CrabbyVec3 {
    unsafe {
        let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let size = size_of::<CrabbyTransform3D>();
        assert_eq!(crabby_get(world, e, j.transform, (&raw mut t).cast(), size), OK);
        t.position
    }
}

fn distance(a: CrabbyVec3, b: CrabbyVec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

#[test]
fn bodies_move_as_their_joint_allows() {
    let mut j = Joints::default();
    let user = (&raw mut j).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"joints".as_ptr(), 1, 1);
        assert_eq!(crabby_game_fixed_hz(g, 60), OK);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), OK);
        assert_eq!(crabby_game_sim_system(g, c"count".as_ptr(), 1, Some(count), user), OK);
        assert_eq!(crabby_game_run_headless(g, 60, Some(setup), user, &mut world), OK);
    }
    let at: Vec<CrabbyVec3> = j.bodies.iter().map(|&b| position(world, &j, b)).collect();
    let name = |i: usize| NAMES[i];
    let free_fall = 0.5 * 9.81 * 1.0;
    // Fixed: the ball is where it hung (a free ball would be 4.9 m lower).
    assert!(
        distance(at[0], v3(0.0, 19.0, 0.0)) < 0.1,
        "{}: {:?}",
        name(0),
        (at[0].x, at[0].y, at[0].z)
    );
    // Revolute: stayed in the plane of the hinge, on its circle, and swung down.
    let p = j.pivots[1];
    assert!((at[1].x - p.x).abs() < 0.05, "{}: x {}", name(1), at[1].x);
    assert!(
        (distance(at[1], p) - 1.0).abs() < 0.1,
        "{}: {} from its pivot",
        name(1),
        distance(at[1], p)
    );
    assert!(at[1].y < p.y - 0.05, "{}: y {}", name(1), at[1].y);
    // Prismatic: slid along x, and neither fell nor drifted sideways.
    let p = j.pivots[2];
    assert!(at[2].x > p.x + 1.0, "{}: x {}", name(2), at[2].x);
    assert!(
        (at[2].y - p.y).abs() < 0.05 && (at[2].z - p.z).abs() < 0.05,
        "{}: {:?}",
        name(2),
        (at[2].x, at[2].y, at[2].z)
    );
    // Spherical: swung off its hanging spot (a free throw would be 3 m away by now), and still 1 m from the anchor.
    let p = j.pivots[3];
    assert!(
        (distance(at[3], p) - 1.0).abs() < 0.1,
        "{}: {} from its anchor",
        name(3),
        distance(at[3], p)
    );
    assert!((at[3].x - p.x).abs() > 0.1, "{}: x {}", name(3), at[3].x);
    // Generic with nothing locked: free fall.
    assert!(
        (at[4].y - (19.0 - free_fall)).abs() < 0.3 && at[4].y < 15.0,
        "{}: y {}",
        name(4),
        at[4].y
    );
    // The world counted the five joints, then four after one was despawned.
    assert_eq!(*j.counts.last().unwrap(), 4, "{:?}", j.counts);
    assert!(j.counts[2..5].iter().all(|&n| n == 5), "{:?}", j.counts);
    unsafe {
        crabby_world_free(world);
    }
}

#[test]
fn the_joint_count_is_an_error_without_a_context_or_a_place_to_write() {
    unsafe {
        let mut n = 0;
        assert_eq!(crabby_ctx_physics3d_joint_count(null_mut(), &mut n), BAD);
    }
}

// ---- shape queries ----------------------------------------------------------------

#[derive(Default)]
struct Queries {
    transform: u32,
    body: u32,
    collider: u32,
    wall: u64,
    post: u64,
    sensor: u64,
    cast_hit: Option<CrabbyHit3D>,
    cast_miss: bool,
    box_hits: Vec<u64>,
    box_total: usize,
    box_turned: Vec<u64>,
    capsule_hits: Vec<u64>,
    capsule_miss: usize,
    asked: bool,
}

unsafe fn solid(ctx: *mut CrabbyCtx, q: &Queries, at: CrabbyVec3, shape: CrabbyCollider3D) -> u64 {
    unsafe {
        let t = crabby_transform3d_at(at);
        let fixed = crabby_rigid_body3d(CrabbyBodyKind::CRABBY_BODY_FIXED as u32);
        let parts = [value(q.transform, &t), value(q.body, &fixed), value(q.collider, &shape)];
        let mut e = 0u64;
        assert_eq!(crabby_spawn(crabby_ctx_world(ctx), parts.as_ptr(), 3, &mut e), OK);
        e
    }
}

unsafe extern "C" fn build(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let q = &mut *user.cast::<Queries>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform3d_component(w, &mut q.transform), OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut q.body), OK);
        assert_eq!(crabby_collider3d_component(w, &mut q.collider), OK);
        // A wall 1 m thick at x = 10, 10 m tall and wide; a thin post at x = -6.
        q.wall = solid(ctx, q, v3(10.0, 0.0, 0.0), crabby_collider3d_box(v3(1.0, 10.0, 10.0)));
        q.post = solid(ctx, q, v3(-6.0, 0.0, 0.0), crabby_collider3d_box(v3(0.5, 4.0, 0.5)));
        // A sensor at the origin: seen only when the filter asks for sensors.
        let mut sensor = crabby_collider3d_sphere(1.0);
        sensor.sensor = true;
        q.sensor = solid(ctx, q, v3(0.0, 0.0, 0.0), sensor);
    }
}

unsafe extern "C" fn ask(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let q = &mut *user.cast::<Queries>();
        if crabby_ctx_sim_tick(ctx) != 1 {
            return;
        }
        let filter = crabby_query_filter3d();
        let bits = |list: &[CrabbyEntity], n: usize| -> Vec<u64> {
            let mut v: Vec<u64> = list[..n.min(list.len())]
                .iter()
                .map(|e| crabby_entity_bits(*e))
                .collect();
            v.sort();
            v
        };

        // A capsule 2 m tall (the distance between its ends) and 0.5 m wide
        // flies along +x from the origin and hits the wall's face at x = 9.5.
        let (mut found, mut hit) = (false, CrabbyHit3D::default());
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(1.0, 0.0, 0.0),
                30.0,
                filter,
                &mut found,
                &mut hit
            ),
            OK
        );
        q.cast_hit = found.then_some(hit);
        // Aimed away from the wall and the post, it finds nothing.
        let mut missed = true;
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(0.0, 1.0, 0.0),
                30.0,
                filter,
                &mut missed,
                &mut hit
            ),
            OK
        );
        q.cast_miss = !missed;
        // Too short a reach for the wall.
        let mut short = true;
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(1.0, 0.0, 0.0),
                5.0,
                filter,
                &mut short,
                &mut hit
            ),
            OK
        );
        assert!(!short);
        // The sensor at the origin is in the way of a cast that starts inside it, only if asked for.
        let mut with_sensors = filter;
        with_sensors.sensors = true;
        let mut inside = false;
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(0.0, 1.0, 0.0),
                30.0,
                with_sensors,
                &mut inside,
                &mut hit
            ),
            OK
        );
        assert!(inside, "sensors count when the filter says so");
        // The wall can be looked through.
        let mut skip_wall = filter;
        skip_wall.exclude = true;
        skip_wall.exclude_entity = q.wall;
        let mut through = true;
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(1.0, 0.0, 0.0),
                30.0,
                skip_wall,
                &mut through,
                &mut hit
            ),
            OK
        );
        assert!(!through);
        let (mut f, mut h) = (true, CrabbyHit3D::default());
        let cast_bad = |origin, height, radius, dir, max, out_found: *mut bool, out_hit| {
            crabby_ctx_cast_capsule3d(ctx, origin, height, radius, dir, max, filter, out_found, out_hit)
        };
        // A cast with no direction goes nowhere; a radius of 0 is a hair's width, not an error.
        assert_eq!(
            cast_bad(v3(0.0, 0.0, 0.0), 2.0, 0.5, v3(0.0, 0.0, 0.0), 5.0, &mut f, &mut h),
            OK
        );
        assert!(!f);
        assert_eq!(
            cast_bad(v3(0.0, 0.0, 0.0), 2.0, 0.0, v3(1.0, 0.0, 0.0), 5.0, &mut f, &mut h),
            OK
        );
        assert_eq!(
            cast_bad(v3(0.0, 0.0, 0.0), 2.0, 0.5, v3(1.0, 0.0, 0.0), 5.0, null_mut(), &mut h),
            BAD
        );
        assert_eq!(
            cast_bad(v3(0.0, 0.0, 0.0), 2.0, 0.5, v3(1.0, 0.0, 0.0), 5.0, &mut f, null_mut()),
            BAD
        );
        assert_eq!(
            crabby_ctx_cast_capsule3d(
                null_mut(),
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                v3(1.0, 0.0, 0.0),
                5.0,
                filter,
                &mut f,
                &mut h
            ),
            BAD
        );

        // A box 2 m across at x = 9 reaches into the wall (its face is at 9.5); at x = 7 it doesn't.
        let still = crabby_quat_identity();
        let mut list = [CrabbyEntity::default(); 8];
        let mut n = 0;
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(9.0, 0.0, 0.0),
                v3(2.0, 2.0, 2.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        q.box_hits = bits(&list, n);
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(7.0, 0.0, 0.0),
                v3(2.0, 2.0, 2.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        q.box_total = n;
        // A long thin box along x (12 m by 0.2 m), turned a quarter around y so it points along z
        // instead: it no longer reaches the wall from x = 4.
        let long = v3(12.0, 0.2, 0.2);
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(4.0, 0.0, 0.0),
                long,
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        assert_eq!(bits(&list, n), [q.wall], "the long box reaches the wall");
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let quarter = CrabbyQuat {
            x: 0.0,
            y: half,
            z: 0.0,
            w: half,
        };
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(4.0, 0.0, 0.0),
                long,
                quarter,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        q.box_turned = bits(&list, n);
        // The sensor is only listed when asked for; the capacity only limits what is written.
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(1.0, 1.0, 1.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        assert_eq!(n, 0, "sensors are skipped by default");
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(1.0, 1.0, 1.0),
                still,
                with_sensors,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK
        );
        assert_eq!(bits(&list, n), [q.sensor]);
        // Counting only: no list, no room.
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(100.0, 100.0, 100.0),
                still,
                with_sensors,
                null_mut(),
                0,
                &mut n
            ),
            OK
        );
        assert_eq!(n, 3, "the wall, the post and the sensor");
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(100.0, 100.0, 100.0),
                still,
                with_sensors,
                list.as_mut_ptr(),
                1,
                &mut n
            ),
            OK
        );
        assert_eq!(n, 3, "the total, though one fits");
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(50.0, 0.0, 0.0),
                v3(0.0, 1.0, 1.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            OK,
            "a flat box is a hair's width, not an error"
        );
        assert_eq!(n, 0);
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(1.0, 1.0, 1.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                null_mut()
            ),
            BAD
        );
        assert_eq!(
            crabby_ctx_overlap_box3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                v3(1.0, 1.0, 1.0),
                still,
                filter,
                null_mut(),
                8,
                &mut n
            ),
            BAD,
            "room claimed but no list"
        );
        assert_eq!(
            crabby_ctx_overlap_box3d(
                null_mut(),
                v3(0.0, 0.0, 0.0),
                v3(1.0, 1.0, 1.0),
                still,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            BAD
        );

        // An upright capsule 2 m tall and 0.5 m wide standing at x = -5.5 touches the post's side
        // (the post spans x -6.25 to -5.75); one at x = -4 doesn't.
        assert_eq!(
            crabby_ctx_overlap_capsule3d(ctx, v3(-5.5, 0.0, 0.0), 2.0, 0.5, filter, list.as_mut_ptr(), 8, &mut n),
            OK
        );
        q.capsule_hits = bits(&list, n);
        assert_eq!(
            crabby_ctx_overlap_capsule3d(ctx, v3(-4.0, 0.0, 0.0), 2.0, 0.5, filter, list.as_mut_ptr(), 8, &mut n),
            OK
        );
        q.capsule_miss = n;
        assert_eq!(
            crabby_ctx_overlap_capsule3d(ctx, v3(50.0, 0.0, 0.0), 2.0, 0.0, filter, list.as_mut_ptr(), 8, &mut n),
            OK,
            "no radius is a hair's width, not an error"
        );
        assert_eq!(n, 0);
        assert_eq!(
            crabby_ctx_overlap_capsule3d(
                ctx,
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                filter,
                list.as_mut_ptr(),
                8,
                null_mut()
            ),
            BAD
        );
        assert_eq!(
            crabby_ctx_overlap_capsule3d(
                null_mut(),
                v3(0.0, 0.0, 0.0),
                2.0,
                0.5,
                filter,
                list.as_mut_ptr(),
                8,
                &mut n
            ),
            BAD
        );
        q.asked = true;
    }
}

#[test]
fn capsule_casts_and_box_and_capsule_overlaps_find_what_they_touch() {
    let mut q = Queries::default();
    let user = (&raw mut q).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"queries".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), OK);
        assert_eq!(crabby_game_sim_system(g, c"ask".as_ptr(), 1, Some(ask), user), OK);
        assert_eq!(crabby_game_run_headless(g, 2, Some(build), user, &mut world), OK);
        crabby_world_free(world);
    }
    assert!(q.asked);
    let hit = q.cast_hit.expect("the wall is in the capsule's way");
    assert_eq!(crabby_entity_bits(hit.entity), q.wall);
    // The capsule's side is 0.5 m wide, so it stops 0.5 m from the face at x = 9.5.
    assert!((hit.distance - 9.0).abs() < 0.1, "{}", hit.distance);
    assert!(
        hit.normal.x < -0.9,
        "the face points back at the capsule: {:?}",
        (hit.normal.x, hit.normal.y)
    );
    assert!(q.cast_miss, "nothing above the origin");
    assert_eq!(q.box_hits, [q.wall]);
    assert_eq!(q.box_total, 0);
    assert!(!q.box_turned.contains(&q.wall), "{:?}", q.box_turned);
    assert_eq!(q.capsule_hits, [q.post]);
    assert_eq!(q.capsule_miss, 0);
}

// ---- heightfields ----------------------------------------------------------------

#[test]
fn heightfields_answer_height_and_slope_at_a_point() {
    unsafe {
        // 3 x 3 heights over 4 m, so the cells are 2 m: flat but for a 2 m hill in the middle.
        let heights = [0.0f32, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0];
        let ground = crabby_heightfield_new(heights.as_ptr(), 9, 4.0);
        assert!(!ground.is_null());
        let (mut found, mut y, mut n) = (false, -1.0f32, v3(0.0, 0.0, 0.0));
        let at =
            |x, z, found: &mut bool, y: &mut f32, n: *mut CrabbyVec3| crabby_heightfield_at(ground, x, z, found, y, n);

        assert_eq!(at(2.0, 2.0, &mut found, &mut y, &mut n), OK);
        assert!(found);
        assert!((y - 2.0).abs() < 1e-5, "the top of the hill: {y}");
        // Halfway down the slope along a grid line the height is halfway.
        assert_eq!(at(1.0, 2.0, &mut found, &mut y, &mut n), OK);
        assert!(found && (y - 1.0).abs() < 1e-5, "{y}");
        assert_eq!(at(2.0, 3.0, &mut found, &mut y, &mut n), OK);
        assert!(found && (y - 1.0).abs() < 1e-5, "{y}");
        // The slope leans away from the hill: the normal tilts toward -x on the -x side.
        assert_eq!(at(1.0, 2.0, &mut found, &mut y, &mut n), OK);
        assert!(n.x < -0.1 && n.y > 0.1, "{:?}", (n.x, n.y, n.z));
        let length = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        assert!((length - 1.0).abs() < 1e-4, "unit length: {length}");
        // The corners are flat.
        assert_eq!(at(0.0, 0.0, &mut found, &mut y, &mut n), OK);
        assert!(found && y.abs() < 1e-5);
        assert_eq!(at(4.0, 4.0, &mut found, &mut y, &mut n), OK);
        assert!(found && y.abs() < 1e-5);
        // Off the tile: not found, and the outputs are left alone.
        (y, n) = (-7.0, v3(7.0, 7.0, 7.0));
        for (x, z) in [(-0.5, 2.0), (4.5, 2.0), (2.0, -0.5), (2.0, 4.5)] {
            assert_eq!(at(x, z, &mut found, &mut y, &mut n), OK);
            assert!(!found, "({x}, {z}) is off the tile");
        }
        assert_eq!((y, n.x), (-7.0, 7.0));
        // The normal is optional.
        assert_eq!(at(2.0, 2.0, &mut found, &mut y, null_mut()), OK);
        assert!(found);
        // Errors.
        assert_eq!(
            crabby_heightfield_at(std::ptr::null(), 0.0, 0.0, &mut found, &mut y, &mut n),
            BAD
        );
        assert_eq!(crabby_heightfield_at(ground, 0.0, 0.0, null_mut(), &mut y, &mut n), BAD);
        assert_eq!(
            crabby_heightfield_at(ground, 0.0, 0.0, &mut found, null_mut(), &mut n),
            BAD
        );
        crabby_heightfield_free(ground);
        crabby_heightfield_free(null_mut());

        // A flat 2 x 2 tile faces straight up.
        let flat = [1.0f32; 4];
        let ground = crabby_heightfield_new(flat.as_ptr(), 4, 2.0);
        assert_eq!(crabby_heightfield_at(ground, 1.0, 1.0, &mut found, &mut y, &mut n), OK);
        assert!(found && (y - 1.0).abs() < 1e-5);
        assert!(
            n.x.abs() < 1e-5 && (n.y - 1.0).abs() < 1e-5 && n.z.abs() < 1e-5,
            "{:?}",
            (n.x, n.y, n.z)
        );
        crabby_heightfield_free(ground);

        // Bad input makes no heightfield.
        assert!(crabby_heightfield_new(null_mut(), 4, 2.0).is_null());
        assert!(
            crabby_heightfield_new(heights.as_ptr(), 8, 4.0).is_null(),
            "not a square"
        );
        assert!(
            crabby_heightfield_new(heights.as_ptr(), 1, 4.0).is_null(),
            "one point is not a tile"
        );
        assert!(crabby_heightfield_new(heights.as_ptr(), 9, 0.0).is_null());
        assert!(crabby_heightfield_new(heights.as_ptr(), 9, f32::NAN).is_null());
        assert!(crabby_heightfield_new(heights.as_ptr(), 9, -1.0).is_null());
    }
}
