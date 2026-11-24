//! Collect the panel letters an input already carries.
//!
//! A `data-svg-grid-label` marker may sit on any element — a `<text>`, a wrapping
//! `<g>`, or a `<defs>` definition pulled in by `<use>`. Reading its letter is
//! only correct if it mirrors what the renderer draws:
//!
//! * a subtree that is not rendered (`<defs>`, `display:none`, …) carries no
//!   letter;
//! * a nested marker owns its own text, so the outer marker must not read it
//!   (R51 A: one glyph used to be counted at two sites);
//! * a definition counts once per `<use>` instantiation, and a `<use>` that
//!   points at its own ancestor is a circular reference that draws nothing
//!   (R51 B);
//! * text that paints neither a fill nor a stroke leaves no ink, so a
//!   `fill="none"` glyph is not a letter (R51 E).
//!
//! The duplicate policy lives in [`crate::label_conflicts`]; this module only
//! answers "which letters does this subtree render, and at which marker".

use std::collections::HashMap;

use crate::dom::{Element, Node};
use crate::ink::{parse_number, style_prop, use_target, MAX_USE_DEPTH, NON_RENDERED};

/// One letter an input already carries: the marker's index and its rendered text.
pub(crate) struct InheritedLetter {
    pub(crate) marker: String,
    pub(crate) letter: String,
}

/// Collect every rendered marker in `root`, in document order.
pub(crate) fn collect_letters(root: &Element) -> Vec<InheritedLetter> {
    let mut ctx = Ctx {
        ids: collect_ids(root),
        ancestors: Vec::new(),
        depth: 0,
    };
    let mut out = Vec::new();
    walk_letters(&mut ctx, root, true, Paint::DEFAULT, &mut out);
    out
}

/// Walk state shared by the marker walk and the text walk: the `id` index that
/// resolves `<use>` references, the ancestor chain (cycle detection), and how
/// many `<use>` hops deep we already are.
struct Ctx<'a> {
    ids: HashMap<&'a str, &'a Element>,
    ancestors: Vec<&'a Element>,
    depth: u32,
}

/// Index every `id` in the document — a `<use>` may reference a definition in
/// `<defs>` *or* an element drawn elsewhere; the first occurrence wins, as in the
/// renderer.
fn collect_ids(root: &Element) -> HashMap<&str, &Element> {
    let mut ids = HashMap::new();
    index_ids(root, &mut ids);
    ids
}

fn index_ids<'a>(element: &'a Element, ids: &mut HashMap<&'a str, &'a Element>) {
    if let Some(id) = element.attr("id") {
        ids.entry(id).or_insert(element);
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            index_ids(child, ids);
        }
    }
}

/// Whether a text run would leave visible ink: `fill` defaults to black,
/// `stroke` to `none`, and both inherit. `none` / `transparent` and a zero
/// opacity paint nothing; the keywords are case-sensitive, because both reference
/// renderers draw `fill="NONE"` as black (R51 D).
#[derive(Clone, Copy)]
struct Paint {
    fill: bool,
    stroke: bool,
}

impl Paint {
    const DEFAULT: Paint = Paint {
        fill: true,
        stroke: false,
    };

    fn any(self) -> bool {
        self.fill || self.stroke
    }

    fn inherit(self, element: &Element) -> Paint {
        Paint {
            fill: paint_prop(element, "fill", "fill-opacity").unwrap_or(self.fill),
            stroke: paint_prop(element, "stroke", "stroke-opacity").unwrap_or(self.stroke),
        }
    }
}

fn paint_prop(element: &Element, prop: &str, opacity_prop: &str) -> Option<bool> {
    let value = style_prop(element, prop)?.trim();
    let painted = !matches!(value, "none" | "transparent");
    let opacity = style_prop(element, opacity_prop)
        .and_then(parse_number)
        .unwrap_or(1.0);
    let width = if prop == "stroke" {
        style_prop(element, "stroke-width")
            .and_then(parse_number)
            .unwrap_or(1.0)
    } else {
        1.0
    };
    Some(painted && opacity > 0.0 && width > 0.0)
}

/// Recursive half of [`collect_letters`]: `visible` is the effective inherited
/// `visibility`, `inherited` the inherited paint.
fn walk_letters<'a>(
    ctx: &mut Ctx<'a>,
    element: &'a Element,
    inherited_visible: bool,
    inherited: Paint,
    out: &mut Vec<InheritedLetter>,
) {
    let Some(visible) = enter(element, inherited_visible) else {
        return;
    };
    let paint = inherited.inherit(element);
    ctx.ancestors.push(element);
    if let Some(marker) = element.attr("data-svg-grid-label") {
        let mut text = String::new();
        collect_text(ctx, element, visible, paint, &mut text);
        let letter = crate::normalize_label(&text);
        // An empty (or whitespace-only) marker is not an "empty letter": it
        // draws nothing, so it must not take part in the collision check.
        if !letter.is_empty() {
            out.push(InheritedLetter {
                marker: marker.to_string(),
                letter,
            });
        }
    }
    if element.name == "use" {
        expand_use(ctx, element, visible, false, |ctx, target, visible| {
            walk_letters(ctx, target, visible, paint, out);
        });
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            walk_letters(ctx, child, visible, paint, out);
        }
    }
    ctx.ancestors.pop();
}

/// Concatenate the text a marker renders, in document order, skipping subtrees
/// that are not drawn and nested markers (they are letters of their own).
fn collect_text<'a>(
    ctx: &mut Ctx<'a>,
    element: &'a Element,
    visible: bool,
    paint: Paint,
    out: &mut String,
) {
    if element.name == "use" {
        // The instanced text is drawn where the `<use>` sits.
        expand_use(ctx, element, visible, true, |ctx, target, visible| {
            collect_text(ctx, target, visible, paint, out);
        });
        return;
    }
    for child in &element.children {
        match child {
            Node::Text(text) => {
                if visible && paint.any() {
                    out.push_str(text);
                }
            }
            Node::Element(child) => {
                // A nested marker is its own letter; letting the outer marker
                // read its text counts one glyph at two sites (R51 A).
                if child.attr("data-svg-grid-label").is_some() {
                    continue;
                }
                if let Some(child_visible) = enter(child, visible) {
                    collect_text(ctx, child, child_visible, paint.inherit(child), out);
                }
            }
        }
    }
}

/// Follow a `<use>`'s reference once. `visit` receives the instanced subtree
/// root (or, for a `<symbol>` wrapper, each child — the wrapper itself is not
/// drawn). A target already on the ancestor chain is a circular reference: the
/// renderer draws nothing for it, so it is not expanded again.
fn expand_use<'a, F>(
    ctx: &mut Ctx<'a>,
    element: &'a Element,
    visible: bool,
    skip_marked: bool,
    mut visit: F,
) where
    F: FnMut(&mut Ctx<'a>, &'a Element, bool),
{
    let Some(target) = use_target(element, &ctx.ids) else {
        return;
    };
    // Reading text out of an instanced *marker* would count that glyph once as
    // the outer letter's text and once as the inner marker's own letter; the
    // inner marker owns it (R51 A).
    if skip_marked && target.attr("data-svg-grid-label").is_some() {
        return;
    }
    if ctx.depth >= MAX_USE_DEPTH
        || ctx
            .ancestors
            .iter()
            .any(|ancestor| std::ptr::eq(*ancestor, target))
    {
        return;
    }
    ctx.ancestors.push(target);
    ctx.depth += 1;
    if NON_RENDERED.contains(&target.name.as_str()) {
        for child in &target.children {
            if let Node::Element(child) = child {
                visit(ctx, child, visible);
            }
        }
    } else {
        visit(ctx, target, visible);
    }
    ctx.depth -= 1;
    ctx.ancestors.pop();
}

/// Fold one element's own `display` / `opacity` / `visibility` onto the inherited
/// visibility. `None` means the element and its whole subtree are not rendered:
/// `display:none` and `opacity:0` cannot be undone by a descendant, while
/// `visibility:hidden` can (`visibility="visible"` on a child wins).
fn enter(element: &Element, inherited_visible: bool) -> Option<bool> {
    if NON_RENDERED.contains(&element.name.as_str()) {
        return None;
    }
    if matches!(style_prop(element, "display").map(str::trim), Some("none")) {
        return None;
    }
    if style_prop(element, "opacity")
        .and_then(parse_number)
        .is_some_and(|opacity| opacity <= 0.0)
    {
        return None;
    }
    Some(match style_prop(element, "visibility").map(str::trim) {
        Some("hidden") | Some("collapse") => false,
        Some("visible") => true,
        _ => inherited_visible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letters(markup: &str) -> Vec<(String, String)> {
        let root = crate::dom::parse(markup).expect("test markup parses");
        collect_letters(&root)
            .into_iter()
            .map(|letter| (letter.marker, letter.letter))
            .collect()
    }

    #[test]
    fn a_nested_marker_is_collected_once_by_its_own_element() {
        let found = letters(
            "<g data-svg-grid-label=\"0\">\
             <text data-svg-grid-label=\"1\">A</text></g>",
        );
        assert_eq!(found, vec![("1".to_string(), "A".to_string())]);
    }

    #[test]
    fn an_outer_marker_keeps_its_own_text_next_to_a_nested_one() {
        let found = letters(
            "<g data-svg-grid-label=\"0\"><text>A</text>\
             <text data-svg-grid-label=\"1\">B</text></g>",
        );
        assert_eq!(
            found,
            vec![
                ("0".to_string(), "A".to_string()),
                ("1".to_string(), "B".to_string())
            ]
        );
    }

    #[test]
    fn a_definition_counts_once_per_use() {
        let one = letters(
            "<g><defs><text id=\"L\" data-svg-grid-label=\"0\">A</text></defs>\
             <use href=\"#L\"/></g>",
        );
        assert_eq!(one, vec![("0".to_string(), "A".to_string())]);
        let two = letters(
            "<g><defs><text id=\"L\" data-svg-grid-label=\"0\">A</text></defs>\
             <use href=\"#L\"/><use href=\"#L\"/></g>",
        );
        assert_eq!(two.len(), 2, "two instantiations render two letters");
    }

    #[test]
    fn an_unreferenced_definition_is_not_a_letter() {
        let found =
            letters("<g><defs><text id=\"L\" data-svg-grid-label=\"0\">A</text></defs></g>");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_circular_use_expands_only_once() {
        let found = letters(
            "<g id=\"L\" data-svg-grid-label=\"0\"><text>A</text>\
             <use href=\"#L\" x=\"30\"/></g>",
        );
        assert_eq!(found, vec![("0".to_string(), "A".to_string())]);
    }

    #[test]
    fn a_symbol_is_drawn_through_use() {
        let found = letters(
            "<g><defs><symbol id=\"S\">\
             <text data-svg-grid-label=\"0\">A</text></symbol></defs>\
             <use href=\"#S\"/></g>",
        );
        assert_eq!(found, vec![("0".to_string(), "A".to_string())]);
    }

    #[test]
    fn text_that_paints_nothing_is_not_a_letter() {
        assert!(letters("<text data-svg-grid-label=\"0\" fill=\"none\">A</text>").is_empty());
        assert!(
            letters("<g fill=\"none\"><text data-svg-grid-label=\"0\">A</text></g>").is_empty()
        );
        assert_eq!(
            letters("<g fill=\"none\"><text data-svg-grid-label=\"0\" fill=\"black\">A</text></g>"),
            vec![("0".to_string(), "A".to_string())]
        );
        assert_eq!(
            letters("<text data-svg-grid-label=\"0\" fill=\"none\" stroke=\"black\">A</text>"),
            vec![("0".to_string(), "A".to_string())]
        );
    }

    #[test]
    fn important_and_case_follow_the_renderer() {
        assert!(letters(
            "<g style=\"display:none !important\"><text data-svg-grid-label=\"0\">A</text></g>"
        )
        .is_empty());
        // Both reference renderers draw these (R51 D): they are letters.
        for wrapper in [
            "<g style=\"display:NONE\">",
            "<g style=\"DISPLAY:none\">",
            "<g display=\"NONE\">",
            "<g visibility=\"HIDDEN\">",
        ] {
            let markup = format!("{wrapper}<text data-svg-grid-label=\"0\">A</text></g>");
            assert_eq!(letters(&markup).len(), 1, "{wrapper} renders the A");
        }
    }
}
