//! Binary serialized assets: a small Unity 6 style file built here, byte
//! by byte, the way the editor lays one out.

use ponko_unity::*;

/// Offsets into Unity's built-in strings (high bit set).
const fn common(offset: u32) -> u32 {
    0x8000_0000 | offset
}
const ARRAY: u32 = common(49);
const BASE: u32 = common(55);
const INT: u32 = common(222);
const M_NAME: u32 = common(427);
const SINT16: u32 = common(800);
const SINT64: u32 = common(814);
const STRING: u32 = common(840);
const SIZE: u32 = common(795);
const DATA: u32 = common(106);
const VECTOR: u32 = common(981);
const CHAR: u32 = common(81);

/// One type tree node: (level, is array, type, name, size, flags).
type Node = (u8, bool, u32, u32, i32, i32);

struct Strings(Vec<u8>);

impl Strings {
    fn add(&mut self, s: &str) -> u32 {
        let at = self.0.len() as u32;
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
        at
    }
}

fn file() -> Vec<u8> {
    let mut local = Strings(Vec::new());
    let test_object = local.add("TestObject");
    let count = local.add("m_Count");
    let heights = local.add("m_Heights");
    let pptr = local.add("PPtr<Object>");
    let link = local.add("m_Link");
    let file_id = local.add("m_FileID");
    let path_id = local.add("m_PathID");
    const ALIGN: i32 = 0x4000;
    let nodes: Vec<Node> = vec![
        (0, false, test_object, BASE, -1, 0),
        (1, false, STRING, M_NAME, -1, 0),
        (2, true, ARRAY, ARRAY, -1, ALIGN),
        (3, false, INT, SIZE, 4, 0),
        (3, false, CHAR, DATA, 1, 0),
        (1, false, INT, count, 4, 0),
        (1, false, VECTOR, heights, -1, 0),
        (2, true, ARRAY, ARRAY, -1, ALIGN),
        (3, false, INT, SIZE, 4, 0),
        (3, false, SINT16, DATA, 2, 0),
        (1, false, pptr, link, 12, 0),
        (2, false, INT, file_id, 4, 0),
        (2, false, SINT64, path_id, 8, 0),
    ];
    let mut tree = Vec::new();
    tree.extend((nodes.len() as i32).to_le_bytes());
    tree.extend((local.0.len() as i32).to_le_bytes());
    for (level, array, ty, name, size, flags) in &nodes {
        tree.extend(1u16.to_le_bytes());
        tree.push(*level);
        tree.push(u8::from(*array));
        tree.extend(ty.to_le_bytes());
        tree.extend(name.to_le_bytes());
        tree.extend(size.to_le_bytes());
        tree.extend(0i32.to_le_bytes());
        tree.extend(flags.to_le_bytes());
        tree.extend(0u64.to_le_bytes());
    }
    tree.extend(&local.0);

    // The object: "Rock" (padded), 7, three heights (padded), a link
    // into external file 1, object 42.
    let mut object = Vec::new();
    object.extend(4i32.to_le_bytes());
    object.extend(b"Rock");
    object.extend(7i32.to_le_bytes());
    object.extend(3i32.to_le_bytes());
    for h in [100i16, -5, 32000] {
        object.extend(h.to_le_bytes());
    }
    object.extend([0, 0]); // to 4 bytes
    object.extend(1i32.to_le_bytes());
    object.extend(42i64.to_le_bytes());

    let mut meta = Vec::new();
    meta.extend(b"6000.0.0f1\0");
    meta.extend((-2i32).to_le_bytes());
    meta.push(1); // type trees
    meta.extend(1i32.to_le_bytes()); // one type
    meta.extend(999i32.to_le_bytes());
    meta.push(0);
    meta.extend((-1i16).to_le_bytes());
    meta.extend([0u8; 32]); // the hashes
    meta.extend(((tree.len() + 8) as u32).to_le_bytes());
    meta.extend(b"mhtt");
    meta.extend(23u32.to_le_bytes());
    meta.extend(&tree);
    meta.extend(0i32.to_le_bytes()); // dependencies
    meta.extend(1i32.to_le_bytes()); // one object
    while (48 + meta.len()) % 4 != 0 {
        meta.push(0);
    }
    meta.extend(77i64.to_le_bytes()); // path id
    meta.extend(0i64.to_le_bytes()); // start
    meta.extend((object.len() as u32).to_le_bytes());
    meta.extend(0i32.to_le_bytes()); // type 0
    meta.extend(0i32.to_le_bytes()); // scripts
    meta.extend(1i32.to_le_bytes()); // externals
    meta.push(0);
    meta.extend([0x12, 0xab, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]);
    meta.extend(2i32.to_le_bytes());
    meta.extend(b"Assets/Other.asset\0");
    meta.push(0); // user information

    let data_offset = (48 + meta.len()).next_multiple_of(16);
    let mut out = Vec::new();
    out.extend(0u32.to_be_bytes());
    out.extend(0u32.to_be_bytes());
    out.extend(23u32.to_be_bytes());
    out.extend(0u32.to_be_bytes());
    out.push(0); // little endian
    out.extend([0, 0, 0]);
    out.extend((meta.len() as u32).to_be_bytes());
    out.extend(((data_offset + object.len()) as u64).to_be_bytes());
    out.extend((data_offset as u64).to_be_bytes());
    out.extend(0u64.to_be_bytes());
    out.extend(&meta);
    out.resize(data_offset, 0);
    out.extend(&object);
    out
}

#[test]
fn binary_objects_decode_through_their_type_tree() {
    let parsed = UnityFile::parse_binary(&file()).unwrap();
    let o = &parsed.objects[0];
    assert_eq!((o.class_id, o.file_id, o.type_name.as_str()), (999, 77, "TestObject"));
    assert_eq!(o.body["m_Name"].as_str(), Some("Rock"));
    assert_eq!(o.body["m_Count"].as_i64(), Some(7));
    assert_eq!(o.body["m_Heights"].as_numbers(), Some(&[100.0, -5.0, 32000.0][..]));
    let link = o.body["m_Link"].obj_ref().unwrap();
    assert_eq!(link.file_id, 42);
    assert_eq!(
        link.guid.as_deref(),
        Some("21ba0000000000000000000000000010"),
        "digits swapped per byte"
    );
}

#[test]
fn files_are_read_as_text_or_binary() {
    let dir = std::env::temp_dir().join(format!("crabby_unity_binary_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.asset"), file()).unwrap();
    std::fs::write(
        dir.join("b.asset"),
        "%YAML 1.1\n--- !u!1 &1\nGameObject:\n  m_Name: B\n",
    )
    .unwrap();
    assert_eq!(
        UnityFile::read(dir.join("a.asset")).unwrap().objects[0].type_name,
        "TestObject"
    );
    assert_eq!(
        UnityFile::read(dir.join("b.asset")).unwrap().objects[0].type_name,
        "GameObject"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn truncated_binary_files_are_errors_not_panics() {
    let whole = file();
    for cut in [60, whole.len() / 2, whole.len() - 3] {
        assert!(UnityFile::parse_binary(&whole[..cut]).is_err(), "cut at {cut}");
    }
}
