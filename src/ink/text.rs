//! Proportional ink estimate for `text` / `tspan` runs.

use crate::dom::{Element, Node};
use crate::{LABEL_CAP_HEIGHT_RATIO, LABEL_INK_WIDTH_RATIO};

use super::bbox::Bbox;
use super::walk::{number_attr, style_prop, Ctx};

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
    match style_prop(element, "text-anchor") {
        Some("middle") => TextAnchor::Middle,
        Some("end") => TextAnchor::End,
        _ => TextAnchor::Start,
    }
}

fn baseline(element: &Element) -> Baseline {
    match style_prop(element, "dominant-baseline") {
        Some("middle" | "central") => Baseline::Middle,
        Some("hanging" | "text-before-edge" | "before-edge") => Baseline::Hanging,
        Some("text-after-edge" | "after-edge") => Baseline::AfterEdge,
        _ => Baseline::Alphabetic,
    }
}
