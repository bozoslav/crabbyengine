//! Shader compile and hot reload results from C (Rust `Graphics::shader_status`):
//! whether an edit took effect, and where an error is in the game's WGSL.

use std::ffi::c_char;

use ponko::render::ShaderKind;
use ponko::render::ShaderOutcome;
use ponko::render::ShaderStatus;

use crate::ctx::{ctx_access, out};
use crate::perf::write_text;
use crate::{CrabbyCtx, CrabbyStatus, boundary, invalid};

/// `CrabbyShaderStatus::kind`: a material shader (`crabby_ctx_create_shader`).
pub const CRABBY_SHADER_MATERIAL: u32 = 0;
/// A screen effect (`crabby_ctx_create_effect`).
pub const CRABBY_SHADER_EFFECT: u32 = 1;
/// A compute pass (`crabby_ctx_create_compute`).
pub const CRABBY_SHADER_COMPUTE: u32 = 2;

/// `CrabbyShaderStatus::outcome`: made, and in use.
pub const CRABBY_SHADER_COMPILED: u32 = 0;
/// Its file changed and the new code is in use.
pub const CRABBY_SHADER_RELOADED: u32 = 1;
/// It could not be made. With hot reload on, a loaded file that fails
/// still has a handle (drawing with a stand-in) and loads when fixed.
pub const CRABBY_SHADER_FAILED: u32 = 2;
/// Its file changed, but the new code has errors: the last good version
/// stays in use.
pub const CRABBY_SHADER_RELOAD_FAILED: u32 = 3;

/// The latest compile of one of the game's shaders (Rust `ShaderStatus`).
/// Its name: `crabby_ctx_shader_status_name`; its errors as a compiler
/// prints them: `crabby_ctx_shader_status_text`.
/// ```
/// let s = ponko_ffi::CrabbyShaderStatus::default();
/// assert_eq!((s.outcome, s.line), (ponko_ffi::CRABBY_SHADER_COMPILED, 0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyShaderStatus {
    /// `CRABBY_SHADER_MATERIAL`, `CRABBY_SHADER_EFFECT` or `CRABBY_SHADER_COMPUTE`.
    pub kind: u32,
    /// `CRABBY_SHADER_COMPILED`, `_RELOADED`, `_FAILED` or `_RELOAD_FAILED`.
    pub outcome: u32,
    /// Whether the code in the file is what's in use (compiled or reloaded).
    pub ok: bool,
    /// How many edits of its file were reloaded successfully.
    pub reloads: u32,
    /// The frame this happened in (`crabby_ctx_frame`).
    pub frame: u64,
    /// How many errors the last compile had (0 when it worked).
    pub diagnostics: u32,
    /// The first error's 1-based line in the game's WGSL (in the composed
    /// shader if `in_engine_code`); 0 if it has no place.
    pub line: u32,
    /// Its 1-based column; 0 if it has no place.
    pub column: u32,
    /// The first error is in the engine's code around the game's (an
    /// unclosed brace, or a hook with the wrong signature).
    pub in_engine_code: bool,
}

fn status_to_c(s: &ShaderStatus) -> CrabbyShaderStatus {
    let first = s.diagnostics.first();
    CrabbyShaderStatus {
        kind: match s.kind {
            ShaderKind::Material => CRABBY_SHADER_MATERIAL,
            ShaderKind::Effect => CRABBY_SHADER_EFFECT,
            _ => CRABBY_SHADER_COMPUTE,
        },
        outcome: match s.outcome {
            ShaderOutcome::Compiled => CRABBY_SHADER_COMPILED,
            ShaderOutcome::Reloaded => CRABBY_SHADER_RELOADED,
            ShaderOutcome::Failed => CRABBY_SHADER_FAILED,
            _ => CRABBY_SHADER_RELOAD_FAILED,
        },
        ok: s.is_ok(),
        reloads: s.reloads,
        frame: s.frame,
        diagnostics: u32::try_from(s.diagnostics.len()).unwrap_or(u32::MAX),
        line: first.and_then(|d| d.line).unwrap_or(0),
        column: first.and_then(|d| d.column).unwrap_or(0),
        in_engine_code: first.is_some_and(|d| d.in_engine_code),
    }
}

/// How many shaders `crabby_ctx_shader_status` lists: one per name and
/// kind, oldest first.
/// # Safety
/// `ctx` null or live; `out_count` writable.
/// ```
/// let mut n = 0usize;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_shader_status_count(std::ptr::null(), &mut n) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_count(ctx: *const CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_count, c.gfx.shader_status().len())) })
}

/// Runs `f` on shader status `index`.
///
/// # Safety
/// `ctx` null or live.
unsafe fn with_status(
    ctx: *const CrabbyCtx,
    index: usize,
    f: impl FnOnce(&ShaderStatus) -> Result<(), crate::Failure>,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let status = c
                .gfx
                .shader_status()
                .get(index)
                .ok_or_else(|| invalid("index is past crabby_ctx_shader_status_count"))?;
            f(status)
        })
    })
}

/// The latest compile or hot reload result of shader `index` (Rust
/// `Graphics::shader_status()[index]`): check here whether an edit took effect.
/// # Safety
/// `ctx` null or live; `out_status` writable.
/// ```
/// let mut s = ponko_ffi::CrabbyShaderStatus::default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_shader_status(std::ptr::null(), 0, &mut s) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status(
    ctx: *const CrabbyCtx,
    index: usize,
    out_status: *mut CrabbyShaderStatus,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { with_status(ctx, index, |s| out(out_status, status_to_c(s))) }
}

/// Checks a text output's pointers.
pub(crate) fn text_output(buffer: *mut c_char, capacity: usize, out_len: *mut usize) -> Result<(), crate::Failure> {
    if out_len.is_null() {
        return Err(invalid("output is null"));
    }
    if buffer.is_null() && capacity > 0 {
        return Err(invalid("buffer is null"));
    }
    Ok(())
}

/// The name of shader `index`: the path it was loaded from, or the name
/// given to `crabby_ctx_create_*`. Writes into `buffer` as
/// `crabby_ctx_perf_report` does: the length goes to `out_len`, and the
/// text (NUL-terminated) only if it fits.
/// # Safety
/// `ctx` null or live; `buffer` holds `capacity` bytes (or is null with
/// capacity 0); `out_len` writable.
/// ```
/// let mut len = 0usize;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_shader_status_name(std::ptr::null(), 0, std::ptr::null_mut(), 0, &mut len) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_name(
    ctx: *const CrabbyCtx,
    index: usize,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_status(ctx, index, |s| {
            text_output(buffer, capacity, out_len)?;
            write_text(buffer, capacity, out_len, &s.shader);
            Ok(())
        })
    }
}

/// The errors of shader `index`'s last compile as a compiler prints them
/// (`water.wgsl:4:12: error: ...`, the line of code, notes and the GPU
/// variant), one after another; empty when it worked. Writes into
/// `buffer` as `crabby_ctx_shader_status_name` does.
/// # Safety
/// As `crabby_ctx_shader_status_name`.
/// ```
/// let mut len = 0usize;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_shader_status_text(std::ptr::null(), 0, std::ptr::null_mut(), 0, &mut len) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_text(
    ctx: *const CrabbyCtx,
    index: usize,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        with_status(ctx, index, |s| {
            text_output(buffer, capacity, out_len)?;
            let text: Vec<String> = s.diagnostics.iter().map(ToString::to_string).collect();
            write_text(buffer, capacity, out_len, &text.join("\n"));
            Ok(())
        })
    }
}

/// Every shader's status as JSON, the same as the `shaders.status` control
/// method returns: `{"shaders": [{"shader", "kind", "outcome", "ok",
/// "reloads", "frame", "diagnostics": [{"line", "column", "in_engine_code",
/// "message", "label", "notes", "source_line", "variant", "text"}]}]}`.
/// Writes into `buffer` as `crabby_ctx_shader_status_name` does.
/// # Safety
/// `ctx` null or live; `buffer` holds `capacity` bytes (or is null with
/// capacity 0); `out_len` writable.
/// ```
/// let mut len = 0usize;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_shader_status_json(std::ptr::null(), std::ptr::null_mut(), 0, &mut len) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_json(
    ctx: *const CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        text_output(buffer, capacity, out_len)?;
        let json = ctx_access(ctx, |c| Ok(c.gfx.shader_status_json()))?;
        write_text(buffer, capacity, out_len, &json);
        Ok(())
    })
}
