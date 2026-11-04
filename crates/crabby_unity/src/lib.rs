//! Reads Unity projects saved as text (Force Text serialization), for
//! importing them into Crabby Engine: scenes, prefabs, materials and mesh
//! assets as [`UnityFile`]s of [`UnityObject`]s, the GUIDs in `.meta`
//! files as a [`Project`], and mesh vertex data as [`UnityMesh`].
//!
//! ```
//! let text = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &100\nGameObject:\n  m_Name: Lamp\n";
//! let file = crabby_unity::UnityFile::parse(text).unwrap();
//! assert_eq!(file.objects[0].type_name, "GameObject");
//! assert_eq!(file.objects[0].body["m_Name"].as_str(), Some("Lamp"));
//! ```

mod binary;
mod mesh;
mod project;
mod value;

pub use mesh::{Channel, UnityMesh, VertexFormat};
pub use project::Project;
pub use value::{ObjRef, Value};

use std::fmt;

/// What went wrong reading a file, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// What, with the line when known.
    pub message: String,
}

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// One object of a Unity file: a `--- !u!<class> &<file id>` document.
#[derive(Debug, Clone, PartialEq)]
pub struct UnityObject {
    /// Unity's class id: 1 GameObject, 4 Transform, 21 Material, 43 Mesh,
    /// 114 MonoBehaviour, 1001 PrefabInstance, ...
    pub class_id: i32,
    /// Its id within the file, which references to it use.
    pub file_id: i64,
    /// A stand-in for an object of a nested prefab (`stripped`): only
    /// its links to the prefab instance are here.
    pub stripped: bool,
    /// The type's name: the document's one top-level key.
    pub type_name: String,
    /// Its fields.
    pub body: Value,
}

/// A whole Unity text file: its objects in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnityFile {
    /// Every object, in the order the file lists them.
    pub objects: Vec<UnityObject>,
}

impl UnityFile {
    /// Parses a Unity text file (a scene, prefab, material, `.asset`).
    ///
    /// ```
    /// let file = crabby_unity::UnityFile::parse("--- !u!4 &7 stripped\nTransform:\n  m_PrefabInstance: {fileID: 9}\n").unwrap();
    /// assert!(file.objects[0].stripped);
    /// assert_eq!(file.objects[0].body["m_PrefabInstance"].obj_ref().unwrap().file_id, 9);
    /// ```
    ///
    /// # Errors
    ///
    /// If a document header or its YAML is malformed (the message has the line).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut objects = Vec::new();
        // Documents start at lines `--- !u!<class> &<id>[ stripped]`.
        let mut header: Option<(usize, &str)> = None;
        let mut start = 0;
        let mut offset = 0;
        let mut line_no = 0;
        let mut finish = |header: Option<(usize, &str)>, body: &str| -> Result<(), Error> {
            if let Some((line, h)) = header {
                objects.push(object(line, h, body)?);
            }
            Ok(())
        };
        for line in text.split_inclusive('\n') {
            line_no += 1;
            if header.is_none() && !line.starts_with("--- ") && !line.starts_with('%') && !line.trim().is_empty() {
                return Err(Error::new(format!(
                    "line {line_no}: not a Unity text file (expected `%YAML` or `--- !u!` documents)"
                )));
            }
            if line.starts_with("--- ") {
                finish(header, &text[start..offset])?;
                header = Some((line_no, line.trim_end()));
                start = offset + line.len();
            }
            offset += line.len();
        }
        finish(header, &text[start..])?;
        Ok(Self { objects })
    }

    /// Parses a binary serialized asset saved by the Unity editor (Unity
    /// 2022 or later, with type trees), such as terrain data. Objects'
    /// fields are named as in the text format; references to other
    /// files carry their GUID.
    ///
    /// # Errors
    ///
    /// If it isn't one, it's older, it has no type trees, or its data
    /// doesn't match them.
    pub fn parse_binary(data: &[u8]) -> Result<Self, Error> {
        binary::parse(data)
    }

    /// Reads a Unity file, text or binary.
    ///
    /// # Errors
    ///
    /// If it can't be read or parsed (the message names the file).
    pub fn read(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| Error::new(format!("{}: {e}", path.display())))?;
        let parsed = if binary::is_binary(&data) {
            Self::parse_binary(&data)
        } else {
            let text = std::str::from_utf8(&data).map_err(|_| Error::new("neither Unity text nor a binary asset"))?;
            Self::parse(text)
        };
        parsed.map_err(|e| Error::new(format!("{}: {e}", path.display())))
    }

    /// The object with this file id.
    ///
    /// ```
    /// let file = crabby_unity::UnityFile::parse("--- !u!1 &5\nGameObject:\n  m_Name: A\n").unwrap();
    /// assert_eq!(file.get(5).unwrap().type_name, "GameObject");
    /// assert!(file.get(6).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, file_id: i64) -> Option<&UnityObject> {
        self.objects.iter().find(|o| o.file_id == file_id)
    }

    /// Every object of one class (see [`UnityObject::class_id`]).
    ///
    /// ```
    /// let file = crabby_unity::UnityFile::parse("--- !u!1 &5\nGameObject: {}\n--- !u!4 &6\nTransform: {}\n").unwrap();
    /// assert_eq!(file.of_class(4).count(), 1);
    /// ```
    pub fn of_class(&self, class_id: i32) -> impl Iterator<Item = &UnityObject> {
        self.objects.iter().filter(move |o| o.class_id == class_id)
    }
}

/// One document from its header line and body text.
fn object(line: usize, header: &str, body: &str) -> Result<UnityObject, Error> {
    let bad = || Error::new(format!("line {line}: expected `--- !u!<class> &<id>`, got `{header}`"));
    let mut parts = header.split_whitespace().skip(1);
    let class_id = parts
        .next()
        .and_then(|t| t.strip_prefix("!u!"))
        .and_then(|t| t.parse().ok())
        .ok_or_else(bad)?;
    let file_id = parts
        .next()
        .and_then(|t| t.strip_prefix('&'))
        .and_then(|t| t.parse().ok())
        .ok_or_else(bad)?;
    let stripped = parts.next() == Some("stripped");
    let value = value::parse(body).map_err(|e| Error::new(format!("line {}: {}", line + e.line, e.message)))?;
    let Value::Map(mut entries) = value else {
        return Err(Error::new(format!(
            "line {line}: an object's body must be `TypeName:` and its fields"
        )));
    };
    if entries.len() != 1 {
        return Err(Error::new(format!(
            "line {line}: an object has one type name, got {} top-level keys",
            entries.len()
        )));
    }
    let (type_name, body) = entries.pop().expect("one entry");
    Ok(UnityObject {
        class_id,
        file_id,
        stripped,
        type_name,
        body,
    })
}
