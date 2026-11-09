//! Which models moved since the last frame, for temporal reprojection: the
//! renderer draws their motion vectors from the model matrix they had then.
//! Everything else moves only with the camera, which the renderer works out
//! from depth.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::collect::Instance3D;

/// Hashes entity ids (already well spread in their low bits) with one
/// multiplication: the map is rebuilt every frame.
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

type Models = HashMap<u64, [[f32; 4]; 4], BuildHasherDefault<IdHasher>>;

/// A model that moved: the instance as drawn this frame, then the model
/// matrix it had in the last. The renderer reads it as a vertex buffer
/// (instance attributes, then these four columns).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MovedInstance {
    pub instance: Instance3D,
    pub previous: [[f32; 4]; 4],
}

/// Remembers every drawn model's matrix for one frame, and lists the models
/// whose matrix changed. Does nothing (and keeps nothing) until `enabled`.
#[derive(Default)]
pub(crate) struct Motion {
    /// Whether TAA or temporal AO is on: set before `Frame3D::collect`.
    pub enabled: bool,
    /// Whether the game asked to forget the history (a camera cut).
    pub reset: bool,
    last: Models,
    now: Models,
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
        if !self.enabled {
            self.last.clear();
            self.now.clear();
            return;
        }
        std::mem::swap(&mut self.last, &mut self.now);
        self.now.clear();
        // After a reset the last frame is no past of this one: a model
        // teleported with it has no motion.
        if self.reset {
            self.last.clear();
        }
    }

    /// Notes that the camera sees model `key` as `instance`, drawing `mesh`.
    pub fn note(&mut self, key: u64, mesh: u32, instance: &Instance3D) {
        if !self.enabled || key == NO_KEY {
            return;
        }
        self.now.insert(key, instance.model);
        if let Some(&before) = self.last.get(&key) {
            // The first slot of the matrix carries the decal layers: not motion.
            let mut previous = before;
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

    #[test]
    fn only_models_whose_matrix_changed_are_listed() {
        let mut motion = Motion {
            enabled: true,
            ..Default::default()
        };
        for x in [0.0, 1.0] {
            motion.begin();
            motion.note(1, 5, &at(0.0));
            motion.note(2, 5, &at(x));
            motion.note(3, 4, &at(x * 2.0));
            motion.note(NO_KEY, 4, &at(x * 3.0));
            motion.finish();
        }
        // Models 2 and 3 moved; model 1 and the keyless one did not.
        assert_eq!(motion.runs, vec![(4, 0, 1), (5, 1, 1)]);
        assert_eq!(motion.instances[1].previous[3][0], 0.0);
        assert_eq!(motion.instances[1].instance.model[3][0], 1.0);
        assert_eq!(motion.instances[1].previous[0][3], 3.0);
    }

    #[test]
    fn rotation_animation_preserves_the_previous_matrix() {
        let mut motion = Motion {
            enabled: true,
            ..Default::default()
        };
        let first = at(0.0);
        let mut rotated = first;
        rotated.model = glam::Mat4::from_rotation_y(0.7).to_cols_array_2d();
        rotated.model[0][3] = first.model[0][3];
        for instance in [&first, &rotated] {
            motion.begin();
            motion.note(7, 2, instance);
            motion.finish();
        }
        assert_eq!(motion.instances.len(), 1);
        assert_eq!(motion.instances[0].previous, first.model);
        assert_eq!(motion.instances[0].instance.model, rotated.model);
    }

    #[test]
    fn a_reset_frame_lists_no_motion_and_the_next_does_again() {
        let mut motion = Motion {
            enabled: true,
            ..Default::default()
        };
        // (where the model is, whether the frame resets, whether it moved)
        for (x, reset, moved) in [
            (0.0, false, false),
            (1.0, false, true),
            (5.0, true, false),
            (6.0, false, true),
        ] {
            motion.reset = reset;
            motion.begin();
            motion.note(1, 0, &at(x));
            motion.finish();
            assert_eq!(!motion.runs.is_empty(), moved, "x {x}, reset {reset}");
        }
        assert_eq!(motion.instances[0].previous[3][0], 5.0);
    }

    #[test]
    fn a_first_sighting_has_no_motion_and_disabled_keeps_nothing() {
        let mut motion = Motion::default();
        motion.begin();
        motion.note(1, 0, &at(0.0));
        motion.finish();
        assert!(motion.now.is_empty());
        motion.enabled = true;
        motion.begin();
        motion.note(1, 0, &at(2.0));
        motion.finish();
        assert!(motion.runs.is_empty());
        motion.begin();
        motion.note(1, 0, &at(3.0));
        motion.finish();
        assert_eq!(motion.runs, vec![(0, 0, 1)]);
    }
}
