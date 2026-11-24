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
//! 69016, i.e. the mean is the *worst* of the three.
//!
//! An attribute that accepts only **one** number cannot say two things at once, and
//! the renderer does not even use it linearly:
//!
//! * `feDisplacementMap scale` is applied **quadratically** — the offset is
//!   `(channel - 0.5) · scale² · axis` (`resvg-0.47.0/src/filter/mod.rs`
//!   `apply_displacement_map` together with `displacement_map.rs`) — so the
//!   rewritten value is the original times the **square root** of the axis factor.
//!   Measured (`data-scale="x"`, sx=4, source `scale="10"`): `20` reproduces the
//!   oracle exactly (xor 0), while the linear `40` displaces four times too far
//!   (xor 6061 / maxdiff 255). Two different axis factors have no exact one-number
//!   answer; the fallback is the geometric mean of the two exact values,
//!   `scale · (sx·sy)^(1/4)` — for 4×5 that is `21.147` (the old `44.721`
//!   displaced 4.5× too far).
//! * A light's `z`/`pointsAtZ` is measured with the **RMS** of the two axis
//!   factors, `sqrt((sx² + sy²) / 2)` (`resvg-0.47.0/src/filter/mod.rs`
//!   `transform_light_source`), *not* the geometric mean a plain length uses.
//!   Measured on an anisotropic cell (source 100×80 → 400×400, sx=4/sy=5): the
//!   geometric mean writes `44.721` and leaves xor 37011 / maxdiff 206, the RMS
//!   writes `45.276928` and is xor 0 / maxdiff 0 (three decimals, `45.277`, would
//!   still differ on 5 pixels, maxdiff 1). R59 registered that residual as
//!   intrinsic ("the normal changes under a non-uniform scale"); it was this
//!   missing factor.
//!
//! The lights are also **positions in user space**, independent of
//! `primitiveUnits`: usvg keeps `fePointLight`/`feSpotLight`'s six numbers as plain
//! `f32` (`usvg-0.47.0/src/parser/filter.rs::convert_light_source`) and resvg maps
//! them through the filter transform. So `x`/`y`/`pointsAtX`/`pointsAtY`
//! **translate** with the panel — a two-cell figure puts the same light in the
//! second cell's coordinates (measured xor 5277 / maxdiff 238 before, 0 after) —
//! and they are rewritten even when `primitiveUnits="objectBoundingBox"` (measured
//! xor 100170 / maxdiff 242 before, 2 after). Only the genuinely box-relative
//! lengths (`stdDeviation`, `radius`, `dx`, `dy`, `baseFrequency`, `scale`) stay
//! untouched under `objectBoundingBox`.
//!
//! A primitive's own subregion (`x`/`y`/`width`/`height`, user lengths under
//! `userSpaceOnUse` — `usvg-0.47.0/src/parser/filter.rs:382`
//! `resolve_primitive_region`) is rewritten too: leaving it alone clipped the
//! primitive to the unscaled panel's box (measured: `stdDeviation="2" x="10"
//! y="10" width="80" height="60"` xor 16984 / maxdiff 255).

use super::lengths::{
    length_scale_factor, note_non_finite, note_overflow, scaled_finite, LengthAxis,
};
use super::lights::{is_light_source, is_lighting};
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
/// Called from the walk for **every** `<filter>`, because one family of numbers
/// inside it is user space regardless of `primitiveUnits` (the light sources, see
/// the module doc); the gate itself moved into [`rewrite_one`], where the two kinds
/// are told apart. `policy` is the filter's own policy, i.e. the policy of the
/// element that references it (definition policy), so a filter used from inside a
/// pattern body scales like that body.
pub(super) fn rewrite_primitives(
    element: &mut Element,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let user_space = primitive_units_are_user_space(element, context);
    for child in &mut element.children {
        if let Node::Element(child) = child {
            rewrite_one(child, user_space, policy, context, passthroughs)?;
        }
    }
    Ok(())
}

fn rewrite_one(
    element: &mut Element,
    user_space: bool,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let light = is_light_source(&element.name);
    // Under `objectBoundingBox` the primitive lengths are fractions of the filtered
    // box, which already follows the panel. The lights are the exception (they are
    // user-space positions whatever `primitiveUnits` says) — and so are the two
    // lighting primitives, which only exist to carry them.
    if !user_space && !light && !is_lighting(&element.name) {
        return Ok(());
    }
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
                    rewrite_one(child, user_space, policy, context, passthroughs)?;
                }
            }
        }
        // The lights are user-space positions whatever `primitiveUnits` says, and
        // their z is the RMS of the axis factors: `super::lights` owns the details.
        "fePointLight" | "feSpotLight" => {
            let name = element.name.clone();
            super::lights::rewrite_light(element, &name, policy, context, passthroughs)?;
        }
        // One number, applied quadratically by the renderer: the attribute takes the
        // square root of the axis factor (see the module doc).
        "feDisplacementMap" => scale_axis(
            element,
            "scale",
            LengthAxis::Displacement,
            policy,
            context,
            passthroughs,
        )?,
        _ => {}
    }
    // Every primitive's own subregion is a user length under `userSpaceOnUse`
    // (`usvg-0.47.0/src/parser/filter.rs:382`): `x`/`y` place it (so they
    // translate), `width`/`height` size it. This also covers `feImage`, whose four
    // numbers are exactly this subregion.
    if user_space && !light {
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
    scale_axis_with(element, attr, axis, false, policy, context, passthroughs)
}

/// The same, written with the shortest round-trip digits.
///
/// A light's `z` is the one length where the project's three decimals are too
/// coarse: measured on the anisotropic cell (sx=4/sy=5, `z="10"`), `45.277` differs
/// on 5 pixels (maxdiff 1) where `45.276928` differs on none, because the
/// shading is a smooth ramp near black and a 1e-4 change is enough to flip the
/// rounding of a pixel. Like `feTurbulence baseFrequency`, this keeps full digits.
pub(super) fn scale_axis_precise(
    element: &mut Element,
    attr: &str,
    axis: LengthAxis,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    scale_axis_with(element, attr, axis, true, policy, context, passthroughs)
}

fn scale_axis_with(
    element: &mut Element,
    attr: &str,
    axis: LengthAxis,
    precise: bool,
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
    match scaled_finite(value, factor) {
        Some(scaled) => element.set_attr(attr, format_length(scaled, precise)),
        // The product is not finite (or the factor is not): write nothing and let
        // W7 say the attribute was kept verbatim.
        None => note_overflow(passthroughs, context.index, element, attr, &raw, factor),
    }
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
        // A non-finite number is *kept verbatim*, like an unparsable one: writing
        // `inf`/`NaN` back would both contradict the warning's "kept verbatim" and
        // emit a string that is not a legal SVG number (round-data/R60).
        Ok(value) if value.is_finite() => Some(value),
        Ok(value) => {
            note_non_finite(passthroughs, context.index, element, attr, raw, value);
            None
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
        // `<number-optional-number>` is one or two numbers. usvg's
        // `convert_std_dev_attr` takes *nothing* from any other spelling and the
        // filter is applied with a zero deviation, so keeping the text verbatim
        // reproduces the renderer — but silently, which is why it is reported.
        passthroughs.record(
            context.index,
            &element.name,
            attr,
            format!("{raw:?}: expected one or two numbers"),
        );
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

    let convert = |value: f32, factor: f32| -> Option<f32> {
        let converted = if invert {
            value / factor
        } else {
            value * factor
        };
        (factor.is_finite() && converted.is_finite()).then_some(converted)
    };
    let Some(x) = parsed_or_kept(element, attr, tokens[0], context, passthroughs) else {
        return Ok(());
    };
    // One source value: the two numbers are equal by definition, so they only stay
    // one number when the scale is uniform. Under an anisotropic panel the single
    // value has to become an explicit pair (measured: the geometric mean is far
    // worse than the pair).
    if tokens.len() == 1 && (fx - fy).abs() < f32::EPSILON {
        match convert(x, fx) {
            Some(scaled) => element.set_attr(attr, format_length(scaled, precise)),
            None => note_overflow(passthroughs, context.index, element, attr, &raw, fx),
        }
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
    let (Some(x), Some(y)) = (convert(x, fx), convert(y, fy)) else {
        note_overflow(passthroughs, context.index, element, attr, &raw, fx);
        return Ok(());
    };
    element.set_attr(
        attr,
        format!(
            "{} {}",
            format_length(x, precise),
            format_length(y, precise)
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
