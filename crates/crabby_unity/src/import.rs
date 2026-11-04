//! Turning Unity scenes and prefabs into Crabby scenes: every drawn mesh
//! becomes a `.glb` with its materials, its textures become PNGs, and the
//! scene a `crabby-world` save of `Transform3D` and `Model` entities that
//! `Ctx::load_scene` loads.
//!
//! Unity is left-handed (+Z forward) and Crabby right-handed (-Z forward):
//! everything is mirrored along Z on the way, and UVs flipped to glTF's
//! top-down v.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use glam::{Mat4, Quat, Vec3};
use serde_json::{Value as Json, json};

use crate::glb::{self, Primitive, Vertices};
use crate::material::{Surface, UnityMaterial};
use crate::{Error, Hierarchy, ObjRef, Project, UnityFile, UnityMesh, Value};

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

/// An image to write once the scenes are read, from source images.
#[derive(Debug, Clone, PartialEq)]
enum Job {
    /// A texture as it is (made at most `max` texels on a side).
    Copy { from: PathBuf, max: u32 },
    /// glTF's occlusion (red), roughness (green) and metalness (blue) from
    /// Unity's maps: metalness in red and smoothness in alpha of `metal`,
    /// or smoothness in `gloss`'s alpha; occlusion in `occlusion`'s green.
    Orm {
        metal: Option<PathBuf>,
        gloss: Option<PathBuf>,
        occlusion: Option<PathBuf>,
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
    /// Images to write, by file name under `meshes/textures/`.
    jobs: BTreeMap<String, Job>,
    used_names: HashSet<String>,
    report: Report,
}

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
            jobs: BTreeMap::new(),
            used_names: HashSet::new(),
            report: Report::default(),
        }
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
        if guid == BUILTIN {
            self.report.skip(format!("built-in mesh {}", mesh.file_id));
            return Ok(None);
        }
        let Some(path) = self.project.path_of(guid) else {
            self.report.skip("a mesh not in the project");
            return Ok(None);
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "asset" {
            self.report.skip(format!("a mesh in a .{ext} file"));
            return Ok(None);
        }
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
        let unity = match UnityMesh::from_object(object) {
            Ok(m) => m,
            Err(e) => {
                self.report
                    .skip(format!("a mesh that can't be read: {}", first_words(&e.message)));
                return Ok(None);
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
        }
        self.materials.insert(r.clone(), m.clone());
        m
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
    fn source(&mut self, r: &ObjRef) -> Option<(PathBuf, u32)> {
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
        Some((file, max))
    }

    fn copy_job(&mut self, r: &ObjRef) -> Option<(String, Job)> {
        let (from, max) = self.source(r)?;
        Some((format!("{}.png", r.guid.as_deref()?), Job::Copy { from, max }))
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
    let open = |p: &Path, max: u32| -> Option<image::RgbaImage> {
        let image = image::open(p).ok()?;
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
            let metal = metal.as_deref().map(|p| open(p, *max)).map(Option::unwrap_or_default);
            let gloss = gloss.as_deref().map(|p| open(p, *max)).map(Option::unwrap_or_default);
            let occlusion = occlusion
                .as_deref()
                .map(|p| open(p, *max))
                .map(Option::unwrap_or_default);
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
}
