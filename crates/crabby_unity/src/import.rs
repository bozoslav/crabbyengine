//! Turning Unity scenes and prefabs into Crabby scenes: every drawn mesh
//! becomes a `.glb` with its materials, its textures become PNGs, and the
//! scene a `crabby-world` save of `Transform3D` and `Model` entities that
//! `Ctx::load_scene` loads.
//!
//! Unity is left-handed (+Z forward) and Crabby right-handed (-Z forward):
//! everything is mirrored along Z on the way, and UVs flipped to glTF's
//! top-down v.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use glam::{Mat4, Quat, Vec3};
use serde_json::{Value as Json, json};

use crate::fbx::ModelFile;
use crate::glb::{self, Primitive, Vertices};
use crate::material::{Surface, UnityMaterial};
use crate::{Error, Hierarchy, ObjRef, Project, UnityFile, UnityMesh, UnityTexture, Value};

const MESH_FILTER: i32 = 33;
const MESH_RENDERER: i32 = 23;
const SKINNED_MESH_RENDERER: i32 = 137;
const LOD_GROUP: i32 = 205;
/// The GUID Unity's built-in resources (Cube, Sphere, Default-Material) share.
const BUILTIN: &str = "0000000000000000e000000000000000";

/// What an import made and what it left out, and why.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Entities in the scenes written.
    pub entities: usize,
    /// `.glb` files written.
    pub meshes: usize,
    /// Images written.
    pub textures: usize,
    /// What was left out, by reason, with how often.
    pub skipped: BTreeMap<String, usize>,
    /// The shaders of the materials imported, by asset path (or built-in
    /// id), with how many renderers use each: which need a Crabby shader.
    pub shaders: BTreeMap<String, usize>,
}

impl Report {
    fn skip(&mut self, why: impl Into<String>) {
        *self.skipped.entry(why.into()).or_default() += 1;
    }
}

/// Where a texture's texels are.
#[derive(Debug, Clone, PartialEq)]
enum Source {
    /// An image file.
    File(PathBuf),
    /// A texture asset: the file, and the object's id in it.
    Asset(PathBuf, i64),
}

/// An image to write once the scenes are read, from source images.
#[derive(Debug, Clone, PartialEq)]
enum Job {
    /// A texture as it is (made at most `max` texels on a side).
    Copy { from: Source, max: u32 },
    /// glTF's occlusion (red), roughness (green) and metalness (blue) from
    /// Unity's maps: metalness in red and smoothness in alpha of `metal`,
    /// or smoothness in `gloss`'s alpha; occlusion in `occlusion`'s green.
    Orm {
        metal: Option<Source>,
        gloss: Option<Source>,
        occlusion: Option<Source>,
        smoothness: f32,
        occlusion_strength: f32,
        max: u32,
    },
}

/// Imports Unity scenes and prefabs of one project into a folder of a
/// Crabby game's `assets/`, sharing meshes and textures between them.
///
/// ```no_run
/// use crabby_unity::{Hierarchy, Importer, Project};
///
/// let project = Project::open("MyUnityGame").unwrap();
/// let file = crabby_unity::UnityFile::read(project.root().join("Assets/Scenes/Town.unity")).unwrap();
/// let scene = Hierarchy::build(&file, &project).unwrap();
/// let mut import = Importer::new(&project, "my_game/assets/town", "town/");
/// let world = import.scene(&scene).unwrap();
/// std::fs::write("my_game/assets/town/scene.json", serde_json::to_string(&world).unwrap()).unwrap();
/// let report = import.finish().unwrap();
/// println!("{} meshes, {} textures", report.meshes, report.textures);
/// ```
pub struct Importer<'p> {
    project: &'p Project,
    /// The output folder, and its asset name (`"town/"`).
    out: PathBuf,
    prefix: String,
    /// Mesh files written, by mesh and materials: their asset names.
    models: HashMap<(ObjRef, Vec<ObjRef>), Option<String>>,
    materials: HashMap<ObjRef, Option<UnityMaterial>>,
    /// Model files (FBX, OBJ) read so far, by GUID.
    model_files: HashMap<String, Result<ModelFile, String>>,
    /// Images to write, by file name under `meshes/textures/`.
    jobs: BTreeMap<String, Job>,
    used_names: HashSet<String>,
    /// Shaders the game draws with its own (`custom_shader`), by asset path.
    custom_shaders: HashSet<String>,
    /// Their materials, by name, for `materials.json`.
    custom_materials: BTreeMap<String, Json>,
    /// How some of them want their meshes' vertex colors (`vertex_colors`).
    vertex_colors: HashMap<String, VertexColors>,
    report: Report,
}

/// Makes a mesh's vertex colors for a custom shader (`vertex_colors`).
type VertexColors = std::rc::Rc<dyn Fn(&UnityMesh, usize) -> [f32; 4]>;

impl<'p> Importer<'p> {
    /// Writes into `out` (a folder under the game's `assets/`), whose asset
    /// name is `prefix` (`"town/"` for `assets/town`).
    pub fn new(project: &'p Project, out: impl Into<PathBuf>, prefix: &str) -> Self {
        let mut prefix = prefix.to_string();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        Self {
            project,
            out: out.into(),
            prefix,
            models: HashMap::new(),
            materials: HashMap::new(),
            model_files: HashMap::new(),
            jobs: BTreeMap::new(),
            used_names: HashSet::new(),
            custom_shaders: HashSet::new(),
            custom_materials: BTreeMap::new(),
            vertex_colors: HashMap::new(),
            report: Report::default(),
        }
    }

    /// Says the game draws materials of `shader` (its asset path, as
    /// [`Report::shaders`] lists it) with a Crabby shader of its own. Their
    /// meshes keep their vertex colors, and [`finish`](Self::finish)
    /// writes `materials.json`: each such material by name (the name its
    /// glTF material has, for `Ctx::loaded_materials`), with its shader and
    /// every number, color and texture it sets, the textures written as
    /// PNGs beside the others:
    ///
    /// ```json
    /// { "Plaster": { "shader": "Assets/Shaders/Wall.shader",
    ///   "floats": { "_Smoothness": 0.12 }, "colors": { "_BaseColor": [1, 1, 1, 1] },
    ///   "textures": { "_MaskMap": { "file": "town/meshes/textures/9f3c.png", "scale": [1, 1], "offset": [0, 0] } } } }
    /// ```
    ///
    /// Call it before importing scenes.
    pub fn custom_shader(&mut self, shader: &str) {
        self.custom_shaders.insert(shader.to_string());
    }

    /// Like [`custom_shader`](Self::custom_shader), and the vertex colors of
    /// the meshes drawn with `shader` are `colors(mesh, vertex)` instead of
    /// the mesh's own: for a shader that reads something else per vertex,
    /// such as a second UV set, from the color its Crabby port gets
    /// (`s.color`). They're stored as 8 bits per channel, from 0 to 1.
    ///
    /// ```no_run
    /// # let project = crabby_unity::Project::open("MyUnityGame").unwrap();
    /// let mut import = crabby_unity::Importer::new(&project, "my_game/assets/town", "town/");
    /// // A river shader that reads its flow from UV set 1: speed up to 4, foam 0 to 1.
    /// import.vertex_colors("Assets/Shaders/River.shader", |mesh, v| {
    ///     let [speed, foam] = mesh.uvs[1].get(v).copied().unwrap_or_default();
    ///     [speed / 4.0, foam, 0.0, 1.0]
    /// });
    /// ```
    pub fn vertex_colors(&mut self, shader: &str, colors: impl Fn(&UnityMesh, usize) -> [f32; 4] + 'static) {
        self.custom_shader(shader);
        self.vertex_colors.insert(shader.to_string(), std::rc::Rc::new(colors));
    }

    /// What's been imported and left out so far.
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// The `crabby-world` save of `scene`: an entity per drawn mesh, placed
    /// in the world (the hierarchy is baked in). Writes the meshes it
    /// needs; their textures are written by [`finish`](Self::finish).
    ///
    /// # Errors
    ///
    /// If a mesh file can't be written.
    pub fn scene(&mut self, scene: &Hierarchy) -> Result<Json, Error> {
        let objects = &scene.objects;
        // World matrices and activity, parents first.
        let mut world: Vec<Mat4> = Vec::with_capacity(objects.len());
        let mut active: Vec<bool> = Vec::with_capacity(objects.len());
        for o in objects {
            let local = Mat4::from_scale_rotation_translation(
                Vec3::from(o.local_scale()),
                Quat::from_array(o.local_rotation()).normalize(),
                Vec3::from(o.local_position()),
            );
            let (parent, parent_active) = o.parent.map_or((Mat4::IDENTITY, true), |p| (world[p], active[p]));
            world.push(parent * local);
            active.push(o.active && parent_active);
        }
        // Renderers that only draw from a distance: LOD 1 and up.
        let mut far: HashSet<i64> = HashSet::new();
        for o in objects {
            if let Some(lod) = o.component(LOD_GROUP)
                && let Some(lods) = lod.body["m_LODs"].as_seq()
            {
                for level in lods.iter().skip(1) {
                    for r in level["renderers"].as_seq().unwrap_or(&[]) {
                        if let Some(r) = r["renderer"].obj_ref() {
                            far.insert(r.file_id);
                        }
                    }
                }
            }
        }

        let mut entities = Vec::new();
        for (i, o) in objects.iter().enumerate() {
            if o.model.is_some() {
                self.report.skip("an instance of a model file (FBX, OBJ, ...)");
            }
            if o.component(SKINNED_MESH_RENDERER).is_some() {
                self.report.skip("a skinned mesh");
            }
            let (Some(filter), Some(renderer)) = (o.component(MESH_FILTER), o.component(MESH_RENDERER)) else {
                continue;
            };
            if !active[i] {
                self.report.skip("inactive");
                continue;
            }
            if renderer.body["m_Enabled"].as_i64() == Some(0) {
                self.report.skip("renderer off");
                continue;
            }
            if far.contains(&renderer.file_id) {
                self.report.skip("a far level of detail (LOD 1+)");
                continue;
            }
            let Some(mesh) = filter.body["m_Mesh"].obj_ref().filter(|m| !m.is_none()) else {
                self.report.skip("no mesh");
                continue;
            };
            let materials: Vec<ObjRef> = renderer.body["m_Materials"]
                .as_seq()
                .unwrap_or(&[])
                .iter()
                .filter_map(Value::obj_ref)
                .collect();
            let Some(name) = self.model(&mesh, &materials)? else {
                continue;
            };
            let (scale, rotation, position) = mirror(world[i]).to_scale_rotation_translation();
            entities.push(json!({
                "entity": entities.len(),
                "components": {
                    "Transform3D": {
                        "position": position.to_array(),
                        "rotation": rotation.to_array(),
                        "scale": scale.to_array(),
                    },
                    "Model": { "mesh": name },
                },
            }));
        }
        self.report.entities += entities.len();
        Ok(json!({
            "format": "crabby-world",
            "version": 1,
            "entities": entities,
            "resources": {},
        }))
    }

    /// The asset name of the `.glb` for `mesh` drawn with `materials`,
    /// written on first use; `None` when it can't be imported (and why is
    /// in the report).
    fn model(&mut self, mesh: &ObjRef, materials: &[ObjRef]) -> Result<Option<String>, Error> {
        let key = (mesh.clone(), materials.to_vec());
        if let Some(name) = self.models.get(&key) {
            return Ok(name.clone());
        }
        let name = self.write_model(mesh, materials)?;
        self.models.insert(key, name.clone());
        Ok(name)
    }

    fn write_model(&mut self, mesh: &ObjRef, materials: &[ObjRef]) -> Result<Option<String>, Error> {
        let Some(guid) = mesh.guid.as_deref() else {
            self.report.skip("a mesh saved in the scene itself");
            return Ok(None);
        };
        let unity = if guid == BUILTIN {
            match UnityMesh::builtin(mesh.file_id) {
                Some(m) => m,
                None => {
                    let what = crate::builtin::name(mesh.file_id).map_or(mesh.file_id.to_string(), str::to_owned);
                    self.report.skip(format!("built-in mesh {what}"));
                    return Ok(None);
                }
            }
        } else {
            match self.project_mesh(guid, mesh)? {
                Some(m) => m,
                None => return Ok(None),
            }
        };
        let mats: Vec<Option<UnityMaterial>> = materials.iter().map(|m| self.material(m)).collect();
        let glb = self.build_glb(&unity, &mats);
        let file_name = self.unique_name(&unity.name, mesh, materials);
        let dir = self.out.join("meshes");
        std::fs::create_dir_all(&dir).map_err(|e| Error::new(format!("{}: {e}", dir.display())))?;
        let at = dir.join(&file_name);
        std::fs::write(&at, glb).map_err(|e| Error::new(format!("{}: {e}", at.display())))?;
        self.report.meshes += 1;
        Ok(Some(format!("{}meshes/{file_name}", self.prefix)))
    }

    /// A mesh from a file of the project: a mesh asset, FBX or OBJ; `None`
    /// when it can't be read (and why is in the report).
    fn project_mesh(&mut self, guid: &str, mesh: &ObjRef) -> Result<Option<UnityMesh>, Error> {
        let Some(path) = self.project.path_of(guid) else {
            self.report.skip("a mesh not in the project");
            return Ok(None);
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let unity = if ext == "asset" {
            let file = match UnityFile::read(self.project.root().join(path)) {
                Ok(f) => f,
                Err(e) => {
                    self.report.skip(format!("an unreadable mesh file: {e}"));
                    return Ok(None);
                }
            };
            let Some(object) = file.get(mesh.file_id) else {
                self.report.skip("a mesh missing from its file");
                return Ok(None);
            };
            match UnityMesh::from_object(object) {
                Ok(m) => m,
                Err(e) => {
                    self.report
                        .skip(format!("a mesh that can't be read: {}", first_words(&e.message)));
                    return Ok(None);
                }
            }
        } else if ["fbx", "obj"].contains(&ext.as_str()) {
            let full = self.project.root().join(path);
            let model = self
                .model_files
                .entry(guid.to_string())
                .or_insert_with(|| ModelFile::open(&full).map_err(|e| e.message));
            let mesh = match model {
                Ok(model) => model.mesh(mesh.file_id),
                Err(e) => Err(Error::new(e.clone())),
            };
            match mesh {
                Ok(m) => m,
                Err(e) => {
                    self.report.skip(format!(
                        "a mesh that can't be read from a .{ext}: {}",
                        first_words(&e.message)
                    ));
                    return Ok(None);
                }
            }
        } else {
            self.report.skip(format!("a mesh in a .{ext} file"));
            return Ok(None);
        };
        Ok(Some(unity))
    }
    /// The material `r` names, read once; `None` (drawn plain) when it
    /// can't be found.
    fn material(&mut self, r: &ObjRef) -> Option<UnityMaterial> {
        if let Some(m) = self.materials.get(r) {
            if let Some(m) = m {
                *self.report.shaders.entry(self.shader_name(m)).or_default() += 1;
            }
            return m.clone();
        }
        let m = r
            .guid
            .as_deref()
            .filter(|g| *g != BUILTIN)
            .and_then(|g| self.project.load(g).ok())
            .and_then(|f| f.get(r.file_id).map(UnityMaterial::from_object))
            .and_then(Result::ok);
        if m.is_none() && !r.is_none() {
            self.report.skip("a material that can't be read (drawn plain)");
        }
        if let Some(m) = &m {
            *self.report.shaders.entry(self.shader_name(m)).or_default() += 1;
            if self.is_custom(m) {
                self.describe(m);
            }
        }
        self.materials.insert(r.clone(), m.clone());
        m
    }

    fn is_custom(&self, m: &UnityMaterial) -> bool {
        !self.custom_shaders.is_empty() && self.custom_shaders.contains(&self.shader_name(m))
    }

    /// Adds `m` to `materials.json`, queuing its textures.
    fn describe(&mut self, m: &UnityMaterial) {
        let mut textures = serde_json::Map::new();
        for (prop, t) in &m.textures {
            if let Some((name, job)) = self.copy_job(&t.texture) {
                self.jobs.entry(name.clone()).or_insert(job);
                textures.insert(
                    prop.clone(),
                    json!({ "file": format!("{}meshes/textures/{name}", self.prefix), "scale": t.scale, "offset": t.offset }),
                );
            }
        }
        let entry = json!({
            "shader": self.shader_name(m),
            "floats": m.floats,
            "colors": m.colors,
            "textures": textures,
        });
        match self.custom_materials.get(&m.name) {
            Some(old) if *old != entry => self.report.skip(format!(
                "a second material named {} (materials.json keeps the first)",
                m.name
            )),
            Some(_) => {}
            None => {
                self.custom_materials.insert(m.name.clone(), entry);
            }
        }
    }

    fn shader_name(&self, m: &UnityMaterial) -> String {
        match m.shader.as_deref() {
            Some(BUILTIN) | None => format!("built-in shader {}", m.shader_file_id),
            Some(g) => self.project.path_of(g).map_or_else(
                || format!("shader {g}"),
                |p| {
                    let p = p.strip_prefix(self.project.root()).unwrap_or(p);
                    p.to_string_lossy().replace('\\', "/")
                },
            ),
        }
    }

    /// The mesh as a `.glb`, in glTF's space, a primitive per submesh with
    /// its material.
    fn build_glb(&mut self, mesh: &UnityMesh, mats: &[Option<UnityMaterial>]) -> Vec<u8> {
        let mut textures: Vec<String> = Vec::new();
        let mut materials: Vec<Json> = Vec::new();
        let mut vertices = Vertices::default();
        let mut primitives = Vec::new();
        // Vertex colors, for meshes a custom shader draws (`custom_shader`),
        // made by that shader's function if it has one (`vertex_colors`).
        let made = mats
            .iter()
            .flatten()
            .find_map(|m| self.vertex_colors.get(&self.shader_name(m)).cloned());
        let colors = made.is_some() || (!mesh.colors.is_empty() && mats.iter().flatten().any(|m| self.is_custom(m)));
        // Submeshes past the last material reuse it, as in Unity; materials
        // past the last submesh draw it again, which is skipped.
        for (s, indices) in mesh.submeshes.iter().enumerate() {
            let m = mats.get(s).or(mats.last()).cloned().flatten();
            let (tiling, offset) = m.as_ref().map_or(([1.0; 2], [0.0; 2]), |m| (m.tiling, m.offset));
            // Each submesh gets its own vertices: they can differ by the
            // material's tiling.
            let mut remap: HashMap<u32, u32> = HashMap::new();
            let mut out = Vec::with_capacity(indices.len());
            for tri in indices.as_chunks::<3>().0 {
                // Mirrored, so the winding turns around.
                for &v in &[tri[0], tri[2], tri[1]] {
                    let at = *remap.entry(v).or_insert_with(|| {
                        let v = v as usize;
                        let p = mesh.positions.get(v).copied().unwrap_or_default();
                        vertices.positions.push([p[0], p[1], -p[2]]);
                        if !mesh.normals.is_empty() {
                            let n = mesh.normals.get(v).copied().unwrap_or([0.0, 1.0, 0.0]);
                            vertices.normals.push([n[0], n[1], -n[2]]);
                        }
                        if let Some(made) = &made {
                            vertices.colors.push(made(mesh, v).map(|c| c.clamp(0.0, 1.0)));
                        } else if colors {
                            vertices.colors.push(mesh.colors.get(v).copied().unwrap_or([1.0; 4]));
                        }
                        if !mesh.uvs[0].is_empty() {
                            let uv = mesh.uvs[0].get(v).copied().unwrap_or_default();
                            vertices
                                .uvs
                                .push([uv[0] * tiling[0] + offset[0], 1.0 - (uv[1] * tiling[1] + offset[1])]);
                        }
                        (vertices.positions.len() - 1) as u32
                    });
                    out.push(at);
                }
            }
            let index = materials.len();
            materials.push(self.gltf_material(m.as_ref(), &mut textures));
            primitives.push(Primitive {
                indices: out,
                material: index,
            });
        }
        glb::write(&mesh.name, &vertices, &primitives, materials, &textures)
    }

    /// A glTF material for `m` (plain white without one), queuing its
    /// images and adding their URIs to `textures`.
    fn gltf_material(&mut self, m: Option<&UnityMaterial>, textures: &mut Vec<String>) -> Json {
        let Some(m) = m else {
            return json!({ "name": "plain" });
        };
        let unlit = self.shader_name(m).ends_with("Unlit.shader");
        let mut texture = |importer: &mut Self, r: &Option<ObjRef>| -> Option<usize> {
            let (name, job) = importer.copy_job(r.as_ref()?)?;
            importer.jobs.entry(name.clone()).or_insert(job);
            Some(index_of(textures, format!("textures/{name}")))
        };
        let base = texture(self, &m.base_map);
        let normal = texture(self, &m.normal_map);
        let emission = texture(self, &m.emission_map);
        let color = linear(m.color);
        let mut pbr = json!({
            "baseColorFactor": color,
            "metallicFactor": m.metallic,
            "roughnessFactor": 1.0 - m.smoothness,
        });
        if let Some(b) = base {
            pbr["baseColorTexture"] = json!({ "index": b });
        }
        let mut out = json!({ "name": m.name, "pbrMetallicRoughness": pbr, "doubleSided": m.double_sided });
        if unlit {
            // Drawn by its own light: black, glowing its color.
            out["pbrMetallicRoughness"]["baseColorFactor"] = json!([0.0, 0.0, 0.0, color[3]]);
            out["emissiveFactor"] = json!([color[0], color[1], color[2]]);
            if let Some(b) = base {
                out["emissiveTexture"] = json!({ "index": b });
            }
        } else {
            if let Some(n) = normal {
                out["normalTexture"] = json!({ "index": n, "scale": m.normal_scale });
            }
            if m.emission != [0.0; 3] {
                let e = linear([m.emission[0], m.emission[1], m.emission[2], 1.0]);
                out["emissiveFactor"] = json!([e[0], e[1], e[2]]);
                if let Some(e) = emission {
                    out["emissiveTexture"] = json!({ "index": e });
                }
            }
            if let Some((name, job)) = self.orm_job(m) {
                let has_occlusion = matches!(&job, Job::Orm { occlusion: Some(_), .. });
                self.jobs.entry(name.clone()).or_insert(job);
                let i = index_of(textures, format!("textures/{name}"));
                out["pbrMetallicRoughness"]["metallicRoughnessTexture"] = json!({ "index": i });
                out["pbrMetallicRoughness"]["roughnessFactor"] = json!(1.0);
                if m.metallic_map.is_some() {
                    out["pbrMetallicRoughness"]["metallicFactor"] = json!(1.0);
                }
                if has_occlusion {
                    out["occlusionTexture"] = json!({ "index": i });
                }
            }
        }
        match m.surface {
            Surface::Opaque => {}
            Surface::Cutout(cutoff) => {
                out["alphaMode"] = json!("MASK");
                out["alphaCutoff"] = json!(cutoff);
            }
            Surface::Transparent => out["alphaMode"] = json!("BLEND"),
        }
        out
    }

    /// The source image of texture `r`, with the size limit its import
    /// settings give; `None` when it isn't a file this can read.
    fn source(&mut self, r: &ObjRef) -> Option<(Source, u32)> {
        let guid = r.guid.as_deref()?;
        let Some(path) = self.project.path_of(guid) else {
            self.report.skip("a texture not in the project");
            return None;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "asset" {
            // Generated textures are as big as they were made.
            return Some((Source::Asset(self.project.root().join(path), r.file_id), u32::MAX));
        }
        if !["png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp"].contains(&ext.as_str()) {
            self.report.skip(format!("a .{ext} texture"));
            return None;
        }
        let file = self.project.root().join(path);
        let mut meta = file.clone().into_os_string();
        meta.push(".meta");
        let max = std::fs::read_to_string(meta)
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|l| l.trim().strip_prefix("maxTextureSize:"))
                    .and_then(|n| n.trim().parse().ok())
            })
            .unwrap_or(2048);
        Some((Source::File(file), max))
    }

    fn copy_job(&mut self, r: &ObjRef) -> Option<(String, Job)> {
        let (from, max) = self.source(r)?;
        let name = match &from {
            Source::Asset(_, id) if *id != 2_800_000 => format!("{}_{id}.png", r.guid.as_deref()?),
            _ => format!("{}.png", r.guid.as_deref()?),
        };
        Some((name, Job::Copy { from, max }))
    }

    /// The ORM image `m` needs, if any map feeds it.
    fn orm_job(&mut self, m: &UnityMaterial) -> Option<(String, Job)> {
        let metal = m.metallic_map.as_ref().and_then(|r| self.source(r));
        let gloss = if m.smoothness_in_base_alpha {
            m.base_map.as_ref().and_then(|r| self.source(r))
        } else {
            None
        };
        let occlusion = m.occlusion_map.as_ref().and_then(|r| self.source(r));
        if metal.is_none() && gloss.is_none() && occlusion.is_none() {
            return None;
        }
        let max = [&metal, &gloss, &occlusion]
            .iter()
            .filter_map(|s| s.as_ref().map(|s| s.1))
            .max()?;
        let guid = |r: &Option<ObjRef>| r.as_ref().and_then(|r| r.guid.clone()).unwrap_or_default();
        let name = format!(
            "orm_{}_{}_{}_{}_{}.png",
            short(&guid(&m.metallic_map)),
            short(&if m.smoothness_in_base_alpha {
                guid(&m.base_map)
            } else {
                String::new()
            }),
            short(&guid(&m.occlusion_map)),
            (m.smoothness * 100.0).round() as i32,
            (m.occlusion_strength * 100.0).round() as i32,
        );
        Some((
            name,
            Job::Orm {
                metal: metal.map(|s| s.0),
                gloss: gloss.map(|s| s.0),
                occlusion: occlusion.map(|s| s.0),
                smoothness: m.smoothness,
                occlusion_strength: m.occlusion_strength,
                max,
            },
        ))
    }

    /// A file name for a mesh: its name, and a short hash of what it is.
    fn unique_name(&mut self, name: &str, mesh: &ObjRef, materials: &[ObjRef]) -> String {
        let clean: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .take(48)
            .collect();
        let mut hash = fnv(format!("{mesh:?}{materials:?}").as_bytes());
        loop {
            let file = format!("{clean}_{:08x}.glb", hash as u32);
            if self.used_names.insert(file.clone()) {
                return file;
            }
            hash = hash.wrapping_add(1);
        }
    }

    /// Writes the images the meshes use (in parallel) and returns the
    /// report.
    ///
    /// # Errors
    ///
    /// If an image can't be written. Images that can't be read are left
    /// out (and counted in the report).
    pub fn finish(mut self) -> Result<Report, Error> {
        use rayon::prelude::*;
        let dir = self.out.join("meshes").join("textures");
        std::fs::create_dir_all(&dir).map_err(|e| Error::new(format!("{}: {e}", dir.display())))?;
        let jobs: Vec<(String, Job)> = std::mem::take(&mut self.jobs).into_iter().collect();
        let results: Vec<Result<bool, Error>> = jobs
            .par_iter()
            .map(|(name, job)| {
                let at = dir.join(name);
                let Some(image) = run(job) else {
                    return Ok(false);
                };
                image
                    .save_with_format(&at, image::ImageFormat::Png)
                    .map_err(|e| Error::new(format!("{}: {e}", at.display())))?;
                Ok(true)
            })
            .collect();
        if !self.custom_materials.is_empty() {
            let at = self.out.join("materials.json");
            let text = serde_json::to_string_pretty(&self.custom_materials).expect("JSON");
            std::fs::write(&at, text).map_err(|e| Error::new(format!("{}: {e}", at.display())))?;
        }
        for r in results {
            if r? {
                self.report.textures += 1;
            } else {
                self.report.skip("a texture that can't be decoded");
            }
        }
        Ok(self.report)
    }
}

/// Unity's space to Crabby's: mirrored along Z on both sides.
fn mirror(m: Mat4) -> Mat4 {
    let s = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
    s * m * s
}

/// Makes one image of `job`; `None` if a source can't be read.
fn run(job: &Job) -> Option<image::RgbaImage> {
    let open = |source: &Source, max: u32| -> Option<image::RgbaImage> {
        let image = match source {
            Source::File(p) => image::open(p).ok()?,
            Source::Asset(p, id) => {
                let file = UnityFile::read(p).ok()?;
                let t = UnityTexture::from_object(file.get(*id)?).ok()?;
                // An array's first layer.
                let one = (t.width * t.height * 4) as usize;
                let rgba = image::RgbaImage::from_raw(t.width, t.height, t.rgba[..one].to_vec())?;
                image::DynamicImage::ImageRgba8(rgba)
            }
        };
        Some(fit(image, max).to_rgba8())
    };
    match job {
        Job::Copy { from, max } => open(from, *max),
        Job::Orm {
            metal,
            gloss,
            occlusion,
            smoothness,
            occlusion_strength,
            max,
        } => {
            let metal = metal.as_ref().map(|p| open(p, *max)).map(Option::unwrap_or_default);
            let gloss = gloss.as_ref().map(|p| open(p, *max)).map(Option::unwrap_or_default);
            let occlusion = occlusion.as_ref().map(|p| open(p, *max)).map(Option::unwrap_or_default);
            let (w, h) = [&metal, &gloss, &occlusion]
                .iter()
                .filter_map(|i| i.as_ref().map(image::RgbaImage::dimensions))
                .max()?;
            if w == 0 || h == 0 {
                return None;
            }
            let sized = |i: Option<image::RgbaImage>| {
                i.filter(|i| i.width() > 0).map(|i| {
                    if i.dimensions() == (w, h) {
                        i
                    } else {
                        image::imageops::resize(&i, w, h, image::imageops::FilterType::Triangle)
                    }
                })
            };
            let (metal, gloss, occlusion) = (sized(metal), sized(gloss), sized(occlusion));
            let smooth_source = gloss.as_ref().or(metal.as_ref());
            let mut out = image::RgbaImage::new(w, h);
            for (x, y, px) in out.enumerate_pixels_mut() {
                let ao = occlusion.as_ref().map_or(1.0, |o| {
                    let g = f32::from(o.get_pixel(x, y)[1]) / 255.0;
                    1.0 + (g - 1.0) * occlusion_strength
                });
                let rough = smooth_source.map_or(1.0, |s| 1.0 - f32::from(s.get_pixel(x, y)[3]) / 255.0 * smoothness);
                let metalness = metal.as_ref().map_or(255, |m| m.get_pixel(x, y)[0]);
                *px = image::Rgba([
                    (ao * 255.0).round() as u8,
                    (rough * 255.0).round() as u8,
                    metalness,
                    255,
                ]);
            }
            Some(out)
        }
    }
}

/// `image` made at most `max` texels on its longer side.
fn fit(image: image::DynamicImage, max: u32) -> image::DynamicImage {
    let (w, h) = (image.width(), image.height());
    if w.max(h) <= max || max == 0 {
        image
    } else {
        image.resize(max, max, image::imageops::FilterType::Triangle)
    }
}

/// sRGB color to linear, alpha kept.
fn linear(c: [f32; 4]) -> [f32; 4] {
    let l = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [l(c[0]), l(c[1]), l(c[2]), c[3]]
}

fn index_of(list: &mut Vec<String>, item: String) -> usize {
    match list.iter().position(|i| *i == item) {
        Some(i) => i,
        None => {
            list.push(item);
            list.len() - 1
        }
    }
}

fn short(guid: &str) -> &str {
    &guid[..guid.len().min(8)]
}

fn first_words(s: &str) -> String {
    s.split(':').next().unwrap_or(s).chars().take(60).collect()
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirroring_keeps_rotations_rotations() {
        let m = Mat4::from_rotation_translation(Quat::from_rotation_y(0.5), Vec3::new(1.0, 2.0, 3.0));
        let (scale, rotation, position) = mirror(m).to_scale_rotation_translation();
        assert!((scale - Vec3::ONE).length() < 1e-5);
        assert!((position - Vec3::new(1.0, 2.0, -3.0)).length() < 1e-5);
        // A turn around Y the other way.
        assert!(rotation.abs_diff_eq(Quat::from_rotation_y(-0.5), 1e-5));
    }

    #[test]
    fn colors_go_linear() {
        let c = linear([1.0, 0.5, 0.0, 0.25]);
        assert!((c[1] - 0.214).abs() < 1e-3);
        assert_eq!((c[0], c[2], c[3]), (1.0, 0.0, 0.25));
    }

    /// A material of the given shader GUID, everything else default.
    fn material_of(shader: &str) -> UnityMaterial {
        let text = format!(
            "--- !u!21 &2100000\nMaterial:\n  m_Name: Wall\n  m_Shader: {{fileID: 4800000, guid: {shader}, type: 3}}\n"
        );
        UnityMaterial::from_object(&UnityFile::parse(&text).unwrap().objects[0]).unwrap()
    }

    fn has_colors(glb: &[u8]) -> bool {
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: Json = serde_json::from_slice(&glb[20..20 + len]).unwrap();
        doc["meshes"][0]["primitives"][0]["attributes"].get("COLOR_0").is_some()
    }

    /// The first primitive's COLOR_0.
    fn colors_of(glb: &[u8]) -> Vec<[f32; 4]> {
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: Json = serde_json::from_slice(&glb[20..20 + len]).unwrap();
        let bin = &glb[20 + len + 8..];
        let accessor = &doc["accessors"][doc["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"]
            .as_u64()
            .unwrap() as usize];
        let view = &doc["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
        let start =
            view["byteOffset"].as_u64().unwrap_or(0) as usize + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
        let floats: Vec<f32> = bin[start..start + accessor["count"].as_u64().unwrap() as usize * 16]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        floats.as_chunks::<4>().0.to_vec()
    }

    #[test]
    fn vertex_colors_can_be_made_from_other_channels() {
        let dir = std::env::temp_dir().join(format!("crabby_unity_made_colors_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Assets")).unwrap();
        std::fs::write(dir.join("Assets/River.shader"), "Shader \"River\" {}").unwrap();
        std::fs::write(dir.join("Assets/River.shader.meta"), "guid: river000\n").unwrap();
        let project = Project::open(&dir).unwrap();
        let mut quad = UnityMesh::builtin(crate::builtin::QUAD).unwrap();
        quad.uvs[1] = (0..quad.positions.len()).map(|v| [v as f32, 0.5]).collect();
        let river = [Some(material_of("river000"))];
        let mut import = Importer::new(&project, dir.join("out"), "");
        import.vertex_colors("Assets/River.shader", |mesh, v| {
            let [speed, foam] = mesh.uvs[1][v];
            [speed / 2.0, foam, 0.0, 9.0]
        });
        let colors = colors_of(&import.build_glb(&quad, &river));
        assert_eq!(colors.len(), 4);
        let mut speeds: Vec<f32> = colors.iter().map(|c| c[0]).collect();
        speeds.sort_by(f32::total_cmp);
        assert_eq!(speeds, [0.0, 0.5, 1.0, 1.0], "from UV set 1, clamped to 0..1");
        assert!(colors.iter().all(|c| c[1] == 0.5 && c[2] == 0.0 && c[3] == 1.0));
        assert!(import.is_custom(river[0].as_ref().unwrap()), "it's a custom shader too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn vertex_colors_are_kept_only_for_custom_shaders() {
        let dir = std::env::temp_dir().join(format!("crabby_unity_colors_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Assets")).unwrap();
        std::fs::write(dir.join("Assets/Wall.shader"), "Shader \"Wall\" {}").unwrap();
        std::fs::write(dir.join("Assets/Wall.shader.meta"), "guid: wall0000\n").unwrap();
        let project = Project::open(&dir).unwrap();
        let mut cube = UnityMesh::builtin(crate::builtin::CUBE).unwrap();
        cube.colors = vec![[0.5, 0.25, 1.0, 0.75]; cube.positions.len()];
        let wall = [Some(material_of("wall0000"))];
        let mut plain = Importer::new(&project, dir.join("out"), "");
        assert!(
            !has_colors(&plain.build_glb(&cube, &wall)),
            "Lit-style shaders ignore them"
        );
        let mut custom = Importer::new(&project, dir.join("out"), "");
        custom.custom_shader("Assets/Wall.shader");
        assert!(has_colors(&custom.build_glb(&cube, &wall)));
        cube.colors.clear();
        assert!(!has_colors(&custom.build_glb(&cube, &wall)), "none to keep");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
