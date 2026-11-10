//! Compute passes: a game's WGSL run on the GPU once per texel of a render
//! target each frame, for simulations kept in textures (snow and mud
//! trails, wet ground, ripples, heat, fog of war).

use super::Material;
use crate::gfx::Graphics;
use crate::{Texture, Vec3};

/// A compute pass shader, made with [`Graphics::load_compute`](crate::Graphics::load_compute) or
/// [`Graphics::create_compute`](crate::Graphics::create_compute); a cheap handle. Run it by adding a
/// [`ComputePass`] to [`Ctx::compute_passes`](crate::Ctx::compute_passes).
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let melt = ctx.gfx.load_compute("compute/melt.wgsl");
/// let snow = ctx.gfx.create_render_target(512, 512);
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

    /// A compute handle from an id (for the C API; check it with `Graphics::has_compute`).
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
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx, trails: Compute, snow: Texture, player: Vec3) {
/// let feet = ctx.gfx.create_material(MaterialDesc {
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
    /// What it updates: made by [`Graphics::create_render_target`](crate::Graphics::create_render_target); other
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
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, c: Compute) {
    /// let b = ctx.gfx.create_gpu_buffer(64, &[]);
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
    /// # use ponko::*;
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
    /// them with [`Graphics::update_material`](crate::Graphics::update_material) to steer the simulation.
    ///
    /// ```no_run
    /// # use ponko::*;
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

impl Graphics {
    /// Loads a compute pass shader (WGSL) from an [added asset
    /// source](crate::Assets::add_source) or `assets/`. It defines
    /// `fn compute(c: Cell) -> vec4<f32>`: the new value of each texel of
    /// the render target, from what it holds now (`c.value`, all 0 at
    /// first), where it is (`c.pixel`, `c.size`, `c.uv`) and its
    /// neighbours (`cell_at(pixel)`, which reads the target as it was
    /// before this pass). It reads `frame` (`frame.time.y` is the frame's
    /// seconds) and `material` as material shaders do. With hot reload on,
    /// saving the file in `assets/` updates the game (an error keeps the old one).
    /// Loading the same file again returns the same compute pass.
    ///
    /// For GPU buffers, define `fn update(i: u32)` instead, accessing the
    /// read-write `items: array<vec4<f32>>`. Schedule it with
    /// [`ComputePass::for_buffer`]; it shares the texture passes' frame uniforms
    /// and ordering. WebGL2 skips buffer passes and buffer-driven draws.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let ripples = ctx.gfx.load_compute("compute/ripples.wgsl");
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the file can't be read or doesn't compile (the message says where).
    /// With hot reload on, a file that doesn't compile doesn't panic: it
    /// gets a stand-in and stays watched until a saved fix compiles
    /// ([`ShaderOutcome::Failed`](crate::ShaderOutcome::Failed), [`Graphics::shader_status`](crate::Graphics::shader_status)).
    pub fn load_compute(&mut self, path: &str) -> Compute {
        self.try_load_compute(path).unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Graphics::load_compute`](crate::Graphics::load_compute) that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_load_compute(&mut self, path: &str) -> Result<Compute, String> {
        let found = match self
            .asset_sources
            .lookup(path, |origin| self.assets3d.loaded_computes.get(origin).copied())?
        {
            crate::assets::Lookup::Cached(_, compute) => return Ok(compute),
            crate::assets::Lookup::Found(found) => found,
        };
        let origin = found.origin.clone();
        let source = found.into_text(path)?;
        let compute = match self.try_create_compute(path, &source) {
            Ok(compute) => compute,
            // With hot reload on, it is skipped until a saved fix compiles;
            // its status says what's wrong.
            Err(e) if self.keeps_failed_load(&origin) => {
                Self::report_stand_in(path, &e);
                let id = self.assets3d.computes;
                self.compile_compute(path, super::COMPUTE_STAND_IN, None)
                    .map_err(|_| e)?;
                if let Some(renderer) = self.renderer.as_mut()
                    && !self.headless
                {
                    renderer.free_compute(id);
                }
                self.assets3d.computes += 1;
                self.assets3d.computes_live.insert(id);
                Compute { id }
            }
            Err(e) => return Err(e),
        };
        self.assets3d.loaded_computes.insert(origin.clone(), compute);
        if let crate::assets::Origin::File(file) = origin {
            self.watcher
                .watch(path, file, crate::hot_reload::Asset::Compute(compute.id));
        }
        Ok(compute)
    }

    /// Makes a compute pass shader from WGSL source; `name` is for error
    /// messages. See [`Graphics::load_compute`](crate::Graphics::load_compute) for what it defines.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     // Fades toward 0 by half a second.
    ///     let fade = ctx.gfx.create_compute("fade", "
    ///         fn compute(c: Cell) -> vec4<f32> {
    ///             return c.value * max(1.0 - 2.0 * frame.time.y, 0.0);
    ///         }");
    ///     let target = ctx.gfx.create_render_target(64, 64);
    ///     ctx.compute_passes.push(ComputePass::new(fade, target));
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// If it doesn't compile (the message says where).
    pub fn create_compute(&mut self, name: &str, source: &str) -> Compute {
        self.try_create_compute(name, source)
            .unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Graphics::create_compute`](crate::Graphics::create_compute) that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_compute(&mut self, name: &str, source: &str) -> Result<Compute, String> {
        let id = self.assets3d.computes;
        let compiled = self.compile_compute(name, source, None);
        self.record_shader(name, super::ShaderKind::Compute, false, compiled)?;
        self.assets3d.computes += 1;
        self.assets3d.computes_live.insert(id);
        Ok(Compute { id })
    }

    /// Checks compute `source` and builds its pipeline, as `replace` or as the next one.
    fn compile_compute(
        &mut self,
        name: &str,
        source: &str,
        replace: Option<u32>,
    ) -> Result<(), Box<super::ShaderDiagnostic>> {
        #[cfg(feature = "window")]
        crate::render::diagnose(
            name,
            super::ShaderKind::Compute,
            crate::render::COMPUTE_VARIANT,
            &crate::render::compose_compute(source),
        )?;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let built = match replace {
                Some(id) => renderer.replace_compute(id, name, source),
                None => renderer
                    .create_compute(name, source)
                    .map(|gpu_id| debug_assert_eq!(gpu_id, self.assets3d.computes)),
            };
            built?;
        }
        let _ = (name, source, replace);
        Ok(())
    }

    /// Recompiles a compute pass whose file changed (hot reload).
    pub(crate) fn reload_compute(&mut self, id: u32, name: &str, source: &str) -> Result<(), String> {
        if !self.assets3d.computes_live.contains(id) {
            return Ok(());
        }
        let compiled = self.compile_compute(name, source, Some(id));
        self.record_shader(name, super::ShaderKind::Compute, true, compiled)
    }

    /// Whether this game made `compute` and hasn't freed it (for the C API,
    /// which takes ids): [`Ctx::is_live`](crate::Ctx::is_live).
    #[doc(hidden)]
    #[must_use]
    pub fn has_compute(&self, compute: Compute) -> bool {
        self.assets3d.computes_live.contains(compute.id)
    }

    /// Frees compute pass `id` (see [`Ctx::free`](crate::Ctx::free)): entries of
    /// [`Ctx::compute_passes`](crate::Ctx::compute_passes) running it are skipped from now on.
    pub(crate) fn free_compute_id(&mut self, id: u32) {
        if !self.assets3d.computes_live.remove(id) {
            return;
        }
        self.assets3d.loaded_computes.retain(|_, c| c.id != id);
        self.watcher
            .forget(|a| matches!(a, crate::hot_reload::Asset::Compute(c) if *c == id));
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.free_compute(id);
        }
    }
}

/// A GPU array of `vec4<f32>`. Empty initial data creates zeroed elements.
/// ```
/// # use ponko::*;
/// Game::new("buffers", 1, 1).run_headless(0, |ctx| {
///     let b = ctx.gfx.create_gpu_buffer(128, &[]);
///     ctx.gfx.write_gpu_buffer(b, 0, &[Vec4::ONE]);
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
impl Graphics {
    /// Creates `len` elements. Initial data may be empty or exactly `len` elements.
    /// A freed buffer's id is reused first (see [`Ctx::free`](crate::Ctx::free)).
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
    /// # use ponko::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    /// let b = ctx.gfx.create_gpu_buffer(2, &[]); assert_eq!(ctx.gfx.gpu_buffer_len(b), Some(2));
    /// });
    /// ```
    pub fn gpu_buffer_len(&self, b: GpuBuffer) -> Option<u32> {
        self.assets3d
            .gpu_buffers
            .get(b.id as usize)
            .copied()
            .filter(|&len| len > 0)
    }
    /// Frees `b`'s GPU memory: the engine side of [`Ctx::free`](crate::Ctx::free) with a
    /// buffer, which also removes the compute passes that write to it.
    pub(crate) fn free_gpu_buffer(&mut self, b: GpuBuffer) {
        let Some(len) = self.assets3d.gpu_buffers.get_mut(b.id as usize) else {
            return;
        };
        if *len == 0 {
            return;
        }
        *len = 0;
        self.assets3d.free_gpu_buffers.push(b.id);
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
    /// [`Graphics::set_mesh_gpu_bounds`](crate::Graphics::set_mesh_gpu_bounds). Disable occlusion culling for
    /// shader-displaced geometry. Unsupported GPUs draw nothing. Panics for
    /// invalid handles or count.
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, mesh: Mesh) {
    /// let b = ctx.gfx.create_gpu_buffer(100, &[]);
    /// ctx.gfx.set_mesh_gpu_buffer(mesh, b, 100);
    /// ctx.gfx.set_mesh_gpu_bounds(mesh, Vec3::ZERO, 10.0);
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
    /// simulation (see [`Graphics::set_mesh_gpu_buffer`](crate::Graphics::set_mesh_gpu_buffer)) can put its instances.
    /// The result is the mesh's own bounds united with the sphere; calling it
    /// again replaces the sphere. Panics for a freed mesh or a negative or
    /// non-finite radius.
    /// ```
    /// # use ponko::*;
    /// Game::new("b", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.gfx.default_material();
    ///     let mesh = ctx.gfx.create_mesh(&MeshData::cube(0.1), material);
    ///     ctx.gfx.set_mesh_gpu_bounds(mesh, Vec3::Y * 5.0, 6.0);
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
            let material = ctx.gfx.default_material();
            let mesh = ctx.gfx.create_mesh(&crate::MeshData::cube(0.5), material);
            let own = ctx.gfx.assets3d.bounds[mesh.id as usize];
            ctx.gfx.set_mesh_gpu_bounds(mesh, Vec3::Y * 10.0, 2.0);
            let grown = ctx.gfx.assets3d.bounds[mesh.id as usize];
            assert!(grown.1 > own.1 + 5.0);
            // Calling again replaces the sphere instead of piling on.
            ctx.gfx.set_mesh_gpu_bounds(mesh, Vec3::Y * 10.0, 2.0);
            assert_eq!(ctx.gfx.assets3d.bounds[mesh.id as usize], grown);
            // A freed mesh's slot starts over.
            ctx.gfx.free_mesh(mesh);
            let again = ctx.gfx.create_mesh(&crate::MeshData::cube(0.5), material);
            assert_eq!((again, ctx.gfx.assets3d.bounds[again.id as usize]), (mesh, own));
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
