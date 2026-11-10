//! Scenes and prefabs from C: worlds and entities kept as JSON files under
//! `assets/` (see `Ctx::load_scene` and `Ctx::spawn_prefab` in Rust).
use std::ffi::c_char;

use ponko_ecs::Entity;

use crate::ctx::{CrabbyCtx, ctx_access, out, text};
use crate::types::CrabbyVec2;
use crate::{CrabbyStatus, boundary, invalid_owned};

/// Replaces the world with the scene in `assets/<path>`; entities keep their
/// ids. Components must be reflected. On an error the world is unchanged.
/// In a lockstep session it works from a sim system or setup (every peer
/// runs those); elsewhere (a frame system) it fails, since it would change
/// only this peer.
/// # Safety
/// Live ctx; `path` NUL-terminated.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_scene(std::ptr::null_mut(), c"a.json".as_ptr()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_scene(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| c.load_scene(text(path)?).map_err(invalid_owned)) })
}

/// `crabby_ctx_load_scene`, named for a lockstep game: the same load on
/// every peer at the same tick (`Ctx::load_scene_lockstep`), with the same
/// rules (a sim system or setup in a session, anywhere offline).
/// # Safety
/// Live ctx; `path` NUL-terminated.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_scene_lockstep(std::ptr::null_mut(), c"a.json".as_ptr()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_scene_lockstep(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| c.load_scene_lockstep(text(path)?).map_err(invalid_owned)) })
}

/// Saves the world's reflected components and resources as a scene in
/// `assets/<path>`, over the file if it exists. Not in web builds.
/// # Safety
/// Live ctx; `path` NUL-terminated.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_save_scene(std::ptr::null_mut(), c"a.json".as_ptr()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_scene(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.assets.save_scene(&c.world, text(path)?).map_err(invalid_owned)
        })
    })
}

/// Spawns the prefab in `assets/<path>` with its position at `at`, writing
/// the new entity to `*out_entity`. The file is read once, then kept.
/// # Safety
/// Live ctx; `path` NUL-terminated; `out_entity` writable.
/// ```
/// let mut e = 0;
/// let status = unsafe { ponko_ffi::crabby_ctx_spawn_prefab(std::ptr::null_mut(), c"a.json".as_ptr(), ponko_ffi::CrabbyVec2 { x: 0.0, y: 0.0 }, &mut e) };
/// assert_eq!(status as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_prefab(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    at: CrabbyVec2,
    out_entity: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let e = ctx_access(ctx, |c| {
            c.spawn_prefab(text(path)?, ponko::vec2(at.x, at.y))
                .map_err(invalid_owned)
        })?;
        out(out_entity, e.to_bits())
    })
}

/// Saves `entity`'s reflected components as a prefab in `assets/<path>`
/// (without its position). Not in web builds.
/// # Safety
/// Live ctx; `path` NUL-terminated.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_save_prefab(std::ptr::null_mut(), 0, c"a.json".as_ptr()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_prefab(ctx: *mut CrabbyCtx, entity: u64, path: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.assets
                .save_prefab(&c.world, Entity::from_bits(entity), text(path)?)
                .map_err(invalid_owned)
        })
    })
}
