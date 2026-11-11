//! Declared access for C systems: building declarations, conflicts, the
//! report, and the debug check of what a C system used.
use ponko_ffi::*;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::{null, null_mut};

use PonkoStatus::*;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Health {
    hp: u32,
}

/// Ids looked up in setup, shared with the systems.
#[derive(Default)]
struct State {
    health: u32,
    hit: u32,
    entity: u64,
    seen: usize,
}

fn user<T>(t: &mut T) -> *mut c_void {
    (t as *mut T).cast()
}

fn access(steps: impl FnOnce(*mut PonkoAccess)) -> *mut PonkoAccess {
    let a = ponko_access_new();
    assert!(!a.is_null());
    steps(a);
    a
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let w = ponko_ctx_world(ctx);
        assert_eq!(
            ponko_component_register(w, c"health".as_ptr(), 4, 4, &mut s.health),
            PONKO_OK
        );
        assert_eq!(ponko_component_register(w, c"hit".as_ptr(), 4, 4, &mut s.hit), PONKO_OK);
        let h = Health { hp: 10 };
        let value = PonkoValue {
            component: s.health,
            data: (&raw const h).cast(),
            size: 4,
        };
        assert_eq!(ponko_spawn(w, &value, 1, &mut s.entity), PONKO_OK);
        let score = 0u32;
        assert_eq!(
            ponko_resource_set(w, c"score".as_ptr(), (&raw const score).cast(), 4, 4),
            PONKO_OK
        );
    }
}

/// Hurts the entity, bumps the score and sends a hit: within its declaration.
unsafe extern "C" fn hurt(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let w = ponko_ctx_world(ctx);
        let mut h = Health::default();
        assert_eq!(ponko_get(w, s.entity, s.health, (&raw mut h).cast(), 4), PONKO_OK);
        h.hp -= 1;
        assert_eq!(ponko_set(w, s.entity, s.health, (&raw const h).cast(), 4), PONKO_OK);
        let mut score = 0u32;
        assert_eq!(
            ponko_resource_get(w, c"score".as_ptr(), (&raw mut score).cast(), 4),
            PONKO_OK
        );
        score += 1;
        assert_eq!(
            ponko_resource_set(w, c"score".as_ptr(), (&raw const score).cast(), 4, 4),
            PONKO_OK
        );
        let damage = 1u32;
        assert_eq!(
            ponko_ctx_send_event(ctx, s.hit, (&raw const damage).cast(), 4),
            PONKO_OK
        );
    }
}

unsafe extern "C" fn count_hits(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let (mut data, mut count) = (null(), 0);
        assert_eq!(ponko_ctx_read_events(ctx, s.hit, 4, &mut data, &mut count), PONKO_OK);
        s.seen += count;
    }
}

fn hurt_access() -> *mut PonkoAccess {
    access(|a| unsafe {
        assert_eq!(ponko_access_write_component(a, c"health".as_ptr()), PONKO_OK);
        assert_eq!(ponko_access_write_resource(a, c"score".as_ptr()), PONKO_OK);
        assert_eq!(ponko_access_send_events(a, c"hit".as_ptr()), PONKO_OK);
    })
}

fn reader_access() -> *mut PonkoAccess {
    access(|a| unsafe {
        assert_eq!(ponko_access_read_events(a, c"hit".as_ptr()), PONKO_OK);
        assert_eq!(
            ponko_access_service(a, PonkoService::PONKO_SERVICE_TIME as u32, false),
            PONKO_OK
        );
    })
}

fn last_error() -> String {
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn declared_c_systems_run_within_their_declarations() {
    let mut state = State::default();
    let (hurt_a, read_a) = (hurt_access(), reader_access());
    unsafe {
        let g = ponko_game_new(c"access".as_ptr(), 1, 1);
        let p = PonkoPhase::PONKO_PHASE_AFTER_PHYSICS as u32;
        let u = user(&mut state);
        assert_eq!(
            ponko_game_add_system_with_access(g, p, c"hurt".as_ptr(), 1, hurt_a, Some(hurt), u),
            PONKO_OK
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, p, c"count".as_ptr(), 1, read_a, Some(count_hits), u),
            PONKO_OK
        );
        ponko_access_free(hurt_a);
        ponko_access_free(read_a);
        let mut world = null_mut();
        assert_eq!(ponko_game_run_headless(g, 3, Some(setup), u, &mut world), PONKO_OK);
        let mut h = Health::default();
        assert_eq!(
            ponko_get(world, state.entity, state.health, (&raw mut h).cast(), 4),
            PONKO_OK
        );
        assert_eq!(h.hp, 7);
        ponko_world_free(world);
    }
    assert_eq!(state.seen, 3);
}

/// The game's access report as text.
unsafe fn report(g: *mut PonkoGame) -> String {
    // SAFETY: the caller's live game; the buffer is sized by the first call.
    unsafe {
        let mut len = 0;
        assert_eq!(ponko_game_access_report(g, null_mut(), 0, &mut len), PONKO_OK);
        let mut buf = vec![0 as c_char; len + 1];
        assert_eq!(
            ponko_game_access_report(g, buf.as_mut_ptr(), buf.len(), &mut len),
            PONKO_OK
        );
        CStr::from_ptr(buf.as_ptr()).to_str().unwrap().to_owned()
    }
}

#[cfg(all(feature = "physics", feature = "physics3d"))]
#[test]
fn the_physics_and_navigation_steps_a_c_game_adds_are_declared() {
    let mut state = State::default();
    let b = reader_access();
    unsafe {
        let g = ponko_game_new(c"steps".as_ptr(), 1, 1);
        assert_eq!(ponko_game_physics(g, PonkoVec2 { x: 0.0, y: 0.0 }), PONKO_OK);
        let zero = PonkoVec3 { x: 0.0, y: 0.0, z: 0.0 };
        assert_eq!(ponko_game_physics3d(g, zero), PONKO_OK);
        assert_eq!(ponko_game_navmesh3d(g), PONKO_OK);
        let p = PonkoPhase::PONKO_PHASE_AFTER_PHYSICS as u32;
        assert_eq!(
            ponko_game_add_system_with_access(g, p, c"count".as_ptr(), 1, b, Some(count_hits), user(&mut state)),
            PONKO_OK
        );
        ponko_access_free(b);
        let text = report(g);
        // The two steps share the Physics phase. Declared, they are paired
        // (undeclared systems are left out of the report); in their order
        // nothing of one is visible to the other.
        let steps = text
            .lines()
            .find(|l| l.contains("step_physics +") && l.contains("step_physics3d:"))
            .unwrap_or_else(|| panic!("the steps are paired: {text}"));
        assert!(steps.contains("no conflict"), "{steps}");
        // The navigation step is declared too: it and a C system that
        // touches none of its inputs don't conflict.
        let nav = text
            .lines()
            .find(|l| l.contains("step_navmesh3d + count:"))
            .unwrap_or_else(|| panic!("navigation is paired with the C system: {text}"));
        assert!(nav.contains("no conflict"), "{nav}");
        ponko_game_free(g);
    }
}

#[test]
fn the_report_lists_declared_c_systems() {
    let mut state = State::default();
    let (a, b) = (hurt_access(), reader_access());
    unsafe {
        let g = ponko_game_new(c"access".as_ptr(), 1, 1);
        let p = PonkoPhase::PONKO_PHASE_AFTER_PHYSICS as u32;
        let u = user(&mut state);
        assert_eq!(
            ponko_game_add_system_with_access(g, p, c"hurt".as_ptr(), 1, a, Some(hurt), u),
            PONKO_OK
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, p, c"count".as_ptr(), 1, b, Some(count_hits), u),
            PONKO_OK
        );
        let mut len = 0;
        assert_eq!(ponko_game_access_report(g, null_mut(), 0, &mut len), PONKO_OK);
        let mut buf = vec![0 as c_char; len + 1];
        assert_eq!(
            ponko_game_access_report(g, buf.as_mut_ptr(), buf.len(), &mut len),
            PONKO_OK
        );
        let text = CStr::from_ptr(buf.as_ptr()).to_str().unwrap();
        assert_eq!(text, "hurt + count: conflict on events `hit`\n");
        assert_eq!(
            ponko_game_access_report(g, null_mut(), 0, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        // The scheduler can be turned on, and the plan says why C systems
        // still run one at a time.
        assert_eq!(ponko_game_parallel_systems(g, true), PONKO_OK);
        assert_eq!(ponko_game_schedule_plan(g, null_mut(), 0, &mut len), PONKO_OK);
        let mut buf = vec![0 as c_char; len + 1];
        assert_eq!(
            ponko_game_schedule_plan(g, buf.as_mut_ptr(), buf.len(), &mut len),
            PONKO_OK
        );
        let plan = CStr::from_ptr(buf.as_ptr()).to_str().unwrap();
        assert!(plan.contains("  hurt (alone: takes `&mut Ctx`)\n"), "{plan}");
        assert!(plan.contains("  count (alone: takes `&mut Ctx`)\n"), "{plan}");
        assert_eq!(
            ponko_game_schedule_plan(g, null_mut(), 0, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_game_parallel_systems(null_mut(), true), PONKO_INVALID_ARGUMENT);
        ponko_game_free(g);
        ponko_access_free(a);
        ponko_access_free(b);
    }
}

#[test]
fn conflicts_follow_the_rules() {
    let (send, read) = (hurt_access(), reader_access());
    let mut n = 0;
    unsafe {
        assert_eq!(ponko_access_conflicts(send, read, true, &mut n), PONKO_OK);
        assert_eq!(n, 1, "sender before reader");
        assert_eq!(ponko_access_conflicts(read, send, true, &mut n), PONKO_OK);
        assert_eq!(n, 0, "reader before sender");
        assert_eq!(ponko_access_conflicts(read, send, false, &mut n), PONKO_OK);
        assert_eq!(n, 1, "either order");
        assert_eq!(ponko_access_conflicts(send, send, true, &mut n), PONKO_OK);
        assert_eq!(n, 2, "same component and resource written");
        let cmd = access(|a| assert_eq!(ponko_access_commands(a), PONKO_OK));
        assert_eq!(ponko_access_conflicts(cmd, send, true, &mut n), PONKO_OK);
        assert_eq!(n, 1, "commands before a world user");
        assert_eq!(
            ponko_access_conflicts(cmd, null(), true, &mut n),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_access_conflicts(cmd, cmd, true, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        let r = access(|a| assert_eq!(ponko_access_read_component(a, c"health".as_ptr()), PONKO_OK));
        let rr = access(|a| assert_eq!(ponko_access_read_resource(a, c"score".as_ptr()), PONKO_OK));
        assert_eq!(ponko_access_conflicts(r, send, false, &mut n), PONKO_OK);
        assert_eq!(n, 1);
        assert_eq!(ponko_access_conflicts(rr, send, false, &mut n), PONKO_OK);
        assert_eq!(n, 1);
        assert_eq!(ponko_access_conflicts(r, rr, false, &mut n), PONKO_OK);
        assert_eq!(n, 0);
        for a in [send, read, cmd, r, rr] {
            ponko_access_free(a);
        }
    }
}

#[test]
fn bad_arguments_are_rejected() {
    let a = ponko_access_new();
    unsafe {
        assert_eq!(ponko_access_service(a, 13, true), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_access_read_component(a, null()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_access_commands(null_mut()), PONKO_INVALID_ARGUMENT);
        let g = ponko_game_new(c"access".as_ptr(), 1, 1);
        assert_eq!(
            ponko_game_add_system_with_access(g, 3, null(), 1, null(), Some(count_hits), null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, 3, null(), 0, a, Some(count_hits), null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, 5, null(), 2, a, Some(count_hits), null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, 99, null(), 1, a, Some(count_hits), null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_game_add_system_with_access(g, 3, null(), 1, a, None, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        ponko_game_free(g);
        ponko_access_free(a);
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_c_system_beyond_its_declaration_stops_the_run_in_debug_builds() {
    let mut state = State::default();
    // Reads hit events but declares only sending them... and writes health undeclared.
    let a = access(|a| unsafe {
        assert_eq!(ponko_access_send_events(a, c"hit".as_ptr()), PONKO_OK);
    });
    unsafe {
        let g = ponko_game_new(c"access".as_ptr(), 1, 1);
        let u = user(&mut state);
        assert_eq!(
            ponko_game_add_system_with_access(g, 3, c"sneaky".as_ptr(), 1, a, Some(hurt), u),
            PONKO_OK
        );
        ponko_access_free(a);
        let mut world = null_mut();
        let status = ponko_game_run_headless(g, 1, Some(setup), u, &mut world);
        assert_ne!(status, PONKO_OK);
        assert!(world.is_null());
    }
    let error = last_error();
    assert!(
        error.contains("system `sneaky` went beyond its declared access"),
        "{error}"
    );
    assert!(error.contains("component `health`"), "{error}");
    assert!(error.contains("resource `score`"), "{error}");
}
