//! Contact modification from C: the modifier component (one-way surfaces and
//! conveyors), the custom contact hook and its pair accessors.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoBodyKind::*;
use PonkoStatus::*;

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

fn value<T>(component: u32, v: &T) -> PonkoValue {
    PonkoValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}
fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}

unsafe fn ids_of(w: *mut PonkoWorld, ids: &mut Ids) {
    unsafe {
        assert_eq!(ponko_transform3d_component(w, &mut ids.transform), PONKO_OK);
        assert_eq!(ponko_rigid_body3d_component(w, &mut ids.body), PONKO_OK);
        assert_eq!(ponko_collider3d_component(w, &mut ids.collider), PONKO_OK);
        assert_eq!(ponko_velocity3d_component(w, &mut ids.velocity), PONKO_OK);
        assert_eq!(ponko_contact_modifier3d_component(w, &mut ids.modifier), PONKO_OK);
    }
}

/// A 20 x 0.4 x 20 fixed floor at `at`, with a modifier.
unsafe fn floor(w: *mut PonkoWorld, ids: &Ids, at: PonkoVec3, modifier: PonkoContactModifier3D) -> u64 {
    unsafe {
        let (t, body, shape) = (
            ponko_transform3d_at(at),
            ponko_rigid_body3d(PONKO_BODY_FIXED as u32),
            ponko_collider3d_box(v3(20.0, 0.4, 20.0)),
        );
        let values = [
            value(ids.transform, &t),
            value(ids.body, &body),
            value(ids.collider, &shape),
            value(ids.modifier, &modifier),
        ];
        let mut e = 0;
        assert_eq!(ponko_spawn(w, values.as_ptr(), 4, &mut e), PONKO_OK);
        e
    }
}

unsafe fn ball(w: *mut PonkoWorld, ids: &Ids, at: PonkoVec3, linear: PonkoVec3) -> u64 {
    unsafe {
        let (t, body, shape) = (
            ponko_transform3d_at(at),
            ponko_rigid_body3d(PONKO_BODY_DYNAMIC as u32),
            ponko_collider3d_sphere(0.25),
        );
        let velocity = PonkoVelocity3D {
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
        assert_eq!(ponko_spawn(w, values.as_ptr(), 4, &mut e), PONKO_OK);
        e
    }
}

fn y_of(world: *mut PonkoWorld, ids: &Ids, e: u64) -> f32 {
    let mut t = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
    let n = size_of::<PonkoTransform3D>();
    assert_eq!(
        unsafe { ponko_get(world, e, ids.transform, (&raw mut t).cast(), n) },
        PONKO_OK
    );
    t.position.y
}

fn position_of(world: *mut PonkoWorld, ids: &Ids, e: u64) -> PonkoVec3 {
    let mut t = ponko_transform3d_at(v3(0.0, 0.0, 0.0));
    let n = size_of::<PonkoTransform3D>();
    assert_eq!(
        unsafe { ponko_get(world, e, ids.transform, (&raw mut t).cast(), n) },
        PONKO_OK
    );
    t.position
}

/// Ignores the pair where it touches at x > 2, drops the one point where
/// x < -5, and records what the accessors said.
unsafe extern "C" fn hook(pair: *mut PonkoContactPair3D, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        ids.calls += 1;
        let (mut a, mut b) = (0, 0);
        assert_eq!(ponko_contact_pair3d_entities(pair, &mut a, &mut b), PONKO_OK);
        ids.paired_with_floor = a == ids.floor || b == ids.floor;
        let mut n = v3(0.0, 0.0, 0.0);
        assert_eq!(ponko_contact_pair3d_normal(pair, &mut n), PONKO_OK);
        ids.normal_is_vertical = n.y.abs() > 0.99;
        let mut count = 0;
        assert_eq!(ponko_contact_pair3d_point_count(pair, &mut count), PONKO_OK);
        // Asking after a point that isn't there, or for nonsense, is refused.
        let (mut p, mut d) = (v3(0.0, 0.0, 0.0), 0.0);
        ids.bad_calls_refused = ponko_contact_pair3d_point(pair, count + 5, &mut p, &mut d) == PONKO_NOT_FOUND
            && ponko_contact_pair3d_set_friction(pair, f32::NAN) == PONKO_INVALID_ARGUMENT
            && ponko_contact_pair3d_set_restitution(pair, f32::INFINITY) == PONKO_INVALID_ARGUMENT
            && ponko_contact_pair3d_set_surface_velocity(pair, v3(f32::NAN, 0.0, 0.0)) == PONKO_INVALID_ARGUMENT
            && ponko_contact_pair3d_friction(pair, null_mut()) == PONKO_INVALID_ARGUMENT
            && ponko_contact_pair3d_remove_point(pair, count + 5) == PONKO_OK;
        if count == 0 {
            return;
        }
        assert_eq!(ponko_contact_pair3d_point(pair, 0, &mut p, &mut d), PONKO_OK);
        assert_eq!(ponko_contact_pair3d_set_friction(pair, 0.5), PONKO_OK);
        assert_eq!(ponko_contact_pair3d_set_restitution(pair, 0.25), PONKO_OK);
        assert_eq!(
            ponko_contact_pair3d_friction(pair, &mut ids.friction_after_set),
            PONKO_OK
        );
        assert_eq!(
            ponko_contact_pair3d_restitution(pair, &mut ids.restitution_after_set),
            PONKO_OK
        );
        if p.x > 2.0 {
            assert_eq!(ponko_contact_pair3d_ignore(pair), PONKO_OK);
            let mut left = 99;
            assert_eq!(ponko_contact_pair3d_point_count(pair, &mut left), PONKO_OK);
            assert_eq!(left, 0, "ignored pairs have no points");
        } else if p.x < -5.0 {
            assert_eq!(ponko_contact_pair3d_remove_point(pair, 0), PONKO_OK);
        }
    }
}

unsafe extern "C" fn setup_hook(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = ponko_ctx_world(ctx);
        ids_of(w, ids);
        ids.floor = floor(w, ids, v3(0.0, 0.0, 0.0), ponko_contact_modifier3d_hook());
        assert_eq!(ponko_ctx_set_contact_hook3d(ctx, Some(hook), user), PONKO_OK);
        for x in [3.0, -3.0, -6.0] {
            let b = ball(w, ids, v3(x, 1.0, 0.0), v3(0.0, 0.0, 0.0));
            ids.balls.push(b);
        }
    }
}

unsafe extern "C" fn setup_hook_cleared(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        setup_hook(ctx, user);
        // A NULL hook clears it: the hooked floor stays an ordinary one.
        assert_eq!(ponko_ctx_set_contact_hook3d(ctx, None, null_mut()), PONKO_OK);
    }
}

fn run(setup: unsafe extern "C" fn(*mut PonkoCtx, *mut c_void), ticks: u64, ids: &mut Ids) -> *mut PonkoWorld {
    let user = (ids as *mut Ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = ponko_game_new(c"contacts3d".as_ptr(), 1, 1);
        assert_eq!(ponko_game_physics3d(g, v3(0.0, -9.81, 0.0)), PONKO_OK);
        assert_eq!(
            ponko_game_run_headless(g, ticks, Some(setup), user, &mut world),
            PONKO_OK
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
    unsafe { ponko_world_free(world) };
}

#[test]
fn a_null_hook_clears_it() {
    let mut ids = Ids::default();
    let world = run(setup_hook_cleared, 100, &mut ids);
    assert_eq!(ids.calls, 0);
    for &b in &ids.balls {
        assert!(y_of(world, &ids, b) > 0.3, "the plain floor holds every ball");
    }
    unsafe { ponko_world_free(world) };
}

unsafe extern "C" fn setup_one_way_and_conveyor(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = ponko_ctx_world(ctx);
        ids_of(w, ids);
        // A one-way platform; a ball thrown up from below comes up through it.
        floor(
            w,
            ids,
            v3(0.0, 0.0, 0.0),
            ponko_contact_modifier3d_one_way(v3(0.0, 1.0, 0.0)),
        );
        let up = ball(w, ids, v3(0.0, -1.0, 0.0), v3(0.0, 8.0, 0.0));
        // An ordinary one blocks the same throw.
        floor(w, ids, v3(30.0, 0.0, 0.0), ponko_contact_modifier3d_default());
        let blocked = ball(w, ids, v3(30.0, -1.0, 0.0), v3(0.0, 8.0, 0.0));
        // A belt carrying things along +X at 2 m/s.
        floor(
            w,
            ids,
            v3(60.0, 0.0, 0.0),
            ponko_contact_modifier3d_conveyor(v3(2.0, 0.0, 0.0)),
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
    unsafe { ponko_world_free(world) };
}

#[test]
fn modifiers_round_trip_through_the_world_and_bad_ones_are_refused() {
    let mut ids = Ids::default();
    unsafe {
        let w = ponko_world_new();
        ids_of(w, &mut ids);
        let mut m = ponko_contact_modifier3d_one_way(v3(0.0, 1.0, 0.0));
        m.one_way_angle = 0.5;
        m.surface_velocity = v3(1.0, 0.0, 2.0);
        m.hook = true;
        let values = [value(ids.modifier, &m)];
        let mut e = 0;
        assert_eq!(ponko_spawn(w, values.as_ptr(), 1, &mut e), PONKO_OK);
        let mut back = ponko_contact_modifier3d_default();
        let n = size_of::<PonkoContactModifier3D>();
        assert_eq!(ponko_get(w, e, ids.modifier, (&raw mut back).cast(), n), PONKO_OK);
        assert_eq!(back, m);

        // Without a one-way side the direction is ignored.
        let none = ponko_contact_modifier3d_default();
        assert!(!none.has_one_way);
        assert_eq!(ponko_set(w, e, ids.modifier, (&raw const none).cast(), n), PONKO_OK);
        assert_eq!(ponko_get(w, e, ids.modifier, (&raw mut back).cast(), n), PONKO_OK);
        assert_eq!(back, none);

        for bad in [
            PonkoContactModifier3D {
                one_way_angle: -0.1,
                ..none
            },
            PonkoContactModifier3D {
                one_way_angle: f32::NAN,
                ..none
            },
            PonkoContactModifier3D {
                surface_velocity: v3(f32::INFINITY, 0.0, 0.0),
                ..none
            },
            PonkoContactModifier3D {
                has_one_way: true,
                one_way: v3(f32::NAN, 0.0, 0.0),
                ..none
            },
        ] {
            assert_eq!(
                ponko_set(w, e, ids.modifier, (&raw const bad).cast(), n),
                PONKO_INVALID_ARGUMENT
            );
        }
        assert_eq!(ponko_contact_pair3d_ignore(null_mut()), PONKO_INVALID_ARGUMENT);
        ponko_world_free(w);
    }
}
