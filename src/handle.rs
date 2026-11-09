//! One lifecycle for every GPU and audio resource handle: [`Handle`],
//! [`Ctx::is_live`], [`Ctx::free`] and [`Ctx::live_counts`].
//!
//! A handle is a small `Copy` value naming something the engine holds for
//! the game: a texture, a mesh, a sound. Each kind answers the same three
//! questions the same way:
//!
//! - Is it still valid? [`Ctx::is_live`].
//! - Release it: [`Ctx::free`]. Ids are never reused within a run, so a
//!   freed handle stays dead; freeing it again does nothing. Meshes and GPU
//!   buffers are the exception: streamed worlds make and free them all the
//!   time, so the next one made takes a freed id. Drop those handles once
//!   freed.
//! - An empty handle: [`Handle::NONE`], which is never live.
//!
//! Freeing something others still use is defined: they draw with the
//! default. A model whose mesh was freed draws nothing (as with
//! [`Mesh::NONE`](crate::Mesh::NONE)); a material whose texture was freed
//! samples white, as if it had none; a mesh part whose material was freed
//! draws with the default material; text in a freed font uses the built-in
//! one. Nothing is reference counted: freeing is explicit, and handles
//! stay `Copy`.

use std::hash::Hash;

use crate::ctx::Ctx;

mod sealed {
    pub trait Sealed {}
}

/// A handle to a resource the engine holds for the game: [`Texture`](crate::Texture),
/// [`Sound`](crate::Sound), [`Font`](crate::Font), and with the `3d` feature [`Mesh`](crate::Mesh),
/// [`Material`](crate::Material), [`Shader`](crate::Shader),
/// [`Compute`](crate::Compute), [`Effect`](crate::Effect),
/// [`GpuBuffer`](crate::GpuBuffer), [`InstanceBatch`](crate::InstanceBatch)
/// and [`TextureArray`](crate::TextureArray). Render targets are textures.
///
/// Check one with [`Ctx::is_live`] and release it with [`Ctx::free`].
/// Implemented by the engine's handle types only.
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let pixel = ctx.gfx.create_texture(1, 1, &[255; 4]);
///     assert!(ctx.is_live(pixel));
///     ctx.free(pixel);
///     assert!(!ctx.is_live(pixel));
///     assert!(!ctx.is_live(Texture::NONE));
/// });
/// ```
pub trait Handle: Copy + Eq + Hash + std::fmt::Debug + sealed::Sealed {
    /// No resource: never live, and freeing it does nothing. Whatever shows
    /// it draws the default (nothing for a mesh, white for a texture, the
    /// built-in font for text).
    ///
    /// ```
    /// # use ponko::*;
    /// assert_eq!(Texture::NONE.id(), u32::MAX);
    /// ```
    const NONE: Self;

    /// The engine's id of this handle, for language bindings, tools and
    /// maps keyed by id. Unique per kind within a run.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let a = ctx.gfx.create_texture(1, 1, &[255; 4]);
    ///     let b = ctx.gfx.create_texture(1, 1, &[255; 4]);
    ///     assert_ne!(a.id(), b.id());
    /// });
    /// ```
    fn id(self) -> u32;

    /// A handle from an id (from [`id`](Self::id)). Only the id is kept:
    /// for [`Texture`](crate::Texture), [`Sound`](crate::Sound) and [`TextureArray`](crate::TextureArray)
    /// the size or length reads 0, so compare such handles by id. Check it
    /// with [`Ctx::is_live`] before trusting it.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let t = ctx.gfx.create_texture(2, 2, &[255; 16]);
    ///     assert!(ctx.is_live(Texture::from_id(t.id())));
    /// });
    /// ```
    fn from_id(id: u32) -> Self;

    /// Whether `ctx` holds this resource (see [`Ctx::is_live`]).
    #[doc(hidden)]
    fn live_in(self, ctx: &Ctx) -> bool;

    /// Releases this resource in `ctx` (see [`Ctx::free`]).
    #[doc(hidden)]
    fn free_in(self, ctx: &mut Ctx);
}

/// How many handles of each kind are live, from [`Ctx::live_counts`]: for
/// tests of streaming code and leak checks. Kinds the build doesn't have
/// (without the `3d` feature) count 0.
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let before = ctx.live_counts();
///     let t = ctx.gfx.create_texture(1, 1, &[255; 4]);
///     assert_eq!(ctx.live_counts().textures, before.textures + 1);
///     ctx.free(t);
///     assert_eq!(ctx.live_counts(), before);
/// });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiveCounts {
    /// Textures the game made or loaded, render targets included.
    pub textures: usize,
    /// Of `textures`, the render targets.
    pub render_targets: usize,
    /// Meshes (each detail level of a terrain tile is one).
    pub meshes: usize,
    /// Materials, the default one included once it was made.
    pub materials: usize,
    /// Material shaders (not the built-in one).
    pub shaders: usize,
    /// Compute passes.
    pub computes: usize,
    /// Screen effects.
    pub effects: usize,
    /// GPU buffers.
    pub gpu_buffers: usize,
    /// Instance batches.
    pub instance_batches: usize,
    /// Texture arrays.
    pub texture_arrays: usize,
    /// Sounds, streams included.
    pub sounds: usize,
    /// Fonts the game loaded (not the built-in one).
    pub fonts: usize,
}

impl Ctx {
    /// Whether `handle` names something this game made and hasn't freed.
    /// False for [`Handle::NONE`] and for handles from another game.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let font = Font::DEFAULT;
    ///     assert!(ctx.is_live(font));
    ///     assert!(!ctx.is_live(Font::NONE));
    /// });
    /// ```
    #[must_use]
    pub fn is_live<H: Handle>(&self, handle: H) -> bool {
        handle.live_in(self)
    }

    /// Releases what `handle` names: its GPU memory (or decoded audio), and
    /// the file cache entry if it was loaded, so loading the file again
    /// loads it afresh. Its id is never given to anything else, so the
    /// handle stays dead; freeing it again, or freeing
    /// [`Handle::NONE`], does nothing.
    ///
    /// Whatever still uses it draws with the default: models showing a
    /// freed mesh draw nothing, materials sampling a freed texture see
    /// white, mesh parts with a freed material draw with the default
    /// material, a freed shader's materials draw with the built-in shader,
    /// target cameras, compute passes and screen effects using a freed
    /// target, pass or effect are skipped, sounds stop, and text in a freed
    /// font uses the built-in font. Despawn or update them to stop that.
    /// The built-in font is never freed.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let beep = ctx.audio.create(8_000, &[0.0; 800]);
    ///     ctx.free(beep);
    ///     assert!(!ctx.is_live(beep));
    ///     let again = ctx.audio.create(8_000, &[0.0; 800]);
    ///     assert_ne!(again.id(), beep.id()); // ids are never reused
    /// });
    /// ```
    pub fn free<H: Handle>(&mut self, handle: H) {
        handle.free_in(self);
    }

    /// How many handles of each kind are live now (see [`LiveCounts`]).
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     assert_eq!(ctx.live_counts().sounds, 0);
    /// });
    /// ```
    #[must_use]
    pub fn live_counts(&self) -> LiveCounts {
        #[cfg_attr(not(feature = "3d"), allow(unused_mut))]
        let mut counts = LiveCounts {
            textures: self.gfx.textures_live.len(),
            sounds: self.audio.sounds_live.len(),
            fonts: self.gfx.text.loaded_fonts(),
            ..LiveCounts::default()
        };
        #[cfg(feature = "3d")]
        self.gfx.count_live_3d(&mut counts);
        counts
    }
}

/// Which ids of one kind were made, and which of those were freed. Ids
/// index a byte each, so a kind made a million times costs a megabyte.
#[derive(Debug, Default)]
pub(crate) struct LiveSet {
    state: Vec<u8>,
    live: usize,
}

const MADE: u8 = 1;
const FREED: u8 = 2;

impl LiveSet {
    /// Marks `id` made and live (once: a freed id stays freed).
    pub(crate) fn insert(&mut self, id: u32) {
        let i = id as usize;
        if i >= self.state.len() {
            self.state.resize(i + 1, 0);
        }
        if self.state[i] == 0 {
            self.state[i] = MADE;
            self.live += 1;
        }
    }

    /// Whether `id` was made and not freed.
    pub(crate) fn contains(&self, id: u32) -> bool {
        self.state.get(id as usize) == Some(&MADE)
    }

    /// Whether `id` was made, freed or not.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub(crate) fn was_made(&self, id: u32) -> bool {
        self.state.get(id as usize).is_some_and(|&s| s != 0)
    }

    /// Marks `id` freed; true if it was live.
    pub(crate) fn remove(&mut self, id: u32) -> bool {
        match self.state.get_mut(id as usize) {
            Some(s) if *s == MADE => {
                *s = FREED;
                self.live -= 1;
                true
            }
            _ => false,
        }
    }

    /// How many are live.
    pub(crate) fn len(&self) -> usize {
        self.live
    }
}

/// `Handle` for a handle type whose only field is `id: u32`.
#[cfg(feature = "3d")]
macro_rules! id_handle {
    ($ty:ty, $live:expr, $free:expr) => {
        impl sealed::Sealed for $ty {}
        impl Handle for $ty {
            const NONE: Self = Self { id: u32::MAX };
            fn id(self) -> u32 {
                self.id
            }
            fn from_id(id: u32) -> Self {
                Self { id }
            }
            fn live_in(self, ctx: &Ctx) -> bool {
                #[allow(clippy::redundant_closure_call)]
                ($live)(ctx, self)
            }
            fn free_in(self, ctx: &mut Ctx) {
                #[allow(clippy::redundant_closure_call)]
                ($free)(ctx, self)
            }
        }
    };
}

impl sealed::Sealed for crate::Texture {}
impl Handle for crate::Texture {
    const NONE: Self = crate::Texture::new(u32::MAX, 0, 0);
    fn id(self) -> u32 {
        self.id
    }
    fn from_id(id: u32) -> Self {
        crate::Texture::new(id, 0, 0)
    }
    fn live_in(self, ctx: &Ctx) -> bool {
        ctx.gfx.textures_live.contains(self.id)
    }
    fn free_in(self, ctx: &mut Ctx) {
        ctx.gfx.free_texture_id(self.id);
    }
}

impl sealed::Sealed for crate::Sound {}
impl Handle for crate::Sound {
    const NONE: Self = crate::Sound::new(u32::MAX, 0.0);
    fn id(self) -> u32 {
        self.id
    }
    fn from_id(id: u32) -> Self {
        crate::Sound::new(id, 0.0)
    }
    fn live_in(self, ctx: &Ctx) -> bool {
        ctx.audio.sounds_live.contains(self.id)
    }
    fn free_in(self, ctx: &mut Ctx) {
        ctx.audio.free_id(self.id);
    }
}

impl sealed::Sealed for crate::Font {}
impl Handle for crate::Font {
    const NONE: Self = crate::Font(u32::MAX);
    fn id(self) -> u32 {
        self.0
    }
    fn from_id(id: u32) -> Self {
        crate::Font(id)
    }
    fn live_in(self, ctx: &Ctx) -> bool {
        ctx.gfx.text.has_font(self)
    }
    fn free_in(self, ctx: &mut Ctx) {
        ctx.gfx.free_font_id(self.0);
    }
}

#[cfg(feature = "3d")]
mod three {
    use super::*;
    use crate::{Compute, Effect, GpuBuffer, InstanceBatch, Material, Mesh, Shader, TextureArray};

    id_handle!(Mesh, |c: &Ctx, h: Mesh| c.gfx.is_mesh_live(h), |c: &mut Ctx, h| c
        .gfx
        .free_mesh(h));
    id_handle!(
        Material,
        |c: &Ctx, h: Material| c.gfx.assets3d.materials_live.contains(h.id),
        |c: &mut Ctx, h: Material| c.gfx.free_material_id(h.id)
    );
    id_handle!(
        Shader,
        |c: &Ctx, h: Shader| c.gfx.assets3d.shaders_live.contains(h.id),
        |c: &mut Ctx, h: Shader| c.gfx.free_shader_id(h.id)
    );
    id_handle!(
        Compute,
        |c: &Ctx, h: Compute| c.gfx.assets3d.computes_live.contains(h.id),
        |c: &mut Ctx, h: Compute| c.gfx.free_compute_id(h.id)
    );
    id_handle!(
        Effect,
        |c: &Ctx, h: Effect| c.gfx.assets3d.effects_live.contains(h.id),
        |c: &mut Ctx, h: Effect| c.gfx.free_effect_id(h.id)
    );
    id_handle!(
        GpuBuffer,
        |c: &Ctx, h: GpuBuffer| c.gfx.gpu_buffer_len(h).is_some(),
        |c: &mut Ctx, h: GpuBuffer| {
            // Clear what pointed at it, so the id's next buffer starts clean.
            if c.gfx.gpu_buffer_len(h).is_some() {
                c.compute_passes.retain(|p| p.buffer != Some(h));
            }
            c.gfx.free_gpu_buffer(h);
        }
    );
    id_handle!(
        InstanceBatch,
        |c: &Ctx, h: InstanceBatch| c.gfx.instance_batch_len(h).is_some(),
        |c: &mut Ctx, h| c.gfx.free_instance_batch(h)
    );

    impl sealed::Sealed for TextureArray {}
    impl Handle for TextureArray {
        const NONE: Self = TextureArray {
            id: u32::MAX,
            width: 0,
            height: 0,
            layers: 0,
        };
        fn id(self) -> u32 {
            self.id
        }
        fn from_id(id: u32) -> Self {
            TextureArray {
                id,
                width: 0,
                height: 0,
                layers: 0,
            }
        }
        fn live_in(self, ctx: &Ctx) -> bool {
            ctx.gfx.assets3d.arrays_live.contains(self.id)
        }
        fn free_in(self, ctx: &mut Ctx) {
            ctx.gfx.free_texture_array_id(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_freed_id_stays_freed() {
        let mut set = LiveSet::default();
        set.insert(3);
        assert!(set.contains(3) && set.was_made(3) && !set.was_made(2));
        assert_eq!(set.len(), 1);
        assert!(set.remove(3));
        assert!(!set.remove(3));
        set.insert(3);
        assert!(!set.contains(3) && set.was_made(3));
        assert_eq!(set.len(), 0);
    }
}
