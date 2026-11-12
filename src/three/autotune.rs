//! GPU-only scene quality controller.

use serde::Serialize;

/// Opt-in GPU quality policy for one scene. Disabled until passed to
/// [`crate::Ctx::set_quality_autotune`]. Only render scale and sun shadow
/// resolution are controlled; every other setting stays game-owned.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct QualityAutotune {
    /// GPU frame budget in milliseconds. Default 16.667 (60 Hz).
    pub target_gpu_ms: f32,
    /// Inclusive scale floor, clamped to 0.25..=2. Default 0.5.
    pub min_render_scale: f32,
    /// Inclusive scale ceiling. Default 1.
    pub max_render_scale: f32,
    /// Sun shadow resolution floor, clamped to 256..=4096. Default 512.
    pub min_shadow_resolution: u32,
    /// Sun shadow resolution ceiling. Default 2048.
    pub max_shadow_resolution: u32,
}

impl Default for QualityAutotune {
    fn default() -> Self {
        Self {
            target_gpu_ms: 1000.0 / 60.0,
            min_render_scale: 0.5,
            max_render_scale: 1.0,
            min_shadow_resolution: 512,
            max_shadow_resolution: 2048,
        }
    }
}

impl QualityAutotune {
    fn clamped(self) -> Self {
        let finite = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        let min = finite(self.min_render_scale, 0.5).clamp(0.25, 2.0);
        let shadow = self.min_shadow_resolution.clamp(256, 4096);
        Self {
            target_gpu_ms: finite(self.target_gpu_ms, 1000.0 / 60.0).clamp(1.0, 1000.0),
            min_render_scale: min,
            max_render_scale: finite(self.max_render_scale, 1.0).clamp(min, 2.0),
            min_shadow_resolution: shadow,
            max_shadow_resolution: self.max_shadow_resolution.clamp(shadow, 4096),
        }
    }
}

/// Why the controller is waiting, or why its last decision was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QualityAutotuneReason {
    /// No distinct, valid, recent GPU timestamp sample is available.
    GpuUnavailable,
    /// Capture, headless, paused execution, or explicit freeze.
    Frozen,
    /// Waiting for enough consecutive evidence outside the dead band.
    Observing,
    /// Waiting after a quality decision.
    Cooldown,
    /// Sustained GPU time above 110% of the budget.
    OverBudget,
    /// Sustained GPU time below 75% of the budget.
    UnderBudget,
    /// No permitted setting can move further in this direction.
    BoundsOrPinned,
    /// A quadratic pixel/map-size estimate would exceed 90% of the budget.
    InsufficientHeadroom,
}
use QualityAutotuneReason as Reason;

/// One decision. History retains the last 16 decisions, oldest first.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct QualityChange {
    /// Monotonic GPU readback sequence, not CPU frame time.
    pub sample: u64,
    /// Measured unsmoothed GPU milliseconds that completed the evidence run.
    pub gpu_ms: f32,
    /// Reason for the adjustment.
    pub reason: QualityAutotuneReason,
    /// Previous render scale.
    pub previous_render_scale: f32,
    /// New render scale.
    pub render_scale: f32,
    /// Previous sun shadow resolution.
    pub previous_shadow_resolution: u32,
    /// New sun shadow resolution.
    pub shadow_resolution: u32,
}

/// Read-only controller diagnostics. Constructed on request, never per frame.
#[derive(Debug, Clone, Serialize)]
pub struct QualityAutotuneStatus {
    /// Sanitized scene policy.
    pub config: QualityAutotune,
    /// Current render scale.
    pub render_scale: f32,
    /// Current sun shadow resolution.
    pub shadow_resolution: u32,
    /// Explicit settings take precedence for the remainder of the scene.
    pub render_scale_pinned: bool,
    /// Whether sun shadow resolution is game-owned.
    pub shadow_resolution_pinned: bool,
    /// Whether the game explicitly froze the controller.
    pub frozen: bool,
    /// Current waiting/decision reason.
    pub reason: QualityAutotuneReason,
    /// Number of decisions since this policy was enabled.
    pub changes: u64,
    /// Last 16 decisions, oldest first.
    pub history: Vec<QualityChange>,
}

#[derive(Clone, Copy)]
pub(crate) struct Sample {
    pub sequence: u64,
    pub ms: f32,
    pub fresh: bool,
}

pub(crate) struct Controller {
    config: QualityAutotune,
    pub(crate) scale: f32,
    pub(crate) shadow: u32,
    pub(crate) scale_pinned: bool,
    pub(crate) shadow_pinned: bool,
    pub(crate) frozen: bool,
    reason: Reason,
    last_sample: u64,
    over: u32,
    under: u32,
    cooldown: u32,
    changes: u64,
    history: [Option<QualityChange>; 16],
}

impl Controller {
    pub(crate) fn new(
        config: QualityAutotune,
        scale: f32,
        shadow: u32,
        scale_pinned: bool,
        shadow_pinned: bool,
    ) -> Self {
        let config = config.clamped();
        let scale_pinned = scale_pinned || !(config.min_render_scale..=config.max_render_scale).contains(&scale);
        let shadow_pinned =
            shadow_pinned || !(config.min_shadow_resolution..=config.max_shadow_resolution).contains(&shadow);
        Self {
            config,
            scale,
            shadow,
            scale_pinned,
            shadow_pinned,
            frozen: false,
            reason: Reason::GpuUnavailable,
            last_sample: 0,
            over: 0,
            under: 0,
            cooldown: 0,
            changes: 0,
            history: [None; 16],
        }
    }

    pub(crate) fn status(&self) -> QualityAutotuneStatus {
        QualityAutotuneStatus {
            config: self.config,
            render_scale: self.scale,
            shadow_resolution: self.shadow,
            render_scale_pinned: self.scale_pinned,
            shadow_resolution_pinned: self.shadow_pinned,
            frozen: self.frozen,
            reason: self.reason,
            changes: self.changes,
            history: self.history.iter().flatten().copied().collect(),
        }
    }

    // No allocation, CPU timing input, or GPU polling. Called once before scene preparation.
    pub(crate) fn update(&mut self, sample: Option<Sample>, frozen: bool, scale: f32, shadow: u32) {
        if scale != self.scale {
            self.scale_pinned = true;
            self.scale = scale;
        }
        if shadow != self.shadow {
            self.shadow_pinned = true;
            self.shadow = shadow;
        }
        let sample = sample
            .filter(|s| s.fresh && s.sequence > self.last_sample && s.ms.is_finite() && s.ms > 0.0 && s.ms < 10_000.0);
        if let Some(s) = sample {
            self.last_sample = s.sequence;
        }
        if frozen || self.frozen || sample.is_none() {
            self.over = 0;
            self.under = 0;
            self.reason = if frozen || self.frozen {
                Reason::Frozen
            } else {
                Reason::GpuUnavailable
            };
            return;
        }
        let s = sample.unwrap();
        if self.cooldown > 0 {
            self.cooldown -= 1;
            self.reason = Reason::Cooldown;
            return;
        }
        self.over = if s.ms > self.config.target_gpu_ms * 1.10 {
            self.over + 1
        } else {
            0
        };
        self.under = if s.ms < self.config.target_gpu_ms * 0.75 {
            self.under + 1
        } else {
            0
        };
        let down = self.over >= 30;
        if !down && self.under < 180 {
            self.reason = Reason::Observing;
            return;
        }
        self.over = 0;
        self.under = 0;
        let old_scale = self.scale;
        let old_shadow = self.shadow;
        let q = self.config;
        // One dimension per decision. Reduce pixels first, then sun maps;
        // restore sun maps first, then pixels. Never toggle effects/features.
        if down {
            if !self.scale_pinned && self.scale > q.min_render_scale {
                self.scale = (self.scale - 0.05).max(q.min_render_scale).min(q.max_render_scale);
            } else if !self.shadow_pinned && self.shadow > q.min_shadow_resolution {
                self.shadow = (self.shadow / 2)
                    .max(q.min_shadow_resolution)
                    .min(q.max_shadow_resolution);
            }
        } else if !self.shadow_pinned && self.shadow < q.max_shadow_resolution {
            let next = (self.shadow * 2)
                .min(q.max_shadow_resolution)
                .max(q.min_shadow_resolution);
            let ratio = next as f32 / self.shadow as f32;
            if s.ms * ratio * ratio > q.target_gpu_ms * 0.9 {
                self.reason = Reason::InsufficientHeadroom;
                return;
            }
            self.shadow = next;
        } else if !self.scale_pinned && self.scale < q.max_render_scale {
            let next = (self.scale + 0.05).min(q.max_render_scale).max(q.min_render_scale);
            let ratio = next / self.scale;
            if s.ms * ratio * ratio > q.target_gpu_ms * 0.9 {
                self.reason = Reason::InsufficientHeadroom;
                return;
            }
            self.scale = next;
        }
        if self.scale == old_scale && self.shadow == old_shadow {
            self.reason = Reason::BoundsOrPinned;
            return;
        }
        self.reason = if down { Reason::OverBudget } else { Reason::UnderBudget };
        self.history.rotate_left(1);
        self.history[15] = Some(QualityChange {
            sample: s.sequence,
            gpu_ms: s.ms,
            reason: self.reason,
            previous_render_scale: old_scale,
            render_scale: self.scale,
            previous_shadow_resolution: old_shadow,
            shadow_resolution: self.shadow,
        });
        self.changes += 1;
        self.cooldown = 120;
    }
}

impl crate::Ctx {
    /// Enables or disables automatic quality for this scene. Default is off.
    /// Existing non-default values and values set with `Graphics::set_quality`
    /// are pinned. Opting in delegates untouched defaults to the controller.
    /// Direct writes differing from the last automatic value pin that field;
    /// use `pin_render_scale` / `pin_shadow_resolution` for same-value writes.
    /// Values outside the policy bounds are also pinned rather than snapped.
    /// Scene replacement disables the policy; enable it again for the new scene.
    /// Disabling retains the last choices. Headless/capture/paused frames freeze it.
    pub fn set_quality_autotune(&mut self, policy: Option<QualityAutotune>) {
        self.gfx.autotune = policy.map(|q| {
            Controller::new(
                q,
                self.post.render_scale,
                self.lighting.shadow_resolution,
                self.gfx.quality_pinned
                    || self.gfx.scale_pinned
                    || self.post.render_scale != super::Post::default().render_scale,
                self.gfx.quality_pinned
                    || self.gfx.shadow_pinned
                    || self.lighting.shadow_resolution != super::Lighting::default().shadow_resolution,
            )
        });
    }

    /// Current policy and bounded decision history, or `None` when disabled.
    #[must_use]
    pub fn quality_autotune_status(&self) -> Option<QualityAutotuneStatus> {
        self.gfx.autotune.as_ref().map(|c| {
            let mut status = c.status();
            status.render_scale = self.post.render_scale;
            status.shadow_resolution = self.lighting.shadow_resolution;
            status.render_scale_pinned |= self.post.render_scale != c.scale;
            status.shadow_resolution_pinned |= self.lighting.shadow_resolution != c.shadow;
            if self.headless || self.time.is_paused() || self.input.scenarios.run.is_some() || c.frozen {
                status.reason = Reason::Frozen;
            }
            status
        })
    }

    /// Freezes/unfreezes the policy without losing its choices/history.
    /// Use before a deterministic capture sequence, including its warm-up.
    pub fn freeze_quality_autotune(&mut self, frozen: bool) {
        if let Some(c) = &mut self.gfx.autotune {
            c.frozen = frozen;
            c.over = 0;
            c.under = 0;
        }
    }

    /// Writes and pins render scale, even when equal to the automatic value.
    pub fn pin_render_scale(&mut self, scale: f32) {
        self.post.render_scale = scale;
        if let Some(c) = &mut self.gfx.autotune {
            c.scale_pinned = true;
            c.scale = scale;
        }
        self.gfx.scale_pinned = true;
    }

    /// Writes and pins sun shadow resolution, including same-value writes.
    pub fn pin_shadow_resolution(&mut self, resolution: u32) {
        self.lighting.shadow_resolution = resolution;
        if let Some(c) = &mut self.gfx.autotune {
            c.shadow_pinned = true;
            c.shadow = resolution;
        }
        self.gfx.shadow_pinned = true;
    }

    pub(crate) fn update_quality_autotune(&mut self, sample: Option<Sample>, frozen: bool) {
        if let Some(c) = &mut self.gfx.autotune {
            c.update(
                sample,
                frozen || self.headless || self.time.is_paused() || self.input.scenarios.run.is_some(),
                self.post.render_scale,
                self.lighting.shadow_resolution,
            );
            if !c.scale_pinned && c.scale != self.post.render_scale {
                self.post.render_scale = c.scale;
                // The same central reset contract as teleports and scene changes.
                // prepare_view also invalidates targets when dimensions change.
                self.gfx.reset_temporal_history();
            }
            let c = self.gfx.autotune.as_ref().unwrap();
            if !c.shadow_pinned {
                self.lighting.shadow_resolution = c.shadow;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(times: impl IntoIterator<Item = Option<f32>>) -> Controller {
        let mut c = Controller::new(QualityAutotune::default(), 1.0, 2048, false, false);
        for (i, ms) in times.into_iter().enumerate() {
            c.update(
                ms.map(|ms| Sample {
                    sequence: i as u64 + 1,
                    ms,
                    fresh: true,
                }),
                false,
                c.scale,
                c.shadow,
            );
        }
        c
    }

    #[test]
    fn sustained_load_steps_to_bounds_without_reversing() {
        let over = run([Some(30.0); 4000]);
        assert_eq!(over.scale, 0.5);
        assert_eq!(over.shadow, 512);
        assert!(over.history.iter().flatten().all(|h| h.reason == Reason::OverBudget));
        let under = run([Some(4.0); 4000]);
        assert_eq!(under.scale, 1.0);
        assert_eq!(under.shadow, 2048);
        assert_eq!(under.changes, 0);
    }

    #[test]
    fn noise_spikes_and_missing_data_never_cause_decisions() {
        for times in [
            (0..4000)
                .map(|i| Some(if i % 2 == 0 { 15.0 } else { 18.0 }))
                .collect::<Vec<_>>(),
            (0..4000).map(|i| Some(if i % 20 == 0 { 80.0 } else { 16.0 })).collect(),
            vec![None; 4000],
        ] {
            assert_eq!(run(times).changes, 0);
        }
    }

    #[test]
    fn stale_repeated_invalid_and_frozen_samples_do_not_accumulate_evidence() {
        let mut c = run([None]);
        for i in 1..500 {
            for sample in [
                None,
                Some(Sample {
                    sequence: i,
                    ms: 99.0,
                    fresh: false,
                }),
                Some(Sample {
                    sequence: i,
                    ms: f32::NAN,
                    fresh: true,
                }),
            ] {
                c.update(sample, false, 1.0, 2048);
            }
            c.update(
                Some(Sample {
                    sequence: i,
                    ms: 99.0,
                    fresh: true,
                }),
                true,
                1.0,
                2048,
            );
        }
        assert_eq!(c.changes, 0);
        let sample = Some(Sample {
            sequence: 1000,
            ms: 99.0,
            fresh: true,
        });
        for _ in 0..500 {
            c.update(sample, false, 1.0, 2048);
        }
        assert_eq!(c.changes, 0);
    }
}

#[cfg(test)]
mod regressions {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn overload_cycles_do_not_reverse_and_recovery_is_slow_and_bounded() {
        let mut c = Controller::new(QualityAutotune::default(), 1.0, 2048, false, false);
        let mut seq = 0;
        for i in 0..6000 {
            seq += 1;
            c.update(
                Some(Sample {
                    sequence: seq,
                    ms: if i % 240 < 120 { 30.0 } else { 4.0 },
                    fresh: true,
                }),
                false,
                c.scale,
                c.shadow,
            );
        }
        assert_eq!((c.scale, c.shadow), (0.5, 512));
        assert!(c.history.iter().flatten().all(|h| h.reason == Reason::OverBudget));
        let count = c.changes;
        for _ in 0..6000 {
            seq += 1;
            c.update(
                Some(Sample {
                    sequence: seq,
                    ms: 2.0,
                    fresh: true,
                }),
                false,
                c.scale,
                c.shadow,
            );
        }
        assert_eq!((c.scale, c.shadow), (1.0, 2048));
        assert!(c.changes > count);
        for h in c.history.iter().flatten() {
            assert!((h.render_scale - h.previous_render_scale).abs() <= 0.05001);
        }
        let entries: Vec<_> = c.history.iter().flatten().collect();
        for pair in entries.windows(2) {
            assert!(pair[1].sample - pair[0].sample >= 150);
        }
    }

    #[test]
    fn explicit_settings_freeze_and_headless_win_even_with_gpu_samples() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
        assert!(ctx.quality_autotune_status().is_none());
        ctx.gfx
            .set_quality(&mut ctx.post, &mut ctx.lighting, crate::render::GraphicsQuality::high());
        ctx.set_quality_autotune(Some(QualityAutotune::default()));
        let q = ctx.gfx.quality(&ctx.post, &ctx.lighting);
        for i in 1..4000 {
            ctx.update_quality_autotune(
                Some(Sample {
                    sequence: i,
                    ms: 80.0,
                    fresh: true,
                }),
                false,
            );
        }
        assert_eq!(ctx.gfx.quality(&ctx.post, &ctx.lighting), q);
        assert_eq!(ctx.quality_autotune_status().unwrap().changes, 0);
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
        ctx.set_quality_autotune(Some(QualityAutotune::default()));
        ctx.pin_render_scale(1.0);
        ctx.pin_shadow_resolution(2048);
        for i in 1..4000 {
            ctx.update_quality_autotune(
                Some(Sample {
                    sequence: i,
                    ms: 80.0,
                    fresh: true,
                }),
                false,
            );
        }
        assert_eq!(ctx.quality_autotune_status().unwrap().changes, 0);
        for headless in [false, true] {
            let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
            ctx.set_headless(headless);
            ctx.set_quality_autotune(Some(QualityAutotune::default()));
            for i in 1..4000 {
                ctx.update_quality_autotune(
                    Some(Sample {
                        sequence: i,
                        ms: 80.0,
                        fresh: true,
                    }),
                    !headless,
                );
            }
            assert_eq!(ctx.quality_autotune_status().unwrap().changes, 0);
        }
    }

    #[test]
    fn direct_writes_and_invalid_bounds_remain_safe() {
        let mut c = Controller::new(
            QualityAutotune {
                min_render_scale: f32::NAN,
                max_render_scale: -4.0,
                target_gpu_ms: f32::INFINITY,
                min_shadow_resolution: 8192,
                max_shadow_resolution: 0,
            },
            1.0,
            2048,
            false,
            false,
        );
        for i in 1..4000 {
            c.update(
                Some(Sample {
                    sequence: i,
                    ms: 80.0,
                    fresh: true,
                }),
                false,
                0.8,
                1024,
            );
        }
        assert_eq!((c.scale, c.shadow), (0.8, 1024));
        assert!(c.scale_pinned && c.shadow_pinned);
        assert_eq!(c.changes, 0);
        assert_eq!(c.config.min_render_scale, c.config.max_render_scale);
        assert_eq!(c.config.min_shadow_resolution, c.config.max_shadow_resolution);
    }

    #[test]
    fn cpu_load_and_old_gpu_statistics_are_never_gpu_evidence() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
        ctx.set_quality_autotune(Some(QualityAutotune::default()));
        ctx.perf.last_render_stats.cpu_ms = 500.0;
        ctx.perf.last_render_stats.gpu_ms = Some(80.0); // stale smoothed statistic
        for _ in 0..4000 {
            ctx.update_quality_autotune(None, false);
        }
        assert_eq!(ctx.post.render_scale, 1.0);
        assert_eq!(ctx.lighting.shadow_resolution, 2048);
        assert_eq!(ctx.quality_autotune_status().unwrap().changes, 0);
        ctx.freeze_quality_autotune(true);
        for i in 1..4000 {
            ctx.update_quality_autotune(
                Some(Sample {
                    sequence: i,
                    ms: 80.0,
                    fresh: true,
                }),
                false,
            );
        }
        assert_eq!(ctx.quality_autotune_status().unwrap().changes, 0);
        ctx.freeze_quality_autotune(false);
        for i in 4000..4030 {
            ctx.update_quality_autotune(
                Some(Sample {
                    sequence: i,
                    ms: 80.0,
                    fresh: true,
                }),
                false,
            );
        }
        assert_eq!(ctx.quality_autotune_status().unwrap().changes, 1);
    }

    #[test]
    fn shadow_feedback_does_not_ping_pong_when_doubling_would_exceed_budget() {
        let mut c = Controller::new(QualityAutotune::default(), 1.0, 2048, true, false);
        for i in 1..6000 {
            // Shadow-dominated synthetic GPU: each dimension doubles cost.
            let ms = 80.0 * (c.shadow as f32 / 2048.0).powi(2);
            c.update(
                Some(Sample {
                    sequence: i,
                    ms,
                    fresh: true,
                }),
                false,
                c.scale,
                c.shadow,
            );
        }
        assert_eq!(c.shadow, 512);
        assert_eq!(c.changes, 2, "failed shadow upgrades must not repeat forever");
    }

    #[test]
    fn scene_replacement_discards_the_old_scene_policy() {
        let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
        ctx.set_quality_autotune(Some(QualityAutotune::default()));
        let save = ctx.world.save();
        ctx.replace_world(&save).unwrap();
        assert!(ctx.quality_autotune_status().is_none());
    }

    /// Run explicitly in release; this measures controller dispatch only,
    /// excluding GPU polling and target/map recreation on a decision.
    #[test]
    #[ignore = "release microbenchmark, not a timing assertion"]
    fn quality_autotune_overhead() {
        use std::hint::black_box;
        const N: u64 = 2_000_000;
        for enabled in [false, true] {
            let mut ctx = Ctx::new(1.0 / 60.0, Vec2::ONE);
            if enabled {
                ctx.set_quality_autotune(Some(QualityAutotune::default()));
            }
            for i in 1..10_000 {
                ctx.update_quality_autotune(
                    Some(Sample {
                        sequence: i,
                        ms: 16.0,
                        fresh: true,
                    }),
                    false,
                );
            }
            let start = std::time::Instant::now();
            for i in 10_000..N + 10_000 {
                black_box(&mut ctx).update_quality_autotune(
                    black_box(Some(Sample {
                        sequence: i,
                        ms: 16.0,
                        fresh: true,
                    })),
                    false,
                );
            }
            eprintln!(
                "autotune enabled={enabled}: {:.2} ns/frame ({N} iterations after 9999 warm-up)",
                start.elapsed().as_nanos() as f64 / N as f64
            );
            black_box(ctx);
        }
    }
}
