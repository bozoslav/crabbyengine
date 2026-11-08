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
    pub skin: Option<crate::mesh::MeshSkin>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    /// UV set 1 as Unity has it (not flipped: it's data for a shader).
    pub uvs1: Vec<[f32; 2]>,
    /// Morph targets: offsets per vertex (normals' may be empty).
    pub targets: Vec<Target>,
}

/// One morph target of [`Vertices`], in glTF's space.
#[derive(Default)]
pub(crate) struct Target {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub weight: f32,
    pub frame_weight: f32,
    pub channel: String,
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
    let mut skin_json = None;
    if let Some(skin) = &vertices.skin {
        let bytes: Vec<u8> = vertices.joints.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        attributes.insert(
            "JOINTS_0".into(),
            json!(w.add(&bytes, vertices.joints.len(), "VEC4", 5123, json!({}))),
        );
        attributes.insert(
            "WEIGHTS_0".into(),
            json!(w.floats(&vertices.weights, "VEC4", json!({}))),
        );
        let binds = w.floats(&skin.inverse_binds, "MAT4", json!({}));
        w.views
            .last_mut()
            .expect("bind view")
            .as_object_mut()
            .expect("object")
            .remove("target");
        skin_json = Some(
            json!({"joints": skin.joints.iter().map(|j| j + 1).collect::<Vec<_>>(), "inverseBindMatrices": binds}),
        );
    }
    let targets: Vec<Json> = vertices
        .targets
        .iter()
        .map(|t| {
            let (mut min, mut max) = ([0.0f32; 3], [0.0f32; 3]);
            for p in &t.positions {
                for i in 0..3 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
            }
            let mut target = json!({ "POSITION": w.floats(&t.positions, "VEC3", json!({ "min": min, "max": max })) });
            if !t.normals.is_empty() {
                target["NORMAL"] = json!(w.floats(&t.normals, "VEC3", json!({})));
            }
            target
        })
        .collect();
    let primitives: Vec<Json> = primitives
        .iter()
        .filter(|p| !p.indices.is_empty())
        .map(|p| {
            let bytes: Vec<u8> = p.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let indices = w.add(&bytes, p.indices.len(), "SCALAR", 5125, json!({}));
            let mut primitive = json!({ "attributes": attributes, "indices": indices, "material": p.material });
            if !targets.is_empty() {
                primitive["targets"] = json!(targets);
            }
            primitive
        })
        .collect();
    let mut animations = Vec::new();
    if let Some(skin) = &vertices.skin {
        for clip in &skin.animations {
            if clip.times.is_empty() {
                continue;
            }
            let input = w.floats(
                &clip.times.iter().map(|&t| [t]).collect::<Vec<_>>(),
                "SCALAR",
                json!({"min": [clip.times[0]], "max": [clip.times[clip.times.len()-1]]}),
            );
            w.views.last_mut().unwrap().as_object_mut().unwrap().remove("target");
            let mut channels = Vec::new();
            let mut samplers = Vec::new();
            for (node, samples) in clip.nodes.iter().enumerate() {
                if samples.len() != clip.times.len() {
                    continue;
                }
                let poses: Vec<_> = samples
                    .iter()
                    .map(|m| glam::Mat4::from_cols_array(m).to_scale_rotation_translation())
                    .collect();
                let mut rotations = Vec::new();
                let mut previous = glam::Quat::IDENTITY;
                for &(_, mut q, _) in &poses {
                    if previous.dot(q) < 0.0 {
                        q = -q;
                    }
                    rotations.push(q.to_array());
                    previous = q;
                }
                let translations: Vec<_> = poses.iter().map(|p| p.2.to_array()).collect();
                let scales: Vec<_> = poses.iter().map(|p| p.0.to_array()).collect();
                let step = clip.step.get(node).copied().unwrap_or_default();
                for (component, (path, output, constant)) in [
                    (
                        "translation",
                        w.floats(&translations, "VEC3", json!({})),
                        translations.windows(2).all(|v| v[0] == v[1]),
                    ),
                    (
                        "rotation",
                        w.floats(&rotations, "VEC4", json!({})),
                        rotations.windows(2).all(|v| v[0] == v[1]),
                    ),
                    (
                        "scale",
                        w.floats(&scales, "VEC3", json!({})),
                        scales.windows(2).all(|v| v[0] == v[1]),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    samplers.push(json!({"input": input, "output": output, "interpolation": if constant || step[component] {"STEP"} else {"LINEAR"}}));
                    channels.push(json!({"sampler": samplers.len()-1, "target": {"node": node+1, "path": path}}));
                }
                for view in w.views.iter_mut().rev().take(3) {
                    view.as_object_mut().unwrap().remove("target");
                }
            }
            if !clip.morphs.is_empty() && !vertices.targets.is_empty() {
                let values: Vec<[f32; 1]> = (0..clip.times.len())
                    .flat_map(|i| {
                        vertices.targets.iter().map(move |t| {
                            [clip
                                .morphs
                                .iter()
                                .find(|(name, _, _)| *name == t.name)
                                .map_or(t.weight, |(_, v, _)| v[i])]
                        })
                    })
                    .collect();
                let output = w.floats(&values, "SCALAR", json!({}));
                w.views.last_mut().unwrap().as_object_mut().unwrap().remove("target");
                let step = clip.morphs.iter().all(|(_, _, step)| *step);
                samplers
                    .push(json!({"input": input, "output": output, "interpolation": if step {"STEP"} else {"LINEAR"}}));
                channels.push(json!({"sampler": samplers.len()-1, "target": {"node": 0, "path": "weights"}}));
            }
            if !channels.is_empty() {
                animations.push(json!({"name": clip.name, "extras": {"loop": clip.looping, "crabbyAnimatedTargets": clip.morphs.iter().map(|(n,_,_)| n).collect::<Vec<_>>()}, "channels": channels, "samplers": samplers}));
            }
        }
    }
    while !w.bin.len().is_multiple_of(4) {
        w.bin.push(0);
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "ponko_unity" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": name }],
        "meshes": [{ "name": name, "primitives": primitives }],
        "materials": materials,
        "buffers": [{ "byteLength": w.bin.len() }],
        "bufferViews": w.views,
        "accessors": w.accessors,
    });
    if !vertices.targets.is_empty() {
        let names: Vec<&str> = vertices.targets.iter().map(|t| t.name.as_str()).collect();
        doc["meshes"][0]["extras"] = json!({ "targetNames": names });
        doc["meshes"][0]["extras"]["crabbyMorphFrames"] =
            json!(vertices.targets.iter().enumerate().filter_map(|(i,t)| {
            let channel = vertices.targets.iter().position(|p| p.name == t.channel)?;
            Some(json!({"channel": channel, "target": i, "weight": if t.frame_weight > 0. {t.frame_weight} else {1.}}))
        }).collect::<Vec<_>>());
        doc["meshes"][0]["weights"] = json!(vertices.targets.iter().map(|t| t.weight).collect::<Vec<_>>());
    }
    if let (Some(skin), Some(mut skin_json)) = (&vertices.skin, skin_json) {
        if let Some(root) = skin.root {
            skin_json["skeleton"] = json!(root + 1);
        }
        doc["nodes"][0]["skin"] = json!(0);
        for (i, node) in skin.nodes.iter().enumerate() {
            let children: Vec<_> = skin
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.parent == Some(i))
                .map(|(j, _)| j + 1)
                .collect();
            let (scale, rotation, translation) =
                glam::Mat4::from_cols_array(&node.matrix).to_scale_rotation_translation();
            let mut value = json!({"name": node.name, "scale": scale.to_array(), "rotation": rotation.to_array(), "translation": translation.to_array()});
            if !children.is_empty() {
                value["children"] = json!(children);
            }
            doc["nodes"].as_array_mut().expect("nodes").push(value);
            if node.parent.is_none() {
                doc["scenes"][0]["nodes"]
                    .as_array_mut()
                    .expect("roots")
                    .push(json!(i + 1));
            }
        }
        doc["skins"] = json!([skin_json]);
    }
    if !animations.is_empty() {
        doc["animations"] = json!(animations);
    }
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
    fn animation_targets_skin_nodes_and_round_trips_samples() {
        let rest = glam::Mat4::from_translation(glam::vec3(1.0, 2.0, -3.0)).to_cols_array();
        let end = glam::Mat4::from_translation(glam::vec3(2.0, 2.0, -3.0)).to_cols_array();
        let vertices = Vertices {
            positions: vec![[0.0; 3]; 3],
            joints: vec![[0; 4]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
            skin: Some(crate::mesh::MeshSkin {
                nodes: vec![crate::mesh::SkinNode {
                    name: "Bone".into(),
                    parent: None,
                    matrix: rest,
                }],
                joints: vec![0],
                inverse_binds: vec![glam::Mat4::IDENTITY.to_cols_array()],
                animations: vec![crate::mesh::SkinAnimation {
                    morphs: Vec::new(),
                    name: "Move".into(),
                    looping: true,
                    step: vec![[true, false, false]],
                    times: vec![0.0, 0.5],
                    nodes: vec![vec![rest, end]],
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let bytes = write(
            "mesh",
            &vertices,
            &[Primitive {
                indices: vec![0, 1, 2],
                material: 0,
            }],
            vec![json!({})],
            &[],
        );
        let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
        let clip = gltf.animations().next().unwrap();
        assert_eq!(clip.name(), Some("Move"));
        assert_eq!(clip.channels().count(), 3);
        let channel = clip.channels().next().unwrap();
        assert_eq!(channel.sampler().interpolation(), gltf::animation::Interpolation::Step);
        assert_eq!(
            channel.target().node().index(),
            gltf.skins().next().unwrap().joints().next().unwrap().index()
        );
        let reader = channel.reader(|_| gltf.blob.as_deref());
        assert_eq!(reader.read_inputs().unwrap().collect::<Vec<_>>(), [0.0, 0.5]);
        let gltf::animation::util::ReadOutputs::Translations(values) = reader.read_outputs().unwrap() else {
            panic!("translation")
        };
        let values: Vec<_> = values.collect();
        assert_eq!(values, [[1.0, 2.0, -3.0], [2.0, 2.0, -3.0]]);
        assert_eq!(channel.target().node().transform().decomposed().0, values[0]);
    }

    #[test]
    fn skin_round_trips_hierarchy_weights_and_inverse_binds() {
        use crate::mesh::{MeshSkin, SkinNode, influences};
        let (joints, weights) = influences([(0, 0.1), (1, 0.4), (2, 0.2), (3, 0.15), (4, 0.1495), (5, 0.0005)]);
        assert_eq!(joints, [1, 2, 3, 4]);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        let bind = glam::Mat4::from_translation(glam::Vec3::new(0.0, -2.0, 3.0)).to_cols_array();
        let vertices = Vertices {
            positions: vec![[0.0; 3]; 3],
            joints: vec![joints; 3],
            weights: vec![weights; 3],
            skin: Some(MeshSkin {
                nodes: (0..6)
                    .map(|i| SkinNode {
                        name: format!("bone{i}"),
                        parent: (i > 0).then_some(0),
                        matrix: glam::Mat4::IDENTITY.to_cols_array(),
                    })
                    .collect(),
                joints: (0..6).collect(),
                inverse_binds: vec![bind; 6],
                ..Default::default()
            }),
            ..Default::default()
        };
        let bytes = write(
            "skin",
            &vertices,
            &[Primitive {
                indices: vec![0, 1, 2],
                material: 0,
            }],
            vec![json!({})],
            &[],
        );
        let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
        let blob = gltf.blob.as_deref().unwrap();
        let skin = gltf.skins().next().unwrap();
        assert_eq!(
            skin.joints().map(|n| n.index()).collect::<Vec<_>>(),
            (1..7).collect::<Vec<_>>()
        );
        assert_eq!(gltf.nodes().nth(1).unwrap().children().count(), 5);
        assert_eq!(
            skin.reader(|_| Some(blob))
                .read_inverse_bind_matrices()
                .unwrap()
                .next()
                .unwrap()
                .concat(),
            bind
        );
        let primitive = gltf.meshes().next().unwrap().primitives().next().unwrap();
        let reader = primitive.reader(|_| Some(blob));
        assert_eq!(reader.read_joints(0).unwrap().into_u16().next().unwrap(), joints);
        assert_eq!(reader.read_weights(0).unwrap().into_f32().next().unwrap(), weights);
        assert!((weights[0] - 0.4 / 0.8995).abs() < 1e-6);
        let (_, dropped) = influences([(0, 0.9995), (1, 0.0005)]);
        assert_eq!(dropped, [1.0, 0.0, 0.0, 0.0]);
    }

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
