//! [`TerrainTile`]: a square of heightfield terrain, drawn more coarsely
//! further from the camera.

use glam::{Vec2, Vec4};

use super::{Material, MaterialDesc, Mesh, MeshData, TextureArray};
use crate::Ctx;

/// The most ground layers a terrain material blends.
pub const TERRAIN_LAYERS: usize = 8;

/// What a terrain material is made from: ground layers (grass, dirt, rock,
/// ...) and a weight map saying how much of each shows where. Made into a
/// [`Material`] with [`Ctx::create_terrain_material`] and used for
/// [`Ctx::create_terrain_tile`].
///
/// The layers' textures are lined up with the world, not with each tile,
/// so they run on across tile edges; each repeats every
/// [`layer_size`](Self::layer_size) world units.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let ground = ctx.load_texture_array(&["grass.png", "dirt.png", "rock.png"], TextureKind::Color);
/// // One RGBA weight map for the whole 512 m: red grass, green dirt, blue rock.
/// let weights = ctx.load_texture_array(&["weights.png"], TextureKind::Data);
/// let desc = TerrainMaterialDesc::new(ground, weights, vec2(0.0, 0.0), vec2(512.0, 512.0));
/// let terrain = ctx.create_terrain_material(&desc);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainMaterialDesc {
    /// Each layer's color texture ([`TextureKind::Color`](crate::TextureKind::Color)),
    /// up to [`TERRAIN_LAYERS`].
    pub layers: TextureArray,
    /// The layers' normal maps, one each in the same order
    /// ([`TextureKind::Data`](crate::TextureKind::Data), OpenGL style).
    /// Default none.
    pub normals: Option<TextureArray>,
    /// How strong the normal maps are. Default 1.
    pub normal_strength: f32,
    /// How much of each layer shows where ([`TextureKind::Data`](crate::TextureKind::Data)):
    /// its first image's red, green, blue and alpha are layers 0 to 3, its
    /// second image's are layers 4 to 7. Weights are evened out to add up
    /// to 1; where they are all 0, the first layer shows.
    pub weights: TextureArray,
    /// The world corner (x, z) where the weight map's top-left pixel
    /// lies: its rows run along +Z.
    pub weights_corner: Vec2,
    /// How much ground (x, z) the weight map covers, in world units.
    pub weights_size: Vec2,
    /// How many world units each layer's texture covers before it repeats.
    /// Default 4.
    pub layer_size: [f32; TERRAIN_LAYERS],
    /// 0 is mirror-smooth, 1 fully rough. Default 0.9.
    pub roughness: f32,
}

impl TerrainMaterialDesc {
    /// Ground `layers`, blended by a `weights` map lying over `size` world
    /// units from `corner`; everything else as by default.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ground: TextureArray, weights: TextureArray) {
    /// let desc = TerrainMaterialDesc { roughness: 0.8, ..TerrainMaterialDesc::new(ground, weights, Vec2::ZERO, vec2(256.0, 256.0)) };
    /// # }
    /// ```
    #[must_use]
    pub fn new(layers: TextureArray, weights: TextureArray, corner: Vec2, size: Vec2) -> Self {
        Self {
            layers,
            normals: None,
            normal_strength: 1.0,
            weights,
            weights_corner: corner,
            weights_size: size,
            layer_size: [4.0; TERRAIN_LAYERS],
            roughness: 0.9,
        }
    }

    /// Says what's wrong, if anything.
    pub(crate) fn check(&self) -> Result<(), String> {
        let count = self.layers.layers() as usize;
        if count > TERRAIN_LAYERS {
            return Err(format!(
                "a terrain material blends at most {TERRAIN_LAYERS} layers (got {count})"
            ));
        }
        let needed = count.div_ceil(4) as u32;
        if self.weights.layers() < needed {
            return Err(format!(
                "{count} terrain layers need {needed} weight images (4 layers each), got {}",
                self.weights.layers()
            ));
        }
        if let Some(normals) = self.normals
            && normals.layers() as usize != count
        {
            return Err(format!(
                "{count} terrain layers need {count} normal maps, got {}",
                normals.layers()
            ));
        }
        let positive = |v: f32| v > 0.0 && v.is_finite();
        if !positive(self.weights_size.x) || !positive(self.weights_size.y) {
            return Err(format!(
                "the weight map's size must be above 0 (got {})",
                self.weights_size
            ));
        }
        if let Some(bad) = self.layer_size[..count].iter().find(|s| !positive(**s)) {
            return Err(format!("each terrain layer's size must be above 0 (got {bad})"));
        }
        if !(0.0..=1.0).contains(&self.roughness) {
            return Err(format!("roughness goes from 0 to 1 (got {})", self.roughness));
        }
        Ok(())
    }
}

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
    /// Makes a material for terrain from ground layers and a weight map
    /// (see [`TerrainMaterialDesc`]). It's an ordinary [`Material`]: give
    /// it to [`Ctx::create_terrain_tile`], or to any mesh.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, desc: TerrainMaterialDesc, heights: &[f32]) {
    /// let ground = ctx.create_terrain_material(&desc);
    /// let tile = ctx.create_terrain_tile(heights, 256.0, ground);
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If there are more than [`TERRAIN_LAYERS`] layers, too few weight
    /// images or normal maps, a size isn't above 0, or an array comes from
    /// another game.
    pub fn create_terrain_material(&mut self, desc: &TerrainMaterialDesc) -> Material {
        self.try_create_terrain_material(desc)
            .unwrap_or_else(|e| panic!("crabbyengine: create_terrain_material: {e}"))
    }

    /// [`Ctx::create_terrain_material`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_terrain_material(&mut self, desc: &TerrainMaterialDesc) -> Result<Material, String> {
        desc.check()?;
        let arrays = [Some(desc.layers), desc.normals, Some(desc.weights)];
        if arrays.into_iter().flatten().any(|a| !self.has_texture_array(a)) {
            return Err("that texture array wasn't made by this game".into());
        }
        let shader = match self.assets3d.terrain_shader {
            Some(shader) => shader,
            None => {
                let shader = self.try_create_shader("terrain.wgsl (built in)", include_str!("terrain.wgsl"))?;
                self.assets3d.terrain_shader = Some(shader);
                shader
            }
        };
        let inverse = desc.layer_size.map(|s| if s > 0.0 { 1.0 / s } else { 0.0 });
        let custom = [
            Vec4::new(
                desc.layers.layers() as f32,
                desc.normal_strength,
                if desc.normals.is_some() { 1.0 } else { 0.0 },
                0.0,
            ),
            Vec4::from_slice(&inverse[..4]),
            Vec4::from_slice(&inverse[4..]),
            Vec4::new(
                desc.weights_corner.x,
                desc.weights_corner.y,
                1.0 / desc.weights_size.x,
                1.0 / desc.weights_size.y,
            ),
        ];
        Ok(self.create_material(MaterialDesc {
            roughness: desc.roughness,
            shader: Some(shader),
            custom,
            custom_arrays: arrays,
            ..Default::default()
        }))
    }

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
