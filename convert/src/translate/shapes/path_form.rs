//! The draw-form decision table for `<path>`: filled / stroked / subpath count
//! decide the emitted `<polyline>`/`<polygon>` (see the module docs above).

use crate::affine::Affine;
use crate::dom::Element;
use crate::error::ConvertError;
use crate::path::{flatten_path, Subpath, DEFAULT_TOLERANCE};
use crate::stroke::is_stroked;
use crate::transform::fmt_num;

use crate::translate::style::{parse_style, style_without};

pub(crate) fn path_shapes(
    element: &Element,
    transform: Affine,
) -> Result<Vec<Element>, ConvertError> {
    let d = element.attr("d").unwrap_or("");
    let subpaths = flatten_path(d, DEFAULT_TOLERANCE, &transform).map_err(|error| {
        ConvertError::Translate(format!("failed to flatten <path d={d:?}>: {error}"))
    })?;
    if subpaths.is_empty() {
        // Only degenerate subpaths (e.g. a lone `M x y`): nothing to draw, exactly
        // as before (no empty `<polygon>` is emitted).
        return Ok(Vec::new());
    }

    let style = element.attr("style").unwrap_or("");
    let properties = parse_style(style);
    let fill = property(&properties, element, "fill").unwrap_or_else(|| "black".to_string());
    let filled = !fill.trim().eq_ignore_ascii_case("none");
    let stroked = is_stroked(property(&properties, element, "stroke").as_deref());
    let stroke_width = property(&properties, element, "stroke-width");
    let remaining_style = style_without(style, "stroke-width");

    let mut shapes = Vec::new();
    if !filled {
        // Stroked-only path: one `<polyline>` per subpath, presentation copied
        // verbatim (a zero-area contour must not be merged or dropped).
        for subpath in &subpaths {
            let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
            shape.name = "polyline".to_string();
            shape.set_attr("points", join_points(subpath.points.iter()));
            shapes.push(shape);
        }
    } else if !stroked {
        // Fill-only path: the whole region in one `<polygon>`; the source
        // `fill-rule` (if any) is carried over by `shape_skeleton` so the holes
        // keep their meaning, and every subpath is explicitly closed (see the
        // module docs).
        let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
        shape.name = "polygon".to_string();
        shape.set_attr("points", join_closed_points(&subpaths));
        shapes.push(shape);
    } else if subpaths.len() == 1 {
        // Filled *and* stroked with a single subpath (the common patch/bar): keep
        // the pre-existing shape exactly — a `<polygon>` when closed, a
        // `<polyline>` otherwise, carrying both paints.
        let subpath = &subpaths[0];
        let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
        shape.name = if subpath.closed {
            "polygon"
        } else {
            "polyline"
        }
        .to_string();
        shape.set_attr("points", join_points(subpath.points.iter()));
        shapes.push(shape);
    } else {
        // Filled *and* stroked with several subpaths: the fill is one region but
        // the stroke follows each subpath, and a single primitive cannot paint
        // both (it would stroke the bridges). Split into a fill-only `<polygon>`
        // and one stroke-only shape per subpath. Each paint is removed
        // *thoroughly*: the declaration is dropped from the inline `style` and the
        // opposite paint is forced with a presentation attribute, which overrides
        // both the source's own declaration and anything inherited.
        let fill_style = remaining_style
            .as_deref()
            .and_then(|style| style_without(style, "stroke"));
        let mut fill_shape = shape_skeleton(element, &None, &fill_style);
        fill_shape.name = "polygon".to_string();
        fill_shape.remove_attr("stroke-width");
        fill_shape.set_attr("stroke", "none");
        fill_shape.set_attr("points", join_closed_points(&subpaths));
        shapes.push(fill_shape);

        let stroke_style = remaining_style
            .as_deref()
            .and_then(|style| style_without(style, "fill"));
        for subpath in &subpaths {
            let mut shape = shape_skeleton(element, &stroke_width, &stroke_style);
            shape.name = if subpath.closed {
                "polygon"
            } else {
                "polyline"
            }
            .to_string();
            shape.set_attr("fill", "none");
            shape.set_attr("points", join_points(subpath.points.iter()));
            shapes.push(shape);
        }

        debug_assert!(
            shapes
                .iter()
                .all(|shape| !(own_fill(shape) && own_stroke(shape))),
            "a split <path> must not produce a shape that both fills and strokes"
        );
    }
    Ok(shapes)
}

/// The element's own value for a presentation property: an inline `style`
/// declaration wins over the presentation attribute (CSS), as the browser
/// resolves it.
fn property(properties: &[(String, String)], element: &Element, name: &str) -> Option<String> {
    properties
        .iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
        .or_else(|| element.attr(name).map(str::to_string))
}

/// [`property`] for an already-built element, reparsing its serialized style.
/// Used by the split assertion (and its tests).
fn own_property(element: &Element, name: &str) -> Option<String> {
    let style = element.attr("style").unwrap_or("");
    property(&parse_style(style), element, name)
}

/// Whether the element paints a fill of its own: its `fill` resolves to something
/// other than `none` (a missing `fill` is the SVG default black).
pub(super) fn own_fill(element: &Element) -> bool {
    !matches!(
        own_property(element, "fill").as_deref(),
        Some(value) if value.trim().eq_ignore_ascii_case("none")
    )
}

/// Whether the element paints a stroke of its own (the SVG default is `none`).
pub(super) fn own_stroke(element: &Element) -> bool {
    is_stroked(own_property(element, "stroke").as_deref())
}

fn join_points<'a>(points: impl Iterator<Item = &'a (f32, f32)>) -> String {
    points
        .map(|(x, y)| format!("{},{}", fmt_num(*x), fmt_num(*y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Concatenate every subpath into one point string, **explicitly closing** each:
/// a subpath whose last point is not its first gets the first point appended, so
/// the bridges between subpaths cancel (see the module docs).
fn join_closed_points(subpaths: &[Subpath]) -> String {
    let mut points = Vec::new();
    for subpath in subpaths {
        points.extend(subpath.points.iter().copied());
        if let (Some(first), Some(last)) = (subpath.points.first(), subpath.points.last()) {
            if first != last {
                points.push(*first);
            }
        }
    }
    join_points(points.iter())
}

/// A fresh shape carrying the path's non-`d` presentation (including `fill-rule`,
/// which must be preserved verbatim — we never invent `evenodd`), the resolved
/// `stroke-width` and the leftover inline style.
fn shape_skeleton(
    element: &Element,
    stroke_width: &Option<String>,
    remaining_style: &Option<String>,
) -> Element {
    let mut shape = Element {
        name: String::new(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    for (attr, value) in &element.attrs {
        if matches!(attr.as_str(), "d" | "style" | "transform" | "id") {
            continue;
        }
        shape.set_attr(attr, value.clone());
    }
    if let Some(stroke_width) = stroke_width {
        shape.set_attr("stroke-width", stroke_width.clone());
    }
    if let Some(remaining_style) = remaining_style {
        shape.set_attr("style", remaining_style.clone());
    }
    shape
}
