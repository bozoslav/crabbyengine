//! Headless callbacks run on the engine's fixed simulation clock.
use crabby_ffi::*;
use std::ffi::c_void;
use std::ptr;

#[derive(Default)]
struct Calls {
    ticks: Vec<(u64, f32)>,
    setup: bool,
    stop_at: Option<u64>,
}

unsafe extern "C" fn setup(world: *mut CrabbyWorld, user: *mut c_void) -> bool {
    // SAFETY: tests pass a live Calls and the engine lends a live world.
    unsafe {
        let calls = &mut *user.cast::<Calls>();
        calls.setup = true;
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_BUSY);
        let mut entity = 0;
        assert_eq!(
            crabby_spawn(world, ptr::null(), 0, &mut entity),
            CrabbyStatus::CRABBY_OK
        );
    }
    true
}

unsafe extern "C" fn step(world: *mut CrabbyWorld, tick: u64, dt: f32, user: *mut c_void) -> bool {
    // SAFETY: tests pass a live Calls and the engine lends a live world.
    unsafe {
        let calls = &mut *user.cast::<Calls>();
        calls.ticks.push((tick, dt));
        let mut len = 0;
        assert_eq!(crabby_world_len(world, &mut len), CrabbyStatus::CRABBY_OK);
        assert_eq!(len, 1);
        calls.stop_at != Some(tick)
    }
}

#[test]
fn callbacks_use_fixed_ticks_and_return_an_owned_world() {
    let mut calls = Calls::default();
    let mut world = ptr::null_mut();
    // SAFETY: outputs and user state live through the call; callbacks follow the ABI.
    unsafe {
        assert_eq!(
            crabby_run_headless(
                3,
                120,
                Some(setup),
                Some(step),
                (&mut calls as *mut Calls).cast(),
                &mut world
            ),
            CrabbyStatus::CRABBY_OK
        );
        assert!(calls.setup);
        assert_eq!(calls.ticks.iter().map(|p| p.0).collect::<Vec<_>>(), [0, 1, 2]);
        assert!(calls.ticks.iter().all(|p| (p.1 - 1.0 / 120.0).abs() < 1e-8));
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
    }
}

#[test]
fn false_stops_the_run_and_zero_ticks_still_sets_up() {
    for (ticks, stop_at, expected) in [(100, Some(1), 2), (0, None, 0)] {
        let mut calls = Calls {
            stop_at,
            ..Default::default()
        };
        let mut world = ptr::null_mut();
        // SAFETY: all pointers are live and callbacks honor borrowing rules.
        unsafe {
            assert_eq!(
                crabby_run_headless(
                    ticks,
                    60,
                    Some(setup),
                    Some(step),
                    (&mut calls as *mut Calls).cast(),
                    &mut world
                ),
                CrabbyStatus::CRABBY_OK
            );
            assert!(calls.setup);
            assert_eq!(calls.ticks.len(), expected);
            assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
        }
    }
}

#[test]
fn invalid_clock_or_output_is_rejected_before_callbacks() {
    let mut calls = Calls::default();
    let mut world = ptr::null_mut();
    // SAFETY: intentional null output is rejected; user state is live.
    unsafe {
        assert_eq!(
            crabby_run_headless(1, 0, Some(setup), None, (&mut calls as *mut Calls).cast(), &mut world),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_run_headless(
                1,
                60,
                Some(setup),
                None,
                (&mut calls as *mut Calls).cast(),
                ptr::null_mut()
            ),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
    }
    assert!(!calls.setup);
    assert!(world.is_null());
}

#[test]
fn absent_callbacks_are_allowed_and_false_setup_prevents_steps() {
    unsafe extern "C" fn stop(_: *mut CrabbyWorld, _: *mut c_void) -> bool {
        false
    }
    let mut calls = Calls::default();
    let mut world = ptr::null_mut();
    // SAFETY: outputs and callback state stay live throughout each synchronous run.
    unsafe {
        assert_eq!(
            crabby_run_headless(2, 60, None, None, ptr::null_mut(), &mut world),
            CrabbyStatus::CRABBY_OK
        );
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
        assert_eq!(
            crabby_run_headless(
                2,
                60,
                Some(stop),
                Some(step),
                (&mut calls as *mut Calls).cast(),
                &mut world
            ),
            CrabbyStatus::CRABBY_OK
        );
        assert!(calls.ticks.is_empty());
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
    }
}
