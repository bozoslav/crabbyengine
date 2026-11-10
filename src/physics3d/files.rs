//! Mesh colliders that scenes name by file: `"Collider3D": {"shape":
//! {"Mesh": {"file": "rock.glb", "scale": [1, 1, 1], "convex": false}}}`.
//! Loading the scene reads the file's mesh, scales it and makes a triangle
//! mesh (or a convex hull) of it, once per description.

use serde_json::Value;

use crate::reflect::{ColliderFiles, collider_file, collider_file_key};
use crate::{Ctx, Vec3};

/// Every mesh description under a `"Collider3D"` in `json`.
fn collect<'a>(json: &'a Value, out: &mut Vec<&'a Value>) {
    match json {
        Value::Object(map) => {
            for (k, value) in map {
                if k == "Collider3D"
                    && let Some(file) = collider_file(value)
                {
                    out.push(file);
                } else {
                    collect(value, out);
                }
            }
        }
        Value::Array(list) => list.iter().for_each(|v| collect(v, out)),
        _ => {}
    }
}

impl Ctx {
    /// Makes the collision meshes `json` (a scene, a prefab or components)
    /// names by file, so its colliders can be spawned.
    pub(crate) fn load_collider_files(&mut self, json: &Value) -> Result<(), String> {
        let mut files = Vec::new();
        collect(json, &mut files);
        if files.is_empty() {
            return Ok(());
        }
        if self.world.get_resource::<ColliderFiles>().is_none() {
            self.world.insert_resource(ColliderFiles::default());
        }
        for file in files {
            let key = collider_file_key(file);
            if self.world.resource::<ColliderFiles>().by_file.contains_key(&key) {
                continue;
            }
            let path = file["file"]
                .as_str()
                .ok_or("a mesh collider's \"file\" must be a string")?;
            let scale = match &file["scale"] {
                Value::Array(s) if s.len() == 3 => {
                    Vec3::from_array(std::array::from_fn(|i| s[i].as_f64().unwrap_or(1.0) as f32))
                }
                _ => Vec3::ONE,
            };
            let convex = file["convex"].as_bool().unwrap_or(false);
            let data = self.gfx.load_mesh_data(path)?;
            let points: Vec<Vec3> = data.positions.iter().map(|&p| p * scale).collect();
            let mesh = if convex {
                self.physics3d.convex_hull(&points)
            } else {
                let triangles: Vec<[u32; 3]> = data.indices.as_chunks::<3>().0.to_vec();
                self.physics3d.trimesh(&points, &triangles)
            }
            .ok_or_else(|| format!("{path}: no collision shape can be made of it"))?;
            let files = self.world.resource_mut::<ColliderFiles>();
            files.by_file.insert(key, mesh);
            files.by_mesh.insert(mesh, file.clone());
        }
        Ok(())
    }
}
