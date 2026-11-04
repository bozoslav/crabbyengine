//! [`TerrainTile`]: a square of heightfield terrain, drawn more coarsely
//! further from the camera.

use super::{Material, Mesh, MeshData};
use crate::Ctx;

/// How many detail levels a [`TerrainTile`] has: every height, then every
/// 2nd, 4th and 8th.
pub const TERRAIN_LODS: usize = 4;

/// A square of terrain from a grid of heights, made with
/// [`Ctx::create_terrain_tile`]. Spawn it with a [`Transform3D`](crate::Transform3D)
/// at the tile's corner, instead of a [`Model`](crate::Model). The renderer
/// picks a detail level by how far the camera is: the full grid nearer than
/// `lod_distance`, then every 2nd height to twice that, every 4th to four
/// times that, and every 8th beyond. Skirts hide the cracks where tiles of
/// different detail meet.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let grass = ctx.create_material(MaterialDesc { roughness: 0.95, ..MaterialDesc::color(Color::hex(0x5A8A3C)) });
/// // 65 x 65 heights, 2 m apart: a 128 m tile with gentle hills.
/// let n = 65;
/// let heights: Vec<f32> = (0..n * n)
///     .map(|i| ((i % n) as f32 * 0.1).sin() * 3.0 + ((i / n) as f32 * 0.07).cos() * 4.0)
///     .collect();
/// let tile = ctx.create_terrain_tile(&heights, 128.0, grass);
/// ctx.world.spawn((Transform3D::at(vec3(-64.0, 0.0, -64.0)), tile));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)] // shared with C/C++ (`CrabbyTerrainTile`)
pub struct TerrainTile {
    /// The meshes, finest first.
    pub lods: [Mesh; TERRAIN_LODS],
    /// Up to how far from the camera the finest mesh is drawn, in world
    /// units. Default: the tile's size.
    pub lod_distance: f32,
}

impl TerrainTile {
    /// Which of [`lods`](Self::lods) is drawn `distance` away from the
    /// camera (measured to the nearest point of the tile's bounds).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(tile: TerrainTile) {
    /// assert_eq!(tile.lod(0.0), 0);
    /// assert_eq!(tile.lod(tile.lod_distance * 10.0), 3);
    /// # }
    /// ```
    #[must_use]
    pub fn lod(&self, distance: f32) -> usize {
        lod_at(distance, self.lod_distance)
    }
}

/// 0 nearer than `lod_distance`, then one more for every doubling.
pub(crate) fn lod_at(distance: f32, lod_distance: f32) -> usize {
    if lod_distance <= 0.0 || distance.is_nan() || distance < lod_distance {
        return 0;
    }
    let doublings = (distance / lod_distance).log2().floor() as usize;
    (doublings + 1).min(TERRAIN_LODS - 1)
}

impl Ctx {
    /// Makes a [`TerrainTile`] `size` wide from a square grid of heights
    /// (see [`MeshData::heightfield`]: row after row along +Z, NaN for a
    /// hole), drawn with `material`. Neighbouring tiles share their edge
    /// heights, so a tile of n × n heights starts where the last one ended,
    /// `size` further on.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, heights: &[f32], ground: Material) {
    /// let tile = ctx.create_terrain_tile(heights, 256.0, ground);
    /// ctx.world.spawn((Transform3D::at(vec3(256.0, 0.0, 0.0)), tile));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If `heights` isn't a square grid of at least 2 × 2, `size` isn't
    /// above 0, or the material comes from another game.
    pub fn create_terrain_tile(&mut self, heights: &[f32], size: f32, material: Material) -> TerrainTile {
        assert!(
            size > 0.0 && size.is_finite(),
            "crabbyengine: create_terrain_tile: size must be above 0 (got {size})"
        );
        let lods = std::array::from_fn(|i| {
            let data = MeshData::heightfield(heights, size, 1 << i);
            self.create_mesh(&data, material)
        });
        TerrainTile {
            lods,
            lod_distance: size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_drops_once_per_doubling_of_distance() {
        let at = |d| lod_at(d, 100.0);
        assert_eq!(at(0.0), 0);
        assert_eq!(at(99.9), 0);
        assert_eq!(at(100.0), 1);
        assert_eq!(at(199.0), 1);
        assert_eq!(at(200.0), 2);
        assert_eq!(at(399.0), 2);
        assert_eq!(at(400.0), 3);
        assert_eq!(at(1e9), 3);
        assert_eq!(at(f32::NAN), 0);
        assert_eq!(lod_at(50.0, 0.0), 0, "no distance: always the finest");
    }
}
