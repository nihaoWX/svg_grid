//! Filter primitive lengths: `primitiveUnits`, **not** `filterUnits`
//! (round-data/R58).
//!
//! `filterUnits` decides what the filter *region* (`x`/`y`/`width`/`height` on the
//! `<filter>`) means; `primitiveUnits` decides what the lengths *inside* the
//! primitives mean. They are independent attributes with **opposite** defaults, and
//! before R58 the code gated both on `filterUnits`:
//!
//! * `filterUnits` defaults to `objectBoundingBox` — the region is a fraction of
//!   the filtered element's box, which already follows the panel;
//! * `primitiveUnits` defaults to `userSpaceOnUse` (`usvg-0.47.0/src/parser/
//!   filter.rs:171`), so every user length inside a primitive has to follow the
//!   panel — and the default case is the one that *does* need scaling.
//!
//! Measured (source 100×80 → cell 200×160, s=2, oracle = the source rendered at 2):
//! `feGaussianBlur stdDeviation="2"` left verbatim is xor 8360 / maxdiff 89;
//! writing `4` is xor 0. The same gate error leaves `feOffset`, `feMorphology`,
//! `feDropShadow`, `feTurbulence`, the lighting lights, `feImage` and
//! `feDisplacementMap` unscaled.
//!
//! What is rewritten is exactly the set of user lengths the renderer reads in
//! primitive units (each measured at s=2: xor 0 after the rewrite):
//! `feGaussianBlur stdDeviation`, `feOffset dx/dy`, `feMorphology radius`,
//! `feDropShadow dx/dy/stdDeviation`, `feTurbulence baseFrequency` (**divided**),
//! `fePointLight x/y/z`, `feSpotLight x/y/z/pointsAtX/pointsAtY/pointsAtZ`
//! (round-data/R59: those six are lengths, not angles — only
//! `specularExponent`/`limitingConeAngle` are dimensionless), `feImage
//! x/y/width/height`, `feDisplacementMap scale`. Everything else stays:
//! `feConvolveMatrix`'s numbers are counts, kernel entries and a bias;
//! `feDistantLight azimuth/elevation` are angles; and
//! `feColorMatrix`/`feComponentTransfer`/`feComposite`/`feBlend`/`feMerge`/
//! `feFlood`/`feTile` carry no user length at all.
//!
//! A malformed primitive length (`stdDeviation="2px"`, `dx="abc"`) is **not**
//! fatal: the attribute is kept byte-for-byte and reported as W7, exactly like a
//! malformed `transform` (round-data/R59; R58 used to fail the whole run with
//! rc=2 here, which was a regression against the "malformed value is a warning"
//! rule in `ARCHITECTURE/warnings.md`).
//!
//! Anisotropy: an attribute that accepts **one or two** numbers (`stdDeviation`,
//! `radius`, `baseFrequency`) gets two values whenever the two axis factors differ —
//! measured (source 100×80 → cell 400×400, sx=4/sy=5): `stdDeviation` as `8 10` is
//! xor 0, as `8` xor 24644, as `10` xor 22936, as `8.944` (geometric mean) xor
//! 69016, i.e. the mean is the *worst* of the three. A single-number attribute
//! (`feDisplacementMap scale`) has no per-axis spelling and falls back to the
//! geometric mean. The lighting primitives cannot be made exact anisotropically
//! (the surface normal itself changes under a non-uniform scale; every candidate
//! measured xor ≈1.5e5), so they only scale uniformly and the residual is
//! registered instead of claimed fixed.

use super::lengths::{length_scale_factor, note_non_finite, LengthAxis};
use super::links::resolved_units;
use super::primitives::rewrite_axes;
use super::units::Units;
use super::walk::Context;
use super::ScalePolicy;
use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::parse_f32;
use crate::warnings::Passthroughs;

/// Whether the lengths inside this `<filter>`'s primitives are user units.
///
/// Resolved along `href` like the renderer (`filter` may inherit **any** attribute
/// from another `<filter>`, see [`super::links`]); an unparsable value falls back to
/// the `userSpaceOnUse` default, so a typo scales rather than silently staying.
pub(super) fn primitive_units_are_user_space(element: &Element, context: &Context<'_>) -> bool {
    resolved_units(
        element,
        context.links,
        "primitiveUnits",
        Units::UserSpaceOnUse,
    ) == Units::UserSpaceOnUse
}

/// Rewrite the lengths of every primitive inside a `<filter>`.
///
/// Called from the walk only when [`primitive_units_are_user_space`] says the
/// numbers are user units. `policy` is the filter's own policy, i.e. the policy of
/// the element that references it (definition policy), so a filter used from inside
/// a pattern body scales like that body.
pub(super) fn rewrite_primitives(
    element: &mut Element,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    for child in &mut element.children {
        if let Node::Element(child) = child {
            rewrite_one(child, policy, context, passthroughs)?;
        }
    }
    Ok(())
}

fn rewrite_one(
    element: &mut Element,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    match element.name.as_str() {
        "feGaussianBlur" => scale_pair(
            element,
            "stdDeviation",
            false,
            false,
            policy,
            context,
            passthroughs,
        )?,
        "feOffset" => {
            scale_axis(element, "dx", LengthAxis::X, policy, context, passthroughs)?;
            scale_axis(element, "dy", LengthAxis::Y, policy, context, passthroughs)?;
        }
        "feMorphology" => scale_pair(
            element,
            "radius",
            false,
            false,
            policy,
            context,
            passthroughs,
        )?,
        "feDropShadow" => {
            scale_axis(element, "dx", LengthAxis::X, policy, context, passthroughs)?;
            scale_axis(element, "dy", LengthAxis::Y, policy, context, passthroughs)?;
            scale_pair(
                element,
                "stdDeviation",
                false,
                false,
                policy,
                context,
                passthroughs,
            )?;
        }
        // A frequency is an inverse length: the panel grows, the wave gets longer,
        // so the number is *divided* by the axis factor. It is also the one filter
        // length the project's 3-decimal writer is too coarse for (see
        // [`scale_pair`]'s `precise`), so it keeps its full round-trip digits.
        "feTurbulence" => scale_pair(
            element,
            "baseFrequency",
            true,
            true,
            policy,
            context,
            passthroughs,
        )?,
        // The light sources are children of the lighting primitive and are the only
        // lengths that family owns (`surfaceScale`/`diffuseConstant`/
        // `specularExponent`/`lighting-color` are dimensionless).
        "feDiffuseLighting" | "feSpecularLighting" => {
            for child in &mut element.children {
                if let Node::Element(child) = child {
                    rewrite_one(child, policy, context, passthroughs)?;
                }
            }
        }
        "fePointLight" => {
            scale_axis(element, "x", LengthAxis::X, policy, context, passthroughs)?;
            scale_axis(element, "y", LengthAxis::Y, policy, context, passthroughs)?;
            scale_axis(
                element,
                "z",
                LengthAxis::Uniform,
                policy,
                context,
                passthroughs,
            )?;
        }
        // `feSpotLight` is a light *placed* in user space: `x`/`y`/`pointsAtX`/
        // `pointsAtY` are lengths along their own axis, `z`/`pointsAtZ` along z
        // (uniform, like `fePointLight`'s `z`). Only `specularExponent` and
        // `limitingConeAngle` are dimensionless; they are deliberately not listed.
        // R58 skipped the whole element as "angles", which left the light at the
        // unscaled panel's position (measured at s=2: xor 18003 / maxdiff 205).
        "feSpotLight" => {
            scale_axis(element, "x", LengthAxis::X, policy, context, passthroughs)?;
            scale_axis(element, "y", LengthAxis::Y, policy, context, passthroughs)?;
            scale_axis(
                element,
                "z",
                LengthAxis::Uniform,
                policy,
                context,
                passthroughs,
            )?;
            scale_axis(
                element,
                "pointsAtX",
                LengthAxis::X,
                policy,
                context,
                passthroughs,
            )?;
            scale_axis(
                element,
                "pointsAtY",
                LengthAxis::Y,
                policy,
                context,
                passthroughs,
            )?;
            scale_axis(
                element,
                "pointsAtZ",
                LengthAxis::Uniform,
                policy,
                context,
                passthroughs,
            )?;
        }
        // `x`/`y` place the referenced image in user space (they move with the
        // panel, like a `<rect>`'s position), `width`/`height` are sizes.
        "feImage" => {
            rewrite_axes(
                element,
                &[("x", false), ("y", true)],
                policy,
                context.source,
                context.target,
                context.index,
                passthroughs,
            )?;
            scale_axis(
                element,
                "width",
                LengthAxis::X,
                policy,
                context,
                passthroughs,
            )?;
            scale_axis(
                element,
                "height",
                LengthAxis::Y,
                policy,
                context,
                passthroughs,
            )?;
        }
        // One number for both axes: the geometric mean is the only spelling.
        "feDisplacementMap" => scale_axis(
            element,
            "scale",
            LengthAxis::Uniform,
            policy,
            context,
            passthroughs,
        )?,
        _ => {}
    }
    Ok(())
}

/// Scale a single-number length on one axis.
fn scale_axis(
    element: &mut Element,
    attr: &str,
    axis: LengthAxis,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(raw) = element.attr(attr).map(str::to_string) else {
        return Ok(());
    };
    let Some(factor) = length_scale_factor(policy, context.source, context.target, axis) else {
        return Ok(());
    };
    let Some(value) = parsed_or_kept(element, attr, &raw, context, passthroughs) else {
        return Ok(());
    };
    element.set_attr(attr, crate::format_number(value * factor));
    Ok(())
}

/// Parse a primitive length, or keep it verbatim and report W7.
///
/// A malformed number is data, not a structural error: a browser ignores the
/// declaration and keeps the rest of the figure, and rejecting the whole run for
/// it is the regression R59 fixes (R58 returned
/// `invalid input: invalid numeric SVG attribute stdDeviation="2px"`, rc=2).
/// The model is `transform`'s unparsable-value path (`super::transforms`).
fn parsed_or_kept(
    element: &Element,
    attr: &str,
    raw: &str,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Option<f32> {
    match parse_f32(raw, attr) {
        Ok(value) => {
            note_non_finite(passthroughs, context.index, element, attr, raw, value);
            Some(value)
        }
        Err(error) => {
            passthroughs.record(
                context.index,
                &element.name,
                attr,
                format!("unparsable number {raw:?}: {error}"),
            );
            None
        }
    }
}

/// Scale a `<number-optional-number>` length: one value means both axes.
///
/// `invert` divides instead of multiplying (frequencies). A value with more than
/// two numbers is not a `<number-optional-number>`: it is left verbatim rather than
/// guessed at.
fn scale_pair(
    element: &mut Element,
    attr: &str,
    invert: bool,
    precise: bool,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(raw) = element.attr(attr).map(str::to_string) else {
        return Ok(());
    };
    let tokens: Vec<&str> = raw
        .split([' ', ','])
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() || tokens.len() > 2 {
        return Ok(());
    }
    // A single-axis policy (`data-scale="x"` / `"y"`) scales one side and leaves
    // the other alone. The missing factor is **1**, not "do not rewrite": a scalar
    // filter length applies to both directions, so under `x` the value `2` has to
    // become the explicit pair `8 2` (source 100→400) — leaving it as `2` blurs the
    // y direction as if the panel had not grown (measured: xor 6024 at s=2; the
    // pair is xor 0, round-data/R59). Both factors absent (position/none) is the
    // only case that has nothing to rewrite.
    let fx_opt = length_scale_factor(policy, context.source, context.target, LengthAxis::X);
    let fy_opt = length_scale_factor(policy, context.source, context.target, LengthAxis::Y);
    if fx_opt.is_none() && fy_opt.is_none() {
        return Ok(());
    }
    let fx = fx_opt.unwrap_or(1.0);
    let fy = fy_opt.unwrap_or(1.0);
    if invert && (fx.abs() < f32::EPSILON || fy.abs() < f32::EPSILON) {
        return Ok(());
    }

    let convert = |value: f32, factor: f32| {
        if invert {
            value / factor
        } else {
            value * factor
        }
    };
    let Some(x) = parsed_or_kept(element, attr, tokens[0], context, passthroughs) else {
        return Ok(());
    };
    // One source value: the two numbers are equal by definition, so they only stay
    // one number when the scale is uniform. Under an anisotropic panel the single
    // value has to become an explicit pair (measured: the geometric mean is far
    // worse than the pair).
    if tokens.len() == 1 && (fx - fy).abs() < f32::EPSILON {
        element.set_attr(attr, format_length(convert(x, fx), precise));
        return Ok(());
    }
    let y = match tokens.get(1) {
        Some(raw_y) => match parsed_or_kept(element, attr, raw_y, context, passthroughs) {
            Some(y) => y,
            // A half-parsed pair is kept whole: the first number is not written
            // without the second one it was written with.
            None => return Ok(()),
        },
        None => x,
    };
    element.set_attr(
        attr,
        format!(
            "{} {}",
            format_length(convert(x, fx), precise),
            format_length(convert(y, fy), precise)
        ),
    );
    Ok(())
}

/// Format one rewritten filter length.
///
/// The project writes coordinates with at most three decimals (`format_number`);
/// that is ample for geometry but not for `feTurbulence baseFrequency`, which is a
/// *small* number: `0.05 / 4` came out as `0.013` (4% off) and the noise field then
/// stopped matching the oracle (measured: xor 68684). Writing the shortest
/// round-trip form gives `0.0125` and xor 2 (the noise itself is not bit
/// reproducible; md 1). Only frequencies ask for this.
fn format_length(value: f32, precise: bool) -> String {
    if precise {
        format!("{value}")
    } else {
        crate::format_number(value)
    }
}
