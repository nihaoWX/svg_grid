//! The `data-scale` protocol executor: rewrite every coordinate of an input panel so
//! it lands in its grid cell (`ARCHITECTURE/tag-protocol.md`).
//!
//! Split by responsibility (R14, `ARCHITECTURE.md` §跨领域约定 体积口径):
//! this module walks the tree and dispatches, [`lengths`] owns the scale factors and
//! the lengths they apply to, [`primitives`] rewrites the plain shapes, and [`paths`]
//! rewrites `<path d>`. Behaviour is unchanged by the split.

mod lengths;
mod paths;
mod primitives;

use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

use lengths::{rewrite_scaled_length, LengthAxis};
use paths::rewrite_path;
use primitives::{
    rewrite_axis_coordinates, rewrite_pair, rewrite_points, rewrite_rect, rewrite_rotate_transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScalePolicy {
    Xy,
    X,
    Y,
    Position,
    None,
}

/// Rewrite every coordinate of `root` under the composer's `data-scale` policy.
///
/// `index` is the input's position in the grid and `passthroughs` collects the
/// attributes that could **not** be rewritten but were deliberately kept verbatim
/// (warning W7): a malformed `<path d>` or a non-finite numeric attribute does not
/// fail the run — the composer records it and leaves the value alone.
pub fn transform_svg(
    root: &mut Element,
    source_panel: Rect,
    target_panel: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    transform_element(
        root,
        ScalePolicy::None,
        source_panel,
        target_panel,
        index,
        passthroughs,
    )
}

fn transform_element(
    element: &mut Element,
    inherited: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let policy = element
        .attr("data-scale")
        .map(parse_policy)
        .unwrap_or(inherited);

    rewrite_primitive(element, policy, source, target, index, passthroughs)?;

    // A `patternContentUnits="objectBoundingBox"` body is a *fraction* of the
    // referencing element's box, not a user-space coordinate: leaving the body alone
    // is what keeps it attached to the shape (the tile box itself may still be
    // `userSpaceOnUse`, so it is rewritten above).
    let body_is_fractional = element.name == "pattern"
        && matches!(
            element.attr("patternContentUnits"),
            Some(units) if units.trim() != "userSpaceOnUse"
        );
    if body_is_fractional {
        return Ok(());
    }

    for child in &mut element.children {
        if let Node::Element(child) = child {
            transform_element(child, policy, source, target, index, passthroughs)?;
        }
    }

    Ok(())
}

/// A paint server's coordinates only live in the panel's user space when the units
/// say so. `objectBoundingBox` (the SVG default) makes every number a fraction of
/// the referencing element's own box, so remapping it would move the paint away
/// from the shape.
fn user_space_units(element: &Element, attr: &str) -> bool {
    matches!(
        element.attr(attr),
        Some(units) if units.trim() == "userSpaceOnUse"
    )
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
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    match element.name.as_str() {
        "circle" => {
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
            rewrite_scaled_length(
                element,
                "r",
                policy,
                source,
                target,
                LengthAxis::Uniform,
                index,
                passthroughs,
            )?;
        }
        "text" => rewrite_pair(
            element,
            "x",
            "y",
            policy,
            source,
            target,
            index,
            passthroughs,
        )?,
        // A `<tspan>`'s `x`/`y` are *absolute* positions in the current user
        // coordinate system (they override the parent `<text>`'s position), so
        // they must move with the text or the run silently stays behind. Unlike
        // `<text>` (whose `rewrite_pair` requires both axes), the two axes are
        // rewritten independently: `<tspan x="10" dy="1.2em">` carries only one
        // coordinate, and skipping it would misplace the run.
        "tspan" => rewrite_axis_coordinates(element, policy, source, target, index, passthroughs)?,
        "use" => {
            rewrite_pair(
                element,
                "x",
                "y",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
            rewrite_scaled_length(
                element,
                "width",
                policy,
                source,
                target,
                LengthAxis::X,
                index,
                passthroughs,
            )?;
            rewrite_scaled_length(
                element,
                "height",
                policy,
                source,
                target,
                LengthAxis::Y,
                index,
                passthroughs,
            )?;
        }
        "line" => {
            rewrite_pair(
                element,
                "x1",
                "y1",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
            rewrite_pair(
                element,
                "x2",
                "y2",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
        }
        "rect" => rewrite_rect(element, policy, source, target, index, passthroughs)?,
        // An `<image>` covers exactly the axis-aligned `x`/`y`/`width`/`height`
        // rectangle it is placed in, so it is rewritten like a `<rect>`: the
        // bitmap is stretched to the new box. The converter that produces it
        // writes `preserveAspectRatio="none"` so the stretch is not defeated by
        // the default `xMidYMid meet` letterboxing.
        "image" => rewrite_rect(element, policy, source, target, index, passthroughs)?,
        "polyline" | "polygon" => {
            rewrite_points(element, policy, source, target, index, passthroughs)?
        }
        // A `<path>`'s `d` is pure geometry: every coordinate is remapped per axis
        // (H only x, V only y, `A` radii by their axis), the `A` rotation and both
        // flags stay untouched, and a rotated arc sheared by a non-uniform scale
        // is flattened to a polyline (see `crate::path`). A `d` the parser cannot
        // understand is **not** fail-closed: it is passed through verbatim with a
        // W7 warning.
        "path" => rewrite_path(element, policy, source, target, index, passthroughs)?,
        // A paint server is placed in the user space of the element that references
        // it — the same user space as every other coordinate of this panel — so with
        // `*Units="userSpaceOnUse"` its numbers must be remapped with the geometry or
        // the gradient/pattern stays where the unscaled panel was (the fill is still
        // painted, just with the wrong part of the gradient, or with the wrong tile
        // size — a silent, hard-to-see change). The `objectBoundingBox` default is
        // left untouched: those numbers are fractions of the referencing box.
        "linearGradient" if user_space_units(element, "gradientUnits") => {
            rewrite_pair(
                element,
                "x1",
                "y1",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
            rewrite_pair(
                element,
                "x2",
                "y2",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
        }
        // `r`/`fr` are radii: under a non-uniform scale a circle would become an
        // ellipse, which `r` cannot express, so they follow `<circle r>`'s rule
        // (uniform factor).
        "radialGradient" if user_space_units(element, "gradientUnits") => {
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
            rewrite_pair(
                element,
                "fx",
                "fy",
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
            for attr in ["r", "fr"] {
                rewrite_scaled_length(
                    element,
                    attr,
                    policy,
                    source,
                    target,
                    LengthAxis::Uniform,
                    index,
                    passthroughs,
                )?;
            }
        }
        // The tile box follows the same rule as a `<rect>`: the tile grid repeats
        // with period `width`/`height` from `x`/`y`, and both scale with the axis.
        "pattern" if user_space_units(element, "patternUnits") => {
            rewrite_rect(element, policy, source, target, index, passthroughs)?
        }
        _ => {}
    }
    rewrite_scaled_length(
        element,
        "stroke-width",
        policy,
        source,
        target,
        LengthAxis::Uniform,
        index,
        passthroughs,
    )?;
    rewrite_rotate_transform(element, policy, source, target, index, passthroughs)?;

    Ok(())
}
