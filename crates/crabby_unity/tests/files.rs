//! Parsing Unity text files: documents, values, references, meshes and
//! project GUIDs. The fixtures are written by hand for these tests.

use crabby_unity::*;

const PREFAB: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Component:
  - component: {fileID: 400}
  - component: {fileID: 2300}
  m_Layer: 8
  m_Name: 'Lamp: brass'
  m_IsActive: 1
--- !u!4 &400
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0.6, z: 0, w: 0.8}
  m_LocalPosition: {x: 1.5, y: -2, z: 3e2}
  m_Children:
  - {fileID: 401}
  m_Father: {fileID: 0}
--- !u!23 &2300
MeshRenderer:
  m_GameObject: {fileID: 100}
  m_Materials:
  - {fileID: 2100000, guid: 6f2a0b9c00000000a000000000000001, type: 2}
  m_LightmapParameters: {fileID: 0}
--- !u!4 &401 stripped
Transform:
  m_CorrespondingSourceObject: {fileID: 9001, guid: 77, type: 3}
  m_PrefabInstance: {fileID: 1001}
--- !u!114 &500
MonoBehaviour:
  m_Script: {fileID: 11500000, guid: abc, type: 3}
  hexish: 0000000001
  note: \"two
    lines\"
  list:
  - 1
  - 2
  empty:
";

#[test]
fn documents_keep_their_class_id_and_stripped_flag() {
    let file = UnityFile::parse(PREFAB).unwrap();
    let ids: Vec<(i32, i64, bool)> = file
        .objects
        .iter()
        .map(|o| (o.class_id, o.file_id, o.stripped))
        .collect();
    assert_eq!(
        ids,
        [
            (1, 100, false),
            (4, 400, false),
            (23, 2300, false),
            (4, 401, true),
            (114, 500, false)
        ]
    );
    assert_eq!(file.get(2300).unwrap().type_name, "MeshRenderer");
    assert_eq!(file.of_class(4).count(), 2);
}

#[test]
fn values_read_as_numbers_vectors_and_references() {
    let file = UnityFile::parse(PREFAB).unwrap();
    let go = &file.get(100).unwrap().body;
    assert_eq!(go["m_Name"].as_str(), Some("Lamp: brass"));
    assert_eq!(go["m_Layer"].as_i64(), Some(8));
    assert_eq!(go["m_IsActive"].as_bool(), Some(true));
    assert_eq!(go["m_Component"][1]["component"].obj_ref().unwrap().file_id, 2300);
    let t = &file.get(400).unwrap().body;
    assert_eq!(t["m_LocalPosition"].vec3(), Some([1.5, -2.0, 300.0]));
    assert_eq!(t["m_LocalRotation"].vec4(), Some([0.0, 0.6, 0.0, 0.8]));
    assert!(t["m_Father"].obj_ref().unwrap().is_none());
    let r = file.get(2300).unwrap().body["m_Materials"][0].obj_ref().unwrap();
    assert_eq!(r.guid.as_deref(), Some("6f2a0b9c00000000a000000000000001"));
    assert_eq!(r.kind, Some(2));
}

#[test]
fn scalars_keep_their_exact_text() {
    let file = UnityFile::parse(PREFAB).unwrap();
    let b = &file.get(500).unwrap().body;
    // Hex blobs of digits must not turn into numbers.
    assert_eq!(b["hexish"].as_str(), Some("0000000001"));
    assert_eq!(b["note"].as_str(), Some("two lines"));
    assert_eq!(b["list"].as_seq().unwrap().len(), 2);
    assert_eq!(b["empty"].as_str(), Some(""));
    assert!(b["missing"].is_null() && b["list"][7].is_null());
}

#[test]
fn bad_headers_say_where() {
    let err = UnityFile::parse("--- !u!1 &100\nGameObject: {}\n--- nonsense\nX: 1\n").unwrap_err();
    assert!(err.message.contains("line 3"), "{err}");
    let err = UnityFile::parse("--- !u!1 &100\nGameObject:\n  a: [1, 2\n").unwrap_err();
    assert!(err.message.starts_with("line "), "{err}");
}

#[test]
fn meshes_decode_streams_and_submeshes() {
    let file = UnityFile::parse(include_str!("fixtures/quad.asset")).unwrap();
    let mesh = UnityMesh::from_object(&file.objects[0]).unwrap();
    assert_eq!(mesh.name, "Quad");
    assert_eq!(mesh.positions[3], [1.0, 0.0, 1.0]);
    assert_eq!(mesh.normals[2], [0.0, 1.0, 0.0]);
    // Stream 1 (half floats) starts 16-byte aligned after stream 0.
    assert_eq!(mesh.uvs[0][3], [1.0, 1.0]);
    assert_eq!(mesh.uvs[0][4], [0.5, 0.5]);
    assert!(mesh.uvs[1].is_empty() && mesh.colors.is_empty() && mesh.tangents.is_empty());
    assert_eq!(
        mesh.submeshes,
        [vec![0, 2, 1, 1, 2, 3], vec![1, 2, 4]],
        "base vertex added"
    );
    // Not a mesh.
    let go = UnityFile::parse(PREFAB).unwrap();
    assert!(UnityMesh::from_object(&go.objects[0]).is_err());
}

#[test]
fn mesh_blend_shapes_keep_their_full_weight_frame() {
    // "Lift" moves vertex 3 up; "Dent" has an in-between frame (vertex 0
    // down by 0.5) and its full frame (vertex 1 down by 1, normal tilted).
    let shapes = "  m_Shapes:
    vertices:
    - vertex: {x: 0, y: 1, z: 0}
      normal: {x: 0, y: 0, z: 0}
      tangent: {x: 0, y: 0, z: 0}
      index: 3
    - vertex: {x: 0, y: -0.5, z: 0}
      normal: {x: 0, y: 0, z: 0}
      tangent: {x: 0, y: 0, z: 0}
      index: 0
    - vertex: {x: 0, y: -1, z: 0}
      normal: {x: 0.5, y: 0, z: 0}
      tangent: {x: 0, y: 0, z: 0}
      index: 1
    shapes:
    - firstVertex: 0
      vertexCount: 1
      hasNormals: 0
      hasTangents: 0
    - firstVertex: 1
      vertexCount: 1
      hasNormals: 1
      hasTangents: 0
    - firstVertex: 2
      vertexCount: 1
      hasNormals: 1
      hasTangents: 0
    channels:
    - name: Lift
      nameHash: 1
      frameIndex: 0
      frameCount: 1
    - name: Dent
      nameHash: 2
      frameIndex: 1
      frameCount: 2
    fullWeights:
    - 100
    - 50
    - 100
";
    // Windows checkouts may end the fixture's lines with \r\n.
    let text = include_str!("fixtures/quad.asset")
        .replace("\r\n", "\n")
        .replace("  m_MeshCompression: 0\n", &format!("{shapes}  m_MeshCompression: 0\n"));
    let file = UnityFile::parse(&text).unwrap();
    let mesh = UnityMesh::from_object(&file.objects[0]).unwrap();
    let [lift, dent] = &mesh.blend_shapes[..] else {
        panic!("{:?}", mesh.blend_shapes);
    };
    assert_eq!(lift.name, "Lift");
    assert_eq!(
        lift.positions,
        [[0.0; 3], [0.0; 3], [0.0; 3], [0.0, 1.0, 0.0], [0.0; 3]]
    );
    assert!(lift.normals.is_empty());
    assert_eq!(dent.name, "Dent");
    assert_eq!(dent.positions[0], [0.0; 3], "the in-between frame is left out");
    assert_eq!(dent.positions[1], [0.0, -1.0, 0.0]);
    assert_eq!(dent.normals[1], [0.5, 0.0, 0.0]);
    assert_eq!(dent.normals.len(), 5);
    // Meshes without shapes have none.
    let plain = UnityFile::parse(include_str!("fixtures/quad.asset")).unwrap();
    assert!(
        UnityMesh::from_object(&plain.objects[0])
            .unwrap()
            .blend_shapes
            .is_empty()
    );
}

#[test]
fn projects_map_guids_to_files() {
    let dir = std::env::temp_dir().join(format!("crabby_unity_project_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Assets/Props")).unwrap();
    std::fs::create_dir_all(dir.join("Library/PackageCache/com.example/Shaders")).unwrap();
    std::fs::write(dir.join("Assets/Props/Lamp.prefab"), PREFAB).unwrap();
    std::fs::write(
        dir.join("Assets/Props/Lamp.prefab.meta"),
        "fileFormatVersion: 2\nguid: aa11\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Assets/Props.meta"),
        "fileFormatVersion: 2\nguid: ff00\nfolderAsset: yes\n",
    )
    .unwrap();
    let shader = dir.join("Library/PackageCache/com.example/Shaders/Lit.shader");
    std::fs::write(&shader, "Shader \"Lit\" {}").unwrap();
    std::fs::write(shader.with_extension("shader.meta"), "guid: bb22\n").unwrap();
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.len(), 3);
    assert_eq!(project.path_of("bb22"), Some(shader.as_path()));
    let lamp = project.load("aa11").unwrap();
    assert_eq!(lamp.objects.len(), 5);
    assert!(project.load("bb22").is_err(), "a shader isn't Unity YAML");
    assert!(project.load("nope").is_err());
    assert!(Project::open(dir.join("Assets")).is_err(), "no Assets folder inside");
    std::fs::remove_dir_all(&dir).unwrap();
}
