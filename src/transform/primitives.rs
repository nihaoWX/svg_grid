//! The plain shapes: `<circle>`, `<text>`, `<tspan>`, `<use>`, `<line>`, `<rect>`,
//! `<image>`, `<polyline>`/`<polygon>` and a `<rect>`'s `rotate(...)` (see `super`).

use crate::dom::Element;
use crate::error::Result;
use crate::geom::{parse_f32, Rect};
use crate::warnings::Passthroughs;

use super::lengths::{
    length_factor, map_point, note_non_finite, note_overflow, rewrite_scaled_length, scaled_finite,
    LengthAxis,
};
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
    // A non-finite coordinate is kept verbatim: the pair is written together or not
    // at all, and `inf`/`NaN` are not legal lengths (round-data/R60).
    if !x.is_finite() || !y.is_finite() {
        return Ok(());
    }
    let (x, y) = map_point(x, y, policy, source, target);
    if !x.is_finite() || !y.is_finite() {
        // The pair is written together or not at all: a finite source coordinate
        // times an overflowing factor (or a non-finite factor) must not become
        // `inf`/`NaN` on either axis.
        for (attr, raw, mapped) in [(x_attr, &x_raw, x), (y_attr, &y_raw, y)] {
            if !mapped.is_finite() {
                note_overflow(
                    passthroughs,
                    index,
                    element,
                    attr,
                    raw,
                    length_factor(policy, source, target),
                );
            }
        }
        return Ok(());
    }
    element.set_attr(x_attr, crate::format_number(x));
    element.set_attr(y_attr, crate::format_number(y));

    Ok(())
}

/// Rewrite each of `axes` *independently*, each on its own single-axis projection
/// of [`map_point`], so an element that declares only one of a coordinate pair is
/// still moved.
///
/// Two callers need this rather than [`rewrite_pair`]: a `<tspan>` (`<tspan
/// x="10" dy="1.2em">`, the usual multi-line form, carries only one coordinate and
/// skipping it would misplace the run), and a gradient, whose coordinates inherit
/// along `href` one by one — a `<linearGradient href="#base" x1="10">` declares
/// only `x1` and a pair-wise rewrite would skip it entirely (round-data/R46).
///
/// The two projections are exactly the axes of the affine map, because every
/// policy here is separable, so a caller that does declare both coordinates gets
/// the same numbers [`rewrite_pair`] would produce.
#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_axes(
    element: &mut Element,
    axes: &[(&str, bool)],
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    for (attr, is_y) in axes {
        let Some(raw) = element.attr(attr).map(str::to_string) else {
            continue;
        };
        let value = parse_f32(&raw, attr)?;
        note_non_finite(passthroughs, index, element, attr, &raw, value);
        if !value.is_finite() {
            // Kept verbatim, as in `super::lengths::rewrite_scaled_length`.
            continue;
        }
        let mapped = if *is_y {
            map_point(0.0, value, policy, source, target).1
        } else {
            map_point(value, 0.0, policy, source, target).0
        };
        if !mapped.is_finite() {
            note_overflow(
                passthroughs,
                index,
                element,
                attr,
                &raw,
                length_factor(policy, source, target),
            );
            continue;
        }
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
    // A box with one non-finite side is kept as written: mapping the other three
    // would move geometry the renderer cannot draw anyway, and the original text is
    // what the warning promises.
    if ![x, y, width, height].iter().all(|value| value.is_finite()) {
        return Ok(());
    }

    let (x0, y0) = map_point(x, y, policy, source, target);
    let (x1, y1) = match policy {
        ScalePolicy::Xy
        | ScalePolicy::X
        | ScalePolicy::Y
        | ScalePolicy::Scale
        | ScalePolicy::ScaleX
        | ScalePolicy::ScaleY => map_point(x + width, y + height, policy, source, target),
        ScalePolicy::Position | ScalePolicy::None => (x0 + width, y0 + height),
    };

    let width = (x1 - x0).abs();
    let height = (y1 - y0).abs();
    if ![x0, y0, width, height]
        .iter()
        .all(|value| value.is_finite())
    {
        // One overflowing side invalidates the whole box (R60 review): mapping the
        // other three would move geometry the renderer cannot draw, and the width
        // of an `inf - inf` subtraction is `NaN`.
        let factor = length_factor(policy, source, target);
        for (attr, raw, writer) in [
            ("x", &x_raw, x0),
            ("y", &y_raw, y0),
            ("width", &width_raw, width),
            ("height", &height_raw, height),
        ] {
            if !writer.is_finite() {
                note_overflow(passthroughs, index, element, attr, raw, factor);
            }
        }
        return Ok(());
    }
    element.set_attr("x", crate::format_number(x0));
    element.set_attr("y", crate::format_number(y0));
    element.set_attr("width", crate::format_number(width));
    element.set_attr("height", crate::format_number(height));

    Ok(())
}

/// Rewrite the region of a definition that owns an **optional** `x`/`y`/`width`/
/// `height` -- a `<filter>` or a `<mask>` (round-data/R58).
///
/// Unlike a `<rect>`, a missing region attribute is **not** zero: the renderer
/// defaults it, or inherits it along `href` (a `<filter href="#base"/>` takes its
/// whole region from `base`). [`rewrite_rect`] fills every absent attribute with
/// `"0"` and writes it back, which turns such a definition into a 0-sized region:
/// the filtered/masked content disappears completely (measured: ink 0 instead of
/// 9588, and cairosvg refuses the produced document with "The SVG size is
/// undefined"). So this variant rewrites **only what the element itself spells**:
/// positions through [`rewrite_axes`] (they get the panel offset), sizes through
/// [`rewrite_scaled_length`] (they only scale). An absent attribute stays absent
/// and keeps whatever the renderer would have used.
pub(super) fn rewrite_region(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    rewrite_axes(
        element,
        &[("x", false), ("y", true)],
        policy,
        source,
        target,
        index,
        passthroughs,
    )?;
    for (attr, axis) in [("width", LengthAxis::X), ("height", LengthAxis::Y)] {
        rewrite_scaled_length(
            element,
            attr,
            policy,
            source,
            target,
            axis,
            index,
            passthroughs,
        )?;
    }
    Ok(())
}

/// The viewport of a `<marker>` whose `markerUnits` is `userSpaceOnUse`: the
/// reference box and the reference point are user lengths, so they follow the
/// panel like any other shape coordinate.
///
/// `usvg-0.47.0/src/parser/marker.rs:458-465` defaults `markerWidth`/
/// `markerHeight` to `3` and `refX`/`refY` to `0`, so an absent width/height is
/// not "nothing to scale": the renderer still clips the body to a 3×3 box, and
/// that box has to grow with the panel or the arrowhead is cut off. `refX`/`refY`
/// default to zero, and zero needs no rewriting.
pub(super) fn rewrite_marker_box(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    for (attr, axis, default) in [
        ("markerWidth", LengthAxis::X, Some(3.0_f32)),
        ("markerHeight", LengthAxis::Y, Some(3.0_f32)),
        ("refX", LengthAxis::X, None),
        ("refY", LengthAxis::Y, None),
    ] {
        if element.attr(attr).is_some() {
            rewrite_scaled_length(
                element,
                attr,
                policy,
                source,
                target,
                axis,
                index,
                passthroughs,
            )?;
        } else if let (Some(default), Some(factor)) = (
            default,
            super::lengths::length_scale_factor(policy, source, target, axis),
        ) {
            match scaled_finite(default, factor) {
                Some(scaled) => element.set_attr(attr, crate::format_number(scaled)),
                None => note_overflow(
                    passthroughs,
                    index,
                    element,
                    attr,
                    &default.to_string(),
                    factor,
                ),
            }
        }
    }
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
    let mut finite = true;

    for pair in points.split_whitespace() {
        let Some((x_raw, y_raw)) = pair.split_once(',') else {
            continue;
        };
        let x = parse_f32(x_raw, "points.x")?;
        let y = parse_f32(y_raw, "points.y")?;
        note_non_finite(passthroughs, index, element, "points", x_raw, x);
        note_non_finite(passthroughs, index, element, "points", y_raw, y);
        finite &= x.is_finite() && y.is_finite();
        let (x, y) = map_point(x, y, policy, source, target);
        finite &= x.is_finite() && y.is_finite();
        out.push(format!(
            "{},{}",
            crate::format_number(x),
            crate::format_number(y)
        ));
    }

    // One non-finite vertex invalidates the whole list; keep it verbatim.
    if !finite {
        note_overflow(
            passthroughs,
            index,
            element,
            "points",
            &points,
            length_factor(policy, source, target),
        );
        return Ok(());
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
    if !x.is_finite() || !y.is_finite() {
        // Kept verbatim; a rotation centre of `inf` is not a length we may write back.
        return Ok(());
    }
    let (x, y) = map_point(x, y, policy, source, target);
    if !x.is_finite() || !y.is_finite() {
        note_overflow(
            passthroughs,
            index,
            element,
            "transform",
            &transform,
            length_factor(policy, source, target),
        );
        return Ok(());
    }
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
