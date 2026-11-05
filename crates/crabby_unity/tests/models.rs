//! Meshes read from model files (FBX) the way Unity's model importer
//! places them, with and without "Bake Axis Conversion".

use crabby_unity::*;
use serde_json::Value as Json;

/// A prefab drawing the model's "Lid" mesh at the origin, unturned.
const PREFAB: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Component:
  - component: {fileID: 2}
  - component: {fileID: 3}
  - component: {fileID: 4}
  m_Name: Lid
  m_IsActive: 1
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!33 &3
MeshFilter:
  m_GameObject: {fileID: 1}
  m_Mesh: {fileID: 4300000, guid: lid00000, type: 3}
--- !u!23 &4
MeshRenderer:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Materials: []
";

/// A mesh as the importer wrote it, in Crabby's space.
struct Mesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl Mesh {
    fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut b = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &self.positions {
            for (i, &c) in p.iter().enumerate() {
                b.0[i] = b.0[i].min(c);
                b.1[i] = b.1[i].max(c);
            }
        }
        b
    }
}

/// Imports the prefab with the hand-written Z-up model `lid_z_up.fbx`
/// next to it, whose `.meta` holds `settings` under `meshes:`.
fn import(name: &str, settings: &str) -> Mesh {
    let dir = std::env::temp_dir().join(format!("crabby_unity_models_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let assets = dir.join("Assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("Lid.fbx"), include_bytes!("fixtures/lid_z_up.fbx")).unwrap();
    std::fs::write(
        assets.join("Lid.fbx.meta"),
        format!("guid: lid00000\nModelImporter:\n  fileIDToRecycleName:\n    4300000: Lid\n  meshes:\n{settings}"),
    )
    .unwrap();
    std::fs::write(assets.join("Lid.prefab"), PREFAB).unwrap();
    std::fs::write(assets.join("Lid.prefab.meta"), "guid: lidprefb\n").unwrap();
    let project = Project::open(&dir).unwrap();
    let out = dir.join("out");
    let file = UnityFile::read(project.root().join("Assets/Lid.prefab")).unwrap();
    let hierarchy = Hierarchy::build(&file, &project).unwrap();
    let mut importer = Importer::new(&project, &out, "lid/");
    let scene = importer.scene(&hierarchy).unwrap();
    importer.finish().unwrap();
    let path = scene["entities"][0]["components"]["Model"]["mesh"].as_str().unwrap();
    let glb = std::fs::read(out.join(path.strip_prefix("lid/").unwrap())).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    read_glb(&glb)
}

/// The first primitive of a `.glb` the importer wrote (f32 vectors, u32
/// indices, one buffer).
fn read_glb(bytes: &[u8]) -> Mesh {
    assert_eq!(&bytes[..4], b"glTF");
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let doc: Json = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
    let bin = &bytes[20 + len + 8..];
    let data = |accessor: &Json| -> &[u8] {
        let a = &doc["accessors"][accessor.as_u64().unwrap() as usize];
        let view = &doc["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
        let at = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        &bin[at..at + view["byteLength"].as_u64().unwrap() as usize]
    };
    let words = |b: &[u8]| -> Vec<[u8; 4]> { b.as_chunks::<4>().0.to_vec() };
    let vec3s = |b: &[u8]| -> Vec<[f32; 3]> {
        words(b)
            .into_iter()
            .map(f32::from_le_bytes)
            .collect::<Vec<_>>()
            .as_chunks::<3>()
            .0
            .to_vec()
    };
    let primitive = &doc["meshes"][0]["primitives"][0];
    Mesh {
        positions: vec3s(data(&primitive["attributes"]["POSITION"])),
        normals: vec3s(data(&primitive["attributes"]["NORMAL"])),
        indices: words(data(&primitive["indices"]))
            .into_iter()
            .map(u32::from_le_bytes)
            .collect(),
    }
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5)
}

/// Every triangle's front (counter-clockwise, as glTF draws) faces the way
/// its vertex normals do.
fn assert_winding_matches_normals(mesh: &Mesh) {
    for tri in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = [0, 1, 2].map(|k| mesh.positions[tri[k] as usize]);
        let (u, v) = ([0, 1, 2].map(|i| b[i] - a[i]), [0, 1, 2].map(|i| c[i] - a[i]));
        let face = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let n = mesh.normals[tri[0] as usize];
        assert!(
            face[0] * n[0] + face[1] * n[1] + face[2] * n[2] > 0.0,
            "{face:?} vs {n:?}"
        );
    }
}

#[test]
fn baked_axis_conversion_turns_z_up_meshes_y_up() {
    // The file's plate lies flat 2 m up its Z axis, 1 m along X and 3 m
    // along Y. Unity bakes the file's right, up and front (X, Z, -Y) into
    // its own +X, +Y and -Z: x stays, the height becomes y and the file's
    // y becomes z. Crabby's space then negates Unity's z.
    let mesh = import(
        "baked",
        "    bakeAxisConversion: 1\n    useFileScale: 1\n    globalScale: 1\n",
    );
    let (min, max) = mesh.bounds();
    assert!(
        close(min, [0.0, 2.0, -3.0]) && close(max, [1.0, 2.0, 0.0]),
        "{min:?} {max:?}"
    );
    assert!(
        mesh.normals.iter().all(|n| close(*n, [0.0, 1.0, 0.0])),
        "faces up: {:?}",
        mesh.normals
    );
    assert_winding_matches_normals(&mesh);
}

#[test]
fn baked_axis_conversion_keeps_the_import_scale() {
    let mesh = import(
        "scaled",
        "    bakeAxisConversion: 1\n    useFileScale: 1\n    globalScale: 2\n",
    );
    let (min, max) = mesh.bounds();
    assert!(
        close(min, [0.0, 4.0, -6.0]) && close(max, [2.0, 4.0, 0.0]),
        "{min:?} {max:?}"
    );
}

#[test]
fn without_baked_axis_conversion_only_x_is_mirrored() {
    // Unity keeps the file's axes in the mesh (and turns the model's root
    // instead): X negated, then Crabby negates z.
    for settings in ["    bakeAxisConversion: 0\n", ""] {
        let mesh = import("unbaked", settings);
        let (min, max) = mesh.bounds();
        assert!(
            close(min, [-1.0, 0.0, -2.0]) && close(max, [0.0, 3.0, -2.0]),
            "{min:?} {max:?}"
        );
        assert!(
            mesh.normals.iter().all(|n| close(*n, [0.0, 0.0, -1.0])),
            "{:?}",
            mesh.normals
        );
        assert_winding_matches_normals(&mesh);
    }
}
