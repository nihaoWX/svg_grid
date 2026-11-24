//! Light sources: the one filter content that is in user space **whatever**
//! `primitiveUnits` says (round-data/R60).
//!
//! usvg keeps the six positions as plain `f32`
//! (`usvg-0.47.0/src/parser/filter.rs::convert_light_source` — it never looks at
//! `primitiveUnits`), and resvg maps them through the filter transform
//! (`resvg-0.47.0/src/filter/mod.rs::transform_light_source`):
//!
//! * `x`/`y`/`pointsAtX`/`pointsAtY` are **points**: `ts.map_point` first scales
//!   them and then moves them into the cell, so a light at `x="20"` in the second
//!   panel of a two-cell figure becomes `120` — scaling alone leaves it at `20`
//!   (measured xor 5277 / maxdiff 238 on the second cell);
//! * `z`/`pointsAtZ` are distances along z, multiplied by
//!   `(sx*sx + sy*sy).sqrt() / SQRT_2` — the **root mean square** of the axis
//!   factors. The geometric mean (`sqrt(sx*sy)`) is what R58/R59 wrote: on 4×5 it
//!   writes `44.721` and leaves xor 37011 / maxdiff 206, where the RMS writes
//!   `45.276928` and leaves xor 0 / maxdiff 0. Three decimals are not enough
//!   (`45.277` already differs on 5 pixels, maxdiff 1; the exact value differs on
//!   none), so the value goes out with the shortest round-trip digits, like
//!   `feTurbulence baseFrequency`;
//! * only `specularExponent`/`limitingConeAngle` are dimensionless and stay.

use super::filters::scale_axis_precise;
use super::lengths::{map_point, note_overflow, LengthAxis};
use super::primitives::rewrite_axes;
use super::walk::Context;
use super::ScalePolicy;
use crate::dom::Element;
use crate::error::Result;
use crate::warnings::Passthroughs;

/// Whether `name` is a light source (a user-space position, whatever
/// `primitiveUnits` says).
pub(super) fn is_light_source(name: &str) -> bool {
    matches!(name, "fePointLight" | "feSpotLight")
}

/// Whether `name` is a lighting primitive, whose only interesting content is the
/// light source inside it.
pub(super) fn is_lighting(name: &str) -> bool {
    matches!(name, "feDiffuseLighting" | "feSpecularLighting")
}

/// Rewrite one light source: its positions (translated), and for a spot light
/// `pointsAtX`/`pointsAtY`. `z`/`pointsAtZ` take the RMS factor and do not move.
pub(super) fn rewrite_light(
    element: &mut Element,
    name: &str,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let positions: &[(&str, bool)] = if name == "feSpotLight" {
        &[
            ("x", false),
            ("y", true),
            ("pointsAtX", false),
            ("pointsAtY", true),
        ]
    } else {
        &[("x", false), ("y", true)]
    };
    rewrite_positions(element, positions, policy, context, passthroughs)?;
    scale_axis_precise(
        element,
        "z",
        LengthAxis::LightZ,
        policy,
        context,
        passthroughs,
    )?;
    if name == "feSpotLight" {
        scale_axis_precise(
            element,
            "pointsAtZ",
            LengthAxis::LightZ,
            policy,
            context,
            passthroughs,
        )?;
    }
    Ok(())
}

/// Rewrite the horizontal positions of a light source: `x`/`y` and, for a spot
/// light, `pointsAtX`/`pointsAtY`.
///
/// These are user-space coordinates, so they **translate** as well as scale: a
/// light at `x="20"` in a figure whose second panel starts at 100 has to become
/// `120`, not `80`. An absent position is the renderer's default 0, which is still
/// translated — the moved panel's origin is not the figure's origin — so it is
/// written out when the mapped default is non-zero.
fn rewrite_positions(
    element: &mut Element,
    positions: &[(&str, bool)],
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let present: Vec<(&str, bool)> = positions
        .iter()
        .copied()
        .filter(|(attr, _)| element.attr(attr).is_some())
        .collect();
    if !present.is_empty() {
        rewrite_axes(
            element,
            &present,
            policy,
            context.source,
            context.target,
            context.index,
            passthroughs,
        )?;
    }

    let (dx, dy) = map_point(0.0, 0.0, policy, context.source, context.target);
    for (attr, is_y) in positions {
        if element.attr(attr).is_some() {
            continue;
        }
        let mapped = if *is_y { dy } else { dx };
        if !mapped.is_finite() {
            // The default position 0 still has to move into the cell; if the
            // translation itself is not finite, leave the default alone (W7).
            note_overflow(
                passthroughs,
                context.index,
                element,
                attr,
                "0",
                super::lengths::length_factor(policy, context.source, context.target),
            );
            continue;
        }
        if mapped != 0.0 {
            element.set_attr(attr, crate::format_number(mapped));
        }
    }
    Ok(())
}
