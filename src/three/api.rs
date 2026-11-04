//! The 3D methods on [`Ctx`]: making meshes and materials, loading models.

use std::ops::Range;

use glam::Vec3;

use super::{Blend, Material, MaterialDesc, Mesh, MeshData, Shader, gltf, linear};
use crate::{Ctx, assets};

/// A material as the GPU reads it (one uniform block).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MaterialUniform {
    /// Linear RGBA.
    pub color: [f32; 4],
    /// Linear RGB, then unused.
    pub emissive: [f32; 4],
    /// roughness, metallic, normal map strength, unused.
    pub params: [f32; 4],
    /// Which maps it has (1/0): normal, ORM, emissive, unused.
    pub maps: [u32; 4],
    /// For custom shaders.
    pub custom: [[f32; 4]; 4],
}

/// A material's textures, in binding order: base color, normal map, ORM
/// map, emissive map and two for custom shaders. `None`: the default.
pub(crate) type MaterialTextures = [Option<u32>; 6];

/// Everything the GPU needs to make or change a material.
pub(crate) struct GpuMaterialDesc {
    pub uniform: MaterialUniform,
    pub textures: MaterialTextures,
    /// Texture arrays for custom shaders, by id (`None`: a white one).
    pub arrays: [Option<u32>; 3],
    /// Index into the renderer's shaders (0: the built-in one).
    pub shader: u32,
    pub transparent: bool,
    pub cutout: bool,
    pub double_sided: bool,
}

impl GpuMaterialDesc {
    fn new(desc: &MaterialDesc) -> Self {
        let [r, g, b, _] = linear(desc.emissive);
        let id = |t: Option<crate::Texture>| t.map(|t| t.id);
        Self {
            uniform: MaterialUniform {
                color: linear(desc.color),
                emissive: [r, g, b, 0.0],
                params: [desc.roughness.max(0.03), desc.metallic, desc.normal_strength, 0.0],
                maps: [
                    u32::from(desc.normal_map.is_some()),
                    u32::from(desc.orm_map.is_some()),
                    u32::from(desc.emissive_map.is_some()),
                    0,
                ],
                custom: desc.custom.map(|v| v.to_array()),
            },
            textures: [
                id(desc.texture),
                id(desc.normal_map),
                id(desc.orm_map),
                id(desc.emissive_map),
                id(desc.custom_textures[0]),
                id(desc.custom_textures[1]),
            ],
            arrays: desc.custom_arrays.map(|a| a.map(|a| a.id)),
            shader: desc.shader.map_or(0, |s| s.id + 1),
            transparent: desc.blend == Blend::Transparent,
            cutout: desc.blend == Blend::Cutout,
            double_sided: desc.double_sided,
        }
    }
}

impl Ctx {
    /// Puts a material on the GPU: a color (and texture), how rough and how
    /// metallic, and light it gives off.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let bricks = ctx.load_texture("bricks.png");
    /// let wall = ctx.create_material(MaterialDesc { texture: Some(bricks), roughness: 0.9, ..Default::default() });
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If roughness or metallic is outside 0..=1.
    pub fn create_material(&mut self, desc: MaterialDesc) -> Material {
        self.check_material(&desc);
        let id = self.assets3d.materials;
        self.assets3d.materials += 1;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let gpu_id = renderer.create_material(&GpuMaterialDesc::new(&desc));
            debug_assert_eq!(gpu_id, id);
        }
        Material { id }
    }

    /// Changes a material: every model drawn with it changes too. Handy for
    /// animating a custom shader's values, or a glow.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, lamp: Material) {
    /// let on = MaterialDesc { emissive: Color::hex(0xFFD080), ..Default::default() };
    /// ctx.update_material(lamp, on);
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// As [`Ctx::create_material`], or if the material comes from another game.
    pub fn update_material(&mut self, material: Material, desc: MaterialDesc) {
        assert!(
            material.id < self.assets3d.materials,
            "crabbyengine: update_material: that material wasn't made by this game"
        );
        self.check_material(&desc);
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.update_material(material.id, &GpuMaterialDesc::new(&desc));
        }
    }

    fn check_material(&self, desc: &MaterialDesc) {
        assert!(
            (0.0..=1.0).contains(&desc.roughness) && (0.0..=1.0).contains(&desc.metallic),
            "crabbyengine: material roughness and metallic go from 0 to 1 (got {} and {})",
            desc.roughness,
            desc.metallic
        );
        if let Some(shader) = desc.shader {
            assert!(
                shader.id < self.assets3d.shaders,
                "crabbyengine: that shader wasn't made by this game"
            );
        }
        for array in desc.custom_arrays.into_iter().flatten() {
            assert!(
                self.has_texture_array(array),
                "crabbyengine: that texture array wasn't made by this game"
            );
        }
    }

    /// Loads a material shader (WGSL) from `assets/`. It can define
    /// `fn vertex(v: Vertex) -> Vertex`, to move vertices (wind, waves), and
    /// `fn surface(s: Surface) -> Surface`, to change how each pixel looks;
    /// the engine lights it, shadows it and does the rest. With hot reload
    /// on, saving the file updates the game (an error keeps the old one).
    /// Loading the same file again returns the same shader.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let wind = ctx.load_shader("shaders/wind.wgsl");
    /// let leaves = ctx.create_material(MaterialDesc { shader: Some(wind), double_sided: true, ..Default::default() });
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the file can't be read or doesn't compile (the message says where).
    pub fn load_shader(&mut self, path: &str) -> Shader {
        self.try_load_shader(path)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// [`Ctx::load_shader`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_load_shader(&mut self, path: &str) -> Result<Shader, String> {
        let file = crate::assets::resolve(path)?;
        if let Some(&shader) = self.assets3d.loaded_shaders.get(&file) {
            return Ok(shader);
        }
        let source = crate::assets::read_to_string(&file)?;
        let shader = self.try_create_shader(path, &source)?;
        self.assets3d.loaded_shaders.insert(file.clone(), shader);
        self.watcher
            .watch(path, file, crate::hot_reload::Asset::Shader(shader.id));
        Ok(shader)
    }

    /// Makes a material shader from WGSL source; `name` is for error
    /// messages. See [`Ctx::load_shader`] for what it can define.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let stripes = ctx.create_shader("stripes", "
    ///     fn surface(s: Surface) -> Surface {
    ///         var out = s;
    ///         out.albedo *= step(0.5, fract(s.uv.x * 8.0));
    ///         return out;
    ///     }");
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If it doesn't compile (the message says where).
    pub fn create_shader(&mut self, name: &str, source: &str) -> Shader {
        self.try_create_shader(name, source)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// [`Ctx::create_shader`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_shader(&mut self, name: &str, source: &str) -> Result<Shader, String> {
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_shader(source))?;
        let id = self.assets3d.shaders;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let gpu_id = renderer.create_shader(name, source)?;
            debug_assert_eq!(gpu_id, id + 1);
        }
        self.assets3d.shaders += 1;
        Ok(Shader { id })
    }

    /// Recompiles a shader whose file changed (hot reload).
    pub(crate) fn reload_shader(&mut self, id: u32, name: &str, source: &str) -> Result<(), String> {
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_shader(source))?;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.replace_shader(id + 1, name, source)?;
        }
        Ok(())
    }

    /// Whether this game made `material` (for the C API, which takes ids).
    #[doc(hidden)]
    #[must_use]
    pub fn has_material(&self, material: Material) -> bool {
        material.id < self.assets3d.materials
    }

    /// Whether this game made `shader` (for the C API, which takes ids).
    #[doc(hidden)]
    #[must_use]
    pub fn has_shader(&self, shader: Shader) -> bool {
        shader.id < self.assets3d.shaders
    }

    /// A plain white material, made once and shared.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let white = ctx.default_material();
    /// let ball = ctx.create_mesh(&MeshData::sphere(0.5, 32), white);
    /// # }
    /// ```
    pub fn default_material(&mut self) -> Material {
        if let Some(m) = self.assets3d.default_material {
            return m;
        }
        let m = self.create_material(MaterialDesc::default());
        self.assets3d.default_material = Some(m);
        m
    }

    /// Puts triangles on the GPU as a [`Mesh`] of one material. Draw it by
    /// spawning entities with a [`Model`](crate::Model) of it.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let stone = ctx.create_material(MaterialDesc::color(Color::hex(0x8A8A8A)));
    /// let block = ctx.create_mesh(&MeshData::cube(1.0), stone);
    /// ctx.world.spawn((Transform3D::default(), Model::new(block)));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the data doesn't fit together (see [`MeshData::check`]), or the
    /// material comes from another game.
    pub fn create_mesh(&mut self, data: &MeshData, material: Material) -> Mesh {
        if let Err(e) = data.check() {
            panic!("crabbyengine: create_mesh: {e}");
        }
        assert!(
            material.id < self.assets3d.materials,
            "crabbyengine: create_mesh: that material wasn't made by this game"
        );
        self.mesh_from_parts(data, &[(0..data.indices.len() as u32, material)])
    }

    /// Loads a glTF model (`.glb`, or `.gltf` with its files next to it)
    /// from `assets/` as one [`Mesh`]: every mesh in the file's scene, where
    /// the file puts it, with its materials and their base color, normal,
    /// occlusion-roughness-metalness and emissive maps.
    /// Loading the same file again returns the same mesh.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Forest", 1280, 720).run(|ctx| {
    ///     let tree = ctx.load_mesh("models/tree.glb");
    ///     for i in 0..100 {
    ///         let at = vec3((i % 10) as f32 * 4.0, 0.0, (i / 10) as f32 * -4.0);
    ///         ctx.world.spawn((Transform3D::at(at), Model::new(tree)));
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// With a clear message if the file is missing or can't be read.
    pub fn load_mesh(&mut self, path: &str) -> Mesh {
        self.try_load_mesh(path).unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    pub(crate) fn try_load_mesh(&mut self, path: &str) -> Result<Mesh, String> {
        let file = assets::resolve(path)?;
        if let Some(&mesh) = self.assets3d.loaded.get(&file) {
            return Ok(mesh);
        }
        let bytes = assets::read(&file).map_err(|e| format!("could not read {}: {e}", file.display()))?;
        let shared = &self.assets3d.shared_images;
        let loaded = gltf::load(
            path,
            &bytes,
            &mut |name| {
                let file = assets::resolve(name)?;
                assets::read(&file).map_err(|e| format!("could not read {}: {e}", file.display()))
            },
            &|source| shared.contains_key(source),
        )?;
        let textures: Vec<crate::Texture> = loaded
            .images
            .iter()
            .zip(&loaded.image_sources)
            .map(|((w, h, rgba), source)| match source {
                Some(s) => match self.assets3d.shared_images.get(s) {
                    Some(&t) => t,
                    None => {
                        let t = self.create_texture(*w, *h, rgba);
                        self.assets3d.shared_images.insert(s.clone(), t);
                        t
                    }
                },
                None => self.create_texture(*w, *h, rgba),
            })
            .collect();
        let materials: Vec<Material> = loaded
            .materials
            .iter()
            .map(|m| {
                let texture = |i: Option<usize>| i.map(|i| textures[i]);
                let desc = MaterialDesc {
                    color: m.color,
                    texture: texture(m.image),
                    roughness: m.roughness.clamp(0.0, 1.0),
                    metallic: m.metallic.clamp(0.0, 1.0),
                    emissive: m.emissive,
                    normal_map: texture(m.normal_map),
                    normal_strength: m.normal_strength,
                    orm_map: texture(m.orm_map),
                    emissive_map: texture(m.emissive_map),
                    blend: m.blend,
                    double_sided: m.double_sided,
                    ..Default::default()
                };
                let material = self.create_material(desc);
                if !m.name.is_empty() {
                    self.assets3d
                        .loaded_materials
                        .entry(m.name.clone())
                        .or_default()
                        .push((material, desc));
                }
                material
            })
            .collect();
        let parts: Vec<(Range<u32>, Material)> = loaded
            .parts
            .iter()
            .map(|(range, m)| (range.clone(), materials[*m]))
            .collect();
        let mesh = self.mesh_from_parts(&loaded.data, &parts);
        self.assets3d.loaded.insert(file, mesh);
        if let Some(names) = self.world.get_resource_mut::<crate::reflect::MeshNames>() {
            names.by_id.insert(mesh.id, path.to_string());
            names.by_name.insert(path.to_string(), mesh);
        }
        Ok(mesh)
    }

    /// A mesh whose index ranges each have their own material.
    fn mesh_from_parts(&mut self, data: &MeshData, parts: &[(Range<u32>, Material)]) -> Mesh {
        let id = self.assets3d.bounds.len() as u32;
        self.assets3d.bounds.push(data.bounds());
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let gpu_id = renderer.create_mesh(data, parts);
            debug_assert_eq!(gpu_id, id);
        }
        Mesh { id }
    }

    /// Every material loaded from glTF files so far that the files call
    /// `name`, with what it was made from: to give materials of imported
    /// models a shader of your own, keep their textures and change the
    /// rest with [`Ctx::update_material`]. Models loaded later aren't in it.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.load_scene("town/Town.json").unwrap();
    /// let brick = ctx.load_shader("shaders/brick.wgsl");
    /// for (material, desc) in ctx.loaded_materials("Brick") {
    ///     ctx.update_material(material, MaterialDesc { shader: Some(brick), ..desc });
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn loaded_materials(&self, name: &str) -> Vec<(Material, MaterialDesc)> {
        self.assets3d.loaded_materials.get(name).cloned().unwrap_or_default()
    }

    /// Frees `mesh`'s GPU memory, for worlds streamed in and out. Models
    /// still showing it draw nothing; despawn them too. A mesh loaded from
    /// a file loads afresh next time. Freeing it again does nothing.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.default_material();
    ///     let rock = ctx.create_mesh(&MeshData::cube(1.0), material);
    ///     ctx.free_mesh(rock);
    ///     assert!(!ctx.is_mesh_live(rock));
    /// });
    /// ```
    pub fn free_mesh(&mut self, mesh: Mesh) {
        let Some(bounds) = self.assets3d.bounds.get_mut(mesh.id as usize) else {
            return;
        };
        if bounds.1 < 0.0 {
            return;
        }
        *bounds = (Vec3::ZERO, -1.0);
        self.assets3d.loaded.retain(|_, m| *m != mesh);
        if let Some(names) = self.world.get_resource_mut::<crate::reflect::MeshNames>()
            && let Some(name) = names.by_id.remove(&mesh.id)
        {
            names.by_name.remove(&name);
        }
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.free_mesh(mesh.id);
        }
    }

    /// Whether `mesh` was made by this game and not freed.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.default_material();
    ///     let cube = ctx.create_mesh(&MeshData::cube(1.0), material);
    ///     assert!(ctx.is_mesh_live(cube));
    /// });
    /// ```
    #[must_use]
    pub fn is_mesh_live(&self, mesh: Mesh) -> bool {
        self.assets3d.live_bounds(mesh.id).is_some()
    }

    /// How many 3D models were drawn last frame: those the camera could see.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// println!("{} models on screen", ctx.models_drawn());
    /// # }
    /// ```
    #[must_use]
    pub fn models_drawn(&self) -> usize {
        self.frame3d.instances.len()
    }

    /// Engine side, before drawing: collect the visible models and hand
    /// them, the camera and the light to the renderer.
    /// What each target camera into a real render target sees.
    fn collect_views(&mut self) {
        let targets = &self.assets3d.render_targets;
        let cameras = self.target_cameras.iter().filter(|c| targets.contains(&c.target));
        let count = cameras.clone().count();
        let views = &mut self.frame3d.views;
        views.resize_with(count, Default::default);
        let lighting = super::Lighting {
            shadows: false,
            ..self.lighting
        };
        for (view, camera) in views.iter_mut().zip(cameras) {
            let size = (camera.target.width(), camera.target.height());
            let mut eye = camera.camera;
            eye.set_viewport(glam::Vec2::new(size.0 as f32, size.1 as f32));
            view.frame.collect(&mut self.world, &self.assets3d, &eye, &lighting);
            view.target = camera.target.id;
            view.size = size;
            view.camera = eye.position;
            view.view_proj = match camera.clip_plane {
                Some(plane) => super::target::oblique(eye.view_proj(), eye.view(), plane),
                None => eye.view_proj(),
            };
        }
    }

    pub(crate) fn prepare_3d(&mut self) {
        let time = (self.time() as f32, self.dt());
        if self.renderer.is_none() || self.assets3d.bounds.is_empty() {
            return; // no meshes, no 3D pass
        }
        let fallback = if self.effects.iter().any(|e| e.material.is_none())
            || self.compute_passes.iter().any(|c| c.material.is_none())
        {
            Some(self.default_material())
        } else {
            None
        };
        self.frame3d.effects.clear();
        self.frame3d.effects.extend(self.effects.iter().filter_map(|e| {
            let material = e.material.or(fallback)?;
            Some((e.effect.id, material.id))
        }));
        let start = web_time::Instant::now();
        self.collect_views();
        let targets = &self.assets3d.render_targets;
        self.frame3d.computes.clear();
        self.frame3d.computes.extend(self.compute_passes.iter().filter_map(|c| {
            let material = c.material.or(fallback)?;
            targets
                .contains(&c.target)
                .then_some((c.compute.id, c.target.id, material.id))
        }));
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        self.frame3d
            .collect(&mut self.world, &self.assets3d, &self.camera3d, &self.lighting);
        let light = if self.sky.atmosphere {
            match self.frame3d.sky_light {
                Some((lighting, sky, light)) if lighting == self.lighting && sky == self.sky => light,
                _ => {
                    let light = super::SkyLight::new(&self.lighting, &self.sky);
                    self.frame3d.sky_light = Some((self.lighting, self.sky, light));
                    light
                }
            }
        } else {
            let rgb = |c| {
                let [r, g, b, _] = super::linear(c);
                Vec3::new(r, g, b)
            };
            super::SkyLight {
                sun: rgb(self.lighting.sun_color) * self.lighting.sun_intensity,
                sky: rgb(self.lighting.sky_color),
                ground: rgb(self.lighting.ground_color),
            }
        };
        let collect_ms = start.elapsed().as_secs_f32() * 1000.0;
        let scene = super::Scene3D {
            frame: &self.frame3d,
            lighting: &self.lighting,
            sky: &self.sky,
            light,
            camera: self.camera3d.position,
            post: &self.post,
            time,
        };
        renderer.prepare_3d(&scene, collect_ms);
    }
}
