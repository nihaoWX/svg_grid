//! The `data-scale` protocol executor: rewrite every coordinate of an input panel so
//! it lands in its grid cell (`ARCHITECTURE/tag-protocol.md`).
//!
//! Split by responsibility (R14, `ARCHITECTURE.md` §跨领域约定 体积口径):
//! this module walks the tree and dispatches, [`lengths`] owns the scale factors and
//! the lengths they apply to, [`primitives`] rewrites the plain shapes, [`paths`]
//! rewrites `<path d>`, and [`patterns`] decides what a `<pattern>` body's numbers
//! mean (its `patternContentUnits`/`viewBox`, inherited along `href`).

mod lengths;
mod paths;
mod patterns;
mod primitives;

use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

use lengths::{rewrite_scaled_length, LengthAxis};
use paths::rewrite_path;
use patterns::{pattern_body_is_verbatim, pattern_units_is_user_space, PatternIndex};
use primitives::{
    rewrite_axis_coordinates, rewrite_pair, rewrite_pattern_tile, rewrite_points, rewrite_rect,
    rewrite_rotate_transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScalePolicy {
    Xy,
    X,
    Y,
    Position,
    None,
    /// Scale the numbers but never translate them. Only the composer produces it,
    /// and only for a `<pattern>` body: the body's coordinates are *tile local*
    /// (both resvg and cairosvg place them relative to the tile origin), so the
    /// tile may move to its cell while the body must stay where it is — moving
    /// both would push the body out of its own tile window and the fill would
    /// vanish silently (round-data/R38). `data-scale` never yields this variant.
    Scale,
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
    let patterns = PatternIndex::collect(root);
    transform_element(
        root,
        ScalePolicy::None,
        source_panel,
        target_panel,
        index,
        &patterns,
        passthroughs,
    )
}

fn transform_element(
    element: &mut Element,
    inherited: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    patterns: &PatternIndex,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let policy = element
        .attr("data-scale")
        .map(parse_policy)
        .unwrap_or(inherited);

    rewrite_primitive(
        element,
        policy,
        source,
        target,
        index,
        patterns,
        passthroughs,
    )?;

    // A `<pattern>` body is **tile local** in both renderers: the tile box
    // (`x/y/width/height`, rewritten above) is mapped into the cell, but the body's
    // numbers are placed relative to the tile origin. Translating the body by the
    // panel offset as well would move it out of its own tile window, and the fill
    // disappears with no error at all (measured: ink 9600 -> 0 under both resvg and
    // cairosvg, round-data/R38).
    //
    // What the body's numbers *mean* depends on `patternContentUnits` and on
    // whether the pattern has a `viewBox` — both inherited along `href`, see
    // [`patterns::pattern_body_is_verbatim`]:
    //
    // * neither (the default `userSpaceOnUse`) — user units of the tile, so they
    //   scale with the panel but keep the tile's own origin (`Scale`);
    // * `objectBoundingBox` — fractions of the **referencing element's bbox**, and
    //   that box already scales with the panel. Multiplying the fractions by
    //   `(sx, sy)` on top of that is a double scaling: the motif shrinks/expands
    //   and the output stops matching the source (measured at s=0.5: ink 4 vs 16,
    //   xor 16 md 255; the base commit rendered xor 0 — round-data/R39);
    // * a valid `viewBox` — the body lives in viewBox units and the viewBox→tile
    //   mapping already follows the panel, so scaling the numbers again is the same
    //   mistake (measured at s=2: `2 2 6 6` became `4 4 12 12`, xor 1792 —
    //   round-data/R40).
    //
    // In both of the latter cases the body is left verbatim, without recursion. A
    // chain the composer cannot resolve is treated the same way (fail safe: never
    // *add* a scaling the renderer did not ask for).
    //
    // `patternTransform` is deliberately left verbatim: it is already relative to
    // the tile, so it keeps working, and the corpus uses it nowhere (0 of 220
    // fixtures). If it ever needs rewriting, it must be *accumulated*, not replaced.
    if element.name == "pattern" {
        if pattern_body_is_verbatim(element, patterns) {
            return Ok(());
        }
        for child in &mut element.children {
            if let Node::Element(child) = child {
                transform_element(
                    child,
                    ScalePolicy::Scale,
                    source,
                    target,
                    index,
                    patterns,
                    passthroughs,
                )?;
            }
        }
        return Ok(());
    }

    // A `<marker>` is never rewritten, in either unit mode (round-data/R37).
    //
    // The renderer places a marker by translating to the vertex, rotating by
    // `orient`, and then translating by `-refX,-refY`; the body is clipped to the
    // viewport `(0, 0, markerWidth, markerHeight)` in marker coordinates. So the
    // body's numbers are *relative to the reference point*, and the clip box is
    // anchored at the marker origin: translating the body by the panel offset
    // moves it out of its own viewport and the arrowhead disappears (measured:
    // ink 184 -> 160 px, the whole marker clipped away). Leaving the body alone
    // keeps it attached to the vertex, which is exactly what the panel move
    // needs. Under the default `markerUnits="strokeWidth"` the renderer also
    // scales the body by the stroke width, and `stroke-width` *is* rewritten
    // below, so the arrowhead scales with the panel for free.
    if element.name == "marker" {
        return Ok(());
    }

    // A `<mask>` body is user space by default (`maskContentUnits` defaults to
    // `userSpaceOnUse`), so it is rewritten with the geometry. Only an explicit
    // `objectBoundingBox` makes the numbers fractions of the masked element's
    // box, and then they must stay untouched.
    if element.name == "mask"
        && matches!(
            element.attr("maskContentUnits"),
            Some(units) if units.trim() != "userSpaceOnUse"
        )
    {
        return Ok(());
    }

    // A `<filter>`'s primitives are not shapes: their numbers are filter-region
    // coordinates, and the composer never rewrites them. Descending would only
    // rewrite a `<path>`/`<rect>` that happens to sit inside a primitive and
    // break the filter's own geometry.
    if element.name == "filter" {
        return Ok(());
    }

    for child in &mut element.children {
        if let Node::Element(child) = child {
            transform_element(child, policy, source, target, index, patterns, passthroughs)?;
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
    patterns: &PatternIndex,
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
        // `patternUnits` also inherits along `href` and defaults to
        // `objectBoundingBox`, whose fractions already follow the box
        // (round-data/R41).
        "pattern" if pattern_units_is_user_space(element, patterns) => {
            rewrite_pattern_tile(element, policy, source, target, index, passthroughs)?
        }
        // A mask region is a rectangle in the user space of the masked element
        // only when `maskUnits="userSpaceOnUse"`; the `objectBoundingBox` default
        // makes `x`/`y`/`width`/`height` fractions of that element's box, so they
        // must stay untouched (round-data/R37).
        "mask" if user_space_units(element, "maskUnits") => {
            rewrite_rect(element, policy, source, target, index, passthroughs)?
        }
        // A filter region is likewise user-space only under
        // `filterUnits="userSpaceOnUse"`; the primitives inside are never
        // rewritten (see `transform_element`).
        "filter" if user_space_units(element, "filterUnits") => {
            rewrite_rect(element, policy, source, target, index, passthroughs)?
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

#[cfg(test)]
mod tests;
