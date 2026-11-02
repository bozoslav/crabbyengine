//! Frame timing: delta time and the fixed-timestep accumulator.
//!
//! Kept free of any windowing types so it can be unit tested directly.

/// Longest frame delta the engine will accept, in seconds.
///
/// A stall (dragging the window, a breakpoint, a slow asset load) would otherwise
/// produce one huge `dt` and a burst of fixed steps trying to "catch up"
/// (the classic spiral of death). Clamping trades a brief slowdown for stability.
pub(crate) const MAX_FRAME_DT: f64 = 0.25;

/// Default fixed update rate.
pub(crate) const DEFAULT_FIXED_HZ: u32 = 60;

/// Most simulation steps run in one frame. At high sim speeds, or when the
/// simulation is too heavy to keep up, the backlog is dropped instead of
/// freezing the window: the simulation just runs slower than asked.
pub(crate) const MAX_STEPS_PER_FRAME: u32 = 64;

/// Timing results for one rendered frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FrameTiming {
    /// Clamped seconds since the previous frame.
    pub dt: f32,
    /// How many fixed steps the game should run this frame.
    pub fixed_steps: u32,
    /// How far (0..1) we are between the last fixed step and the next one.
    pub alpha: f32,
}

/// Turns variable frame deltas into a whole number of fixed steps per frame.
#[derive(Debug, Clone)]
pub(crate) struct Clock {
    fixed_dt: f64,
    accumulator: f64,
}

impl Clock {
    pub fn new(fixed_hz: u32) -> Self {
        assert!(fixed_hz > 0, "fixed update rate must be at least 1 Hz");
        Self {
            fixed_dt: 1.0 / f64::from(fixed_hz),
            accumulator: 0.0,
        }
    }

    pub fn fixed_dt(&self) -> f64 {
        self.fixed_dt
    }

    /// Advances the clock by one frame that took `frame_dt` seconds, with
    /// simulated time running `speed` times as fast (0 = paused).
    pub fn advance(&mut self, frame_dt: f64, speed: f64) -> FrameTiming {
        let dt = frame_dt.clamp(0.0, MAX_FRAME_DT);
        self.accumulator += dt * speed;

        let mut steps = (self.accumulator / self.fixed_dt).floor();
        if steps > f64::from(MAX_STEPS_PER_FRAME) {
            steps = f64::from(MAX_STEPS_PER_FRAME);
            self.accumulator = 0.0; // drop the backlog
        } else {
            self.accumulator -= steps * self.fixed_dt;
        }

        FrameTiming {
            dt: dt as f32,
            fixed_steps: steps as u32,
            alpha: (self.accumulator / self.fixed_dt) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_step_per_frame_when_rates_match() {
        let mut clock = Clock::new(60);
        let total: u32 = (0..600).map(|_| clock.advance(1.0 / 60.0, 1.0).fixed_steps).sum();
        // Float rounding may defer one step to the next frame, never more.
        assert!((599..=600).contains(&total), "got {total}");
    }

    #[test]
    fn fast_frames_accumulate_into_steps() {
        let mut clock = Clock::new(60);
        let total: u32 = (0..240).map(|_| clock.advance(1.0 / 240.0, 1.0).fixed_steps).sum();
        assert!((59..=60).contains(&total), "got {total}");
    }

    #[test]
    fn slow_frame_runs_multiple_steps() {
        let mut clock = Clock::new(60);
        let t = clock.advance(3.5 / 60.0, 1.0);
        assert_eq!(t.fixed_steps, 3);
        assert!((t.alpha - 0.5).abs() < 1e-4);
    }

    #[test]
    fn huge_frame_is_clamped() {
        let mut clock = Clock::new(60);
        let t = clock.advance(10.0, 1.0);
        assert_eq!(t.dt, MAX_FRAME_DT as f32);
        assert_eq!(t.fixed_steps, 15);
    }

    #[test]
    fn negative_dt_is_treated_as_zero() {
        let mut clock = Clock::new(60);
        let t = clock.advance(-1.0, 1.0);
        assert_eq!(t.dt, 0.0);
        assert_eq!(t.fixed_steps, 0);
    }

    #[test]
    fn speed_scales_steps() {
        let mut clock = Clock::new(60);
        let total: u32 = (0..60).map(|_| clock.advance(1.0 / 60.0, 4.0).fixed_steps).sum();
        assert!((239..=240).contains(&total), "got {total}");
    }

    #[test]
    fn zero_speed_pauses() {
        let mut clock = Clock::new(60);
        let total: u32 = (0..60).map(|_| clock.advance(1.0 / 60.0, 0.0).fixed_steps).sum();
        assert_eq!(total, 0);
    }

    #[test]
    fn steps_per_frame_are_capped_and_backlog_dropped() {
        let mut clock = Clock::new(60);
        let t = clock.advance(0.25, 100.0); // would be 1500 steps
        assert_eq!(t.fixed_steps, MAX_STEPS_PER_FRAME);
        assert_eq!(t.alpha, 0.0);
        let t = clock.advance(1.0 / 60.0, 1.0);
        assert_eq!(t.fixed_steps, 1, "no catch-up burst after the cap");
    }

    #[test]
    #[should_panic(expected = "at least 1 Hz")]
    fn zero_hz_panics() {
        Clock::new(0);
    }
}
