//! The game's custom passes as declared reads and writes, checked for
//! hazards before a frame runs them.
//!
//! A game adds work to a 3D frame at four named stages, in this order:
//!
//! | Stage | From | Writes its target by |
//! |---|---|---|
//! | compute | `Ctx::compute_passes` | reading what is there (read-modify-write) |
//! | target cameras | `Ctx::target_cameras` | replacing all of it |
//! | scene compute | `Ctx::scene_compute_passes` | reading what is there |
//! | target effects | `Ctx::target_effects` | replacing all of it |
//!
//! (screen effects come last and write only the frame). Each entry declares
//! what it writes (its render target or GPU buffer) and what it reads (the
//! textures its material binds, plus the scene at the later stages). The
//! frame's description (`passes.rs`) declares the stages themselves: what
//! each reads and writes of the engine's images, as aggregates
//! (`ComputeTargets`, `SceneComputeTargets`, `EffectTargets`,
//! `TargetImage`). This module applies the same rules to the game's own
//! render targets, one by one, each frame:
//!
//! - **Feedback**: a pass that renders to a texture can't sample it in the
//!   same pass (a target effect whose material samples its own target).
//!   Compute passes read their target and write a scratch copy, so their
//!   material may sample it.
//! - **Discarded writes**: of two passes writing one render target, the
//!   later one must read what the earlier one wrote. A target camera or
//!   target effect clears its target, so it can't come after any other
//!   writer of it: the earlier result would be lost without a trace. A
//!   compute pass after a target camera reads the camera's picture, which
//!   is allowed.
//!
//! A pass that breaks a rule is skipped, never run with the hazard, and a
//! warning naming the pass, the render target and the rule is printed once
//! (again only if the hazard goes away and comes back).
//! GPU buffers have no such rule: every buffer compute pass reads and
//! writes its buffer in place, in stage and list order.
//!
//! What a pass reads before a later pass of the frame writes it is the
//! previous frame's contents (as `passes.rs` declares with
//! `Access::SamplePrevious`); that is an order, not a hazard.

use std::fmt;

/// A stage of the frame that runs the game's passes, in frame order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Stage {
    /// `Ctx::compute_passes`, before anything is drawn.
    Compute,
    /// `Ctx::target_cameras`, before the main view.
    TargetCamera,
    /// `Ctx::scene_compute_passes`, once the lit frame is settled.
    SceneCompute,
    /// `Ctx::target_effects`, after the scene compute passes.
    TargetEffect,
}

impl Stage {
    const fn name(self) -> &'static str {
        match self {
            Stage::Compute => "compute pass",
            Stage::TargetCamera => "target camera",
            Stage::SceneCompute => "scene compute pass",
            Stage::TargetEffect => "target effect",
        }
    }

    /// The `Ctx` list its entries come from.
    const fn list(self) -> &'static str {
        match self {
            Stage::Compute => "ctx.compute_passes",
            Stage::TargetCamera => "ctx.target_cameras",
            Stage::SceneCompute => "ctx.scene_compute_passes",
            Stage::TargetEffect => "ctx.target_effects",
        }
    }

    /// Whether it replaces its target's whole contents (a cleared render
    /// pass) rather than reading what is there.
    const fn discards(self) -> bool {
        matches!(self, Stage::TargetCamera | Stage::TargetEffect)
    }

    /// Whether it is one of the game's custom passes (which a hazard
    /// skips), not a target camera.
    const fn custom(self) -> bool {
        !matches!(self, Stage::TargetCamera)
    }
}

/// One pass that writes a render target, as the check sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Writer {
    pub stage: Stage,
    /// Its index in its stage's list for this frame.
    pub index: usize,
    /// Its shader: an effect or compute id (a camera's index again).
    pub shader: u32,
    /// The render target it writes (a texture id).
    pub target: u32,
}

impl fmt::Display for Writer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shader = match self.stage {
            Stage::Compute | Stage::SceneCompute => "compute",
            Stage::TargetEffect => "effect",
            Stage::TargetCamera => return write!(f, "{} {} ({})", self.stage.name(), self.index, self.stage.list()),
        };
        write!(
            f,
            "{} {} ({}, {shader} {})",
            self.stage.name(),
            self.index,
            self.stage.list(),
            self.shader
        )
    }
}

/// A rule a frame's custom passes break; the frame skips `skipped()`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hazard {
    /// `pass` renders to its target and its material samples it.
    Feedback { pass: Writer },
    /// `later` clears the render target `earlier` wrote before it this
    /// frame, so one of their results is lost.
    Discarded { earlier: Writer, later: Writer },
}

impl Hazard {
    /// The pass the frame leaves out: the custom pass of the two, the
    /// later one if both are.
    pub(crate) fn skipped(self) -> Writer {
        match self {
            Hazard::Feedback { pass } => pass,
            Hazard::Discarded { later, .. } if later.stage.custom() => later,
            Hazard::Discarded { earlier, .. } => earlier,
        }
    }
}

impl fmt::Display for Hazard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Hazard::Feedback { pass } => write!(
                f,
                "ponko: {pass} skipped: its material samples render target {}, which it renders to \
                 (one pass can't read and write a texture); give it a material without that texture",
                pass.target
            ),
            Hazard::Discarded { earlier, later } => write!(
                f,
                "ponko: {} skipped: render target {} is written by {earlier} and then by {later}, \
                 which replaces all of it, so the first result would be lost; give each pass its own \
                 render target",
                self.skipped(),
                later.target,
            ),
        }
    }
}

impl std::error::Error for Hazard {}

/// Checks a frame's writers of render targets, in frame order (by stage,
/// then list order), for the rules above. `samples_own_target` tells
/// whether a writer's material samples the texture it writes. Fills
/// `hazards` (cleared first); doesn't allocate once it has the capacity.
pub(crate) fn check(writers: &[Writer], samples_own_target: impl Fn(&Writer) -> bool, hazards: &mut Vec<Hazard>) {
    hazards.clear();
    debug_assert!(
        writers
            .windows(2)
            .all(|w| (w[0].stage, w[0].index) < (w[1].stage, w[1].index)),
        "writers are in frame order"
    );
    for pass in writers {
        if pass.stage == Stage::TargetEffect && samples_own_target(pass) {
            hazards.push(Hazard::Feedback { pass: *pass });
        }
    }
    for (i, earlier) in writers.iter().enumerate() {
        for later in &writers[i + 1..] {
            if later.target == earlier.target
                && later.stage.discards()
                && (earlier.stage.custom() || later.stage.custom())
            {
                hazards.push(Hazard::Discarded {
                    earlier: *earlier,
                    later: *later,
                });
            }
        }
    }
}

/// What the renderer keeps of the check between frames.
#[derive(Default)]
pub(crate) struct CustomPasses {
    /// This frame's writers, in frame order (scratch).
    pub writers: Vec<Writer>,
    /// This frame's hazards (scratch).
    pub hazards: Vec<Hazard>,
    /// Hazards already reported, so each is printed once while its passes
    /// stay listed; never more than this frame's hazards.
    warned: Vec<Hazard>,
    /// Frames the check ran in (it doesn't with no custom passes).
    #[cfg(test)]
    pub checked: usize,
}

impl CustomPasses {
    /// Runs the check over `writers` (see [`check`]) and reports new
    /// hazards. A frame without custom passes returns at once.
    pub(crate) fn run(&mut self, samples_own_target: impl Fn(&Writer) -> bool) {
        if !self.writers.iter().any(|w| w.stage.custom()) {
            self.hazards.clear();
            self.warned.clear();
            return;
        }
        #[cfg(test)]
        {
            self.checked += 1;
        }
        check(&self.writers, samples_own_target, &mut self.hazards);
        // Forget hazards that are gone (a pass taken off its list, moved or
        // retargeted), so the list never outgrows this frame's hazards.
        let hazards = &self.hazards;
        self.warned.retain(|h| hazards.contains(h));
        for hazard in &self.hazards {
            if !self.warned.contains(hazard) {
                self.warned.push(*hazard);
                crate::render::traffic::warn(&hazard.to_string());
            }
        }
    }

    /// Whether entry `index` of `stage` is skipped this frame.
    pub(crate) fn skips(&self, stage: Stage, index: usize) -> bool {
        self.hazards.iter().any(|h| {
            let s = h.skipped();
            s.stage == stage && s.index == index
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(stage: Stage, index: usize, target: u32) -> Writer {
        Writer {
            stage,
            index,
            shader: 9,
            target,
        }
    }

    fn hazards(writers: &[Writer], feedback: &[usize]) -> Vec<Hazard> {
        let mut out = Vec::new();
        check(
            writers,
            |p| p.stage == Stage::TargetEffect && feedback.contains(&p.index),
            &mut out,
        );
        out
    }

    #[test]
    fn passes_writing_their_own_targets_have_no_hazard() {
        let frame = [
            w(Stage::Compute, 0, 1),
            w(Stage::TargetCamera, 0, 2),
            w(Stage::SceneCompute, 0, 3),
            w(Stage::TargetEffect, 0, 4),
        ];
        assert!(hazards(&frame, &[]).is_empty());
    }

    #[test]
    fn read_modify_write_chains_are_allowed() {
        // Compute passes of either stage read what is there: a chain of
        // simulation steps, and a scene pass refining a camera's picture.
        let frame = [
            w(Stage::Compute, 0, 1),
            w(Stage::Compute, 1, 1),
            w(Stage::TargetCamera, 0, 2),
            w(Stage::SceneCompute, 0, 1),
            w(Stage::SceneCompute, 1, 2),
        ];
        assert!(hazards(&frame, &[]).is_empty());
    }

    #[test]
    fn a_clearing_pass_after_another_writer_is_a_hazard_naming_both() {
        let compute = w(Stage::SceneCompute, 0, 7);
        let effect = w(Stage::TargetEffect, 1, 7);
        let found = hazards(&[compute, effect], &[]);
        assert_eq!(
            found,
            [Hazard::Discarded {
                earlier: compute,
                later: effect
            }]
        );
        // The later custom pass is skipped, and the message names the
        // pass, its list, its shader and the render target.
        assert_eq!(found[0].skipped(), effect);
        let text = found[0].to_string();
        assert!(
            text.contains("target effect 1 (ctx.target_effects, effect 9) skipped"),
            "{text}"
        );
        assert!(text.contains("render target 7"), "{text}");
        assert!(
            text.contains("scene compute pass 0 (ctx.scene_compute_passes"),
            "{text}"
        );
    }

    #[test]
    fn a_camera_clearing_a_compute_target_skips_the_compute_pass() {
        let compute = w(Stage::Compute, 0, 5);
        let camera = w(Stage::TargetCamera, 0, 5);
        let found = hazards(&[compute, camera], &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].skipped(), compute, "cameras are not custom passes");
        assert!(found[0].to_string().contains("target camera 0 (ctx.target_cameras)"));
    }

    #[test]
    fn two_target_effects_into_one_target_keep_the_first() {
        let first = w(Stage::TargetEffect, 0, 3);
        let second = w(Stage::TargetEffect, 1, 3);
        let found = hazards(&[first, second], &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].skipped(), second);
    }

    #[test]
    fn two_cameras_sharing_a_target_are_not_reported_here() {
        let frame = [w(Stage::TargetCamera, 0, 3), w(Stage::TargetCamera, 1, 3)];
        assert!(hazards(&frame, &[]).is_empty());
    }

    #[test]
    fn a_target_effect_sampling_its_own_target_is_feedback() {
        let effect = w(Stage::TargetEffect, 0, 3);
        let found = hazards(&[effect], &[0]);
        assert_eq!(found, [Hazard::Feedback { pass: effect }]);
        let text = found[0].to_string();
        assert!(
            text.contains("target effect 0") && text.contains("render target 3"),
            "{text}"
        );
        // Compute passes may sample the target they update.
        let compute = w(Stage::Compute, 0, 3);
        let mut out = Vec::new();
        check(&[compute], |_| true, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn the_check_returns_at_once_without_custom_passes() {
        let mut passes = CustomPasses::default();
        passes.writers.push(w(Stage::TargetCamera, 0, 1));
        passes.run(|_| unreachable!("no custom pass to look at"));
        assert_eq!(passes.checked, 0);
        assert!(passes.hazards.is_empty());
        passes.writers.push(w(Stage::TargetEffect, 0, 1));
        passes.run(|_| false);
        assert_eq!(passes.checked, 1);
        assert!(passes.skips(Stage::TargetEffect, 0));
        assert!(!passes.skips(Stage::TargetCamera, 0));
    }

    #[test]
    fn warned_hazards_are_kept_only_while_they_last() {
        let mut passes = CustomPasses::default();
        passes
            .writers
            .extend([w(Stage::SceneCompute, 0, 1), w(Stage::TargetEffect, 0, 1)]);
        passes.run(|_| false);
        passes.run(|_| false);
        assert_eq!(passes.warned.len(), 1, "warned once, kept while it lasts");
        // The target effect moves to a target of its own: nothing to keep.
        passes.writers[1].target = 2;
        passes.run(|_| false);
        assert!(passes.hazards.is_empty() && passes.warned.is_empty());
        // Many hazards come and go; the list follows the frame's.
        for target in 3..100 {
            passes.writers[0].target = target;
            passes.writers[1].target = target;
            passes.run(|_| false);
            assert_eq!(passes.warned.len(), 1);
        }
        passes.writers.clear();
        passes.run(|_| false);
        assert!(passes.warned.is_empty());
    }
}
