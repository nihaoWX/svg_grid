use crate::dom::{Element, Node};
use crate::error::{Result, SvgGridError};
use crate::geom;

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

    scale_element(root, max_side / current_max_side);
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

fn scale_element(element: &mut Element, factor: f32) {
    scale_numeric_attrs(
        element,
        &[
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
            "font-size",
            "stroke-width",
            "markerWidth",
            "markerHeight",
            "refX",
            "refY",
        ],
        factor,
    );
    scale_style_lengths(element, &SCALED_STYLE_PROPS, factor);
    scale_view_box(element, factor);
    scale_points(element, factor);
    scale_rotate_transform(element, factor);

    for child in &mut element.children {
        if let Node::Element(child) = child {
            scale_element(child, factor);
        }
    }
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
        let Ok(value) = value.parse::<f32>() else {
            continue;
        };
        element.set_attr(attr, crate::format_number(value * factor));
    }
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
