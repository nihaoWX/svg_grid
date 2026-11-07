//! A source SVG's **natural** width and height, in points.
//!
//! `doctor` composes every fixture at its own natural size (canvas = panel +
//! `2 × --plot-margin` default) so that no fixture trips W1 just because doctor
//! resized it. That means reading `width`/`height` off the source file — the
//! corpus writes them in `pt` (svglite: `width='216.00pt'`, matplotlib:
//! `width="216pt"`), but `px`/`in`/`cm`/`mm`/`pc` are converted too, and a file
//! that only has a `viewBox` falls back to the viewBox's extent, which is what the
//! renderer does as well.

/// The parsed `width`/`height` of one SVG document, in pt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Size {
    pub width: f32,
    pub height: f32,
}

/// Read the natural size out of an SVG document.
///
/// Errors are one sentence naming what is missing, so a malformed fixture is
/// reported as a failed criterion rather than as a mysterious zero-size canvas.
pub(crate) fn natural_size(svg: &str) -> Result<Size, String> {
    let tag =
        svg_root_tag(svg).ok_or_else(|| "the document has no <svg> root element".to_string())?;
    let attributes = attributes(tag);

    let width = attribute(&attributes, "width").and_then(length_in_pt);
    let height = attribute(&attributes, "height").and_then(length_in_pt);
    if let (Some(width), Some(height)) = (width, height) {
        if width > 0.0 && height > 0.0 {
            return Ok(Size { width, height });
        }
    }

    let view_box = attribute(&attributes, "viewBox").ok_or_else(|| {
        "the <svg> root has no resolvable width/height and no viewBox".to_string()
    })?;
    let numbers: Vec<f32> = view_box
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f32>().ok())
        .collect();
    match numbers.as_slice() {
        [_, _, width, height] if *width > 0.0 && *height > 0.0 => Ok(Size {
            width: *width,
            height: *height,
        }),
        _ => Err(format!(
            "the <svg> root has unusable dimensions (width/height and viewBox={view_box:?})"
        )),
    }
}

/// The `<svg …>` start tag's text (between `<svg` and its closing `>`), skipping
/// anything quoted so a `>` inside an attribute value cannot end the tag early.
fn svg_root_tag(svg: &str) -> Option<&str> {
    let start = svg.find("<svg")? + "<svg".len();
    let rest = &svg[start..];
    let mut quote: Option<char> = None;
    for (offset, character) in rest.char_indices() {
        match quote {
            Some(open) if character == open => quote = None,
            Some(_) => {}
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == '>' => return Some(&rest[..offset]),
            None => {}
        }
    }
    None
}

/// `name="value"` / `name='value'` pairs of one start tag.
fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len() && bytes[index] != b'=' && !(bytes[index] as char).is_whitespace()
        {
            index += 1;
        }
        let name = tag[name_start..index].to_string();
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let quote = bytes[index] as char;
        if quote != '"' && quote != '\'' {
            // Unquoted value: read to the next whitespace.
            let value_start = index;
            while index < bytes.len() && !(bytes[index] as char).is_whitespace() {
                index += 1;
            }
            found.push((name, tag[value_start..index].to_string()));
            continue;
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] as char != quote {
            index += 1;
        }
        if !name.is_empty() {
            found.push((name, tag[value_start..index].to_string()));
        }
        index += 1;
    }
    found
}

fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// One SVG length (`216`, `216.00pt`, `6in`, `100px`) in pt.
///
/// Unitless values are points: that is what the SVG user unit is in this tool's
/// world (1 `pt` = 1 canvas unit), and it keeps `width="216"` from silently
/// becoming 288 pt the way a CSS `px` would.
fn length_in_pt(value: &str) -> Option<f32> {
    let value = value.trim();
    let end = value
        .find(|character: char| {
            !(character.is_ascii_digit()
                || character == '.'
                || character == '+'
                || character == '-'
                || character == 'e')
        })
        .unwrap_or(value.len());
    let number: f32 = value[..end].parse().ok()?;
    let unit = value[end..].trim().to_ascii_lowercase();
    let factor = match unit.as_str() {
        "" | "pt" => 1.0,
        "px" => 0.75, // CSS px: 96 per inch, 72 pt per inch
        "in" => 72.0,
        "pc" => 12.0,
        "cm" => 72.0 / 2.54,
        "mm" => 72.0 / 25.4,
        "q" => 72.0 / 101.6,
        // `%`, `em`, `ex` need a context this reader does not have: fall back to
        // the viewBox, which is exactly what a missing width would do.
        _ => return None,
    };
    let points = number * factor;
    points.is_finite().then_some(points)
}

#[cfg(test)]
mod tests {
    use super::{natural_size, Size};

    #[test]
    fn svglite_points_are_read() {
        let svg = "<?xml version='1.0'?>\n<svg xmlns='http://www.w3.org/2000/svg' \
                   width='216.00pt' height='144.00pt' viewBox='0 0 216.00 144.00'>\n";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 216.0,
                height: 144.0
            })
        );
    }

    #[test]
    fn matplotlib_points_and_a_long_prologue_are_read() {
        let svg = "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \
                   \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n<svg \
                   xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"216pt\" height=\"144pt\" \
                   viewBox=\"0 0 216 144\">\n";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 216.0,
                height: 144.0
            })
        );
    }

    #[test]
    fn other_units_are_converted() {
        let svg = "<svg width=\"6in\" height=\"2in\" viewBox=\"0 0 1 1\">";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 432.0,
                height: 144.0
            })
        );
        let svg = "<svg width=\"288px\" height=\"192px\">";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 216.0,
                height: 144.0
            })
        );
    }

    #[test]
    fn a_missing_size_falls_back_to_the_view_box() {
        let svg = "<svg viewBox=\"10 20 300 200\">";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 300.0,
                height: 200.0
            })
        );
    }

    #[test]
    fn unusable_dimensions_are_an_error_not_a_zero() {
        for svg in [
            "<html><body/></html>",
            "<svg>",
            "<svg width=\"100%\" height=\"100%\">",
            "<svg viewBox=\"0 0 0 0\">",
        ] {
            assert!(
                natural_size(svg).is_err(),
                "{svg:?} must not yield a usable size"
            );
        }
    }

    #[test]
    fn a_quoted_angle_bracket_does_not_end_the_tag() {
        let svg = "<svg width=\"100pt\" height=\"80pt\" data-note=\"a > b\">";
        assert_eq!(
            natural_size(svg),
            Ok(Size {
                width: 100.0,
                height: 80.0
            })
        );
    }
}
