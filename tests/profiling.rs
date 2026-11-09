//! Profiling: frame statistics, the performance overlay and traces.

use ponko::*;

// ANCHOR: profiling
/// F3 shows the performance overlay; F4 records the next 300 frames as a
/// trace to open in Perfetto. Slow stretches are logged for investigation.
fn perf_keys(ctx: &mut Ctx) {
    if ctx.key_pressed(Key::F3) {
        let on = !ctx.perf.hud();
        ctx.perf.set_hud(on);
    }
    if ctx.key_pressed(Key::F4) {
        ctx.perf.record_trace("trace.json", 300);
    }
    let stats = ctx.perf.frame_stats();
    if stats.frames > 0 && stats.p99_ms > 20.0 && ctx.time.frame().is_multiple_of(600) {
        // Display wait includes pacing; inspect measured CPU/GPU work separately.
        println!("slow: p99 {:.1} ms, gpu {:?} ms", stats.p99_ms, stats.gpu_ms);
        for pass in ctx.perf.gpu_passes() {
            println!("  {}: {:.2} ms", pass.name, pass.ms);
        }
    }
}
// ANCHOR_END: profiling

#[test]
fn headless_runs_have_no_frame_stats() {
    Game::new("t", 1, 1).system(perf_keys).run_headless(10, |ctx| {
        assert_eq!(ctx.perf.frame_stats().frames, 0);
        assert!(ctx.perf.gpu_passes().is_empty());
        ctx.perf.set_hud(true);
        assert!(ctx.perf.hud());
        ctx.perf.record_trace("never-written.json", 10); // no frames: nothing
    });
    assert!(!std::path::Path::new("never-written.json").exists());
}
