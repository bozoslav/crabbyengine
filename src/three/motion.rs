//! Which models moved since the last frame, for temporal reprojection: the
//! renderer draws their motion vectors from the model matrix they had then.
//! Everything else moves only with the camera, which the renderer works out
//! from depth.

use std::hash::Hasher;

use super::collect::Instance3D;

/// Hashes entity ids (already well spread in their low bits) with one
/// multiplication, mixing in whatever is written after them.
#[derive(Default)]
pub(crate) struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// A model that moved: the instance as drawn this frame, then the model
/// matrix it had in the last. The renderer reads it as a vertex buffer
/// (instance attributes, then these four columns).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MovedInstance {
    pub instance: Instance3D,
    pub previous: [[f32; 4]; 4],
}

/// Lists the models the camera drew in the last frame and draws again in
/// this one with another matrix. The collection keeps every drawn model's
/// instance between frames (see `retained`), so it tells this what changed;
/// nothing is listed until `enabled`.
#[derive(Default)]
pub(crate) struct Motion {
    /// Whether TAA or temporal AO is on: set before `Frame3D::collect`.
    pub enabled: bool,
    /// Whether the game asked to forget the history (a camera cut).
    pub reset: bool,
    /// Whether the last frame was collected with motion on: the frame it
    /// is turned on has no past.
    armed: bool,
    /// Whether this frame lists motion.
    tracking: bool,
    /// (mesh, moved instance) in the order they were found.
    moved: Vec<(u32, MovedInstance)>,
    /// The moved models, grouped by mesh...
    pub instances: Vec<MovedInstance>,
    /// ...and their runs: (mesh, first instance, count).
    pub runs: Vec<(u32, u32, u32)>,
}

/// An entity key that is none (shadow-only casters have no motion).
pub(crate) const NO_KEY: u64 = u64::MAX;

impl Motion {
    /// Starts a frame.
    pub fn begin(&mut self) {
        self.moved.clear();
        self.instances.clear();
        self.runs.clear();
        // After a reset the last frame is no past of this one: a model
        // teleported with it has no motion.
        self.tracking = self.enabled && self.armed && !self.reset;
        self.armed = self.enabled;
    }

    /// Whether this frame lists motion: `moved` does nothing otherwise.
    pub fn tracking(&self) -> bool {
        self.tracking
    }

    /// Notes that the camera, which drew a model with matrix `previous` in
    /// the last frame, draws it as `instance` (with `mesh`) in this one.
    pub fn moved(&mut self, mesh: u32, instance: &Instance3D, mut previous: [[f32; 4]; 4]) {
        if !self.tracking {
            return;
        }
        // The first slot of the matrix carries the decal layers: not motion.
        previous[0][3] = instance.model[0][3];
        if previous != instance.model {
            self.moved.push((
                mesh,
                MovedInstance {
                    instance: *instance,
                    previous,
                },
            ));
        }
    }

    /// Ends a frame: groups the moved models by mesh.
    pub fn finish(&mut self) {
        self.moved.sort_unstable_by_key(|m| m.0);
        for &(mesh, moved) in &self.moved {
            match self.runs.last_mut() {
                Some(run) if run.0 == mesh => run.2 += 1,
                _ => self.runs.push((mesh, self.instances.len() as u32, 1)),
            }
            self.instances.push(moved);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> Instance3D {
        let mut model = glam::Mat4::from_translation(glam::Vec3::new(x, 0.0, 0.0)).to_cols_array_2d();
        model[0][3] = 3.0; // decal layers
        Instance3D { model, color: [1.0; 4] }
    }

    fn enabled() -> Motion {
        let mut motion = Motion {
            enabled: true,
            ..Default::default()
        };
        // The frame before: motion was on.
        motion.begin();
        motion.finish();
        motion
    }

    #[test]
    fn only_models_whose_matrix_changed_are_listed() {
        let mut motion = enabled();
        motion.begin();
        motion.moved(5, &at(0.0), at(0.0).model);
        motion.moved(5, &at(1.0), at(0.0).model);
        motion.moved(4, &at(2.0), at(0.0).model);
        motion.finish();
        // The second and third moved; the first did not.
        assert_eq!(motion.runs, vec![(4, 0, 1), (5, 1, 1)]);
        assert_eq!(motion.instances[1].previous[3][0], 0.0);
        assert_eq!(motion.instances[1].instance.model[3][0], 1.0);
        assert_eq!(motion.instances[1].previous[0][3], 3.0);
    }

    #[test]
    fn new_decal_layers_alone_are_not_motion() {
        let mut motion = enabled();
        let mut before = at(1.0);
        before.model[0][3] = 9.0;
        motion.begin();
        motion.moved(5, &at(1.0), before.model);
        motion.finish();
        assert!(motion.runs.is_empty());
    }

    #[test]
    fn rotation_animation_preserves_the_previous_matrix() {
        let mut motion = enabled();
        let first = at(0.0);
        let mut rotated = first;
        rotated.model = glam::Mat4::from_rotation_y(0.7).to_cols_array_2d();
        rotated.model[0][3] = first.model[0][3];
        motion.begin();
        motion.moved(2, &rotated, first.model);
        motion.finish();
        assert_eq!(motion.instances.len(), 1);
        assert_eq!(motion.instances[0].previous, first.model);
        assert_eq!(motion.instances[0].instance.model, rotated.model);
    }

    #[test]
    fn a_reset_frame_lists_no_motion_and_the_next_does_again() {
        let mut motion = enabled();
        // (where the model is, whether the frame resets, whether it moved)
        let mut before = 0.0;
        for (x, reset, moved) in [(1.0, false, true), (5.0, true, false), (6.0, false, true)] {
            motion.reset = reset;
            motion.begin();
            motion.moved(0, &at(x), at(before).model);
            motion.finish();
            assert_eq!(!motion.runs.is_empty(), moved, "x {x}, reset {reset}");
            before = x;
        }
        assert_eq!(motion.instances[0].previous[3][0], 5.0);
    }

    #[test]
    fn the_frame_motion_is_turned_on_has_no_past() {
        let mut motion = Motion::default();
        motion.begin();
        motion.moved(0, &at(1.0), at(0.0).model);
        motion.finish();
        assert!(motion.runs.is_empty(), "off: nothing is listed");
        motion.enabled = true;
        motion.begin();
        assert!(!motion.tracking());
        motion.moved(0, &at(2.0), at(1.0).model);
        motion.finish();
        assert!(motion.runs.is_empty());
        motion.begin();
        motion.moved(0, &at(3.0), at(2.0).model);
        motion.finish();
        assert_eq!(motion.runs, vec![(0, 0, 1)]);
    }
}
