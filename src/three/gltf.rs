//! Loading glTF 2.0 files (`.glb`, or `.gltf` with its buffers and images
//! next to it) into one [`MeshData`] with a part per material.
//!
//! Every mesh in the file's default scene is placed where its node puts it
//! and merged, so a file is one [`Mesh`](crate::Mesh): a tree, a house, a
//! car. Node hierarchy and animation aren't kept (yet).

use std::ops::Range;

use glam::{Mat3, Mat4, Vec2, Vec3};

use super::MeshData;
use crate::Color;

/// A loaded file, before it goes to the GPU.
pub(crate) struct Loaded {
    pub data: MeshData,
    /// Index ranges of `data`, each with its material (an index into `materials`).
    pub parts: Vec<(Range<u32>, usize)>,
    pub materials: Vec<LoadedMaterial>,
}

pub(crate) struct LoadedMaterial {
    pub color: Color,
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: Color,
    /// Base color image: width, height, RGBA8 pixels.
    pub image: Option<(u32, u32, Vec<u8>)>,
}

/// Parses glTF `bytes`; `read` fetches files it refers to, by asset name.
pub(crate) fn load(
    name: &str,
    bytes: &[u8],
    read: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<Loaded, String> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| format!("{name}: not a glTF file: {e}"))?;
    let folder = name.rfind('/').map_or("", |i| &name[..=i]);
    let mut buffers = Vec::new();
    for buffer in gltf.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => gltf
                .blob
                .clone()
                .ok_or_else(|| format!("{name}: missing its binary chunk"))?,
            gltf::buffer::Source::Uri(uri) => fetch(uri, folder, read)?,
        };
        if data.len() < buffer.length() {
            return Err(format!("{name}: buffer {} is shorter than it says", buffer.index()));
        }
        buffers.push(data);
    }

    // Materials, with the base color image decoded once each.
    let mut materials: Vec<LoadedMaterial> = Vec::new();
    for material in gltf.materials() {
        let pbr = material.pbr_metallic_roughness();
        let [r, g, b, a] = pbr.base_color_factor();
        let [er, eg, eb] = material.emissive_factor();
        let image = match pbr.base_color_texture() {
            Some(info) => Some(image(&info.texture().source(), &buffers, folder, read, name)?),
            None => None,
        };
        // glTF factors are linear; Color is sRGB.
        materials.push(LoadedMaterial {
            color: Color::rgba(to_srgb(r), to_srgb(g), to_srgb(b), a),
            roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            emissive: Color::rgb(to_srgb(er), to_srgb(eg), to_srgb(eb)),
            image,
        });
    }
    let default_material = materials.len();

    // Every primitive in the scene, placed by its node, grouped by material.
    let scene = gltf
        .default_scene()
        .or_else(|| gltf.scenes().next())
        .ok_or_else(|| format!("{name}: has no scene"))?;
    let mut by_material: Vec<(usize, MeshData)> = Vec::new();
    let mut stack: Vec<(gltf::Node, Mat4)> = scene.nodes().map(|n| (n, Mat4::IDENTITY)).collect();
    while let Some((node, parent)) = stack.pop() {
        let matrix = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    continue; // lines and points aren't drawn
                }
                let material = primitive.material().index().unwrap_or(default_material);
                let at = match by_material.iter().position(|(m, _)| *m == material) {
                    Some(i) => i,
                    None => {
                        by_material.push((material, MeshData::default()));
                        by_material.len() - 1
                    }
                };
                append(&mut by_material[at].1, &primitive, &buffers, matrix, name)?;
            }
        }
        stack.extend(node.children().map(|c| (c, matrix)));
    }
    if by_material.is_empty() {
        return Err(format!("{name}: has no triangles"));
    }
    if by_material.iter().any(|(m, _)| *m == default_material) {
        materials.push(LoadedMaterial {
            color: Color::WHITE,
            roughness: 0.6,
            metallic: 0.0,
            emissive: Color::rgb(0.0, 0.0, 0.0),
            image: None,
        });
    }

    let mut data = MeshData::default();
    let mut parts = Vec::with_capacity(by_material.len());
    for (material, part) in by_material {
        let base = data.positions.len() as u32;
        let first = data.indices.len() as u32;
        data.positions.extend(part.positions);
        data.normals.extend(part.normals);
        data.uvs.extend(part.uvs);
        data.indices.extend(part.indices.iter().map(|i| i + base));
        parts.push((first..data.indices.len() as u32, material));
    }
    data.check().map_err(|e| format!("{name}: {e}"))?;
    Ok(Loaded { data, parts, materials })
}

/// Adds one primitive's triangles to `out`, moved by `matrix`.
fn append(
    out: &mut MeshData,
    primitive: &gltf::Primitive,
    buffers: &[Vec<u8>],
    matrix: Mat4,
    name: &str,
) -> Result<(), String> {
    let reader = primitive.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
    let positions: Vec<Vec3> = reader
        .read_positions()
        .ok_or_else(|| format!("{name}: a mesh has no positions"))?
        .map(Vec3::from)
        .collect();
    let count = positions.len() as u32;
    let indices: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..count).collect(),
    };
    let part = MeshData {
        positions,
        normals: reader
            .read_normals()
            .map(|n| n.map(Vec3::from).collect())
            .unwrap_or_default(),
        uvs: reader
            .read_tex_coords(0)
            .map(|t| t.into_f32().map(Vec2::from).collect())
            .unwrap_or_default(),
        indices,
    };
    part.check().map_err(|e| format!("{name}: {e}"))?;
    let normals = part.normals_or_computed().into_owned();
    // A mirroring node flips triangles inside out: swap two corners back.
    let flips = matrix.determinant() < 0.0;
    let normal_matrix = Mat3::from_mat4(matrix).inverse().transpose();
    let base = out.positions.len() as u32;
    out.positions
        .extend(part.positions.iter().map(|p| matrix.transform_point3(*p)));
    out.normals
        .extend(normals.iter().map(|n| (normal_matrix * *n).normalize_or(Vec3::Y)));
    if part.uvs.is_empty() {
        out.uvs.extend(std::iter::repeat_n(Vec2::ZERO, count as usize));
    } else {
        out.uvs.extend(part.uvs);
    }
    for tri in part.indices.as_chunks::<3>().0 {
        let [a, b, c] = [tri[0] + base, tri[1] + base, tri[2] + base];
        out.indices.extend(if flips { [a, c, b] } else { [a, b, c] });
    }
    Ok(())
}

/// Decodes an image of the file to RGBA8.
fn image(
    image: &gltf::Image,
    buffers: &[Vec<u8>],
    folder: &str,
    read: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    name: &str,
) -> Result<(u32, u32, Vec<u8>), String> {
    let bytes = match image.source() {
        gltf::image::Source::View { view, .. } => {
            let buffer = &buffers[view.buffer().index()];
            buffer
                .get(view.offset()..view.offset() + view.length())
                .ok_or_else(|| format!("{name}: an image is past the end of its buffer"))?
                .to_vec()
        }
        gltf::image::Source::Uri { uri, .. } => fetch(uri, folder, read)?,
    };
    let decoded = image::load_from_memory(&bytes).map_err(|e| format!("{name}: an image can't be read: {e}"))?;
    let rgba = decoded.to_rgba8();
    Ok((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// The bytes a URI names: inline base64 data, or a file next to the glTF.
fn fetch(uri: &str, folder: &str, read: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>) -> Result<Vec<u8>, String> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (_, data) = rest
            .split_once(";base64,")
            .ok_or("only base64 data URIs are supported")?;
        return base64(data);
    }
    let file = percent_decode(uri);
    read(&format!("{folder}{file}"))
}

/// `%20` and friends in file names.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn base64(text: &str) -> Result<Vec<u8>, String> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = value(c).ok_or("bad base64 in a data URI")?;
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

fn to_srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_percent_decoding() {
        assert_eq!(base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64("AAEC").unwrap(), [0, 1, 2]);
        assert!(base64("a$").is_err());
        assert_eq!(percent_decode("my%20tree.bin"), "my tree.bin");
    }

    /// A one-triangle .gltf under a node that moves it up by 2 and mirrors
    /// x; its buffer inline, or in `file` next to it.
    fn triangle_gltf(file: Option<&str>) -> String {
        let mut bytes = Vec::new();
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in v {
                bytes.extend(c.to_le_bytes());
            }
        }
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut b64 = String::new();
        for chunk in bytes.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
            for i in 0..=chunk.len() {
                b64.push(alphabet[(n >> (18 - 6 * i) & 63) as usize] as char);
            }
        }
        let uri = file.map_or(format!("data:application/octet-stream;base64,{b64}"), str::to_string);
        format!(
            r#"{{
            "asset": {{ "version": "2.0" }},
            "scene": 0,
            "scenes": [{{ "nodes": [0] }}],
            "nodes": [{{ "mesh": 0, "translation": [0, 2, 0], "scale": [-1, 1, 1] }}],
            "meshes": [{{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "material": 0 }}] }}],
            "materials": [{{ "pbrMetallicRoughness": {{ "baseColorFactor": [1, 0, 0, 1], "roughnessFactor": 0.25 }} }}],
            "buffers": [{{ "byteLength": 36, "uri": "{uri}" }}],
            "bufferViews": [{{ "buffer": 0, "byteLength": 36 }}],
            "accessors": [{{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                            "min": [0, 0, 0], "max": [1, 1, 0] }}]
        }}"#
        )
    }

    #[test]
    fn a_gltf_file_loads_placed_by_its_node_with_its_material() {
        let loaded = load("tri.gltf", triangle_gltf(None).as_bytes(), &mut |f| Err(f.to_string())).unwrap();
        let data = &loaded.data;
        assert_eq!(
            data.positions,
            vec![
                Vec3::new(0.0, 2.0, 0.0),
                Vec3::new(-1.0, 2.0, 0.0),
                Vec3::new(0.0, 3.0, 0.0)
            ]
        );
        // Mirrored: the winding is swapped so the computed normal (+Z) faces front.
        assert_eq!(data.indices, vec![0, 2, 1]);
        let [a, b, c] = [0, 2, 1].map(|i| data.positions[i]);
        assert!((b - a).cross(c - a).dot(data.normals[0]) > 0.0);
        assert_eq!(loaded.parts, vec![(0..3, 0)]);
        let m = &loaded.materials[0];
        assert!((m.color.r - 1.0).abs() < 1e-5 && m.color.g == 0.0);
        assert_eq!((m.roughness, m.metallic), (0.25, 1.0), "metallic defaults to 1 in glTF");
    }

    #[test]
    fn missing_files_and_junk_are_errors() {
        assert!(load("junk.glb", b"not a gltf", &mut |_| Ok(vec![])).is_err());
        let external = triangle_gltf(Some("my%20tri.bin"));
        let err = load("models/tri.gltf", external.as_bytes(), &mut |f| Err(format!("no {f}")))
            .err()
            .unwrap();
        assert_eq!(err, "no models/my tri.bin");
    }
}
