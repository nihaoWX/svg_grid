//! The plain shapes: `<circle>`, `<text>`, `<tspan>`, `<use>`, `<line>`, `<rect>`,
//! `<image>`, `<polyline>`/`<polygon>` and a `<rect>`'s `rotate(...)` (see `super`).

use crate::dom::Element;
use crate::error::Result;
use crate::geom::{parse_f32, Rect};
use crate::warnings::Passthroughs;

use super::lengths::{map_point, note_non_finite};
use super::ScalePolicy;

#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_pair(
    element: &mut Element,
    x_attr: &str,
    y_attr: &str,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(x_raw) = element.attr(x_attr).map(str::to_string) else {
        return Ok(());
    };
    let Some(y_raw) = element.attr(y_attr).map(str::to_string) else {
        return Ok(());
    };
    let x = parse_f32(&x_raw, x_attr)?;
    let y = parse_f32(&y_raw, y_attr)?;
    note_non_finite(passthroughs, index, element, x_attr, &x_raw, x);
    note_non_finite(passthroughs, index, element, y_attr, &y_raw, y);
    let (x, y) = map_point(x, y, policy, source, target);
    element.set_attr(x_attr, crate::format_number(x));
    element.set_attr(y_attr, crate::format_number(y));

    Ok(())
}

/// Rewrite the `x` and `y` of a `<tspan>` *independently*: each present axis is
/// mapped on its own through the single-axis projection of [`map_point`], so a
/// run that declares only one coordinate (`<tspan x="10" dy="1.2em">`, the usual
/// multi-line form) is still moved. `dx`/`dy` are incremental offsets, not
/// positions, and are deliberately left untouched; when neither `x` nor `y` is
/// present the tspan simply follows the (already rewritten) `<text>` position.
pub(super) fn rewrite_axis_coordinates(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    for (attr, is_y) in [("x", false), ("y", true)] {
        let Some(raw) = element.attr(attr).map(str::to_string) else {
            continue;
        };
        let value = parse_f32(&raw, attr)?;
        note_non_finite(passthroughs, index, element, attr, &raw, value);
        let mapped = if is_y {
            map_point(0.0, value, policy, source, target).1
        } else {
            map_point(value, 0.0, policy, source, target).0
        };
        element.set_attr(attr, crate::format_number(mapped));
    }

    Ok(())
}

pub(super) fn rewrite_rect(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let x_raw = element.attr("x").unwrap_or("0").to_string();
    let y_raw = element.attr("y").unwrap_or("0").to_string();
    let width_raw = element.attr("width").unwrap_or("0").to_string();
    let height_raw = element.attr("height").unwrap_or("0").to_string();
    let x = parse_f32(&x_raw, "x")?;
    let y = parse_f32(&y_raw, "y")?;
    let width = parse_f32(&width_raw, "width")?;
    let height = parse_f32(&height_raw, "height")?;
    note_non_finite(passthroughs, index, element, "x", &x_raw, x);
    note_non_finite(passthroughs, index, element, "y", &y_raw, y);
    note_non_finite(passthroughs, index, element, "width", &width_raw, width);
    note_non_finite(passthroughs, index, element, "height", &height_raw, height);

    let (x0, y0) = map_point(x, y, policy, source, target);
    let (x1, y1) = match policy {
        ScalePolicy::Xy | ScalePolicy::X | ScalePolicy::Y => {
            map_point(x + width, y + height, policy, source, target)
        }
        ScalePolicy::Position | ScalePolicy::None => (x0 + width, y0 + height),
    };

    element.set_attr("x", crate::format_number(x0));
    element.set_attr("y", crate::format_number(y0));
    element.set_attr("width", crate::format_number((x1 - x0).abs()));
    element.set_attr("height", crate::format_number((y1 - y0).abs()));

    Ok(())
}

pub(super) fn rewrite_points(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(points) = element.attr("points") else {
        return Ok(());
    };
    let points = points.to_string();
    let mut out = Vec::new();

    for pair in points.split_whitespace() {
        let Some((x_raw, y_raw)) = pair.split_once(',') else {
            continue;
        };
        let x = parse_f32(x_raw, "points.x")?;
        let y = parse_f32(y_raw, "points.y")?;
        note_non_finite(passthroughs, index, element, "points", x_raw, x);
        note_non_finite(passthroughs, index, element, "points", y_raw, y);
        let (x, y) = map_point(x, y, policy, source, target);
        out.push(format!(
            "{},{}",
            crate::format_number(x),
            crate::format_number(y)
        ));
    }

    element.set_attr("points", out.join(" "));

    Ok(())
}

pub(super) fn rewrite_rotate_transform(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(transform) = element.attr("transform") else {
        return Ok(());
    };
    let transform = transform.to_string();
    let Some(inner) = transform
        .strip_prefix("rotate(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Ok(());
    };
    let parts: Vec<_> = inner
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return Ok(());
    }
    let angle = parts[0];
    let x = parse_f32(parts[1], "transform.rotate.x")?;
    let y = parse_f32(parts[2], "transform.rotate.y")?;
    note_non_finite(
        passthroughs,
        index,
        element,
        "transform.rotate.x",
        parts[1],
        x,
    );
    note_non_finite(
        passthroughs,
        index,
        element,
        "transform.rotate.y",
        parts[2],
        y,
    );
    let (x, y) = map_point(x, y, policy, source, target);
    element.set_attr(
        "transform",
        format!(
            "rotate({}, {}, {})",
            angle,
            crate::format_number(x),
            crate::format_number(y)
        ),
    );
    Ok(())
}
