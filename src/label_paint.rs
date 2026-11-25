//! Whether a marker's text actually leaves ink.
//!
//! A panel letter is only a letter when the renderer draws it: `fill="none"`,
//! a zero opacity or a zero stroke width paint nothing, and so does a `url(#id)`
//! paint reference that does not resolve to a paint server that itself paints
//! (R54). Everything here is decided by what both reference renderers do; the
//! comments name the measured case where a rule is not obvious.
//!
//! The paint model is the SVG presentation context, one property at a time:
//! `fill`, `fill-opacity`, `stroke`, `stroke-opacity` and `stroke-width` are all
//! inherited, so a `<g fill-opacity="0">` silences its whole subtree but a child
//! may restore any one of them (R54 A5-A8, E3-E5).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::dom::{Element, Node};
use crate::ink::{parse_number, style_prop, subtree_inks, NON_RENDERED};

/// Effective paint state of one text run.
#[derive(Clone, Copy)]
pub(crate) struct Paint {
    /// `fill` resolves to something that paints (default: black).
    fill: bool,
    /// Effective inherited `fill-opacity` (default 1).
    fill_opacity: f64,
    /// `stroke` resolves to something that paints (default: none).
    stroke: bool,
    /// Effective inherited `stroke-opacity` (default 1).
    stroke_opacity: f64,
    /// Effective inherited `stroke-width` (default 1; negative means no stroke).
    stroke_width: f64,
    /// Effective inherited `color` (default: black) — what a `currentColor`
    /// paint resolves to (R66b).
    color: bool,
}

impl Paint {
    pub(crate) const DEFAULT: Paint = Paint {
        fill: true,
        fill_opacity: 1.0,
        stroke: false,
        stroke_opacity: 1.0,
        stroke_width: 1.0,
        color: true,
    };

    /// True when the run leaves ink: a painted fill with a non-zero opacity, or a
    /// painted stroke with a non-zero opacity and a positive width.
    pub(crate) fn any(self) -> bool {
        self.fill_paints() || self.stroke_paints()
    }

    /// True when the fill alone leaves ink (a painted fill at a non-zero
    /// opacity). Closed shapes and text are filled; a `<line>` never is.
    pub(crate) fn fill_paints(self) -> bool {
        self.fill && self.fill_opacity > 0.0
    }

    /// True when the stroke alone leaves ink: a painted stroke at a non-zero
    /// opacity and a **positive** width. A negative width is invalid and paints
    /// nothing (measured: d1render and cairosvg both leave the canvas blank).
    pub(crate) fn stroke_paints(self) -> bool {
        self.stroke && self.stroke_opacity > 0.0 && self.stroke_width > 0.0
    }

    /// Effective `stroke-width` (negative means no stroke, see
    /// [`Paint::stroke_paints`]).
    pub(crate) fn stroke_width(self) -> f64 {
        self.stroke_width
    }

    /// Fold the element's own paint properties onto the inherited state. A
    /// property the element does not declare stays inherited; one it does declare
    /// replaces the inherited value even when the value is invalid, exactly like a
    /// CSS declaration.
    pub(crate) fn inherit(self, element: &Element, ids: &HashMap<&str, &Element>) -> Paint {
        let mut next = self;
        // `color` is inherited and is what `currentColor` resolves to, so it must
        // be folded before the fill/stroke that may read it (R66b).
        if let Some(value) = style_prop(element, "color") {
            next.color = colour_paints(value.trim());
        }
        if let Some(value) = style_prop(element, "fill") {
            next.fill = paint_value(value.trim(), ids, next.color);
        }
        if let Some(value) = style_prop(element, "fill-opacity")
            .as_deref()
            .and_then(parse_number)
        {
            next.fill_opacity = value;
        }
        if let Some(value) = style_prop(element, "stroke") {
            next.stroke = paint_value(value.trim(), ids, next.color);
        }
        if let Some(value) = style_prop(element, "stroke-opacity")
            .as_deref()
            .and_then(parse_number)
        {
            next.stroke_opacity = value;
        }
        if let Some(value) = style_prop(element, "stroke-width")
            .as_deref()
            .and_then(parse_number)
        {
            next.stroke_width = value;
        }
        next
    }
}

/// Does the paint value `value` paint anything?
///
/// A `url(#id)` reference paints when `id` resolves to a paint server that paints
/// (see [`paint_server_paints`]). Two measured corner cases decide the rest:
///
/// * an *unresolved* id falls back to the colour after the `)`, so
///   `url(#missing) black` draws black and `url(#missing)` alone draws nothing;
/// * a syntactically broken `url(...)` (`url(`, `url()`, `url(#)`, no `)`) is not
///   a colour at all, and resvg then paints the initial black — measured ink 40,
///   so it is treated as painted.
fn paints(value: &str, ids: &HashMap<&str, &Element>) -> bool {
    let value = value.trim();
    let Some(rest) = value.strip_prefix("url(") else {
        // Case-sensitive on purpose: `URL(#missing)` is not a reference and both
        // renderers draw it black, as they do for any other colour keyword.
        return colour_paints(value);
    };
    let Some((id, fallback)) = parse_reference(rest) else {
        return true;
    };
    match ids.get(id) {
        Some(target) => paint_server_paints(target, ids),
        None => fallback.is_some_and(|fallback| paints(fallback, ids)),
    }
}

/// Does a paint property value paint, given the effective `color` of the run?
///
/// `currentColor` is the one keyword whose meaning is not local: it is the
/// inherited `color` (default black). Measured (R66b): with
/// `color="transparent"`, `fill="currentColor"` leaves both canvases blank while
/// the old judge warned; with `color="#000"` it inks 400 px. The keyword is
/// **case-sensitive** — `fill="currentcolor"` is an invalid colour that both
/// renderers draw black (measured 400 px), so only the exact spelling resolves.
fn paint_value(value: &str, ids: &HashMap<&str, &Element>, current_color: bool) -> bool {
    if value == "currentColor" {
        current_color
    } else {
        paints(value, ids)
    }
}

/// Does a plain colour paint? `none`/`transparent` never do, and a colour whose
/// alpha channel is zero is invisible to resvg even though it is a real colour:
/// `fill="#00000000"`, `fill="rgba(0,0,0,0)"` and `stop-color="#00000000"` all
/// leave ink 0 (measured, R56 P2.2). cairosvg still paints a zero-alpha hex
/// colour but agrees on `rgba()`; this follows resvg, as R51 did for case.
fn colour_paints(value: &str) -> bool {
    if matches!(value, "none" | "transparent") {
        return false;
    }
    colour_alpha(value) != Some(0.0)
}

/// The alpha channel of a colour literal, when it carries one: `#rrggbbaa` and
/// `rgba(r,g,b,a)`. `None` means "opaque or not a colour we parse".
fn colour_alpha(value: &str) -> Option<f64> {
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 8 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return u8::from_str_radix(&hex[6..], 16)
                .ok()
                .map(|alpha| f64::from(alpha) / 255.0);
        }
        return None;
    }
    let inner = value.strip_prefix("rgba(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    parts[3].trim().parse::<f64>().ok()
}

/// Split the inside of `url(...)` into the local id and the optional fallback
/// colour. `None` means the reference is malformed (and therefore paints black).
fn parse_reference(rest: &str) -> Option<(&str, Option<&str>)> {
    let close = rest.find(')')?;
    let inside = rest[..close].trim();
    // Both renderers accept `url('#id')`; the quotes are not part of the id.
    let inside = inside.trim_matches(['\'', '"']).trim();
    let id = inside.strip_prefix('#')?.trim();
    if id.is_empty() {
        return None;
    }
    let fallback = rest[close + 1..].trim();
    Some((id, (!fallback.is_empty()).then_some(fallback)))
}

/// A paint server paints when it has content that paints. An id that resolves to
/// a non-paint-server (a `<rect>`, a `<text>`, …) paints nothing: measured ink 0
/// in both renderers for `fill="url(#some-rect)"`.
fn paint_server_paints(target: &Element, ids: &HashMap<&str, &Element>) -> bool {
    match target.name.as_str() {
        "linearGradient" | "radialGradient" => gradient_paints(target, ids),
        "pattern" => pattern_paints(target, ids),
        _ => false,
    }
}

/// A gradient paints when at least one stop does. A gradient with no stops of its
/// own inherits them from its `href` chain, however long that chain is (R56 P3);
/// an empty one paints nothing (measured ink 0 for `<linearGradient id="g"/>`),
/// while an all-transparent or `stop-opacity="0"` one also paints nothing.
fn gradient_paints(gradient: &Element, ids: &HashMap<&str, &Element>) -> bool {
    // Walk the whole `href` chain, not a fixed number of hops: resvg resolves a
    // ten-deep chain (measured ink 40, R56 P3), and the old depth-4 cap dropped
    // those letters. A pointer seen twice means the chain is circular; neither
    // renderer draws anything for `a --href--> b --href--> a` (measured ink 0).
    let mut seen: Vec<*const Element> = Vec::new();
    let mut current = gradient;
    loop {
        if seen.iter().any(|node| std::ptr::eq(*node, current)) {
            return false;
        }
        seen.push(current);
        let mut has_stop = false;
        for child in &current.children {
            let Node::Element(stop) = child else {
                continue;
            };
            if stop.name != "stop" {
                continue;
            }
            has_stop = true;
            if stop_paints(stop) {
                return true;
            }
        }
        if has_stop {
            return false;
        }
        let Some(next) = current
            .attr("href")
            .or_else(|| current.attr("xlink:href"))
            .map(str::trim)
            .and_then(|href| href.strip_prefix('#'))
            .and_then(|id| ids.get(id))
        else {
            return false;
        };
        current = next;
    }
}

/// One `<stop>` paints unless its colour paints nothing — literally `transparent`
/// or a zero alpha channel (measured ink 0, R56 P2.2) — or its `stop-opacity` is
/// zero. `stop-color="none"` is invalid and resvg paints it black, so it is *not*
/// treated as transparent (measured d1 ink 40; cairosvg disagrees — R54 follows
/// resvg, as R51 did).
fn stop_paints(stop: &Element) -> bool {
    let color = style_prop(stop, "stop-color");
    let color = color.as_deref().map(str::trim).unwrap_or("black");
    if color.eq_ignore_ascii_case("transparent") || !colour_paints(color) {
        return false;
    }
    style_prop(stop, "stop-opacity")
        .as_deref()
        .and_then(parse_number)
        .unwrap_or(1.0)
        > 0.0
}

thread_local! {
    /// Patterns whose content is being judged right now. A tile that paints
    /// `url(#itself)` must not be walked forever: resvg draws nothing for it
    /// (measured d1 ink 0; cairosvg 2.9.1 dies with exit 1), so re-entering a
    /// pattern under evaluation judges it as non-painting.
    static EVALUATING_PATTERNS: RefCell<Vec<*const Element>> = const { RefCell::new(Vec::new()) };
}

/// A pattern paints when its content does. The tile is a rendered subtree, so the
/// question is exactly "would [`crate::ink::subtree_inks`] measure ink inside it?"
/// — a child that exists but paints nothing (`fill="none"`, `transparent`,
/// `fill-opacity="0"`, `opacity="0"`, `visibility="hidden"`, `display="none"`)
/// leaves no ink (measured d1 0 for all five, R66b P2-1), while a child that
/// restores a paint inside a non-painting group still does (measured d1 400).
///
/// A pattern with no content of its own instead inherits the content of its
/// `href`/`xlink:href` chain, however long (R66b P1-2: `<pattern id="p"
/// href="#b"/>` where `b` has a solid rect measures d1 ink 400). A `<title>`
/// child is not content and does not stop the chain (measured d1 400), while a
/// real but non-painting child (`display="none"`) does (measured d1 0) — resvg
/// only falls back to `href` when the pattern has no rendered children at all.
fn pattern_paints(pattern: &Element, ids: &HashMap<&str, &Element>) -> bool {
    let reentered = EVALUATING_PATTERNS.with(|stack| {
        let mut stack = stack.borrow_mut();
        if stack.iter().any(|node| std::ptr::eq(*node, pattern)) {
            return true;
        }
        stack.push(pattern);
        false
    });
    if reentered {
        return false;
    }
    let result = pattern_paints_inner(pattern, ids);
    EVALUATING_PATTERNS.with(|stack| {
        stack.borrow_mut().pop();
    });
    result
}

fn pattern_paints_inner(pattern: &Element, ids: &HashMap<&str, &Element>) -> bool {
    let mut seen: Vec<*const Element> = Vec::new();
    let mut current = pattern;
    loop {
        if seen.iter().any(|node| std::ptr::eq(*node, current)) {
            return false;
        }
        seen.push(current);
        let mut has_content = false;
        for child in &current.children {
            let Node::Element(child) = child else {
                continue;
            };
            // `<title>`, `<desc>`, `<style>`, `<defs>`, … are not tile content.
            if NON_RENDERED.contains(&child.name.as_str()) {
                continue;
            }
            has_content = true;
            if subtree_inks(child, ids) {
                return true;
            }
        }
        if has_content {
            return false;
        }
        let Some(next) = current
            .attr("href")
            .or_else(|| current.attr("xlink:href"))
            .map(str::trim)
            .and_then(|href| href.strip_prefix('#'))
            .and_then(|id| ids.get(id))
        else {
            return false;
        };
        current = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    fn paints_in(markup: &str, value: &str) -> bool {
        let root = dom::parse(markup).expect("test markup parses");
        let mut ids = HashMap::new();
        let mut stack = vec![&root];
        while let Some(element) = stack.pop() {
            if let Some(id) = element.attr("id") {
                ids.entry(id).or_insert(element);
            }
            for child in &element.children {
                if let Node::Element(child) = child {
                    stack.push(child);
                }
            }
        }
        paints(value, &ids)
    }

    #[test]
    fn an_unresolved_reference_paints_nothing_but_its_fallback_does() {
        let doc = "<svg><defs><linearGradient id=\"g\"><stop offset=\"0\" \
                   stop-color=\"black\"/></linearGradient></defs></svg>";
        assert!(!paints_in(doc, "url(#missing)"));
        assert!(paints_in(doc, "url(#missing) black"));
        assert!(!paints_in(doc, "url(#missing) none"));
        assert!(paints_in(doc, "url(#g)"));
        // A non-paint-server target paints nothing (measured ink 0), even with a
        // fallback: resvg ignores the fallback once the id resolves.
        let rect = "<svg><rect id=\"r\" width=\"1\" height=\"1\"/></svg>";
        assert!(!paints_in(rect, "url(#r)"));
        assert!(!paints_in(rect, "url(#r) black"));
    }

    #[test]
    fn a_malformed_reference_is_not_a_colour_and_paints_black() {
        for value in ["url(#", "url()", "url(#)", "url(#missing"] {
            assert!(paints_in("<svg/>", value), "{value} draws black in resvg");
        }
        // Spelled in caps it is not a reference at all.
        assert!(paints_in("<svg/>", "URL(#missing)"));
    }

    #[test]
    fn a_gradient_needs_one_stop_that_paints() {
        assert!(!paints_in(
            "<svg><linearGradient id=\"g\"/></svg>",
            "url(#g)"
        ));
        assert!(paints_in(
            "<svg><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"black\"/>\
             <stop offset=\"1\" stop-color=\"white\"/></linearGradient></svg>",
            "url(#g)"
        ));
        assert!(!paints_in(
            "<svg><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"transparent\"/>\
             </linearGradient></svg>",
            "url(#g)"
        ));
        assert!(!paints_in(
            "<svg><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"black\" \
             stop-opacity=\"0\"/></linearGradient></svg>",
            "url(#g)"
        ));
        // The stop is inherited through the href chain.
        assert!(paints_in(
            "<svg><linearGradient id=\"base\"><stop offset=\"0\" stop-color=\"black\"/>\
             </linearGradient><radialGradient id=\"g\" href=\"#base\"/></svg>",
            "url(#g)"
        ));
    }

    #[test]
    fn a_zero_alpha_colour_paints_nothing() {
        for value in ["#00000000", "rgba(0,0,0,0)", "rgba(1,2,3,0.0)"] {
            assert!(!paints_in("<svg/>", value), "{value} is invisible");
        }
        for value in ["#000000ff", "#00000080", "rgba(0,0,0,0.5)"] {
            assert!(paints_in("<svg/>", value), "{value} leaves ink");
        }
        // Not a colour alpha: leave these to the ordinary path.
        assert!(paints_in("<svg/>", "#fff"));
        assert!(paints_in("<svg/>", "rgb(0,0,0)"));
    }

    #[test]
    fn a_gradient_href_chain_has_no_fixed_depth_and_stops_on_a_cycle() {
        let mut chain = String::from("<svg><linearGradient id=\"g0\"><stop offset=\"0\" stop-color=\"black\"/></linearGradient>");
        for i in 1..=10 {
            chain.push_str(&format!(
                "<linearGradient id=\"g{i}\" href=\"#g{}\" />",
                i - 1
            ));
        }
        chain.push_str("</svg>");
        assert!(paints_in(&chain, "url(#g10)"));
        let cycle =
            "<svg><linearGradient id=\"a\" href=\"#b\"/><linearGradient id=\"b\" href=\"#a\"/></svg>";
        assert!(!paints_in(cycle, "url(#a)"));
    }

    #[test]
    fn a_pattern_needs_a_drawn_child() {
        assert!(!paints_in("<svg><pattern id=\"p\"/></svg>", "url(#p)"));
        assert!(paints_in(
            "<svg><pattern id=\"p\"><rect width=\"1\" height=\"1\"/></pattern></svg>",
            "url(#p)"
        ));
        assert!(!paints_in(
            "<svg><pattern id=\"p\"><rect width=\"1\" height=\"1\" display=\"none\"/>\
             </pattern></svg>",
            "url(#p)"
        ));
    }
}
