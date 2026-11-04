//! Writing binary glTF (`.glb`) files: one mesh, a primitive per
//! material, with the materials' images as files next to it.

use serde_json::{Value as Json, json};

/// One primitive: its triangles and the material they use.
pub(crate) struct Primitive {
    pub indices: Vec<u32>,
    pub material: usize,
}

/// The vertex data, already in glTF's space (right-handed, +Y up, UV v
/// down) and shared by every primitive.
#[derive(Default)]
pub(crate) struct Vertices {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    /// UV set 1 as Unity has it (not flipped: it's data for a shader).
    pub uvs1: Vec<[f32; 2]>,
}

/// Builds the binary chunk and the JSON that points into it.
struct Writer {
    bin: Vec<u8>,
    views: Vec<Json>,
    accessors: Vec<Json>,
}

impl Writer {
    /// Adds `bytes` as a buffer view (4-byte aligned) and an accessor over
    /// it; returns the accessor's index.
    fn add(&mut self, bytes: &[u8], count: usize, kind: &str, component: u32, extra: Json) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let mut view = json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if component == 5125 {
            view["target"] = json!(34963);
        } else {
            view["target"] = json!(34962);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(view);
        let mut accessor = json!({
            "bufferView": self.views.len() - 1,
            "componentType": component,
            "count": count,
            "type": kind,
        });
        if let (Json::Object(a), Json::Object(e)) = (&mut accessor, extra) {
            a.extend(e);
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    fn floats<const N: usize>(&mut self, data: &[[f32; N]], kind: &str, extra: Json) -> usize {
        let bytes: Vec<u8> = data.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
        self.add(&bytes, data.len(), kind, 5126, extra)
    }
}

/// A `.glb` file of `vertices` drawn as `primitives`, with `materials`
/// (glTF material objects), `textures` (image URIs, relative to the file;
/// materials name them by index) and `name`.
pub(crate) fn write(
    name: &str,
    vertices: &Vertices,
    primitives: &[Primitive],
    materials: Vec<Json>,
    textures: &[String],
) -> Vec<u8> {
    let mut w = Writer {
        bin: Vec::new(),
        views: Vec::new(),
        accessors: Vec::new(),
    };
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &vertices.positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    if vertices.positions.is_empty() {
        (min, max) = ([0.0; 3], [0.0; 3]);
    }
    let mut attributes = serde_json::Map::new();
    let position = w.floats(&vertices.positions, "VEC3", json!({ "min": min, "max": max }));
    attributes.insert("POSITION".into(), json!(position));
    if !vertices.normals.is_empty() {
        attributes.insert("NORMAL".into(), json!(w.floats(&vertices.normals, "VEC3", json!({}))));
    }
    if !vertices.uvs.is_empty() {
        attributes.insert("TEXCOORD_0".into(), json!(w.floats(&vertices.uvs, "VEC2", json!({}))));
    }
    if !vertices.colors.is_empty() {
        attributes.insert("COLOR_0".into(), json!(w.floats(&vertices.colors, "VEC4", json!({}))));
    }
    if !vertices.uvs1.is_empty() {
        attributes.insert("TEXCOORD_1".into(), json!(w.floats(&vertices.uvs1, "VEC2", json!({}))));
    }
    let primitives: Vec<Json> = primitives
        .iter()
        .filter(|p| !p.indices.is_empty())
        .map(|p| {
            let bytes: Vec<u8> = p.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let indices = w.add(&bytes, p.indices.len(), "SCALAR", 5125, json!({}));
            json!({ "attributes": attributes, "indices": indices, "material": p.material })
        })
        .collect();
    while !w.bin.len().is_multiple_of(4) {
        w.bin.push(0);
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "crabby_unity" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": name }],
        "meshes": [{ "name": name, "primitives": primitives }],
        "materials": materials,
        "buffers": [{ "byteLength": w.bin.len() }],
        "bufferViews": w.views,
        "accessors": w.accessors,
    });
    if !textures.is_empty() {
        doc["images"] = textures
            .iter()
            .map(|uri| json!({ "uri": percent_encode(uri) }))
            .collect();
        doc["textures"] = (0..textures.len()).map(|i| json!({ "source": i })).collect();
    }
    let mut text = serde_json::to_vec(&doc).expect("JSON");
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + w.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(text);
    out.extend((w.bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(w.bin);
    out
}

/// URIs can't hold spaces and a few other characters as they are.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glb_has_its_header_chunks_and_accessors() {
        let vertices = Vertices {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, -1.0]],
            uvs: vec![[0.0; 2]; 3],
            ..Default::default()
        };
        let glb = write(
            "tri",
            &vertices,
            &[Primitive {
                indices: vec![0, 1, 2],
                material: 0,
            }],
            vec![json!({ "name": "m" })],
            &["textures/a b.png".into()],
        );
        assert_eq!(&glb[..4], b"glTF");
        assert_eq!(u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize, glb.len());
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: Json = serde_json::from_slice(&glb[20..20 + len]).unwrap();
        assert_eq!(doc["accessors"][0]["max"], json!([1.0, 2.0, 0.0]));
        assert_eq!(doc["images"][0]["uri"], "textures/a%20b.png");
        assert_eq!(doc["meshes"][0]["primitives"][0]["indices"], 2);
    }
}
