//! `CrabbyCtx`: what a C system gets, the C side of Rust's `Ctx`.
//!
//! One busy flag (the lent world's) guards the world and every context call,
//! so nothing can reach the `Ctx` while a query borrows its world. Commands
//! and the spatial grid are separate fields of `Ctx`, reached through raw
//! field pointers, so they stay usable inside query callbacks.
use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicBool, Ordering};

use crabby_ecs::{Commands, Entity};
use crabbyengine::{Ctx, SpatialGrid};

use crate::keys::{button_from_c, key_from_c};
use crate::types::*;
use crate::{
    CrabbyStatus, CrabbyValue, CrabbyWorld, Failure, Kind, boundary, error, insert_value, invalid, spawn_values,
};

/// What a system gets: the world, the clock, input, drawing and commands.
/// Valid only during the callback it was passed to.
pub struct CrabbyCtx {
    /// The context's world, lent for this callback. Its busy flag guards `ctx` too.
    pub(crate) world: CrabbyWorld,
    ctx: *mut Ctx,
    commands_busy: AtomicBool,
    dt: f32,
    sim_tick: u64,
    sim_time: f64,
    time: f64,
    frame: u64,
    fps: f32,
    random_bits: u64,
    window: CrabbyVec2,
}

/// Lends `ctx` to C for one callback.
pub(crate) fn with_ctx<R>(ctx: &mut Ctx, f: impl FnOnce(*mut CrabbyCtx) -> R) -> R {
    let snapshot = (
        ctx.dt(),
        ctx.sim_tick(),
        ctx.sim_time(),
        ctx.time(),
        ctx.frame(),
        ctx.fps(),
        ctx.random().to_bits(),
        to_c(ctx.window_size()),
    );
    let raw: *mut Ctx = ctx;
    // SAFETY: `raw` comes from a live `&mut Ctx`; from here on it is only used
    // through raw pointers, and the busy flags keep accesses from overlapping.
    let world = unsafe { &raw mut (*raw).world };
    let mut handle = CrabbyCtx {
        world: CrabbyWorld::new(world, false),
        ctx: raw,
        commands_busy: AtomicBool::new(false),
        dt: snapshot.0,
        sim_tick: snapshot.1,
        sim_time: snapshot.2,
        time: snapshot.3,
        frame: snapshot.4,
        fps: snapshot.5,
        random_bits: snapshot.6,
        window: snapshot.7,
    };
    f(&mut handle)
}

unsafe fn ctx_access<R>(
    handle: *const CrabbyCtx,
    f: impl FnOnce(&mut Ctx) -> Result<R, Failure>,
) -> Result<R, Failure> {
    if handle.is_null() {
        return Err(invalid("ctx is null"));
    }
    // SAFETY: caller supplies a live ctx from a running callback.
    let handle = unsafe { &*handle };
    let Some(_guard) = handle.world.lock() else {
        return Err(error(
            CrabbyStatus::CRABBY_BUSY,
            "ctx is borrowed (a query is running?)",
        ));
    };
    // SAFETY: the busy flag grants exclusive access to the whole Ctx.
    f(unsafe { &mut *handle.ctx })
}

unsafe fn commands<R>(
    handle: *const CrabbyCtx,
    f: impl FnOnce(&mut Commands) -> Result<R, Failure>,
) -> Result<R, Failure> {
    if handle.is_null() {
        return Err(invalid("ctx is null"));
    }
    // SAFETY: caller supplies a live ctx from a running callback.
    let handle = unsafe { &*handle };
    if handle.world.parallel.load(Ordering::Acquire) {
        return Err(error(
            CrabbyStatus::CRABBY_BUSY,
            "commands are off during crabby_query_par",
        ));
    }
    if handle.commands_busy.swap(true, Ordering::Acquire) {
        return Err(error(CrabbyStatus::CRABBY_BUSY, "commands are in use"));
    }
    struct Reset<'a>(&'a AtomicBool);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _reset = Reset(&handle.commands_busy);
    // SAFETY: `commands` is a field no other ABI call touches while this flag
    // is set; ctx-wide calls never run while a C callback (our caller) runs
    // inside them, because they don't call back into C.
    f(unsafe { &mut (*handle.ctx).commands })
}

/// The built-in grid, read-only. Usable from any thread during a callback.
unsafe fn spatial<'a>(handle: *const CrabbyCtx) -> Result<&'a SpatialGrid, Failure> {
    if handle.is_null() {
        return Err(invalid("ctx is null"));
    }
    // SAFETY: the engine only rebuilds the grid between systems; no ABI call writes it.
    Ok(unsafe { &(*(*handle).ctx).spatial })
}

unsafe fn out<T>(ptr: *mut T, value: T) -> Result<(), Failure> {
    if ptr.is_null() {
        return Err(invalid("output is null"));
    }
    // SAFETY: caller supplies writable output (checked non-null).
    unsafe { ptr.write(value) };
    Ok(())
}

unsafe fn text<'a>(ptr: *const c_char) -> Result<&'a str, Failure> {
    if ptr.is_null() {
        return Err(invalid("string is null"));
    }
    // SAFETY: caller supplies a NUL-terminated string.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| invalid("string is not UTF-8"))
}

macro_rules! snapshot {
    ($(#[$doc:meta])* $name:ident -> $ty:ty = $field:ident) => {
        $(#[$doc])*
        /// Returns 0 for a null ctx. Safe to call from any thread during the callback.
        /// # Safety
        /// `ctx` must be null or live.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(ctx: *const CrabbyCtx) -> $ty {
            if ctx.is_null() {
                return Default::default();
            }
            // SAFETY: live ctx; snapshot fields are never written after creation.
            unsafe { (*ctx).$field }
        }
    };
}
snapshot!(
    /// Seconds this system covers: the fixed step in sim systems, the frame time otherwise.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_dt(std::ptr::null()) }, 0.0);
    /// ```
    crabby_ctx_dt -> f32 = dt
);
snapshot!(
    /// Simulation ticks completed so far.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_sim_tick(std::ptr::null()) }, 0);
    /// ```
    crabby_ctx_sim_tick -> u64 = sim_tick
);
snapshot!(
    /// Simulated seconds so far (exact: ticks × fixed step).
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_sim_time(std::ptr::null()) }, 0.0);
    /// ```
    crabby_ctx_sim_time -> f64 = sim_time
);
snapshot!(
    /// Wall-clock seconds since the game started.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_time(std::ptr::null()) }, 0.0);
    /// ```
    crabby_ctx_time -> f64 = time
);
snapshot!(
    /// Frames drawn so far.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_frame(std::ptr::null()) }, 0);
    /// ```
    crabby_ctx_frame -> u64 = frame
);
snapshot!(
    /// Frames per second, averaged.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_fps(std::ptr::null()) }, 0.0);
    /// ```
    crabby_ctx_fps -> f32 = fps
);
snapshot!(
    /// Base of this tick and system's random streams (Rust `ctx.random().to_bits()`).
    /// The C++ header's `crabby::rng` derives the same numbers as Rust from it.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_random_bits(std::ptr::null()) }, 0);
    /// ```
    crabby_ctx_random_bits -> u64 = random_bits
);
snapshot!(
    /// Window size in pixels.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_window_size(std::ptr::null()) }.x, 0.0);
    /// ```
    crabby_ctx_window_size -> CrabbyVec2 = window
);

/// The world of this context. Valid until the callback returns; never free it.
/// # Safety
/// `ctx` must be null or live.
/// ```
/// assert!(unsafe { crabby_ffi::crabby_ctx_world(std::ptr::null_mut()) }.is_null());
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_world(ctx: *mut CrabbyCtx) -> *mut CrabbyWorld {
    if ctx.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: live ctx.
    unsafe { &raw mut (*ctx).world }
}

macro_rules! ctx_call {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) |$c:ident| $body:expr) => {
        $(#[$doc])*
        /// # Safety
        /// `ctx` must be null or live; pointer arguments must be valid.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(ctx: *mut CrabbyCtx $(, $arg: $ty)*) -> CrabbyStatus {
            // SAFETY: forwarded caller contract.
            boundary(|| unsafe { ctx_access(ctx, |$c| $body) })
        }
    };
}

ctx_call!(
    /// Ends the game after this frame (or the headless run after this tick).
    /// ```
    /// use crabby_ffi::*;
    /// assert_eq!(unsafe { crabby_ctx_quit(std::ptr::null_mut()) }, CrabbyStatus::CRABBY_INVALID_ARGUMENT);
    /// ```
    crabby_ctx_quit() |c| { c.quit(); Ok(()) }
);
ctx_call!(
    /// Pauses the simulation (frame systems keep running).
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_pause(std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_pause() |c| { c.pause(); Ok(()) }
);
ctx_call!(
    /// Resumes a paused simulation.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_resume(std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_resume() |c| { c.resume(); Ok(()) }
);
ctx_call!(
    /// Runs one simulation tick while paused.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_step(std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_step() |c| { c.step(); Ok(()) }
);
ctx_call!(
    /// Simulation speed multiplier (1..64); results don't depend on it.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_sim_speed(std::ptr::null_mut(), 2.0) } as u32, 1);
    /// ```
    crabby_ctx_set_sim_speed(speed: f32) |c| {
        if !speed.is_finite() { return Err(invalid("speed must be finite")); }
        c.set_sim_speed(speed); Ok(())
    }
);
ctx_call!(
    /// Current simulation speed.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_sim_speed(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_sim_speed(out_speed: *mut f32) |c| out(out_speed, c.sim_speed())
);
ctx_call!(
    /// Whether the simulation is paused.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_is_paused(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_is_paused(out_paused: *mut bool) |c| out(out_paused, c.is_paused())
);

macro_rules! input {
    ($(#[$doc:meta])* $name:ident, $conv:ident, $method:ident) => {
        ctx_call!(
            $(#[$doc])*
            $name(input: u32, out_state: *mut bool) |c| {
                let input = $conv(input).ok_or_else(|| invalid("unknown key or mouse button"))?;
                out(out_state, c.$method(input))
            }
        );
    };
}
input!(
    /// Whether a key (`CrabbyKey`) is held.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_key_down(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_key_down, key_from_c, key_down
);
input!(
    /// Whether a key went down this frame.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_key_pressed(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_key_pressed, key_from_c, key_pressed
);
input!(
    /// Whether a key went up this frame.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_key_released(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_key_released, key_from_c, key_released
);
input!(
    /// Whether a mouse button (`CrabbyMouseButton`) is held.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_down(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_down, button_from_c, mouse_down
);
input!(
    /// Whether a mouse button went down this frame.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_pressed(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_pressed, button_from_c, mouse_pressed
);
input!(
    /// Whether a mouse button went up this frame.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_released(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_released, button_from_c, mouse_released
);
ctx_call!(
    /// Mouse position in window pixels.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_pos(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_pos(out_pos: *mut CrabbyVec2) |c| out(out_pos, to_c(c.mouse_pos()))
);
ctx_call!(
    /// Mouse position in world units (through the camera).
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_world(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_world(out_pos: *mut CrabbyVec2) |c| out(out_pos, to_c(c.mouse_world()))
);
ctx_call!(
    /// Scroll this frame (lines; positive y = away from the user).
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_mouse_wheel(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_mouse_wheel(out_wheel: *mut CrabbyVec2) |c| out(out_wheel, to_c(c.mouse_wheel()))
);

ctx_call!(
    /// Loads a PNG/JPEG texture (cached by path). Headless runs return a
    /// size-only handle. Errors (missing file) come back as a failed status.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_texture(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_load_texture(path: *const c_char, out_texture: *mut CrabbyTexture) |c| {
        let path = text(path)?;
        if out_texture.is_null() { return Err(invalid("output is null")); }
        out(out_texture, texture_to_c(c.load_texture(path)))
    }
);
ctx_call!(
    /// Makes a texture from `width * height * 4` RGBA bytes.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_texture(std::ptr::null_mut(), 1, 1, std::ptr::null(), 4, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_create_texture(width: u32, height: u32, rgba: *const u8, len: usize, out_texture: *mut CrabbyTexture) |c| {
        let pixels = pixels(width, height, rgba, len)?;
        if out_texture.is_null() { return Err(invalid("output is null")); }
        out(out_texture, texture_to_c(c.create_texture(width, height, pixels)))
    }
);
ctx_call!(
    /// Replaces a texture's pixels (same size, `width * height * 4` bytes).
    /// ```
    /// use crabby_ffi::*;
    /// let t = CrabbyTexture { id: 0, width: 1, height: 1 };
    /// assert_eq!(unsafe { crabby_ctx_update_texture(std::ptr::null_mut(), t, std::ptr::null(), 4) } as u32, 1);
    /// ```
    crabby_ctx_update_texture(texture: CrabbyTexture, rgba: *const u8, len: usize) |c| {
        let pixels = pixels(texture.width, texture.height, rgba, len)?;
        c.update_texture(&texture_from_c(texture), pixels);
        Ok(())
    }
);

fn pixels<'a>(width: u32, height: u32, rgba: *const u8, len: usize) -> Result<&'a [u8], Failure> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if rgba.is_null() || width == 0 || height == 0 || expected != Some(len) {
        return Err(invalid("pixels must be width * height * 4 RGBA bytes"));
    }
    // SAFETY: caller supplies `len` readable bytes.
    Ok(unsafe { std::slice::from_raw_parts(rgba, len) })
}

ctx_call!(
    /// Background color for this frame.
    /// ```
    /// use crabby_ffi::*;
    /// let black = CrabbyColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    /// assert_eq!(unsafe { crabby_ctx_clear(std::ptr::null_mut(), black) } as u32, 1);
    /// ```
    crabby_ctx_clear(color: CrabbyColor) |c| { c.clear(to_c(color)); Ok(()) }
);
ctx_call!(
    /// Draws a texture centered at `pos` (world units) this frame.
    /// ```
    /// use crabby_ffi::*;
    /// let t = CrabbyTexture { id: 0, width: 1, height: 1 };
    /// assert_eq!(unsafe { crabby_ctx_draw(std::ptr::null_mut(), t, CrabbyVec2::default()) } as u32, 1);
    /// ```
    crabby_ctx_draw(texture: CrabbyTexture, pos: CrabbyVec2) |c| {
        c.draw(&texture_from_c(texture), to_c(pos)); Ok(())
    }
);
ctx_call!(
    /// Draws a texture with options (start from `crabby_draw_params_default()`).
    /// ```
    /// use crabby_ffi::*;
    /// let t = CrabbyTexture { id: 0, width: 1, height: 1 };
    /// assert_eq!(unsafe { crabby_ctx_draw_ex(std::ptr::null_mut(), t, CrabbyVec2::default(), std::ptr::null()) } as u32, 1);
    /// ```
    crabby_ctx_draw_ex(texture: CrabbyTexture, pos: CrabbyVec2, params: *const CrabbyDrawParams) |c| {
        if params.is_null() { return Err(invalid("params is null")); }
        c.draw_ex(&texture_from_c(texture), to_c(pos), draw_params_from_c(&*params)); Ok(())
    }
);
ctx_call!(
    /// Draws UTF-8 text with its top-left at `pos`, `size` pixels tall, in window pixels.
    /// ```
    /// use crabby_ffi::*;
    /// let white = CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    /// assert_eq!(unsafe { crabby_ctx_draw_text(std::ptr::null_mut(), c"hi".as_ptr(), CrabbyVec2::default(), 16.0, white) } as u32, 1);
    /// ```
    crabby_ctx_draw_text(string: *const c_char, pos: CrabbyVec2, size: f32, color: CrabbyColor) |c| {
        c.draw_text(text(string)?, to_c(pos), size, to_c(color)); Ok(())
    }
);
ctx_call!(
    /// Size in pixels that `crabby_ctx_draw_text` would cover.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_measure_text(std::ptr::null_mut(), c"hi".as_ptr(), 16.0, std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_measure_text(string: *const c_char, size: f32, out_size: *mut CrabbyVec2) |c| {
        let string = text(string)?;
        if out_size.is_null() { return Err(invalid("output is null")); }
        out(out_size, to_c(c.measure_text(string, size)))
    }
);

/// The camera: which world point is at the window's top-left, and the zoom
/// (window pixels per world unit).
/// ```
/// let cam = crabby_ffi::CrabbyCamera { position: Default::default(), zoom: 1.0 };
/// assert_eq!(cam.zoom, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyCamera {
    /// World point at the top-left corner.
    pub position: CrabbyVec2,
    /// Window pixels per world unit.
    pub zoom: f32,
}

ctx_call!(
    /// Reads the camera.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_camera(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
    /// ```
    crabby_ctx_camera(out_camera: *mut CrabbyCamera) |c| {
        out(out_camera, CrabbyCamera { position: to_c(c.camera.position), zoom: c.camera.zoom })
    }
);
ctx_call!(
    /// Moves or zooms the camera. Zoom must be positive.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_camera(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
    /// ```
    crabby_ctx_set_camera(camera: *const CrabbyCamera) |c| {
        if camera.is_null() { return Err(invalid("camera is null")); }
        let camera = *camera;
        if !(camera.zoom > 0.0 && camera.zoom.is_finite()) { return Err(invalid("zoom must be positive")); }
        c.camera.position = to_c(camera.position);
        c.camera.zoom = camera.zoom;
        Ok(())
    }
);

// ---- commands -----------------------------------------------------------------

macro_rules! command {
    ($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) |$c:ident| $body:expr) => {
        $(#[$doc])*
        /// Works inside `crabby_query` callbacks (not `crabby_query_par`).
        /// Applied after the system returns.
        /// # Safety
        /// `ctx` must be null or live; pointer arguments must be valid.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(ctx: *mut CrabbyCtx $(, $arg: $ty)*) -> CrabbyStatus {
            // SAFETY: forwarded caller contract.
            boundary(|| unsafe { commands(ctx, |$c| $body) })
        }
    };
}

/// Values copied out of C memory for a later spawn or insert.
struct Owned {
    bytes: Vec<u8>,
    /// (component, offset, size) per value.
    parts: Vec<(u32, usize, usize)>,
}
impl Owned {
    unsafe fn copy(values: &[CrabbyValue]) -> Result<Self, Failure> {
        let mut owned = Owned {
            bytes: Vec::new(),
            parts: Vec::with_capacity(values.len()),
        };
        for v in values {
            if v.data.is_null() {
                return Err(invalid("component data is null"));
            }
            let at = owned.bytes.len();
            // SAFETY: caller supplies `size` readable bytes.
            owned
                .bytes
                .extend_from_slice(unsafe { std::slice::from_raw_parts(v.data.cast(), v.size) });
            owned.parts.push((v.component, at, v.size));
        }
        Ok(owned)
    }
    fn values(&self) -> Vec<CrabbyValue> {
        self.parts
            .iter()
            .map(|&(component, at, size)| CrabbyValue {
                component,
                // Non-null even for zero-sized values.
                data: self.bytes.as_ptr().wrapping_add(at).cast(),
                size,
            })
            .collect()
    }
}

fn report(what: &str, failure: Failure) {
    eprintln!("crabby: {what} skipped: {}", failure.1);
}

command!(
    /// Despawns `entity` after this system (stale entities are ignored).
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_despawn_later(std::ptr::null_mut(), 0) } as u32, 1);
    /// ```
    crabby_ctx_despawn_later(entity: u64) |c| { c.despawn(Entity::from_bits(entity)); Ok(()) }
);
command!(
    /// Spawns an entity with copies of `values` after this system. Values
    /// are checked when applied; bad ones skip the spawn with a message on stderr.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_spawn_later(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
    /// ```
    crabby_ctx_spawn_later(values: *const CrabbyValue, count: usize) |c| {
        if values.is_null() && count != 0 { return Err(invalid("values is null")); }
        let owned = if count == 0 { Owned::copy(&[])? } else { Owned::copy(std::slice::from_raw_parts(values, count))? };
        c.add(move |w| {
            // SAFETY: the values point into `owned`, which lives until this returns.
            if let Err(f) = spawn_values(w, &owned.values()) { report("spawn_later", f) }
        });
        Ok(())
    }
);
command!(
    /// Adds or replaces a component of `entity` after this system.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_later(std::ptr::null_mut(), 0, std::ptr::null()) } as u32, 1);
    /// ```
    crabby_ctx_set_later(entity: u64, value: *const CrabbyValue) |c| {
        if value.is_null() { return Err(invalid("value is null")); }
        let owned = Owned::copy(std::slice::from_raw_parts(value, 1))?;
        c.add(move |w| {
            let v = owned.values()[0];
            let applied = crate::value_layout(w, v.component, v.data, v.size)
                // SAFETY: layout validated against the world just now.
                .map(|(id, kind): (_, Kind)| insert_value(w, Entity::from_bits(entity), id, kind, v.data));
            if let Err(f) = applied { report("set_later", f) }
        });
        Ok(())
    }
);
command!(
    /// Removes a component of `entity` after this system.
    /// ```
    /// assert_eq!(unsafe { crabby_ffi::crabby_ctx_remove_later(std::ptr::null_mut(), 0, 0) } as u32, 1);
    /// ```
    crabby_ctx_remove_later(entity: u64, component: u32) |c| {
        c.add(move |w| match crate::component(w, component) {
            Ok((id, _)) => { w.remove_dynamic(Entity::from_bits(entity), id); }
            Err(f) => report("remove_later", f),
        });
        Ok(())
    }
);

// ---- spatial ------------------------------------------------------------------

/// Entities of the built-in grid (`crabby_game_spatial_grid`) within `radius`
/// of `center`. Writes up to `capacity` hits and sets `*out_count` to how many
/// there are in total, so a too-small buffer can be grown and the call repeated.
/// Safe inside query callbacks, also from `crabby_query_par` threads.
/// # Safety
/// Live ctx; `out_hits` writable for `capacity` hits (may be null if 0); writable count.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_spatial_radius(std::ptr::null(), Default::default(), 1.0, std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_radius(
    ctx: *const CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    out_hits: *mut CrabbyGridHit,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { spatial(ctx)? };
        let mut n = 0;
        // SAFETY: forwarded caller contract.
        unsafe {
            collect(out_hits, capacity, out_count, &mut n, |push| {
                grid.for_each_in_radius(to_c(center), radius, |e| push(to_c(*e)))
            })
        }
    })
}

/// Like `crabby_ctx_spatial_radius`, for the rectangle from `min` to `max`.
/// # Safety
/// As `crabby_ctx_spatial_radius`.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_spatial_rect(std::ptr::null(), Default::default(), Default::default(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_rect(
    ctx: *const CrabbyCtx,
    min: CrabbyVec2,
    max: CrabbyVec2,
    out_hits: *mut CrabbyGridHit,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { spatial(ctx)? };
        let mut n = 0;
        // SAFETY: forwarded caller contract.
        unsafe {
            collect(out_hits, capacity, out_count, &mut n, |push| {
                grid.for_each_in_rect(to_c(min), to_c(max), |e| push(to_c(*e)))
            })
        }
    })
}

/// The nearest entity of the built-in grid within `max_radius`, other than
/// `exclude` (pass an entity's own id, or `UINT64_MAX` for none). Returns
/// `CRABBY_NOT_FOUND` if there is none.
/// # Safety
/// Live ctx and writable output.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_spatial_nearest(std::ptr::null(), Default::default(), 1.0, 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_nearest(
    ctx: *const CrabbyCtx,
    center: CrabbyVec2,
    max_radius: f32,
    exclude: u64,
    out_hit: *mut CrabbyGridHit,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded caller contract.
        let grid = unsafe { spatial(ctx)? };
        if out_hit.is_null() {
            return Err(invalid("output is null"));
        }
        let hit = grid
            .nearest(to_c(center), max_radius, |e| e.entity.to_bits() != exclude)
            .ok_or_else(|| error(CrabbyStatus::CRABBY_NOT_FOUND, "nothing in range"))?;
        // SAFETY: checked non-null.
        unsafe { out(out_hit, to_c(hit)) }
    })
}

/// Shared buffer-filling logic of the spatial queries.
pub(crate) unsafe fn collect<T>(
    out_items: *mut T,
    capacity: usize,
    out_count: *mut usize,
    n: &mut usize,
    each: impl FnOnce(&mut dyn FnMut(T)),
) -> Result<(), Failure> {
    if out_count.is_null() || (out_items.is_null() && capacity != 0) {
        return Err(invalid("output buffer or count is null"));
    }
    each(&mut |item| {
        if *n < capacity {
            // SAFETY: caller supplies `capacity` writable items.
            unsafe { out_items.add(*n).write(item) };
        }
        *n += 1;
    });
    // SAFETY: checked non-null.
    unsafe { out_count.write(*n) };
    Ok(())
}
