//! Per-element *own* geometry: the axis-aligned box each supported primitive
//! contributes, before the element's own `transform` is applied.
//!
//! This is the "warning face" of W6 — the element set the composer can draw.

use std::collections::HashMap;

use crate::dom::Element;
use crate::path;

use super::bbox::Bbox;
use super::text::add_text_box;
use super::uses::{add_resolved_use, add_use_placement, UseInk};
use super::walk::{number_attr, Ctx};

/// Measure the element's own geometry into `local` (in its *local* coordinate
/// system, before its own `transform`). Elements outside the supported set add
/// nothing.
pub(super) fn own_box(
    element: &Element,
    ctx: &Ctx,
    ids: &HashMap<&str, &Element>,
    depth: u32,
    local: &mut Bbox,
) {
    match element.name.as_str() {
        "circle" => {
            let cx = number_attr(element, "cx").unwrap_or(0.0);
            let cy = number_attr(element, "cy").unwrap_or(0.0);
            let r = number_attr(element, "r").unwrap_or(0.0).abs();
            let s = ctx.half_stroke();
            local.add_rect(cx - r - s, cy - r - s, cx + r + s, cy + r + s);
        }
        // An `<image>` covers exactly its placement box; a `<rect>`'s rounded
        // corners stay inside it, so ignoring `rx`/`ry` only ever over-estimates.
        "rect" | "image" => {
            let x = number_attr(element, "x").unwrap_or(0.0);
            let y = number_attr(element, "y").unwrap_or(0.0);
            let width = number_attr(element, "width").unwrap_or(0.0);
            let height = number_attr(element, "height").unwrap_or(0.0);
            let s = if element.name == "rect" {
                ctx.half_stroke()
            } else {
                0.0
            };
            local.add_rect(x - s, y - s, x + width + s, y + height + s);
        }
        "line" => {
            let x1 = number_attr(element, "x1").unwrap_or(0.0);
            let y1 = number_attr(element, "y1").unwrap_or(0.0);
            let x2 = number_attr(element, "x2").unwrap_or(0.0);
            let y2 = number_attr(element, "y2").unwrap_or(0.0);
            let s = ctx.half_stroke();
            local.add_rect(
                x1.min(x2) - s,
                y1.min(y2) - s,
                x1.max(x2) + s,
                y1.max(y2) + s,
            );
        }
        "polyline" | "polygon" => {
            let points = parse_points(element.attr("points").unwrap_or(""));
            if let (Some(min_x), Some(min_y)) = (
                points.iter().map(|p| p.0).reduce(f64::min),
                points.iter().map(|p| p.1).reduce(f64::min),
            ) {
                let max_x = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
                let max_y = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                let s = ctx.half_stroke();
                local.add_rect(min_x - s, min_y - s, max_x + s, max_y + s);
            }
        }
        // A `<path>` is measured from its *flattened* geometry (exactly what the
        // composer rewrites), not from its control points.
        "path" => {
            if let Some((min_x, min_y, max_x, max_y)) = path::bbox(element.attr("d").unwrap_or(""))
            {
                let s = ctx.half_stroke();
                local.add_rect(
                    min_x as f64 - s,
                    min_y as f64 - s,
                    max_x as f64 + s,
                    max_y as f64 + s,
                );
            }
        }
        // A `<use>` places its target at `x`/`y` (positive, side-effect-free).
        // When the target resolves to a `<defs>` element it is measured and shifted
        // by `(x, y)` — once per reference. A `<symbol>` / nested `<svg>` (which
        // the composer does not carry) falls back to the placement box, an
        // under-estimate by design; a *dangling* reference (id absent from the
        // document) draws nothing and so contributes no ink at all (see the docs).
        "use" => {
            let x = number_attr(element, "x").unwrap_or(0.0);
            let y = number_attr(element, "y").unwrap_or(0.0);
            match add_resolved_use(element, ctx, ids, depth, x, y, local) {
                UseInk::Measured => {}
                // A dangling reference renders nothing: counting the placement box
                // here would raise a false "overflow" (W6).
                UseInk::Dangling => {}
                // The id exists but names content the composer does not carry:
                // keep the under-estimated placement box.
                UseInk::Unsupported => add_use_placement(element, ctx, x, y, local),
            }
        }
        // A `<text>`'s own `<tspan>`s are folded into its descendant character
        // run, so only the outermost text node is measured (a top-level `<tspan>`
        // outside any `<text>` is still handled).
        "text" | "tspan" if !ctx.in_text => add_text_box(element, ctx, local),
        _ => {}
    }
}

/// Pair up the numbers in an SVG `points` list, tolerating either `"x,y"` or
/// `"x y"` separators (and any mix).
fn parse_points(value: &str) -> Vec<(f64, f64)> {
    let numbers: Vec<f64> = value
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .filter_map(super::walk::parse_number)
        .collect();
    numbers
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}
