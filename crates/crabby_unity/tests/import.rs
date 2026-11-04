//! Importing a small hand-written Unity project into Crabby scene files.

use crabby_unity::*;
use serde_json::Value as Json;

/// A Unity project in a temp dir of its own (tests run in parallel), and
/// an output folder in it.
struct Fixture {
    dir: std::path::PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const MATERIAL: &str = "%YAML 1.1
--- !u!21 &2100000
Material:
  m_Name: Bark
  m_Shader: {fileID: 4800000, guid: lit00000, type: 3}
  m_SavedProperties:
    m_TexEnvs:
    - _BaseMap:
        m_Texture: {fileID: 2800000, guid: bark0000, type: 3}
        m_Scale: {x: 2, y: 1}
        m_Offset: {x: 0, y: 0}
    - _MetallicGlossMap:
        m_Texture: {fileID: 2800000, guid: gloss000, type: 3}
        m_Scale: {x: 1, y: 1}
        m_Offset: {x: 0, y: 0}
    m_Floats:
    - _Smoothness: 0.5
    - _AlphaClip: 1
    - _Cutoff: 0.3
    - _Cull: 0
    m_Colors:
    - _BaseColor: {r: 1, g: 0.5, b: 0, a: 1}
";

/// A prefab: a root at (1, 2, 3) turned 90° around Y, with a child at
/// (0, 0, 1) drawing the quad; a LOD 1 child and an inactive one that
/// draw it too, and a model instance.
const PREFAB: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Component:
  - component: {fileID: 2}
  - component: {fileID: 9}
  m_Name: Root
  m_IsActive: 1
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0.70710677, z: 0, w: 0.70710677}
  m_LocalPosition: {x: 1, y: 2, z: 3}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children:
  - {fileID: 4}
  - {fileID: 14}
  - {fileID: 24}
  m_Father: {fileID: 0}
--- !u!205 &9
LODGroup:
  m_GameObject: {fileID: 1}
  m_LODs:
  - renderers:
    - renderer: {fileID: 6}
  - renderers:
    - renderer: {fileID: 16}
--- !u!1 &3
GameObject:
  m_Component:
  - component: {fileID: 4}
  - component: {fileID: 5}
  - component: {fileID: 6}
  m_Name: Near
  m_IsActive: 1
--- !u!4 &4
Transform:
  m_GameObject: {fileID: 3}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 1}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!33 &5
MeshFilter:
  m_GameObject: {fileID: 3}
  m_Mesh: {fileID: 4300000, guid: quad0000, type: 2}
--- !u!23 &6
MeshRenderer:
  m_GameObject: {fileID: 3}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: bark0mat, type: 2}
--- !u!1 &13
GameObject:
  m_Component:
  - component: {fileID: 14}
  - component: {fileID: 15}
  - component: {fileID: 16}
  m_Name: Far
  m_IsActive: 1
--- !u!4 &14
Transform:
  m_GameObject: {fileID: 13}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!33 &15
MeshFilter:
  m_GameObject: {fileID: 13}
  m_Mesh: {fileID: 4300000, guid: quad0000, type: 2}
--- !u!23 &16
MeshRenderer:
  m_GameObject: {fileID: 13}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: bark0mat, type: 2}
--- !u!1 &23
GameObject:
  m_Component:
  - component: {fileID: 24}
  - component: {fileID: 25}
  - component: {fileID: 26}
  m_Name: Hidden
  m_IsActive: 0
--- !u!4 &24
Transform:
  m_GameObject: {fileID: 23}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!33 &25
MeshFilter:
  m_GameObject: {fileID: 23}
  m_Mesh: {fileID: 4300000, guid: quad0000, type: 2}
--- !u!23 &26
MeshRenderer:
  m_GameObject: {fileID: 23}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: bark0mat, type: 2}
";

fn fixture(name: &str) -> (Fixture, Project) {
    let dir = std::env::temp_dir().join(format!("crabby_unity_import_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let assets = dir.join("Assets");
    std::fs::create_dir_all(&assets).unwrap();
    let write = |file: &str, text: &[u8], guid: &str| {
        std::fs::write(assets.join(file), text).unwrap();
        std::fs::write(assets.join(format!("{file}.meta")), format!("guid: {guid}\n")).unwrap();
    };
    write("Quad.asset", include_bytes!("fixtures/quad.asset"), "quad0000");
    write("Bark.mat", MATERIAL.as_bytes(), "bark0mat");
    write("Tree.prefab", PREFAB.as_bytes(), "tree0000");
    write("Lit.shader", b"Shader \"Lit\" {}", "lit00000");
    let png = |pixel: [u8; 4], size: u32| {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(size, size, image::Rgba(pixel))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    };
    write("Bark.png", &png([200, 100, 50, 255], 8), "bark0000");
    // Metalness 1 in red, smoothness 0.8 in alpha.
    write("Gloss.png", &png([255, 0, 0, 204], 4), "gloss000");
    // Textures over the size limit their settings give are made smaller.
    std::fs::write(
        assets.join("Bark.png.meta"),
        "guid: bark0000\nTextureImporter:\n  maxTextureSize: 4\n",
    )
    .unwrap();
    let project = Project::open(&dir).unwrap();
    (Fixture { dir }, project)
}

/// The JSON chunk of a `.glb`.
fn glb_json(bytes: &[u8]) -> Json {
    assert_eq!(&bytes[..4], b"glTF");
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + len]).unwrap()
}

fn import(name: &str) -> (Fixture, Json, Report) {
    let (fixture, project) = fixture(name);
    let out = fixture.dir.join("out");
    let file = UnityFile::read(project.root().join("Assets/Tree.prefab")).unwrap();
    let hierarchy = Hierarchy::build(&file, &project).unwrap();
    let mut importer = Importer::new(&project, &out, "trees");
    let scene = importer.scene(&hierarchy).unwrap();
    let report = importer.finish().unwrap();
    (fixture, scene, report)
}

#[test]
fn drawn_meshes_become_models_placed_in_crabbys_space() {
    let (_f, scene, report) = import("place");
    assert_eq!(scene["format"], "crabby-world");
    let entities = scene["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 1, "the far LOD and the inactive child are left out");
    assert_eq!(report.skipped["a far level of detail (LOD 1+)"], 1);
    assert_eq!(report.skipped["inactive"], 1);
    let t = &entities[0]["components"]["Transform3D"];
    // In Unity the child is at (1, 2, 3) + the root's turn applied to
    // (0, 0, 1) = (2, 2, 3); mirrored along Z that's (2, 2, -3).
    let p: Vec<f64> = t["position"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    for (got, want) in p.iter().zip([2.0, 2.0, -3.0]) {
        assert!((got - want).abs() < 1e-5, "{p:?}");
    }
    // The turn goes the other way: y and w of opposite signs (q and -q
    // are the same rotation).
    let r: Vec<f64> = t["rotation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert!((r[1] * r[3] + 0.5).abs() < 1e-5, "{r:?}");
    assert_eq!(
        entities[0]["components"]["Model"]["mesh"]
            .as_str()
            .unwrap()
            .split('/')
            .next(),
        Some("trees")
    );
    assert_eq!(report.shaders["Assets/Lit.shader"], 1);
}

#[test]
fn meshes_are_mirrored_and_their_materials_converted() {
    let (f, scene, report) = import("mesh");
    assert_eq!((report.meshes, report.textures), (1, 2));
    let name = scene["entities"][0]["components"]["Model"]["mesh"].as_str().unwrap();
    let glb = std::fs::read(f.dir.join("out").join(name.strip_prefix("trees/").unwrap())).unwrap();
    let doc = glb_json(&glb);
    let unity =
        UnityMesh::from_object(&UnityFile::parse(include_str!("fixtures/quad.asset")).unwrap().objects[0]).unwrap();
    // Two submeshes, both drawn with the one material.
    let primitives = doc["meshes"][0]["primitives"].as_array().unwrap();
    assert_eq!(primitives.len(), unity.submeshes.len());
    assert!(primitives.iter().all(|p| p["material"] == 0 || p["material"] == 1));
    let position = &doc["accessors"][primitives[0]["attributes"]["POSITION"].as_u64().unwrap() as usize];
    let unity_min_z = unity.positions.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
    let unity_max_z = unity.positions.iter().map(|p| p[2]).fold(f32::MIN, f32::max);
    assert_eq!(position["min"][2].as_f64().unwrap() as f32, -unity_max_z);
    assert_eq!(position["max"][2].as_f64().unwrap() as f32, -unity_min_z);

    // The material: linear color, cut out, two-sided, with an ORM image.
    let m = &doc["materials"][0];
    let color = m["pbrMetallicRoughness"]["baseColorFactor"].as_array().unwrap();
    assert!(
        (color[1].as_f64().unwrap() - 0.214).abs() < 1e-3,
        "sRGB 0.5 is linear 0.214"
    );
    assert_eq!(m["alphaMode"], "MASK");
    assert!((m["alphaCutoff"].as_f64().unwrap() - 0.3).abs() < 1e-6);
    assert_eq!(m["doubleSided"], true);
    assert_eq!(m["pbrMetallicRoughness"]["metallicFactor"], 1.0);
    let images: Vec<&str> = doc["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["uri"].as_str().unwrap())
        .collect();
    assert_eq!(images[0], "textures/bark0000.png");
    let textures = f.dir.join("out/meshes");
    let orm = image::open(textures.join(images[1])).unwrap().to_rgba8();
    // No occlusion: 1; roughness 1 - 0.8 * 0.5 = 0.6; metalness 1.
    assert_eq!(orm.get_pixel(0, 0).0, [255, 153, 255, 255]);
    let bark = image::open(textures.join(images[0])).unwrap();
    assert_eq!(bark.width(), 4, "made as small as its settings say");
}

#[test]
fn shared_meshes_and_materials_are_written_once() {
    let (fixture, project) = fixture("shared");
    let file = UnityFile::read(project.root().join("Assets/Tree.prefab")).unwrap();
    let hierarchy = Hierarchy::build(&file, &project).unwrap();
    let mut importer = Importer::new(&project, fixture.dir.join("out"), "trees/");
    let a = importer.scene(&hierarchy).unwrap();
    let b = importer.scene(&hierarchy).unwrap();
    assert_eq!(
        a["entities"][0]["components"]["Model"],
        b["entities"][0]["components"]["Model"]
    );
    let report = importer.finish().unwrap();
    assert_eq!((report.entities, report.meshes, report.textures), (2, 1, 2));
}

/// A prefab drawing Unity's built-in Cube and Sphere.
const BUILTINS: &str = "%YAML 1.1
--- !u!1 &1
GameObject:
  m_Component:
  - component: {fileID: 2}
  - component: {fileID: 3}
  - component: {fileID: 4}
  m_Name: Box
  m_IsActive: 1
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children:
  - {fileID: 12}
  m_Father: {fileID: 0}
--- !u!33 &3
MeshFilter:
  m_GameObject: {fileID: 1}
  m_Mesh: {fileID: 10202, guid: 0000000000000000e000000000000000, type: 0}
--- !u!23 &4
MeshRenderer:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: bark0mat, type: 2}
--- !u!1 &11
GameObject:
  m_Component:
  - component: {fileID: 12}
  - component: {fileID: 13}
  - component: {fileID: 14}
  m_Name: Ball
  m_IsActive: 1
--- !u!4 &12
Transform:
  m_GameObject: {fileID: 11}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!33 &13
MeshFilter:
  m_GameObject: {fileID: 11}
  m_Mesh: {fileID: 10207, guid: 0000000000000000e000000000000000, type: 0}
--- !u!23 &14
MeshRenderer:
  m_GameObject: {fileID: 11}
  m_Enabled: 1
  m_Materials:
  - {fileID: 2100000, guid: bark0mat, type: 2}
";

#[test]
fn built_in_cubes_are_made_and_spheres_reported() {
    let (fixture, project) = fixture("builtin");
    let file = UnityFile::parse(BUILTINS).unwrap();
    let hierarchy = Hierarchy::build(&file, &project).unwrap();
    let mut importer = Importer::new(&project, fixture.dir.join("out"), "box/");
    let scene = importer.scene(&hierarchy).unwrap();
    let report = importer.finish().unwrap();
    let entities = scene["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(report.skipped["built-in mesh Sphere"], 1);
    let name = entities[0]["components"]["Model"]["mesh"].as_str().unwrap();
    let glb = std::fs::read(fixture.dir.join("out").join(name.strip_prefix("box/").unwrap())).unwrap();
    let doc = glb_json(&glb);
    let position = &doc["accessors"][doc["meshes"][0]["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .unwrap() as usize];
    assert_eq!(position["count"], 24);
    assert_eq!(position["min"], serde_json::json!([-0.5, -0.5, -0.5]));
}

#[test]
fn custom_shader_materials_are_described_for_the_game() {
    let (fixture, project) = fixture("custom");
    let out = fixture.dir.join("out");
    let file = UnityFile::read(project.root().join("Assets/Tree.prefab")).unwrap();
    let hierarchy = Hierarchy::build(&file, &project).unwrap();
    let mut importer = Importer::new(&project, &out, "trees");
    importer.custom_shader("Assets/Lit.shader");
    let scene = importer.scene(&hierarchy).unwrap();
    importer.finish().unwrap();
    let described: Json = serde_json::from_str(&std::fs::read_to_string(out.join("materials.json")).unwrap()).unwrap();
    let bark = &described["Bark"];
    assert_eq!(bark["shader"], "Assets/Lit.shader");
    assert_eq!(
        bark["floats"]["_Cutoff"].as_f64().map(|f| (f * 10.0).round()),
        Some(3.0)
    );
    assert_eq!(bark["colors"]["_BaseColor"], serde_json::json!([1.0, 0.5, 0.0, 1.0]));
    // Every texture, written where the file says, with its tiling.
    let base = &bark["textures"]["_BaseMap"];
    assert_eq!(base["scale"], serde_json::json!([2.0, 1.0]));
    let file = base["file"].as_str().unwrap();
    assert!(out.join(file.strip_prefix("trees/").unwrap()).is_file(), "{file}");
    assert!(bark["textures"]["_MetallicGlossMap"]["file"].is_string());
    // The glTF material has the name materials.json uses.
    let name = scene["entities"][0]["components"]["Model"]["mesh"].as_str().unwrap();
    let glb = std::fs::read(out.join(name.strip_prefix("trees/").unwrap())).unwrap();
    assert_eq!(glb_json(&glb)["materials"][0]["name"], "Bark");
}

#[test]
fn without_custom_shaders_there_is_no_materials_file() {
    let (f, _, _) = import("plain");
    assert!(!f.dir.join("out/materials.json").exists());
}
