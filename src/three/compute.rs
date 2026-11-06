//! Compute passes: a game's WGSL run on the GPU once per texel of a render
//! target each frame, for simulations kept in textures (snow and mud
//! trails, wet ground, ripples, heat, fog of war).

use super::Material;
use crate::{Ctx, Texture};

/// A compute pass shader, made with [`Ctx::load_compute`] or
/// [`Ctx::create_compute`]; a cheap handle. Run it by adding a
/// [`ComputePass`] to [`Ctx::compute_passes`](crate::Ctx::compute_passes).
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let melt = ctx.load_compute("compute/melt.wgsl");
/// let snow = ctx.create_render_target(512, 512);
/// ctx.compute_passes.push(ComputePass::new(melt, snow));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // shared with C/C++ (`CrabbyCompute`)
pub struct Compute {
    pub(crate) id: u32,
}

impl Compute {
    /// The engine's id of this compute pass (for the C API).
    #[doc(hidden)]
    #[must_use]
    pub fn id(self) -> u32 {
        self.id
    }

    /// A compute handle from an id (for the C API; check it with `Ctx::has_compute`).
    #[doc(hidden)]
    #[must_use]
    pub fn from_id(id: u32) -> Self {
        Self { id }
    }
}

/// One entry of [`Ctx::compute_passes`](crate::Ctx::compute_passes): a
/// shader, the render target it updates, and a material whose custom
/// values and textures it reads.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx, trails: Compute, snow: Texture, player: Vec3) {
/// let feet = ctx.create_material(MaterialDesc {
///     custom: [player.extend(0.5), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO],
///     ..Default::default()
/// });
/// ctx.compute_passes.push(ComputePass::new(trails, snow).with_material(feet));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputePass {
    /// The shader.
    pub compute: Compute,
    /// What it updates: made by [`Ctx::create_render_target`]; other
    /// textures are skipped.
    pub target: Texture,
    /// Storage buffer target, instead of `target`, when present.
    pub buffer: Option<GpuBuffer>,
    /// Where its values and textures come from. `None`: the default
    /// material (all custom values 0).
    pub material: Option<Material>,
}

impl ComputePass {
    /// Runs `fn update(i: u32)` once per buffer element, in groups of 64.
    /// The shader can read and write `items[i]` and read `frame` and `material`.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, c: Compute) {
    /// let b = ctx.create_gpu_buffer(64, &[]);
    /// ctx.compute_passes.push(ComputePass::for_buffer(c, b));
    /// # }
    pub fn for_buffer(compute: Compute, buffer: GpuBuffer) -> Self {
        Self {
            compute,
            target: Texture::new(0, 0, 0),
            buffer: Some(buffer),
            material: None,
        }
    }

    /// Runs `compute` on `target` with no values of its own.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, c: Compute, t: Texture) {
    /// ctx.compute_passes.push(ComputePass::new(c, t));
    /// # }
    /// ```
    #[must_use]
    pub fn new(compute: Compute, target: Texture) -> Self {
        Self {
            compute,
            target,
            buffer: None,
            material: None,
        }
    }

    /// The same, reading its values and textures from `material`; change
    /// them with [`Ctx::update_material`] to steer the simulation.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(c: Compute, t: Texture, m: Material) {
    /// let pass = ComputePass::new(c, t).with_material(m);
    /// # }
    /// ```
    #[must_use]
    pub fn with_material(self, material: Material) -> Self {
        Self {
            material: Some(material),
            ..self
        }
    }
}

impl Ctx {
    /// Loads a compute pass shader (WGSL) from `assets/`. It defines
    /// `fn compute(c: Cell) -> vec4<f32>`: the new value of each texel of
    /// the render target, from what it holds now (`c.value`, all 0 at
    /// first), where it is (`c.pixel`, `c.size`, `c.uv`) and its
    /// neighbours (`cell_at(pixel)`, which reads the target as it was
    /// before this pass). It reads `frame` (`frame.time.y` is the frame's
    /// seconds) and `material` as material shaders do. With hot reload on,
    /// saving the file updates the game (an error keeps the old one).
    /// Loading the same file again returns the same compute pass.
    ///
    /// For GPU buffers, define `fn update(i: u32)` instead, accessing the
    /// read-write `items: array<vec4<f32>>`. Schedule it with
    /// [`ComputePass::for_buffer`]; it shares the texture passes' frame uniforms
    /// and ordering. WebGL2 skips buffer passes and buffer-driven draws.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let ripples = ctx.load_compute("compute/ripples.wgsl");
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the file can't be read or doesn't compile (the message says where).
    pub fn load_compute(&mut self, path: &str) -> Compute {
        self.try_load_compute(path)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// [`Ctx::load_compute`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_load_compute(&mut self, path: &str) -> Result<Compute, String> {
        let file = crate::assets::resolve(path)?;
        if let Some(&compute) = self.assets3d.loaded_computes.get(&file) {
            return Ok(compute);
        }
        let source = crate::assets::read_to_string(&file)?;
        let compute = self.try_create_compute(path, &source)?;
        self.assets3d.loaded_computes.insert(file.clone(), compute);
        self.watcher
            .watch(path, file, crate::hot_reload::Asset::Compute(compute.id));
        Ok(compute)
    }

    /// Makes a compute pass shader from WGSL source; `name` is for error
    /// messages. See [`Ctx::load_compute`] for what it defines.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     // Fades toward 0 by half a second.
    ///     let fade = ctx.create_compute("fade", "
    ///         fn compute(c: Cell) -> vec4<f32> {
    ///             return c.value * max(1.0 - 2.0 * frame.time.y, 0.0);
    ///         }");
    ///     let target = ctx.create_render_target(64, 64);
    ///     ctx.compute_passes.push(ComputePass::new(fade, target));
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// If it doesn't compile (the message says where).
    pub fn create_compute(&mut self, name: &str, source: &str) -> Compute {
        self.try_create_compute(name, source)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// [`Ctx::create_compute`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_compute(&mut self, name: &str, source: &str) -> Result<Compute, String> {
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_compute(source))?;
        let id = self.assets3d.computes;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let gpu_id = renderer.create_compute(name, source)?;
            debug_assert_eq!(gpu_id, id);
        }
        self.assets3d.computes += 1;
        Ok(Compute { id })
    }

    /// Recompiles a compute pass whose file changed (hot reload).
    pub(crate) fn reload_compute(&mut self, id: u32, name: &str, source: &str) -> Result<(), String> {
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_compute(source))?;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.replace_compute(id, name, source)?;
        }
        Ok(())
    }

    /// Whether this game made `compute` (for the C API, which takes ids).
    #[doc(hidden)]
    #[must_use]
    pub fn has_compute(&self, compute: Compute) -> bool {
        compute.id < self.assets3d.computes
    }
}

/// A GPU array of `vec4<f32>`. Empty initial data creates zeroed elements.
/// ```
/// # use crabbyengine::*;
/// Game::new("buffers", 1, 1).run_headless(0, |ctx| {
///     let b = ctx.create_gpu_buffer(128, &[]);
///     ctx.write_gpu_buffer(b, 0, &[Vec4::ONE]);
/// });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct GpuBuffer {
    pub(crate) id: u32,
}
impl GpuBuffer {
    /// Internal handle ID for language bindings.
    #[doc(hidden)]
    pub fn id(self) -> u32 {
        self.id
    }
    /// Internal handle conversion for language bindings; validate before use.
    #[doc(hidden)]
    pub fn from_id(id: u32) -> Self {
        Self { id }
    }
}
impl Ctx {
    /// Creates `len` elements. Initial data may be empty or exactly `len` elements.
    /// Panics for zero length or mismatched data. See [`GpuBuffer`].
    pub fn create_gpu_buffer(&mut self, len: u32, data: &[crate::Vec4]) -> GpuBuffer {
        assert!(
            len > 0 && (data.is_empty() || data.len() == len as usize),
            "invalid GPU buffer length"
        );
        let id = self.assets3d.gpu_buffers.len() as u32;
        if let Some(r) = self.renderer.as_mut()
            && !self.headless
        {
            r.create_gpu_buffer(len, data);
        }
        self.assets3d.gpu_buffers.push(len);
        GpuBuffer { id }
    }
    /// Number of elements, or `None` for an invalid handle.
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    /// let b = ctx.create_gpu_buffer(2, &[]); assert_eq!(ctx.gpu_buffer_len(b), Some(2));
    /// });
    /// ```
    pub fn gpu_buffer_len(&self, b: GpuBuffer) -> Option<u32> {
        self.assets3d.gpu_buffers.get(b.id as usize).copied()
    }
    /// Updates a range before the next frame. Panics for invalid handles or ranges.
    /// See [`GpuBuffer`] for an example.
    pub fn write_gpu_buffer(&mut self, b: GpuBuffer, offset: u32, data: &[crate::Vec4]) {
        let len = self.gpu_buffer_len(b).expect("invalid GPU buffer");
        assert!(
            offset <= len && data.len() <= (len - offset) as usize,
            "GPU buffer write out of bounds"
        );
        if !data.is_empty()
            && let Some(r) = self.renderer.as_mut()
            && !self.headless
        {
            r.write_gpu_buffer(b.id, offset, data);
        }
    }
    /// Draws a mesh as `count` GPU instances, using `items[v.instance_index]`
    /// in its custom vertex shader. Instances use identity transforms and white
    /// tint; the model's transform only controls CPU visibility. Use a generous
    /// mesh bound and disable occlusion culling for shader-displaced geometry.
    /// Unsupported GPUs draw nothing. Panics for invalid handles or count.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, mesh: Mesh) {
    /// let b = ctx.create_gpu_buffer(100, &[]);
    /// ctx.set_mesh_gpu_buffer(mesh, b, 100);
    /// # }
    /// ```
    pub fn set_mesh_gpu_buffer(&mut self, mesh: super::Mesh, b: GpuBuffer, count: u32) {
        assert!(self.is_mesh_live(mesh), "invalid mesh");
        assert!(
            count <= self.gpu_buffer_len(b).expect("invalid GPU buffer"),
            "GPU instance count out of bounds"
        );
        if let Some(r) = self.renderer.as_mut()
            && !self.headless
        {
            r.set_mesh_gpu_buffer(mesh.id, b.id, count);
        }
    }
}
