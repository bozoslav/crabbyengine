//! Declared system access for C and C++ systems: what a system reads and
//! writes, checked in debug builds and listed in the access report.
use crate::game::{CrabbyGame, CrabbySystemFn, build, ok, phase, system, system_name};
use crate::perf::write_text;
use crate::{CrabbyStatus, Failure, boundary, invalid};
use ponko::Phase;
use ponko::access::Service;
use ponko::access::SystemAccess;
use std::ffi::{CStr, c_char, c_void};

/// An owned access declaration: what a system reads and writes. Build it
/// with the `crabby_access_*` calls, register a system with
/// `crabby_game_add_system_with_access` (which copies it), free it with
/// `crabby_access_free`.
///
/// Meant for registration, not per frame: each access call
/// copies the whole declaration, and each name it is given is kept for the
/// rest of the program.
/// ```
/// let a = ponko_ffi::crabby_access_new();
/// unsafe { ponko_ffi::crabby_access_free(a); }
/// ```
pub struct CrabbyAccess(SystemAccess);

/// Engine services a declaration can name (`crabby_access_service`). Two
/// systems conflict on a service when both use it and one writes it.
/// ```
/// assert_eq!(ponko_ffi::CrabbyService::CRABBY_SERVICE_NET as u32, 12);
/// ```
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum CrabbyService {
    /// Keys, mouse, gamepads and actions.
    CRABBY_SERVICE_INPUT = 0,
    /// Clocks: dt, ticks, pause and speed.
    CRABBY_SERVICE_TIME = 1,
    /// The system's random stream.
    CRABBY_SERVICE_RANDOM = 2,
    /// Drawing, textures, meshes and render settings.
    CRABBY_SERVICE_GFX = 3,
    /// Sounds, buses and the listener.
    CRABBY_SERVICE_AUDIO = 4,
    /// Hot reload, data files, prefabs and scenes.
    CRABBY_SERVICE_ASSETS = 5,
    /// The retained UI and input capture.
    CRABBY_SERVICE_UI = 6,
    /// Profiling, statistics, the HUD and traces.
    CRABBY_SERVICE_PERF = 7,
    /// The 2D and 3D cameras.
    CRABBY_SERVICE_CAMERA = 8,
    /// The spatial grid.
    CRABBY_SERVICE_SPATIAL = 9,
    /// The 2D physics world.
    CRABBY_SERVICE_PHYSICS = 10,
    /// The 3D physics world.
    CRABBY_SERVICE_PHYSICS3D = 11,
    /// Lockstep networking.
    CRABBY_SERVICE_NET = 12,
}

unsafe fn text<'a>(ptr: *const c_char) -> Result<&'a str, Failure> {
    if ptr.is_null() {
        return Err(invalid("name is null"));
    }
    // SAFETY: caller supplies a NUL-terminated string, alive for the call.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| invalid("name is not UTF-8"))
}

/// Applies a builder step to a live declaration.
unsafe fn edit(
    access: *mut CrabbyAccess,
    f: impl FnOnce(SystemAccess) -> Result<SystemAccess, Failure>,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies a live, exclusively accessed handle or null.
        let a = unsafe { access.as_mut() }.ok_or_else(|| invalid("access is null"))?;
        let next = f(a.0.clone())?;
        a.0 = next;
        Ok(())
    })
}

/// A declaration of nothing. Free once with `crabby_access_free`.
/// ```
/// let a = ponko_ffi::crabby_access_new();
/// assert!(!a.is_null());
/// unsafe { ponko_ffi::crabby_access_free(a); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_access_new() -> *mut CrabbyAccess {
    let mut result = std::ptr::null_mut();
    boundary(|| {
        result = Box::into_raw(Box::new(CrabbyAccess(SystemAccess::new())));
        Ok(())
    });
    result
}

/// Frees a declaration; null is ignored.
/// ```
/// unsafe { ponko_ffi::crabby_access_free(std::ptr::null_mut()); }
/// ```
/// # Safety
/// Null, or an owned, live handle not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_free(access: *mut CrabbyAccess) {
    if !access.is_null() {
        // SAFETY: caller transfers the owned handle exactly once.
        drop(unsafe { Box::from_raw(access) });
    }
}

/// Declares reading the component registered under `name`
/// (`crabby_world_register_component`'s name). Debug builds can't tell a
/// C read from a write, so declare a write when the system changes it.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_read_component(std::ptr::null_mut(), c"pos".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_component(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.read_named(text(name)?))) }
}

/// Declares reading and writing the component registered under `name`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_write_component(std::ptr::null_mut(), c"pos".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_write_component(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.write_named(text(name)?))) }
}

/// Declares reading the resource stored under `name` (`crabby_world_set_resource`).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_read_resource(std::ptr::null_mut(), c"score".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_resource(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.read_resource_named(text(name)?))) }
}

/// Declares reading and writing the resource stored under `name`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_write_resource(std::ptr::null_mut(), c"score".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_write_resource(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.write_resource_named(text(name)?))) }
}

/// Declares reading events whose type is the component registered under
/// `name` (events are sent by that component's id; ids differ per world,
/// names don't).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_read_events(std::ptr::null_mut(), c"hit".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_events(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.read_events_named(text(name)?))) }
}

/// Declares sending events whose type is the component registered under
/// `name` (`crabby_ctx_send_event`). Sending doesn't cover reading.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_send_events(std::ptr::null_mut(), c"hit".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// Live handle (or null); `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_send_events(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.send_events_named(text(name)?))) }
}

/// Declares recording commands (`crabby_ctx_*_later`).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_commands(std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// Live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_commands(access: *mut CrabbyAccess) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { edit(access, |a| Ok(a.commands())) }
}

/// Declares using `service` (a `CrabbyService`), reading or also writing it.
/// An unknown service is `CRABBY_INVALID_ARGUMENT`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_access_service(std::ptr::null_mut(), 0, false) } as u32, 1);
/// ```
/// # Safety
/// Live handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_service(access: *mut CrabbyAccess, service: u32, write: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        edit(access, |a| {
            let s = *Service::ALL
                .get(service as usize)
                .ok_or_else(|| invalid("unknown service"))?;
            Ok(if write { a.write_service(s) } else { a.read_service(s) })
        })
    }
}

/// Counts why two declared systems can't share a parallel batch: with
/// `ordered`, by the finer rules for `earlier` registered before `later`;
/// otherwise in either order. 0 means they could.
/// ```
/// unsafe {
///     let a = ponko_ffi::crabby_access_new();
///     let mut n = 9;
///     assert_eq!(ponko_ffi::crabby_access_conflicts(a, a, false, &mut n), ponko_ffi::CrabbyStatus::CRABBY_OK);
///     assert_eq!(n, 0);
///     ponko_ffi::crabby_access_free(a);
/// }
/// ```
/// # Safety
/// Live handles (or null); `out_count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_conflicts(
    earlier: *const CrabbyAccess,
    later: *const CrabbyAccess,
    ordered: bool,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies live handles or null.
        let (a, b) = unsafe { (earlier.as_ref(), later.as_ref()) };
        let (a, b) = a.zip(b).ok_or_else(|| invalid("access is null"))?;
        if out_count.is_null() {
            return Err(invalid("out_count is null"));
        }
        let n = if ordered {
            a.0.conflicts_before(&b.0).len()
        } else {
            a.0.conflicts_with(&b.0).len()
        };
        // SAFETY: checked non-null; caller supplies writable storage.
        unsafe { out_count.write(n) };
        Ok(())
    })
}

/// Like `crabby_game_add_system`, with a declaration of what the system
/// reads and writes (copied; free yours when done). Debug builds check
/// each run against it and abort the game with an error naming what was
/// used without being declared. C systems stay sequential: the
/// declaration is for tools and the access report.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_game_add_system_with_access(std::ptr::null_mut(), 3, std::ptr::null(), 1, std::ptr::null(), None, std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// Live game; live `access`; `system` and `user` valid while the game runs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_add_system_with_access(
    game: *mut CrabbyGame,
    phase_raw: u32,
    name: *const c_char,
    every: u32,
    access: *const CrabbyAccess,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        let checked = (|| {
            let f = system_fn.ok_or_else(|| invalid("system is null"))?;
            let a = access.as_ref().ok_or_else(|| invalid("access is null"))?;
            let p: Phase = phase(phase_raw)?;
            if every == 0 {
                return Err(invalid("every must be at least 1"));
            }
            if every > 1 && p > Phase::SimLast {
                return Err(invalid("only fixed-tick phases take an interval"));
            }
            Ok((f, p, system_name(name, "C system")?, a.0.clone()))
        })();
        build(game, |g| match checked {
            Ok((f, p, name, a)) => ok(g.named_add_with_access(p, name, every, a, system(f, user))),
            Err(e) => (g, Err(e)),
        })
    }
}

/// Writes the game's access report: one line per pair of declared systems
/// in the same phase, saying whether they could run together. Copies it
/// into `buffer` (NUL-terminated if it fits) and its length to `out_len`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_game_access_report(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// Live game; `buffer` holds `capacity` bytes (or null with 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_access_report(
    game: *mut CrabbyGame,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    if out_len.is_null() || (buffer.is_null() && capacity > 0) {
        return boundary(|| Err(invalid("buffer or out_len is null")));
    }
    let mut report = String::new();
    // SAFETY: forwarded caller contract.
    let status = unsafe {
        build(game, |g| {
            report = g.access_report().to_string();
            ok(g)
        })
    };
    if status == CrabbyStatus::CRABBY_OK {
        // SAFETY: checked above; caller supplies the buffer.
        unsafe { write_text(buffer, capacity, out_len, &report) };
    }
    status
}
