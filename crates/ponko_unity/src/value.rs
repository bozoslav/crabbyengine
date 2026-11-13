//! A field of a Unity object: scalars kept as their text (so hex blobs and
//! long ids come through untouched), maps in file order, and lists.

use std::ops::Index;

use yaml_rust2::parser::{Event, Parser};

/// A field's value.
///
/// ```
/// let file = ponko_unity::UnityFile::parse("--- !u!4 &1\nTransform:\n  m_LocalPosition: {x: 1, y: 2.5, z: -3}\n  m_Children: []\n").unwrap();
/// let t = &file.objects[0].body;
/// assert_eq!(t["m_LocalPosition"].vec3(), Some([1.0, 2.5, -3.0]));
/// assert_eq!(t["m_Children"].as_seq().map(<[_]>::len), Some(0));
/// assert!(t["m_Missing"].is_null());
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Absent (a missing key reads as this).
    #[default]
    Null,
    /// A scalar's text, exactly as written (quotes removed).
    Scalar(String),
    /// Keys and values in file order.
    Map(Vec<(String, Value)>),
    /// A list.
    Seq(Vec<Value>),
    /// Raw bytes (binary assets' `TypelessData`, such as image data).
    Bytes(Vec<u8>),
    /// A list of numbers (binary assets' arrays of numbers, such as
    /// heights), kept compact.
    Numbers(Vec<f64>),
}

static NULL: Value = Value::Null;

impl Index<&str> for Value {
    type Output = Value;

    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&NULL)
    }
}

impl Index<usize> for Value {
    type Output = Value;

    fn index(&self, i: usize) -> &Value {
        self.as_seq().and_then(|s| s.get(i)).unwrap_or(&NULL)
    }
}

impl Value {
    /// The value under `key`, if this is a map that has it.
    ///
    /// ```
    /// let v = ponko_unity::UnityFile::parse("--- !u!1 &1\nGameObject:\n  m_Layer: 8\n").unwrap();
    /// assert!(v.objects[0].body.get("m_Layer").is_some());
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Whether it's absent.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The text of a scalar.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// A scalar as a whole number.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_str()?.trim().parse().ok()
    }

    /// A scalar as a number.
    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        self.as_str()?.trim().parse().ok()
    }

    /// A 0/1 flag (Unity writes booleans as numbers).
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        Some(self.as_i64()? != 0)
    }

    /// A map's entries.
    #[must_use]
    pub fn as_map(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    /// A list's items.
    #[must_use]
    pub fn as_seq(&self) -> Option<&[Value]> {
        match self {
            Value::Seq(s) => Some(s),
            _ => None,
        }
    }

    /// Raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// A compact list of numbers (see [`Value::Numbers`]).
    #[must_use]
    pub fn as_numbers(&self) -> Option<&[f64]> {
        match self {
            Value::Numbers(n) => Some(n),
            _ => None,
        }
    }

    /// `{x, y}` as numbers.
    #[must_use]
    pub fn vec2(&self) -> Option<[f32; 2]> {
        Some([self["x"].as_f32()?, self["y"].as_f32()?])
    }

    /// `{x, y, z}` as numbers.
    #[must_use]
    pub fn vec3(&self) -> Option<[f32; 3]> {
        Some([self["x"].as_f32()?, self["y"].as_f32()?, self["z"].as_f32()?])
    }

    /// `{x, y, z, w}` as numbers (a rotation, a color as `{r, g, b, a}`
    /// reads with [`Value::color`]).
    #[must_use]
    pub fn vec4(&self) -> Option<[f32; 4]> {
        Some([
            self["x"].as_f32()?,
            self["y"].as_f32()?,
            self["z"].as_f32()?,
            self["w"].as_f32()?,
        ])
    }

    /// `{r, g, b, a}` as numbers.
    #[must_use]
    pub fn color(&self) -> Option<[f32; 4]> {
        Some([
            self["r"].as_f32()?,
            self["g"].as_f32()?,
            self["b"].as_f32()?,
            self["a"].as_f32()?,
        ])
    }

    /// A reference `{fileID: .., guid: .., type: ..}`; `None` when it isn't
    /// one. A file id of 0 is a reference to nothing.
    ///
    /// ```
    /// let f = ponko_unity::UnityFile::parse("--- !u!23 &1\nMeshRenderer:\n  m_Materials:\n  - {fileID: 2100000, guid: 0123abc, type: 2}\n").unwrap();
    /// let r = f.objects[0].body["m_Materials"][0].obj_ref().unwrap();
    /// assert_eq!((r.file_id, r.guid.as_deref()), (2100000, Some("0123abc")));
    /// ```
    #[must_use]
    pub fn obj_ref(&self) -> Option<ObjRef> {
        let file_id = self.get("fileID")?.as_i64()?;
        Some(ObjRef {
            file_id,
            guid: self["guid"].as_str().map(str::to_owned),
            kind: self["type"].as_i64().map(|t| t as i32),
        })
    }
}

/// A reference to an object: in this file (no GUID) or in the asset with
/// that GUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjRef {
    /// The object's id in its file; 0 is nothing.
    pub file_id: i64,
    /// The asset's GUID, for objects in other files.
    pub guid: Option<String>,
    /// Unity's reference type (2: an asset, 3: an imported asset such as a model).
    pub kind: Option<i32>,
}

impl ObjRef {
    /// Whether it refers to nothing.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.file_id == 0
    }
}

pub(crate) struct ParseError {
    pub line: usize,
    pub message: String,
}

/// How deep maps and lists may nest in a text file. Unity's nest a
/// handful of levels; values deeper than this would take a stack frame
/// per level to build, compare and drop.
pub(crate) const MAX_DEPTH: usize = 128;

/// Builds values from the parser's events.
#[derive(Default)]
struct Builder {
    /// Open maps and lists; a map holds its pending key.
    stack: Vec<(Value, Option<String>)>,
    done: Option<Value>,
}

impl Builder {
    fn push_value(&mut self, v: Value) {
        match self.stack.last_mut() {
            None => self.done = Some(v),
            Some((Value::Seq(items), _)) => items.push(v),
            Some((Value::Map(entries), key)) => match key.take() {
                None => {
                    // A key: Unity's keys are scalars.
                    *key = Some(match v {
                        Value::Scalar(s) => s,
                        _ => String::new(),
                    });
                }
                Some(k) => entries.push((k, v)),
            },
            Some(_) => unreachable!("only maps and lists are open"),
        }
    }

    /// Takes one event; an error for nesting deeper than [`MAX_DEPTH`].
    fn on_event(&mut self, ev: Event) -> Result<(), String> {
        let open = match ev {
            Event::Scalar(text, ..) => {
                self.push_value(Value::Scalar(text));
                return Ok(());
            }
            Event::SequenceStart(..) => Value::Seq(Vec::new()),
            Event::MappingStart(..) => Value::Map(Vec::new()),
            Event::SequenceEnd | Event::MappingEnd => {
                if let Some((v, _)) = self.stack.pop() {
                    self.push_value(v);
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        if self.stack.len() >= MAX_DEPTH {
            return Err(format!("maps and lists nested deeper than {MAX_DEPTH}"));
        }
        self.stack.push((open, None));
        Ok(())
    }
}

/// Parses one document's YAML. The parser is driven event by event here
/// (its own loader recurses once per nesting level), so how deep a file
/// nests is checked before anything is built for it.
pub(crate) fn parse(text: &str) -> Result<Value, ParseError> {
    let mut builder = Builder::default();
    let mut parser = Parser::new_from_str(text);
    loop {
        let (ev, mark) = parser.next_token().map_err(|e| ParseError {
            line: e.marker().line(),
            message: e.info().to_owned(),
        })?;
        // One document: the first one's end (or the stream's) finishes it.
        if matches!(ev, Event::DocumentEnd | Event::StreamEnd) {
            break;
        }
        builder.on_event(ev).map_err(|message| ParseError {
            line: mark.line(),
            message,
        })?;
    }
    Ok(builder.done.unwrap_or_default())
}
