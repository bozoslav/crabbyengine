//! Mesh assets (class 43): vertex streams and index buffers stored as hex.

use crate::{Error, UnityObject};

/// How one vertex attribute is stored (Unity's `VertexAttributeFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    /// 32-bit float.
    Float32,
    /// 16-bit float.
    Float16,
    /// 0..255 as 0..1.
    UNorm8,
    /// -127..127 as -1..1.
    SNorm8,
    /// 0..65535 as 0..1.
    UNorm16,
    /// -32767..32767 as -1..1.
    SNorm16,
    /// Whole numbers (bone indices).
    UInt8,
    /// Whole numbers.
    SInt8,
    /// Whole numbers.
    UInt16,
    /// Whole numbers.
    SInt16,
    /// Whole numbers.
    UInt32,
    /// Whole numbers.
    SInt32,
}

impl VertexFormat {
    fn from_unity(code: i64) -> Option<Self> {
        use VertexFormat::*;
        Some(match code {
            0 => Float32,
            1 => Float16,
            2 => UNorm8,
            3 => SNorm8,
            4 => UNorm16,
            5 => SNorm16,
            6 => UInt8,
            7 => SInt8,
            8 => UInt16,
            9 => SInt16,
            10 => UInt32,
            11 => SInt32,
            _ => return None,
        })
    }

    /// Bytes per component.
    #[must_use]
    pub fn size(self) -> usize {
        use VertexFormat::*;
        match self {
            UNorm8 | SNorm8 | UInt8 | SInt8 => 1,
            Float16 | UNorm16 | SNorm16 | UInt16 | SInt16 => 2,
            Float32 | UInt32 | SInt32 => 4,
        }
    }

    fn read(self, b: &[u8]) -> f32 {
        use VertexFormat::*;
        match self {
            Float32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            Float16 => half(u16::from_le_bytes([b[0], b[1]])),
            UNorm8 => f32::from(b[0]) / 255.0,
            SNorm8 => (f32::from(b[0] as i8) / 127.0).max(-1.0),
            UNorm16 => f32::from(u16::from_le_bytes([b[0], b[1]])) / 65535.0,
            SNorm16 => (f32::from(i16::from_le_bytes([b[0], b[1]])) / 32767.0).max(-1.0),
            UInt8 => f32::from(b[0]),
            SInt8 => f32::from(b[0] as i8),
            UInt16 => f32::from(u16::from_le_bytes([b[0], b[1]])),
            SInt16 => f32::from(i16::from_le_bytes([b[0], b[1]])),
            UInt32 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
            SInt32 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32,
        }
    }
}

/// A 16-bit float's value.
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((bits >> 10) & 0x1f);
    let frac = f32::from(bits & 0x3ff);
    sign * match exp {
        0 => frac * 2f32.powi(-24),
        31 if frac == 0.0 => f32::INFINITY,
        31 => f32::NAN,
        _ => (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// Where one attribute is in the vertex data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// Which stream (each stream's vertices are stored one after another).
    pub stream: usize,
    /// Bytes from the start of a vertex in its stream.
    pub offset: usize,
    /// How each component is stored.
    pub format: VertexFormat,
    /// How many components (0: the mesh has no such attribute).
    pub dimension: usize,
}

/// A mesh asset's triangles in Unity's own space (left-handed, Y up): the
/// importer turns them into Crabby's.
///
/// ```
/// # let text = include_str!("../tests/fixtures/quad.asset");
/// let file = crabby_unity::UnityFile::parse(text).unwrap();
/// let mesh = crabby_unity::UnityMesh::from_object(&file.objects[0]).unwrap();
/// assert_eq!(mesh.positions.len(), 5);
/// assert_eq!(mesh.submeshes[0].len(), 6);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnityMesh {
    /// `m_Name`.
    pub name: String,
    /// Every vertex's position.
    pub positions: Vec<[f32; 3]>,
    /// Normals, one per vertex, or none.
    pub normals: Vec<[f32; 3]>,
    /// Tangents (w is the bitangent's sign), or none.
    pub tangents: Vec<[f32; 4]>,
    /// Vertex colors, or none.
    pub colors: Vec<[f32; 4]>,
    /// Texture coordinate sets 0 to 7 (the first two components), each
    /// one per vertex or empty.
    pub uvs: [Vec<[f32; 2]>; 8],
    /// Each submesh's triangle list, as vertex indices (its base vertex
    /// added). Submesh i uses the renderer's material i.
    pub submeshes: Vec<Vec<u32>>,
}

/// Unity's fixed channel order.
const POSITION: usize = 0;
const NORMAL: usize = 1;
const TANGENT: usize = 2;
const COLOR: usize = 3;
const UV0: usize = 4;

impl UnityMesh {
    /// Reads a `Mesh` object (class 43) saved uncompressed.
    ///
    /// # Errors
    ///
    /// If it isn't a mesh, it's compressed, it isn't made of triangles, or
    /// its data is shorter than its layout says.
    pub fn from_object(object: &UnityObject) -> Result<Self, Error> {
        if object.class_id != 43 {
            return Err(Error::new(format!(
                "{} isn't a Mesh (class {})",
                object.type_name, object.class_id
            )));
        }
        let m = &object.body;
        let name = m["m_Name"].as_str().unwrap_or_default().to_owned();
        let fail = |what: &str| Error::new(format!("mesh {name}: {what}"));
        if m["m_MeshCompression"].as_i64().unwrap_or(0) != 0 {
            return Err(fail("compressed meshes aren't supported (turn Mesh Compression off)"));
        }
        let vd = &m["m_VertexData"];
        let count = vd["m_VertexCount"].as_i64().ok_or_else(|| fail("no m_VertexCount"))? as usize;
        let channels: Vec<Channel> = vd["m_Channels"]
            .as_seq()
            .ok_or_else(|| fail("no m_Channels"))?
            .iter()
            .map(|c| {
                Ok(Channel {
                    stream: c["stream"].as_i64().unwrap_or(0) as usize,
                    offset: c["offset"].as_i64().unwrap_or(0) as usize,
                    format: VertexFormat::from_unity(c["format"].as_i64().unwrap_or(0))
                        .ok_or_else(|| fail("unknown vertex format"))?,
                    dimension: (c["dimension"].as_i64().unwrap_or(0) & 0xf) as usize,
                })
            })
            .collect::<Result<_, Error>>()?;
        let data = hex(vd["_typelessdata"].as_str().unwrap_or_default());
        // Each stream: its vertices back to back, starting 16-byte aligned.
        let streams = channels
            .iter()
            .filter(|c| c.dimension > 0)
            .map(|c| c.stream + 1)
            .max()
            .unwrap_or(0);
        let mut strides = vec![0usize; streams];
        for c in channels.iter().filter(|c| c.dimension > 0) {
            strides[c.stream] = strides[c.stream].max(c.offset + c.format.size() * c.dimension);
        }
        let mut starts = vec![0usize; streams];
        let mut at = 0;
        for s in 0..streams {
            starts[s] = at;
            at = (at + strides[s] * count).next_multiple_of(16);
        }
        let read = |ch: usize, n: usize| -> Result<Vec<[f32; 4]>, Error> {
            let Some(c) = channels.get(ch).filter(|c| c.dimension > 0) else {
                return Ok(Vec::new());
            };
            let size = c.format.size();
            (0..count)
                .map(|v| {
                    let base = starts[c.stream] + v * strides[c.stream] + c.offset;
                    let mut out = [0.0, 0.0, 0.0, 1.0];
                    for (k, slot) in out.iter_mut().enumerate().take(c.dimension.min(n)) {
                        let b = data
                            .get(base + k * size..base + (k + 1) * size)
                            .ok_or_else(|| fail("vertex data too short"))?;
                        *slot = c.format.read(b);
                    }
                    Ok(out)
                })
                .collect()
        };
        let xyz = |v: Vec<[f32; 4]>| v.into_iter().map(|p| [p[0], p[1], p[2]]).collect();
        let uvs: [Result<Vec<[f32; 2]>, Error>; 8] =
            std::array::from_fn(|i| read(UV0 + i, 2).map(|v| v.into_iter().map(|p| [p[0], p[1]]).collect()));
        let mut mesh = Self {
            positions: xyz(read(POSITION, 3)?),
            normals: xyz(read(NORMAL, 3)?),
            tangents: read(TANGENT, 4)?,
            colors: read(COLOR, 4)?,
            uvs: Default::default(),
            submeshes: Vec::new(),
            name: name.clone(),
        };
        for (slot, uv) in mesh.uvs.iter_mut().zip(uvs) {
            *slot = uv?;
        }
        // Indices: 16 or 32 bits each.
        let wide = m["m_IndexFormat"].as_i64().unwrap_or(0) == 1;
        let indices = hex(m["m_IndexBuffer"].as_str().unwrap_or_default());
        let index = |i: usize| -> Option<u32> {
            if wide {
                indices
                    .get(i * 4..i * 4 + 4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            } else {
                indices
                    .get(i * 2..i * 2 + 2)
                    .map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])))
            }
        };
        for sub in m["m_SubMeshes"].as_seq().unwrap_or_default() {
            if sub["topology"].as_i64().unwrap_or(0) != 0 {
                return Err(fail("only triangle submeshes are supported"));
            }
            let first = sub["firstByte"].as_i64().unwrap_or(0) as usize / if wide { 4 } else { 2 };
            let n = sub["indexCount"].as_i64().unwrap_or(0) as usize;
            let base = sub["baseVertex"].as_i64().unwrap_or(0) as u32;
            let list = (first..first + n)
                .map(|i| index(i).map(|v| v + base).filter(|&v| (v as usize) < count))
                .collect::<Option<Vec<u32>>>()
                .ok_or_else(|| fail("index buffer too short or out of range"))?;
            mesh.submeshes.push(list);
        }
        Ok(mesh)
    }
}

/// Bytes from hex text: its leading whole hex pairs. Some files carry a
/// stray character after the data, which Unity ignores too; a buffer that
/// comes out short for its layout is still an error where it's read.
fn hex(text: &str) -> Vec<u8> {
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    };
    text.trim()
        .as_bytes()
        .chunks_exact(2)
        .map_while(|p| Some(digit(p[0])? << 4 | digit(p[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_decode() {
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0xc000), -2.0);
        assert_eq!(half(0x3800), 0.5);
        assert_eq!(half(0), 0.0);
        assert!(half(0x7c00).is_infinite());
    }

    #[test]
    fn hex_decodes_its_leading_whole_pairs() {
        assert_eq!(hex("00ff10"), vec![0, 255, 16]);
        assert_eq!(hex("00ff1"), vec![0, 255], "a stray digit is left off");
        assert_eq!(hex("00ffy"), vec![0, 255], "so is a stray letter");
        assert_eq!(hex("00zz11"), vec![0], "decoding stops at the first non-hex pair");
    }
}
