//! The context's profiling calls from C: frame statistics, the overlay,
//! traces, turning profiling on, and the performance report. Headless runs:
//! there is no GPU time and no trace is written, but the report counts
//! ticks, systems and scopes.
use ponko_ffi::*;
use std::ffi::{CString, c_char, c_void};
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

const OK: CrabbyStatus = CRABBY_OK;
const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;
const TICKS: u64 = 20;

#[derive(Default)]
struct Run {
    /// The report as the last tick read it.
    report: String,
    stats: Option<CrabbyFrameStats>,
    uploads: Option<CrabbyGpuUploads>,
    /// Whether the buffer rules held.
    checked_buffers: bool,
    trace: Option<CString>,
}

unsafe fn report(ctx: *mut CrabbyCtx) -> String {
    unsafe {
        let mut len = 0;
        assert_eq!(
            crabby_ctx_perf_report(ctx, null_mut(), 0, &mut len),
            OK,
            "the size, asked first"
        );
        let mut buf = vec![1u8; len + 1];
        let mut again = 0;
        assert_eq!(
            crabby_ctx_perf_report(ctx, buf.as_mut_ptr().cast(), buf.len(), &mut again),
            OK
        );
        assert_eq!(again, len);
        assert_eq!(buf[len], 0, "terminated");
        String::from_utf8(buf[..len].to_vec()).unwrap()
    }
}

unsafe extern "C" fn work(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let scope = crabby_profile_scope(c"perf_test/inner".as_ptr());
        let token = crabby_profile_begin(scope);
        std::hint::black_box((0..2000).map(|i| (i as f32).sqrt()).sum::<f32>());
        crabby_profile_end(scope, token);
        if crabby_ctx_sim_tick(ctx) != TICKS - 1 {
            return;
        }
        run.report = report(ctx);

        let mut stats = CrabbyFrameStats::default();
        assert_eq!(crabby_ctx_frame_stats(ctx, &mut stats), OK);
        run.stats = Some(stats);
        assert_eq!(crabby_ctx_frame_stats(ctx, null_mut()), BAD);
        assert_eq!(crabby_ctx_frame_stats(null(), &mut stats), BAD);

        let mut uploads = CrabbyGpuUploads {
            bytes: 7,
            reallocations: 7,
            ..Default::default()
        };
        assert_eq!(crabby_ctx_gpu_uploads(ctx, &mut uploads), OK);
        run.uploads = Some(uploads);
        assert_eq!(crabby_ctx_gpu_uploads(ctx, null_mut()), BAD);
        assert_eq!(crabby_ctx_gpu_uploads(null(), &mut uploads), BAD);

        // A buffer that is too small gets an empty string and the length it needs.
        let mut len = 0;
        let mut small = [1 as c_char; 4];
        assert_eq!(crabby_ctx_perf_report(ctx, small.as_mut_ptr(), 4, &mut len), OK);
        assert_eq!(small[0], 0);
        assert_eq!(len, run.report.len());
        assert!(len > 4);
        assert_eq!(crabby_ctx_perf_report(ctx, null_mut(), 0, null_mut()), BAD);
        assert_eq!(
            crabby_ctx_perf_report(ctx, null_mut(), 8, &mut len),
            BAD,
            "room claimed, none given"
        );
        assert_eq!(crabby_ctx_perf_report(null(), null_mut(), 0, &mut len), BAD);

        // Traces are a window feature: headless, the call succeeds and writes nothing.
        let path = run.trace.as_ref().unwrap();
        assert_eq!(crabby_ctx_record_trace(ctx, path.as_ptr(), 10), OK);
        assert_eq!(
            crabby_ctx_record_trace(ctx, path.as_ptr(), 0),
            OK,
            "at least one frame is recorded"
        );
        assert_eq!(crabby_ctx_record_trace(ctx, null(), 10), BAD);
        assert_eq!(crabby_ctx_record_trace(null_mut(), path.as_ptr(), 10), BAD);
        run.checked_buffers = true;
    }
}

/// Profiling switches a process-wide scope registry, so the runs of this file take turns.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs a game whose setup does `prepare`, and returns what the last tick saw.
fn run(prepare: unsafe extern "C" fn(*mut CrabbyCtx, *mut c_void)) -> Run {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let file = std::env::temp_dir().join("ponko-ffi-perf-trace.json");
    let _ = std::fs::remove_file(&file);
    let mut run = Run {
        trace: Some(CString::new(file.to_str().unwrap()).unwrap()),
        ..Run::default()
    };
    unsafe {
        let g = crabby_game_new(c"perf".as_ptr(), 1, 1);
        let user = (&raw mut run).cast();
        assert_eq!(crabby_game_sim_system(g, c"work".as_ptr(), 1, Some(work), user), OK);
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, TICKS, Some(prepare), user, &mut world), OK);
        crabby_world_free(world);
    }
    assert!(run.checked_buffers);
    assert!(!file.exists(), "headless runs write no trace");
    run
}

unsafe extern "C" fn profiling_on(ctx: *mut CrabbyCtx, _user: *mut c_void) {
    unsafe {
        assert_eq!(crabby_ctx_set_profiling(ctx, true), OK);
        assert_eq!(crabby_ctx_set_profiling(null_mut(), true), BAD);
    }
}

unsafe extern "C" fn profiling_untouched(_ctx: *mut CrabbyCtx, _user: *mut c_void) {}

unsafe extern "C" fn profiling_toggled(ctx: *mut CrabbyCtx, _user: *mut c_void) {
    unsafe {
        assert_eq!(crabby_ctx_set_profiling(ctx, true), OK);
        assert_eq!(crabby_ctx_set_profiling(ctx, false), OK);
    }
}

unsafe extern "C" fn overlay_on(ctx: *mut CrabbyCtx, _user: *mut c_void) {
    unsafe {
        assert_eq!(crabby_ctx_set_perf_hud(ctx, true), OK);
        assert_eq!(crabby_ctx_set_perf_hud(null_mut(), true), BAD);
    }
}

unsafe extern "C" fn overlay_toggled(ctx: *mut CrabbyCtx, _user: *mut c_void) {
    unsafe {
        assert_eq!(crabby_ctx_set_perf_hud(ctx, true), OK);
        assert_eq!(crabby_ctx_set_perf_hud(ctx, false), OK);
    }
}

fn json(run: &Run) -> serde_json::Value {
    serde_json::from_str(&run.report).unwrap_or_else(|e| panic!("{e}: {}", run.report))
}

#[test]
fn a_profiled_run_reports_its_ticks_systems_and_scopes() {
    let run = run(profiling_on);
    let report = json(&run);
    assert!(report["frames"].as_u64().unwrap() >= TICKS - 1, "{report}");
    let systems = report["systems"].as_array().unwrap();
    assert!(systems.iter().any(|s| s["name"] == "work"), "{systems:?}");
    let scopes = report["scopes"].as_array().unwrap();
    assert!(scopes.iter().any(|s| s["name"] == "perf_test/inner"), "{scopes:?}");
    // The report text from the same JSON, through the report functions.
    let text = CString::new(run.report.clone()).unwrap();
    let (mut len, mut buf) = (0, vec![0u8; 8192]);
    unsafe {
        assert_eq!(
            crabby_perf_report_text(text.as_ptr(), buf.as_mut_ptr().cast(), buf.len(), &mut len),
            OK
        );
    }
    let text = String::from_utf8(buf[..len].to_vec()).unwrap();
    assert!(text.contains("perf_test/inner"), "{text}");
}

#[test]
fn headless_frame_stats_have_no_gpu_time() {
    let run = run(profiling_on);
    let stats = run.stats.expect("read at the last tick");
    assert_eq!(stats.gpu_ms, -1.0, "no GPU, so no GPU time");
    assert!(stats.avg_ms >= 0.0 && stats.max_ms >= stats.p50_ms);
}

#[test]
fn headless_runs_upload_nothing_to_the_gpu() {
    let run = run(profiling_on);
    let uploads = run.uploads.expect("read at the last tick");
    assert_eq!(
        uploads,
        CrabbyGpuUploads::default(),
        "no renderer, so nothing uploaded or made"
    );
}

#[test]
fn gpu_upload_counts_mirror_the_rust_ones_field_for_field() {
    // `GpuUploads` is `#[non_exhaustive]`: set every field there is now.
    let mut rust = ponko::GpuUploads::default();
    rust.instances = 1;
    rust.sprites = 2;
    rust.static_batches = 3;
    rust.uniforms = 4;
    rust.lights = 5;
    rust.materials = 6;
    rust.skins = 7;
    rust.meshes = 8;
    rust.textures = 9;
    rust.other = 10;
    rust.buffers_created = 11;
    rust.reallocations = 12;
    rust.buffer_bytes_created = 13;
    let c = CrabbyGpuUploads::from(rust);
    assert_eq!(c.bytes, rust.bytes());
    assert_eq!(
        [c.instances, c.sprites, c.static_batches, c.uniforms, c.lights],
        [1, 2, 3, 4, 5]
    );
    assert_eq!([c.materials, c.skins, c.meshes, c.textures, c.other], [6, 7, 8, 9, 10]);
    assert_eq!(
        (c.buffers_created, c.reallocations, c.buffer_bytes_created),
        (11, 12, 13)
    );
}

#[test]
fn without_profiling_the_report_is_empty() {
    for prepare in [profiling_untouched, profiling_toggled, overlay_toggled] {
        let run = run(prepare);
        let report = json(&run);
        assert_eq!(report["frames"], 0, "profiling is off: {report}");
        assert!(report["scopes"].as_array().unwrap().is_empty(), "{report}");
    }
}

#[test]
fn the_overlay_switches_profiling_on() {
    let run = run(overlay_on);
    let report = json(&run);
    assert!(report["frames"].as_u64().unwrap() > 0, "{report}");
}
