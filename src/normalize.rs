use crate::dom::{Element, Node};
use crate::error::{Result, SvgGridError};
use crate::geom;
use crate::transform::{is_view_box, marker_units_are_user_space, units_of, Definitions, Units};

pub fn normalize_svg(root: &mut Element, max_side: f32) -> Result<()> {
    if max_side <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "input normalization max side must be greater than zero".to_string(),
        ));
    }

    let canvas = geom::find_canvas_box(root)?;
    let current_max_side = canvas.width.max(canvas.height);
    if current_max_side <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "input SVG canvas must have positive width and height".to_string(),
        ));
    }

    let defs = Definitions::collect(root);
    scale_element(root, max_side / current_max_side, &defs);
    Ok(())
}

/// Presentational length properties that may be declared inside `style="..."`.
///
/// Both dialects we consume put scale-relevant lengths there (matplotlib:
/// `font-size`/`stroke-width`; svglite: `font-size`), so a normalization that only
/// touched attributes silently broke the "text : geometry" ratio. Scaling these by
/// the *same* factor as the attributes keeps the normalization uniform.
const SCALED_STYLE_PROPS: [&str; 4] = [
    "font-size",
    "stroke-width",
    "stroke-dasharray",
    "stroke-dashoffset",
];

/// Geometry that is a user-unit length no matter which element carries it.
const GEOMETRY_ATTRS: [&str; 16] = [
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "width",
    "height",
    "textLength",
    "font-size",
    "stroke-width",
];

/// Definition tags whose numbers may be **fractions of a box** instead of user
/// units. Their content is scaled by the renderer on its own, so multiplying those
/// fractions here shrinks the motif twice (round-data/R46 item D: ink 4800 → 1200).
const DEFINITION_TAGS: [&str; 7] = [
    "pattern",
    "marker",
    "mask",
    "filter",
    "linearGradient",
    "radialGradient",
    "clipPath",
];

fn scale_element(element: &mut Element, factor: f32, defs: &Definitions) {
    if DEFINITION_TAGS.contains(&element.name.as_str()) {
        scale_definition(element, factor, defs);
        return;
    }

    scale_numeric_attrs(element, &GEOMETRY_ATTRS, factor);
    scale_style_lengths(element, &SCALED_STYLE_PROPS, factor);
    scale_view_box(element, factor);
    scale_points(element, factor);
    scale_path(element, factor);
    scale_rotate_transform(element, factor);
    scale_children(element, factor, defs);
}

fn scale_children(element: &mut Element, factor: f32, defs: &Definitions) {
    for child in &mut element.children {
        if let Node::Element(child) = child {
            scale_element(child, factor, defs);
        }
    }
}

/// Scale a definition only where its numbers are user units.
///
/// Mirrors the renderer's own reading of each `*Units` attribute
/// (`usvg-0.47.0/src/parser/`): `patternUnits`, `patternContentUnits`,
/// `gradientUnits` and `filterUnits` are resolved along the `href` chain, while
/// `maskUnits`, `maskContentUnits` and `markerUnits` are read off the element
/// itself (`mask.rs:20-30`, `marker.rs:222`). A `viewBox` body is already mapped
/// onto the tile/viewport by the renderer, so it must stay verbatim.
fn scale_definition(element: &mut Element, factor: f32, defs: &Definitions) {
    // The definition's **own** `stroke-width`/`font-size` are inherited by its
    // body, so they live in the body's coordinate system, not the panel's. Under
    // `objectBoundingBox` they are fractions of the referencing box and under a
    // `viewBox` they are viewBox units; the renderer already maps both, so
    // scaling them here shrinks the outline twice (round-data/R49).
    if content_is_user_space(element, defs) {
        scale_numeric_attrs(element, &["font-size", "stroke-width"], factor);
        scale_style_lengths(element, &SCALED_STYLE_PROPS, factor);
    }
    scale_rotate_transform(element, factor);

    match element.name.as_str() {
        "pattern" => {
            if defs.units(element, "patternUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse
            {
                scale_numeric_attrs(element, &["x", "y", "width", "height"], factor);
            }
            scale_view_box(element, factor);
            let verbatim = defs.units(element, "patternContentUnits", Units::UserSpaceOnUse)
                == Units::ObjectBoundingBox
                || has_view_box(element);
            if !verbatim {
                scale_children(element, factor, defs);
            }
        }
        "marker" => {
            let user_space = marker_units_are_user_space(element);
            if user_space {
                scale_numeric_attrs(
                    element,
                    &["markerWidth", "markerHeight", "refX", "refY"],
                    factor,
                );
            }
            scale_view_box(element, factor);
            if user_space && !has_view_box(element) {
                scale_children(element, factor, defs);
            }
        }
        "mask" => {
            if units_of(element, "maskUnits", Units::ObjectBoundingBox) == Units::UserSpaceOnUse {
                scale_numeric_attrs(element, &["x", "y", "width", "height"], factor);
            }
            if units_of(element, "maskContentUnits", Units::UserSpaceOnUse) == Units::UserSpaceOnUse
            {
                scale_children(element, factor, defs);
            }
        }
        "filter" => {
            if units_of(element, "filterUnits", Units::ObjectBoundingBox) == Units::UserSpaceOnUse {
                scale_numeric_attrs(element, &["x", "y", "width", "height"], factor);
            }
            // Filter primitives are in `primitiveUnits` (user space by default);
            // their own lengths were already rewritten before this round.
            scale_children(element, factor, defs);
        }
        "linearGradient"
            if defs.units(element, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            scale_numeric_attrs(element, &["x1", "y1", "x2", "y2"], factor);
        }
        "radialGradient"
            if defs.units(element, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            scale_numeric_attrs(element, &["cx", "cy", "r", "fx", "fy", "fr"], factor);
        }
        "clipPath"
            if units_of(element, "clipPathUnits", Units::UserSpaceOnUse)
                == Units::UserSpaceOnUse =>
        {
            scale_children(element, factor, defs);
        }
        _ => {}
    }
}

/// Whether a definition's body is drawn in **user space**, so its own inherited
/// lengths follow the panel. `objectBoundingBox` bodies are fractions of the
/// referencing box and `viewBox` bodies are viewBox units; both are mapped by the
/// renderer and must stay verbatim (round-data/R49).
///
/// The units are read the way `usvg-0.47.0` reads them: `patternContentUnits`
/// and `primitiveUnits` along the `href` chain (`paint_server.rs:169`,
/// `filter.rs:171`), `maskContentUnits` and `clipPathUnits` off the element
/// itself (`mask.rs:20-30`, `clippath.rs:23-25`), and `markerUnits` as the
/// literal `userSpaceOnUse` token (`marker.rs:222`). A gradient has no content
/// domain, so it keeps the old unconditional behaviour.
fn content_is_user_space(element: &Element, defs: &Definitions) -> bool {
    match element.name.as_str() {
        "pattern" => {
            !has_view_box(element)
                && defs.units(element, "patternContentUnits", Units::UserSpaceOnUse)
                    == Units::UserSpaceOnUse
        }
        "marker" => marker_units_are_user_space(element),
        "mask" => {
            units_of(element, "maskContentUnits", Units::UserSpaceOnUse) == Units::UserSpaceOnUse
        }
        "filter" => {
            defs.units(element, "primitiveUnits", Units::UserSpaceOnUse) == Units::UserSpaceOnUse
        }
        "clipPath" => {
            units_of(element, "clipPathUnits", Units::UserSpaceOnUse) == Units::UserSpaceOnUse
        }
        _ => true,
    }
}

/// Whether the element carries a `viewBox` the renderer honours.
fn has_view_box(element: &Element) -> bool {
    element.attr("viewBox").is_some_and(is_view_box)
}

/// Scale the numeric length of every presentational property in the element's
/// inline `style="..."`, leaving everything else (order, spacing, unrelated
/// declarations) untouched.
fn scale_style_lengths(element: &mut Element, props: &[&str], factor: f32) {
    let Some(style) = element.attr("style").map(str::to_string) else {
        return;
    };
    let mut current = style;
    let mut changed = false;
    for prop in props {
        if let Some(next) = crate::style::scale_length(&current, prop, factor) {
            current = next;
            changed = true;
        }
    }
    if changed {
        element.set_attr("style", current);
    }
}

fn scale_numeric_attrs(element: &mut Element, attrs: &[&str], factor: f32) {
    for attr in attrs {
        let Some(value) = element.attr(attr) else {
            continue;
        };
        let Some(scaled) = scale_numeric_prefix(value, factor) else {
            continue;
        };
        element.set_attr(attr, scaled);
    }
}

/// Multiply the numeric prefix of a length attribute by `factor`, keeping
/// whatever follows verbatim. svglite writes `textLength="102.4264px"` (a unit
/// suffix we must preserve), and the same rule is the right one for `width`,
/// `height`, `r`, … : a bare `"100"` scales as before, `"100px"` becomes
/// `"200px"`, and a token with no numeric prefix (`"auto"`, `"middle"`) is left
/// alone. Mirrors [`crate::style::scale_token`].
fn scale_numeric_prefix(value: &str, factor: f32) -> Option<String> {
    // Longest prefix (by character boundary) that parses as a number.
    // `char_indices` never yields `value.len()`, so the full-value case is
    // handled separately.
    let (number_end, number) = match value.parse::<f32>() {
        Ok(number) => (value.len(), number),
        Err(_) => {
            let mut best = None;
            for (index, _) in value.char_indices() {
                if index == 0 {
                    continue;
                }
                if let Ok(number) = value[..index].parse::<f32>() {
                    best = Some((index, number));
                }
            }
            best?
        }
    };
    Some(format!(
        "{}{}",
        crate::format_number(number * factor),
        &value[number_end..]
    ))
}

fn scale_view_box(element: &mut Element, factor: f32) {
    let Some(view_box) = element.attr("viewBox") else {
        return;
    };
    let values: Vec<_> = view_box.split_whitespace().collect();
    if values.len() != 4 {
        return;
    }
    let parsed: Option<Vec<f32>> = values
        .iter()
        .map(|value| value.parse::<f32>().ok())
        .collect();
    let Some(parsed) = parsed else {
        return;
    };
    element.set_attr(
        "viewBox",
        format!(
            "{} {} {} {}",
            crate::format_number(parsed[0] * factor),
            crate::format_number(parsed[1] * factor),
            crate::format_number(parsed[2] * factor),
            crate::format_number(parsed[3] * factor)
        ),
    );
}

fn scale_points(element: &mut Element, factor: f32) {
    let Some(points) = element.attr("points") else {
        return;
    };
    let mut out = Vec::new();
    for pair in points.split_whitespace() {
        let Some((x, y)) = pair.split_once(',') else {
            return;
        };
        let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) else {
            return;
        };
        out.push(format!(
            "{},{}",
            crate::format_number(x * factor),
            crate::format_number(y * factor)
        ));
    }
    element.set_attr("points", out.join(" "));
}

/// Scale every coordinate of a `<path d>` by the same uniform factor as the
/// other primitives, so its geometry keeps pace with the rescaled `viewBox`. A
/// uniform factor means arcs stay arcs (see `crate::path`); an unparseable `d`
/// is left untouched, exactly like `scale_points`.
fn scale_path(element: &mut Element, factor: f32) {
    let Some(d) = element.attr("d").map(str::to_string) else {
        return;
    };
    let map = crate::path::AxisMap {
        x_scale: factor,
        y_scale: factor,
        x_offset: 0.0,
        y_offset: 0.0,
        x_length: factor,
        y_length: factor,
    };
    if let Ok(rewritten) = crate::path::rewrite(&d, &map) {
        element.set_attr("d", rewritten);
    }
}

fn scale_rotate_transform(element: &mut Element, factor: f32) {
    let Some(transform) = element.attr("transform") else {
        return;
    };
    let Some(inner) = transform
        .strip_prefix("rotate(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return;
    };
    let parts: Vec<_> = inner
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return;
    }
    let (Ok(x), Ok(y)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>()) else {
        return;
    };
    element.set_attr(
        "transform",
        format!(
            "rotate({}, {}, {})",
            parts[0],
            crate::format_number(x * factor),
            crate::format_number(y * factor)
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{parse, serialize};

    fn normalized(svg: &str, max_side: f32) -> String {
        let mut root = parse(svg).unwrap();
        normalize_svg(&mut root, max_side).unwrap();
        serialize(&root).unwrap()
    }

    #[test]
    fn an_object_bounding_box_pattern_keeps_its_own_stroke_width() {
        let text = normalized(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
                  <pattern id="h" width="0.25" height="0.25"
                           patternContentUnits="objectBoundingBox" stroke-width="0.1">
                    <rect x="0" y="0" width="0.1" height="0.1" fill="none" stroke="#000"/>
                  </pattern>
                  <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
                </svg>"##,
            50.0,
        );
        assert!(text.contains(r#"stroke-width="0.1""#), "{text}");
        assert!(!text.contains(r#"stroke-width="0.050""#), "{text}");
    }

    #[test]
    fn a_user_space_pattern_scales_its_own_stroke_width() {
        let text = normalized(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
                  <pattern id="h" patternUnits="userSpaceOnUse"
                           patternContentUnits="userSpaceOnUse" x="0" y="0"
                           width="20" height="20" stroke-width="2">
                    <rect x="0" y="0" width="6" height="6" fill="none" stroke="#000"/>
                  </pattern>
                  <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
                </svg>"##,
            50.0,
        );
        assert!(text.contains(r#"stroke-width="1""#), "{text}");
    }

    #[test]
    fn a_pattern_with_a_view_box_keeps_its_own_stroke_width() {
        let text = normalized(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
                  <pattern id="h" patternUnits="userSpaceOnUse"
                           patternContentUnits="userSpaceOnUse" x="0" y="0"
                           width="20" height="20" viewBox="0 0 20 20" stroke-width="2">
                    <rect x="0" y="0" width="6" height="6" fill="none" stroke="#000"/>
                  </pattern>
                  <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
                </svg>"##,
            50.0,
        );
        assert!(text.contains(r#"stroke-width="2""#), "{text}");
    }
}
