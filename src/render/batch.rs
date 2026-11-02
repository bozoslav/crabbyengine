//! CPU side of sprite batching: collect draw calls, order them, split into
//! per-texture batches. No GPU types here, so it is unit tested directly.
//!
//! Ordering rule: higher `z` draws on top; for equal `z`, later draws go on
//! top (painter's order). Consecutive sprites with the same texture become
//! one instanced draw call.

use bytemuck::{Pod, Zeroable};

/// One sprite as the GPU sees it (one instance of a 6-vertex quad).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    /// Where the pivot lands, in world units.
    pub pos: [f32; 2],
    /// Final size in world units (texture size × scale; may be negative to flip).
    pub size: [f32; 2],
    /// Pivot inside the sprite, 0..1 (0.5, 0.5 = center).
    pub pivot: [f32; 2],
    /// `cos(rotation)`, `sin(rotation)`, precomputed on the CPU.
    pub rot: [f32; 2],
    /// Texture region: `u0, v0, du, dv`.
    pub uv: [f32; 4],
    /// sRGB tint, 8 bits per channel.
    pub color: [u8; 4],
}

/// A run of instances that share one texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Batch {
    pub texture: u32,
    pub start: u32,
    pub end: u32,
}

/// Draw commands for one frame. All buffers are reused between frames,
/// so steady-state frames don't allocate.
#[derive(Debug, Default)]
pub(crate) struct DrawList {
    instances: Vec<SpriteInstance>,
    textures: Vec<u32>,
    zs: Vec<f32>,
    /// True once two different `z` values have been pushed this frame.
    needs_sort: bool,

    // Scratch space for the sorted path.
    order: Vec<(u32, u32)>,
    sorted: Vec<SpriteInstance>,
    batches: Vec<Batch>,
}

/// Maps an `f32` to a `u32` whose unsigned order matches the float order.
fn z_key(z: f32) -> u32 {
    let bits = z.to_bits();
    if bits >> 31 == 1 {
        !bits
    } else {
        bits | 0x8000_0000
    }
}

impl DrawList {
    pub fn push(&mut self, texture: u32, z: f32, instance: SpriteInstance) {
        if let Some(&first_z) = self.zs.first()
            && first_z != z
        {
            self.needs_sort = true;
        }
        self.instances.push(instance);
        self.textures.push(texture);
        self.zs.push(z);
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.textures.clear();
        self.zs.clear();
        self.needs_sort = false;
        self.batches.clear();
    }

    /// Orders the frame's sprites and groups them into batches.
    /// Returns the instance data to upload and the draw ranges.
    pub fn finish(&mut self) -> (&[SpriteInstance], &[Batch]) {
        self.batches.clear();

        if !self.needs_sort {
            // Fast path (the common case): everything shares one z, so
            // submission order is already the draw order.
            Self::build_batches(&mut self.batches, self.textures.iter().copied());
            return (&self.instances, &self.batches);
        }

        // (z, submission index) is unique, so an unstable sort is deterministic
        // and doesn't allocate.
        self.order.clear();
        self.order.extend(
            self.zs
                .iter()
                .enumerate()
                .map(|(i, &z)| (z_key(z), i as u32)),
        );
        self.order.sort_unstable();

        self.sorted.clear();
        self.sorted
            .extend(self.order.iter().map(|&(_, i)| self.instances[i as usize]));
        let textures = &self.textures;
        Self::build_batches(
            &mut self.batches,
            self.order.iter().map(|&(_, i)| textures[i as usize]),
        );
        (&self.sorted, &self.batches)
    }

    fn build_batches(batches: &mut Vec<Batch>, textures: impl Iterator<Item = u32>) {
        for (i, texture) in textures.enumerate() {
            let i = i as u32;
            match batches.last_mut() {
                Some(b) if b.texture == texture => b.end = i + 1,
                _ => batches.push(Batch {
                    texture,
                    start: i,
                    end: i + 1,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(tag: f32) -> SpriteInstance {
        SpriteInstance {
            pos: [tag, 0.0],
            size: [1.0, 1.0],
            pivot: [0.5, 0.5],
            rot: [1.0, 0.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [255; 4],
        }
    }

    fn tags(instances: &[SpriteInstance]) -> Vec<f32> {
        instances.iter().map(|i| i.pos[0]).collect()
    }

    #[test]
    fn same_texture_is_one_batch() {
        let mut list = DrawList::default();
        for i in 0..1000 {
            list.push(7, 0.0, inst(i as f32));
        }
        let (instances, batches) = list.finish();
        assert_eq!(instances.len(), 1000);
        assert_eq!(
            batches,
            &[Batch {
                texture: 7,
                start: 0,
                end: 1000
            }]
        );
    }

    #[test]
    fn texture_changes_split_batches_in_submission_order() {
        let mut list = DrawList::default();
        list.push(1, 0.0, inst(0.0));
        list.push(1, 0.0, inst(1.0));
        list.push(2, 0.0, inst(2.0));
        list.push(1, 0.0, inst(3.0));
        let (instances, batches) = list.finish();
        assert_eq!(tags(instances), [0.0, 1.0, 2.0, 3.0]);
        assert_eq!(
            batches,
            &[
                Batch {
                    texture: 1,
                    start: 0,
                    end: 2
                },
                Batch {
                    texture: 2,
                    start: 2,
                    end: 3
                },
                Batch {
                    texture: 1,
                    start: 3,
                    end: 4
                },
            ]
        );
    }

    #[test]
    fn higher_z_draws_later_and_ties_keep_submission_order() {
        let mut list = DrawList::default();
        list.push(1, 5.0, inst(0.0));
        list.push(1, -2.0, inst(1.0));
        list.push(2, 5.0, inst(2.0));
        list.push(1, 0.0, inst(3.0));
        list.push(1, -2.0, inst(4.0));
        let (instances, batches) = list.finish();
        assert_eq!(tags(instances), [1.0, 4.0, 3.0, 0.0, 2.0]);
        assert_eq!(
            batches,
            &[
                Batch {
                    texture: 1,
                    start: 0,
                    end: 4
                },
                Batch {
                    texture: 2,
                    start: 4,
                    end: 5
                },
            ]
        );
    }

    #[test]
    fn clear_resets_for_next_frame() {
        let mut list = DrawList::default();
        list.push(1, 1.0, inst(0.0));
        list.push(1, 2.0, inst(1.0));
        list.clear();
        assert_eq!(list.len(), 0);
        list.push(3, 9.0, inst(0.0));
        let (instances, batches) = list.finish();
        assert_eq!(instances.len(), 1);
        assert_eq!(
            batches,
            &[Batch {
                texture: 3,
                start: 0,
                end: 1
            }]
        );
    }

    #[test]
    fn z_key_orders_like_floats() {
        let zs = [-100.0, -1.5, -0.0, 0.0, 0.25, 3.0, 1e9];
        for w in zs.windows(2) {
            assert!(z_key(w[0]) <= z_key(w[1]), "{} vs {}", w[0], w[1]);
        }
    }
}
