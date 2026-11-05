//! Plain Unity AnimationClip transform curves.
use crate::{Error, MeshSkin, SkinAnimation, UnityFile, Value};
use glam::{Mat4, Quat, Vec3};

/// A plain-curve Unity `.anim` clip. Compressed and binary clips are unsupported.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    name: String,
    rate: f32,
    start: f32,
    stop: f32,
    looping: bool,
    curves: Vec<Curve>,
    skipped: Vec<String>,
    morphs: Vec<(String, Curve)>,
}
#[derive(Debug, Clone)]
struct Curve {
    path: String,
    kind: usize,
    keys: Vec<Key>,
}
#[derive(Debug, Clone)]
struct Key {
    time: f32,
    value: [f32; 4],
    incoming: [f32; 4],
    outgoing: [f32; 4],
}
fn number(v: &Value) -> f32 {
    match v.as_str().unwrap_or("") {
        ".inf" | "Infinity" => f32::INFINITY,
        "-.inf" | "-Infinity" => f32::NEG_INFINITY,
        s => s.parse().unwrap_or(0.0),
    }
}
fn vector(v: &Value) -> [f32; 4] {
    ["x", "y", "z", "w"].map(|k| number(&v[k]))
}
impl AnimationClip {
    /// Parses text YAML; weighted tangents are approximated as unweighted Hermite.
    ///
    /// ```
    /// let clip = crabby_unity::AnimationClip::parse("--- !u!74 &1\nAnimationClip:\n  m_Name: Idle\n  m_SampleRate: 30\n").unwrap();
    /// ```
    /// # Errors
    /// Returns an error for binary/compressed clips, malformed YAML or invalid timing.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if !text.trim_start().starts_with(['%', '-']) || text.contains('\0') {
            return Err(Error::new(
                "animation clip is compressed or binary; plain YAML curves required",
            ));
        }
        let file = UnityFile::parse(text)?;
        let m = &file
            .of_class(74)
            .next()
            .ok_or_else(|| Error::new("expected AnimationClip"))?
            .body;
        if number(&m["m_Compressed"]) != 0.0
            || has_data(&m["m_StreamedClip"])
            || has_data(&m["m_DenseClip"])
            || has_data(&m["m_MuscleClip"]["m_Clip"]["m_StreamedClip"]["data"])
            || has_data(&m["m_MuscleClip"]["m_Clip"]["m_DenseClip"]["m_SampleArray"])
            || has_data(&m["m_CompressedRotationCurves"])
        {
            return Err(Error::new("animation clip is compressed; plain YAML curves required"));
        }
        let mut curves = Vec::new();
        let mut skipped = Vec::new();
        for field in ["m_PPtrCurves", "m_ClipBindingConstant"] {
            if has_data(&m[field]) {
                skipped.push(format!("skipped {field}"));
            }
        }
        for (kind, field) in ["m_PositionCurves", "m_RotationCurves", "m_ScaleCurves", "m_EulerCurves"]
            .iter()
            .enumerate()
        {
            for entry in m[*field].as_seq().unwrap_or_default() {
                let mut keys = Vec::new();
                for k in entry["curve"]["m_Curve"].as_seq().unwrap_or_default() {
                    if number(&k["weightedMode"]) != 0.0 {
                        skipped.push("weighted tangents approximated as unweighted Hermite".into());
                    }
                    let key = Key {
                        time: number(&k["time"]),
                        value: vector(&k["value"]),
                        incoming: vector(&k["inSlope"]),
                        outgoing: vector(&k["outSlope"]),
                    };
                    if !key.time.is_finite() || key.value.iter().any(|v| !v.is_finite()) {
                        return Err(Error::new("non-finite animation key"));
                    }
                    keys.push(key);
                }
                keys.sort_by(|a, b| a.time.total_cmp(&b.time));
                if keys.windows(2).any(|w| w[0].time == w[1].time) {
                    return Err(Error::new("duplicate animation key time"));
                }
                if !keys.is_empty() {
                    curves.push(Curve {
                        path: entry["path"].as_str().unwrap_or("").into(),
                        kind,
                        keys,
                    });
                }
            }
        }
        let mut morphs = Vec::new();
        for entry in m["m_FloatCurves"].as_seq().unwrap_or_default() {
            let name = entry["attribute"].as_str().unwrap_or("").strip_prefix("blendShape.");
            if let Some(name) = name.filter(|_| entry["classID"].as_i64() == Some(137)) {
                let mut keys = Vec::new();
                for k in entry["curve"]["m_Curve"].as_seq().unwrap_or_default() {
                    let time = number(&k["time"]);
                    let value = number(&k["value"]) / 100.;
                    let incoming = number(&k["inSlope"]) / 100.;
                    let outgoing = number(&k["outSlope"]) / 100.;
                    if !time.is_finite() || !value.is_finite() || incoming.is_nan() || outgoing.is_nan() {
                        return Err(Error::new("non-finite morph key"));
                    }
                    keys.push(Key {
                        time,
                        value: [value; 4],
                        incoming: [incoming; 4],
                        outgoing: [outgoing; 4],
                    });
                }
                keys.sort_by(|a, b| a.time.total_cmp(&b.time));
                if keys.windows(2).any(|w| w[0].time == w[1].time) {
                    return Err(Error::new("duplicate morph key time"));
                }
                if !keys.is_empty() {
                    morphs.push((
                        name.into(),
                        Curve {
                            path: entry["path"].as_str().unwrap_or("").into(),
                            kind: 0,
                            keys,
                        },
                    ));
                }
            } else {
                skipped.push("skipped m_FloatCurves property".into());
            }
        }
        let settings = &m["m_AnimationClipSettings"];
        let start = number(&settings["m_StartTime"]);
        let stop = settings["m_StopTime"].as_f32().unwrap_or_else(|| {
            curves
                .iter()
                .chain(morphs.iter().map(|(_, c)| c))
                .flat_map(|c| c.keys.iter().map(|k| k.time))
                .fold(start, f32::max)
        });
        let rate = m["m_SampleRate"].as_f32().unwrap_or(60.0);
        if !start.is_finite()
            || !stop.is_finite()
            || stop < start
            || !rate.is_finite()
            || rate <= 0.0
            || (stop - start) * rate > 1_000_000.0
        {
            return Err(Error::new("invalid animation clip timing/sample rate"));
        }
        Ok(Self {
            name: m["m_Name"].as_str().unwrap_or("Clip").into(),
            rate,
            start,
            stop,
            looping: settings["m_LoopTime"].as_bool().unwrap_or(false),
            curves,
            skipped,
            morphs,
        })
    }

    /// Reads a plain YAML `.anim` asset.
    ///
    /// ```no_run
    /// let clip = crabby_unity::AnimationClip::read("Assets/Walk.anim")?;
    /// # Ok::<(), crabby_unity::Error>(())
    /// ```
    /// # Errors
    /// Reports file errors and unsupported compressed/binary clips.
    pub fn read(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| Error::new(e.to_string()))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| Error::new("animation clip is compressed or binary; plain YAML curves required"))?;
        Self::parse(text)
    }

    /// Appends sampled local poses in Unity space, returning skipped/unresolved diagnostics.
    /// `root` is the node animated by an empty path. Other paths walk child names.
    /// Export through `Importer::animation_clips` to apply the normal Z mirror.
    ///
    /// ```
    /// # use crabby_unity::{AnimationClip, MeshSkin};
    /// # let clip = AnimationClip::parse("--- !u!74 &1\nAnimationClip: {}\n").unwrap();
    /// let mut skin = MeshSkin::default();
    /// let diagnostics = clip.append_to_skin(&mut skin, 0);
    /// ```
    pub fn append_to_skin(&self, skin: &mut MeshSkin, root: usize) -> Vec<String> {
        let mut diagnostics = self.skipped.clone();
        let count = ((self.stop - self.start) * self.rate).ceil() as usize;
        let mut times: Vec<_> = (0..=count)
            .map(|i| (i as f32 / self.rate).min(self.stop - self.start))
            .collect();
        // Keep source keys, including discontinuities, alongside regular samples.
        times.extend(
            self.curves
                .iter()
                .flat_map(|c| c.keys.iter())
                .filter(|k| k.time >= self.start && k.time <= self.stop)
                .map(|k| k.time - self.start),
        );
        times.extend(
            self.morphs
                .iter()
                .flat_map(|(_, c)| c.keys.iter())
                .filter(|k| k.time >= self.start && k.time <= self.stop)
                .map(|k| k.time - self.start),
        );
        times.sort_by(f32::total_cmp);
        times.dedup();
        let mut morphs = Vec::new();
        for (name, c) in &self.morphs {
            let mut node = (root < skin.nodes.len()).then_some(root);
            for part in c.path.split('/').filter(|s| !s.is_empty()) {
                node = node.and_then(|parent| {
                    let mut found = skin
                        .nodes
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| n.parent == Some(parent) && n.name == part);
                    let first = found.next().map(|(i, _)| i);
                    if found.next().is_some() { None } else { first }
                });
            }
            if node.is_some() {
                morphs.push((
                    name.clone(),
                    times.iter().map(|t| c.sample(*t + self.start)[0]).collect(),
                    c.keys
                        .windows(2)
                        .all(|w| w[0].outgoing[0].is_infinite() || w[1].incoming[0].is_infinite()),
                ));
            } else {
                diagnostics.push(format!("{}: unresolved morph path {:?}", self.name, c.path));
            }
        }
        let mut animation = SkinAnimation {
            morphs,
            name: self.name.clone(),
            looping: self.looping,
            times,
            nodes: vec![Vec::new(); skin.nodes.len()],
            step: vec![[false; 3]; skin.nodes.len()],
        };
        let mut resolved = Vec::new();
        for curve in &self.curves {
            let mut node = (root < skin.nodes.len()).then_some(root);
            for part in curve.path.split('/').filter(|s| !s.is_empty()) {
                node = node.and_then(|parent| {
                    let mut matches = skin
                        .nodes
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| n.parent == Some(parent) && n.name == part);
                    let first = matches.next().map(|(i, _)| i);
                    if matches.next().is_some() { None } else { first }
                });
            }
            if let Some(node) = node {
                resolved.push((node, curve));
            } else {
                diagnostics.push(format!("{}: unresolved transform path {:?}", self.name, curve.path));
            }
        }
        for (node, rest) in skin.nodes.iter().enumerate() {
            let tracks: Vec<_> = resolved.iter().filter(|(n, _)| *n == node).map(|(_, c)| *c).collect();
            if tracks.is_empty() {
                continue;
            }
            for c in &tracks {
                animation.step[node][if c.kind == 3 { 1 } else { c.kind }] = c.keys.windows(2).all(|w| {
                    (0..if c.kind == 1 { 4 } else { 3 })
                        .all(|i| w[0].outgoing[i].is_infinite() || w[1].incoming[i].is_infinite())
                });
            }
            for &time in &animation.times {
                let (mut s, mut r, mut t) = Mat4::from_cols_array(&rest.matrix).to_scale_rotation_translation();
                for c in &tracks {
                    let v = c.sample(time + self.start);
                    match c.kind {
                        0 => t = Vec3::from_array([v[0], v[1], v[2]]),
                        1 => {
                            let q = Quat::from_array(v);
                            r = if q.length_squared() > 0.0 {
                                q.normalize()
                            } else {
                                Quat::IDENTITY
                            };
                        }
                        2 => s = Vec3::from_array([v[0], v[1], v[2]]),
                        _ => {
                            r = Quat::from_rotation_y(v[1].to_radians())
                                * Quat::from_rotation_x(v[0].to_radians())
                                * Quat::from_rotation_z(v[2].to_radians())
                        }
                    }
                }
                animation.nodes[node].push(Mat4::from_scale_rotation_translation(s, r, t).to_cols_array());
            }
        }
        if !animation.morphs.is_empty() || animation.nodes.iter().any(|n| !n.is_empty()) {
            skin.animations.push(animation);
        }
        diagnostics.sort();
        diagnostics.dedup();
        diagnostics
    }
}
fn has_data(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Seq(s) => !s.is_empty(),
        Value::Map(m) => m.iter().any(|(_, v)| has_data(v)),
        Value::Scalar(s) => !s.is_empty() && s != "0",
        Value::Bytes(b) => !b.is_empty(),
        Value::Numbers(n) => !n.is_empty(),
    }
}
impl Curve {
    fn sample(&self, time: f32) -> [f32; 4] {
        let right = self.keys.partition_point(|k| k.time <= time);
        if right == 0 {
            return self.keys[0].value;
        }
        let a = &self.keys[right - 1];
        let Some(b) = self.keys.get(right) else {
            return a.value;
        };
        let dt = b.time - a.time;
        let u = (time - a.time) / dt;
        std::array::from_fn(|i| {
            if a.outgoing[i].is_infinite() || b.incoming[i].is_infinite() {
                return a.value[i];
            }
            (2.0 * u.powi(3) - 3.0 * u * u + 1.0) * a.value[i]
                + (u.powi(3) - 2.0 * u * u + u) * dt * a.outgoing[i]
                + (-2.0 * u.powi(3) + 3.0 * u * u) * b.value[i]
                + (u.powi(3) - u * u) * dt * b.incoming[i]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yaml_curves_round_trip_sampled_hermite_and_zxy() {
        let mut text = String::from(
            "--- !u!74 &1\nAnimationClip:\n  m_Name: Motion\n  m_SampleRate: 2\n  m_AnimationClipSettings: {m_StartTime: 1, m_StopTime: 2, m_LoopTime: 1}\n  m_FloatCurves: [{attribute: ignored}]\n",
        );
        for (field, path, a, b, outgoing, incoming) in [
            (
                "m_PositionCurves",
                "",
                "{x: 0, y: 0, z: 0}",
                "{x: 2, y: 0, z: 4}",
                "{x: 4, y: 0, z: 0}",
                "{x: 0, y: 0, z: 0}",
            ),
            (
                "m_ScaleCurves",
                "Child",
                "{x: 1, y: 1, z: 1}",
                "{x: 3, y: 3, z: 3}",
                "{x: .inf, y: .inf, z: .inf}",
                "{x: 0, y: 0, z: 0}",
            ),
            (
                "m_RotationCurves",
                "",
                "{x: 0, y: 0, z: 0, w: 1}",
                "{x: 0, y: 1, z: 0, w: 0}",
                "{}",
                "{}",
            ),
            (
                "m_EulerCurves",
                "Child",
                "{x: 30, y: 40, z: 50}",
                "{x: 30, y: 40, z: 50}",
                "{}",
                "{}",
            ),
        ] {
            text.push_str(&format!("  {field}:\n  - path: '{path}'\n    curve:\n      m_Curve:\n      - time: 1\n        value: {a}\n        inSlope: {{}}\n        outSlope: {outgoing}\n      - time: 2\n        value: {b}\n        inSlope: {incoming}\n        outSlope: {{}}\n"));
        }
        text.push_str("  m_PositionCurves_UNUSED: []\n");
        // A second entry in an existing curve list exercises unresolved paths.
        text = text.replace(
            "  m_ScaleCurves:",
            "  - path: Missing\n    curve:\n      m_Curve:\n      - {time: 1, value: {x: 9}}\n  m_ScaleCurves:",
        );
        let clip = AnimationClip::parse(&text).unwrap();
        let mut skin = MeshSkin {
            root: Some(0),
            nodes: vec![
                crate::SkinNode {
                    name: "Root".into(),
                    parent: None,
                    matrix: Mat4::IDENTITY.to_cols_array(),
                },
                crate::SkinNode {
                    name: "Child".into(),
                    parent: Some(0),
                    matrix: Mat4::IDENTITY.to_cols_array(),
                },
            ],
            joints: vec![0, 1],
            inverse_binds: vec![Mat4::IDENTITY.to_cols_array(); 2],
            ..Default::default()
        };
        let report = clip.append_to_skin(&mut skin, 0);
        assert!(report.iter().any(|s| s.contains("Missing")));
        assert!(report.iter().any(|s| s.contains("m_FloatCurves")));
        let mirror = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
        for n in &mut skin.animations[0].nodes {
            for m in n {
                *m = (mirror * Mat4::from_cols_array(m) * mirror).to_cols_array();
            }
        }
        let vertices = crate::glb::Vertices {
            skin: Some(skin),
            positions: vec![[0.0; 3]; 3],
            joints: vec![[0; 4]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
            ..Default::default()
        };
        let bytes = crate::glb::write(
            "Model",
            &vertices,
            &[crate::glb::Primitive {
                indices: vec![0, 1, 2],
                material: 0,
            }],
            vec![serde_json::json!({})],
            &[],
        );
        let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
        for channel in gltf.animations().next().unwrap().channels() {
            let reader = channel.reader(|_| gltf.blob.as_deref());
            assert_eq!(reader.read_inputs().unwrap().collect::<Vec<_>>(), [0.0, 0.5, 1.0]);
            let child = channel.target().node().name() == Some("Child");
            match reader.read_outputs().unwrap() {
                gltf::animation::util::ReadOutputs::Translations(v) if !child => assert_eq!(
                    v.collect::<Vec<_>>(),
                    [[0.0, 0.0, 0.0], [1.5, 0.0, -2.0], [2.0, 0.0, -4.0]]
                ),
                gltf::animation::util::ReadOutputs::Scales(v) if child => {
                    for (actual, expected) in v.zip([[1.0; 3], [1.0; 3], [3.0; 3]]) {
                        assert!(actual.into_iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-5));
                    }
                    assert_eq!(channel.sampler().interpolation(), gltf::animation::Interpolation::Step);
                }
                gltf::animation::util::ReadOutputs::Rotations(v) => {
                    let values: Vec<_> = v.into_f32().collect();
                    let expected = if child {
                        Quat::from_rotation_y(40f32.to_radians())
                            * Quat::from_rotation_x(30f32.to_radians())
                            * Quat::from_rotation_z(50f32.to_radians())
                    } else {
                        Quat::from_xyzw(0.0, 0.5, 0.0, 0.5).normalize()
                    };
                    let expected = Quat::from_mat4(&(mirror * Mat4::from_quat(expected) * mirror));
                    assert!(Quat::from_array(values[1]).dot(expected).abs() > 0.99999);
                }
                _ => {}
            }
        }
    }
    #[test]
    fn blend_shape_curves_scale_percent_and_keep_sparse_names() {
        let clip = AnimationClip::parse("--- !u!74 &1\nAnimationClip:\n  m_Name: Face\n  m_SampleRate: 2\n  m_FloatCurves:\n  - attribute: blendShape.Raise\n    classID: 137\n    path: ''\n    curve:\n      m_Curve:\n      - {time: 0, value: 0, inSlope: 100, outSlope: 100}\n      - {time: 1, value: 100, inSlope: 100, outSlope: 100}\n").unwrap();
        let mut skin = MeshSkin {
            nodes: vec![crate::SkinNode {
                name: "Root".into(),
                parent: None,
                matrix: Mat4::IDENTITY.to_cols_array(),
            }],
            ..Default::default()
        };
        assert!(clip.append_to_skin(&mut skin, 0).is_empty());
        assert_eq!(skin.animations[0].times, [0., 0.5, 1.]);
        assert_eq!(skin.animations[0].morphs, [("Raise".into(), vec![0., 0.5, 1.], false)]);
        let vertices = crate::glb::Vertices {
            skin: Some(skin),
            positions: vec![[0.; 3]; 3],
            joints: vec![[0; 4]; 3],
            weights: vec![[1., 0., 0., 0.]; 3],
            targets: vec![
                crate::glb::Target {
                    name: "Raise".into(),
                    positions: vec![[0.; 3]; 3],
                    ..Default::default()
                },
                crate::glb::Target {
                    name: "Untouched".into(),
                    positions: vec![[0.; 3]; 3],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let bytes = crate::glb::write(
            "Face",
            &vertices,
            &[crate::glb::Primitive {
                indices: vec![0, 1, 2],
                material: 0,
            }],
            vec![serde_json::json!({})],
            &[],
        );
        let g = gltf::Gltf::from_slice(&bytes).unwrap();
        let a = g.animations().next().unwrap();
        let ch = a
            .channels()
            .find(|c| c.target().property() == gltf::animation::Property::MorphTargetWeights)
            .unwrap();
        let gltf::animation::util::ReadOutputs::MorphTargetWeights(v) =
            ch.reader(|_| g.blob.as_deref()).read_outputs().unwrap()
        else {
            panic!()
        };
        assert_eq!(v.into_f32().collect::<Vec<_>>(), [0., 0., 0.5, 0., 1., 0.]);
        let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let doc: serde_json::Value = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
        assert_eq!(
            doc["animations"][0]["extras"]["crabbyAnimatedTargets"],
            serde_json::json!(["Raise"])
        );
    }
    #[test]
    fn compressed_and_binary_clips_report_the_limitation() {
        for text in [
            "binary",
            "--- !u!74 &1\nAnimationClip:\n  m_Compressed: 1\n",
            "--- !u!74 &1\nAnimationClip:\n  m_StreamedClip: {data: [1]}\n",
        ] {
            assert!(AnimationClip::parse(text).unwrap_err().message.contains("compressed"));
        }
    }
}
