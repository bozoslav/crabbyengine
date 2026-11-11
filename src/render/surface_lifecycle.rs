//! Presentation availability is separate from offscreen rendering. Losing a
//! surface or pausing presentation invalidates scene history, but never gates
//! a requested capture (or an offscreen benchmark).
#[derive(Default)]
pub(crate) struct SurfaceLifecycle {
    suspended: bool,
    occluded: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SurfaceEvent {
    Suspended(bool),
    Occluded(bool),
    Recreated,
}

impl SurfaceLifecycle {
    /// Whether scene targets/history must be invalidated. Repeated platform
    /// notifications do not keep resetting hidden offscreen frames.
    pub(crate) fn update(&mut self, event: SurfaceEvent) -> bool {
        let (state, value) = match event {
            SurfaceEvent::Suspended(value) => (&mut self.suspended, value),
            SurfaceEvent::Occluded(value) => (&mut self.occluded, value),
            SurfaceEvent::Recreated => return true,
        };
        let changed = *state != value;
        *state = value;
        changed
    }

    pub(crate) fn suspended(&self) -> bool {
        self.suspended
    }

    pub(crate) fn draw(has_surface: bool, capture: bool, draw_hidden: bool) -> bool {
        has_surface || capture || draw_hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_transitions_reset_once_and_never_block_offscreen_capture() {
        let mut state = SurfaceLifecycle::default();
        for event in [
            SurfaceEvent::Occluded(true),
            SurfaceEvent::Occluded(false),
            SurfaceEvent::Suspended(true),
            SurfaceEvent::Suspended(false),
            SurfaceEvent::Recreated, // lost
            SurfaceEvent::Recreated, // outdated/suboptimal
        ] {
            assert!(state.update(event), "{event:?}");
            if !matches!(event, SurfaceEvent::Recreated) {
                assert!(!state.update(event), "duplicate {event:?}");
            }
            assert!(SurfaceLifecycle::draw(false, true, false));
            assert!(SurfaceLifecycle::draw(false, false, true));
            assert!(!SurfaceLifecycle::draw(false, false, false));
            assert!(SurfaceLifecycle::draw(true, false, false));
        }
        state.update(SurfaceEvent::Suspended(true));
        state.update(SurfaceEvent::Occluded(false));
        assert!(
            state.suspended(),
            "an occlusion notification cannot resume a suspension"
        );
    }
}
