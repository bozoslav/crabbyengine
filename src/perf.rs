//! [`Perf`], the performance service at `ctx.perf`: where a frame's time
//! and allocations go.
//!
//! **What lives here:** each system's time ([`Perf::system_timings`]), the
//! last frame's drawing cost ([`Perf::render_stats`], [`Perf::gpu_passes`]),
//! frame-time statistics ([`Perf::frame_stats`]), the on-screen HUD
//! ([`Perf::set_hud`]), Perfetto traces ([`Perf::record_trace`]), the deeper
//! profiling switch ([`Perf::set_profiling`]) with named scopes
//! ([`Perf::scope`], [`Perf::scope_timings`]) and the report that sums it all
//! up ([`Perf::report`]).
//!
//! **When to call it:** reads are valid in any system and describe the
//! frame (or headless tick) before the current one. Switches (`set_hud`,
//! `set_profiling`, `record_trace`) take effect from the next frame; flip
//! them from a frame system or setup.
//!
//! **What it needs:** system timings, scopes and the report work headless.
//! Render statistics, GPU passes, frame statistics, the HUD and traces need
//! a window (feature `window`): without one they are zero, empty or do
//! nothing. GPU pass times also need a GPU with timestamp queries.

use std::path::PathBuf;

use crate::render::RenderStats;

/// How long one system took. See [`Perf::system_timings`].
#[derive(Debug, Clone, PartialEq)]
pub struct SystemTiming {
    /// The system's function name, e.g. `my_game::movement`.
    pub name: &'static str,
    /// `true` for simulation systems (they may run several times per frame).
    pub sim: bool,
    /// Time spent in the system during the last frame (all fixed steps
    /// together), in milliseconds.
    pub last_ms: f32,
    /// Smoothed average of `last_ms`.
    pub avg_ms: f32,
}

/// Timings, statistics and profiling switches: `ctx.perf`.
///
/// - **What lives here:** each system's time, the last frame's drawing cost
///   and GPU passes, frame-time statistics, the on-screen HUD, Perfetto
///   traces, the profiling switch with named scopes, and the report that
///   sums it all up.
/// - **When to call it:** reads work in any system and describe the frame
///   (or headless tick) before the current one. Switches (`set_hud`,
///   `set_profiling`, `record_trace`) apply from the next frame.
/// - **What it needs:** system timings, scopes and the report work
///   headless. Render statistics, GPU passes, frame statistics, the HUD and
///   traces need a window; without one they are zero, empty or do nothing.
///
/// ```
/// use ponko::*;
///
/// fn report_slow_systems(ctx: &mut Ctx) {
///     for t in ctx.perf.system_timings() {
///         if t.avg_ms > 1.0 {
///             println!("{}: {:.2} ms", t.name, t.avg_ms);
///         }
///     }
/// }
/// Game::new("t", 1, 1).sim_system(report_slow_systems).run_headless(2, |_| {});
/// ```
pub struct Perf {
    pub(crate) timings: Vec<SystemTiming>,
    /// Each timed system's place in the schedule, parallel to `timings`.
    pub(crate) schedule: Vec<crate::diagnostics::ScheduleSlot>,
    pub(crate) last_render_stats: RenderStats,
    /// The renderer's extra numbers for the last drawn frame, `None`
    /// before the first frame and without a renderer.
    pub(crate) last_render_detail: Option<crate::render::RenderDetail>,
    /// Frame statistics, the HUD, traces and benchmark runs.
    pub(crate) profiler: crate::profiler::Profiler,
    /// Scopes, allocation counts and the history behind `report`.
    pub(crate) state: crate::perf_report::PerfState,
    /// The renderer's per-pass GPU times, copied after every frame.
    pub(crate) gpu_passes: Vec<crate::GpuPass>,
    /// How long the last frame waited for the display, copied after every frame.
    pub(crate) display_wait_ms: f32,
    /// No window or GPU: there are no frames to trace or draw statistics for.
    pub(crate) headless: bool,
    /// Entities alive when the last system finished, for the report.
    pub(crate) entities: usize,
}

impl Perf {
    pub(crate) fn new() -> Self {
        Self {
            timings: Vec::new(),
            schedule: Vec::new(),
            last_render_stats: RenderStats::default(),
            last_render_detail: None,
            profiler: crate::profiler::Profiler::new(),
            state: crate::perf_report::PerfState::new(),
            gpu_passes: Vec::new(),
            display_wait_ms: 0.0,
            headless: false,
            entities: 0,
        }
    }

    /// How long each system took, in the order they run (simulation systems
    /// first). For finding what's slow; the inspector will graph these.
    ///
    /// ```
    /// # use ponko::*;
    /// fn print_slow_systems(ctx: &mut Ctx) {
    ///     for t in ctx.perf.system_timings() {
    ///         if t.avg_ms > 1.0 {
    ///             println!("{}: {:.2} ms", t.name, t.avg_ms);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn system_timings(&self) -> &[SystemTiming] {
        &self.timings
    }

    /// What drawing the previous frame cost: sprites, draw calls, CPU time
    /// and GPU time. All zero (and `gpu_ms` is `None`) in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn overlay(ctx: &mut Ctx) {
    ///     let stats = ctx.perf.render_stats();
    ///     if let Some(gpu) = stats.gpu_ms {
    ///         println!("{} sprites: {:.2} ms CPU, {gpu:.2} ms GPU", stats.sprites, stats.cpu_ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(overlay).run_headless(1, |_| {});
    /// ```
    pub fn render_stats(&self) -> RenderStats {
        self.last_render_stats
    }

    /// Frame-time statistics over the last few seconds: frames per second,
    /// average and percentile frame times, and where the time went (systems,
    /// drawing on the CPU, waiting for the display, the GPU). Refreshed four
    /// times a second. All zero in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn report(ctx: &mut Ctx) {
    ///     let s = ctx.perf.frame_stats();
    ///     if s.p99_ms > 2.0 * s.p50_ms {
    ///         println!("stutter: p99 {:.1} ms vs median {:.1} ms", s.p99_ms, s.p50_ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(report).run_headless(1, |_| {});
    /// ```
    pub fn frame_stats(&self) -> crate::FrameStats {
        self.profiler.stats()
    }

    /// The GPU time of each pass of the last frames (shadow cascades, depth
    /// prepass, opaque, transparent, bloom, post, ...), smoothed, in the
    /// order they run. Empty in headless runs and on GPUs without timestamp
    /// queries.
    ///
    /// ```
    /// # use ponko::*;
    /// fn slowest_pass(ctx: &mut Ctx) {
    ///     if let Some(p) = ctx.perf.gpu_passes().iter().max_by(|a, b| a.ms.total_cmp(&b.ms)) {
    ///         println!("slowest GPU pass: {} ({:.2} ms)", p.name, p.ms);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(slowest_pass).run_headless(1, |_| {});
    /// ```
    pub fn gpu_passes(&self) -> &[crate::GpuPass] {
        &self.gpu_passes
    }

    /// How long the last frame waited for the display (ms): idle time, not
    /// work. For benchmarks that subtract it from the frame time.
    #[doc(hidden)]
    #[must_use]
    pub fn display_wait_ms(&self) -> f32 {
        self.display_wait_ms
    }

    /// Shows or hides the performance HUD: frame times, CPU and GPU time,
    /// the slowest GPU passes and systems, drawn in the top-left corner.
    /// `PONKO_HUD=1` shows it from the start.
    ///
    /// ```
    /// # use ponko::*;
    /// fn toggle_hud(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F3) {
    ///         let on = ctx.perf.hud();
    ///         ctx.perf.set_hud(!on);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(toggle_hud).run_headless(1, |_| {});
    /// ```
    pub fn set_hud(&mut self, on: bool) {
        self.profiler.hud = on;
        self.sync_profiling();
    }

    /// Whether the performance HUD is showing.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let on = ctx.perf.hud();
    /// # }
    /// ```
    pub fn hud(&self) -> bool {
        self.profiler.hud
    }

    /// Records the next `frames` frames as a Chrome trace at `path`: each
    /// frame's systems, drawing and display wait on one track, and the GPU
    /// passes on another. Open it in <https://ui.perfetto.dev>. Does nothing
    /// in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// fn capture(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F4) {
    ///         ctx.perf.record_trace("frames.json", 120);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(capture).run_headless(1, |_| {});
    /// ```
    pub fn record_trace(&mut self, path: impl Into<PathBuf>, frames: usize) {
        if !self.headless {
            self.profiler.start_trace(path.into(), frames.max(1));
        }
    }

    /// Turns on the deeper profiling: your [`profile_scope!`](crate::profile_scope)
    /// scopes and [`Perf::scope`] calls are timed, allocations are counted
    /// (if [`CountingAllocator`](crate::CountingAllocator) is installed), and
    /// [`report`](Self::report) has numbers. The overlay
    /// ([`set_hud`](Self::set_hud)) and `PONKO_BENCH` switch it on by themselves. While
    /// it is off, scopes cost one atomic load.
    ///
    /// ```
    /// # use ponko::*;
    /// fn debug_keys(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F5) {
    ///         let on = !ctx.perf.profiling();
    ///         ctx.perf.set_profiling(on);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(debug_keys).run_headless(1, |_| {});
    /// ```
    pub fn set_profiling(&mut self, on: bool) {
        self.state.explicit = on;
        self.sync_profiling();
    }

    /// Whether scopes are being timed and allocations counted right now.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// if ctx.perf.profiling() { /* build an expensive debug string */ }
    /// # }
    /// ```
    pub fn profiling(&self) -> bool {
        self.state.on
    }

    /// Times the code until the returned guard is dropped, as a scope called
    /// `name` (see [`profile_scope!`](crate::profile_scope), which is cheaper
    /// in hot code: this looks the name up each time). Times add up across
    /// calls. The guard borrows nothing, so the code it times is free to use
    /// all of `ctx`.
    ///
    /// ```
    /// # use ponko::*;
    /// fn ai(ctx: &mut Ctx) {
    ///     let _timing = ctx.perf.scope("ai/pathing");
    ///     for _t in ctx.world.query::<&mut Transform2D>().iter() { /* ... */ }
    /// }
    /// # Game::new("t", 1, 1).sim_system(ai).run_headless(1, |_| {});
    /// ```
    pub fn scope(&self, name: &str) -> crate::ScopeGuard {
        if !crate::scope::active() {
            return crate::ScopeGuard::off();
        }
        crate::scope_named(name).enter()
    }

    /// Each profiling scope's time in the last frame, in the order they
    /// first ran. Empty while profiling is off.
    ///
    /// ```
    /// # use ponko::*;
    /// fn report(ctx: &mut Ctx) {
    ///     for s in ctx.perf.scope_timings() {
    ///         println!("{}: {:.2} ms in {} calls", s.name, s.last_ms, s.calls);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(report).run_headless(1, |_| {});
    /// ```
    pub fn scope_timings(&self) -> &[crate::ScopeTiming] {
        &self.state.scopes.timings
    }

    /// Where time and allocations went over the last frames (about half a
    /// minute in a window; every tick of a [`Game::profile_headless`](crate::Game::profile_headless) run):
    /// frame-time percentiles, systems, scopes, allocations, the slowest
    /// frames and what was in them, drawing stats, and warnings with advice.
    /// `println!("{report}")` prints it. Empty until profiling is on (see
    /// [`set_profiling`](Self::set_profiling)).
    ///
    /// Its `entities` count is from the last time commands were applied
    /// (after each system and at the end of the frame), so entities the
    /// running system spawned straight into `ctx.world` aren't in it yet.
    ///
    /// ```
    /// # use ponko::*;
    /// fn dump(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F6) {
    ///         println!("{}", ctx.perf.report());
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(dump).run_headless(1, |_| {});
    /// ```
    pub fn report(&self) -> crate::PerfReport {
        let mut report = self.state.collector.report();
        report.entities = self.entities;
        if !self.headless && report.frames > 0 {
            let f = self.profiler.stats();
            let r = self.last_render_stats;
            let mut passes: Vec<_> = self
                .gpu_passes
                .iter()
                .map(|p| crate::PassReport {
                    name: p.name.into(),
                    ms: p.ms,
                })
                .collect();
            passes.sort_by(|a, b| b.ms.total_cmp(&a.ms));
            report.render = Some(crate::RenderReport {
                systems_ms: f.systems_ms,
                render_cpu_ms: f.render_cpu_ms,
                display_wait_ms: f.display_wait_ms,
                gpu_ms: f.gpu_ms,
                draw_calls: r.draw_calls,
                sprites: r.sprites,
                models: r.instances,
                triangles: r.triangles,
                shadow_models: r.shadow_instances,
                shadow_triangles: r.shadow_triangles,
                occluded: r.occluded,
                gpu_passes: passes,
            });
        }
        crate::perf_report::analyze(&mut report);
        report
    }

    /// Starts or stops the deeper profiling to match what's wanted: the
    /// overlay, a benchmark run, or `set_profiling`.
    pub(crate) fn sync_profiling(&mut self) {
        let wanted = self.state.explicit || self.profiler.hud || self.profiler.benchmarking();
        self.state.sync(wanted);
    }

    /// Ends a frame (or headless tick) that took `ms` for the report.
    pub(crate) fn end_frame(&mut self, ms: f32) {
        self.state.end_frame(ms, &self.timings);
    }

    /// Hands one frame's timings to the profiler (`gpu_frame` is the
    /// renderer's GPU spans of a finished frame, if it has one). `true` when
    /// a benchmark run is done and the game should quit.
    pub(crate) fn record_frame(
        &mut self,
        sample: crate::profiler::FrameSample,
        gpu_frame: Option<(&[crate::profiler::PassSpan], u64)>,
    ) -> bool {
        let quit = self.profiler.record(
            sample,
            &self.timings,
            &self.gpu_passes,
            gpu_frame,
            self.last_render_stats,
        );
        self.end_frame(sample.frame_ms);
        if self.profiler.warmup_ends_now() {
            // A benchmark's report covers the measured frames only.
            self.state.collector.clear();
        }
        if quit {
            let report = self.report();
            eprint!("{report}");
            if let Some(path) = crate::environment::var_os("PONKO_BENCH_REPORT")
                && let Err(e) = report.save(&path)
            {
                eprintln!("ponko: could not write {}: {e}", std::path::Path::new(&path).display());
            }
        }
        quit
    }
}
