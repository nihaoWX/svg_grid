use crate::dom::{Element, Node};
use crate::error::{Result, SvgGridError};
use crate::geom;
use crate::transform::{
    is_view_box, marker_units_are_user_space, note_overflow, scaled_finite, units_of, Definitions,
    Units,
};
use crate::warnings::Passthroughs;

pub fn normalize_svg(
    root: &mut Element,
    max_side: f32,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
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
    let factor = max_side / current_max_side;
    if !factor.is_finite() {
        // The canvas itself is degenerate (`width="1e-45"`) so every product would
        // be `inf`/`NaN`; the input is kept exactly as written and W7 names it.
        passthroughs.record(
            index,
            &root.name,
            "width",
            "non-finite normalization factor, input kept verbatim".to_string(),
        );
        return Ok(());
    }
    scale_element(root, factor, &defs, index, passthroughs);
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

fn scale_element(
    element: &mut Element,
    factor: f32,
    defs: &Definitions,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    if DEFINITION_TAGS.contains(&element.name.as_str()) {
        scale_definition(element, factor, defs, index, passthroughs);
        return;
    }

    scale_numeric_attrs(element, &GEOMETRY_ATTRS, factor, index, passthroughs);
    scale_style_lengths(element, &SCALED_STYLE_PROPS, factor, index, passthroughs);
    scale_view_box(element, factor, index, passthroughs);
    scale_points(element, factor, index, passthroughs);
    scale_path(element, factor, index, passthroughs);
    scale_rotate_transform(element, factor, index, passthroughs);
    scale_children(element, factor, defs, index, passthroughs);
}

fn scale_children(
    element: &mut Element,
    factor: f32,
    defs: &Definitions,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    for child in &mut element.children {
        if let Node::Element(child) = child {
            scale_element(child, factor, defs, index, passthroughs);
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
fn scale_definition(
    element: &mut Element,
    factor: f32,
    defs: &Definitions,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    // The definition's **own** `stroke-width`/`font-size` are inherited by its
    // body, so they live in the body's coordinate system, not the panel's. Under
    // `objectBoundingBox` they are fractions of the referencing box and under a
    // `viewBox` they are viewBox units; the renderer already maps both, so
    // scaling them here shrinks the outline twice (round-data/R49).
    if content_is_user_space(element, defs) {
        scale_numeric_attrs(
            element,
            &["font-size", "stroke-width"],
            factor,
            index,
            passthroughs,
        );
        scale_style_lengths(element, &SCALED_STYLE_PROPS, factor, index, passthroughs);
    }
    scale_rotate_transform(element, factor, index, passthroughs);

    match element.name.as_str() {
        "pattern" => {
            if defs.units(element, "patternUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse
            {
                scale_numeric_attrs(
                    element,
                    &["x", "y", "width", "height"],
                    factor,
                    index,
                    passthroughs,
                );
            }
            scale_view_box(element, factor, index, passthroughs);
            let verbatim = defs.units(element, "patternContentUnits", Units::UserSpaceOnUse)
                == Units::ObjectBoundingBox
                || has_view_box(element);
            if !verbatim {
                scale_children(element, factor, defs, index, passthroughs);
            }
        }
        "marker" => {
            let user_space = marker_units_are_user_space(element);
            if user_space {
                scale_numeric_attrs(
                    element,
                    &["markerWidth", "markerHeight", "refX", "refY"],
                    factor,
                    index,
                    passthroughs,
                );
            }
            scale_view_box(element, factor, index, passthroughs);
            if user_space && !has_view_box(element) {
                scale_children(element, factor, defs, index, passthroughs);
            }
        }
        "mask" => {
            if units_of(element, "maskUnits", Units::ObjectBoundingBox) == Units::UserSpaceOnUse {
                scale_numeric_attrs(
                    element,
                    &["x", "y", "width", "height"],
                    factor,
                    index,
                    passthroughs,
                );
            }
            if units_of(element, "maskContentUnits", Units::UserSpaceOnUse) == Units::UserSpaceOnUse
            {
                scale_children(element, factor, defs, index, passthroughs);
            }
        }
        "filter" => {
            if units_of(element, "filterUnits", Units::ObjectBoundingBox) == Units::UserSpaceOnUse {
                scale_numeric_attrs(
                    element,
                    &["x", "y", "width", "height"],
                    factor,
                    index,
                    passthroughs,
                );
            }
            // Filter primitives are in `primitiveUnits` (user space by default);
            // their own lengths were already rewritten before this round.
            scale_children(element, factor, defs, index, passthroughs);
        }
        "linearGradient"
            if defs.units(element, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            scale_numeric_attrs(
                element,
                &["x1", "y1", "x2", "y2"],
                factor,
                index,
                passthroughs,
            );
        }
        "radialGradient"
            if defs.units(element, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            scale_numeric_attrs(
                element,
                &["cx", "cy", "r", "fx", "fy", "fr"],
                factor,
                index,
                passthroughs,
            );
        }
        "clipPath"
            if units_of(element, "clipPathUnits", Units::UserSpaceOnUse)
                == Units::UserSpaceOnUse =>
        {
            scale_children(element, factor, defs, index, passthroughs);
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
fn scale_style_lengths(
    element: &mut Element,
    props: &[&str],
    factor: f32,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    let Some(style) = element.attr("style").map(str::to_string) else {
        return;
    };
    let mut current = style;
    let mut changed = false;
    for prop in props {
        let (next, kept) = crate::style::scale_length_report(&current, prop, factor);
        if let Some(next) = next {
            current = next;
            changed = true;
        }
        // A token the style helper left verbatim (value or product not finite)
        // speaks the same way as the attribute path (round-data/R61 D).
        for token in kept {
            passthroughs.record(
                index,
                &element.name,
                prop,
                format!("style token {token:?} kept verbatim: not finite"),
            );
        }
    }
    if changed {
        element.set_attr("style", current);
    }
}

fn scale_numeric_attrs(
    element: &mut Element,
    attrs: &[&str],
    factor: f32,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    for attr in attrs {
        let Some(value) = element.attr(attr).map(str::to_string) else {
            continue;
        };
        let Some((number_end, number)) = numeric_prefix(&value) else {
            continue;
        };
        match scaled_finite(number, factor) {
            Some(scaled) => element.set_attr(
                attr,
                format!("{}{}", crate::format_number(scaled), &value[number_end..]),
            ),
            None => note_overflow(passthroughs, index, element, attr, &value, factor),
        }
    }
}

/// Split a length attribute into its numeric prefix and the verbatim suffix.
/// svglite writes `textLength="102.4264px"` (a unit suffix we must preserve), and
/// the same rule is the right one for `width`, `height`, `r`, … . A token with no
/// numeric prefix (`"auto"`, `"middle"`) is left alone.
fn numeric_prefix(value: &str) -> Option<(usize, f32)> {
    // Longest prefix (by character boundary) that parses as a number.
    // `char_indices` never yields `value.len()`, so the full-value case is
    // handled separately.
    match value.parse::<f32>() {
        Ok(number) => Some((value.len(), number)),
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
            best
        }
    }
}

fn scale_view_box(
    element: &mut Element,
    factor: f32,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    let Some(view_box) = element.attr("viewBox").map(str::to_string) else {
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
    let mut scaled = [0.0f32; 4];
    for (slot, value) in scaled.iter_mut().zip(&parsed) {
        let Some(product) = scaled_finite(*value, factor) else {
            note_overflow(passthroughs, index, element, "viewBox", &view_box, factor);
            return;
        };
        *slot = product;
    }
    element.set_attr(
        "viewBox",
        format!(
            "{} {} {} {}",
            crate::format_number(scaled[0]),
            crate::format_number(scaled[1]),
            crate::format_number(scaled[2]),
            crate::format_number(scaled[3])
        ),
    );
}

fn scale_points(element: &mut Element, factor: f32, index: usize, passthroughs: &mut Passthroughs) {
    let Some(points) = element.attr("points").map(str::to_string) else {
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
        let (Some(x), Some(y)) = (scaled_finite(x, factor), scaled_finite(y, factor)) else {
            // One non-finite vertex invalidates the whole list: keep it verbatim.
            note_overflow(passthroughs, index, element, "points", &points, factor);
            return;
        };
        out.push(format!(
            "{},{}",
            crate::format_number(x),
            crate::format_number(y)
        ));
    }
    element.set_attr("points", out.join(" "));
}

/// Scale every coordinate of a `<path d>` by the same uniform factor as the
/// other primitives, so its geometry keeps pace with the rescaled `viewBox`. A
/// uniform factor means arcs stay arcs (see `crate::path`); an unparseable `d`
/// is left untouched, exactly like `scale_points`.
fn scale_path(element: &mut Element, factor: f32, index: usize, passthroughs: &mut Passthroughs) {
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
    match crate::path::rewrite(&d, &map) {
        Ok(rewritten) => element.set_attr("d", rewritten),
        Err(error) if error == crate::path::NON_FINITE => {
            passthroughs.record(
                index,
                &element.name,
                "d",
                format!("scaled path is not finite, kept verbatim: {d:?}"),
            );
        }
        Err(_) => {}
    }
}

fn scale_rotate_transform(
    element: &mut Element,
    factor: f32,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    let Some(transform) = element.attr("transform").map(str::to_string) else {
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
    let (Some(x), Some(y)) = (scaled_finite(x, factor), scaled_finite(y, factor)) else {
        note_overflow(
            passthroughs,
            index,
            element,
            "transform",
            &transform,
            factor,
        );
        return;
    };
    element.set_attr(
        "transform",
        format!(
            "rotate({}, {}, {})",
            parts[0],
            crate::format_number(x),
            crate::format_number(y)
        ),
    );
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
