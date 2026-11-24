//! The plain shapes: `<circle>`, `<text>`, `<tspan>`, `<use>`, `<line>`, `<rect>`,
//! `<image>`, `<polyline>`/`<polygon>` and a `<rect>`'s `rotate(...)` (see `super`).

use crate::dom::Element;
use crate::error::Result;
use crate::geom::{parse_f32, Rect};
use crate::warnings::Passthroughs;

use super::lengths::{
    length_factor, length_scale_factor, map_point, note_non_finite, note_overflow,
    rewrite_scaled_length, scaled_finite, LengthAxis,
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

/// `<ellipse>`: the centre is a coordinate pair, `rx` follows the x axis and `ry`
/// the y axis, exactly like `<circle>`'s `cx/cy/r` split across two radii.
///
/// One SVG rule makes this more than three rewrites: when only one radius is
/// given, the other is **equal to it**. Writing back only the spelled radius would
/// therefore leave the implicit one at the old value — an ellipse of the wrong
/// shape as soon as the axes scale differently (measured at s=8 on `rx` alone:
/// 修前 xor 20656/255 with both radii at 10, 修后 `rx="80"` `ry="80"` xor 0 —
/// round-data/R62). The missing radius is materialised whenever the policy scales
/// *any* axis: with the
/// factor of the axis it belongs to, or factor 1 when that axis is not scaled
/// (`data-scale="x"`: `rx` grows, `ry` keeps the source value, which is exactly
/// "equal to the *old* rx"). A policy that scales nothing (`position`/`none`)
/// leaves the pair alone so untouched input keeps its bytes.
///
/// A radius spelled only in inline `style` cannot be materialised safely: the
/// renderer derives the missing radius from the style value, so writing an
/// attribute would invent geometry the source does not show (measured: d1render
/// draws neither spelling, xor 0/0 before and after — round-data/R62 registers the
/// case). Both radii therefore stay verbatim.
#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_ellipse(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let (radius_x, radius_y) = radii(element);
    rewrite_pair(
        element,
        "cx",
        "cy",
        policy,
        source,
        target,
        index,
        passthroughs,
    )?;
    // Exactly one radius, spelled in `style`: leave both verbatim rather than
    // materialise geometry the source's own renderer does not show (see above).
    let style_only = match (&radius_x, &radius_y) {
        (Some(radius), None) | (None, Some(radius)) => radius.style,
        _ => false,
    };
    if style_only {
        return Ok(());
    }
    rewrite_scaled_length(
        element,
        "rx",
        policy,
        source,
        target,
        LengthAxis::X,
        index,
        passthroughs,
    )?;
    rewrite_scaled_length(
        element,
        "ry",
        policy,
        source,
        target,
        LengthAxis::Y,
        index,
        passthroughs,
    )?;
    let Some((radius, missing, axis)) = implied_radius(&radius_x, &radius_y) else {
        return Ok(());
    };
    write_implied_radius(
        element,
        radius,
        missing,
        axis,
        policy,
        source,
        target,
        index,
        passthroughs,
    )
}

/// One radius as it is spelled, and in which channel.
struct Radius {
    raw: String,
    style: bool,
}

/// The `rx`/`ry` the renderer will use: the inline `style` wins over the
/// presentation attribute for the same property, so the style is asked first.
fn radii(element: &Element) -> (Option<Radius>, Option<Radius>) {
    let style = element.attr("style").unwrap_or_default();
    let from_style =
        |property: &str| style_length(style, property).map(|raw| Radius { raw, style: true });
    let from_attribute = |property: &str| {
        element.attr(property).map(|raw| Radius {
            raw: raw.to_string(),
            style: false,
        })
    };
    (
        from_style("rx").or_else(|| from_attribute("rx")),
        from_style("ry").or_else(|| from_attribute("ry")),
    )
}

/// The value of `property` in an inline `style`, if declared.
fn style_length(style: &str, property: &str) -> Option<String> {
    style.split(';').find_map(|declaration| {
        let (name, value) = declaration.split_once(':')?;
        (name.trim() == property).then(|| value.trim().to_string())
    })
}

/// The one radius whose value the other inherits, with the attribute to write and
/// the axis that attribute belongs to.
fn implied_radius<'a>(
    radius_x: &'a Option<Radius>,
    radius_y: &'a Option<Radius>,
) -> Option<(&'a Radius, &'static str, LengthAxis)> {
    match (radius_x, radius_y) {
        (Some(radius), None) => Some((radius, "ry", LengthAxis::Y)),
        (None, Some(radius)) => Some((radius, "rx", LengthAxis::X)),
        _ => None,
    }
}

/// `rx` and `ry` are an equal pair when one of them is absent; copy the present
/// value onto the missing axis, scaled by that axis's factor.
#[allow(clippy::too_many_arguments)]
fn write_implied_radius(
    element: &mut Element,
    radius: &Radius,
    missing: &str,
    axis: LengthAxis,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let factor = length_scale_factor(policy, source, target, axis);
    let other = match axis {
        LengthAxis::Y => LengthAxis::X,
        _ => LengthAxis::Y,
    };
    // A policy that scales neither axis keeps the implicit pair correct as is.
    if factor.is_none() && length_scale_factor(policy, source, target, other).is_none() {
        return Ok(());
    }
    let factor = factor.unwrap_or(1.0);
    let spelled = if missing == "ry" { "rx" } else { "ry" };
    let value = parse_f32(&radius.raw, spelled)?;
    if !value.is_finite() {
        note_non_finite(passthroughs, index, element, missing, &radius.raw, value);
        return Ok(());
    }
    match scaled_finite(value, factor) {
        Some(scaled) => {
            element.set_attr(missing, crate::format_number(scaled));
        }
        None => note_overflow(passthroughs, index, element, missing, &radius.raw, factor),
    }
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
