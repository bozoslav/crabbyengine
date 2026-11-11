//! `ponko_last_error_kind`: the kind of the last failure on this thread,
//! next to `ponko_last_error`'s message. Failing calls keep the status they
//! always returned.
use ponko_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::null_mut;

use PonkoErrorKind::*;
use PonkoStatus::*;

/// Runs `body` with a headless game's context during setup.
fn run_ctx(body: impl FnOnce(*mut PonkoCtx)) {
    type Body = Box<dyn FnOnce(*mut PonkoCtx)>;
    struct Slot {
        body: Option<Body>,
        panic: Option<Box<dyn std::any::Any + Send>>,
    }
    unsafe extern "C" fn trampoline(ctx: *mut PonkoCtx, user: *mut c_void) {
        // SAFETY: `user` is the `Slot` below, alive for the run.
        let slot = unsafe { &mut *user.cast::<Slot>() };
        let body = slot.body.take().expect("setup runs once");
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(ctx))) {
            slot.panic = Some(p);
        }
    }
    let body: Box<dyn FnOnce(*mut PonkoCtx) + '_> = Box::new(body);
    // SAFETY: the box is consumed inside the headless run, before this returns.
    let body: Body = unsafe { std::mem::transmute(body) };
    let mut slot = Slot {
        body: Some(body),
        panic: None,
    };
    unsafe {
        let game = ponko_game_new(c"errors".as_ptr(), 1, 1);
        assert_eq!(ponko_game_hot_reload(game, false), PONKO_OK);
        let mut world = null_mut();
        let status = ponko_game_run_headless(game, 1, Some(trampoline), (&raw mut slot).cast(), &mut world);
        if let Some(p) = slot.panic.take() {
            std::panic::resume_unwind(p);
        }
        assert_eq!(status, PONKO_OK);
        ponko_world_free(world);
    }
}

fn last_error() -> String {
    // SAFETY: the engine's thread-local, NUL-terminated buffer.
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn engine_errors_report_their_kind_and_keep_their_status() {
    run_ctx(|ctx| unsafe {
        let mut shader = PonkoShader { id: 0 };
        let status = ponko_ctx_load_shader(ctx, c"no/such.wgsl".as_ptr(), &mut shader);
        assert_eq!(status, PONKO_INVALID_ARGUMENT, "the status is unchanged");
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_NOT_FOUND);
        assert!(last_error().contains("no/such.wgsl"), "{}", last_error());
        assert!(!last_error().starts_with("ponko:"), "messages keep their old form");

        let broken = c"fn surface(s: Surface) -> Surface { return 1.0; }";
        let status = ponko_ctx_create_shader(ctx, c"broken".as_ptr(), broken.as_ptr(), &mut shader);
        assert_eq!(status, PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_SHADER);
        assert!(last_error().starts_with("broken:1:"), "{}", last_error());
    });
}

#[test]
fn failures_without_an_engine_error_get_a_kind_from_their_status() {
    unsafe {
        let mut shader = PonkoShader { id: 0 };
        let status = ponko_ctx_load_shader(null_mut(), c"x.wgsl".as_ptr(), &mut shader);
        assert_eq!(status, PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_INVALID);
    }
}

#[test]
fn the_kind_starts_empty_and_successful_calls_leave_it() {
    // Each test runs on its own thread, so nothing has failed here yet.
    std::thread::spawn(|| unsafe {
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_NONE);
        let world = ponko_world_new();
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_NONE);
        let mut shader = PonkoShader { id: 0 };
        ponko_ctx_load_shader(null_mut(), c"x.wgsl".as_ptr(), &mut shader);
        assert_eq!(ponko_world_free(world), PONKO_OK);
        assert_eq!(
            ponko_last_error_kind(),
            PONKO_ERROR_INVALID,
            "a success keeps the last kind"
        );
    })
    .join()
    .unwrap();
}
