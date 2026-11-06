//! The 3D methods on [`Ctx`]: making meshes and materials, loading models.

use std::ops::Range;

use glam::Vec3;

use super::{
    AnimationClip, Blend, Material, MaterialDesc, Mesh, MeshData, MeshMorph, MeshSkin, Shader, Skeleton, gltf, linear,
};
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
    pub(crate) fn new(desc: &MaterialDesc) -> Self {
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
        self.mesh_from_parts(data, &[(0..data.indices.len() as u32, material)], None, Vec::new())
    }

    /// Makes a mesh that bends with `skeleton`: each corner of `data` has
    /// up to four [`joints`](MeshData::joints) (indices into the
    /// skeleton's joints) and how much each moves it. `data` is modelled
    /// with the skeleton at rest. Draw it with a [`Model`](crate::Model)
    /// and a [`Skeleton`] (this one, or any made from the same joints, or
    /// with joints of the same names) on the entity.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let skeleton = Skeleton::new(vec![
    ///         Joint { name: "base".into(), parent: None, rest: Transform3D::default() },
    ///         Joint { name: "top".into(), parent: Some(0), rest: Transform3D::at(vec3(0.0, 1.0, 0.0)) },
    ///     ]);
    ///     let mut post = MeshData::cube(0.2);
    ///     // Corners above the middle follow the top joint.
    ///     post.joints = post.positions.iter().map(|p| [if p.y > 0.0 { 1 } else { 0 }, 0, 0, 0]).collect();
    ///     post.weights = vec![[1.0, 0.0, 0.0, 0.0]; post.positions.len()];
    ///     let material = ctx.default_material();
    ///     let mesh = ctx.create_skinned_mesh(&post, material, &skeleton);
    ///     ctx.world.spawn((Transform3D::default(), Model::new(mesh), skeleton));
    /// });
    /// ```
    ///
    /// # Panics
    ///
    /// If the data doesn't fit together (see [`MeshData::check`]), has no
    /// joints, names a joint the skeleton doesn't have, or the material
    /// comes from another game.
    pub fn create_skinned_mesh(&mut self, data: &MeshData, material: Material, skeleton: &Skeleton) -> Mesh {
        if let Err(e) = data.check() {
            panic!("crabbyengine: create_skinned_mesh: {e}");
        }
        assert!(
            !data.joints.is_empty(),
            "crabbyengine: create_skinned_mesh: the mesh data has no joints (use create_mesh)"
        );
        let n = skeleton.joints().len();
        if let Some(bad) = data.joints.iter().flatten().find(|&&j| usize::from(j) >= n) {
            panic!("crabbyengine: create_skinned_mesh: joint {bad} is past the skeleton's {n} joints");
        }
        assert!(
            material.id < self.assets3d.materials,
            "crabbyengine: create_skinned_mesh: that material wasn't made by this game"
        );
        let rest = Skeleton::from_shared(skeleton.shared_joints().clone());
        let inverse_binds = (0..n).map(|j| rest.joint_matrix(j).inverse()).collect();
        let mut data = data.clone();
        for w in &mut data.weights {
            *w = super::skin::normalized(*w);
        }
        let skin = MeshSkin::new(
            skeleton.shared_joints().clone(),
            (0..n as u16).collect(),
            inverse_binds,
            &data,
        );
        self.mesh_from_parts(
            &data,
            &[(0..data.indices.len() as u32, material)],
            Some(skin),
            Vec::new(),
        )
    }

    /// The [`Skeleton`] of a glTF file with skins (loading its mesh with
    /// [`load_mesh`](Self::load_mesh) if that hasn't happened), at rest:
    /// every node of the file's scene is a joint, named as in the file.
    /// Put it on the entity with the file's [`Model`](crate::Model) and
    /// change its `pose` to bend the model.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let fox = ctx.load_mesh("models/fox.glb");
    /// let skeleton = ctx.load_skeleton("models/fox.glb");
    /// ctx.world.spawn((Transform3D::default(), Model::new(fox), skeleton));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// With a clear message if the file is missing, can't be read or has
    /// no skins.
    pub fn load_skeleton(&mut self, path: &str) -> Skeleton {
        self.try_load_skeleton(path)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    pub(crate) fn try_load_skeleton(&mut self, path: &str) -> Result<Skeleton, String> {
        self.try_load_mesh(path)?;
        let file = assets::resolve(path)?;
        self.assets3d
            .loaded_skeletons
            .get(&file)
            .cloned()
            .ok_or_else(|| format!("{path} has no skins or animations, so no skeleton"))
    }

    /// Loads all animation clips from a glTF file, cached per file.
    /// Panics with the loading error if the file cannot be loaded.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let clips = ctx.load_clips("models/character.glb");
    /// # }
    /// ```
    pub fn load_clips(&mut self, path: &str) -> Vec<AnimationClip> {
        self.try_load_clips(path)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// Fallible clip loading; a file without skins or animations returns an error.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let clips = ctx.try_load_clips("models/character.glb");
    /// # }
    /// ```
    pub fn try_load_clips(&mut self, path: &str) -> Result<Vec<AnimationClip>, String> {
        self.try_load_skeleton(path)?;
        let file = assets::resolve(path)?;
        Ok(self.assets3d.loaded_clips.get(&file).cloned().unwrap_or_default())
    }

    /// Loads a named clip. Panics with available names if it is missing.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let walk = ctx.load_clip("models/character.glb", "walk");
    /// # }
    /// ```
    pub fn load_clip(&mut self, path: &str, name: &str) -> AnimationClip {
        self.try_load_clip(path, name)
            .unwrap_or_else(|e| panic!("crabbyengine: {e}"))
    }

    /// Fallible named clip loading; errors list available names.
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let walk = ctx.try_load_clip("models/character.glb", "walk");
    /// # }
    /// ```
    pub fn try_load_clip(&mut self, path: &str, name: &str) -> Result<AnimationClip, String> {
        let clips = self.try_load_clips(path)?;
        clips.iter().find(|c| c.name() == name).cloned().ok_or_else(|| {
            let names: Vec<_> = clips.iter().map(AnimationClip::name).collect();
            format!("{path}: no animation {name:?}; available animations: {names:?}")
        })
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
        self.assets3d.loaded_clips.insert(file.clone(), loaded.clips);
        let skin = loaded.skin.map(|skin| {
            let skeleton = Skeleton::from_shared(skin.joints);
            let mesh_skin = MeshSkin::new(
                skeleton.shared_joints().clone(),
                skin.slots,
                skin.inverse_binds,
                &loaded.data,
            );
            self.assets3d.loaded_skeletons.insert(file.clone(), skeleton);
            mesh_skin
        });
        let mesh = self.mesh_from_parts(&loaded.data, &parts, skin, loaded.morph_defaults);
        self.assets3d.loaded.insert(file, mesh);
        if let Some(names) = self.world.get_resource_mut::<crate::reflect::MeshNames>() {
            names.by_id.insert(mesh.id, path.to_string());
            names.by_name.insert(path.to_string(), mesh);
        }
        Ok(mesh)
    }

    /// A glTF file's mesh on the CPU (all its parts in one, placed as in
    /// the file), without making anything on the GPU: to cut
    /// [decals](MeshData::decal) from, build collision shapes, or read its
    /// corners. Loads the file each call; keep the result.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) -> Result<(), String> {
    /// let wall = ctx.load_mesh_data("models/wall.glb")?;
    /// let hole = wall.decal(Mat4::from_scale(Vec3::splat(0.1)), 0.002);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// If the file is missing or isn't a glTF file with triangles.
    pub fn load_mesh_data(&self, path: &str) -> Result<MeshData, String> {
        let file = assets::resolve(path)?;
        let bytes = assets::read(&file).map_err(|e| format!("could not read {}: {e}", file.display()))?;
        let loaded = gltf::load(
            path,
            &bytes,
            &mut |name| {
                let file = assets::resolve(name)?;
                assets::read(&file).map_err(|e| format!("could not read {}: {e}", file.display()))
            },
            &|_| true,
        )?;
        Ok(loaded.data)
    }

    /// A mesh whose index ranges each have their own material.
    /// With `skin`, it bends with a skeleton. Its morph targets, if any,
    /// show `morph_defaults` on models without [`MorphWeights`](crate::MorphWeights).
    fn mesh_from_parts(
        &mut self,
        data: &MeshData,
        parts: &[(Range<u32>, Material)],
        skin: Option<MeshSkin>,
        morph_defaults: Vec<f32>,
    ) -> Mesh {
        // A freed mesh's id first, so streaming meshes in and out doesn't
        // grow the tables.
        let id = match self.assets3d.free_meshes.pop() {
            Some(id) => {
                self.assets3d.bounds[id as usize] = data.bounds();
                id
            }
            None => {
                self.assets3d.bounds.push(data.bounds());
                self.assets3d.bounds.len() as u32 - 1
            }
        };
        let posed = skin.is_some() || !data.morph_targets.is_empty();
        if let Some(skin) = skin {
            if self.assets3d.skins.len() <= id as usize {
                self.assets3d.skins.resize(id as usize + 1, None);
            }
            self.assets3d.skins[id as usize] = Some(std::sync::Arc::new(skin));
        }
        if let Some(morph) = MeshMorph::new(data, morph_defaults) {
            if self.assets3d.morphs.len() <= id as usize {
                self.assets3d.morphs.resize_with(id as usize + 1, || None);
            }
            self.assets3d.morphs[id as usize] = Some(std::sync::Arc::new(morph));
        }
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.create_mesh(id, data, parts, posed);
        }
        Mesh { id }
    }

    /// The index of `mesh`'s morph target called `name`, for
    /// [`MorphWeights`](crate::MorphWeights); `None` if it has no such
    /// target (or was freed).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let mut quad = MeshData::plane(1.0);
    ///     let up = vec![vec3(0.0, 0.1, 0.0); quad.positions.len()];
    ///     quad.morph_targets.push(MorphTarget { name: "lift".into(), positions: up, normals: Vec::new() });
    ///     let material = ctx.default_material();
    ///     let mesh = ctx.create_mesh(&quad, material);
    ///     assert_eq!(ctx.morph_target(mesh, "lift"), Some(0));
    ///     assert_eq!(ctx.morph_target(mesh, "sag"), None);
    /// });
    /// ```
    #[must_use]
    pub fn morph_target(&self, mesh: Mesh, name: &str) -> Option<usize> {
        self.assets3d.morph(mesh.id)?.names.iter().position(|n| n == name)
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
    /// still showing it draw nothing; despawn them too, and drop the handle:
    /// the next mesh made takes its id, and old handles would show that one.
    /// A mesh loaded from a file loads afresh next time. Freeing it again
    /// does nothing (until a new mesh takes its id).
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
        self.assets3d.free_meshes.push(mesh.id);
        if let Some(shadowless) = self.assets3d.shadowless.get_mut(mesh.id as usize) {
            *shadowless = false;
        }
        if let Some(skin) = self.assets3d.skins.get_mut(mesh.id as usize) {
            *skin = None;
        }
        if let Some(morph) = self.assets3d.morphs.get_mut(mesh.id as usize) {
            *morph = None;
        }
        let freed: Vec<_> = self
            .assets3d
            .loaded
            .iter()
            .filter(|(_, m)| **m == mesh)
            .map(|(f, _)| f.clone())
            .collect();
        for file in freed {
            self.assets3d.loaded_skeletons.remove(&file);
            self.assets3d.loaded_clips.remove(&file);
        }
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

    /// Whether models showing `mesh` cast shadows: on for every mesh at
    /// first. Turn it off for small things that crowd the ground, such as
    /// grass and pebbles: their shadows are too small to see but drawing
    /// them into the shadow map costs as much as drawing them.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.default_material();
    ///     let tuft = ctx.create_mesh(&MeshData::cube(0.2), material);
    ///     ctx.set_mesh_shadows(tuft, false);
    ///     assert!(!ctx.mesh_shadows(tuft));
    /// });
    /// ```
    pub fn set_mesh_shadows(&mut self, mesh: Mesh, casts: bool) {
        let id = mesh.id as usize;
        if id >= self.assets3d.bounds.len() {
            return;
        }
        let shadowless = &mut self.assets3d.shadowless;
        if shadowless.len() <= id {
            if casts {
                return;
            }
            shadowless.resize(id + 1, false);
        }
        shadowless[id] = !casts;
    }

    /// Whether models showing `mesh` cast shadows (see
    /// [`set_mesh_shadows`](Self::set_mesh_shadows)).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let material = ctx.default_material();
    ///     let house = ctx.create_mesh(&MeshData::cube(4.0), material);
    ///     assert!(ctx.mesh_shadows(house));
    /// });
    /// ```
    #[must_use]
    pub fn mesh_shadows(&self, mesh: Mesh) -> bool {
        self.assets3d.casts_shadows(mesh.id)
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

    /// Makes [`AntiAliasing::Taa`](super::AntiAliasing::Taa) forget the
    /// frames before this one: call it when the camera cuts to somewhere
    /// else (a teleport, a new scene), so the old view does not blend into
    /// the new one. The engine resets by itself on a window resize, when
    /// temporal antialiasing is turned on, and when the camera jumps far or
    /// turns sharply in one frame. Harmless with the other modes.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// Game::new("t", 1, 1).run_headless(1, |ctx| {
    ///     ctx.post.anti_aliasing = AntiAliasing::Taa;
    ///     ctx.camera3d.position = vec3(500.0, 2.0, 0.0);
    ///     ctx.reset_temporal_history();
    /// });
    /// ```
    pub fn reset_temporal_history(&mut self) {
        self.frame3d.motion.reset = true;
    }

    /// Engine side, before drawing: collect the visible models and hand
    /// them, the camera and the light to the renderer.
    /// What each target camera into a real render target sees.
    pub(crate) fn collect_views(&mut self, skins: &super::SkinFrame) {
        let targets = &self.assets3d.render_targets;
        let mut capture_cameras = Vec::new();
        self.frame3d.probe_bakes.clear();
        self.frame3d.probe_uploads.clear();
        self.assets3d.probes.take_requests();
        self.frame3d
            .probe_uploads
            .extend(self.assets3d.probes.in_flight_uploads.iter().cloned());
        for &(slot, center) in &self.assets3d.probes.in_flight {
            let faces = self.assets3d.probes.captures[slot as usize - 1];
            self.frame3d.probe_bakes.push((slot, faces.map(|t| t.id)));
            for (face, target) in faces.into_iter().enumerate() {
                let (direction, up) = match face {
                    0 => (Vec3::X, Vec3::Y),
                    1 => (-Vec3::X, Vec3::Y),
                    2 => (Vec3::Y, -Vec3::Z),
                    3 => (-Vec3::Y, Vec3::Z),
                    4 => (Vec3::Z, Vec3::Y),
                    _ => (-Vec3::Z, Vec3::Y),
                };
                let mut camera = self.camera3d;
                camera.position = center;
                camera.rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(direction.cross(up), up, -direction));
                camera.fov_y = std::f32::consts::FRAC_PI_2;
                camera.near = 0.05;
                capture_cameras.push(super::TargetCamera::new(camera, target));
            }
        }
        self.frame3d.probes.fill(Default::default());
        for (t, p) in self
            .world
            .query::<(&super::Transform3D, &super::ReflectionProbe3D)>()
            .iter()
        {
            if (1..=4).contains(&p.content) && p.weight(t.position, t.position) > 0.0 {
                self.frame3d.probes[p.content as usize - 1] = super::probe::ProbeUniform {
                    center: t.position.extend(p.content as f32).to_array(),
                    size: (p.size * 0.5).extend(p.blend_distance.max(0.0)).to_array(),
                    params: [p.intensity, f32::from(p.box_projection), 0.0, 0.0],
                };
            }
        }
        let cameras = self
            .target_cameras
            .iter()
            .chain(capture_cameras.iter())
            .filter(|c| targets.contains(&c.target));
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
            view.frame.lod = self.lod;
            view.frame
                .collect(&mut self.world, &self.assets3d, skins, &eye, &lighting);
            view.target = camera.target.id;
            view.size = size;
            view.camera = eye.position;
            view.view_proj = match camera.clip_plane {
                Some(plane) => super::target::oblique(eye.view_proj(), eye.view(), plane),
                None => eye.view_proj(),
            };
        }
    }

    /// What each mesh cost to draw in the last frame: the models and
    /// triangles the camera drew with it, and the same for the shadow maps,
    /// most triangles first. For finding what makes a frame slow; it
    /// allocates, so call it now and then, not every frame. Empty headless.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// fn heaviest(ctx: &mut Ctx) {
    ///     for c in ctx.mesh_costs().iter().take(5) {
    ///         let name = c.name.as_deref().unwrap_or("(made in code)");
    ///         println!("{name}: {} models, {} + {} shadow triangles", c.models, c.triangles, c.shadow_triangles);
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(heaviest).run_headless(1, |_| {});
    /// ```
    pub fn mesh_costs(&self) -> Vec<super::MeshCost> {
        let Some(renderer) = self.renderer.as_ref() else {
            return Vec::new();
        };
        let names: std::collections::HashMap<u32, &std::path::Path> = self
            .assets3d
            .loaded
            .iter()
            .map(|(path, mesh)| (mesh.id, path.as_path()))
            .collect();
        let mut costs: Vec<super::MeshCost> = renderer
            .mesh_costs()
            .into_iter()
            .map(
                |(id, models, triangles, shadow_models, shadow_triangles)| super::MeshCost {
                    mesh: super::Mesh { id },
                    name: names.get(&id).map(|p| p.display().to_string()),
                    models,
                    triangles,
                    shadow_models,
                    shadow_triangles,
                },
            )
            .collect();
        costs.sort_by_key(|c| std::cmp::Reverse(c.triangles + c.shadow_triangles));
        costs
    }

    /// Advances visual particle pools after frame systems.
    pub(crate) fn update_particles(&mut self) {
        let dt = self.dt();
        let mut emitters = self
            .world
            .query::<(&mut super::ParticleEmitter3D, &super::Transform3D)>();
        let mut any = emitters.count() > 0;
        emitters.par_for_each(|(emitter, transform)| emitter.update(dt, transform));
        let mut trails = self.world.query::<(&mut super::Trail3D, &super::Transform3D)>();
        any |= trails.count() > 0;
        trails.for_each(|(trail, transform)| trail.update(dt, transform.position));
        any |= self.world.query::<&super::Line3D>().count() > 0;
        if any && !self.headless && self.renderer.is_some() {
            self.default_material(); // also initializes a particle-only 3D renderer
        }
    }

    /// Advances every animator not in manual mode by this frame's time. The
    /// game runs it after the frame systems, so draw systems (and the
    /// renderer) see this frame's pose.
    pub(crate) fn update_animators(&mut self) {
        let dt = self.dt();
        #[cfg(feature = "physics3d")]
        let mut finished: Vec<crate::Entity> = Vec::new();
        #[cfg(feature = "physics3d")]
        {
            // Ragdolls pose their skeletons (after this frame's interpolation);
            // their animators wait, except while blending back.
            crate::physics3d::pose_ragdolls(&mut self.world, &mut self.physics3d.ragdolls);
            self.world
                .query::<(
                    crate::Entity,
                    &mut super::Animator,
                    &mut Skeleton,
                    &mut crate::Ragdoll3D,
                )>()
                .for_each(|(e, animator, skeleton, ragdoll)| {
                    if ragdoll.is_blending() {
                        if !animator.manual {
                            animator.update(dt, skeleton);
                        }
                        if crate::physics3d::blend_back(ragdoll, skeleton, dt) {
                            finished.push(e);
                        }
                    }
                });
            self.world
                .query::<(crate::Entity, &mut Skeleton, &mut crate::Ragdoll3D)>()
                .without::<super::Animator>()
                .for_each(|(e, skeleton, ragdoll)| {
                    if ragdoll.is_blending() && crate::physics3d::blend_back(ragdoll, skeleton, dt) {
                        finished.push(e);
                    }
                });
            for e in finished.drain(..) {
                self.world.remove::<crate::Ragdoll3D>(e);
            }
        }
        #[allow(unused_mut)]
        let mut animated = self
            .world
            .query::<(&mut super::Animator, &mut Skeleton, Option<&mut super::Transform3D>)>();
        #[cfg(feature = "physics3d")]
        let mut animated = animated.without::<crate::Ragdoll3D>();
        animated.for_each(|(animator, skeleton, transform)| {
            if animator.manual {
                return;
            }
            animator.update(dt, skeleton);
            if let (true, Some(t)) = (animator.apply_root_motion, transform) {
                let motion = animator.root_motion();
                t.position += t.rotation * (motion.position * t.scale);
                t.rotation = (t.rotation * motion.rotation).normalize();
            }
        });
        self.write_animated_morphs();
    }

    /// Writes sampled skeleton morph channels to models, including SkinOf
    /// attachments. Automatic animators call this each frame; call it after
    /// manual sampling. Unanimated weights retain their current values.
    /// ```
    /// use crabbyengine::*;
    /// Game::new("morph", 1, 1).run_headless(0, |ctx| ctx.write_animated_morphs());
    /// ```
    pub fn write_animated_morphs(&mut self) {
        let mut entities = std::mem::take(&mut self.assets3d.morph_entities);
        entities.clear();
        self.world
            .query::<(crate::Entity, &super::Model)>()
            .for_each(|(e, _)| entities.push(e));
        for e in entities.iter().copied() {
            let mesh_id = self.world.get::<super::Model>(e).unwrap().mesh.id;
            let Some(morph) = self.assets3d.morphs.get(mesh_id as usize).and_then(|m| m.clone()) else {
                continue;
            };
            let owner = self.world.get::<super::SkinOf>(e).map_or(e, |s| s.0);
            if self.world.get::<Skeleton>(owner).is_none_or(|s| s.morphs.is_empty()) {
                continue;
            }
            let mut weights = self
                .world
                .get_mut::<super::MorphWeights>(e)
                .map(|w| std::mem::take(&mut *w))
                .unwrap_or_else(|| super::MorphWeights(morph.defaults.clone()));
            let base = self
                .assets3d
                .morph_bases
                .entry(e)
                .or_insert_with(|| (mesh_id, weights.clone(), weights.clone()));
            if base.0 != mesh_id {
                base.0 = mesh_id;
                base.1.clone_from(&weights);
            }
            for i in 0..weights.0.len().max(base.2.0.len()) {
                if weights.get(i) != base.2.get(i) {
                    base.1.set(i, weights.get(i));
                }
            }
            self.world
                .get::<Skeleton>(owner)
                .unwrap()
                .morphs
                .write_base(&morph.names, &base.1, &mut weights);
            base.2.clone_from(&weights);
            if self.world.get::<super::MorphWeights>(e).is_some() {
                *self.world.get_mut::<super::MorphWeights>(e).unwrap() = weights;
            } else {
                self.world.insert(e, weights);
            }
        }
        self.assets3d.morph_bases.retain(|e, _| self.world.is_alive(*e));
        self.assets3d.morph_entities = entities;
    }

    pub(crate) fn prepare_3d(&mut self) {
        if self.renderer.is_none()
            || (self.assets3d.bounds.is_empty()
                && self.frame3d.particles.runs.is_empty()
                && self.frame3d.particles.strips.runs.is_empty()
                && self.world.query::<&super::ParticleEmitter3D>().count() == 0
                && self.world.query::<&super::Trail3D>().count() == 0
                && self.world.query::<&super::Line3D>().count() == 0
                && self.frame3d.particles.texts.vertices.is_empty()
                && self.world.query::<&super::Text3D>().count() == 0)
        {
            return; // no drawable 3D scene
        }
        let time = (self.time() as f32, self.dt());
        self.assets3d.decals_supported = self.supports_decals();
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
        let mut skins = std::mem::take(&mut self.frame3d.skins);
        skins.collect(&mut self.world, &self.assets3d);
        self.collect_views(&skins);
        let targets = &self.assets3d.render_targets;
        self.frame3d.computes.clear();
        self.frame3d.computes.extend(self.compute_passes.iter().filter_map(|c| {
            let material = c.material.or(fallback)?;
            targets
                .contains(&c.target)
                .then_some((c.compute.id, c.target.id, material.id))
        }));
        self.collect_text3d();
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        self.frame3d.size_limit = renderer.max_texture_size();
        self.frame3d.lod = self.lod;
        self.frame3d.motion.enabled = self.post.anti_aliasing == super::AntiAliasing::Taa;
        if !self.camera3d.occlusion_culling {
            self.frame3d.occlusion = None;
        } else if let Some(grid) = renderer.take_occlusion() {
            self.frame3d.occlusion = Some(grid);
        }
        self.frame3d
            .collect(&mut self.world, &self.assets3d, &skins, &self.camera3d, &self.lighting);
        self.frame3d.skins = skins;
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
            occlusion: self.camera3d.occlusion_culling,
        };
        renderer.prepare_3d(&scene, collect_ms);
        self.frame3d.motion.reset = false;
    }
}

#[cfg(test)]
mod clip_tests {
    use super::*;

    fn cached_clips() -> (Ctx, String) {
        let mut ctx = Ctx::new(1.0 / 60.0, glam::Vec2::ONE);
        // A real existing path, with its contents bypassed by the asset cache.
        let path = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
        let file = assets::resolve(&path).unwrap();
        let skeleton = Skeleton::new(vec![]);
        let clips = vec![
            AnimationClip::new("idle", &skeleton, vec![]),
            AnimationClip::new("walk", &skeleton, vec![]),
        ];
        ctx.assets3d.loaded.insert(file.clone(), Mesh { id: 0 });
        ctx.assets3d.loaded_skeletons.insert(file.clone(), skeleton);
        ctx.assets3d.loaded_clips.insert(file, clips);
        (ctx, path)
    }

    #[test]
    fn cached_clips_load_by_name_and_missing_names_list_choices() {
        let (mut ctx, path) = cached_clips();
        let clips = ctx.load_clips(&path);
        assert_eq!(clips.len(), 2);
        assert_eq!(ctx.load_clip(&path, "walk").name(), "walk");
        assert!(std::sync::Arc::ptr_eq(
            clips[0].source_joints(),
            ctx.load_clip(&path, "idle").source_joints()
        ));
        let error = ctx.try_load_clip(&path, "run").unwrap_err();
        assert!(error.contains("idle") && error.contains("walk") && error.contains("run"));
    }

    #[test]
    #[should_panic(expected = "available animations: [\"idle\", \"walk\"]")]
    fn missing_named_clip_panics_with_available_names() {
        let (mut ctx, path) = cached_clips();
        ctx.load_clip(&path, "run");
    }
}
