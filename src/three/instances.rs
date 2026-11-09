//! Persistent static geometry, partitioned into bounded spatial cells.
use super::{Mesh, collect::Instance3D};
use crate::{Color, Ctx, Mat4, Vec3};

/// A persistent batch. Handles are never recycled; freeing twice is harmless.
/// Create one batch per streamed region and free it when that region unloads.
/// ```
/// use ponko::*;
/// Game::new("instances", 1, 1).run_headless(0, |ctx| {
///     let mat = ctx.create_material(MaterialDesc::default());
///     let mesh = ctx.create_mesh(&MeshData::cube(1.0), mat);
///     let batch = ctx.create_instance_batch(mesh, &[Mat4::IDENTITY], InstanceBatchOptions::default());
///     ctx.free_instance_batch(batch);
/// });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InstanceBatch {
    pub(crate) id: u32,
}
impl InstanceBatch {
    /// Raw handle for language bindings.
    pub fn id(self) -> u32 {
        self.id
    }
    /// Raw handle conversion; operations validate liveness.
    pub fn from_id(id: u32) -> Self {
        Self { id }
    }
}
/// Settings for a batch; materials come from the mesh's parts.
/// ```
/// use ponko::*;
/// let settings = InstanceBatchOptions { max_distance: 500.0, ..Default::default() };
/// assert!(settings.cast_shadows);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct InstanceBatchOptions {
    /// Width of each spatial cell in world units. Must be finite and positive.
    pub cell_size: f32,
    /// Maximum distance from this view's camera or shadow-casting light to
    /// an instance's bounding sphere. Sun cascades use the main camera.
    /// Positive infinity disables distance culling.
    pub max_distance: f32,
    /// Whether the batch casts sun, point-light and spot-light shadows.
    pub cast_shadows: bool,
    /// Uniform tint, in sRGB like `Model::with_color`.
    pub tint: Color,
}
impl Default for InstanceBatchOptions {
    fn default() -> Self {
        Self {
            cell_size: 32.0,
            max_distance: f32::INFINITY,
            cast_shadows: true,
            tint: Color::WHITE,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Cell {
    pub min: Vec3,
    pub max: Vec3,
    pub start: u32,
    pub count: u32,
}
pub(crate) struct Batch {
    pub mesh: Mesh,
    pub options: InstanceBatchOptions,
    pub instances: Vec<Instance3D>,
    pub spheres: Vec<[f32; 4]>,
    pub cells: Vec<Cell>,
}
impl Batch {
    fn new(mesh: Mesh, transforms: &[Mat4], options: InstanceBatchOptions, bounds: (Vec3, f32)) -> Self {
        assert!(
            options.cell_size.is_finite() && options.cell_size > 0.0,
            "invalid cell size"
        );
        assert!(options.max_distance >= 0.0, "invalid maximum distance");
        assert!(transforms.len() <= 16_777_216, "too many instances");
        let mut sorted: Vec<_> = transforms
            .iter()
            .map(|&m| {
                assert!(
                    m.is_finite() && m.row(3) == crate::Vec4::W,
                    "instances require finite affine transforms"
                );
                let center = m.transform_point3(bounds.0);
                let radius = bounds.1 * Vec3::new(m.x_axis.length(), m.y_axis.length(), m.z_axis.length()).length();
                let key = (center / options.cell_size).floor().as_ivec3().to_array();
                (key, m, center, radius)
            })
            .collect();
        sorted.sort_by_key(|e| e.0);
        let mut batch = Self {
            mesh,
            options,
            instances: Vec::with_capacity(sorted.len()),
            spheres: Vec::with_capacity(sorted.len()),
            cells: Vec::new(),
        };
        let mut previous = None;
        for (key, matrix, center, radius) in sorted {
            if previous != Some(key) || batch.cells.last().is_some_and(|c| c.count == 256) {
                batch.cells.push(Cell {
                    min: center - radius,
                    max: center + radius,
                    start: batch.instances.len() as u32,
                    count: 0,
                });
                previous = Some(key);
            }
            let cell = batch.cells.last_mut().unwrap();
            cell.min = cell.min.min(center - radius);
            cell.max = cell.max.max(center + radius);
            cell.count += 1;
            let mut model = matrix.to_cols_array_2d();
            model[0][3] = super::projected::receiver_mask(None);
            batch.instances.push(Instance3D {
                model,
                color: super::linear(options.tint),
            });
            batch.spheres.push(center.extend(radius).to_array());
        }
        batch
    }
}
impl Ctx {
    /// Uploads transforms once; the mesh retains its normal materials.
    /// See [`InstanceBatch`] for an example. Empty batches are valid.
    pub fn create_instance_batch(
        &mut self,
        mesh: Mesh,
        transforms: &[Mat4],
        options: InstanceBatchOptions,
    ) -> InstanceBatch {
        let bounds = self.assets3d.live_bounds(mesh.id).expect("invalid mesh");
        let batch = Batch::new(mesh, transforms, options, bounds);
        let id = self.assets3d.instance_batches.len() as u32;
        if let Some(r) = self.renderer.as_mut() {
            r.set_instance_batch(id, Some(&batch));
        }
        self.assets3d.instance_batches.push(Some(batch));
        InstanceBatch { id }
    }
    /// Replaces transforms and rebuilds cells. Returns false for a freed handle.
    /// See [`InstanceBatch`].
    pub fn update_instance_batch(&mut self, batch: InstanceBatch, transforms: &[Mat4]) -> bool {
        let Some(Some(old)) = self.assets3d.instance_batches.get(batch.id as usize) else {
            return false;
        };
        let Some(bounds) = self.assets3d.live_bounds(old.mesh.id) else {
            return false;
        };
        let new = Batch::new(old.mesh, transforms, old.options, bounds);
        if let Some(r) = self.renderer.as_mut() {
            r.set_instance_batch(batch.id, Some(&new));
        }
        self.assets3d.instance_batches[batch.id as usize] = Some(new);
        true
    }
    /// Releases a region's CPU and GPU resources. See [`InstanceBatch`].
    pub fn free_instance_batch(&mut self, batch: InstanceBatch) {
        if let Some(slot) = self.assets3d.instance_batches.get_mut(batch.id as usize) {
            *slot = None;
            if let Some(r) = self.renderer.as_mut() {
                r.set_instance_batch(batch.id, None);
            }
        }
    }
    /// Returns the instance count, or None for a freed handle. See [`InstanceBatch`].
    pub fn instance_batch_len(&self, batch: InstanceBatch) -> Option<usize> {
        self.assets3d
            .instance_batches
            .get(batch.id as usize)?
            .as_ref()
            .map(|b| b.instances.len())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dense_cells_split_into_bounded_clusters() {
        let b = Batch::new(
            Mesh { id: 0 },
            &vec![Mat4::IDENTITY; 1000],
            Default::default(),
            (Vec3::ZERO, 1.0),
        );
        assert_eq!(b.cells.len(), 4);
        assert!(b.cells.iter().all(|c| c.count <= 256));
        assert_eq!(b.cells.iter().map(|c| c.count).sum::<u32>(), 1000);
    }
    #[test]
    fn cells_contain_transformed_bounds_and_partition_instances() {
        let matrices = [-33.0, -1.0, 0.0, 2.0, 40.0].map(|x| Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
        let b = Batch::new(Mesh { id: 0 }, &matrices, Default::default(), (Vec3::ZERO, 1.0));
        assert_eq!(b.cells.len(), 4);
        assert_eq!(b.cells.iter().map(|c| c.count).sum::<u32>(), 5);
        for c in &b.cells {
            for s in &b.spheres[c.start as usize..(c.start + c.count) as usize] {
                let p = Vec3::from_slice(s);
                assert!((p - s[3]).cmpge(c.min).all() && (p + s[3]).cmple(c.max).all());
            }
        }
    }
}
