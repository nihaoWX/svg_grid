//! The per-element rewrite: which attributes of a shape or a definition are
//! coordinates, and which `*Units` decides whether they are user units at all.
//!
//! Split out of `walk.rs` to keep every source file under the 20000-byte gate
//! (round-data/R46).

use super::lengths::{rewrite_scaled_length, LengthAxis};
use super::links::resolved_units;
use super::paths::rewrite_path;
use super::patterns::{
    pattern_body_is_verbatim, pattern_units_is_user_space, rewrite_pattern_tile,
};
use super::primitives::{
    rewrite_axes, rewrite_pair, rewrite_points, rewrite_rect, rewrite_region,
    rewrite_rotate_transform,
};
use super::transforms::rewrite_paint_transform;
use super::units::{units_of, Units};
use super::walk::Context;
use super::ScalePolicy;
use crate::dom::Element;
use crate::error::Result;
use crate::warnings::Passthroughs;

pub(super) fn rewrite_primitive(
    element: &mut Element,
    policy: ScalePolicy,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let source = context.source;
    let target = context.target;
    let index = context.index;
    let patterns = context.patterns;
    let links = context.links;
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
        "text" => {
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
        }
        // A `<tspan>`'s `x`/`y` are *absolute* positions in the current user
        // coordinate system (they override the parent `<text>`'s position), so
        // they must move with the text or the run silently stays behind. Unlike
        // `<text>` (whose `rewrite_pair` requires both axes), the two axes are
        // rewritten independently: `<tspan x="10" dy="1.2em">` carries only one
        // coordinate, and skipping it would misplace the run.
        "tspan" => {
            rewrite_axes(
                element,
                &[("x", false), ("y", true)],
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
        }
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
        "linearGradient"
            if resolved_units(element, links, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            // Rewritten **one attribute at a time**: a gradient may spell only
            // `x1` and inherit `x2` along `href`, and a pair-wise rewrite would
            // skip it entirely (round-data/R46, `gr.svg`). Each element in the
            // chain scales only its own declarations, so nothing is scaled twice.
            rewrite_axes(
                element,
                &[("x1", false), ("y1", true), ("x2", false), ("y2", true)],
                policy,
                source,
                target,
                index,
                passthroughs,
            )?;
            // The transform is written in the same user space as the coordinates,
            // so it is conjugated through the panel scale (round-data/R55).
            rewrite_paint_transform(
                element,
                "gradientTransform",
                policy,
                source,
                target,
                index,
                passthroughs,
            );
        }
        // `r`/`fr` are radii: under a non-uniform scale a circle would become an
        // ellipse, which `r` cannot express, so they follow `<circle r>`'s rule
        // (uniform factor).
        "radialGradient"
            if resolved_units(element, links, "gradientUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            rewrite_axes(
                element,
                &[("cx", false), ("cy", true), ("fx", false), ("fy", true)],
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
            rewrite_paint_transform(
                element,
                "gradientTransform",
                policy,
                source,
                target,
                index,
                passthroughs,
            );
        }
        // The tile box follows the same rule as a `<rect>`: the tile grid repeats
        // with period `width`/`height` from `x`/`y`, and both scale with the axis.
        // `patternUnits` also inherits along `href` and defaults to
        // `objectBoundingBox`, whose fractions already follow the box
        // (round-data/R41).
        "pattern" => {
            if pattern_units_is_user_space(element, patterns) {
                rewrite_pattern_tile(element, policy, source, target, patterns)?;
            }
            // `patternTransform` is user space too, **even when the tile box is
            // `objectBoundingBox`**: the renderer composes it after the tile's
            // own box transform (measured, round-data/R55). A `viewBox` does not
            // exempt it either. Each definition in a nesting chain is conjugated
            // with the scale that references *it*, so they are never accumulated.
            rewrite_paint_transform(
                element,
                "patternTransform",
                policy,
                source,
                target,
                index,
                passthroughs,
            );
        }
        // A mask region is a rectangle in the user space of the masked element
        // only when `maskUnits="userSpaceOnUse"`; the `objectBoundingBox` default
        // makes `x`/`y`/`width`/`height` fractions of that element's box, so they
        // must stay untouched (round-data/R37). The units are read with
        // `units_of`, **not** `resolved_units`: `<mask>` follows no `href` chain
        // at all (`usvg-0.47.0/src/parser/mask.rs:20-30` reads the attribute
        // directly), and unifying it with the gradient families would inherit a
        // unit the renderer does not inherit.
        "mask"
            if units_of(element, "maskUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            rewrite_region(element, policy, source, target, index, passthroughs)?
        }
        // A filter region is likewise user-space only under
        // `filterUnits="userSpaceOnUse"`. `filterUnits` *does* inherit along `href`
        // (`src/parser/filter.rs:170-171` → `convert_units` → `resolve_attr`),
        // unlike the mask units below — which is exactly why the region is
        // rewritten with `rewrite_region` and not `rewrite_rect`: a `<filter
        // href="#base"/>` spells **no** region itself and must keep inheriting it
        // instead of having a 0-sized one invented (round-data/R58; measured ink 0
        // against 9588). The primitives inside are handled by
        // [`super::filters`] under their own gate (`primitiveUnits`).
        "filter"
            if resolved_units(element, links, "filterUnits", Units::ObjectBoundingBox)
                == Units::UserSpaceOnUse =>
        {
            rewrite_region(element, policy, source, target, index, passthroughs)?
        }
        _ => {}
    }
    // A body that stays verbatim keeps the lengths it inherits from the
    // `<pattern>` element too: under `objectBoundingBox` a `stroke-width` there is
    // a fraction of the same box, and under a `viewBox` it is in viewBox units, so
    // scaling either would double-scale the body's outline as well.
    if element.name == "pattern" && pattern_body_is_verbatim(element, patterns) {
        return Ok(());
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
