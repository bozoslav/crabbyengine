//! Profiling from C: frame-time statistics, the performance overlay and
//! traces (Rust `Ctx::frame_stats`, `set_perf_hud`, `record_trace`).

use std::ffi::c_char;

use crate::ctx::{ctx_access, out, text};
use crate::{CrabbyCtx, CrabbyStatus, boundary};

/// Frame timing over the last few seconds (Rust `FrameStats`), in ms.
/// ```
/// let s = crabby_ffi::CrabbyFrameStats::default();
/// assert_eq!((s.frames, s.gpu_ms), (0, -1.0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyFrameStats {
    /// How many frames these numbers cover (0 before the first frame).
    pub frames: u64,
    /// Frames per second.
    pub fps: f32,
    /// Average frame time.
    pub avg_ms: f32,
    /// Half the frames were faster than this.
    pub p50_ms: f32,
    /// The slowest 5% took at least this.
    pub p95_ms: f32,
    /// The slowest 1% took at least this.
    pub p99_ms: f32,
    /// The slowest frame.
    pub max_ms: f32,
    /// CPU time in systems.
    pub systems_ms: f32,
    /// CPU time drawing, waiting on the display included.
    pub render_cpu_ms: f32,
    /// Of that, waiting for the display (or the GPU) to take a frame.
    pub display_wait_ms: f32,
    /// GPU time per frame; -1 where the GPU can't time itself.
    pub gpu_ms: f32,
}

impl Default for CrabbyFrameStats {
    fn default() -> Self {
        Self {
            frames: 0,
            fps: 0.0,
            avg_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            systems_ms: 0.0,
            render_cpu_ms: 0.0,
            display_wait_ms: 0.0,
            gpu_ms: -1.0,
        }
    }
}

/// Writes frame timing over the last few seconds to `*out_stats` (all zero
/// in headless runs).
/// ```
/// let mut s = crabby_ffi::CrabbyFrameStats::default();
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_frame_stats(std::ptr::null(), &mut s) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `out_stats` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_frame_stats(ctx: *const CrabbyCtx, out_stats: *mut CrabbyFrameStats) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let s = ctx_access(ctx, |c| Ok(c.frame_stats()))?;
        out(
            out_stats,
            CrabbyFrameStats {
                frames: s.frames as u64,
                fps: s.fps,
                avg_ms: s.avg_ms,
                p50_ms: s.p50_ms,
                p95_ms: s.p95_ms,
                p99_ms: s.p99_ms,
                max_ms: s.max_ms,
                systems_ms: s.systems_ms,
                render_cpu_ms: s.render_cpu_ms,
                display_wait_ms: s.display_wait_ms,
                gpu_ms: s.gpu_ms.unwrap_or(-1.0),
            },
        )
    })
}

/// Shows or hides the on-screen performance overlay (fps, frame-time
/// percentiles, CPU and GPU time, the slowest GPU passes).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_perf_hud(std::ptr::null_mut(), true) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_perf_hud(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.set_perf_hud(on);
            Ok(())
        })
    })
}

/// Records the next `frames` frames as a Chrome/Perfetto trace at `path`
/// (open it in <https://ui.perfetto.dev>). Does nothing in headless runs.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_record_trace(std::ptr::null_mut(), c"t.json".as_ptr(), 60) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `path` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_record_trace(ctx: *mut CrabbyCtx, path: *const c_char, frames: u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.record_trace(text(path)?, frames as usize);
            Ok(())
        })
    })
}
