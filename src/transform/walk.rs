//! The walk that rewrites one panel: policy resolution, definitions, and the
//! per-element dispatch into [`super::primitives`]/[`super::paths`].
//!
//! Split out of `mod.rs` when the definition-policy branch pushed it past the
//! 20000-byte gate (round-data/R46).

use super::defs::{self, DefsPlan};
use super::dispatch::rewrite_primitive;
use super::filters;
use super::lengths::{rewrite_scaled_length, LengthAxis};
use super::links::IdIndex;
use super::patterns::{self, pattern_body_is_verbatim, PatternIndex};
use super::primitives::rewrite_marker_box;
use super::units::{marker_units_are_user_space, units_of, Units};
use super::ScalePolicy;
use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

/// The user space a definition body is drawn in, once its box has been mapped:
/// bodies are placed relative to their own box, so they scale but never translate.
///
/// `None` means "leave the body verbatim": `data-scale="position"` never changes a
/// size, and `data-scale="none"` never scales one (round-data/R46).
fn body_policy(policy: ScalePolicy) -> Option<ScalePolicy> {
    match policy {
        ScalePolicy::Xy => Some(ScalePolicy::Scale),
        ScalePolicy::X => Some(ScalePolicy::ScaleX),
        ScalePolicy::Y => Some(ScalePolicy::ScaleY),
        ScalePolicy::Position | ScalePolicy::None => None,
        ScalePolicy::Scale | ScalePolicy::ScaleX | ScalePolicy::ScaleY => Some(policy),
    }
}

/// The indexes one panel's transform needs, so the walk does not thread them one by
/// one.
pub(super) struct Context<'a> {
    pub(super) source: Rect,
    pub(super) target: Rect,
    pub(super) index: usize,
    pub(super) patterns: &'a PatternIndex,
    pub(super) links: &'a IdIndex,
    pub(super) defs: &'a DefsPlan,
}

/// Rewrite every coordinate of `root` under the composer's `data-scale` policy.
///
/// `index` is the input's position in the grid and `passthroughs` collects the
/// attributes that could **not** be rewritten but were deliberately kept verbatim
/// (warning W7): a malformed `<path d>` or a non-finite numeric attribute does not
/// fail the run — the composer records it and leaves the value alone.
pub(crate) fn transform_svg(
    root: &mut Element,
    source_panel: Rect,
    target_panel: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    // Definitions are evaluated in the user space of whoever references them, so
    // their policy is decided by the reference graph — and a definition several
    // policies share is copied once per policy before anything is rewritten.
    let defs = defs::split(root);
    let patterns = PatternIndex::collect(root);
    let links = IdIndex::collect(root);
    let context = Context {
        source: source_panel,
        target: target_panel,
        index,
        patterns: &patterns,
        links: &links,
        defs: &defs,
    };
    transform_element(root, ScalePolicy::None, false, &context, passthroughs)
}

/// `body` is true for every element whose numbers live in a **definition body**
/// (a pattern/mask/clipPath/marker body rather than panel geometry). It changes
/// exactly one thing today: a `<text>`/`<tspan>` there has its `font-size` scaled
/// (round-data/R58). Panel text keeps its point size under every policy, as the tag
/// protocol requires, so the flag must not be inferred from the policy alone.
pub(super) fn transform_element(
    element: &mut Element,
    inherited: ScalePolicy,
    body: bool,
    context: &Context<'_>,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    // A definition does not inherit the policy of the `<defs>` block it is written
    // in: it inherits the policy of the elements that point at it, which is what
    // `defs` resolved (round-data/R46).
    let policy = context.defs.policy_of(element).unwrap_or_else(|| {
        element
            .attr("data-scale")
            .map(parse_policy)
            .unwrap_or(inherited)
    });

    rewrite_primitive(element, policy, context, passthroughs)?;

    // A body drawn in the panel's user space (a pattern without `viewBox`, a mask or
    // a clipPath body) scales its coordinates, so its glyphs have to grow with them:
    // leaving `font-size="12"` verbatim in a body whose `x`/`y` doubled makes the
    // text half the intended size (measured at s=2: xor 1524 / maxdiff 255, ink 276
    // against 1092; writing 24 is xor 0). Both spellings are handled because
    // [`rewrite_scaled_length`] rewrites the attribute *and* an inline
    // `style="font-size: 12px"`; a `<tspan font-size="12">` is the same defect.
    //
    // This is gated on `body`, not on the policy: ordinary panel `<text>` carries
    // `Xy` too (its coordinates follow the panel) but its size is normalised
    // separately on the paper (`--normalize-input-max-side`), and
    // `ARCHITECTURE/tag-protocol.md` says `font-size` never scales with a policy.
    // `Position`/`None` bodies (`data-scale="position"`) keep their point size
    // anyway: their factor is `None` inside [`rewrite_scaled_length`].
    if body && matches!(element.name.as_str(), "text" | "tspan") {
        rewrite_scaled_length(
            element,
            "font-size",
            policy,
            context.source,
            context.target,
            LengthAxis::Uniform,
            context.index,
            passthroughs,
        )?;
    }

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
    // `patternTransform` is **not** tile relative: the renderer composes it in the
    // user space of the referencing element, after the tile's own box mapping, so
    // it has to be conjugated through the panel scale like every other coordinate
    // (`dispatch::rewrite_paint_transform`, round-data/R55). The earlier comment
    // here claimed the opposite and that a rewrite would have to *accumulate*;
    // both are wrong — measured, nested patterns each need exactly their own
    // single conjugation, and accumulating double-scales the inner one. The
    // attribute is rewritten in `rewrite_primitive` before this branch runs.
    if element.name == "pattern" {
        if pattern_body_is_verbatim(element, context.patterns) {
            return Ok(());
        }
        let Some(body) = body_policy(policy) else {
            return Ok(());
        };
        for child in &mut element.children {
            if let Node::Element(child) = child {
                transform_element(child, body, true, context, passthroughs)?;
            }
        }
        return Ok(());
    }

    // A `<marker>` body is relative to the reference point (round-data/R37): the
    // renderer translates to the vertex, rotates, then translates by
    // `-refX,-refY`, and clips the body to `(0, 0, markerWidth, markerHeight)`.
    //
    // Under the default `markerUnits="strokeWidth"` the body is scaled by the
    // stroke width, and `stroke-width` *is* rewritten below, so the arrowhead
    // follows the panel for free: leaving the body alone is correct. Under
    // `markerUnits="userSpaceOnUse"` the body, the viewport and the reference
    // point are all plain user lengths instead, and nothing else rewrites them —
    // so they have to be scaled here (round-data/R43; measured at s=2:
    // xor 622, ink 1952 -> 1459). The units attribute is compared literally:
    // every other value means the `strokeWidth` default.
    //
    // A **valid `viewBox`** changes what the body's numbers mean: `usvg-0.47.0/
    // src/parser/marker.rs:183-196` builds `Size::from_wh(markerWidth, markerHeight)`
    // and scales the body through `ViewBox::to_transform(size)`, so the body is
    // already expanded to the viewport. Scaling it a second time over-expands it
    // (round-data/R46: `mk.svg` at width 248 drew the arrowhead at 24.8 units inside
    // a 24.8-unit box, xor 747). Only the viewport and `refX`/`refY` scale then.
    if element.name == "marker" {
        if !marker_units_are_user_space(element) {
            return Ok(());
        }
        let Some(body) = body_policy(policy) else {
            return Ok(());
        };
        rewrite_marker_box(
            element,
            body,
            context.source,
            context.target,
            context.index,
            passthroughs,
        )?;
        if marker_body_is_verbatim(element) {
            return Ok(());
        }
        for child in &mut element.children {
            if let Node::Element(child) = child {
                transform_element(child, body, true, context, passthroughs)?;
            }
        }
        return Ok(());
    }

    // A `<mask>` body is user space by default (`maskContentUnits` defaults to
    // `userSpaceOnUse`), so it is rewritten with the geometry. Only a **valid**
    // `objectBoundingBox` makes the numbers fractions of the masked element's
    // box, and then they must stay untouched. An invalid value (a stray space, a
    // typo) is not a synonym for `objectBoundingBox`: the renderer falls back to
    // the `userSpaceOnUse` default and the body must be scaled (round-data/R43;
    // `maskContentUnits="foo"` measured at s=2: xor 6800, ink 6400 -> 1600).
    if element.name == "mask"
        && units_of(element, "maskContentUnits", Units::UserSpaceOnUse) == Units::ObjectBoundingBox
    {
        return Ok(());
    }

    // A `<filter>`'s primitives are not shapes, but their numbers *are* user
    // lengths whenever `primitiveUnits` is `userSpaceOnUse` — which is its
    // **default** (`usvg-0.47.0/src/parser/filter.rs:171`). Leaving them verbatim
    // keeps `stdDeviation`/`dx`/`radius`/`baseFrequency`/the light at the unscaled
    // panel's size (measured at s=2: `feGaussianBlur stdDeviation="2"` xor 8360).
    // So the primitives get their own rewrite pass instead of the generic descent:
    // the generic one would look at a `<path>`/`<rect>` that happens to sit inside
    // a primitive as if it were panel geometry (`super::filters`).
    if element.name == "filter" {
        if filters::primitive_units_are_user_space(element, context) {
            filters::rewrite_primitives(element, policy, context, passthroughs)?;
        }
        return Ok(());
    }

    // A `<linearGradient>`/`<radialGradient>` has no body worth descending into:
    // its `<stop>`s only carry `offset` (a fraction of the gradient vector). The
    // `gradientTransform` itself *is* a user-space length when
    // `gradientUnits="userSpaceOnUse"` and has already been conjugated by
    // `rewrite_primitive` (round-data/R55); under the `objectBoundingBox` default
    // it is a box fraction and stays untouched.
    if element.name == "linearGradient" || element.name == "radialGradient" {
        return Ok(());
    }

    // A `<mask>` body and a `<clipPath>` body are definition bodies too, but their
    // numbers are placed in the **referencing element's** user space (translated to
    // the cell), so they keep the inherited policy for coordinates; only the
    // font-size rule needs to know they are bodies.
    let child_body = body || matches!(element.name.as_str(), "mask" | "clipPath");
    for child in &mut element.children {
        if let Node::Element(child) = child {
            transform_element(child, policy, child_body, context, passthroughs)?;
        }
    }

    Ok(())
}

/// Whether a `<marker>`'s body is already expanded to its viewport by the renderer.
fn marker_body_is_verbatim(element: &Element) -> bool {
    element
        .attr("viewBox")
        .is_some_and(patterns::valid_view_box)
}

pub(crate) fn parse_policy(value: &str) -> ScalePolicy {
    match value {
        "xy" => ScalePolicy::Xy,
        "x" => ScalePolicy::X,
        "y" => ScalePolicy::Y,
        "position" => ScalePolicy::Position,
        "none" => ScalePolicy::None,
        _ => ScalePolicy::None,
    }
}
