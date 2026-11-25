//! Proportional ink estimate for `text` / `tspan` runs.

use std::collections::HashMap;

use crate::dom::{Element, Node};
use crate::{LABEL_CAP_HEIGHT_RATIO, LABEL_INK_WIDTH_RATIO};

use super::bbox::Bbox;
use super::walk::{inherit_ctx, is_display_none, number_attr, style_prop, Ctx, NON_RENDERED};

/// Descender below the baseline as a fraction of the font size (DejaVu
/// descenders reach ≈0.24 em; rounded up). Used only by the text estimate.
const TEXT_DESCENDER_RATIO: f64 = 0.25;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Baseline {
    Alphabetic,
    Middle,
    Hanging,
    AfterEdge,
}

/// Proportional ink box of one text run (see the module docs).
pub(super) fn add_text_box(element: &Element, ctx: &Ctx, bbox: &mut Bbox) {
    let characters = collect_text(element).trim().chars().count();
    if characters == 0 {
        return;
    }
    let size = ctx.font_size;
    let x = number_attr(element, "x").unwrap_or(0.0);
    let y = number_attr(element, "y").unwrap_or(0.0);
    // Same constant/approach as `label_ink_box`: an upper bound on the ink width
    // of one character.
    let width = LABEL_INK_WIDTH_RATIO as f64 * size * characters as f64;
    let left = match text_anchor(element) {
        TextAnchor::Start => x,
        TextAnchor::Middle => x - width / 2.0,
        TextAnchor::End => x - width,
    };
    let cap = LABEL_CAP_HEIGHT_RATIO as f64 * size;
    let (top, bottom) = match baseline(element) {
        Baseline::Alphabetic => (y - cap, y + TEXT_DESCENDER_RATIO * size),
        Baseline::Middle => (y - size / 2.0, y + size / 2.0),
        Baseline::Hanging => (y, y + size),
        Baseline::AfterEdge => (y - size, y),
    };
    bbox.add_rect(left, top, left + width, bottom);
}

/// True when at least one character of this text run really leaves ink, decided
/// by the element that carries the glyphs rather than by the outer element alone
/// (R66b).
///
/// `<text fill="none"><tspan fill="#000">A</tspan></text>` draws the `A`
/// (measured d1 ink 92, cairosvg 95), and a `visibility="visible"` tspan restores
/// glyphs inside a `visibility="hidden"` text — the outer context decides nothing
/// for those. Conversely `<text fill="#000"><tspan fill="none">A</tspan></text>`
/// is blank in both renderers even though the outer paint is black.
///
/// A `<tspan>`'s **own** `opacity` is deliberately ignored: resvg 0.47 still inks
/// its glyphs fully (`<text fill="#000"><tspan opacity="0">A</tspan></text>`
/// measures d1 92 / cairosvg 0), and this tool follows resvg as R51/R54 did. An
/// `opacity` on a non-`tspan` descendant *is* honoured (`<g opacity="0">` inside
/// the text hides its glyphs, measured d1 0). `fill-opacity`, `visibility` and
/// `display` are honoured everywhere (measured d1 0).
pub(super) fn text_paints(element: &Element, ctx: &Ctx, ids: &HashMap<&str, &Element>) -> bool {
    for child in &element.children {
        match child {
            Node::Text(text) => {
                if !text.trim().is_empty() && ctx.paints() {
                    return true;
                }
            }
            Node::Element(child) => {
                if NON_RENDERED.contains(&child.name.as_str()) || is_display_none(child) {
                    continue;
                }
                let mut child_ctx = inherit_ctx(child, ctx, ids);
                if child.name == "tspan" {
                    child_ctx.opaque = ctx.opaque;
                }
                if !child_ctx.opaque {
                    continue;
                }
                if text_paints(child, &child_ctx, ids) {
                    return true;
                }
            }
        }
    }
    false
}

fn collect_text(element: &Element) -> String {
    let mut out = String::new();
    gather_text(element, &mut out);
    out
}

fn gather_text(element: &Element, out: &mut String) {
    for child in &element.children {
        match child {
            Node::Text(text) => out.push_str(text),
            Node::Element(child) => gather_text(child, out),
        }
    }
}

fn text_anchor(element: &Element) -> TextAnchor {
    match style_prop(element, "text-anchor").as_deref() {
        Some("middle") => TextAnchor::Middle,
        Some("end") => TextAnchor::End,
        _ => TextAnchor::Start,
    }
}

fn baseline(element: &Element) -> Baseline {
    match style_prop(element, "dominant-baseline").as_deref() {
        Some("middle" | "central") => Baseline::Middle,
        Some("hanging" | "text-before-edge" | "before-edge") => Baseline::Hanging,
        Some("text-after-edge" | "after-edge") => Baseline::AfterEdge,
        _ => Baseline::Alphabetic,
    }
}
