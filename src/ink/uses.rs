//! `<defs>` / `<use>` instantiation: resolving a `<use>`'s reference and placing
//! the referenced definition's ink at the `<use>`'s offset.

use std::collections::HashMap;

use crate::dom::{Element, Node};

use super::bbox::Bbox;
use super::walk::{number_attr, visit, Ctx};

/// How deep a chain of `<use>` references is followed before we stop, so a
/// self-referential definition can never recurse forever.
const MAX_USE_DEPTH: u32 = 16;

/// What [`add_resolved_use`] did with a `<use>`'s reference.
pub(super) enum UseInk {
    /// The reference resolved and its content was folded into the box.
    Measured,
    /// The id exists but names content the composer does not carry (`<symbol>`,
    /// nested `<svg>`, an exhausted `<use>` chain, or an empty body): the caller
    /// falls back to the placement box, an under-estimate by design.
    Unsupported,
    /// The id is absent from the document (a dangling reference — or no `href`
    /// at all). The `<use>` draws nothing, so it contributes no ink.
    Dangling,
}

/// Measure the `<defs>` content a `<use>` references, shifted by the use's
/// `(x, y)`, into `local`.
pub(super) fn add_resolved_use(
    element: &Element,
    ctx: &Ctx,
    ids: &HashMap<&str, &Element>,
    depth: u32,
    x: f64,
    y: f64,
    local: &mut Bbox,
) -> UseInk {
    // Report a dangling reference (id absent from the document) before anything
    // else: it renders nothing, so its `(x, y)` anchor must never be counted.
    let Some(target) = use_target(element, ids) else {
        return UseInk::Dangling;
    };
    if depth >= MAX_USE_DEPTH {
        return UseInk::Unsupported;
    }
    // `<symbol>` / nested `<svg>` bring their own viewport rules; the composer
    // does not carry them, so they are not measured here.
    if target.name == "symbol" || target.name == "svg" {
        return UseInk::Unsupported;
    }
    let mut referenced = Bbox::empty();
    visit(target, ctx, ids, depth + 1, &mut referenced);
    if referenced.is_empty() {
        return UseInk::Unsupported;
    }
    referenced.translate_into(x, y, local);
    UseInk::Measured
}

/// The under-estimate used when a `<use>`'s target exists but is not carried by
/// the composer (`<symbol>`, nested `<svg>`, …): its declared `width`/`height`
/// box, else just the anchor point (plus the half-stroke).
pub(super) fn add_use_placement(element: &Element, ctx: &Ctx, x: f64, y: f64, local: &mut Bbox) {
    let s = ctx.half_stroke();
    match (
        number_attr(element, "width"),
        number_attr(element, "height"),
    ) {
        (Some(width), Some(height)) => local.add_rect(x - s, y - s, x + width + s, y + height + s),
        _ => local.add_rect(x - s, y - s, x + s, y + s),
    }
}

/// Index every `id` inside a `<defs>` subtree, so a `<use>` can resolve its
/// `href`. `<defs>` is not drawn where it sits, so this is the only way its
/// content is ever measured: through a reference.
pub(super) fn collect_ids(root: &Element) -> HashMap<&str, &Element> {
    let mut ids = HashMap::new();
    collect_ids_in(root, &mut ids);
    ids
}

fn collect_ids_in<'a>(element: &'a Element, ids: &mut HashMap<&'a str, &'a Element>) {
    if element.name == "defs" {
        index_subtree(element, ids);
        return;
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_ids_in(child, ids);
        }
    }
}

fn index_subtree<'a>(element: &'a Element, ids: &mut HashMap<&'a str, &'a Element>) {
    if let Some(id) = element.attr("id") {
        ids.entry(id).or_insert(element);
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            index_subtree(child, ids);
        }
    }
}

/// Resolve a `<use>`'s `href` / `xlink:href` (`#id`) against the `<defs>` index.
fn use_target<'a>(element: &Element, ids: &HashMap<&'a str, &'a Element>) -> Option<&'a Element> {
    let href = element
        .attr("href")
        .or_else(|| element.attr("xlink:href"))?;
    ids.get(href.strip_prefix('#')?).copied()
}
