//! Persistent static geometry, partitioned into bounded spatial cells.
use super::{Mesh, collect::Instance3D};
use crate::gfx::Graphics;
use crate::{Color, Mat4, Vec3};

/// A persistent batch. Handles are never recycled; freeing twice is harmless.
/// Create one batch per streamed region and free it when that region unloads.
/// ```
/// use ponko::prelude::*;
/// Game::new("instances", 1, 1).run_headless(0, |ctx| {
///     let mat = ctx.gfx.create_material(ponko::render::MaterialDesc::default());
///     let mesh = ctx.gfx.create_mesh(&ponko::render::MeshData::cube(1.0), mat);
///     let batch = ctx.gfx.create_instance_batch(mesh, &[Mat4::IDENTITY], ponko::render::InstanceBatchOptions::default());
///     ctx.gfx.free(batch);
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
/// use ponko::prelude::*;
/// let settings = ponko::render::InstanceBatchOptions { max_distance: 500.0, ..Default::default() };
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
/// A mesh drawn in a half-open distance band, measured to the instance origin.
/// Overlapping bands draw both levels. Sun views use the main camera; local
/// shadows use the light position. Mesh shadow settings still apply.
#[derive(Clone, Copy, Debug)]
pub struct InstanceLevel {
    /// Mesh and materials for this level. Must outlive the batch.
    pub mesh: Mesh,
    /// Inclusive, finite, nonnegative lower distance.
    pub min_distance: f32,
    /// Exclusive upper distance; positive infinity is allowed.
    pub max_distance: f32,
    /// Draw in the main camera and target cameras.
    pub camera: bool,
    /// Cast sun and local-light shadows (also requires `cast_shadows`).
    pub shadows: bool,
}
impl InstanceLevel {
    pub(crate) fn contains(&self, distance: f32) -> bool {
        distance >= self.min_distance && distance < self.max_distance
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
    pub levels: Vec<InstanceLevel>,
    pub legacy_distance: bool,
    pub min: Vec3,
    pub max: Vec3,
    pub options: InstanceBatchOptions,
    pub instances: Vec<Instance3D>,
    pub spheres: Vec<[f32; 4]>,
    pub cells: Vec<Cell>,
}
impl Batch {
    fn new(mesh: Mesh, transforms: &[Mat4], options: InstanceBatchOptions, bounds: (Vec3, f32)) -> Self {
        Self::levels(
            &[InstanceLevel {
                mesh,
                min_distance: 0.0,
                max_distance: options.max_distance,
                camera: true,
                shadows: true,
            }],
            transforms,
            &[],
            options,
            bounds,
            true,
        )
    }
    fn levels(
        levels: &[InstanceLevel],
        transforms: &[Mat4],
        colors: &[Color],
        options: InstanceBatchOptions,
        bounds: (Vec3, f32),
        legacy_distance: bool,
    ) -> Self {
        assert!(
            colors.is_empty() || colors.len() == transforms.len(),
            "instance colors must be empty or match transform count"
        );
        assert!(
            options.cell_size.is_finite() && options.cell_size > 0.0,
            "invalid cell size"
        );
        assert!(options.max_distance >= 0.0, "invalid maximum distance");
        assert!(transforms.len() <= 16_777_216, "too many instances");
        let mut sorted: Vec<_> = transforms
            .iter()
            .enumerate()
            .map(|(index, &m)| {
                assert!(
                    m.is_finite() && m.row(3) == crate::Vec4::W,
                    "instances require finite affine transforms"
                );
                let center = m.transform_point3(bounds.0);
                let radius = bounds.1 * Vec3::new(m.x_axis.length(), m.y_axis.length(), m.z_axis.length()).length();
                let key = (center / options.cell_size).floor().as_ivec3().to_array();
                (
                    key,
                    m,
                    center,
                    radius,
                    colors.get(index).copied().unwrap_or(options.tint),
                )
            })
            .collect();
        sorted.sort_by_key(|e| e.0);
        let mut batch = Self {
            levels: levels.to_vec(),
            legacy_distance,
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
            options,
            instances: Vec::with_capacity(sorted.len()),
            spheres: Vec::with_capacity(sorted.len()),
            cells: Vec::new(),
        };
        let mut previous = None;
        for (key, matrix, center, radius, color) in sorted {
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
            if !legacy_distance {
                // Origin bounds keep broad distance rejection conservative even
                // for off-centre meshes and shadow proxies.
                cell.min = cell.min.min(matrix.w_axis.truncate());
                cell.max = cell.max.max(matrix.w_axis.truncate());
            }
            batch.min = batch.min.min(cell.min);
            batch.max = batch.max.max(cell.max);
            cell.count += 1;
            let mut model = matrix.to_cols_array_2d();
            model[0][3] = super::projected::receiver_mask(None);
            batch.instances.push(Instance3D {
                model,
                color: super::linear(color),
            });
            batch.spheres.push(center.extend(radius).to_array());
        }
        batch
    }
}
impl Graphics {
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
        self.assets3d.instance_batch_revision = self.assets3d.instance_batch_revision.wrapping_add(1);
        InstanceBatch { id }
    }
    /// Creates levels sharing one transform/colour population. Bands use origin
    /// distance, independently of `options.max_distance` (legacy API only).
    /// Empty colours use `options.tint`; otherwise one sRGB colour is required
    /// per transform. Cell width and `cast_shadows` apply to every level.
    pub fn create_instance_batch_levels(
        &mut self,
        levels: &[InstanceLevel],
        transforms: &[Mat4],
        colors: &[Color],
        options: InstanceBatchOptions,
    ) -> InstanceBatch {
        let bounds = self.instance_level_bounds(levels).expect("invalid instance level mesh");
        let batch = Batch::levels(levels, transforms, colors, options, bounds, false);
        let id = self.assets3d.instance_batches.len() as u32;
        if let Some(r) = self.renderer.as_mut() {
            r.set_instance_batch(id, Some(&batch));
        }
        self.assets3d.instance_batches.push(Some(batch));
        self.assets3d.instance_batch_revision = self.assets3d.instance_batch_revision.wrapping_add(1);
        InstanceBatch { id }
    }
    fn instance_level_bounds(&self, levels: &[InstanceLevel]) -> Option<(Vec3, f32)> {
        assert!(!levels.is_empty(), "instance batch requires at least one level");
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for level in levels {
            assert!(
                level.min_distance.is_finite() && level.min_distance >= 0.0 && level.max_distance > level.min_distance,
                "invalid instance distance band"
            );
            let (center, radius) = self.assets3d.live_bounds(level.mesh.id)?;
            lo = lo.min(center - radius);
            hi = hi.max(center + radius);
        }
        Some(((lo + hi) * 0.5, (hi - lo).length() * 0.5))
    }
    /// Replaces transforms and rebuilds cells. Returns false for a freed handle.
    /// See [`InstanceBatch`].
    pub fn update_instance_batch(&mut self, batch: InstanceBatch, transforms: &[Mat4]) -> bool {
        self.update_instance_batch_colors(batch, transforms, &[])
    }
    /// Replaces transforms and sRGB colours, rebuilding cells. Empty colours
    /// restore the uniform tint, as does `update_instance_batch`.
    /// Returns false for a freed batch or mesh.
    pub fn update_instance_batch_colors(
        &mut self,
        batch: InstanceBatch,
        transforms: &[Mat4],
        colors: &[Color],
    ) -> bool {
        let Some(Some(old)) = self.assets3d.instance_batches.get(batch.id as usize) else {
            return false;
        };
        let bounds = if old.legacy_distance {
            self.assets3d.live_bounds(old.levels[0].mesh.id)
        } else {
            self.instance_level_bounds(&old.levels)
        };
        let Some(bounds) = bounds else {
            return false;
        };
        let new = Batch::levels(
            &old.levels,
            transforms,
            colors,
            old.options,
            bounds,
            old.legacy_distance,
        );
        if let Some(r) = self.renderer.as_mut() {
            r.set_instance_batch(batch.id, Some(&new));
        }
        self.assets3d.instance_batches[batch.id as usize] = Some(new);
        self.assets3d.instance_batch_revision = self.assets3d.instance_batch_revision.wrapping_add(1);
        true
    }
    /// Releases a region's CPU and GPU resources: [`Graphics::free`](crate::gfx::Graphics::free)
    /// with a batch. See [`InstanceBatch`].
    #[deprecated(note = "use `ctx.gfx.free(batch)`")]
    pub fn free_instance_batch(&mut self, batch: InstanceBatch) {
        self.free(batch);
    }
    /// Frees instance batch `id`: the engine side of [`Graphics::free`](crate::gfx::Graphics::free).
    pub(crate) fn free_instance_batch_id(&mut self, id: u32) {
        let batch = InstanceBatch { id };
        if let Some(slot) = self.assets3d.instance_batches.get_mut(batch.id as usize) {
            *slot = None;
            self.assets3d.instance_batch_revision = self.assets3d.instance_batch_revision.wrapping_add(1);
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

#[cfg(test)]
mod level_tests {
    use super::*;
    #[test]
    fn half_open_bands_partition_overlap_and_leave_gaps() {
        let level = |min_distance, max_distance| InstanceLevel {
            mesh: Mesh { id: 0 },
            min_distance,
            max_distance,
            camera: true,
            shadows: true,
        };
        let levels = [level(0.0, 10.0), level(10.0, 20.0), level(15.0, 30.0)];
        for (distance, expected) in [
            (0.0, vec![0]),
            (9.999, vec![0]),
            (10.0, vec![1]),
            (15.0, vec![1, 2]),
            (20.0, vec![2]),
            (30.0, vec![]),
        ] {
            assert_eq!(
                levels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, l)| l.contains(distance).then_some(i))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
    #[test]
    fn batch_bounds_include_origins_and_every_transformed_proxy() {
        let levels = [InstanceLevel {
            mesh: Mesh { id: 0 },
            min_distance: 0.0,
            max_distance: 100.0,
            camera: true,
            shadows: true,
        }];
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 3.0, 1.0),
            crate::Quat::IDENTITY,
            Vec3::new(-20.0, 3.0, 8.0),
        );
        let b = Batch::levels(
            &levels,
            &[transform],
            &[Color::RED],
            Default::default(),
            (Vec3::new(10.0, 0.0, 0.0), 2.0),
            false,
        );
        let origin = transform.w_axis.truncate();
        assert!(origin.cmpge(b.min).all() && origin.cmple(b.max).all());
        let sphere = b.spheres[0];
        let center = Vec3::from_slice(&sphere);
        assert!((center - sphere[3]).cmpge(b.min).all() && (center + sphere[3]).cmple(b.max).all());
        assert_eq!(b.instances[0].color, super::super::linear(Color::RED));
    }
}
