//! Contact modification from C: the modifier component (one-way surfaces and
//! conveyors), the custom contact hook and its pair accessors.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyBodyKind::*;
use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    body: u32,
    collider: u32,
    velocity: u32,
    modifier: u32,
    floor: u64,
    // The entities the tests look at, by role.
    balls: Vec<u64>,
    // What the hook saw.
    calls: usize,
    paired_with_floor: bool,
    normal_is_vertical: bool,
    friction_after_set: f32,
    restitution_after_set: f32,
    bad_calls_refused: bool,
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

unsafe fn ids_of(w: *mut CrabbyWorld, ids: &mut Ids) {
    unsafe {
        assert_eq!(crabby_transform3d_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut ids.body), CRABBY_OK);
        assert_eq!(crabby_collider3d_component(w, &mut ids.collider), CRABBY_OK);
        assert_eq!(crabby_velocity3d_component(w, &mut ids.velocity), CRABBY_OK);
        assert_eq!(crabby_contact_modifier3d_component(w, &mut ids.modifier), CRABBY_OK);
    }
}

/// A 20 x 0.4 x 20 fixed floor at `at`, with a modifier.
unsafe fn floor(w: *mut CrabbyWorld, ids: &Ids, at: CrabbyVec3, modifier: CrabbyContactModifier3D) -> u64 {
    unsafe {
        let (t, body, shape) = (
            crabby_transform3d_at(at),
            crabby_rigid_body3d(CRABBY_BODY_FIXED as u32),
            crabby_collider3d_box(v3(20.0, 0.4, 20.0)),
        );
        let values = [
            value(ids.transform, &t),
            value(ids.body, &body),
            value(ids.collider, &shape),
            value(ids.modifier, &modifier),
        ];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 4, &mut e), CRABBY_OK);
        e
    }
}

unsafe fn ball(w: *mut CrabbyWorld, ids: &Ids, at: CrabbyVec3, linear: CrabbyVec3) -> u64 {
    unsafe {
        let (t, body, shape) = (
            crabby_transform3d_at(at),
            crabby_rigid_body3d(CRABBY_BODY_DYNAMIC as u32),
            crabby_collider3d_sphere(0.25),
        );
        let velocity = CrabbyVelocity3D {
            linear,
            angular: v3(0.0, 0.0, 0.0),
        };
        let values = [
            value(ids.transform, &t),
            value(ids.body, &body),
            value(ids.collider, &shape),
            value(ids.velocity, &velocity),
        ];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 4, &mut e), CRABBY_OK);
        e
    }
}

fn y_of(world: *mut CrabbyWorld, ids: &Ids, e: u64) -> f32 {
    let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
    let n = size_of::<CrabbyTransform3D>();
    assert_eq!(
        unsafe { crabby_get(world, e, ids.transform, (&raw mut t).cast(), n) },
        CRABBY_OK
    );
    t.position.y
}

fn position_of(world: *mut CrabbyWorld, ids: &Ids, e: u64) -> CrabbyVec3 {
    let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
    let n = size_of::<CrabbyTransform3D>();
    assert_eq!(
        unsafe { crabby_get(world, e, ids.transform, (&raw mut t).cast(), n) },
        CRABBY_OK
    );
    t.position
}

/// Ignores the pair where it touches at x > 2, drops the one point where
/// x < -5, and records what the accessors said.
unsafe extern "C" fn hook(pair: *mut CrabbyContactPair3D, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        ids.calls += 1;
        let (mut a, mut b) = (0, 0);
        assert_eq!(crabby_contact_pair3d_entities(pair, &mut a, &mut b), CRABBY_OK);
        ids.paired_with_floor = a == ids.floor || b == ids.floor;
        let mut n = v3(0.0, 0.0, 0.0);
        assert_eq!(crabby_contact_pair3d_normal(pair, &mut n), CRABBY_OK);
        ids.normal_is_vertical = n.y.abs() > 0.99;
        let mut count = 0;
        assert_eq!(crabby_contact_pair3d_point_count(pair, &mut count), CRABBY_OK);
        // Asking after a point that isn't there, or for nonsense, is refused.
        let (mut p, mut d) = (v3(0.0, 0.0, 0.0), 0.0);
        ids.bad_calls_refused = crabby_contact_pair3d_point(pair, count + 5, &mut p, &mut d) == CRABBY_NOT_FOUND
            && crabby_contact_pair3d_set_friction(pair, f32::NAN) == CRABBY_INVALID_ARGUMENT
            && crabby_contact_pair3d_set_restitution(pair, f32::INFINITY) == CRABBY_INVALID_ARGUMENT
            && crabby_contact_pair3d_set_surface_velocity(pair, v3(f32::NAN, 0.0, 0.0)) == CRABBY_INVALID_ARGUMENT
            && crabby_contact_pair3d_friction(pair, null_mut()) == CRABBY_INVALID_ARGUMENT
            && crabby_contact_pair3d_remove_point(pair, count + 5) == CRABBY_OK;
        if count == 0 {
            return;
        }
        assert_eq!(crabby_contact_pair3d_point(pair, 0, &mut p, &mut d), CRABBY_OK);
        assert_eq!(crabby_contact_pair3d_set_friction(pair, 0.5), CRABBY_OK);
        assert_eq!(crabby_contact_pair3d_set_restitution(pair, 0.25), CRABBY_OK);
        assert_eq!(
            crabby_contact_pair3d_friction(pair, &mut ids.friction_after_set),
            CRABBY_OK
        );
        assert_eq!(
            crabby_contact_pair3d_restitution(pair, &mut ids.restitution_after_set),
            CRABBY_OK
        );
        if p.x > 2.0 {
            assert_eq!(crabby_contact_pair3d_ignore(pair), CRABBY_OK);
            let mut left = 99;
            assert_eq!(crabby_contact_pair3d_point_count(pair, &mut left), CRABBY_OK);
            assert_eq!(left, 0, "ignored pairs have no points");
        } else if p.x < -5.0 {
            assert_eq!(crabby_contact_pair3d_remove_point(pair, 0), CRABBY_OK);
        }
    }
}

unsafe extern "C" fn setup_hook(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        ids_of(w, ids);
        ids.floor = floor(w, ids, v3(0.0, 0.0, 0.0), crabby_contact_modifier3d_hook());
        assert_eq!(crabby_ctx_set_contact_hook3d(ctx, Some(hook), user), CRABBY_OK);
        for x in [3.0, -3.0, -6.0] {
            let b = ball(w, ids, v3(x, 1.0, 0.0), v3(0.0, 0.0, 0.0));
            ids.balls.push(b);
        }
    }
}

unsafe extern "C" fn setup_hook_cleared(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        setup_hook(ctx, user);
        // A NULL hook clears it: the hooked floor stays an ordinary one.
        assert_eq!(crabby_ctx_set_contact_hook3d(ctx, None, null_mut()), CRABBY_OK);
    }
}

fn run(setup: unsafe extern "C" fn(*mut CrabbyCtx, *mut c_void), ticks: u64, ids: &mut Ids) -> *mut CrabbyWorld {
    let user = (ids as *mut Ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"contacts3d".as_ptr(), 1, 1);
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        assert_eq!(
            crabby_game_run_headless(g, ticks, Some(setup), user, &mut world),
            CRABBY_OK
        );
    }
    world
}

#[test]
fn the_c_hook_sees_pairs_and_can_drop_them_or_their_points() {
    let mut ids = Ids::default();
    let world = run(setup_hook, 100, &mut ids);
    assert!(ids.calls > 0);
    assert!(ids.paired_with_floor && ids.normal_is_vertical);
    assert_eq!(ids.friction_after_set, 0.5);
    assert_eq!(ids.restitution_after_set, 0.25);
    assert!(ids.bad_calls_refused, "bad arguments are refused, not obeyed");
    let [ignored, standing, point_removed] = ids.balls[..] else {
        panic!("three balls")
    };
    assert!(y_of(world, &ids, ignored) < -1.0, "ignored pair: it fell through");
    assert!(y_of(world, &ids, standing) > 0.3, "left alone: it stands");
    assert!(
        y_of(world, &ids, point_removed) < -1.0,
        "its only point removed: it fell through"
    );
    unsafe { crabby_world_free(world) };
}

#[test]
fn a_null_hook_clears_it() {
    let mut ids = Ids::default();
    let world = run(setup_hook_cleared, 100, &mut ids);
    assert_eq!(ids.calls, 0);
    for &b in &ids.balls {
        assert!(y_of(world, &ids, b) > 0.3, "the plain floor holds every ball");
    }
    unsafe { crabby_world_free(world) };
}

unsafe extern "C" fn setup_one_way_and_conveyor(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        ids_of(w, ids);
        // A one-way platform; a ball thrown up from below comes up through it.
        floor(
            w,
            ids,
            v3(0.0, 0.0, 0.0),
            crabby_contact_modifier3d_one_way(v3(0.0, 1.0, 0.0)),
        );
        let up = ball(w, ids, v3(0.0, -1.0, 0.0), v3(0.0, 8.0, 0.0));
        // An ordinary one blocks the same throw.
        floor(w, ids, v3(30.0, 0.0, 0.0), crabby_contact_modifier3d_default());
        let blocked = ball(w, ids, v3(30.0, -1.0, 0.0), v3(0.0, 8.0, 0.0));
        // A belt carrying things along +X at 2 m/s.
        floor(
            w,
            ids,
            v3(60.0, 0.0, 0.0),
            crabby_contact_modifier3d_conveyor(v3(2.0, 0.0, 0.0)),
        );
        let carried = ball(w, ids, v3(60.0, 0.5, 0.0), v3(0.0, 0.0, 0.0));
        ids.balls = vec![up, blocked, carried];
    }
}

#[test]
fn c_one_way_platforms_pass_things_up_and_conveyors_carry_them() {
    let mut ids = Ids::default();
    let world = run(setup_one_way_and_conveyor, 150, &mut ids);
    let [up, blocked, carried] = ids.balls[..] else {
        panic!("three balls")
    };
    let rest = 0.2 + 0.25;
    let y = y_of(world, &ids, up);
    assert!((y - rest).abs() < 0.05, "came up through and rests on top: {y}");
    assert!(y_of(world, &ids, blocked) < -0.3, "the plain platform blocked it");
    let p = position_of(world, &ids, carried);
    assert!(p.x > 60.0 + 1.0, "carried along +X: {}", p.x);
    unsafe { crabby_world_free(world) };
}

#[test]
fn modifiers_round_trip_through_the_world_and_bad_ones_are_refused() {
    let mut ids = Ids::default();
    unsafe {
        let w = crabby_world_new();
        ids_of(w, &mut ids);
        let mut m = crabby_contact_modifier3d_one_way(v3(0.0, 1.0, 0.0));
        m.one_way_angle = 0.5;
        m.surface_velocity = v3(1.0, 0.0, 2.0);
        m.hook = true;
        let values = [value(ids.modifier, &m)];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 1, &mut e), CRABBY_OK);
        let mut back = crabby_contact_modifier3d_default();
        let n = size_of::<CrabbyContactModifier3D>();
        assert_eq!(crabby_get(w, e, ids.modifier, (&raw mut back).cast(), n), CRABBY_OK);
        assert_eq!(back, m);

        // Without a one-way side the direction is ignored.
        let none = crabby_contact_modifier3d_default();
        assert!(!none.has_one_way);
        assert_eq!(crabby_set(w, e, ids.modifier, (&raw const none).cast(), n), CRABBY_OK);
        assert_eq!(crabby_get(w, e, ids.modifier, (&raw mut back).cast(), n), CRABBY_OK);
        assert_eq!(back, none);

        for bad in [
            CrabbyContactModifier3D {
                one_way_angle: -0.1,
                ..none
            },
            CrabbyContactModifier3D {
                one_way_angle: f32::NAN,
                ..none
            },
            CrabbyContactModifier3D {
                surface_velocity: v3(f32::INFINITY, 0.0, 0.0),
                ..none
            },
            CrabbyContactModifier3D {
                has_one_way: true,
                one_way: v3(f32::NAN, 0.0, 0.0),
                ..none
            },
        ] {
            assert_eq!(
                crabby_set(w, e, ids.modifier, (&raw const bad).cast(), n),
                CRABBY_INVALID_ARGUMENT
            );
        }
        assert_eq!(crabby_contact_pair3d_ignore(null_mut()), CRABBY_INVALID_ARGUMENT);
        crabby_world_free(w);
    }
}
