//! Residency budgets and asset streaming (Rust `Ctx::set_residency_budget`,
//! `resident_mesh`, `resident_texture`, `stream_texture`, `stream_mesh`,
//! `asset_state`, `set_upload_budget`, `upload_stats`, `finish_uploads`,
//! and the `StreamedLod` component).

use std::ffi::c_char;

use ponko::{AssetState, Handle, ResidencyKind, ResidencyStats, StreamedLod, UploadStats};

use crate::ctx::{ctx_access, out, text};
use crate::three::CrabbyMesh;
use crate::types::{CrabbyTexture, texture_to_c};
use crate::{CrabbyCtx, CrabbyStatus, boundary, invalid};

/// A kind of asset with its own residency budget (Rust `ResidencyKind`);
/// pass it as a `uint32_t`.
/// ```
/// assert_eq!(ponko_ffi::CrabbyResidencyKind::CRABBY_RESIDENCY_TEXTURE as u32, 1);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyResidencyKind {
    /// Meshes (`crabby_ctx_resident_mesh`, `crabby_ctx_stream_mesh`).
    CRABBY_RESIDENCY_MESH = 0,
    /// Textures (`crabby_ctx_resident_texture`, `crabby_ctx_stream_texture`).
    CRABBY_RESIDENCY_TEXTURE = 1,
}

/// Where a streamed asset is (Rust `AssetState`), from
/// `crabby_ctx_asset_state`.
/// ```
/// assert_eq!(ponko_ffi::CrabbyAssetState::CRABBY_ASSET_FAILED as u32, 6);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAssetState {
    /// Never asked for (or freed since).
    CRABBY_ASSET_UNKNOWN = 0,
    /// Being read and decoded on a worker thread.
    CRABBY_ASSET_LOADING = 1,
    /// Decoded, waiting for room in the upload budget.
    CRABBY_ASSET_UPLOADING = 2,
    /// A texture at less than full resolution, getting sharper.
    CRABBY_ASSET_STREAMING = 3,
    /// Fully loaded.
    CRABBY_ASSET_RESIDENT = 4,
    /// Evicted by its budget: a texture shows a small stand-in, a mesh
    /// draws nothing; asking again brings it back under the same handle.
    CRABBY_ASSET_EVICTED = 5,
    /// Loading failed (`crabby_ctx_asset_error` says why).
    CRABBY_ASSET_FAILED = 6,
}

/// A kind's budget and counters (Rust `ResidencyStats`), from
/// `crabby_ctx_residency`. Bytes are estimates of GPU memory.
/// ```
/// assert!(!ponko_ffi::CrabbyResidencyStats::default().has_budget);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyResidencyStats {
    /// Whether there is a budget; `budget` is its bytes.
    pub has_budget: bool,
    /// The budget's bytes (0 without one).
    pub budget: u64,
    /// Assets resident now.
    pub resident: usize,
    /// Their bytes.
    pub resident_bytes: u64,
    /// Bytes over the budget (assets that can't be evicted now).
    pub over_budget_bytes: u64,
    /// Resident assets never evicted (pinned, or loaded directly too).
    pub pinned: usize,
    /// Their bytes.
    pub pinned_bytes: u64,
    /// Assets loaded for the first time.
    pub loads: u64,
    /// Assets loaded again after eviction or freeing.
    pub reloads: u64,
    /// Assets evicted to fit the budget.
    pub evictions: u64,
    /// Bytes those evictions released.
    pub evicted_bytes: u64,
}

impl From<ResidencyStats> for CrabbyResidencyStats {
    fn from(s: ResidencyStats) -> Self {
        Self {
            has_budget: s.budget.is_some(),
            budget: s.budget.unwrap_or(0),
            resident: s.resident,
            resident_bytes: s.resident_bytes,
            over_budget_bytes: s.over_budget_bytes(),
            pinned: s.pinned,
            pinned_bytes: s.pinned_bytes,
            loads: s.loads,
            reloads: s.reloads,
            evictions: s.evictions,
            evicted_bytes: s.evicted_bytes,
        }
    }
}

/// The upload queue's budget and counters (Rust `UploadStats`), from
/// `crabby_ctx_upload_stats`.
/// ```
/// assert_eq!(ponko_ffi::CrabbyUploadStats::default().pending, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyUploadStats {
    /// Whether there is a per-frame budget; `budget` is its bytes.
    pub has_budget: bool,
    /// The budget's bytes (0 without one).
    pub budget: u64,
    /// Assets decoding or waiting to upload.
    pub pending: usize,
    /// Bytes uploaded in the last upload step.
    pub frame_bytes: u64,
    /// Uploads in the last upload step.
    pub frame_uploads: u32,
    /// Bytes uploaded since the start.
    pub total_bytes: u64,
    /// Uploads since the start.
    pub total_uploads: u64,
    /// Steps over the budget because one upload was larger.
    pub oversized: u64,
    /// Times the game thread waited for a decode.
    pub decode_waits: u64,
    /// Texture uploads held back by the texture budget.
    pub held_by_budget: u64,
    /// Loads that failed.
    pub failures: u64,
    /// Milliseconds the last upload step took.
    pub step_ms: f32,
    /// The longest upload step, in milliseconds.
    pub max_step_ms: f32,
}

impl From<UploadStats> for CrabbyUploadStats {
    fn from(s: UploadStats) -> Self {
        Self {
            has_budget: s.budget.is_some(),
            budget: s.budget.unwrap_or(0),
            pending: s.pending,
            frame_bytes: s.frame_bytes,
            frame_uploads: s.frame_uploads,
            total_bytes: s.total_bytes,
            total_uploads: s.total_uploads,
            oversized: s.oversized,
            decode_waits: s.decode_waits,
            held_by_budget: s.held_by_budget,
            failures: s.failures,
            step_ms: s.step_ms,
            max_step_ms: s.max_step_ms,
        }
    }
}

fn kind_from(kind: u32) -> Result<ResidencyKind, crate::Failure> {
    match kind {
        0 => Ok(ResidencyKind::Mesh),
        1 => Ok(ResidencyKind::Texture),
        _ => Err(invalid("unknown residency kind")),
    }
}

fn state_to_c(state: AssetState) -> u32 {
    (match state {
        AssetState::Loading => CrabbyAssetState::CRABBY_ASSET_LOADING,
        AssetState::Uploading => CrabbyAssetState::CRABBY_ASSET_UPLOADING,
        AssetState::Streaming => CrabbyAssetState::CRABBY_ASSET_STREAMING,
        AssetState::Resident => CrabbyAssetState::CRABBY_ASSET_RESIDENT,
        AssetState::Evicted => CrabbyAssetState::CRABBY_ASSET_EVICTED,
        AssetState::Failed => CrabbyAssetState::CRABBY_ASSET_FAILED,
        _ => CrabbyAssetState::CRABBY_ASSET_UNKNOWN,
    }) as u32
}

/// Sets the byte budget of `kind` (a `CrabbyResidencyKind`); `has_budget`
/// false for no limit (the default). Lowering it evicts at once what it can.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_residency_budget(std::ptr::null_mut(), 0, true, 1) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_residency_budget(
    ctx: *mut CrabbyCtx,
    kind: u32,
    has_budget: bool,
    bytes: u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.set_residency_budget(kind_from(kind)?, has_budget.then_some(bytes));
            Ok(())
        })
    })
}

/// The budget and counters of `kind` (a `CrabbyResidencyKind`).
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_residency(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_residency(
    ctx: *mut CrabbyCtx,
    kind: u32,
    out_stats: *mut CrabbyResidencyStats,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_stats, c.gfx.residency(kind_from(kind)?).into())) })
}

/// Whether the asset of `kind` called `name` is resident.
/// # Safety
/// `ctx` null or live; `name` a NUL-terminated string; output writable.
/// ```
/// let mut r = true;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_is_resident(std::ptr::null_mut(), 0, std::ptr::null(), &mut r) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_is_resident(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    out_resident: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(out_resident, c.gfx.is_resident(kind_from(kind)?, name))
        })
    })
}

/// Where the asset of `kind` called `name` is: a `CrabbyAssetState`.
/// # Safety
/// `ctx` null or live; `name` a NUL-terminated string; output writable.
/// ```
/// let mut s = 0;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_asset_state(std::ptr::null_mut(), 0, std::ptr::null(), &mut s) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_state(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    out_state: *mut u32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(out_state, state_to_c(c.gfx.asset_state(kind_from(kind)?, name)))
        })
    })
}

/// Why loading the asset of `kind` called `name` failed: its message,
/// copied NUL-terminated into `buffer` (`capacity` bytes, cut short if
/// needed); `*out_len` is the full length without the NUL, 0 if it didn't
/// fail.
/// # Safety
/// `ctx` null or live; `name` a NUL-terminated string; `buffer` null (with
/// `capacity` 0) or `capacity` writable bytes; output writable.
/// ```
/// let mut n = 0;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_asset_error(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null_mut(), 0, &mut n) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_error(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            let message = c
                .gfx
                .asset_error(kind_from(kind)?, name)
                .map(ToString::to_string)
                .unwrap_or_default();
            if buffer.is_null() && capacity > 0 {
                return Err(invalid("buffer is null"));
            }
            if capacity > 0 {
                let n = message.len().min(capacity - 1);
                std::ptr::copy_nonoverlapping(message.as_ptr(), buffer.cast::<u8>(), n);
                *buffer.add(n) = 0;
            }
            out(out_len, message.len())
        })
    })
}

/// Texture `path` at full resolution under the texture budget, loaded now
/// if it isn't (Rust `Ctx::try_resident_texture`). Ask each frame you need
/// it; evicted ones show a small stand-in under the same handle.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_resident_texture(std::ptr::null_mut(), std::ptr::null(), false, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_resident_texture(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    pin: bool,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_texture.is_null() {
                return Err(invalid("output is null"));
            }
            let texture = c.try_resident_texture(path).map_err(|e| invalid_error(&e))?;
            if pin {
                c.pin_resident_texture(path);
            }
            out(out_texture, texture_to_c(texture))
        })
    })
}

/// Mesh `path` under the mesh budget, loaded now if it isn't (Rust
/// `Ctx::try_resident_mesh`); with `pin`, never evicted until unpinned.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_resident_mesh(std::ptr::null_mut(), std::ptr::null(), false, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_resident_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    pin: bool,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_mesh.is_null() {
                return Err(invalid("output is null"));
            }
            let mesh = c.try_resident_mesh(path).map_err(|e| invalid_error(&e))?;
            if pin {
                c.pin_resident_mesh(path);
            }
            out(out_mesh, CrabbyMesh { id: mesh.id() })
        })
    })
}

/// Lets a pinned asset of `kind` called `path` be evicted again.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_unpin_resident(std::ptr::null_mut(), 0, std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_unpin_resident(
    ctx: *mut CrabbyCtx,
    kind: u32,
    path: *const c_char,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            match kind_from(kind)? {
                ResidencyKind::Mesh => c.unpin_resident_mesh(path),
                _ => c.unpin_resident_texture(path),
            }
            Ok(())
        })
    })
}

/// Texture `path`, streamed in without waiting (Rust
/// `Ctx::stream_texture`): `*out_ready` false until its stand-in is
/// uploaded; then the same texture each call, getting sharper.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; outputs writable.
/// ```
/// let mut ready = true;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_stream_texture(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut(), &mut ready) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stream_texture(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_texture: *mut CrabbyTexture,
    out_ready: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_texture.is_null() || out_ready.is_null() {
                return Err(invalid("output is null"));
            }
            let texture = c.stream_texture(path);
            if let Some(t) = texture {
                out(out_texture, texture_to_c(t))?;
            }
            out(out_ready, texture.is_some())
        })
    })
}

/// Mesh `path`, streamed in without waiting (Rust `Ctx::stream_mesh`):
/// `*out_ready` false until it is uploaded.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; outputs writable.
/// ```
/// let mut ready = true;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_stream_mesh(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut(), &mut ready) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stream_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_mesh: *mut CrabbyMesh,
    out_ready: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_mesh.is_null() || out_ready.is_null() {
                return Err(invalid("output is null"));
            }
            let mesh = c.stream_mesh(path);
            if let Some(m) = mesh {
                out(out_mesh, CrabbyMesh { id: m.id() })?;
            }
            out(out_ready, mesh.is_some())
        })
    })
}

/// Sets the bytes the upload step may send to the GPU each frame;
/// `has_budget` false for no limit.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_upload_budget(std::ptr::null_mut(), true, 1) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_upload_budget(
    ctx: *mut CrabbyCtx,
    has_budget: bool,
    bytes: u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.set_upload_budget(has_budget.then_some(bytes));
            Ok(())
        })
    })
}

/// With `on`, the upload step waits for decoding, so asset states change
/// in the same frames every run (Rust `Graphics::set_deterministic_uploads`).
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_deterministic_uploads(std::ptr::null_mut(), true) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_deterministic_uploads(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.set_deterministic_uploads(on);
            Ok(())
        })
    })
}

/// The upload queue's budget and counters.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_upload_stats(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_upload_stats(
    ctx: *mut CrabbyCtx,
    out_stats: *mut CrabbyUploadStats,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_stats, c.gfx.upload_stats().into())) })
}

/// Waits until everything streamed so far is loaded (loading screens, tests).
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_finish_uploads(std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_finish_uploads(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.finish_uploads();
            Ok(())
        })
    })
}

/// Gives `entity` (which has a LOD group, `crabby_ctx_set_lod_group`) a
/// Rust `StreamedLod`: file `i` of `files` streams in the mesh of level
/// `i`'s first renderer as the camera needs it.
/// # Safety
/// `ctx` null or live; `files` null (with `count` 0) or `count`
/// NUL-terminated strings.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_streamed_lod(std::ptr::null_mut(), 0, std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_streamed_lod(
    ctx: *mut CrabbyCtx,
    entity: u64,
    files: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if files.is_null() && count > 0 {
                return Err(invalid("files is null"));
            }
            let mut names = Vec::with_capacity(count);
            for i in 0..count {
                names.push(text(*files.add(i))?);
            }
            c.world.insert(entity, StreamedLod::new(&names));
            Ok(())
        })
    })
}

fn invalid_error(e: &ponko::Error) -> crate::Failure {
    crate::invalid_owned(e.to_string())
}
