//! Declared access for C systems: building declarations, conflicts, the
//! report, and the debug check of what a C system used.
use ponko_ffi::*;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

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

fn access(steps: impl FnOnce(*mut CrabbyAccess)) -> *mut CrabbyAccess {
    let a = crabby_access_new();
    assert!(!a.is_null());
    steps(a);
    a
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(
            crabby_component_register(w, c"health".as_ptr(), 4, 4, &mut s.health),
            CRABBY_OK
        );
        assert_eq!(
            crabby_component_register(w, c"hit".as_ptr(), 4, 4, &mut s.hit),
            CRABBY_OK
        );
        let h = Health { hp: 10 };
        let value = CrabbyValue {
            component: s.health,
            data: (&raw const h).cast(),
            size: 4,
        };
        assert_eq!(crabby_spawn(w, &value, 1, &mut s.entity), CRABBY_OK);
        let score = 0u32;
        assert_eq!(
            crabby_resource_set(w, c"score".as_ptr(), (&raw const score).cast(), 4, 4),
            CRABBY_OK
        );
    }
}

/// Hurts the entity, bumps the score and sends a hit: within its declaration.
unsafe extern "C" fn hurt(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let w = crabby_ctx_world(ctx);
        let mut h = Health::default();
        assert_eq!(crabby_get(w, s.entity, s.health, (&raw mut h).cast(), 4), CRABBY_OK);
        h.hp -= 1;
        assert_eq!(crabby_set(w, s.entity, s.health, (&raw const h).cast(), 4), CRABBY_OK);
        let mut score = 0u32;
        assert_eq!(
            crabby_resource_get(w, c"score".as_ptr(), (&raw mut score).cast(), 4),
            CRABBY_OK
        );
        score += 1;
        assert_eq!(
            crabby_resource_set(w, c"score".as_ptr(), (&raw const score).cast(), 4, 4),
            CRABBY_OK
        );
        let damage = 1u32;
        assert_eq!(
            crabby_ctx_send_event(ctx, s.hit, (&raw const damage).cast(), 4),
            CRABBY_OK
        );
    }
}

unsafe extern "C" fn count_hits(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let s = &mut *user.cast::<State>();
        let (mut data, mut count) = (null(), 0);
        assert_eq!(crabby_ctx_read_events(ctx, s.hit, 4, &mut data, &mut count), CRABBY_OK);
        s.seen += count;
    }
}

fn hurt_access() -> *mut CrabbyAccess {
    access(|a| unsafe {
        assert_eq!(crabby_access_write_component(a, c"health".as_ptr()), CRABBY_OK);
        assert_eq!(crabby_access_write_resource(a, c"score".as_ptr()), CRABBY_OK);
        assert_eq!(crabby_access_send_events(a, c"hit".as_ptr()), CRABBY_OK);
    })
}

fn reader_access() -> *mut CrabbyAccess {
    access(|a| unsafe {
        assert_eq!(crabby_access_read_events(a, c"hit".as_ptr()), CRABBY_OK);
        assert_eq!(
            crabby_access_service(a, CrabbyService::CRABBY_SERVICE_TIME as u32, false),
            CRABBY_OK
        );
    })
}

fn last_error() -> String {
    unsafe { CStr::from_ptr(crabby_last_error()) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn declared_c_systems_run_within_their_declarations() {
    let mut state = State::default();
    let (hurt_a, read_a) = (hurt_access(), reader_access());
    unsafe {
        let g = crabby_game_new(c"access".as_ptr(), 1, 1);
        let p = CrabbyPhase::CRABBY_PHASE_AFTER_PHYSICS as u32;
        let u = user(&mut state);
        assert_eq!(
            crabby_game_add_system_with_access(g, p, c"hurt".as_ptr(), 1, hurt_a, Some(hurt), u),
            CRABBY_OK
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, p, c"count".as_ptr(), 1, read_a, Some(count_hits), u),
            CRABBY_OK
        );
        crabby_access_free(hurt_a);
        crabby_access_free(read_a);
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, 3, Some(setup), u, &mut world), CRABBY_OK);
        let mut h = Health::default();
        assert_eq!(
            crabby_get(world, state.entity, state.health, (&raw mut h).cast(), 4),
            CRABBY_OK
        );
        assert_eq!(h.hp, 7);
        crabby_world_free(world);
    }
    assert_eq!(state.seen, 3);
}

#[test]
fn the_report_lists_declared_c_systems() {
    let mut state = State::default();
    let (a, b) = (hurt_access(), reader_access());
    unsafe {
        let g = crabby_game_new(c"access".as_ptr(), 1, 1);
        let p = CrabbyPhase::CRABBY_PHASE_AFTER_PHYSICS as u32;
        let u = user(&mut state);
        assert_eq!(
            crabby_game_add_system_with_access(g, p, c"hurt".as_ptr(), 1, a, Some(hurt), u),
            CRABBY_OK
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, p, c"count".as_ptr(), 1, b, Some(count_hits), u),
            CRABBY_OK
        );
        let mut len = 0;
        assert_eq!(crabby_game_access_report(g, null_mut(), 0, &mut len), CRABBY_OK);
        let mut buf = vec![0 as c_char; len + 1];
        assert_eq!(
            crabby_game_access_report(g, buf.as_mut_ptr(), buf.len(), &mut len),
            CRABBY_OK
        );
        let text = CStr::from_ptr(buf.as_ptr()).to_str().unwrap();
        assert_eq!(text, "hurt + count: conflict on events `hit`\n");
        assert_eq!(
            crabby_game_access_report(g, null_mut(), 0, null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_game_free(g);
        crabby_access_free(a);
        crabby_access_free(b);
    }
}

#[test]
fn conflicts_follow_the_rules() {
    let (send, read) = (hurt_access(), reader_access());
    let mut n = 0;
    unsafe {
        assert_eq!(crabby_access_conflicts(send, read, true, &mut n), CRABBY_OK);
        assert_eq!(n, 1, "sender before reader");
        assert_eq!(crabby_access_conflicts(read, send, true, &mut n), CRABBY_OK);
        assert_eq!(n, 0, "reader before sender");
        assert_eq!(crabby_access_conflicts(read, send, false, &mut n), CRABBY_OK);
        assert_eq!(n, 1, "either order");
        assert_eq!(crabby_access_conflicts(send, send, true, &mut n), CRABBY_OK);
        assert_eq!(n, 2, "same component and resource written");
        let cmd = access(|a| assert_eq!(crabby_access_commands(a), CRABBY_OK));
        assert_eq!(crabby_access_conflicts(cmd, send, true, &mut n), CRABBY_OK);
        assert_eq!(n, 1, "commands before a world user");
        assert_eq!(
            crabby_access_conflicts(cmd, null(), true, &mut n),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_access_conflicts(cmd, cmd, true, null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        let r = access(|a| assert_eq!(crabby_access_read_component(a, c"health".as_ptr()), CRABBY_OK));
        let rr = access(|a| assert_eq!(crabby_access_read_resource(a, c"score".as_ptr()), CRABBY_OK));
        assert_eq!(crabby_access_conflicts(r, send, false, &mut n), CRABBY_OK);
        assert_eq!(n, 1);
        assert_eq!(crabby_access_conflicts(rr, send, false, &mut n), CRABBY_OK);
        assert_eq!(n, 1);
        assert_eq!(crabby_access_conflicts(r, rr, false, &mut n), CRABBY_OK);
        assert_eq!(n, 0);
        for a in [send, read, cmd, r, rr] {
            crabby_access_free(a);
        }
    }
}

#[test]
fn bad_arguments_are_rejected() {
    let a = crabby_access_new();
    unsafe {
        assert_eq!(crabby_access_service(a, 13, true), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_access_read_component(a, null()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_access_commands(null_mut()), CRABBY_INVALID_ARGUMENT);
        let g = crabby_game_new(c"access".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_add_system_with_access(g, 3, null(), 1, null(), Some(count_hits), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, 3, null(), 0, a, Some(count_hits), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, 5, null(), 2, a, Some(count_hits), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, 99, null(), 1, a, Some(count_hits), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_game_add_system_with_access(g, 3, null(), 1, a, None, null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_game_free(g);
        crabby_access_free(a);
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_c_system_beyond_its_declaration_stops_the_run_in_debug_builds() {
    let mut state = State::default();
    // Reads hit events but declares only sending them... and writes health undeclared.
    let a = access(|a| unsafe {
        assert_eq!(crabby_access_send_events(a, c"hit".as_ptr()), CRABBY_OK);
    });
    unsafe {
        let g = crabby_game_new(c"access".as_ptr(), 1, 1);
        let u = user(&mut state);
        assert_eq!(
            crabby_game_add_system_with_access(g, 3, c"sneaky".as_ptr(), 1, a, Some(hurt), u),
            CRABBY_OK
        );
        crabby_access_free(a);
        let mut world = null_mut();
        let status = crabby_game_run_headless(g, 1, Some(setup), u, &mut world);
        assert_ne!(status, CRABBY_OK);
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
