//! Helpers shared by the FFI tests of features that run inside a game.
#![allow(dead_code)]
use ponko_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::null_mut;

/// The thread's last error message.
pub fn last_error() -> String {
    // SAFETY: the engine's thread-local, NUL-terminated buffer.
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}

/// The thread's last error path.
pub fn last_path() -> String {
    // SAFETY: the engine's thread-local, NUL-terminated buffer.
    unsafe { CStr::from_ptr(ponko_last_error_path()) }
        .to_string_lossy()
        .into_owned()
}

/// Runs `game` headless for `ticks` with `setup` as its setup callback
/// (panics in `setup` come out of this call) and returns the world.
pub fn run_game(game: *mut PonkoGame, ticks: u64, setup: impl FnOnce(*mut PonkoCtx)) -> *mut PonkoWorld {
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
    let body: Box<dyn FnOnce(*mut PonkoCtx) + '_> = Box::new(setup);
    // SAFETY: the box is consumed inside the headless run, before this returns.
    let body: Body = unsafe { std::mem::transmute(body) };
    let mut slot = Slot {
        body: Some(body),
        panic: None,
    };
    let mut world = null_mut();
    // SAFETY: a live game, run once; the slot outlives the run.
    let status = unsafe { ponko_game_run_headless(game, ticks, Some(trampoline), (&raw mut slot).cast(), &mut world) };
    if let Some(p) = slot.panic.take() {
        std::panic::resume_unwind(p);
    }
    assert_eq!(status, PonkoStatus::PONKO_OK, "{}", last_error());
    world
}

/// A new headless game with hot reload off.
pub fn new_game() -> *mut PonkoGame {
    // SAFETY: a literal title.
    unsafe {
        let game = ponko_game_new(c"test".as_ptr(), 1, 1);
        assert_eq!(ponko_game_hot_reload(game, false), PonkoStatus::PONKO_OK);
        game
    }
}

/// Runs `body` with a headless game's context during setup.
pub fn run_ctx(body: impl FnOnce(*mut PonkoCtx)) {
    let world = run_game(new_game(), 0, body);
    // SAFETY: the world the run returned, freed once.
    unsafe { ponko_world_free(world) };
}

/// A 1x1 RGBA PNG.
pub const PIXEL_PNG: [u8; 70] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196,
    137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0,
    0, 73, 69, 78, 68, 174, 66, 96, 130,
];

/// An action map with `jump` on Space, `throttle` on W/S and `move` on WASD.
pub fn action_map() -> *mut PonkoActionMap {
    fn binding(kind: u32, keys: [PonkoKey; 4]) -> PonkoActionBinding {
        PonkoActionBinding {
            kind,
            pad: 0,
            first: keys[0] as u32,
            second: keys[1] as u32,
            third: keys[2] as u32,
            fourth: keys[3] as u32,
            deadzone: 0.0,
        }
    }
    use PonkoKey::*;
    let map = ponko_action_map_new();
    let jump = [binding(0, [PONKO_KEY_SPACE; 4])];
    let throttle = [binding(3, [PONKO_KEY_S, PONKO_KEY_W, PONKO_KEY_SPACE, PONKO_KEY_SPACE])];
    let walk = [binding(5, [PONKO_KEY_A, PONKO_KEY_D, PONKO_KEY_W, PONKO_KEY_S])];
    // SAFETY: a live map and arrays of the stated lengths.
    unsafe {
        assert_eq!(
            ponko_action_map_set(map, c"jump".as_ptr(), 0, jump.as_ptr(), 1),
            PonkoStatus::PONKO_OK
        );
        assert_eq!(
            ponko_action_map_set(map, c"throttle".as_ptr(), 1, throttle.as_ptr(), 1),
            PonkoStatus::PONKO_OK
        );
        assert_eq!(
            ponko_action_map_set(map, c"move".as_ptr(), 2, walk.as_ptr(), 1),
            PonkoStatus::PONKO_OK
        );
    }
    map
}

/// Installs [`action_map`] in `ctx`.
pub unsafe fn install_actions(ctx: *mut PonkoCtx) {
    let map = action_map();
    // SAFETY: a live ctx and map.
    unsafe {
        assert_eq!(ponko_ctx_set_actions(ctx, map), PonkoStatus::PONKO_OK);
        ponko_action_map_free(map);
    }
}
