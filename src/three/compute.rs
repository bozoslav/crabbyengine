//! Compute passes: a game's WGSL run on the GPU once per texel of a render
//! target each frame, for simulations kept in textures (snow and mud
//! trails, wet ground, ripples, heat, fog of war).

use super::Material;
use crate::{Ctx, Texture, Vec3};

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
    /// A freed buffer's id is reused first (see [`Ctx::free_gpu_buffer`]).
    /// Panics for zero length or mismatched data. See [`GpuBuffer`].
    pub fn create_gpu_buffer(&mut self, len: u32, data: &[crate::Vec4]) -> GpuBuffer {
        assert!(
            len > 0 && (data.is_empty() || data.len() == len as usize),
            "invalid GPU buffer length"
        );
        let id = match self.assets3d.free_gpu_buffers.pop() {
            Some(id) => {
                self.assets3d.gpu_buffers[id as usize] = len;
                id
            }
            None => {
                self.assets3d.gpu_buffers.push(len);
                self.assets3d.gpu_buffers.len() as u32 - 1
            }
        };
        if let Some(r) = self.renderer.as_mut()
            && !self.headless
        {
            r.create_gpu_buffer(id, len, data);
        }
        GpuBuffer { id }
    }
    /// Number of elements, or `None` for an invalid or freed handle.
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    /// let b = ctx.create_gpu_buffer(2, &[]); assert_eq!(ctx.gpu_buffer_len(b), Some(2));
    /// });
    /// ```
    pub fn gpu_buffer_len(&self, b: GpuBuffer) -> Option<u32> {
        self.assets3d
            .gpu_buffers
            .get(b.id as usize)
            .copied()
            .filter(|&len| len > 0)
    }
    /// Frees `b`'s GPU memory, for effects that come and go. Its compute
    /// passes are removed from [`Ctx::compute_passes`] and meshes drawing
    /// from it draw nothing (they can be bound to another buffer again).
    /// Drop the handle: the next buffer made takes its id, and an old handle
    /// would reach that one. Freeing it again does nothing (until a new
    /// buffer takes its id).
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    ///     let b = ctx.create_gpu_buffer(8, &[]);
    ///     ctx.free_gpu_buffer(b);
    ///     assert_eq!(ctx.gpu_buffer_len(b), None);
    ///     // The id is reused.
    ///     assert_eq!(ctx.create_gpu_buffer(4, &[]), b);
    /// });
    /// ```
    pub fn free_gpu_buffer(&mut self, b: GpuBuffer) {
        let Some(len) = self.assets3d.gpu_buffers.get_mut(b.id as usize) else {
            return;
        };
        if *len == 0 {
            return;
        }
        *len = 0;
        self.assets3d.free_gpu_buffers.push(b.id);
        // Clear what pointed at it, so the id's next buffer starts clean.
        self.compute_passes.retain(|p| p.buffer != Some(b));
        if let Some(r) = self.renderer.as_mut()
            && !self.headless
        {
            r.free_gpu_buffer(b.id);
        }
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
    /// in its custom vertex shader. Every instance is drawn at its entity's
    /// transform and tint, like a normal model: the shader receives the
    /// vertex already placed there (`v.position`, `v.origin` is the entity's
    /// position) and moves it by what it reads from `items`, in world axes. A
    /// particle emitter therefore follows its entity. One draw per entity;
    /// use one entity per emitter.
    ///
    /// The entity is culled by the mesh's bounds, which don't know where the
    /// shader puts the instances: give the whole simulation's extent (in the
    /// entity's space, around the entity) with
    /// [`Ctx::set_mesh_gpu_bounds`]. Disable occlusion culling for
    /// shader-displaced geometry. Unsupported GPUs draw nothing. Panics for
    /// invalid handles or count.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, mesh: Mesh) {
    /// let b = ctx.create_gpu_buffer(100, &[]);
    /// ctx.set_mesh_gpu_buffer(mesh, b, 100);
    /// ctx.set_mesh_gpu_bounds(mesh, Vec3::ZERO, 10.0);
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
    /// Grows `mesh`'s culling bounds to also hold the sphere at `center`
    /// with `radius`, in the mesh's space: where a buffer-driven
    /// simulation (see [`Ctx::set_mesh_gpu_buffer`]) can put its instances.
    /// The result is the mesh's own bounds united with the sphere; calling it
    /// again replaces the sphere. Panics for a freed mesh or a negative or
    /// non-finite radius.
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.default_material();
    ///     let mesh = ctx.create_mesh(&MeshData::cube(0.1), material);
    ///     ctx.set_mesh_gpu_bounds(mesh, Vec3::Y * 5.0, 6.0);
    /// });
    /// ```
    pub fn set_mesh_gpu_bounds(&mut self, mesh: super::Mesh, center: Vec3, radius: f32) {
        assert!(
            radius >= 0.0 && radius.is_finite() && center.is_finite(),
            "invalid bounds"
        );
        assert!(self.is_mesh_live(mesh), "invalid mesh");
        let own = *self
            .assets3d
            .gpu_mesh_bounds
            .entry(mesh.id)
            .or_insert_with(|| self.assets3d.bounds[mesh.id as usize]);
        self.assets3d.bounds[mesh.id as usize] = union_spheres(own, (center, radius));
    }
}

/// The smallest sphere holding both spheres (center, radius).
fn union_spheres(a: (Vec3, f32), b: (Vec3, f32)) -> (Vec3, f32) {
    let d = b.0 - a.0;
    let distance = d.length();
    if distance + b.1 <= a.1 {
        return a;
    }
    if distance + a.1 <= b.1 {
        return b;
    }
    let radius = (distance + a.1 + b.1) * 0.5;
    (a.0 + d * ((radius - a.1) / distance), radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3;

    #[test]
    fn gpu_bounds_grow_the_culling_sphere_of_a_buffer_mesh() {
        crate::Game::new("bounds", 1, 1).run_headless(0, |ctx| {
            let material = ctx.default_material();
            let mesh = ctx.create_mesh(&crate::MeshData::cube(0.5), material);
            let own = ctx.assets3d.bounds[mesh.id as usize];
            ctx.set_mesh_gpu_bounds(mesh, Vec3::Y * 10.0, 2.0);
            let grown = ctx.assets3d.bounds[mesh.id as usize];
            assert!(grown.1 > own.1 + 5.0);
            // Calling again replaces the sphere instead of piling on.
            ctx.set_mesh_gpu_bounds(mesh, Vec3::Y * 10.0, 2.0);
            assert_eq!(ctx.assets3d.bounds[mesh.id as usize], grown);
            // A freed mesh's slot starts over.
            ctx.free_mesh(mesh);
            let again = ctx.create_mesh(&crate::MeshData::cube(0.5), material);
            assert_eq!((again, ctx.assets3d.bounds[again.id as usize]), (mesh, own));
        });
    }

    #[test]
    fn union_holds_both_spheres() {
        let (a, b) = ((vec3(0.0, 0.0, 0.0), 1.0), (vec3(10.0, 0.0, 0.0), 2.0));
        let (c, r) = union_spheres(a, b);
        assert!(c.distance(a.0) + a.1 <= r + 1e-4 && c.distance(b.0) + b.1 <= r + 1e-4);
        assert!((r - 6.5).abs() < 1e-4);
        assert_eq!(union_spheres(a, (vec3(0.5, 0.0, 0.0), 0.1)), a);
        assert_eq!(union_spheres((vec3(0.5, 0.0, 0.0), 0.1), a), a);
    }
}
