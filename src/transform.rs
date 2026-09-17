use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::{parse_f32, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScalePolicy {
    Xy,
    X,
    Y,
    Position,
    None,
}

pub fn transform_svg(root: &mut Element, source_panel: Rect, target_panel: Rect) -> Result<()> {
    transform_element(root, ScalePolicy::None, source_panel, target_panel)
}

fn transform_element(
    element: &mut Element,
    inherited: ScalePolicy,
    source: Rect,
    target: Rect,
) -> Result<()> {
    let policy = element
        .attr("data-scale")
        .map(parse_policy)
        .unwrap_or(inherited);

    rewrite_primitive(element, policy, source, target)?;

    for child in &mut element.children {
        if let Node::Element(child) = child {
            transform_element(child, policy, source, target)?;
        }
    }

    Ok(())
}

fn parse_policy(value: &str) -> ScalePolicy {
    match value {
        "xy" => ScalePolicy::Xy,
        "x" => ScalePolicy::X,
        "y" => ScalePolicy::Y,
        "position" => ScalePolicy::Position,
        "none" => ScalePolicy::None,
        _ => ScalePolicy::None,
    }
}

fn rewrite_primitive(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> Result<()> {
    match element.name.as_str() {
        "circle" => {
            rewrite_pair(element, "cx", "cy", policy, source, target)?;
            rewrite_scaled_length(element, "r", policy, source, target, LengthAxis::Uniform)?;
        }
        "text" => rewrite_pair(element, "x", "y", policy, source, target)?,
        "use" => {
            rewrite_pair(element, "x", "y", policy, source, target)?;
            rewrite_scaled_length(element, "width", policy, source, target, LengthAxis::X)?;
            rewrite_scaled_length(element, "height", policy, source, target, LengthAxis::Y)?;
        }
        "line" => {
            rewrite_pair(element, "x1", "y1", policy, source, target)?;
            rewrite_pair(element, "x2", "y2", policy, source, target)?;
        }
        "rect" => rewrite_rect(element, policy, source, target)?,
        // An `<image>` covers exactly the axis-aligned `x`/`y`/`width`/`height`
        // rectangle it is placed in, so it is rewritten like a `<rect>`: the
        // bitmap is stretched to the new box. The converter that produces it
        // writes `preserveAspectRatio="none"` so the stretch is not defeated by
        // the default `xMidYMid meet` letterboxing.
        "image" => rewrite_rect(element, policy, source, target)?,
        "polyline" | "polygon" => rewrite_points(element, policy, source, target)?,
        _ => {}
    }
    rewrite_scaled_length(
        element,
        "stroke-width",
        policy,
        source,
        target,
        LengthAxis::Uniform,
    )?;
    rewrite_rotate_transform(element, policy, source, target)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LengthAxis {
    X,
    Y,
    Uniform,
}

/// Scale a length that may be written either as a presentation *attribute* or as
/// a declaration inside the element's inline `style="..."` (both dialects put
/// `stroke-width` — and svglite `font-size` — in the style). The scale policy is
/// unchanged: `position`/`none` do not scale, `x`/`y`/`xy` follow the axis rule.
fn rewrite_scaled_length(
    element: &mut Element,
    attr: &str,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    axis: LengthAxis,
) -> Result<()> {
    let Some(factor) = length_scale_factor(policy, source, target, axis) else {
        return Ok(());
    };

    if let Some(value) = element.attr(attr) {
        let value = parse_f32(value, attr)?;
        element.set_attr(attr, crate::format_number(value * factor));
    }

    if let Some(style) = element.attr("style").map(str::to_string) {
        if let Some(scaled) = crate::style::scale_length(&style, attr, factor) {
            element.set_attr("style", scaled);
        }
    }

    Ok(())
}

fn length_scale_factor(
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    axis: LengthAxis,
) -> Option<f32> {
    let sx = if source.width.abs() < f32::EPSILON {
        1.0
    } else {
        target.width / source.width
    };
    let sy = if source.height.abs() < f32::EPSILON {
        1.0
    } else {
        target.height / source.height
    };

    match policy {
        ScalePolicy::X => match axis {
            LengthAxis::X | LengthAxis::Uniform => Some(sx.abs()),
            LengthAxis::Y => None,
        },
        ScalePolicy::Y => match axis {
            LengthAxis::X => None,
            LengthAxis::Y | LengthAxis::Uniform => Some(sy.abs()),
        },
        ScalePolicy::Xy => match axis {
            LengthAxis::X => Some(sx.abs()),
            LengthAxis::Y => Some(sy.abs()),
            LengthAxis::Uniform => Some((sx.abs() * sy.abs()).sqrt()),
        },
        ScalePolicy::Position | ScalePolicy::None => None,
    }
}

fn rewrite_pair(
    element: &mut Element,
    x_attr: &str,
    y_attr: &str,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> Result<()> {
    let Some(x_value) = element.attr(x_attr) else {
        return Ok(());
    };
    let Some(y_value) = element.attr(y_attr) else {
        return Ok(());
    };
    let x = parse_f32(x_value, x_attr)?;
    let y = parse_f32(y_value, y_attr)?;
    let (x, y) = map_point(x, y, policy, source, target);
    element.set_attr(x_attr, crate::format_number(x));
    element.set_attr(y_attr, crate::format_number(y));

    Ok(())
}

fn rewrite_rect(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> Result<()> {
    let x = parse_f32(element.attr("x").unwrap_or("0"), "x")?;
    let y = parse_f32(element.attr("y").unwrap_or("0"), "y")?;
    let width = parse_f32(element.attr("width").unwrap_or("0"), "width")?;
    let height = parse_f32(element.attr("height").unwrap_or("0"), "height")?;

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

fn rewrite_points(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> Result<()> {
    let Some(points) = element.attr("points") else {
        return Ok(());
    };
    let points = points.to_string();
    let mut out = Vec::new();

    for pair in points.split_whitespace() {
        let Some((x, y)) = pair.split_once(',') else {
            continue;
        };
        let x = parse_f32(x, "points.x")?;
        let y = parse_f32(y, "points.y")?;
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

fn rewrite_rotate_transform(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
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

fn map_point(x: f32, y: f32, policy: ScalePolicy, source: Rect, target: Rect) -> (f32, f32) {
    let sx = if source.width.abs() < f32::EPSILON {
        1.0
    } else {
        target.width / source.width
    };
    let sy = if source.height.abs() < f32::EPSILON {
        1.0
    } else {
        target.height / source.height
    };
    let translated_x = x + (target.x - source.x);
    let translated_y = y + (target.y - source.y);

    match policy {
        ScalePolicy::Xy | ScalePolicy::Position => (
            target.x + (x - source.x) * sx,
            target.y + (y - source.y) * sy,
        ),
        ScalePolicy::X => (target.x + (x - source.x) * sx, translated_y),
        ScalePolicy::Y => (translated_x, target.y + (y - source.y) * sy),
        ScalePolicy::None => (translated_x, translated_y),
    }
}
