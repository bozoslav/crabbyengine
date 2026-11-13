//! The `try_` loaders: the C side of Rust's `try_load_*`. A missing or
//! broken file is an ordinary failure with its kind (`ponko_last_error_kind`)
//! and path (`ponko_last_error_path`), never a caught panic.
//!
//! The older `ponko_ctx_load_texture`, `_font`, `_sound`, `_stream` and
//! `_mesh` keep what they always did (they report a missing file as
//! `PONKO_PANIC`); the loaders of shaders, effects, compute passes, clips,
//! skeletons, mesh data and texture arrays already report kinds.
use std::ffi::c_char;

use ponko::prelude::Ctx;

use crate::ctx::{ctx_access, out, text};
use crate::media::{PonkoFont, PonkoSound, font_to_c, sound_to_c};
use crate::three::PonkoMesh;
use crate::types::{PonkoTexture, texture_to_c, to_c};
use crate::{PonkoCtx, PonkoStatus, boundary, engine_error};

/// Loads through `load` and writes the result: the shared body of the
/// `try_` loaders.
///
/// # Safety
/// `ctx` null or live; `path` null or NUL-terminated; `output` null or writable.
unsafe fn try_load<T, C>(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    output: *mut C,
    load: impl FnOnce(&mut Ctx, &str) -> Result<T, ponko::prelude::Error>,
    to_c: impl FnOnce(T) -> C,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if output.is_null() {
                return Err(crate::invalid("output is null"));
            }
            let value = load(c, path).map_err(engine_error)?;
            out(output, to_c(value))
        })
    })
}

/// Loads a texture like `ponko_ctx_load_texture`, but a file that is missing
/// returns `PONKO_NOT_FOUND` and one that doesn't decode
/// `PONKO_INVALID_ARGUMENT`, with `ponko_last_error_kind()` and
/// `ponko_last_error_path()` saying which and where (Rust's
/// `gfx.try_load_texture`). `*out_texture` is unchanged on failure.
/// ```
/// let mut t = ponko_ffi::PonkoTexture { id: 0, width: 0, height: 0 };
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_try_load_texture(std::ptr::null_mut(), c"a.png".as_ptr(), &mut t) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `path` NUL-terminated; `out_texture` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_try_load_texture(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    out_texture: *mut PonkoTexture,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { try_load(ctx, path, out_texture, |c, p| c.gfx.try_load_texture(p), texture_to_c) }
}

/// Loads a font (TTF or OTF) like `ponko_ctx_load_font`, reporting failures
/// as `ponko_ctx_try_load_texture` does (Rust's `gfx.try_load_font`).
/// ```
/// let mut f = ponko_ffi::PonkoFont { id: 0 };
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_try_load_font(std::ptr::null_mut(), c"a.ttf".as_ptr(), &mut f) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `path` NUL-terminated; `out_font` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_try_load_font(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    out_font: *mut PonkoFont,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { try_load(ctx, path, out_font, |c, p| c.gfx.try_load_font(p), font_to_c) }
}

/// Loads a sound into memory like `ponko_ctx_load_sound`, reporting failures
/// as `ponko_ctx_try_load_texture` does (Rust's `audio.try_load`).
/// ```
/// let mut s = ponko_ffi::PonkoSound { id: 0, seconds: 0.0 };
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_try_load_sound(std::ptr::null_mut(), c"a.ogg".as_ptr(), &mut s) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `path` NUL-terminated; `out_sound` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_try_load_sound(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    out_sound: *mut PonkoSound,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { try_load(ctx, path, out_sound, |c, p| c.audio.try_load(p), sound_to_c) }
}

/// Opens a sound for streaming like `ponko_ctx_load_stream`, reporting
/// failures as `ponko_ctx_try_load_texture` does (Rust's
/// `audio.try_load_stream`).
/// ```
/// let mut s = ponko_ffi::PonkoSound { id: 0, seconds: 0.0 };
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_try_load_stream(std::ptr::null_mut(), c"a.ogg".as_ptr(), &mut s) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `path` NUL-terminated; `out_sound` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_try_load_stream(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    out_sound: *mut PonkoSound,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { try_load(ctx, path, out_sound, |c, p| c.audio.try_load_stream(p), sound_to_c) }
}

/// Loads a glTF model's first mesh like `ponko_ctx_load_mesh`, reporting
/// failures as `ponko_ctx_try_load_texture` does (Rust's `gfx.try_load_mesh`).
/// ```
/// let mut m = ponko_ffi::PonkoMesh { id: 0 };
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_try_load_mesh(std::ptr::null_mut(), c"a.glb".as_ptr(), &mut m) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `path` NUL-terminated; `out_mesh` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_try_load_mesh(
    ctx: *mut PonkoCtx,
    path: *const c_char,
    out_mesh: *mut PonkoMesh,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { try_load(ctx, path, out_mesh, |c, p| c.gfx.try_load_mesh(p), to_c) }
}
