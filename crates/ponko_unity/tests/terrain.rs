use ponko_unity::{UnityFile, UnityObject, UnityTerrain, Value};

/// A `TerrainData` as binary assets decode it: heights as numbers.
fn terrain(resolution: i64, heights: Vec<f64>, scale: [f32; 3]) -> UnityObject {
    let s = |v: String| Value::Scalar(v);
    let map = |m: Vec<(&str, Value)>| Value::Map(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
    let obj_ref = |id: i64| map(vec![("fileID", s(id.to_string()))]);
    UnityObject {
        class_id: 156,
        file_id: 15600000,
        stripped: false,
        type_name: "TerrainData".into(),
        body: map(vec![
            ("m_Name", s("Tile".into())),
            (
                "m_SplatDatabase",
                map(vec![
                    (
                        "m_TerrainLayers",
                        Value::Seq(vec![
                            map(vec![("fileID", s("11400000".into())), ("guid", s("abc".into()))]),
                            map(vec![("fileID", s("11400000".into())), ("guid", s("def".into()))]),
                        ]),
                    ),
                    ("m_AlphaTextures", Value::Seq(vec![obj_ref(-91)])),
                ]),
            ),
            (
                "m_Heightmap",
                map(vec![
                    ("m_Heights", Value::Numbers(heights)),
                    ("m_Resolution", s(resolution.to_string())),
                    (
                        "m_Scale",
                        map(vec![
                            ("x", s(scale[0].to_string())),
                            ("y", s(scale[1].to_string())),
                            ("z", s(scale[2].to_string())),
                        ]),
                    ),
                ]),
            ),
        ]),
    }
}

#[test]
fn heights_are_world_units_with_unity_far_row_first() {
    // 3 x 3, Unity rows from z = 0: 0s, then half height, then full.
    let h = 32766.0;
    let raw = vec![0.0, 0.0, 0.0, h / 2.0, h / 2.0, h / 2.0, h, h, 0.0];
    let t = UnityTerrain::from_object(&terrain(3, raw, [2.0, 100.0, 2.0])).unwrap();
    assert_eq!((t.resolution, t.size, t.height), (3, 4.0, 100.0));
    assert_eq!(t.heights, [100.0, 100.0, 0.0, 50.0, 50.0, 50.0, 0.0, 0.0, 0.0]);
}

#[test]
fn neighbours_along_unity_z_stay_neighbours() {
    // Unity tiles at z = 0 and z = 4 (4 wide): in Ponko the second one is
    // at -8..-4 and the first at -4..0, sharing the edge at -4.
    let t = UnityTerrain::from_object(&terrain(3, vec![0.0; 9], [2.0, 1.0, 2.0])).unwrap();
    assert_eq!(t.corner([0.0, 0.0, 0.0]), [0.0, 0.0, -4.0]);
    assert_eq!(t.corner([0.0, 0.0, 4.0]), [0.0, 0.0, -8.0]);
    assert_eq!(t.corner([4.0, 3.0, 0.0]), [4.0, 3.0, -4.0], "X and height stay");
}

#[test]
fn painted_layers_and_their_weight_maps_are_listed() {
    let t = UnityTerrain::from_object(&terrain(2, vec![0.0; 4], [1.0, 1.0, 1.0])).unwrap();
    let guids: Vec<_> = t.layers.iter().map(|r| r.guid.as_deref().unwrap()).collect();
    assert_eq!(guids, ["abc", "def"]);
    assert_eq!(t.alphamaps.len(), 1);
    assert_eq!(t.alphamaps[0].file_id, -91);
}

#[test]
fn text_terrain_reads_like_binary() {
    let text = "%YAML 1.1\n--- !u!156 &1\nTerrainData:\n  m_Name: T\n  m_Heightmap:\n    m_Resolution: 2\n    m_Scale: {x: 1, y: 10, z: 1}\n    m_Heights: [32766, 0, 0, 0]\n";
    let file = UnityFile::parse(text).unwrap();
    let t = UnityTerrain::from_object(&file.objects[0]).unwrap();
    assert_eq!(t.heights, [0.0, 0.0, 10.0, 0.0]);
    assert!(t.layers.is_empty() && t.alphamaps.is_empty());
}

#[test]
fn bad_terrain_is_an_error() {
    let short = UnityTerrain::from_object(&terrain(3, vec![0.0; 8], [1.0, 1.0, 1.0])).unwrap_err();
    assert!(short.to_string().contains("8 heights for a resolution of 3"), "{short}");
    let stretched = UnityTerrain::from_object(&terrain(2, vec![0.0; 4], [1.0, 1.0, 2.0])).unwrap_err();
    assert!(stretched.to_string().contains("square"), "{stretched}");
    let mut mesh = terrain(2, vec![0.0; 4], [1.0, 1.0, 1.0]);
    mesh.class_id = 43;
    mesh.type_name = "Mesh".into();
    assert!(
        UnityTerrain::from_object(&mesh)
            .unwrap_err()
            .to_string()
            .contains("not TerrainData")
    );
}
