//! Panel letters: the only text `plot_grid` draws itself.
//!
//! The family comes from `--label-fontfamily` (default `Arial`) and is resolved
//! from `fontlib/` only — see `ARCHITECTURE/fonts.md`. Defaults keep the letter
//! just outside the panel's top-left corner: `hjust = 0` aligns its start with
//! the panel's left edge, `vjust = -0.25` lifts the baseline above the panel top.

use crate::dom;
use crate::geom;
use crate::{format_number, LabelSpec};

pub(crate) fn build(
    index: usize,
    cell: geom::Rect,
    canvas_width: f32,
    labels: &LabelSpec,
) -> Option<dom::Element> {
    let label = labels.labels.get(index)?;
    if label.is_empty() {
        return None;
    }
    let size = labels.size.resolve(canvas_width);
    let x = cell.x + value(&labels.x, index, 0.0) * cell.width
        - value(&labels.hjust, index, 0.0) * size;
    let y = baseline(cell, index, labels, size);
    let mut attrs = vec![
        ("data-svg-grid-label".to_string(), index.to_string()),
        ("data-scale".to_string(), "position".to_string()),
        ("x".to_string(), format_number(x)),
        ("y".to_string(), format_number(y)),
        ("font-family".to_string(), labels.font_family.clone()),
        ("font-size".to_string(), format_number(size)),
        ("fill".to_string(), labels.colour.clone()),
    ];
    apply_font_face(&mut attrs, &labels.font_face);
    Some(dom::Element {
        name: "text".to_string(),
        attrs,
        children: vec![dom::Node::Text(label.clone())],
    })
}

/// Baseline `y` before any canvas clamp: the default `vjust = -0.25` lifts the
/// letter above the cell's top edge, which is what reserves the label band.
pub(crate) fn raw_baseline(cell: geom::Rect, index: usize, labels: &LabelSpec, size: f32) -> f32 {
    cell.y
        + (1.0 - value(&labels.y, index, 1.0)) * cell.height
        + value(&labels.vjust, index, -0.25) * size
}

/// Baseline `y` actually written to the SVG.
///
/// A caller can pin `--plot-margin 0`, which leaves no room for the default
/// above-the-panel baseline. Letters are never allowed to leave the canvas: the
/// baseline is pushed down until the cap height fits, so the glyph lands just
/// inside the cell's top edge instead of being clipped away.
pub(crate) fn baseline(cell: geom::Rect, index: usize, labels: &LabelSpec, size: f32) -> f32 {
    raw_baseline(cell, index, labels, size).max(crate::LABEL_CAP_HEIGHT_RATIO * size)
}

/// Per-letter override, falling back to the first value and then to `fallback`.
pub(crate) fn value(values: &[f32], index: usize, fallback: f32) -> f32 {
    values
        .get(index)
        .copied()
        .or_else(|| values.first().copied())
        .unwrap_or(fallback)
}

fn apply_font_face(attrs: &mut Vec<(String, String)>, font_face: &str) {
    let normalized = font_face.replace(['_', '-', '.'], "").to_lowercase();
    if normalized.contains("bold") {
        attrs.push(("font-weight".to_string(), "bold".to_string()));
    }
    if normalized.contains("italic") {
        attrs.push(("font-style".to_string(), "italic".to_string()));
    }
}

/// Spreadsheet-style lettering: 0 → `A`, 25 → `Z`, 26 → `AA`.
pub(crate) fn alphabet_label(index: usize, uppercase: bool) -> String {
    let mut number = index + 1;
    let mut chars = Vec::new();
    while number > 0 {
        number -= 1;
        let base = if uppercase { b'A' } else { b'a' };
        chars.push((base + (number % 26) as u8) as char);
        number /= 26;
    }
    chars.iter().rev().collect()
}
