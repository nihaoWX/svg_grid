//! `<path>` → `<polyline>`/`<polygon>` rewriting.
//!
//! Straight commands are exact; curves are flattened to within
//! [`DEFAULT_TOLERANCE`] source units, after which the (already baked) transform
//! is applied to every point.

use crate::affine::Affine;
use crate::dom::Element;
use crate::error::ConvertError;
use crate::path::{flatten_path, DEFAULT_TOLERANCE};
use crate::transform::fmt_num;

use super::style::{parse_style, style_without};

pub(super) fn path_shapes(
    element: &Element,
    transform: Affine,
) -> Result<Vec<Element>, ConvertError> {
    let d = element.attr("d").unwrap_or("");
    let subpaths = flatten_path(d, DEFAULT_TOLERANCE, &transform).map_err(|error| {
        ConvertError::Translate(format!("failed to flatten <path d={d:?}>: {error}"))
    })?;

    let style = element.attr("style").unwrap_or("");
    let properties = parse_style(style);
    let fill = properties
        .iter()
        .rev()
        .find(|(key, _)| key == "fill")
        .map(|(_, value)| value.clone())
        .or_else(|| element.attr("fill").map(str::to_string))
        .unwrap_or_else(|| "black".to_string());
    let filled = !fill.trim().eq_ignore_ascii_case("none");
    let stroke_width = properties
        .iter()
        .rev()
        .find(|(key, _)| key == "stroke-width")
        .map(|(_, value)| value.clone())
        .or_else(|| element.attr("stroke-width").map(str::to_string));
    let remaining_style = style_without(style, "stroke-width");

    let mut shapes = Vec::with_capacity(subpaths.len());
    for subpath in subpaths {
        let name = if subpath.closed && filled {
            "polygon"
        } else {
            "polyline"
        };
        let points = subpath
            .points
            .iter()
            .map(|(x, y)| format!("{},{}", fmt_num(*x), fmt_num(*y)))
            .collect::<Vec<_>>()
            .join(" ");
        let mut shape = Element {
            name: name.to_string(),
            attrs: Vec::new(),
            children: Vec::new(),
        };
        for (attr, value) in &element.attrs {
            if matches!(attr.as_str(), "d" | "style" | "transform" | "id") {
                continue;
            }
            shape.set_attr(attr, value.clone());
        }
        if let Some(stroke_width) = &stroke_width {
            shape.set_attr("stroke-width", stroke_width.clone());
        }
        if let Some(remaining_style) = &remaining_style {
            shape.set_attr("style", remaining_style.clone());
        }
        shape.set_attr("points", points);
        shapes.push(shape);
    }
    Ok(shapes)
}
