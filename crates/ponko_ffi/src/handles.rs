//! One handle lifecycle (Rust `Handle`, `Ctx::is_live`, `Ctx::free`,
//! `Ctx::live_counts`): every handle struct starts with its `uint32_t id`,
//! so one pair of calls takes a kind and an id for all of them.

use ponko::handle::LiveCounts;
use ponko::render::Compute;
use ponko::render::Effect;
use ponko::render::GpuBuffer;
use ponko::render::InstanceBatch;
use ponko::render::Material;
use ponko::render::Mesh;
use ponko::render::Shader;
use ponko::terrain::TextureArray;
use ponko::{Font, Handle, Sound, Texture};

use crate::ctx::{ctx_access, out};
use crate::{CrabbyCtx, CrabbyStatus, boundary, invalid};

/// The id no handle of any kind ever has (Rust `Handle::NONE`): never
/// live, and freeing it does nothing. `CRABBY_MESH_NONE` is the same value.
/// ```
/// assert_eq!(ponko_ffi::CRABBY_HANDLE_NONE, u32::MAX);
/// ```
pub const CRABBY_HANDLE_NONE: u32 = u32::MAX;

/// Which kind of handle `crabby_ctx_handle_is_live` and
/// `crabby_ctx_handle_free` take (pass it as a `uint32_t`). Render targets
/// are textures.
/// ```
/// assert_eq!(ponko_ffi::CrabbyHandleKind::CRABBY_HANDLE_TEXTURE_ARRAY as u32, 10);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyHandleKind {
    /// `CrabbyTexture`, render targets included.
    CRABBY_HANDLE_TEXTURE = 0,
    /// `CrabbySound`.
    CRABBY_HANDLE_SOUND = 1,
    /// `CrabbyFont` (the built-in font is never freed).
    CRABBY_HANDLE_FONT = 2,
    /// `CrabbyMesh`.
    CRABBY_HANDLE_MESH = 3,
    /// `CrabbyMaterial`.
    CRABBY_HANDLE_MATERIAL = 4,
    /// `CrabbyShader`.
    CRABBY_HANDLE_SHADER = 5,
    /// `CrabbyCompute`.
    CRABBY_HANDLE_COMPUTE = 6,
    /// `CrabbyEffect`.
    CRABBY_HANDLE_EFFECT = 7,
    /// `CrabbyGpuBuffer`.
    CRABBY_HANDLE_GPU_BUFFER = 8,
    /// `CrabbyInstanceBatch`.
    CRABBY_HANDLE_INSTANCE_BATCH = 9,
    /// `CrabbyTextureArray`.
    CRABBY_HANDLE_TEXTURE_ARRAY = 10,
}

/// How many handles of each kind are live (Rust `LiveCounts`), from
/// `crabby_ctx_live_counts`.
/// ```
/// assert_eq!(ponko_ffi::CrabbyLiveCounts::default().textures, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyLiveCounts {
    /// Textures the game made or loaded, render targets included.
    pub textures: usize,
    /// Of `textures`, the render targets.
    pub render_targets: usize,
    /// Meshes.
    pub meshes: usize,
    /// Materials.
    pub materials: usize,
    /// Material shaders.
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
    /// Sounds.
    pub sounds: usize,
    /// Fonts the game loaded (not the built-in one).
    pub fonts: usize,
}

impl From<LiveCounts> for CrabbyLiveCounts {
    fn from(c: LiveCounts) -> Self {
        Self {
            textures: c.textures,
            render_targets: c.render_targets,
            meshes: c.meshes,
            materials: c.materials,
            shaders: c.shaders,
            computes: c.computes,
            effects: c.effects,
            gpu_buffers: c.gpu_buffers,
            instance_batches: c.instance_batches,
            texture_arrays: c.texture_arrays,
            sounds: c.sounds,
            fonts: c.fonts,
        }
    }
}

/// Calls `f` with the handle of kind `kind` and id `id`.
macro_rules! with_handle {
    ($kind:expr, $id:expr, |$h:ident| $body:expr) => {
        match $kind {
            0 => {
                let $h = Texture::from_id($id);
                Ok($body)
            }
            1 => {
                let $h = Sound::from_id($id);
                Ok($body)
            }
            2 => {
                let $h = Font::from_id($id);
                Ok($body)
            }
            3 => {
                let $h = Mesh::from_id($id);
                Ok($body)
            }
            4 => {
                let $h = Material::from_id($id);
                Ok($body)
            }
            5 => {
                let $h = Shader::from_id($id);
                Ok($body)
            }
            6 => {
                let $h = Compute::from_id($id);
                Ok($body)
            }
            7 => {
                let $h = Effect::from_id($id);
                Ok($body)
            }
            8 => {
                let $h = GpuBuffer::from_id($id);
                Ok($body)
            }
            9 => {
                let $h = InstanceBatch::from_id($id);
                Ok($body)
            }
            10 => {
                let $h = TextureArray::from_id($id);
                Ok($body)
            }
            _ => Err(invalid("unknown handle kind")),
        }
    };
}

/// Whether the handle of `kind` (a `CrabbyHandleKind`) with `id` names
/// something this game made and hasn't freed (Rust `Ctx::is_live`). False
/// for `CRABBY_HANDLE_NONE`.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut live = true;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_handle_is_live(std::ptr::null_mut(), 0, 0, &mut live) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_handle_is_live(
    ctx: *mut CrabbyCtx,
    kind: u32,
    id: u32,
    out_live: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let live = with_handle!(kind, id, |h| c.is_live(h))?;
            out(out_live, live)
        })
    })
}

/// Frees the handle of `kind` (a `CrabbyHandleKind`) with `id` (Rust
/// `Ctx::free`): its memory goes, and its id is never reused (except for
/// meshes and GPU buffers, whose next one made takes it). Whatever still uses it draws the default: nothing for a
/// mesh, white for a texture, the default material, the built-in shader or
/// font; effects and compute passes are skipped and sounds stop. Freeing it
/// again, or `CRABBY_HANDLE_NONE`, does nothing.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_handle_free(std::ptr::null_mut(), 0, 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_handle_free(ctx: *mut CrabbyCtx, kind: u32, id: u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| with_handle!(kind, id, |h| c.free(h))) })
}

/// How many handles of each kind are live now (Rust `Ctx::live_counts`).
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut counts = ponko_ffi::CrabbyLiveCounts::default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_live_counts(std::ptr::null_mut(), &mut counts) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_live_counts(
    ctx: *mut CrabbyCtx,
    out_counts: *mut CrabbyLiveCounts,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_counts, c.live_counts().into())) })
}
