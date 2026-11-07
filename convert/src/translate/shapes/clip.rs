//! `<clipPath>` rectification: rewrite its `<path>` children into `<rect>`s, so
//! a referencing transform can still bake the clip box.

use crate::affine::Affine;
use crate::dom::{Element, Node};
use crate::error::ConvertError;
use crate::path::{flatten_path, DEFAULT_TOLERANCE};
use crate::transform::fmt_num;

/// Rewrite the `<path>` children of every `<clipPath>` into `<rect>` elements.
///
/// `pdftocairo -svg` writes clip boxes as `<path d="M x1 y1 L x2 y1 … Z"/>`
/// (often with a trailing degenerate `M x y` subpath), but the composer can only
/// bake an axis-aligned `<rect>` clip box. Every subpath must be an axis-aligned
/// rectangle, otherwise this fails closed.
pub(crate) fn rectify_clip_paths(element: &mut Element) -> Result<(), ConvertError> {
    if element.name == "clipPath" {
        let mut out = Vec::with_capacity(element.children.len());
        for node in std::mem::take(&mut element.children) {
            match node {
                Node::Element(child) if child.name == "path" => {
                    for rect in clip_rects(&child)? {
                        out.push(Node::Element(rect));
                    }
                }
                other => out.push(other),
            }
        }
        element.children = out;
        return Ok(());
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            rectify_clip_paths(child)?;
        }
    }
    Ok(())
}

fn clip_rects(path: &Element) -> Result<Vec<Element>, ConvertError> {
    let d = path.attr("d").unwrap_or("");
    let subpaths = flatten_path(d, DEFAULT_TOLERANCE, &Affine::IDENTITY).map_err(|error| {
        ConvertError::Translate(format!("failed to flatten clip <path d={d:?}>: {error}"))
    })?;

    // Several `<rect>`s are only a *union*; an even-odd clip is an XOR, which a
    // list of rects cannot express. Fail closed instead of changing the region.
    let even_odd = path
        .attr("clip-rule")
        .map(|value| value.trim().eq_ignore_ascii_case("evenodd"))
        .unwrap_or(false);
    if even_odd && subpaths.len() > 1 {
        return Err(ConvertError::Translate(format!(
            "clip <path d={d:?}> declares clip-rule=\"evenodd\" with {} subpaths; a union of <rect> \
             clip boxes cannot express an even-odd clip",
            subpaths.len()
        )));
    }

    let mut rects = Vec::with_capacity(subpaths.len());
    for subpath in &subpaths {
        let rect = rect_from_subpath(&subpath.points).ok_or_else(|| {
            ConvertError::Translate(format!(
                "clip <path d={d:?}> has a subpath that is not an axis-aligned rectangle; a clip \
                 box must be axis-aligned"
            ))
        })?;
        rects.push(rect);
    }
    Ok(rects)
}

/// The `<rect>` for one subpath, or `None` when the subpath is not an
/// axis-aligned rectangle. A trailing point equal to the first is dropped, so
/// both 4-point and 5-point (`… Z`) rectangles are accepted. Degenerate
/// subpaths (`< 2` points) never reach here — `flatten_path` discards them.
fn rect_from_subpath(points: &[(f32, f32)]) -> Option<Element> {
    let mut corners = points.to_vec();
    if corners.len() >= 2 && corners.first() == corners.last() {
        corners.pop();
    }
    if corners.len() != 4 {
        return None;
    }

    let xmin = corners.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let xmax = corners
        .iter()
        .map(|p| p.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let ymin = corners.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let ymax = corners
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max);
    if !(xmax > xmin && ymax > ymin) {
        return None;
    }

    let near = |a: f32, b: f32| (a - b).abs() <= 1e-4;
    let mut hit = [false; 4];
    for &(x, y) in &corners {
        if !(near(x, xmin) || near(x, xmax)) || !(near(y, ymin) || near(y, ymax)) {
            return None;
        }
        let column = usize::from(near(x, xmax));
        let row = usize::from(near(y, ymax));
        hit[row * 2 + column] = true;
    }
    if hit.iter().any(|&seen| !seen) {
        return None;
    }

    Some(Element::new(
        "rect",
        vec![
            ("x", fmt_num(xmin)),
            ("y", fmt_num(ymin)),
            ("width", fmt_num(xmax - xmin)),
            ("height", fmt_num(ymax - ymin)),
        ],
    ))
}
