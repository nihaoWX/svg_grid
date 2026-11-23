use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::{Result, SvgGridError};

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
            .find(|(attr_name, _)| attr_name == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn set_attr(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        if self.attr(name).is_none() {
            self.attrs.push((name.to_string(), value));
            return;
        }

        if let Some((_, attr_value)) = self
            .attrs
            .iter_mut()
            .find(|(attr_name, _)| attr_name == name)
        {
            *attr_value = value;
        }
    }

    pub fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|(key, _)| key != name);
    }
}

pub fn parse(svg: &str) -> Result<Element> {
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;

    loop {
        match reader.read_event()? {
            Event::Start(start) => stack.push(element_from_start(&reader, &start)?),
            Event::Empty(start) => {
                let element = element_from_start(&reader, &start)?;
                push_element(&mut stack, &mut root, element)?;
            }
            Event::Text(text) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(
                        text.xml_content()
                            .map_err(quick_xml::Error::from)?
                            .into_owned(),
                    ));
                }
            }
            Event::CData(cdata) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(
                        cdata
                            .xml_content()
                            .map_err(quick_xml::Error::from)?
                            .into_owned(),
                    ));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(parent) = stack.last_mut() {
                    let reference = reference
                        .xml_content()
                        .map_err(quick_xml::Error::from)?
                        .into_owned();
                    // `quick_xml` hands the reference over without its `&`/`;`.
                    // `unescape` is the authority for both character references
                    // (`&#176;`, `&#x42;`) and the five predefined entities; a
                    // hand-written number parser would miss `0`, the surrogate
                    // range and `+`/`-` (round-data/R47).
                    let decoded = quick_xml::escape::unescape(&format!("&{reference};"))
                        .map_err(quick_xml::Error::from)?
                        .into_owned();
                    parent.children.push(Node::Text(decoded));
                }
            }
            Event::End(_) => {
                let element = stack.pop().ok_or_else(|| {
                    SvgGridError::InvalidInput("unexpected closing SVG tag".to_string())
                })?;
                push_element(&mut stack, &mut root, element)?;
            }
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) | Event::Comment(_) => {}
        }
    }

    if !stack.is_empty() {
        return Err(SvgGridError::InvalidInput(
            "unexpected end of SVG before closing all tags".to_string(),
        ));
    }

    root.ok_or_else(|| {
        SvgGridError::InvalidInput("SVG input does not contain a root element".into())
    })
}

pub fn serialize(root: &Element) -> Result<String> {
    let mut writer = Writer::new(Vec::new());
    write_element(&mut writer, root)?;
    String::from_utf8(writer.into_inner()).map_err(|error| {
        SvgGridError::InvalidInput(format!("serialized SVG is not UTF-8: {error}"))
    })
}

fn element_from_start(reader: &Reader<&[u8]>, start: &BytesStart<'_>) -> Result<Element> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for attr in start.attributes().with_checks(false) {
        let attr = attr.map_err(quick_xml::Error::from)?;
        let attr_name = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let attr_value = attr
            .decode_and_unescape_value(reader.decoder())?
            .into_owned();
        attrs.push((attr_name, attr_value));
    }
    Ok(Element {
        name,
        attrs,
        children: Vec::new(),
    })
}

fn push_element(stack: &mut [Element], root: &mut Option<Element>, element: Element) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(Node::Element(element));
    } else if root.replace(element).is_some() {
        return Err(SvgGridError::InvalidInput(
            "SVG input contains multiple root elements".to_string(),
        ));
    }
    Ok(())
}

fn write_element(writer: &mut Writer<Vec<u8>>, element: &Element) -> Result<()> {
    let mut start = BytesStart::new(element.name.as_str());
    for (name, value) in &element.attrs {
        start.push_attribute((name.as_str(), value.as_str()));
    }

    if element.children.is_empty() {
        writer.write_event(Event::Empty(start))?;
        return Ok(());
    }

    writer.write_event(Event::Start(start))?;
    for child in &element.children {
        match child {
            Node::Element(child_element) => write_element(writer, child_element)?,
            Node::Text(text) => writer.write_event(Event::Text(BytesText::new(text)))?,
        }
    }
    writer.write_event(Event::End(BytesEnd::new(element.name.as_str())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse one `<text>` body and return the decoded character data.
    fn decode(body: &str) -> Result<String> {
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
    fn character_references_decode_to_their_characters() {
        for (reference, expected) in [
            ("&#65;", "A"),
            ("&#x42;", "B"),
            ("&#176;", "°"),
            ("&#065;", "A"),
            ("&#x0041;", "A"),
            ("&#x1F600;", "😀"),
        ] {
            assert_eq!(decode(reference).unwrap(), expected, "{reference}");
        }
    }

    #[test]
    fn whitespace_character_references_are_passed_through() {
        for (reference, expected) in [
            ("&#9;", "\t"),
            ("&#10;", "\n"),
            ("&#13;", "\r"),
            ("&#32;", " "),
        ] {
            assert_eq!(decode(reference).unwrap(), expected, "{reference}");
        }
    }

    #[test]
    fn the_five_predefined_entities_still_decode() {
        assert_eq!(decode("&amp;&lt;&gt;&quot;&apos;").unwrap(), "&<>\"'");
    }

    #[test]
    fn invalid_and_undefined_references_are_rejected() {
        for (reference, expected) in [
            ("&#0;", "invalid character reference"),
            ("&#x110000;", "invalid character reference"),
            ("&#55296;", "invalid character reference"),
            ("&#;", "invalid character reference"),
            ("&#-1;", "invalid character reference"),
            ("&#X41;", "invalid character reference"),
            ("&nbsp;", "unrecognized entity"),
        ] {
            let error = decode(reference).unwrap_err().to_string();
            assert!(error.contains(expected), "{reference} => {error}");
        }
    }
}
