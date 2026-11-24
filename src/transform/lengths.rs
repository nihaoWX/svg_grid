//! Scale factors and the lengths they apply to: the shared arithmetic of the
//! `data-scale` policy (see `super`).

use crate::dom::Element;
use crate::error::Result;
use crate::geom::{parse_f32, Rect};
use crate::warnings::Passthroughs;

use super::ScalePolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LengthAxis {
    X,
    Y,
    Uniform,
    /// A light source's `z`/`pointsAtZ`. The renderer measures the distance in
    /// filter space with the **RMS** of the two axis factors,
    /// `sqrt((sx² + sy²) / 2)` (`resvg-0.47.0/src/filter/mod.rs`,
    /// `transform_light_source`), not the geometric mean a plain length uses:
    /// an anisotropic panel needs a different factor for z (round-data/R60).
    LightZ,
    /// `feDisplacementMap scale`. The renderer squares the number
    /// (`offset = (c - 0.5) · scale² · axis`), so the attribute takes the
    /// **square root** of the axis factor to displace by the axis factor.
    Displacement,
}

/// Note a well-formed but **non-finite** number (`nan`/`inf`) as kept verbatim
/// (warning W7) and return nothing: the caller keeps its existing computation, so
/// the bytes written for the attribute are exactly what they were before — the
/// only change is that stderr now says so. Structural garbage (`x="abc"`) is *not*
/// routed here: [`parse_f32`] still fails closed for it.
pub(super) fn note_non_finite(
    passthroughs: &mut Passthroughs,
    index: usize,
    element: &Element,
    attr: &str,
    raw: &str,
    value: f32,
) {
    if !value.is_finite() {
        passthroughs.record(
            index,
            &element.name,
            attr,
            format!("non-finite number {raw:?}"),
        );
    }
}

/// Scale a length that may be written either as a presentation *attribute* or as
/// a declaration inside the element's inline `style="..."` (both dialects put
/// `stroke-width` — and svglite `font-size` — in the style). The scale policy is
/// unchanged: `position`/`none` do not scale, `x`/`y`/`xy` follow the axis rule.
// One extra argument over clippy's limit: the W7 collector and the input index
// travel together with the scale policy (same shape as `flatten_arc`).
#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_scaled_length(
    element: &mut Element,
    attr: &str,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    axis: LengthAxis,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let factor = length_scale_factor(policy, source, target, axis);

    if let Some(raw) = element.attr(attr).map(str::to_string) {
        match factor {
            // A length the policy scales is parsed strictly (structural garbage
            // still fails closed) and rewritten; a non-finite value is flagged but
            // written exactly as before.
            Some(factor) => {
                let value = parse_f32(&raw, attr)?;
                if value.is_finite() {
                    element.set_attr(attr, crate::format_number(value * factor));
                } else {
                    // Never write `inf`/`NaN` back: the warning says the value is
                    // kept verbatim, and `inf` is not even a legal SVG number.
                    note_non_finite(passthroughs, index, element, attr, &raw, value);
                }
            }
            // The policy leaves the length untouched, so the value is already
            // verbatim — but a non-finite one (`r="nan"`, `stroke-width="inf"`)
            // must still speak up. Parse leniently here: no *new* hard error for
            // garbage the previous version also left alone under this policy.
            None => {
                if let Ok(value) = raw.parse::<f32>() {
                    note_non_finite(passthroughs, index, element, attr, &raw, value);
                }
            }
        }
    }

    if let Some(factor) = factor {
        if let Some(style) = element.attr("style").map(str::to_string) {
            if let Some(scaled) = crate::style::scale_length(&style, attr, factor) {
                element.set_attr("style", scaled);
            }
        }
    }

    Ok(())
}

pub(super) fn length_scale_factor(
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

    let (ax, ay) = (sx.abs(), sy.abs());
    match policy {
        ScalePolicy::X => match axis {
            LengthAxis::X | LengthAxis::Uniform => Some(ax),
            LengthAxis::Y => None,
            LengthAxis::LightZ => Some(rms(ax, 1.0)),
            LengthAxis::Displacement => Some(ax.sqrt()),
        },
        ScalePolicy::Y => match axis {
            LengthAxis::X => None,
            LengthAxis::Y | LengthAxis::Uniform => Some(ay),
            LengthAxis::LightZ => Some(rms(1.0, ay)),
            LengthAxis::Displacement => Some(ay.sqrt()),
        },
        ScalePolicy::Xy => match axis {
            LengthAxis::X => Some(ax),
            LengthAxis::Y => Some(ay),
            LengthAxis::Uniform => Some((ax * ay).sqrt()),
            LengthAxis::LightZ => Some(rms(ax, ay)),
            // One number for two axes: no single value is exact, and the value the
            // renderer squares is the geometric mean of the two exact values
            // (`super::filters` has the measurements).
            LengthAxis::Displacement => Some((ax * ay).sqrt().sqrt()),
        },
        // A pattern/marker body scales with the panel but keeps its own origin.
        ScalePolicy::Scale => match axis {
            LengthAxis::X => Some(ax),
            LengthAxis::Y => Some(ay),
            LengthAxis::Uniform | LengthAxis::LightZ => Some(if axis == LengthAxis::LightZ {
                rms(ax, ay)
            } else {
                (ax * ay).sqrt()
            }),
            LengthAxis::Displacement => Some((ax * ay).sqrt().sqrt()),
        },
        // The same, on one axis only: a body referenced from a `data-scale="x"`
        // group scales its x lengths and leaves its y lengths alone, or a square
        // motif becomes an oblong one (round-data/R46).
        ScalePolicy::ScaleX => match axis {
            LengthAxis::X | LengthAxis::Uniform => Some(ax),
            LengthAxis::Y => None,
            LengthAxis::LightZ => Some(rms(ax, 1.0)),
            LengthAxis::Displacement => Some(ax.sqrt()),
        },
        ScalePolicy::ScaleY => match axis {
            LengthAxis::X => None,
            LengthAxis::Y | LengthAxis::Uniform => Some(ay),
            LengthAxis::LightZ => Some(rms(1.0, ay)),
            LengthAxis::Displacement => Some(ay.sqrt()),
        },
        ScalePolicy::Position | ScalePolicy::None => None,
    }
}

/// The factor the renderer applies along z: the **root mean square** of the two
/// axis factors, `sqrt((sx² + sy²) / 2)`.
///
/// Spelled exactly as `resvg-0.47.0/src/filter/mod.rs::transform_light_source`
/// spells it (`(sx*sx + sy*sy).sqrt() / SQRT_2`) so the factor is bit-identical to
/// the one the oracle's `scale(...)` transform produces: the shading is sensitive
/// enough that three decimals in `z` already flip a handful of pixels.
fn rms(sx: f32, sy: f32) -> f32 {
    (sx * sx + sy * sy).sqrt() / std::f32::consts::SQRT_2
}

pub(super) fn map_point(
    x: f32,
    y: f32,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> (f32, f32) {
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
        // Scale only: a definition body keeps the tile's/marker's own origin.
        ScalePolicy::Scale => (x * sx, y * sy),
        ScalePolicy::ScaleX => (x * sx, y),
        ScalePolicy::ScaleY => (x, y * sy),
    }
}
