//! Tree traversal, the inherited presentation context, and the shared attribute
//! helpers — the entry point [`ink_bbox`].

use std::borrow::Cow;
use std::collections::HashMap;

use crate::dom::{Element, Node};

use super::bbox::{element_transform, Bbox};
use super::shapes::own_box;
use super::uses::collect_ids;

/// Fallback font size when neither the element nor an ancestor declares one (the
/// SVG initial value of `medium` is 16 px).
const DEFAULT_FONT_SIZE: f64 = 16.0;

/// Containers whose subtree is not drawn where it sits: `<defs>` content is
/// pulled in by a `<use>` (resolved separately against [`collect_ids`]), while
/// the rest is never drawn at all. None of their geometry is counted in place.
pub(crate) const NON_RENDERED: [&str; 12] = [
    "defs", "clipPath", "mask", "symbol", "pattern", "marker", "metadata", "title", "desc",
    "style", "script", "filter",
];

/// Inherited presentation state that the primitives below need.
#[derive(Clone, Copy)]
pub(super) struct Ctx {
    /// Whether a stroke is painted at all (SVG default: no).
    pub(super) stroke_painted: bool,
    /// Effective `stroke-width` (SVG default: 1).
    pub(super) stroke_width: f64,
    /// Effective `font-size` (SVG default: `medium` = 16 px).
    pub(super) font_size: f64,
    /// True once we are inside a `<text>`: its descendant `<tspan>`s are then
    /// covered by the enclosing text's estimate and must not be counted twice.
    pub(super) in_text: bool,
}

impl Default for Ctx {
    fn default() -> Self {
        Self {
            stroke_painted: false,
            stroke_width: 1.0,
            font_size: DEFAULT_FONT_SIZE,
            in_text: false,
        }
    }
}

impl Ctx {
    /// Half the stroke width when a stroke is painted, else `0` (the outward ink
    /// a centred stroke adds beyond the geometric outline).
    pub(super) fn half_stroke(&self) -> f64 {
        if self.stroke_painted {
            (self.stroke_width / 2.0).abs()
        } else {
            0.0
        }
    }
}

/// Ink bounding box of the content of `root`, as `(min_x, min_y, max_x, max_y)`
/// in the root's user coordinate system, or `None` when nothing measurable is
/// drawn.
///
/// This is the W6 judge input: the caller compares it with
/// [`crate::geom::find_canvas_box`]. The result is an *estimate* for text runs (see
/// the module docs); geometric primitives are exact up to the stroke expansion.
pub fn ink_bbox(root: &Element) -> Option<(f64, f64, f64, f64)> {
    let ids = collect_ids(root);
    let mut bbox = Bbox::empty();
    visit(root, &Ctx::default(), &ids, 0, &mut bbox);
    (!bbox.is_empty()).then_some((bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y))
}

pub(super) fn visit(
    element: &Element,
    ctx: &Ctx,
    ids: &HashMap<&str, &Element>,
    depth: u32,
    bbox: &mut Bbox,
) {
    if NON_RENDERED.contains(&element.name.as_str()) || is_display_none(element) {
        return;
    }
    let ctx = inherit_ctx(element, ctx);

    // The element's own geometry is measured in its *local* coordinate system
    // (before its own `transform`), then mapped through that transform and folded
    // into the shared box. An unsupported transform yields `None`, in which case
    // the element's own box is deliberately dropped rather than measured at a
    // position the transform would have moved it away from (see the module docs).
    let mut local = Bbox::empty();
    own_box(element, &ctx, ids, depth, &mut local);
    if let Some(transform) = element_transform(element) {
        local.transform_into(&transform, bbox);
    }

    let child_ctx = if element.name == "text" {
        Ctx {
            in_text: true,
            ..ctx
        }
    } else {
        ctx
    };
    for child in &element.children {
        if let Node::Element(child) = child {
            visit(child, &child_ctx, ids, depth, bbox);
        }
    }
}

/// Fold the element's own presentation properties onto the inherited context.
fn inherit_ctx(element: &Element, ctx: &Ctx) -> Ctx {
    let mut next = *ctx;
    if let Some(stroke) = style_prop(element, "stroke") {
        next.stroke_painted = !matches!(stroke.as_ref(), "none" | "transparent");
    }
    if let Some(width) = style_prop(element, "stroke-width").and_then(|v| parse_number(&v)) {
        next.stroke_width = width;
    }
    if let Some(size) = style_prop(element, "font-size").and_then(|v| parse_number(&v)) {
        next.font_size = size;
    }
    next
}

fn is_display_none(element: &Element) -> bool {
    matches!(style_prop(element, "display").as_deref(), Some("none"))
}

/// Read a presentation property, honouring the inline `style="..."` declaration
/// list first (it outranks a same-named presentation attribute) and falling back
/// to the attribute itself.
///
/// The inline list is a real cascade (R54): `/* … */` comments are stripped, when
/// the same property is declared twice the later declaration wins, and a
/// declaration carrying `!important` outranks one without it. Among *equal*
/// `!important` declarations the **first** one wins: resvg 0.47 keeps the first
/// and cairosvg keeps the last (measured, R56 P1), and this follows resvg as R51
/// did for keyword case.
///
/// The renderers also disagree on whether a `/* … */` comment is stripped at all
/// (`display:none /* c */` is `none` for resvg but a literal value for
/// cairosvg); again resvg wins.
pub(crate) fn style_prop<'a>(element: &'a Element, prop: &str) -> Option<Cow<'a, str>> {
    if let Some(style) = element.attr("style") {
        let style = strip_comments(style);
        let mut plain: Option<&str> = None;
        let mut important: Option<&str> = None;
        for declaration in style.split(';') {
            let Some((name, value)) = declaration.split_once(':') else {
                continue;
            };
            if name.trim() != prop {
                continue;
            }
            match split_important(value) {
                // First `!important` wins (resvg 0.47): a later one must not
                // replace it, or `display:block !important;display:none
                // !important` would hide a letter resvg draws (R56 P1).
                (value, true) => {
                    important.get_or_insert(value);
                }
                (value, false) => plain = Some(value),
            }
        }
        let winner = important.or(plain);
        if let Some(value) = winner {
            return Some(Cow::Owned(value.to_string()));
        }
    }
    element.attr(prop).map(Cow::Borrowed)
}

/// Drop `/* … */` comments from a declaration list, leaving a single space behind
/// so two tokens never fuse.
fn strip_comments(style: &str) -> Cow<'_, str> {
    if !style.contains("/*") {
        return Cow::Borrowed(style);
    }
    let mut out = String::with_capacity(style.len());
    let mut rest = style;
    loop {
        let Some(start) = rest.find("/*") else {
            out.push_str(rest);
            return Cow::Owned(out);
        };
        out.push_str(&rest[..start]);
        out.push(' ');
        let after = &rest[start + 2..];
        match after.find("*/") {
            Some(end) => rest = &after[end + 2..],
            None => return Cow::Owned(out),
        }
    }
}

/// Split a trailing `!important` from a declaration value. Any trailing `!` is
/// dropped, whatever follows it: resvg treats `display:none !` and
/// `display:none ! foo` as `none` (measured, R56 P1), so the lagging token must
/// not leak into the value we compare keywords against.
fn split_important(value: &str) -> (&str, bool) {
    let value = value.trim();
    let Some((head, _flag)) = value.rsplit_once('!') else {
        return (value, false);
    };
    (head.trim_end(), true)
}

/// Read a numeric presentation attribute through [`parse_number`].
pub(super) fn number_attr(element: &Element, name: &str) -> Option<f64> {
    element.attr(name).and_then(parse_number)
}

/// Parse a length's numeric prefix (`"12"`, `"12px"`, `"1.5em"`), mirroring the
/// leniency of `style::scale_token`. Returns `None` when no numeric prefix exists.
pub(crate) fn parse_number(value: &str) -> Option<f64> {
    let value = value.trim();
    if let Ok(number) = value.parse::<f64>() {
        return Some(number);
    }
    let mut best = None;
    for (index, _) in value.char_indices() {
        if index == 0 {
            continue;
        }
        if let Ok(number) = value[..index].parse::<f64>() {
            best = Some(number);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style_of(markup: &str, prop: &str) -> Option<String> {
        let root = crate::dom::parse(markup).expect("test markup parses");
        let text = root
            .children
            .iter()
            .find_map(|node| match node {
                Node::Element(element) if element.name == "text" => Some(element),
                _ => None,
            })
            .expect("a <text> child");
        style_prop(text, prop).map(|value| value.into_owned())
    }

    #[test]
    fn style_prop_keeps_the_first_important_declaration() {
        // resvg 0.47: the first equal `!important` wins (measured, R56 P1).
        let both = r#"<svg><text style="display:block !important;display:none !important"/></svg>"#;
        assert_eq!(style_of(both, "display").as_deref(), Some("block"));
        // An important declaration still outranks a plain one, wherever it sits.
        let mixed = r#"<svg><text style="display:none !important;display:block"/></svg>"#;
        assert_eq!(style_of(mixed, "display").as_deref(), Some("none"));
    }

    #[test]
    fn style_prop_lets_the_later_plain_declaration_win() {
        let later = r#"<svg><text style="display:none;display:block"/></svg>"#;
        assert_eq!(style_of(later, "display").as_deref(), Some("block"));
        let later_none = r#"<svg><text style="display:block;display:none"/></svg>"#;
        assert_eq!(style_of(later_none, "display").as_deref(), Some("none"));
    }

    #[test]
    fn style_prop_strips_comments_and_a_stray_bang() {
        let comment = r#"<svg><text style="display:none /* c *//* d */"/></svg>"#;
        assert_eq!(style_of(comment, "display").as_deref(), Some("none"));
        // Anything after the last `!` is dropped, `important` or not (R56 P1).
        for style in ["display:none !", "display:none ! foo"] {
            let markup = format!(r#"<svg><text style="{style}"/></svg>"#);
            assert_eq!(
                style_of(&markup, "display").as_deref(),
                Some("none"),
                "{style}"
            );
        }
    }
}
