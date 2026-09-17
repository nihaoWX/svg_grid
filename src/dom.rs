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
                    parent
                        .children
                        .push(Node::Text(resolve_reference(&reference)));
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

fn resolve_reference(reference: &str) -> String {
    match reference {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => format!("&{reference};"),
    }
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
