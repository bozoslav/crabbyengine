//! Meshes inside model files (FBX, OBJ), read with ufbx the way Unity's
//! model importer sees them: X mirrored into Unity's left-handed space,
//! the winding turned to match, scaled to meters (`useFileScale`) and by
//! `globalScale`, a submesh per material.

use std::collections::HashMap;
use std::path::Path;

use crate::{Error, UnityMesh};

/// A model file, loaded once for every mesh taken from it.
pub(crate) struct ModelFile {
    scene: ufbx::SceneRoot,
    /// Mesh sub-asset ids (`fileID`s that `MeshFilter`s use) to names.
    names: HashMap<i64, String>,
    /// Unity's import scale for it: units to meters times `globalScale`.
    scale: f32,
}

impl ModelFile {
    /// Loads `path`, with its `.meta` next to it.
    pub(crate) fn open(path: &Path) -> Result<Self, Error> {
        let opts = ufbx::LoadOpts {
            generate_missing_normals: true,
            ignore_animation: true,
            ignore_embedded: true,
            ..Default::default()
        };
        let scene = ufbx::load_file(&path.to_string_lossy(), opts)
            .map_err(|e| Error::new(format!("{}: {}", path.display(), e.description)))?;
        let mut meta = path.as_os_str().to_owned();
        meta.push(".meta");
        let meta = std::fs::read_to_string(meta).unwrap_or_default();
        let setting = |key: &str| -> Option<f32> {
            meta.lines()
                .find_map(|l| l.trim().strip_prefix(key)?.trim().parse().ok())
        };
        let file_scale = if setting("useFileScale:").unwrap_or(1.0) >= 0.5 {
            scene.settings.unit_meters as f32
        } else {
            1.0
        };
        let scale = file_scale * setting("globalScale:").unwrap_or(1.0);
        let mut names = mesh_names(&meta);
        // Unity 2019.3+ leaves the table empty and derives ids from names:
        // xxHash64 of "Type:Mesh->" + name + how many came before by that name.
        let mut count: HashMap<String, usize> = HashMap::new();
        for node in scene.nodes.iter().filter(|n| n.mesh.is_some()) {
            let name = node.element.name.to_string();
            let k = count.entry(name.clone()).or_default();
            names.entry(hashed_id("Mesh", &name, *k)).or_insert(name);
            *k += 1;
        }
        Ok(Self { scene, names, scale })
    }

    /// The mesh with sub-asset id `file_id`, in Unity's space.
    pub(crate) fn mesh(&self, file_id: i64) -> Result<UnityMesh, Error> {
        let name = self
            .names
            .get(&file_id)
            .ok_or_else(|| Error::new(format!("no mesh {file_id} in the model's .meta")))?;
        let node = self
            .scene
            .nodes
            .iter()
            .find(|n| n.element.name == name.as_str() && n.mesh.is_some());
        let (mesh, to_node) = match node {
            Some(n) => (n.mesh.as_ref().expect("checked"), Some(n.geometry_to_node)),
            None => (
                self.scene
                    .meshes
                    .iter()
                    .find(|m| m.element.name == name.as_str())
                    .ok_or_else(|| Error::new(format!("no mesh \"{name}\" in the model")))?,
                None,
            ),
        };
        let place = |p: ufbx::Vec3| {
            let p = to_node.map_or(p, |m| transform(&m, p, 1.0));
            [
                -(p.x as f32) * self.scale,
                p.y as f32 * self.scale,
                p.z as f32 * self.scale,
            ]
        };
        let turn = |n: ufbx::Vec3| {
            let n = to_node.map_or(n, |m| transform(&m, n, 0.0));
            let len = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt().max(1e-12);
            [-(n.x / len) as f32, (n.y / len) as f32, (n.z / len) as f32]
        };
        let mut out = UnityMesh {
            name: name.clone(),
            ..Default::default()
        };
        let mut seen: HashMap<[u32; 8], u32> = HashMap::new();
        let mut corners = vec![0u32; mesh.max_face_triangles * 3];
        let groups: Vec<Vec<u32>> = if mesh.material_parts.is_empty() {
            vec![(0..mesh.faces.len() as u32).collect()]
        } else {
            mesh.material_parts
                .iter()
                .map(|p| p.face_indices.iter().copied().collect())
                .collect()
        };
        for faces in groups {
            let mut indices = Vec::new();
            for &f in &faces {
                let face = mesh.faces[f as usize];
                let n = ufbx::triangulate_face(&mut corners, mesh, face) as usize;
                for tri in corners[..n * 3].as_chunks::<3>().0 {
                    // Mirrored, so the winding turns around.
                    for &i in &[tri[0], tri[2], tri[1]] {
                        let i = i as usize;
                        let p = place(mesh.vertex_position[i]);
                        let nrm = if mesh.vertex_normal.exists {
                            turn(mesh.vertex_normal[i])
                        } else {
                            [0.0, 1.0, 0.0]
                        };
                        let uv = if mesh.vertex_uv.exists {
                            let uv = mesh.vertex_uv[i];
                            [uv.x as f32, uv.y as f32]
                        } else {
                            [0.0; 2]
                        };
                        let key = [p[0], p[1], p[2], nrm[0], nrm[1], nrm[2], uv[0], uv[1]].map(f32::to_bits);
                        let at = *seen.entry(key).or_insert_with(|| {
                            out.positions.push(p);
                            out.normals.push(nrm);
                            out.uvs[0].push(uv);
                            (out.positions.len() - 1) as u32
                        });
                        indices.push(at);
                    }
                }
            }
            out.submeshes.push(indices);
        }
        if !mesh.vertex_uv.exists {
            out.uvs[0].clear();
        }
        Ok(out)
    }
}

fn transform(m: &ufbx::Matrix, v: ufbx::Vec3, w: f64) -> ufbx::Vec3 {
    ufbx::Vec3 {
        x: m.m00 * v.x + m.m01 * v.y + m.m02 * v.z + m.m03 * w,
        y: m.m10 * v.x + m.m11 * v.y + m.m12 * v.z + m.m13 * w,
        z: m.m20 * v.x + m.m21 * v.y + m.m22 * v.z + m.m23 * w,
    }
}

/// The id Unity 2019.3+ gives sub-asset `name` of `type` (`"Mesh"`), the
/// `index`th by that name.
fn hashed_id(type_name: &str, name: &str, index: usize) -> i64 {
    xxhash_rust::xxh64::xxh64(format!("Type:{type_name}->{name}{index}").as_bytes(), 0) as i64
}

/// Mesh sub-asset ids to names from a model's `.meta`: Unity 2019.3+
/// `internalIDToNameTable` (`first: {43: id}`, `second: name`), or the
/// older `fileIDToRecycleName` (`4300002: name`).
fn mesh_names(meta: &str) -> HashMap<i64, String> {
    let mut names = HashMap::new();
    let mut pending: Option<i64> = None;
    let mut recycle = false;
    for line in meta.lines() {
        let t = line.trim();
        if t.starts_with("fileIDToRecycleName:") {
            recycle = true;
            continue;
        }
        if recycle {
            match t.split_once(": ") {
                Some((id, name)) if line.starts_with("    ") => {
                    if let Ok(id) = id.parse::<i64>()
                        && (4_300_000..4_400_000).contains(&id)
                    {
                        names.insert(id, name.to_string());
                    }
                    continue;
                }
                _ => recycle = false,
            }
        }
        if let Some(id) = t.strip_prefix("43: ") {
            pending = id.trim().parse().ok();
        } else if let Some(name) = t.strip_prefix("second: ")
            && let Some(id) = pending.take()
        {
            names.insert(id, name.to_string());
        } else if t.starts_with("- first:") || t.starts_with("first:") {
            pending = None;
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashed_ids_match_unitys() {
        // The id Unity 6 gave a model's mesh named Body (checked in an
        // editor-imported project), and the next one by that name.
        assert_eq!(hashed_id("Mesh", "Body", 0), 2959467708922500008);
        assert_eq!(hashed_id("Mesh", "Body", 1), -1107389024503286118);
    }

    #[test]
    fn mesh_names_come_from_either_meta_table() {
        let new = "ModelImporter:\n  internalIDToNameTable:\n  - first:\n      1: 100000\n    second: //RootNode\n  - first:\n      43: -2153578634513813181\n    second: Birch_1\n  - first:\n      43: 4300002\n    second: Birch_2\n";
        let names = mesh_names(new);
        assert_eq!(names[&-2153578634513813181], "Birch_1");
        assert_eq!(names[&4300002], "Birch_2");
        assert_eq!(names.len(), 2);
        let old = "ModelImporter:\n  fileIDToRecycleName:\n    100000: //RootNode\n    4300000: Rock\n    4300002: Rock_LOD1\n  externalObjects: {}\n";
        let names = mesh_names(old);
        assert_eq!(
            (names[&4300000].as_str(), names[&4300002].as_str()),
            ("Rock", "Rock_LOD1")
        );
        assert_eq!(names.len(), 2);
    }
}
