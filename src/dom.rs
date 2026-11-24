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

/// XML 1.0 `Char` production:
/// `#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`.
///
/// `#xFFFE`/`#xFFFF` are **not** legal (the upper bound is `#xFFFD`), and neither
/// are the surrogate range or `#x0`. A Rust `char` cannot hold a surrogate, so the
/// two BMP ranges below already exclude `#xD800-#xDFFF`.
pub(crate) fn is_xml_char(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&c)
        || ('\u{E000}'..='\u{FFFD}').contains(&c)
        || ('\u{10000}'..='\u{10FFFF}').contains(&c)
}

/// The first character in `value` that XML 1.0 forbids, if any.
pub(crate) fn first_invalid_xml_char(value: &str) -> Option<char> {
    value.chars().find(|c| !is_xml_char(*c))
}

/// `U+0001`-style label for a codepoint (four hex digits minimum).
pub(crate) fn codepoint_label(code: u32) -> String {
    format!("U+{code:04X}")
}

/// Decode one reference (`reference` is the text between `&` and `;`) and reject
/// anything that is not a legal XML 1.0 character.
///
/// `quick_xml::escape::unescape` stays the authority for the reference syntax: it
/// rejects `0`, the surrogate range, `+`/`-`, an empty body and undefined names
/// (round-data/R47). This wrapper adds the XML 1.0 `Char` check that `unescape`
/// does not perform (`&#1;`, `&#xFFFE;`, …), so both DOMs share one predicate.
pub(crate) fn decode_reference(reference: &str) -> std::result::Result<String, String> {
    let literal = format!("&{reference};");
    match quick_xml::escape::unescape(&literal) {
        Ok(decoded) => match first_invalid_xml_char(&decoded) {
            Some(c) => Err(format!(
                "character reference {literal} decodes to {}, which is not a legal XML 1.0 character",
                codepoint_label(c as u32)
            )),
            None => Ok(decoded.into_owned()),
        },
        Err(error) => Err(match numeric_reference_codepoint(reference) {
            Some(code) => format!(
                "character reference {literal} ({}) is not a legal XML 1.0 character: {error}",
                codepoint_label(code)
            ),
            None => format!("reference {literal} cannot be resolved: {error}"),
        }),
    }
}

/// The codepoint a numeric reference names, when it is syntactically one. Used
/// only to make the error message name `U+…`; validation stays in `unescape`.
fn numeric_reference_codepoint(reference: &str) -> Option<u32> {
    let digits = reference.strip_prefix('#')?;
    let (radix, digits) = match digits.strip_prefix('x') {
        Some(hex) => (16, hex),
        None => (10, digits),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    u32::from_str_radix(digits, radix).ok()
}

/// The first reference in a raw attribute value that must be rejected, if any.
///
/// `raw` is the value as written (still escaped), so the message can quote the
/// reference exactly as the source spells it (`&#x1;` stays `&#x1;`).
pub(crate) fn invalid_reference_in_raw(raw: &[u8]) -> Option<String> {
    let mut rest = raw;
    while let Some(amp) = rest.iter().position(|byte| *byte == b'&') {
        let after = &rest[amp + 1..];
        let end = after.iter().position(|byte| *byte == b';')?;
        if let Ok(reference) = std::str::from_utf8(&after[..end]) {
            if let Err(message) = decode_reference(reference) {
                return Some(message);
            }
        }
        rest = &after[end + 1..];
    }
    None
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
                    let text = text
                        .xml_content()
                        .map_err(quick_xml::Error::from)?
                        .into_owned();
                    if let Some(c) = first_invalid_xml_char(&text) {
                        return Err(SvgGridError::Xml(format!(
                            "text contains {}, which is not a legal XML 1.0 character",
                            codepoint_label(c as u32)
                        )));
                    }
                    parent.children.push(Node::Text(text));
                }
            }
            Event::CData(cdata) => {
                if let Some(parent) = stack.last_mut() {
                    let text = cdata
                        .xml_content()
                        .map_err(quick_xml::Error::from)?
                        .into_owned();
                    if let Some(c) = first_invalid_xml_char(&text) {
                        return Err(SvgGridError::Xml(format!(
                            "CDATA section contains {}, which is not a legal XML 1.0 character",
                            codepoint_label(c as u32)
                        )));
                    }
                    parent.children.push(Node::Text(text));
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(parent) = stack.last_mut() {
                    let reference = reference
                        .xml_content()
                        .map_err(quick_xml::Error::from)?
                        .into_owned();
                    // `quick_xml` hands the reference over without its `&`/`;`.
                    // `decode_reference` keeps `unescape` as the syntax authority
                    // and adds the XML 1.0 `Char` check (round-data/R50).
                    let decoded = decode_reference(&reference).map_err(SvgGridError::Xml)?;
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
        if let Some(message) = invalid_reference_in_raw(attr.value.as_ref()) {
            return Err(SvgGridError::Xml(format!(
                "attribute `{attr_name}`: {message}"
            )));
        }
        let attr_value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| SvgGridError::Xml(format!("attribute `{attr_name}`: {error}")))?
            .into_owned();
        if let Some(c) = first_invalid_xml_char(&attr_value) {
            return Err(SvgGridError::Xml(format!(
                "attribute `{attr_name}` contains {}, which is not a legal XML 1.0 character",
                codepoint_label(c as u32)
            )));
        }
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

    /// The XML 1.0 `Char` production, boundary by boundary.
    #[test]
    fn the_xml_char_predicate_matches_the_xml_1_0_production() {
        for c in [
            '\u{9}',
            '\u{A}',
            '\u{D}',
            '\u{20}',
            '\u{D7FF}',
            '\u{E000}',
            '\u{FFFD}',
            '\u{10000}',
            '\u{10FFFF}',
        ] {
            assert!(is_xml_char(c), "{c:?} must be legal");
        }
        for c in [
            '\u{0}', '\u{8}', '\u{B}', '\u{C}', '\u{1F}', '\u{FFFE}', '\u{FFFF}',
        ] {
            assert!(!is_xml_char(c), "{c:?} must be illegal");
        }
    }

    /// The legal surface R50 must not regress: every reference stays accepted and
    /// decodes to the character it names.
    #[test]
    fn legal_character_references_stay_accepted() {
        for (reference, expected) in [
            ("&#9;", "\t"),
            ("&#10;", "\n"),
            ("&#13;", "\r"),
            ("&#32;", " "),
            ("&#65;", "A"),
            ("&#x1F600;", "😀"),
            ("&#xFFFD;", "\u{FFFD}"),
            ("&#x10FFFF;", "\u{10FFFF}"),
            ("&#xE000;", "\u{E000}"),
            ("&#xD7FF;", "\u{D7FF}"),
        ] {
            assert_eq!(decode(reference).unwrap(), expected, "{reference}");
        }
    }

    /// The illegal surface R50 must reject: the message names the reference as
    /// written and the codepoint, never a range relative to a synthesized string.
    #[test]
    fn illegal_character_references_name_the_reference_and_codepoint() {
        for (reference, codepoint) in [
            ("&#0;", "U+0000"),
            ("&#x0;", "U+0000"),
            ("&#1;", "U+0001"),
            ("&#8;", "U+0008"),
            ("&#11;", "U+000B"),
            ("&#12;", "U+000C"),
            ("&#31;", "U+001F"),
            ("&#xFFFE;", "U+FFFE"),
            ("&#xFFFF;", "U+FFFF"),
            ("&#55296;", "U+D800"),
            ("&#xDFFF;", "U+DFFF"),
            ("&#x110000;", "U+110000"),
        ] {
            let error = decode(reference).unwrap_err().to_string();
            assert!(
                error.starts_with("SVG XML error: "),
                "{reference} => {error}"
            );
            assert!(error.contains(reference), "{reference} => {error}");
            assert!(error.contains(codepoint), "{reference} => {error}");
            assert!(!error.contains("at 1.."), "{reference} => {error}");
        }
    }

    /// The attribute path uses the same predicate and names the attribute.
    #[test]
    fn illegal_character_references_in_attributes_name_the_attribute() {
        for (reference, codepoint) in [
            ("&#0;", "U+0000"),
            ("&#1;", "U+0001"),
            ("&#xFFFE;", "U+FFFE"),
            ("&#55296;", "U+D800"),
            ("&#x110000;", "U+110000"),
        ] {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg"><text data-x="a{reference}b">x</text></svg>"#
            );
            let error = parse(&svg).unwrap_err().to_string();
            assert!(
                error.contains("attribute `data-x`"),
                "{reference} => {error}"
            );
            assert!(error.contains(reference), "{reference} => {error}");
            assert!(error.contains(codepoint), "{reference} => {error}");
        }
    }

    /// A literal (not referenced) control character is the same defect class and
    /// must not reach the serializer either.
    #[test]
    fn literal_control_characters_are_rejected_in_text_and_attributes() {
        let text = "<svg xmlns=\"http://www.w3.org/2000/svg\"><text>a\u{1}b</text></svg>";
        let error = parse(text).unwrap_err().to_string();
        assert!(error.contains("text contains U+0001"), "{error}");

        let attr =
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><text data-x=\"a\u{1}b\">x</text></svg>";
        let error = parse(attr).unwrap_err().to_string();
        assert!(
            error.contains("attribute `data-x` contains U+0001"),
            "{error}"
        );
    }
}
