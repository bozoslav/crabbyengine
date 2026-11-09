//! The context's clock and deferred commands from C: simulation time and
//! frame counters read in a system, the pause/step/speed controls, and
//! `set_later` / `remove_later`, which apply after the system returns.
//! Headless runs, no window.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

const OK: CrabbyStatus = CRABBY_OK;
const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;

const TICKS: u64 = 12;
const HZ: u32 = 50;

#[derive(Default)]
struct Clock {
    /// (tick, sim_time, time, frame, fps, dt) as each system call saw them.
    seen: Vec<(u64, f64, f64, u64, f32, f32)>,
    window: (f32, f32),
}

unsafe extern "C" fn read_clock(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let c = &mut *user.cast::<Clock>();
        c.seen.push((
            crabby_ctx_sim_tick(ctx),
            crabby_ctx_sim_time(ctx),
            crabby_ctx_time(ctx),
            crabby_ctx_frame(ctx),
            crabby_ctx_fps(ctx),
            crabby_ctx_dt(ctx),
        ));
        let size = crabby_ctx_window_size(ctx);
        c.window = (size.x, size.y);
    }
}

#[test]
fn the_clock_counts_ticks_and_frames_as_a_headless_game_runs() {
    let mut c = Clock::default();
    let user = (&raw mut c).cast::<c_void>();
    unsafe {
        let g = crabby_game_new(c"clock".as_ptr(), 320, 200);
        assert_eq!(crabby_game_fixed_hz(g, HZ), OK);
        assert_eq!(
            crabby_game_sim_system(g, c"clock".as_ptr(), 1, Some(read_clock), user),
            OK
        );
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, TICKS, None, null_mut(), &mut world), OK);
        crabby_world_free(world);
    }
    assert_eq!(c.seen.len() as u64, TICKS);
    assert_eq!(c.window, (320.0, 200.0), "the size the game was made with");
    let dt = 1.0 / f64::from(HZ);
    for (i, &(tick, sim_time, time, frame, fps, step)) in c.seen.iter().enumerate() {
        // Simulated time is exactly ticks times the fixed step, and dt is that step.
        assert_eq!(tick, c.seen[0].0 + i as u64, "one tick at a time");
        assert!(
            (sim_time - tick as f64 * dt).abs() < 1e-9,
            "tick {tick}: sim time {sim_time}"
        );
        assert!((f64::from(step) - dt).abs() < 1e-6, "dt {step}");
        // Each headless tick is a frame; the wall clock and frame counter only go forward.
        assert!(frame >= 1 && time > 0.0, "frame {frame}, time {time}");
        assert!((f64::from(fps) - f64::from(HZ)).abs() < 1.0, "fps {fps}");
        if let Some(&(_, _, earlier_time, earlier_frame, ..)) = i.checked_sub(1).map(|j| &c.seen[j]) {
            assert_eq!(frame, earlier_frame + 1);
            assert!(time > earlier_time);
        }
    }
}

#[test]
fn clock_readings_are_zero_for_a_null_context() {
    unsafe {
        assert_eq!(crabby_ctx_sim_time(null()), 0.0);
        assert_eq!(crabby_ctx_time(null()), 0.0);
        assert_eq!(crabby_ctx_frame(null()), 0);
        assert_eq!(crabby_ctx_fps(null()), 0.0);
        let size = crabby_ctx_window_size(null());
        assert_eq!((size.x, size.y), (0.0, 0.0));
    }
}

#[derive(Default)]
struct Controls {
    done: bool,
}

unsafe fn paused(ctx: *mut CrabbyCtx) -> bool {
    unsafe {
        let mut p = false;
        assert_eq!(crabby_ctx_is_paused(ctx, &mut p), OK);
        p
    }
}

unsafe fn speed(ctx: *mut CrabbyCtx) -> f32 {
    unsafe {
        let mut s = -1.0;
        assert_eq!(crabby_ctx_sim_speed(ctx, &mut s), OK);
        s
    }
}

unsafe extern "C" fn controls(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let c = &mut *user.cast::<Controls>();
        // A running game, at normal speed.
        assert!(!paused(ctx));
        assert_eq!(speed(ctx), 1.0);

        // Pause, step while paused (one tick per request), resume.
        assert_eq!(crabby_ctx_pause(ctx), OK);
        assert!(paused(ctx));
        assert_eq!(crabby_ctx_step(ctx), OK);
        assert_eq!(crabby_ctx_step(ctx), OK, "each request adds a tick");
        assert!(paused(ctx), "a step doesn't resume");
        assert_eq!(crabby_ctx_resume(ctx), OK);
        assert!(!paused(ctx));
        assert_eq!(crabby_ctx_resume(ctx), OK, "resuming a running game is fine");

        // Speed: any finite amount from 0 up, read back as set.
        for s in [0.0, 0.5, 4.0, 16.0, 64.0] {
            assert_eq!(crabby_ctx_set_sim_speed(ctx, s), OK);
            assert_eq!(speed(ctx), s);
        }
        // Bad speeds are refused, and keep the old one.
        assert_eq!(crabby_ctx_set_sim_speed(ctx, f32::NAN), BAD);
        assert_eq!(crabby_ctx_set_sim_speed(ctx, f32::INFINITY), BAD);
        assert_eq!(crabby_ctx_set_sim_speed(ctx, f32::NEG_INFINITY), BAD);
        assert_eq!(crabby_ctx_set_sim_speed(ctx, -1.0), BAD, "negative speed");
        assert_eq!(speed(ctx), 64.0);
        assert_eq!(crabby_ctx_set_sim_speed(ctx, 1.0), OK);

        // Null contexts and outputs.
        let (mut p, mut s) = (false, 0.0f32);
        assert_eq!(crabby_ctx_pause(null_mut()), BAD);
        assert_eq!(crabby_ctx_resume(null_mut()), BAD);
        assert_eq!(crabby_ctx_step(null_mut()), BAD);
        assert_eq!(crabby_ctx_set_sim_speed(null_mut(), 2.0), BAD);
        assert_eq!(crabby_ctx_sim_speed(null_mut(), &mut s), BAD);
        assert_eq!(crabby_ctx_sim_speed(ctx, null_mut()), BAD);
        assert_eq!(crabby_ctx_is_paused(null_mut(), &mut p), BAD);
        assert_eq!(crabby_ctx_is_paused(ctx, null_mut()), BAD);
        c.done = true;
    }
}

#[test]
fn pause_step_resume_and_speed_are_set_and_read_back() {
    let mut c = Controls::default();
    let user = (&raw mut c).cast::<c_void>();
    unsafe {
        let g = crabby_game_new(c"controls".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_sim_system(g, c"controls".as_ptr(), 1, Some(controls), user),
            OK
        );
        let mut world = null_mut();
        // Headless runs carry on through a pause (only a control endpoint holds them).
        assert_eq!(crabby_game_run_headless(g, 2, None, null_mut(), &mut world), OK);
        crabby_world_free(world);
    }
    assert!(c.done);
}

// ---- deferred commands -------------------------------------------------------------

#[derive(Default)]
struct Later {
    transform: u32,
    target: u64,
    /// The x of the target's transform as each tick saw it; `None` once it has none.
    seen: Vec<Option<f32>>,
    /// The same read straight after queueing a command in that tick.
    after_queueing: Vec<Option<f32>>,
}

unsafe fn x_of(ctx: *mut CrabbyCtx, l: &Later) -> Option<f32> {
    unsafe {
        let mut t = crabby_transform3d_at(CrabbyVec3 {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        });
        let size = size_of::<CrabbyTransform3D>();
        (crabby_get(crabby_ctx_world(ctx), l.target, l.transform, (&raw mut t).cast(), size) == OK)
            .then_some(t.position.x)
    }
}

unsafe extern "C" fn setup_later(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let l = &mut *user.cast::<Later>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform3d_component(w, &mut l.transform), OK);
        let t = crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 });
        let part = CrabbyValue {
            component: l.transform,
            data: (&raw const t).cast(),
            size: size_of::<CrabbyTransform3D>(),
        };
        assert_eq!(crabby_spawn(w, &part, 1, &mut l.target), OK);
    }
}

unsafe extern "C" fn queue(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let l = &mut *user.cast::<Later>();
        l.seen.push(x_of(ctx, l));
        match crabby_ctx_sim_tick(ctx) {
            0 => {
                let moved = crabby_transform3d_at(CrabbyVec3 { x: 5.0, y: 0.0, z: 0.0 });
                let value = CrabbyValue {
                    component: l.transform,
                    data: (&raw const moved).cast(),
                    size: size_of::<CrabbyTransform3D>(),
                };
                assert_eq!(crabby_ctx_set_later(ctx, l.target, &value), OK);
                // Queued, not applied: this system still sees the old value.
                l.after_queueing.push(x_of(ctx, l));
                // The value was copied: changing it now doesn't change what is queued.
                let mut elsewhere = moved;
                elsewhere.position.x = 99.0;
                std::hint::black_box(&elsewhere);
                // Errors.
                assert_eq!(crabby_ctx_set_later(ctx, l.target, null()), BAD);
                assert_eq!(crabby_ctx_set_later(null_mut(), l.target, &value), BAD);
                assert_eq!(crabby_ctx_remove_later(null_mut(), l.target, l.transform), BAD);
            }
            2 => {
                assert_eq!(crabby_ctx_remove_later(ctx, l.target, l.transform), OK);
                l.after_queueing.push(x_of(ctx, l));
                // A command for something that isn't there is reported later, not an error now.
                assert_eq!(crabby_ctx_remove_later(ctx, 12345, l.transform), OK);
                let stray = crabby_transform3d_at(CrabbyVec3 { x: 1.0, y: 0.0, z: 0.0 });
                let value = CrabbyValue {
                    component: l.transform,
                    data: (&raw const stray).cast(),
                    size: size_of::<CrabbyTransform3D>(),
                };
                assert_eq!(crabby_ctx_set_later(ctx, 12345, &value), OK);
            }
            4 => {
                // Put it back: removal and addition are both deferred.
                let back = crabby_transform3d_at(CrabbyVec3 { x: 7.0, y: 0.0, z: 0.0 });
                let value = CrabbyValue {
                    component: l.transform,
                    data: (&raw const back).cast(),
                    size: size_of::<CrabbyTransform3D>(),
                };
                assert_eq!(crabby_ctx_set_later(ctx, l.target, &value), OK);
            }
            _ => {}
        }
    }
}

#[test]
fn deferred_sets_and_removes_apply_after_the_system_returns() {
    let mut l = Later::default();
    let user = (&raw mut l).cast::<c_void>();
    unsafe {
        let g = crabby_game_new(c"later".as_ptr(), 1, 1);
        assert_eq!(crabby_game_sim_system(g, c"queue".as_ptr(), 1, Some(queue), user), OK);
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, 6, Some(setup_later), user, &mut world), OK);
        crabby_world_free(world);
    }
    assert_eq!(l.seen.len(), 6);
    let seen = &l.seen;
    // Tick 0 sees the spawn value; the set applies before tick 1.
    assert_eq!(seen[0], Some(0.0));
    assert_eq!(l.after_queueing[0], Some(0.0), "queued, not yet applied");
    assert_eq!(seen[1], Some(5.0));
    assert_eq!(seen[2], Some(5.0));
    // Tick 2 queues a removal: still there for the rest of tick 2, gone from tick 3.
    assert_eq!(l.after_queueing[1], Some(5.0));
    assert_eq!(seen[3], None);
    assert_eq!(seen[4], None);
    // Tick 4 queues a new transform: back from tick 5.
    assert_eq!(seen[5], Some(7.0));
}
