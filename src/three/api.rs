//! The 3D methods on [`Ctx`]: making meshes and materials, loading models.

use std::ops::Range;

use super::{Material, MaterialDesc, Mesh, MeshData, gltf, linear};
use crate::{Ctx, assets};

/// A material as the GPU reads it (one uniform block).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MaterialUniform {
    /// Linear RGBA.
    pub color: [f32; 4],
    /// Linear RGB, then unused.
    pub emissive: [f32; 4],
    /// roughness, metallic, unused, unused.
    pub params: [f32; 4],
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
        assert!(
            (0.0..=1.0).contains(&desc.roughness) && (0.0..=1.0).contains(&desc.metallic),
            "crabbyengine: material roughness and metallic go from 0 to 1 (got {} and {})",
            desc.roughness,
            desc.metallic
        );
        let id = self.assets3d.materials;
        self.assets3d.materials += 1;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let [r, g, b, _] = linear(desc.emissive);
            let uniform = MaterialUniform {
                color: linear(desc.color),
                emissive: [r, g, b, 0.0],
                params: [desc.roughness.max(0.03), desc.metallic, 0.0, 0.0],
            };
            let gpu_id = renderer.create_material(&uniform, desc.texture.map(|t| t.id));
            debug_assert_eq!(gpu_id, id);
        }
        Material { id }
    }

    /// A plain white material, made once and shared.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let ball = ctx.create_mesh(&MeshData::sphere(0.5, 32), ctx.default_material());
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
    /// If the data doesn't fit together (see [`MeshData::check`]).
    pub fn create_mesh(&mut self, data: &MeshData, material: Material) -> Mesh {
        if let Err(e) = data.check() {
            panic!("crabbyengine: create_mesh: {e}");
        }
        self.mesh_from_parts(data, &[(0..data.indices.len() as u32, material)])
    }

    /// Loads a glTF model (`.glb`, or `.gltf` with its files next to it)
    /// from `assets/` as one [`Mesh`]: every mesh in the file's scene, where
    /// the file puts it, with its materials and base color textures.
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
        let loaded = gltf::load(path, &bytes, &mut |name| {
            let file = assets::resolve(name)?;
            assets::read(&file).map_err(|e| format!("could not read {}: {e}", file.display()))
        })?;
        let materials: Vec<Material> = loaded
            .materials
            .iter()
            .map(|m| {
                let texture = m.image.as_ref().map(|(w, h, rgba)| self.create_texture(*w, *h, rgba));
                self.create_material(MaterialDesc {
                    color: m.color,
                    texture,
                    roughness: m.roughness.clamp(0.0, 1.0),
                    metallic: m.metallic.clamp(0.0, 1.0),
                    emissive: m.emissive,
                })
            })
            .collect();
        let parts: Vec<(Range<u32>, Material)> = loaded
            .parts
            .iter()
            .map(|(range, m)| (range.clone(), materials[*m]))
            .collect();
        let mesh = self.mesh_from_parts(&loaded.data, &parts);
        self.assets3d.loaded.insert(file, mesh);
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
    pub(crate) fn prepare_3d(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        if self.assets3d.bounds.is_empty() {
            return; // no meshes, no 3D pass
        }
        let view_proj = self.camera3d.view_proj();
        self.frame3d.collect(&mut self.world, &self.assets3d, view_proj);
        renderer.prepare_3d(&self.frame3d, &self.lighting, self.camera3d.position);
    }
}
