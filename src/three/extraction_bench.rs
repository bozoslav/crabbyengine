//! A headless handle on the 3D frame collection, for `ponko_bench` and
//! tests: the same `Frame3D::collect` a drawn frame runs, without a GPU.
//! Not part of the public API.

use glam::Vec3;

use super::collect::{Frame3D, INSTANCE_BRIDGE, Instance3D, shadow_views};
use super::{Assets3D, Camera3D, Lighting, Mesh, SkinFrame};
use crate::World;

/// Collects a world's 3D frame the way the engine does before drawing, and
/// measures what that costs: see [`ExtractionSample`].
#[doc(hidden)]
#[derive(Default)]
pub struct ExtractionBench {
    frame: Frame3D,
    assets: Assets3D,
    skins: SkinFrame,
    /// What the GPU's instance buffer would hold: the last synced frame.
    mirror: Vec<Instance3D>,
}

/// What one collected frame holds.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtractionSample {
    /// Instances the camera draws.
    pub visible: usize,
    /// Instances the shadow views draw.
    pub shadow: usize,
    /// Draw runs, the camera's and the shadow views' together.
    pub runs: usize,
    /// Bytes the collection keeps between frames (its buffers' capacity).
    pub state_bytes: usize,
}

impl ExtractionBench {
    /// A collection for a world whose models use meshes `0..meshes`, each
    /// with a unit bounding sphere around its origin.
    #[must_use]
    pub fn new(meshes: u32) -> Self {
        Self {
            assets: Assets3D {
                bounds: vec![(Vec3::ZERO, 1.0); meshes as usize],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// The handle of mesh `id`, for [`Model::new`](super::Model::new).
    #[must_use]
    pub fn mesh(&self, id: u32) -> Mesh {
        assert!((id as usize) < self.assets.bounds.len(), "no mesh {id}");
        Mesh { id }
    }

    /// A camera for a `width` x `height` view, as a new game's is.
    #[must_use]
    pub fn camera(width: f32, height: f32) -> Camera3D {
        Camera3D::new(glam::Vec2::new(width, height))
    }

    /// Collects one frame.
    pub fn collect(&mut self, world: &mut World, camera: &Camera3D, lighting: &Lighting) -> ExtractionSample {
        self.frame.collect(world, &self.assets, &self.skins, camera, lighting);
        let count = |runs: &[(u32, u32, u32)]| runs.iter().map(|r| r.2 as usize).sum::<usize>();
        let views = shadow_views(self.frame.cascades, self.frame.light_views);
        ExtractionSample {
            visible: count(&self.frame.runs),
            shadow: views.clone().map(|v| count(&self.frame.shadow_runs[v])).sum(),
            runs: self.frame.runs.len() + views.map(|v| self.frame.shadow_runs[v].len()).sum::<usize>(),
            state_bytes: self.frame.extraction_bytes(),
        }
    }

    /// Brings the stand-in for the GPU's instance buffer up to the last
    /// collected frame, as the renderer does, and returns the bytes that
    /// uploads: every instance that differs from the frame before, with the
    /// few unchanged ones between two of them.
    pub fn sync_uploads(&mut self) -> usize {
        let new = &self.frame.instances;
        let same = |i: usize| {
            (self.mirror.get(i)).is_some_and(|before| bytemuck::bytes_of(before) == bytemuck::bytes_of(&new[i]))
        };
        // The end of the last changed instance's run, and what is written.
        let (mut last, mut written) = (None, 0);
        for i in (0..new.len()).filter(|&i| !same(i)) {
            written += match last {
                Some(last) if i - last <= INSTANCE_BRIDGE => i + 1 - last,
                _ => 1,
            };
            last = Some(i + 1);
        }
        if self.mirror.len() < new.len() {
            self.mirror.resize(new.len(), Instance3D::default());
        }
        self.mirror[..new.len()].copy_from_slice(new);
        written * size_of::<Instance3D>()
    }
}
