//! Unity's binary serialized files (assets saved by the editor in binary,
//! such as terrain data): decoded with the type trees they carry.

use crate::{Error, UnityFile, UnityObject, Value};

/// Unity's built-in strings, which type trees refer to by offset (high
/// bit set) instead of storing them: the editor's table, in its order.
const COMMON: &str = "AABB\0AnimationClip\0AnimationCurve\0AnimationState\0Array\0Base\0BitField\0bitset\0bool\0char\0ColorRGBA\0Component\0data\0deque\0double\0dynamic_array\0FastPropertyName\0first\0float\0Font\0GameObject\0Generic Mono\0GradientNEW\0GUID\0GUIStyle\0int\0list\0long long\0map\0Matrix4x4f\0MdFour\0MonoBehaviour\0MonoScript\0m_ByteSize\0m_Curve\0m_EditorClassIdentifier\0m_EditorHideFlags\0m_Enabled\0m_ExtensionPtr\0m_GameObject\0m_Index\0m_IsArray\0m_IsStatic\0m_MetaFlag\0m_Name\0m_ObjectHideFlags\0m_PrefabInternal\0m_PrefabParentObject\0m_Script\0m_StaticEditorFlags\0m_Type\0m_Version\0Object\0pair\0PPtr<Component>\0PPtr<GameObject>\0PPtr<Material>\0PPtr<MonoBehaviour>\0PPtr<MonoScript>\0PPtr<Object>\0PPtr<Prefab>\0PPtr<Sprite>\0PPtr<TextAsset>\0PPtr<Texture>\0PPtr<Texture2D>\0PPtr<Transform>\0Prefab\0Quaternionf\0Rectf\0RectInt\0RectOffset\0second\0set\0short\0size\0SInt16\0SInt32\0SInt64\0SInt8\0staticvector\0string\0TextAsset\0TextMesh\0Texture\0Texture2D\0Transform\0TypelessData\0UInt16\0UInt32\0UInt64\0UInt8\0unsigned int\0unsigned long long\0unsigned short\0vector\0Vector2f\0Vector3f\0Vector4f\0m_ScriptingClassIdentifier\0Gradient\0Type*\0int2_storage\0int3_storage\0BoundsInt\0m_CorrespondingSourceObject\0m_PrefabInstance\0m_PrefabAsset\0FileSize\0Hash128\0RenderingLayerMask\0fixed_array\0EntityId\0LoadableObjectId\0LoadableSceneId\0";

/// Unity 6's wrapper around each type tree: `mhtt`.
const TREE_MAGIC: u32 = u32::from_le_bytes(*b"mhtt");

/// The node flag saying the data after it is padded to 4 bytes.
const ALIGN: i32 = 0x4000;

struct Node {
    level: u8,
    array: bool,
    ty: String,
    name: String,
    flags: i32,
}

struct Type {
    class_id: i32,
    nodes: Vec<Node>,
}

/// A little reader over the file's bytes.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    big: bool,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len());
        let end = end.ok_or_else(|| Error::new(format!("binary asset: data ends early (at byte {})", self.at)))?;
        let b = &self.data[self.at..end];
        self.at = end;
        Ok(b)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut a: [u8; N] = self.bytes(N)?.try_into().expect("N bytes");
        if self.big {
            a.reverse();
        }
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn i16(&mut self) -> Result<i16, Error> {
        Ok(i16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.array()?))
    }
    fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    fn cstring(&mut self) -> Result<String, Error> {
        let rest = &self.data[self.at.min(self.data.len())..];
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| Error::new("binary asset: unterminated string"))?;
        let s = String::from_utf8_lossy(&rest[..end]).into_owned();
        self.at += end + 1;
        Ok(s)
    }

    fn align(&mut self) {
        self.at = self.at.next_multiple_of(4);
    }
}

/// Whether `data` looks like a binary serialized file.
pub(crate) fn is_binary(data: &[u8]) -> bool {
    data.len() >= 48 && u32::from_be_bytes([data[8], data[9], data[10], data[11]]) >= 9 && data[..4] != *b"%YAM"
}

/// Parses a binary serialized file (format 22 or later, with type trees).
pub(crate) fn parse(data: &[u8]) -> Result<UnityFile, Error> {
    let mut r = Reader { data, at: 0, big: true };
    let _legacy_meta = r.u32()?;
    let _legacy_size = r.u32()?;
    let version = r.u32()?;
    let _legacy_offset = r.u32()?;
    if version < 22 {
        return Err(Error::new(format!(
            "binary asset format {version} is older than Unity 2022 (22); resave it"
        )));
    }
    let big_endian = r.u8()? != 0;
    r.bytes(3)?;
    let _meta_size = r.u32()?;
    let _file_size = r.u64()?;
    let data_offset = r.u64()? as usize;
    let _unknown = r.u64()?;
    r.big = big_endian;

    let _unity_version = r.cstring()?;
    let _platform = r.i32()?;
    if r.u8()? == 0 {
        return Err(Error::new(
            "binary asset has no type trees (only editor-saved assets are supported)",
        ));
    }
    let type_count = r.i32()?;
    let mut types = Vec::new();
    for _ in 0..type_count {
        types.push(read_type(&mut r)?);
    }
    let object_count = r.i32()?;
    let mut entries = Vec::new();
    for _ in 0..object_count {
        r.align();
        let path_id = r.i64()?;
        let start = r.i64()? as usize;
        let size = r.u32()? as usize;
        let type_index = r.i32()? as usize;
        entries.push((path_id, start, size, type_index));
    }
    // Scripts, then the files that references point into.
    let scripts = r.i32()?;
    for _ in 0..scripts {
        let _file = r.i32()?;
        r.align();
        let _id = r.i64()?;
    }
    let externals = r.i32()?;
    let mut guids = Vec::new();
    for _ in 0..externals {
        let _empty = r.cstring()?;
        let guid = r.bytes(16)?;
        let _kind = r.i32()?;
        let _path = r.cstring()?;
        guids.push(guid_text(guid));
    }

    let mut objects = Vec::new();
    for (path_id, start, size, type_index) in entries {
        let ty = types
            .get(type_index)
            .ok_or_else(|| Error::new(format!("binary asset: object {path_id} has no type {type_index}")))?;
        let begin = data_offset + start;
        let bytes = data
            .get(begin..begin + size)
            .ok_or_else(|| Error::new(format!("binary asset: object {path_id} lies past the end")))?;
        let mut o = Reader {
            data: bytes,
            at: 0,
            big: big_endian,
        };
        let mut i = 0;
        let body = read_node(&mut o, &ty.nodes, &mut i, &guids)?;
        if o.at != bytes.len() {
            return Err(Error::new(format!(
                "binary asset: object {path_id} ({}) read {} of its {} bytes",
                ty.nodes[0].ty,
                o.at,
                bytes.len()
            )));
        }
        objects.push(UnityObject {
            class_id: ty.class_id,
            file_id: path_id,
            stripped: false,
            type_name: ty.nodes[0].ty.clone(),
            body,
        });
    }
    Ok(UnityFile { objects })
}

/// Unity writes a GUID's 16 bytes as text with each byte's two hex
/// digits swapped.
fn guid_text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:x}{:x}", b & 0xf, b >> 4)).collect()
}

fn read_type(r: &mut Reader) -> Result<Type, Error> {
    let class_id = r.i32()?;
    let _stripped = r.u8()?;
    let _script_index = r.i16()?;
    // Hashes: the script's id (scripts only before Unity 6, every type
    // since) and the type's. Then, since Unity 6: the tree's size,
    // `mhtt` and a version; before: the tree itself.
    let magic_after = |skip: usize| {
        let at = r.at + skip + 4;
        r.data
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            == Some(TREE_MAGIC)
    };
    if magic_after(16) {
        r.bytes(16 + 12)?;
    } else if magic_after(32) {
        r.bytes(32 + 12)?;
    } else {
        r.bytes(if class_id == 114 { 32 } else { 16 })?;
    }
    let count = r.i32()? as usize;
    let strings_len = r.i32()? as usize;
    let mut raw = Vec::with_capacity(count);
    for _ in 0..count {
        let _version = r.u16()?;
        let level = r.u8()?;
        let type_flags = r.u8()?;
        let ty = r.u32()?;
        let name = r.u32()?;
        let size = r.i32()?;
        let _index = r.i32()?;
        let flags = r.i32()?;
        let _ref_hash = r.u64()?;
        raw.push((level, type_flags, ty, name, size, flags));
    }
    let strings = r.bytes(strings_len)?;
    let text = |offset: u32| -> String {
        let (table, at) = if offset & 0x8000_0000 != 0 {
            (COMMON.as_bytes(), (offset & 0x7fff_ffff) as usize)
        } else {
            (strings, offset as usize)
        };
        let rest = table.get(at..).unwrap_or_default();
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        String::from_utf8_lossy(&rest[..end]).into_owned()
    };
    let nodes = raw
        .into_iter()
        .map(|(level, type_flags, ty, name, _size, flags)| Node {
            level,
            array: type_flags & 1 != 0,
            ty: text(ty),
            name: text(name),
            flags,
        })
        .collect();
    let dependencies = r.i32()?;
    for _ in 0..dependencies {
        r.i32()?;
    }
    Ok(Type { class_id, nodes })
}

/// The range of `nodes` that is node `i`'s subtree (i itself first).
fn subtree_end(nodes: &[Node], i: usize) -> usize {
    let level = nodes[i].level;
    let mut j = i + 1;
    while j < nodes.len() && nodes[j].level > level {
        j += 1;
    }
    j
}

/// Reads node `*i` (and its children), leaving `*i` after its subtree.
fn read_node(r: &mut Reader, nodes: &[Node], i: &mut usize, guids: &[String]) -> Result<Value, Error> {
    let node = &nodes[*i];
    let end = subtree_end(nodes, *i);
    let value = if let Some(v) = primitive(r, &node.ty)? {
        *i += 1;
        v
    } else if node.ty == "string" {
        let n = r.i32()? as usize;
        let s = String::from_utf8_lossy(r.bytes(n)?).into_owned();
        // A string's array child carries the padding flag.
        if nodes.get(*i + 1).is_some_and(|c| c.flags & ALIGN != 0) {
            r.align();
        }
        *i = end;
        Value::Scalar(s)
    } else if node.ty == "TypelessData" {
        let n = r.i32()? as usize;
        let b = r.bytes(n)?.to_vec();
        *i = end;
        Value::Bytes(b)
    } else if node.array {
        // `Array`: its size, then that many of its data child.
        let element = *i + 2;
        let n = r.i32()? as usize;
        let value = read_array(r, nodes, element, n, guids)?;
        *i = end;
        value
    } else if node.ty.starts_with("PPtr<") {
        *i += 1;
        let mut file = 0;
        let mut path = 0;
        while *i < end {
            let child = &nodes[*i];
            let v = read_node(r, nodes, i, guids)?;
            match child.name.as_str() {
                "m_FileID" => file = v.as_i64().unwrap_or(0),
                "m_PathID" => path = v.as_i64().unwrap_or(0),
                _ => {}
            }
        }
        let mut entries = vec![("fileID".to_owned(), Value::Scalar(path.to_string()))];
        if file > 0
            && let Some(g) = guids.get(file as usize - 1)
        {
            entries.push(("guid".to_owned(), Value::Scalar(g.clone())));
        }
        Value::Map(entries)
    } else {
        // A struct: its children by name. A child that's an array makes
        // the struct a container (vector, map, ...): it stands for it.
        *i += 1;
        let mut entries = Vec::new();
        while *i < end {
            let name = nodes[*i].name.clone();
            entries.push((name, read_node(r, nodes, i, guids)?));
        }
        if let [(name, v)] = entries.as_slice()
            && name == "Array"
        {
            v.clone()
        } else {
            Value::Map(entries)
        }
    };
    if node.flags & ALIGN != 0 {
        r.align();
    }
    Ok(value)
}

/// `n` elements of node `element`'s type.
fn read_array(r: &mut Reader, nodes: &[Node], element: usize, n: usize, guids: &[String]) -> Result<Value, Error> {
    let el = nodes
        .get(element)
        .ok_or_else(|| Error::new("binary asset: an array without an element type"))?;
    // Numbers come out as one compact list.
    let simple = subtree_end(nodes, element) == element + 1 && el.flags & ALIGN == 0;
    if simple {
        let mut numbers = Vec::with_capacity(n.min(1 << 24));
        let wide = matches!(
            el.ty.as_str(),
            "SInt64" | "UInt64" | "long long" | "unsigned long long" | "FileSize"
        );
        if !wide {
            for _ in 0..n {
                match primitive(r, &el.ty)? {
                    Some(Value::Scalar(s)) => numbers.push(s.parse::<f64>().unwrap_or(0.0)),
                    _ => {
                        return Err(Error::new(format!(
                            "binary asset: an array of {} isn't supported",
                            el.ty
                        )));
                    }
                }
            }
            return Ok(Value::Numbers(numbers));
        }
    }
    let mut items = Vec::with_capacity(n.min(1 << 20));
    for _ in 0..n {
        let mut i = element;
        items.push(read_node(r, nodes, &mut i, guids)?);
    }
    Ok(Value::Seq(items))
}

/// A primitive type's value, or `None` for other types.
fn primitive(r: &mut Reader, ty: &str) -> Result<Option<Value>, Error> {
    let text = match ty {
        "bool" => if r.u8()? != 0 { "1" } else { "0" }.to_owned(),
        "SInt8" => (r.u8()? as i8).to_string(),
        "UInt8" | "char" => r.u8()?.to_string(),
        "SInt16" | "short" => r.i16()?.to_string(),
        "UInt16" | "unsigned short" => r.u16()?.to_string(),
        "SInt32" | "int" | "Type*" => r.i32()?.to_string(),
        "UInt32" | "unsigned int" => r.u32()?.to_string(),
        "SInt64" | "long long" | "FileSize" => r.i64()?.to_string(),
        "UInt64" | "unsigned long long" => r.u64()?.to_string(),
        "float" => r.f32()?.to_string(),
        "double" => r.f64()?.to_string(),
        _ => return Ok(None),
    };
    Ok(Some(Value::Scalar(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guids_swap_each_bytes_digits() {
        assert_eq!(guid_text(&[0x12, 0xab]), "21ba");
    }

    #[test]
    fn common_strings_start_where_type_trees_expect() {
        // Offsets type trees use for these.
        let at = |s: &str| COMMON.find(&format!("{s}\0")).unwrap();
        assert_eq!(at("AABB"), 0);
        assert_eq!(at("Array"), 49);
        assert_eq!(at("Base"), 55);
        assert_eq!(COMMON.matches('\0').count(), 114);
    }
}
