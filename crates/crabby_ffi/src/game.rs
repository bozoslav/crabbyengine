//! `CrabbyGame`: the C side of Rust's `Game` builder. Systems are C function
//! pointers plus a `user` pointer; they get a `CrabbyCtx`.
use std::ffi::{CStr, c_char, c_void};

use crabbyengine::{Ctx, Game};

use crate::ctx::{CrabbyCtx, with_ctx};
use crate::{CrabbyStatus, CrabbyWorld, Failure, boundary, error, invalid, owned_world};

/// A game being set up. Run it with `crabby_game_run` (window) or
/// `crabby_game_run_headless`; both consume it. Free it only if never run.
pub struct CrabbyGame {
    game: Option<Game>,
}

/// A system or setup callback. `ctx` is valid until it returns.
/// ```
/// let none: crabby_ffi::CrabbySystemFn = None;
/// assert!(none.is_none());
/// ```
pub type CrabbySystemFn = Option<unsafe extern "C" fn(ctx: *mut CrabbyCtx, user: *mut c_void)>;

/// Starts a game with a window title and size (ignored when headless). Like
/// Rust's `Game::new`, it includes the built-in systems (sprite drawing,
/// animation, tilemaps); remove them with `crabby_game_remove_system`.
/// Returns null on bad arguments (see `crabby_last_error`).
/// # Safety
/// `title` must be a NUL-terminated string.
/// ```
/// let g = unsafe { crabby_ffi::crabby_game_new(c"My game".as_ptr(), 800, 600) };
/// unsafe { crabby_ffi::crabby_game_free(g); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_new(title: *const c_char, width: u32, height: u32) -> *mut CrabbyGame {
    let mut result = std::ptr::null_mut();
    boundary(|| {
        // SAFETY: caller supplies a NUL-terminated string.
        let title = unsafe { name(title)? };
        result = Box::into_raw(Box::new(CrabbyGame {
            game: Some(Game::new(title, width, height)),
        }));
        Ok(())
    });
    result
}

/// Frees a game that was never run. Null is ignored.
/// # Safety
/// `game` must be null or a live, never-run game, not used afterwards.
/// ```
/// unsafe { crabby_ffi::crabby_game_free(std::ptr::null_mut()); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_free(game: *mut CrabbyGame) {
    if !game.is_null() {
        // SAFETY: caller passes a game from crabby_game_new, once.
        drop(unsafe { Box::from_raw(game) });
    }
}

unsafe fn name<'a>(ptr: *const c_char) -> Result<&'a str, Failure> {
    if ptr.is_null() {
        return Err(invalid("string is null"));
    }
    // SAFETY: caller supplies a NUL-terminated string.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| invalid("string is not UTF-8"))
}

/// Applies a builder step. `f` hands the game back with the result, so a
/// rejected argument leaves the game as it was (only a panic loses it).
unsafe fn build(game: *mut CrabbyGame, f: impl FnOnce(Game) -> (Game, Result<(), Failure>)) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies a live game.
        let game = unsafe { game.as_mut() }.ok_or_else(|| invalid("game is null"))?;
        let g = game.game.take().ok_or_else(|| invalid("game was already run"))?;
        let (g, result) = f(g);
        game.game = Some(g);
        result
    })
}
fn ok(g: Game) -> (Game, Result<(), Failure>) {
    (g, Ok(()))
}
fn reject(g: Game, message: &'static str) -> (Game, Result<(), Failure>) {
    (g, Err(invalid(message)))
}

/// Simulation ticks per second (default 60). Must be at least 1.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_fixed_hz(std::ptr::null_mut(), 30) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_fixed_hz(game: *mut CrabbyGame, hz: u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        build(game, |g| {
            if hz == 0 {
                reject(g, "hz must be at least 1")
            } else {
                ok(g.fixed_hz(hz))
            }
        })
    }
}
/// Seed of `crabby_ctx_random_bits` streams (runs with the same seed repeat exactly).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_seed(std::ptr::null_mut(), 7) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_seed(game: *mut CrabbyGame, seed: u64) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { build(game, |g| ok(g.seed(seed))) }
}
/// Wait for the display between frames (default on).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_vsync(std::ptr::null_mut(), false) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_vsync(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { build(game, |g| ok(g.vsync(on))) }
}
/// Turns on the built-in spatial grid with cells of `cell_size`, rebuilt
/// from every `Transform2D` at the start of each tick (`crabby_ctx_spatial_*`).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_spatial_grid(std::ptr::null_mut(), 32.0) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_spatial_grid(game: *mut CrabbyGame, cell_size: f32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        build(game, |g| {
            if cell_size > 0.0 && cell_size.is_finite() {
                ok(g.spatial_grid(cell_size))
            } else {
                reject(g, "cell size must be positive")
            }
        })
    }
}
/// Opens the control endpoint for tools and agents (crabby-mcp); also on
/// with `CRABBY_CONTROL=1`.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_control_endpoint(std::ptr::null_mut(), true) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_control_endpoint(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { build(game, |g| ok(g.control_endpoint(on))) }
}

/// Systems are named by `&'static str`, so C names are interned: each
/// distinct name is stored once for the life of the program.
unsafe fn system_name(ptr: *const c_char, fallback: &'static str) -> Result<&'static str, Failure> {
    static NAMES: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    if ptr.is_null() {
        return Ok(fallback);
    }
    // SAFETY: forwarded caller contract.
    let name = unsafe { name(ptr)? };
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&known) = names.iter().find(|n| **n == name) {
        return Ok(known);
    }
    let interned: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.push(interned);
    Ok(interned)
}

fn system(f: unsafe extern "C" fn(*mut CrabbyCtx, *mut c_void), user: *mut c_void) -> impl FnMut(&mut Ctx) + 'static {
    move |ctx: &mut Ctx| {
        // SAFETY: the caller of crabby_game_*_system promised `f` and `user` stay valid while the game runs.
        with_ctx(ctx, |c| unsafe { f(c, user) })
    }
}

/// Adds a simulation system: runs on the fixed tick, every `every` ticks
/// (1 = every tick; larger = simulation LOD, like Rust's `sim_system_every`).
/// `name` (may be null) shows up in timings and tools.
/// # Safety
/// Live game; `system` and `user` must stay valid while the game runs.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_sim_system(std::ptr::null_mut(), std::ptr::null(), 1, None, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_sim_system(
    game: *mut CrabbyGame,
    name: *const c_char,
    every: u32,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        let checked = (|| {
            let f = system_fn.ok_or_else(|| invalid("system is null"))?;
            if every == 0 {
                return Err(invalid("every must be at least 1"));
            }
            Ok((f, system_name(name, "C sim system")?))
        })();
        build(game, |g| match checked {
            Ok((f, name)) => ok(g.named_sim_system(name, every, system(f, user))),
            Err(e) => (g, Err(e)),
        })
    }
}

/// Adds a frame system: runs once per frame before drawing (input, camera, UI).
/// Not run by `crabby_game_run_headless`.
/// # Safety
/// Live game; `system` and `user` must stay valid while the game runs.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_game_system(std::ptr::null_mut(), std::ptr::null(), None, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_system(
    game: *mut CrabbyGame,
    name: *const c_char,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        let checked = (|| {
            Ok((
                system_fn.ok_or_else(|| invalid("system is null"))?,
                system_name(name, "C system")?,
            ))
        })();
        build(game, |g| match checked {
            Ok((f, name)) => ok(g.named_system(name, system(f, user))),
            Err(e) => (g, Err(e)),
        })
    }
}

/// Removes systems by name: your own, or a built-in such as
/// `"crabbyengine::ecs::draw_sprites"`. `CRABBY_NOT_FOUND` if none matched.
/// # Safety
/// Live game; NUL-terminated name.
/// ```
/// unsafe {
///     let g = crabby_ffi::crabby_game_new(c"t".as_ptr(), 1, 1);
///     assert_eq!(crabby_ffi::crabby_game_remove_system(g, c"crabbyengine::ecs::draw_sprites".as_ptr()), crabby_ffi::CrabbyStatus::CRABBY_OK);
///     crabby_ffi::crabby_game_free(g);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_remove_system(game: *mut CrabbyGame, name_ptr: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        let checked = name(name_ptr);
        build(game, |mut g| match checked {
            Ok(name) if g.remove_named_system(name) > 0 => ok(g),
            Ok(_) => (
                g,
                Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "no system with that name")),
            ),
            Err(e) => (g, Err(e)),
        })
    }
}

unsafe fn take(game: *mut CrabbyGame) -> Result<Game, Failure> {
    if game.is_null() {
        return Err(invalid("game is null"));
    }
    // SAFETY: caller passes a game from crabby_game_new; running consumes it.
    let game = unsafe { Box::from_raw(game) };
    game.game.ok_or_else(|| invalid("game was already run"))
}

fn setup_fn(setup: crate::CrabbySystemFn, user: *mut c_void) -> impl FnOnce(&mut Ctx) {
    move |ctx: &mut Ctx| {
        if let Some(f) = setup {
            // SAFETY: the caller promised `f` and `user` are valid for the run.
            with_ctx(ctx, |c| unsafe { f(c, user) });
        }
    }
}

/// Opens the window and runs the game until it quits, then frees `game`.
/// `setup` (may be null) runs once first: load textures, spawn entities.
/// On macOS it must be called from the main thread.
/// # Safety
/// Live game (consumed even on failure); callbacks and `user` valid until it returns.
/// ```no_run
/// unsafe {
///     let g = crabby_ffi::crabby_game_new(c"My game".as_ptr(), 800, 600);
///     crabby_ffi::crabby_game_run(g, None, std::ptr::null_mut());
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_run(
    game: *mut CrabbyGame,
    setup: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let game = unsafe { take(game)? };
        game.run(setup_fn(setup, user));
        Ok(())
    })
}

/// Runs up to `ticks` simulation ticks with no window, GPU or audio, as fast
/// as possible, then frees `game` and gives you the world in `*out_world`
/// (free it with `crabby_world_free`). `setup` runs once first; frame systems
/// don't run. A system can stop early with `crabby_ctx_quit`.
/// # Safety
/// Live game (consumed even on failure); writable `out_world`; callbacks and
/// `user` valid until it returns.
/// ```
/// use crabby_ffi::*;
/// unsafe {
///     let g = crabby_game_new(c"sim".as_ptr(), 1, 1);
///     let mut world = std::ptr::null_mut();
///     assert_eq!(crabby_game_run_headless(g, 10, None, std::ptr::null_mut(), &mut world), CrabbyStatus::CRABBY_OK);
///     crabby_world_free(world);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_run_headless(
    game: *mut CrabbyGame,
    ticks: u64,
    setup: CrabbySystemFn,
    user: *mut c_void,
    out_world: *mut *mut CrabbyWorld,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let game = unsafe { take(game)? };
        if out_world.is_null() {
            return Err(invalid("out_world is null"));
        }
        // SAFETY: checked non-null; caller supplies writable storage.
        unsafe { out_world.write(std::ptr::null_mut()) };
        let world = game.run_headless(ticks, setup_fn(setup, user));
        // SAFETY: as above.
        unsafe { out_world.write(owned_world(world)) };
        Ok(())
    })
}
