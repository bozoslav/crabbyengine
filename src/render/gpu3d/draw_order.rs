//! The order models are drawn in, and how many GPU state changes that costs.
//!
//! The frame's runs come from collection in mesh order. Passes that only
//! keep the nearest depth (shadow maps, depth prepasses) or that test
//! against a finished prepass draw the same pixels in any order, so they
//! draw the runs sorted by the state each draw binds: pipeline, then
//! material, then mesh. Neighbouring runs then share pipelines and material
//! bind groups instead of switching back and forth. The transparent pass
//! blends, so it keeps the collected order of the runs, and draws each
//! run's models far to near: the frame keeps a model's place in its mesh's
//! instances while the camera sees it, so their order there says nothing
//! about where they are, and changes as other models come and go.

use super::{FADED, Instance3D, Renderer3D, Run};
use std::sync::atomic::{AtomicU32, Ordering};

/// GPU state changes and draws the model passes recorded in a frame
/// (counted from the commands the renderer issues, before wgpu's own
/// redundancy checks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StateChanges {
    pub pipelines: u32,
    pub bind_groups: u32,
    pub vertex_buffers: u32,
    pub index_buffers: u32,
    pub draws: u32,
}

/// [`StateChanges`] added up across the passes, which record on different
/// threads; each pass adds its own counts once at its end.
#[derive(Default)]
pub(super) struct StateCounters([AtomicU32; 5]);

impl StateCounters {
    pub fn add(&self, c: &StateChanges) {
        let all = [c.pipelines, c.bind_groups, c.vertex_buffers, c.index_buffers, c.draws];
        for (counter, n) in self.0.iter().zip(all) {
            if n > 0 {
                counter.fetch_add(n, Ordering::Relaxed);
            }
        }
    }

    #[cfg(test)]
    pub fn take(&self) -> StateChanges {
        let [pipelines, bind_groups, vertex_buffers, index_buffers, draws] =
            std::array::from_fn(|i| self.0[i].swap(0, Ordering::Relaxed));
        StateChanges {
            pipelines,
            bind_groups,
            vertex_buffers,
            index_buffers,
            draws,
        }
    }
}

impl Renderer3D {
    /// What a run's opaque draws bind, as a sort key: its first solid
    /// part's pipeline (shader, cut-out, sides, dither, per-model storage),
    /// then that part's material, then the mesh. A run with a material
    /// override is keyed by the override. Runs with only transparent parts,
    /// which these passes skip, go last.
    fn state_key(&self, &(mesh, _, _): &Run) -> (u32, u32, u32) {
        let fading = mesh & FADED != 0;
        let base = &self.meshes[self.base(mesh) as usize];
        let solid = base
            .parts
            .iter()
            .map(|&(_, m)| self.part_material(mesh, m))
            .map(|m| (m, &self.materials[m as usize]))
            .find(|(_, m)| !m.transparent);
        let Some((id, m)) = solid else {
            return (u32::MAX, u32::MAX, mesh);
        };
        let pipeline = (m.shader << 4)
            | u32::from(m.cutout) << 3
            | u32::from(m.double_sided) << 2
            | u32::from(fading) << 1
            | u32::from(base.gpu_buffer.is_some());
        (pipeline, id, mesh)
    }

    /// Sorts `runs` by the state their draws bind (see [`Self::state_key`]),
    /// with `keys` as reusable scratch space.
    pub(super) fn sort_by_state(&self, runs: &mut [Run], keys: &mut Vec<((u32, u32, u32), Run)>) {
        if !self.group_draws || runs.len() < 2 {
            return;
        }
        keys.clear();
        keys.extend(runs.iter().map(|r| (self.state_key(r), *r)));
        // Mesh ids (with the fade bit) are unique within a list of runs, so
        // the keys are too and the unstable sort's order is deterministic.
        keys.sort_unstable_by_key(|&(key, _)| key);
        for (run, &(_, sorted)) in runs.iter_mut().zip(keys.iter()) {
            *run = sorted;
        }
    }

    /// Fills `solid_runs` and sorts the shadow views' and target cameras'
    /// runs (none of which blend) by state. After skinning's prepare, as
    /// skinned models' ids name its draws.
    pub(super) fn order_runs(&mut self) {
        let mut keys = std::mem::take(&mut self.sort_keys);
        let mut solid = std::mem::take(&mut self.solid_runs);
        solid.clear();
        solid.extend_from_slice(&self.runs);
        self.sort_by_state(&mut solid, &mut keys);
        self.solid_runs = solid;
        for view in super::shadow_views(self.cascades, self.light_views) {
            let mut runs = std::mem::take(&mut self.shadow_runs[view]);
            self.sort_by_state(&mut runs, &mut keys);
            self.shadow_runs[view] = runs;
        }
        let mut views = std::mem::take(&mut self.views);
        for (_, runs, _) in &mut views {
            self.sort_by_state(runs, &mut keys);
        }
        self.views = views;
        self.sort_keys = keys;
    }

    /// Whether a run draws any part with a transparent material.
    fn blends(&self, mesh: u32) -> bool {
        self.meshes[self.base(mesh) as usize]
            .parts
            .iter()
            .any(|&(_, m)| self.materials[self.part_material(mesh, m) as usize].transparent)
    }

    /// Fills `blended_runs` with the camera's runs that blend, each drawing
    /// a copy (in `blended`) of its instances sorted far to near from `eye`,
    /// by the models' origins. Models as far as each other are ordered by
    /// their instances' bytes, so the same models always draw the same
    /// picture, however the frame holds them. `base`: where `blended`
    /// starts in the instance buffer.
    pub(super) fn sort_blended(&mut self, instances: &[Instance3D], eye: glam::Vec3, base: u32) {
        let mut runs = std::mem::take(&mut self.blended_runs);
        let mut sorted = std::mem::take(&mut self.blended);
        let mut keys = std::mem::take(&mut self.blend_keys);
        runs.clear();
        sorted.clear();
        for &(mesh, start, count) in &self.runs {
            if !self.blends(mesh) {
                continue;
            }
            let run = &instances[start as usize..(start + count) as usize];
            keys.clear();
            keys.extend(run.iter().enumerate().map(|(i, instance)| {
                let [x, y, z, _] = instance.model[3];
                (glam::Vec3::new(x, y, z).distance_squared(eye), i as u32)
            }));
            let bytes = |i: u32| bytemuck::bytes_of(&run[i as usize]);
            keys.sort_unstable_by(|a, b| b.0.total_cmp(&a.0).then_with(|| bytes(a.1).cmp(bytes(b.1))));
            runs.push((mesh, base + sorted.len() as u32, count));
            sorted.extend(keys.iter().map(|&(_, i)| run[i as usize]));
        }
        self.blended_runs = runs;
        self.blended = sorted;
        self.blend_keys = keys;
    }

    /// The state changes and draws the model passes recorded since the last
    /// call (for tests and benchmarks).
    #[cfg(test)]
    pub(crate) fn take_state_changes(&self) -> StateChanges {
        self.state_counters.take()
    }
}
