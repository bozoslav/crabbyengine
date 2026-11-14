//! A small element tree over `quick-xml`, enough for Tiled's XML files
//! (`.tmx` maps, `.tsx` tilesets, `.tx` templates).

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// One XML element: its name, attributes, child elements and text.
#[derive(Debug, Default, Clone)]
pub(crate) struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    /// Attribute `key`.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The first child named `name`.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Every child named `name`.
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
}

fn open(e: &BytesStart) -> Result<Element, String> {
    let mut el = Element {
        name: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
        ..Default::default()
    };
    for a in e.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let value = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| e.to_string())?
            .into_owned();
        el.attrs.push((key, value));
    }
    Ok(el)
}

/// The predefined entities and character references.
fn entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        _ => {
            let n = name.strip_prefix('#')?;
            let code = match n.strip_prefix('x') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)?
        }
    })
}

/// Parses `text` into its root element.
pub(crate) fn parse(text: &str) -> Result<Element, String> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Element> = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("XML error at byte {}: {e}", reader.buffer_position()))?;
        match event {
            Event::Start(e) => stack.push(open(&e)?),
            Event::Empty(e) => {
                let el = open(&e)?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => return Ok(el),
                }
            }
            Event::End(_) => {
                let el = stack.pop().ok_or("XML error: a closing tag without its opening tag")?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => return Ok(el),
                }
            }
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.decode().map_err(|e| e.to_string())?);
                }
            }
            Event::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.decode().map_err(|e| e.to_string())?);
                }
            }
            Event::GeneralRef(r) => {
                let name = r.decode().map_err(|e| e.to_string())?;
                let c = entity(&name).ok_or_else(|| format!("XML error: unknown entity &{name};"))?;
                if let Some(top) = stack.last_mut() {
                    top.text.push(c);
                }
            }
            Event::Eof => return Err("XML error: the document has no root element, or it isn't closed".into()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_attributes_text_and_entities_are_read() {
        let root = parse(r#"<?xml version="1.0"?><map a="1" b="x &amp; y"><data>1,2&#44;3</data><tile id="4"/></map>"#)
            .unwrap();
        assert_eq!(root.name, "map");
        assert_eq!(root.attr("b"), Some("x & y"));
        assert_eq!(root.child("data").unwrap().text, "1,2,3");
        assert_eq!(root.all("tile").next().unwrap().attr("id"), Some("4"));
        assert!(parse("<map>").is_err());
    }
}
