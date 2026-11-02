//! The native ABI's fixed-step runner uses the same engine loop as Rust games.
use std::ffi::c_void;

use crabbyengine::{Ctx, Game};

use crate::{CrabbyStatus, CrabbyWorld, boundary, error, owned_world, with_lent_world};

/// Setup callback. The world is borrowed until return; `false` stops before ticking.
///
/// ```
/// use crabby_ffi::CrabbySetupFn;
/// let no_setup: CrabbySetupFn = None;
/// assert!(no_setup.is_none());
/// ```
pub type CrabbySetupFn = Option<unsafe extern "C" fn(*mut CrabbyWorld, *mut c_void) -> bool>;

/// Fixed-step callback, with zero-based tick and timestep in seconds.
/// Return `false` to stop after this callback. Never unwind across the C boundary.
///
/// ```
/// use crabby_ffi::CrabbyStepFn;
/// let no_system: CrabbyStepFn = None;
/// assert!(no_system.is_none());
/// ```
pub type CrabbyStepFn = Option<unsafe extern "C" fn(*mut CrabbyWorld, u64, f32, *mut c_void) -> bool>;

/// Runs at most `ticks` fixed simulation steps without a window, GPU or audio.
///
/// `hz` must be nonzero. Both callbacks may be null. Setup runs even for zero
/// ticks. The callbacks receive a borrowed world: use normal world operations,
/// but do not retain or free that handle. On success, `out_world` receives an
/// owned world, which must eventually be freed. On failure it is null.
/// The control endpoint is disabled so environment settings cannot block this run.
///
/// # Safety
/// `out_world` must point to writable pointer storage. Any callbacks and `user`
/// must remain valid for this synchronous call, follow the world API's safety
/// rules, and never unwind. No concurrent access to a callback's world is allowed.
///
/// ```
/// use crabby_ffi::*;
/// let mut world = std::ptr::null_mut();
/// unsafe {
///     assert_eq!(crabby_run_headless(2, 60, None, None, std::ptr::null_mut(), &mut world),
///                CrabbyStatus::CRABBY_OK);
///     assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_run_headless(
    ticks: u64,
    hz: u32,
    setup: CrabbySetupFn,
    step: CrabbyStepFn,
    user: *mut c_void,
    out_world: *mut *mut CrabbyWorld,
) -> CrabbyStatus {
    boundary(|| {
        if out_world.is_null() {
            return Err(error(CrabbyStatus::CRABBY_INVALID_ARGUMENT, "out_world is null"));
        }
        // SAFETY: caller provides writable storage, checked non-null above.
        unsafe { out_world.write(std::ptr::null_mut()) };
        if hz == 0 {
            return Err(error(CrabbyStatus::CRABBY_INVALID_ARGUMENT, "hz must be nonzero"));
        }
        let world = Game::new("C/C++ headless", 1, 1)
            .control_endpoint(false)
            .fixed_hz(hz)
            .sim_system(move |ctx: &mut Ctx| {
                if let Some(step) = step {
                    let tick = ctx.sim_tick();
                    let dt = ctx.dt();
                    // SAFETY: callback/user lifetimes are the caller's contract;
                    // the stack handle is valid only during this synchronous call.
                    let keep_running = with_lent_world(&mut ctx.world, |world| unsafe { step(world, tick, dt, user) });
                    if !keep_running {
                        ctx.quit();
                    }
                }
            })
            .run_headless(ticks, move |ctx| {
                if let Some(setup) = setup {
                    // SAFETY: same synchronous callback contract as the step above.
                    let keep_running = with_lent_world(&mut ctx.world, |world| unsafe { setup(world, user) });
                    if !keep_running {
                        ctx.quit();
                    }
                }
            });
        // SAFETY: caller's writable output remains valid through this call.
        unsafe { out_world.write(owned_world(world)) };
        Ok(())
    })
}
