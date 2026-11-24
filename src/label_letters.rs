//! Collect the panel letters an input already carries.
//!
//! A `data-svg-grid-label` marker may sit on any element — a `<text>`, a wrapping
//! `<g>`, or a `<defs>` definition pulled in by `<use>`. Reading its letter is
//! only correct if it mirrors what the renderer draws:
//!
//! * a subtree that is not rendered (`<defs>`, `display:none`, …) carries no
//!   letter;
//! * a marker's letter is the text its **whole subtree** draws, nested markers
//!   included, because that is what the renderer paints (R54 D);
//! * when a marker contributes no characters of its own — its subtree text is
//!   exactly the concatenation of its nested markers' letters — the outer marker
//!   is dropped and only the inner ones count, so one glyph is never counted at
//!   two sites (R51 A, R54 D);
//! * character data only draws inside a text-content element (`<text>`,
//!   `<tspan>`, `<textPath>`, `<a>`); bare text inside a `<g>` is not rendered by
//!   either reference renderer (R54 D);
//! * a definition counts once per `<use>` instantiation, a `<symbol>` marker is
//!   drawn through its `<use>` (R54 C), and a `<use>` that points at its own
//!   ancestor is a circular reference that draws nothing (R51 B);
//! * text that paints neither a fill nor a stroke leaves no ink, so a
//!   `fill="none"` glyph, a `url(#missing)` fill or a zero opacity is not a
//!   letter (R51 E, R54 A).
//!
//! The duplicate policy lives in [`crate::label_conflicts`]; this module only
//! answers "which letters does this subtree render, and at which marker".

use std::collections::HashMap;

use crate::dom::{Element, Node};
use crate::ink::{parse_number, style_prop, use_target, MAX_USE_DEPTH, NON_RENDERED};
use crate::label_paint::Paint;

/// Trim a panel letter and collapse internal whitespace runs to one space.
///
/// An inherited `data-svg-grid-label` text and an explicit `--labels` entry must
/// compare the same way: `A`, `" A "` and `"A  B"` are `A`, `A` and `A B`.
///
/// Only the four whitespace characters SVG's default `xml:space="default"`
/// collapses are folded (U+0020, U+0009, U+000A, U+000D). U+00A0 is *content*,
/// not layout whitespace, and stays significant (R51: `A` and `A&#160;` render
/// to identical pixels but are different label strings, as in the R48 base).
/// Zero-width characters are removed instead: U+200B/U+200C/U+200D/U+FEFF add
/// no ink and no advance, so `A` and `A\u{200b}` are the same letter (R51:
/// both reference renderers produce byte-identical PNGs).
pub fn normalize_label(text: &str) -> String {
    let without_zero_width: String = text
        .chars()
        .filter(|c| !matches!(c, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}'))
        .collect();
    without_zero_width
        .split([' ', '\t', '\n', '\r'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

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
    let in_text = text_context(&root.name, false);
    let (letters, _) = subtree(&mut ctx, root, true, Paint::DEFAULT, in_text, false);
    letters
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

/// Does character data directly inside `name` draw? Only a text-content element
/// renders its own character data; `<g>A</g>` draws nothing in either reference
/// renderer, and a `<g>` inside a `<text>` breaks the text run (R54 D).
fn text_context(name: &str, parent_in_text: bool) -> bool {
    match name {
        "text" => true,
        "tspan" | "textPath" | "a" => parent_in_text,
        _ => false,
    }
}

/// Read one subtree: the letters it renders (its own marker first, then the
/// nested markers' in document order) and the text it draws.
///
/// A marker's letter is the text its **whole** subtree draws, nested markers
/// included, because that is what the renderer paints. When that text is exactly
/// the concatenation of the nested markers' letters the marker contributed no
/// characters of its own, so it is dropped and only the nested ones count (R54 D:
/// `<g label="0"><text label="1">A</text></g>` is one letter, not two, while
/// `<g label="0"><text>A</text><text label="1">B</text></g>` is "AB" plus "B").
fn subtree<'a>(
    ctx: &mut Ctx<'a>,
    element: &'a Element,
    inherited_visible: bool,
    inherited: Paint,
    in_text: bool,
    as_instance: bool,
) -> (Vec<InheritedLetter>, String) {
    let Some(visible) = enter(element, inherited_visible, as_instance) else {
        return (Vec::new(), String::new());
    };
    let paint = inherited.inherit(element, &ctx.ids);
    ctx.ancestors.push(element);
    let mut letters = Vec::new();
    let mut full = String::new();
    if in_text && visible && paint.any() {
        for child in &element.children {
            if let Node::Text(text) = child {
                full.push_str(text);
            }
        }
    }
    for child in &element.children {
        let Node::Element(child) = child else {
            continue;
        };
        // A `<use>` draws its target in the use's own place, so it keeps the
        // surrounding text context; any other element starts a fresh one.
        let child_text = if child.name == "use" {
            in_text
        } else {
            text_context(&child.name, in_text)
        };
        let (inner, text) = subtree(ctx, child, visible, paint, child_text, false);
        letters.extend(inner);
        full.push_str(&text);
    }
    if element.name == "use" {
        expand_use(
            ctx,
            element,
            visible,
            |ctx, target, visible, as_instance| {
                // The instanced element brings its own text context: a `<use>` of a
                // `<text>` draws text even when the `<use>` sits in a `<g>`.
                let target_text = text_context(&target.name, in_text);
                let (inner, text) = subtree(ctx, target, visible, paint, target_text, as_instance);
                letters.extend(inner);
                full.push_str(&text);
            },
        );
    }
    if let Some(marker) = element.attr("data-svg-grid-label") {
        let own = crate::normalize_label(&full);
        if !own.is_empty() {
            let nested: String = letters
                .iter()
                .map(|letter| letter.letter.as_str())
                .collect();
            if own != nested {
                letters.insert(
                    0,
                    InheritedLetter {
                        marker: marker.to_string(),
                        letter: own,
                    },
                );
            }
        }
    }
    ctx.ancestors.pop();
    (letters, full)
}

/// Follow a `<use>`'s reference once. `visit` receives the instanced subtree root
/// and whether it is an instance root (a `<symbol>` is not drawn where it sits but
/// is drawn through the `<use>`). A target already on the ancestor chain is a
/// circular reference: the renderer draws nothing for it, so it is not expanded
/// again.
fn expand_use<'a, F>(ctx: &mut Ctx<'a>, element: &'a Element, visible: bool, mut visit: F)
where
    F: FnMut(&mut Ctx<'a>, &'a Element, bool, bool),
{
    let Some(target) = use_target(element, &ctx.ids) else {
        return;
    };
    // Among the non-rendered containers only a `<symbol>` is drawn through a
    // `<use>`; `<defs>`, `<clipPath>`, `<marker>` and `<pattern>` draw nothing
    // (measured ink 0 in both reference renderers, R54 C).
    let as_instance = NON_RENDERED.contains(&target.name.as_str());
    if as_instance && target.name != "symbol" {
        return;
    }
    // A `<symbol>` with a `viewBox` clips its content to that rectangle; when the
    // instance's ink misses the box the renderer draws nothing (measured ink 0,
    // R56 P3e).
    if as_instance && !crate::label_clip::instance_draws(target) {
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
    visit(ctx, target, visible, as_instance);
    ctx.depth -= 1;
    ctx.ancestors.pop();
}

/// Fold one element's own `display` / `opacity` / `visibility` onto the inherited
/// visibility. `None` means the element and its whole subtree are not rendered:
/// `display:none` and `opacity:0` cannot be undone by a descendant, while
/// `visibility:hidden` can (`visibility="visible"` on a child wins).
fn enter(element: &Element, inherited_visible: bool, as_instance: bool) -> Option<bool> {
    if !as_instance && NON_RENDERED.contains(&element.name.as_str()) {
        return None;
    }
    // `display` does not apply to a `<symbol>` itself (SVG 1.1): a symbol marked
    // display:none is still drawn through `<use>` (measured d1 ink 40, cairosvg
    // 0, R56 P3). A `<g style="display:none">` instance, by contrast, is hidden.
    let display_applies = !(as_instance && element.name == "symbol");
    if display_applies
        && matches!(
            style_prop(element, "display").as_deref().map(str::trim),
            Some("none")
        )
    {
        return None;
    }
    if style_prop(element, "opacity")
        .as_deref()
        .and_then(parse_number)
        .is_some_and(|opacity| opacity <= 0.0)
    {
        return None;
    }
    Some(
        match style_prop(element, "visibility").as_deref().map(str::trim) {
            Some("hidden") | Some("collapse") => false,
            Some("visible") => true,
            _ => inherited_visible,
        },
    )
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

    fn one(marker: &str, letter: &str) -> (String, String) {
        (marker.to_string(), letter.to_string())
    }

    #[test]
    fn a_nested_marker_is_collected_once_by_its_own_element() {
        let found = letters(
            "<g data-svg-grid-label=\"0\">\
             <text data-svg-grid-label=\"1\">A</text></g>",
        );
        assert_eq!(found, vec![one("1", "A")]);
    }

    #[test]
    fn an_outer_marker_reads_the_text_its_subtree_draws() {
        // The renderer draws "AB" once, so the outer marker's letter is "AB" and
        // the nested marker keeps its own "B" (R54 D).
        let found = letters(
            "<g data-svg-grid-label=\"0\"><text>A</text>\
             <text data-svg-grid-label=\"1\">B</text></g>",
        );
        assert_eq!(found, vec![one("0", "AB"), one("1", "B")]);
    }

    #[test]
    fn bare_text_inside_a_group_is_not_drawn() {
        // `<g>A<text>B</text>C</g>` renders only "B" (measured: pixel-identical
        // to `<text>B</text>`), so the outer marker contributes nothing and is
        // dropped.
        let found = letters(
            "<g data-svg-grid-label=\"0\">A\
             <text data-svg-grid-label=\"1\">B</text>C</g>",
        );
        assert_eq!(found, vec![one("1", "B")]);
    }

    #[test]
    fn a_group_inside_text_breaks_the_run() {
        // `<text>A<g>B</g>C</text>` draws "AC" only (measured ink 82 = "AC").
        let found = letters("<text data-svg-grid-label=\"0\">A<g>B</g>C</text>");
        assert_eq!(found, vec![one("0", "AC")]);
    }

    #[test]
    fn a_definition_counts_once_per_use() {
        let one_use = letters(
            "<g><defs><text id=\"L\" data-svg-grid-label=\"0\">A</text></defs>\
             <use href=\"#L\"/></g>",
        );
        assert_eq!(one_use, vec![one("0", "A")]);
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
        assert_eq!(found, vec![one("0", "A")]);
    }

    #[test]
    fn a_symbol_is_drawn_through_use() {
        let found = letters(
            "<g><defs><symbol id=\"S\">\
             <text data-svg-grid-label=\"0\">A</text></symbol></defs>\
             <use href=\"#S\"/></g>",
        );
        assert_eq!(found, vec![one("0", "A")]);
    }

    #[test]
    fn a_marker_on_a_symbol_is_drawn_through_use() {
        // R54 C: the marker sits on the `<symbol>` itself, which is only drawn
        // through the `<use>`; the instanced A is a letter.
        let found = letters(
            "<g><defs><symbol id=\"S\" data-svg-grid-label=\"0\">\
             <text>A</text></symbol></defs><use href=\"#S\"/></g>",
        );
        assert_eq!(found, vec![one("0", "A")]);
        // Unreferenced, the symbol draws nothing.
        let unused = letters(
            "<g><defs><symbol id=\"S\" data-svg-grid-label=\"0\">\
             <text>A</text></symbol></defs></g>",
        );
        assert!(unused.is_empty(), "{unused:?}");
    }

    #[test]
    fn a_use_of_a_non_rendered_container_draws_nothing() {
        // Only a `<symbol>` is instanced; `<defs>` / `<clipPath>` / `<pattern>`
        // draw nothing through a `<use>` (measured ink 0).
        for name in ["defs", "clipPath", "pattern", "marker"] {
            let markup = format!(
                "<g><{name} id=\"T\" data-svg-grid-label=\"0\">\
                 <text>A</text></{name}><use href=\"#T\"/></g>"
            );
            assert!(letters(&markup).is_empty(), "{name} draws nothing");
        }
    }

    #[test]
    fn text_that_paints_nothing_is_not_a_letter() {
        assert!(letters("<text data-svg-grid-label=\"0\" fill=\"none\">A</text>").is_empty());
        assert!(
            letters("<g fill=\"none\"><text data-svg-grid-label=\"0\">A</text></g>").is_empty()
        );
        assert_eq!(
            letters("<g fill=\"none\"><text data-svg-grid-label=\"0\" fill=\"black\">A</text></g>"),
            vec![one("0", "A")]
        );
        assert_eq!(
            letters("<text data-svg-grid-label=\"0\" fill=\"none\" stroke=\"black\">A</text>"),
            vec![one("0", "A")]
        );
    }

    #[test]
    fn an_unresolved_paint_reference_is_not_a_letter() {
        // R54 A: both renderers leave ink 0 for these, so they are not letters.
        for attrs in [
            "fill=\"url(#missing)\"",
            "style=\"fill:url(#missing)\"",
            "fill=\"none\" stroke=\"url(#missing)\"",
            "fill-opacity=\"0\"",
        ] {
            let markup = format!("<text data-svg-grid-label=\"0\" {attrs}>A</text>");
            assert!(letters(&markup).is_empty(), "{attrs} paints nothing");
        }
        // A fill-side wrapper leaves a default-fill glyph unpainted; a
        // stroke-side wrapper needs a stroked glyph to show that it is the
        // wrapper, not the absence of paint, that removes the ink.
        for (wrapper, body) in [
            (
                "<g fill=\"url(#missing)\">",
                "<text data-svg-grid-label=\"0\">A</text>",
            ),
            (
                "<g fill-opacity=\"0\">",
                "<text data-svg-grid-label=\"0\">A</text>",
            ),
            (
                "<g stroke-opacity=\"0\">",
                "<text data-svg-grid-label=\"0\" fill=\"none\" stroke=\"black\">A</text>",
            ),
            (
                "<g stroke-width=\"0\">",
                "<text data-svg-grid-label=\"0\" fill=\"none\" stroke=\"black\">A</text>",
            ),
        ] {
            let markup = format!("{wrapper}{body}</g>");
            assert!(letters(&markup).is_empty(), "{wrapper} paints nothing");
        }
        // A resolvable gradient paints, and so does a fallback colour.
        let gradient = "<defs><linearGradient id=\"g\"><stop offset=\"0\" \
                        stop-color=\"black\"/></linearGradient></defs>";
        assert_eq!(
            letters(&format!(
                "<svg>{gradient}<text data-svg-grid-label=\"0\" fill=\"url(#g)\">A</text></svg>"
            )),
            vec![one("0", "A")]
        );
        assert_eq!(
            letters("<text data-svg-grid-label=\"0\" fill=\"url(#missing) black\">A</text>"),
            vec![one("0", "A")]
        );
    }

    #[test]
    fn a_child_can_restore_an_inherited_paint_property() {
        assert_eq!(
            letters(
                "<g fill-opacity=\"0\"><text data-svg-grid-label=\"0\" \
                 fill-opacity=\"1\">A</text></g>"
            ),
            vec![one("0", "A")]
        );
        assert_eq!(
            letters(
                "<g stroke-width=\"0\"><text data-svg-grid-label=\"0\" stroke-width=\"1\" \
                 fill=\"none\" stroke=\"black\">A</text></g>"
            ),
            vec![one("0", "A")]
        );
        assert_eq!(
            letters("<g fill-opacity=\"0.5\"><text data-svg-grid-label=\"0\">A</text></g>"),
            vec![one("0", "A")]
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
