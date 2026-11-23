//! Minimal XML DOM (mirrors `tools/svg_grid/src/dom.rs` so the byte layout of
//! the output matches what the composer expects to re-parse).

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::convert::error::ConvertError;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn set_attr(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some((_, slot)) = self.attrs.iter_mut().find(|(key, _)| key == name) {
            *slot = value;
        } else {
            self.attrs.push((name.to_string(), value));
        }
    }

    pub(crate) fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|(key, _)| key != name);
    }

    pub(crate) fn new(name: impl Into<String>, attrs: Vec<(&str, String)>) -> Self {
        Element {
            name: name.into(),
            attrs: attrs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
            children: Vec::new(),
        }
    }
}

pub(crate) fn parse(svg: &str) -> Result<Element, ConvertError> {
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;

    loop {
        match reader.read_event().map_err(ConvertError::xml)? {
            Event::Start(start) => stack.push(element_from_start(&reader, &start)?),
            Event::Empty(start) => {
                let element = element_from_start(&reader, &start)?;
                push_element(&mut stack, &mut root, element)?;
            }
            Event::Text(text) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(
                        text.xml_content().map_err(ConvertError::xml)?.into_owned(),
                    ));
                }
            }
            Event::CData(cdata) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(
                        cdata.xml_content().map_err(ConvertError::xml)?.into_owned(),
                    ));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(parent) = stack.last_mut() {
                    let reference = reference.xml_content().map_err(ConvertError::xml)?;
                    // Same authority as `src/dom.rs`: `unescape` resolves both
                    // character references and the five predefined entities, and
                    // rejects undefined names (round-data/R47).
                    let decoded = quick_xml::escape::unescape(&format!("&{reference};"))
                        .map_err(ConvertError::xml)?
                        .into_owned();
                    parent.children.push(Node::Text(decoded));
                }
            }
            Event::End(_) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| ConvertError::Xml("unexpected closing tag".to_string()))?;
                push_element(&mut stack, &mut root, element)?;
            }
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) | Event::Comment(_) => {}
        }
    }

    if !stack.is_empty() {
        return Err(ConvertError::Xml(
            "unexpected end of input before all tags were closed".to_string(),
        ));
    }
    root.ok_or_else(|| ConvertError::Xml("input does not contain a root element".to_string()))
}

fn element_from_start(
    reader: &Reader<&[u8]>,
    start: &BytesStart<'_>,
) -> Result<Element, ConvertError> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for attr in start.attributes().with_checks(false) {
        let attr = attr.map_err(ConvertError::xml)?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(ConvertError::xml)?
            .into_owned();
        attrs.push((key, value));
    }
    Ok(Element {
        name,
        attrs,
        children: Vec::new(),
    })
}

fn push_element(
    stack: &mut [Element],
    root: &mut Option<Element>,
    element: Element,
) -> Result<(), ConvertError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(Node::Element(element));
    } else if root.replace(element).is_some() {
        return Err(ConvertError::Xml(
            "input contains multiple root elements".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn serialize(root: &Element) -> String {
    let mut writer = Writer::new(Vec::new());
    write_element(&mut writer, root);
    let body = String::from_utf8(writer.into_inner()).unwrap_or_default();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{body}")
}

fn write_element(writer: &mut Writer<Vec<u8>>, element: &Element) {
    let mut start = BytesStart::new(element.name.as_str());
    for (name, value) in &element.attrs {
        start.push_attribute((name.as_str(), value.as_str()));
    }
    if element.children.is_empty() {
        writer
            .write_event(Event::Empty(start))
            .expect("writing to an in-memory buffer cannot fail");
        return;
    }
    writer
        .write_event(Event::Start(start))
        .expect("writing to an in-memory buffer cannot fail");
    for child in &element.children {
        match child {
            Node::Element(child) => write_element(writer, child),
            Node::Text(text) => {
                writer
                    .write_event(Event::Text(BytesText::new(text)))
                    .expect("writing to an in-memory buffer cannot fail");
            }
        }
    }
    writer
        .write_event(Event::End(BytesEnd::new(element.name.as_str())))
        .expect("writing to an in-memory buffer cannot fail");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse one `<text>` body and return the decoded character data.
    fn decode(body: &str) -> Result<String, ConvertError> {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg"><text>{body}</text></svg>"#);
        let root = parse(&svg)?;
        let Node::Element(text) = &root.children[0] else {
            panic!("expected a <text> element");
        };
        let mut decoded = String::new();
        for child in &text.children {
            match child {
                Node::Text(value) => decoded.push_str(value),
                Node::Element(element) => panic!("unexpected <{}>", element.name),
            }
        }
        Ok(decoded)
    }

    #[test]
    fn character_references_are_unescaped_with_quick_xml_semantics() {
        for (reference, expected) in [
            ("&#65;", "A"),
            ("&#x42;", "B"),
            ("&#176;", "°"),
            ("&#x1F600;", "😀"),
            ("&amp;", "&"),
        ] {
            assert_eq!(decode(reference).unwrap(), expected, "{reference}");
        }
    }

    #[test]
    fn undefined_and_malformed_references_fail_closed() {
        for (reference, expected) in [
            ("&nbsp;", "unrecognized entity"),
            ("&#0;", "invalid character reference"),
            ("&#55296;", "invalid character reference"),
            ("&#X41;", "invalid character reference"),
        ] {
            let error = decode(reference).unwrap_err().to_string();
            assert!(error.starts_with("XML error: "), "{reference} => {error}");
            assert!(error.contains(expected), "{reference} => {error}");
        }
    }
}
