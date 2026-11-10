//! What the GPU can do (Rust `GpuCapabilities`) and the GPU tiers (Rust
//! `GpuTier`): the report as JSON, and plain answers for the tier and each
//! feature.

use std::ffi::c_char;

use ponko::render::GpuCapabilities;
use ponko::render::GpuFeature;
use ponko::render::GpuTier;

use crate::ctx::{ctx_access, out};
use crate::perf::write_text;
use crate::three::{CrabbyGraphicsQuality, quality_to_c};
use crate::{CrabbyCtx, CrabbyStatus, CrabbyStatus::CRABBY_NOT_FOUND, boundary, error, invalid};

/// A class of GPU (Rust `GpuTier`): what every device of the class draws.
/// See `crabby_gpu_tier_report` for the features each may lack.
/// ```
/// assert_eq!(ponko_ffi::CrabbyGpuTier::CRABBY_GPU_TIER_DESKTOP as u32, 2);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGpuTier {
    /// No compute shaders: WebGL2 and anything below baseline.
    CRABBY_GPU_TIER_WEBGL2 = 0,
    /// Compute shaders and storage: every screen-space effect and GPU culling.
    CRABBY_GPU_TIER_BASELINE = 1,
    /// Also GPU skinning, decals and global illumination.
    CRABBY_GPU_TIER_DESKTOP = 2,
}

/// A renderer feature that depends on the GPU (Rust `GpuFeature`), in the
/// report's order.
/// ```
/// assert_eq!(ponko_ffi::CrabbyGpuFeature::CRABBY_GPU_FEATURE_MULTISAMPLING as u32, 12);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGpuFeature {
    /// GPU timestamp queries (per-pass GPU times).
    CRABBY_GPU_FEATURE_TIMESTAMPS = 0,
    /// Compute shaders and storage textures.
    CRABBY_GPU_FEATURE_COMPUTE = 1,
    /// Storage buffers in shaders.
    CRABBY_GPU_FEATURE_STORAGE_BUFFERS = 2,
    /// Static instance batches culled per instance on the GPU.
    CRABBY_GPU_FEATURE_COMPUTE_CULLING = 3,
    /// Skinned and morphed meshes posed on the GPU.
    CRABBY_GPU_FEATURE_GPU_SKINNING = 4,
    /// Hiding models behind others with the previous frame's depth.
    CRABBY_GPU_FEATURE_OCCLUSION_CULLING = 5,
    /// Temporal antialiasing.
    CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING = 6,
    /// Ground-truth ambient occlusion.
    CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION = 7,
    /// Screen-space contact shadows.
    CRABBY_GPU_FEATURE_CONTACT_SHADOWS = 8,
    /// Screen-space reflections.
    CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS = 9,
    /// Probe-based global illumination.
    CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION = 10,
    /// Projected decals.
    CRABBY_GPU_FEATURE_DECALS = 11,
    /// The game's multisampling count for the 3D frame.
    CRABBY_GPU_FEATURE_MULTISAMPLING = 12,
}

fn tier(value: u32) -> Result<GpuTier, crate::Failure> {
    GpuTier::ALL
        .get(value as usize)
        .copied()
        .ok_or_else(|| invalid("tier must be 0 (webgl2), 1 (baseline) or 2 (desktop)"))
}

fn feature(value: u32) -> Result<GpuFeature, crate::Failure> {
    GpuFeature::ALL
        .get(value as usize)
        .copied()
        .ok_or_else(|| invalid("feature must be a CrabbyGpuFeature (0 to 12)"))
}

/// Runs `f` on the context's report: `CRABBY_NOT_FOUND` before a GPU opens
/// (headless runs, before the window).
unsafe fn with_caps<R>(ctx: *const CrabbyCtx, f: impl FnOnce(&GpuCapabilities) -> R) -> Result<R, crate::Failure> {
    // SAFETY: forwarded caller contract.
    unsafe {
        ctx_access(ctx, |c| {
            c.gfx
                .gpu_capabilities()
                .map(f)
                .ok_or_else(|| error(CRABBY_NOT_FOUND, "no GPU is open (headless, or before the window)"))
        })
    }
}

/// Writes `json` to `buffer` like the other text calls: the length to
/// `out_len`, the text NUL-terminated when it fits.
unsafe fn json_out(
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
    json: &str,
) -> Result<(), crate::Failure> {
    if out_len.is_null() {
        return Err(invalid("output is null"));
    }
    if buffer.is_null() && capacity > 0 {
        return Err(invalid("buffer is null"));
    }
    // SAFETY: checked above; forwarded caller contract.
    unsafe { write_text(buffer, capacity, out_len, json) };
    Ok(())
}

/// The GPU report as JSON (Rust `ctx.gfx.gpu_capabilities()`): the adapter, its
/// limits, `tier`, `forced_tier`, and one `{feature, available, fallback,
/// reason}` entry per feature. Writes its length to `out_len` and, when
/// `capacity` is larger, the NUL-terminated text to `buffer` (call with a
/// null buffer and capacity 0 to size it). `CRABBY_NOT_FOUND` before a GPU
/// opens (headless runs, before the window).
/// ```
/// let mut len = 0usize;
/// let s = unsafe { ponko_ffi::crabby_ctx_gpu_capabilities(std::ptr::null(), std::ptr::null_mut(), 0, &mut len) };
/// assert_eq!(s as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `buffer` holds `capacity` bytes (or is null with
/// capacity 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_capabilities(
    ctx: *const CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let json = with_caps(ctx, |caps| serde_json::to_string(caps).expect("the report serializes"))?;
        json_out(buffer, capacity, out_len, &json)
    })
}

/// The GPU's tier (a `CrabbyGpuTier`, Rust `GpuCapabilities::tier`).
/// `CRABBY_NOT_FOUND` before a GPU opens.
/// # Safety
/// `ctx` null or live; `out_tier` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_tier(ctx: *const CrabbyCtx, out_tier: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let t = with_caps(ctx, |caps| caps.tier as u32)?;
        out(out_tier, t)
    })
}

/// Whether the GPU has `feature` (a `CrabbyGpuFeature`; Rust
/// `GpuCapabilities::available`). The fallback and reason are in the JSON
/// report. `CRABBY_NOT_FOUND` before a GPU opens.
/// # Safety
/// `ctx` null or live; `out_available` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_feature(
    ctx: *const CrabbyCtx,
    feature_id: u32,
    out_available: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let f = feature(feature_id)?;
        let available = with_caps(ctx, |caps| caps.available(f))?;
        out(out_available, available)
    })
}

/// The report a device with exactly `tier`'s minimum limits gives, as JSON
/// (Rust `GpuTier::report`): what the tier guarantees, and for the other
/// features the fallback and the reason. Needs no GPU. Text output as in
/// `crabby_ctx_gpu_capabilities`.
/// ```
/// let mut len = 0usize;
/// let s = unsafe { ponko_ffi::crabby_gpu_tier_report(1, std::ptr::null_mut(), 0, &mut len) };
/// assert_eq!((s as u32, len > 100), (0, true));
/// ```
/// # Safety
/// `buffer` holds `capacity` bytes (or is null with capacity 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_gpu_tier_report(
    tier_id: u32,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let report = tier(tier_id)?.report();
        let json = serde_json::to_string(&report).expect("the report serializes");
        json_out(buffer, capacity, out_len, &json)
    })
}

/// Whether every GPU of `tier` has `feature` (Rust
/// `!GpuTier::unavailable_features().contains(..)`). Needs no GPU.
/// ```
/// let mut on = true;
/// let s = unsafe { ponko_ffi::crabby_gpu_tier_feature(0, 6, &mut on) };
/// assert_eq!((s as u32, on), (0, false), "no TAA on WebGL2");
/// ```
/// # Safety
/// `out_available` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_gpu_tier_feature(
    tier_id: u32,
    feature_id: u32,
    out_available: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let (t, f) = (tier(tier_id)?, feature(feature_id)?);
        out(out_available, t.report().available(f))
    })
}

/// The quality preset meant for `tier` (Rust `GraphicsQuality::for_tier`):
/// low with FXAA, medium, or high, asking for nothing the tier lacks.
/// ```
/// let mut q = std::mem::MaybeUninit::<ponko_ffi::CrabbyGraphicsQuality>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_graphics_quality_for_tier(0, q.as_mut_ptr()) } as u32, 0);
/// assert_eq!(unsafe { q.assume_init() }.anti_aliasing, 1, "FXAA");
/// assert_eq!(unsafe { ponko_ffi::crabby_graphics_quality_for_tier(3, q.as_mut_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `out_quality` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_for_tier(
    tier_id: u32,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        out(
            out_quality,
            quality_to_c(ponko::render::GraphicsQuality::for_tier(tier(tier_id)?)),
        )
    })
}

/// The preset for the open GPU, fitted to everything it has (Rust
/// `GraphicsQuality::recommended`). Apply it with
/// `crabby_ctx_set_graphics_quality`. `CRABBY_NOT_FOUND` before a GPU opens.
/// # Safety
/// `ctx` null or live; `out_quality` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_recommended_graphics_quality(
    ctx: *const CrabbyCtx,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let q = with_caps(ctx, ponko::render::GraphicsQuality::recommended)?;
        out(out_quality, quality_to_c(q))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_enums_follow_the_rust_order() {
        use CrabbyGpuFeature::*;
        use CrabbyGpuTier::*;
        let tiers = [
            (CRABBY_GPU_TIER_WEBGL2, GpuTier::WebGl2),
            (CRABBY_GPU_TIER_BASELINE, GpuTier::Baseline),
            (CRABBY_GPU_TIER_DESKTOP, GpuTier::Desktop),
        ];
        assert_eq!(tiers.len(), GpuTier::ALL.len());
        for (c, rust) in tiers {
            assert_eq!(c as u32, rust as u32, "{c:?}");
            assert_eq!(tier(c as u32).ok(), Some(rust), "{c:?}");
        }
        let features = [
            (CRABBY_GPU_FEATURE_TIMESTAMPS, GpuFeature::Timestamps),
            (CRABBY_GPU_FEATURE_COMPUTE, GpuFeature::Compute),
            (CRABBY_GPU_FEATURE_STORAGE_BUFFERS, GpuFeature::StorageBuffers),
            (CRABBY_GPU_FEATURE_COMPUTE_CULLING, GpuFeature::ComputeCulling),
            (CRABBY_GPU_FEATURE_GPU_SKINNING, GpuFeature::GpuSkinning),
            (CRABBY_GPU_FEATURE_OCCLUSION_CULLING, GpuFeature::OcclusionCulling),
            (
                CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING,
                GpuFeature::TemporalAntialiasing,
            ),
            (CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION, GpuFeature::AmbientOcclusion),
            (CRABBY_GPU_FEATURE_CONTACT_SHADOWS, GpuFeature::ContactShadows),
            (
                CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS,
                GpuFeature::ScreenSpaceReflections,
            ),
            (CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION, GpuFeature::GlobalIllumination),
            (CRABBY_GPU_FEATURE_DECALS, GpuFeature::Decals),
            (CRABBY_GPU_FEATURE_MULTISAMPLING, GpuFeature::Multisampling),
        ];
        assert_eq!(features.len(), GpuFeature::ALL.len(), "one C value per Rust feature");
        for (i, (c, rust)) in features.into_iter().enumerate() {
            assert_eq!(c as usize, i, "{c:?} in report order");
            assert_eq!(feature(c as u32).ok(), Some(rust), "{c:?}");
        }
        assert!(feature(features.len() as u32).is_err());
        assert!(tier(tiers.len() as u32).is_err());
    }
}
