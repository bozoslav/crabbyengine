//! What the game holds right now: live textures, meshes, materials,
//! sounds and other asset handles, with estimated bytes ([`Ctx::asset_inventory`]).
//!
//! Counts come from the engine's own handle tables, the same ones that
//! decide whether a handle is live, so they also work in headless runs.
//! A windowed game adds what the renderer actually holds on the GPU
//! ([`GpuResidency`]), to cross-check the two.

use std::fmt;

use serde::Serialize;

use crate::ctx::Ctx;

/// A class of asset handle in an [`AssetInventory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AssetClass {
    /// Images from [`Graphics::create_texture`](crate::Graphics::create_texture) and [`Ctx::load_texture`]
    /// (fonts and UI images too). Never freed.
    Texture,
    /// 3D: textures cameras draw into ([`Graphics::create_render_target`](crate::Graphics::create_render_target)). Never freed.
    RenderTarget,
    /// 3D: [`TextureArray`](crate::TextureArray)s. Never freed.
    TextureArray,
    /// 3D: [`Material`](crate::Material)s. Never freed.
    Material,
    /// 3D: material [`Shader`](crate::Shader)s. Never freed.
    Shader,
    /// 3D: [`Mesh`](crate::Mesh)es, freed with [`Graphics::free_mesh`](crate::Graphics::free_mesh).
    Mesh,
    /// 3D: [`GpuBuffer`](crate::GpuBuffer)s, freed with [`Ctx::free`](crate::Ctx::free).
    GpuBuffer,
    /// 3D: screen [`Effect`](crate::Effect)s. Never freed.
    Effect,
    /// 3D: [`Compute`](crate::Compute) passes. Never freed.
    Compute,
    /// [`Sound`](crate::Sound)s, loaded or made. Never freed.
    Sound,
}

impl AssetClass {
    /// Its name in reports, like `"render_target"`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::RenderTarget => "render_target",
            Self::TextureArray => "texture_array",
            Self::Material => "material",
            Self::Shader => "shader",
            Self::Mesh => "mesh",
            Self::GpuBuffer => "gpu_buffer",
            Self::Effect => "effect",
            Self::Compute => "compute",
            Self::Sound => "sound",
        }
    }
}

impl fmt::Display for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How many handles of one [`AssetClass`] are live, and about how much
/// memory they hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AssetCount {
    /// What they are.
    pub class: AssetClass,
    /// Handles made and not freed.
    pub live: usize,
    /// Estimated bytes they hold, on the GPU (or in memory, for sounds).
    /// An estimate from what was handed in: base image levels at 4 bytes
    /// a pixel (mipmaps a 3D material adds are not counted), mesh corners
    /// and indices as the renderer lays them out (skinning and morph data
    /// not counted), decoded audio samples (streamed music counts 0). 0
    /// for classes whose size is small or unknown: materials, shaders,
    /// effects and compute passes.
    pub bytes: u64,
}

/// How much one [`AssetClass`] grew between two inventories, from
/// [`AssetInventory::growth_since`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AssetGrowth {
    /// What grew.
    pub class: AssetClass,
    /// More live handles than before (negative: fewer).
    pub live: i64,
    /// More estimated bytes than before (negative: fewer).
    pub bytes: i64,
}

/// What the renderer actually holds on the GPU, counted from its own
/// tables rather than from handles: a handle freed while its GPU memory
/// stays behind shows up here. `None` in headless runs, which have no GPU.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GpuResidency {
    /// 2D textures, render targets included, and the renderer's own
    /// (such as its 1x1 white texture).
    pub textures: usize,
    /// Their bytes, mipmaps included.
    pub texture_bytes: u64,
    /// 3D meshes with vertex and index buffers.
    pub meshes: usize,
    /// Those buffers' bytes (skinning buffers not counted).
    pub mesh_bytes: u64,
    /// 3D GPU buffers.
    pub gpu_buffers: usize,
    /// Their bytes.
    pub gpu_buffer_bytes: u64,
}

/// Live asset handles by [`AssetClass`], from [`Ctx::asset_inventory`]:
/// to see what a long-running game holds, and to find what grows when it
/// shouldn't (a leak). Take one as a baseline, run the game for a while,
/// and compare with [`growth_since`](Self::growth_since).
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let before = ctx.asset_inventory();
///     let _sprite = ctx.gfx.create_texture(2, 2, &[255; 16]);
///     let after = ctx.asset_inventory();
///     assert_eq!(after.get(AssetClass::Texture).live, before.get(AssetClass::Texture).live + 1);
///     let growth = after.growth_since(&before);
///     assert_eq!(growth[0].class, AssetClass::Texture);
///     assert_eq!(growth[0].bytes, 2 * 2 * 4);
/// });
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AssetInventory {
    /// Every class this build has, in [`AssetClass`] order, empty ones too.
    pub classes: Vec<AssetCount>,
    /// What the renderer holds, to cross-check the handles against.
    pub gpu: Option<GpuResidency>,
}

impl AssetInventory {
    /// The count for `class` (zero when this build doesn't have it, such
    /// as meshes without the `3d` feature).
    #[must_use]
    pub fn get(&self, class: AssetClass) -> AssetCount {
        self.classes
            .iter()
            .find(|c| c.class == class)
            .copied()
            .unwrap_or(AssetCount {
                class,
                live: 0,
                bytes: 0,
            })
    }

    /// Estimated bytes of every class together.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.classes.iter().map(|c| c.bytes).sum()
    }

    /// The classes with more live handles or bytes than in `baseline`,
    /// most bytes first: empty when nothing grew. A leak test takes an
    /// inventory, loads and frees in a loop, and expects this empty.
    #[must_use]
    pub fn growth_since(&self, baseline: &Self) -> Vec<AssetGrowth> {
        let mut grown: Vec<AssetGrowth> = self
            .classes
            .iter()
            .map(|now| {
                let then = baseline.get(now.class);
                AssetGrowth {
                    class: now.class,
                    live: now.live as i64 - then.live as i64,
                    bytes: now.bytes as i64 - then.bytes as i64,
                }
            })
            .filter(|g| g.live > 0 || g.bytes > 0)
            .collect();
        grown.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(b.live.cmp(&a.live)));
        grown
    }
}

impl fmt::Display for AssetInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:<14} {:>8} {:>12}", "class", "live", "bytes")?;
        for c in &self.classes {
            writeln!(f, "{:<14} {:>8} {:>12}", c.class.name(), c.live, c.bytes)?;
        }
        if let Some(g) = &self.gpu {
            writeln!(
                f,
                "on the GPU: {} textures ({} bytes), {} meshes ({} bytes), {} buffers ({} bytes)",
                g.textures, g.texture_bytes, g.meshes, g.mesh_bytes, g.gpu_buffers, g.gpu_buffer_bytes
            )?;
        }
        Ok(())
    }
}

/// Classes whose handles have no table of their own to count from (2D
/// textures): what each live one holds.
#[derive(Default)]
pub(crate) struct Ledger {
    /// Bytes of each 2D texture made and not freed, by texture id.
    pub textures: std::collections::HashMap<u32, u64>,
}

/// Bytes of an RGBA8 image.
pub(crate) fn rgba8_bytes(width: u32, height: u32) -> u64 {
    u64::from(width) * u64::from(height) * 4
}

/// Bytes of an image with every mipmap level down to 1x1.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(crate) fn mip_chain_bytes(width: u32, height: u32, bytes_per_pixel: u64) -> u64 {
    let (mut w, mut h, mut total) = (u64::from(width.max(1)), u64::from(height.max(1)), 0);
    loop {
        total += w * h * bytes_per_pixel;
        if w == 1 && h == 1 {
            return total;
        }
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
}

impl Ctx {
    /// Live asset handles by class, with estimated bytes: see
    /// [`AssetInventory`]. Cheap enough to call every frame, but meant for
    /// diagnostics and tests. The control endpoint's `perf.get` includes it
    /// as `assets`.
    #[must_use]
    pub fn asset_inventory(&self) -> AssetInventory {
        let mut classes = vec![AssetCount {
            class: AssetClass::Texture,
            live: self.gfx.asset_ledger.textures.len(),
            bytes: self.gfx.asset_ledger.textures.values().sum(),
        }];
        #[cfg(feature = "3d")]
        classes.extend(self.gfx.assets3d.inventory(&self.gfx.textures_live));
        let (sounds, sound_bytes) = self.audio.backend.inventory();
        classes.push(AssetCount {
            class: AssetClass::Sound,
            live: sounds,
            bytes: sound_bytes,
        });
        let gpu = if self.headless {
            None
        } else {
            self.gfx.renderer.as_ref().and_then(crate::render::Renderer::residency)
        };
        AssetInventory { classes, gpu }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chains_add_every_level() {
        assert_eq!(mip_chain_bytes(1, 1, 4), 4);
        // 4x2, 2x1, 1x1.
        assert_eq!(mip_chain_bytes(4, 2, 4), (8 + 2 + 1) * 4);
    }

    #[test]
    fn growth_lists_only_classes_that_grew_biggest_first() {
        let count = |class, live, bytes| AssetCount { class, live, bytes };
        let before = AssetInventory {
            classes: vec![count(AssetClass::Texture, 2, 100), count(AssetClass::Sound, 1, 10)],
            gpu: None,
        };
        let after = AssetInventory {
            classes: vec![
                count(AssetClass::Texture, 3, 150),
                count(AssetClass::Sound, 1, 10),
                count(AssetClass::Mesh, 1, 900),
            ],
            gpu: None,
        };
        let growth = after.growth_since(&before);
        assert_eq!(growth.len(), 2);
        assert_eq!(
            (growth[0].class, growth[0].live, growth[0].bytes),
            (AssetClass::Mesh, 1, 900)
        );
        assert_eq!(
            (growth[1].class, growth[1].live, growth[1].bytes),
            (AssetClass::Texture, 1, 50)
        );
        assert!(before.growth_since(&after).is_empty(), "shrinking is not growth");
        assert!(after.to_string().contains("mesh"));
    }
}
