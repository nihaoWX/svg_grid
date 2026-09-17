//! Affine baking: accumulate `<g transform>` down the tree and push it into the
//! leaf coordinate attributes, so the output contains no `<g transform>`.
//!
//! A `<clipPath>` referenced from a transformed context is transformed by the
//! *same* matrix (collected here, applied by [`apply_clip_refs`]) so the clip box
//! stays in the coordinate system of the geometry it clips.

use std::collections::HashMap;

use crate::affine::{parse_transform, Affine};
use crate::dom::{Element, Node};
use crate::error::ConvertError;
use crate::transform::fmt_num;

use super::image::bake_image;
use super::shapes::path_shapes;
use super::{find_by_id, number, url_id};

pub(super) fn bake_children(
    children: &mut Vec<Node>,
    acc: Affine,
    clip_refs: &mut Vec<(String, Affine)>,
) -> Result<(), ConvertError> {
    let mut out = Vec::with_capacity(children.len());
    for node in std::mem::take(children) {
        match node {
            Node::Element(mut element) => {
                bake_element(&mut element, acc, clip_refs, &mut out)?;
            }
            other => out.push(other),
        }
    }
    *children = out;
    Ok(())
}

fn bake_element(
    element: &mut Element,
    acc: Affine,
    clip_refs: &mut Vec<(String, Affine)>,
    out: &mut Vec<Node>,
) -> Result<(), ConvertError> {
    let local = match element.attr("transform") {
        Some(value) => parse_transform(value).ok_or_else(|| {
            ConvertError::Translate(format!(
                "unsupported transform {value:?} on <{}>",
                element.name
            ))
        })?,
        None => Affine::IDENTITY,
    };
    let world = acc.mul(local);

    if let Some(clip) = element.attr("clip-path") {
        if let Some(id) = url_id(clip) {
            clip_refs.push((id.to_string(), world));
        }
    }

    match element.name.as_str() {
        // `<defs>` holds the `<clipPath>` rectangles, which must stay in the
        // (root) coordinate system of the geometry they clip.
        "defs" => out.push(Node::Element(element.clone())),
        "g" => {
            element.remove_attr("transform");
            bake_children(&mut element.children, world, clip_refs)?;
            out.push(Node::Element(element.clone()));
        }
        "path" => {
            for shape in path_shapes(element, world)? {
                out.push(Node::Element(shape));
            }
        }
        "image" => {
            element.remove_attr("transform");
            bake_image(element, world)?;
            out.push(Node::Element(element.clone()));
        }
        "text" => {
            // Only the *ancestor* transform must be trivial: the text's own leaf
            // transform is normalized later by `normalize_transforms`.
            if !acc.is_identity() {
                return Err(ConvertError::Translate(format!(
                    "<text> under a non-identity ancestor transform ({acc:?}) cannot be baked; only \
                     a leaf `rotate(a x y)` / `translate(x y) rotate(a)` text transform is supported"
                )));
            }
            out.push(Node::Element(element.clone()));
        }
        "circle" | "rect" | "line" | "polyline" | "polygon" => {
            bake_shape(element, world)?;
            element.remove_attr("transform");
            out.push(Node::Element(element.clone()));
        }
        _ => out.push(Node::Element(element.clone())),
    }
    Ok(())
}

fn bake_shape(element: &mut Element, transform: Affine) -> Result<(), ConvertError> {
    if transform.is_identity() {
        return Ok(());
    }
    match element.name.as_str() {
        "rect" => {
            let (sx, sy, _, _) = transform.as_axis_scale_translate().ok_or_else(|| {
                ConvertError::Translate(
                    "<rect> under a rotation/skew transform cannot be baked".to_string(),
                )
            })?;
            let _ = (sx, sy);
            let x = number(element, "x")?;
            let y = number(element, "y")?;
            let width = number(element, "width")?;
            let height = number(element, "height")?;
            let (x0, y0) = transform.apply(x, y);
            let (x1, y1) = transform.apply(x + width, y + height);
            element.set_attr("x", fmt_num(x0.min(x1)));
            element.set_attr("y", fmt_num(y0.min(y1)));
            element.set_attr("width", fmt_num((x1 - x0).abs()));
            element.set_attr("height", fmt_num((y1 - y0).abs()));
        }
        "circle" => {
            let (scale, _, _) = transform.as_uniform_scale_translate().ok_or_else(|| {
                ConvertError::Translate(
                    "<circle> under a non-uniform/rotating transform cannot be baked".to_string(),
                )
            })?;
            let cx = number(element, "cx")?;
            let cy = number(element, "cy")?;
            let r = number(element, "r")?;
            let (cx, cy) = transform.apply(cx, cy);
            element.set_attr("cx", fmt_num(cx));
            element.set_attr("cy", fmt_num(cy));
            element.set_attr("r", fmt_num(r * scale.abs()));
        }
        "line" => {
            map_pair(element, "x1", "y1", transform)?;
            map_pair(element, "x2", "y2", transform)?;
        }
        "polyline" | "polygon" => {
            let points = element.attr("points").unwrap_or("").to_string();
            element.set_attr("points", map_points(&points, transform)?);
        }
        _ => {}
    }
    Ok(())
}

fn map_pair(
    element: &mut Element,
    x_attr: &str,
    y_attr: &str,
    transform: Affine,
) -> Result<(), ConvertError> {
    let x = number(element, x_attr)?;
    let y = number(element, y_attr)?;
    let (x, y) = transform.apply(x, y);
    element.set_attr(x_attr, fmt_num(x));
    element.set_attr(y_attr, fmt_num(y));
    Ok(())
}

fn map_points(points: &str, transform: Affine) -> Result<String, ConvertError> {
    let mut out = Vec::new();
    for pair in points.split_whitespace() {
        let (x, y) = pair
            .split_once(',')
            .ok_or_else(|| ConvertError::Translate(format!("malformed point pair {pair:?}")))?;
        let x = x
            .trim()
            .parse::<f32>()
            .map_err(|_| ConvertError::Translate(format!("non-numeric point coordinate {x:?}")))?;
        let y = y
            .trim()
            .parse::<f32>()
            .map_err(|_| ConvertError::Translate(format!("non-numeric point coordinate {y:?}")))?;
        let (x, y) = transform.apply(x, y);
        out.push(format!("{},{}", fmt_num(x), fmt_num(y)));
    }
    Ok(out.join(" "))
}

pub(super) fn apply_clip_refs(
    root: &mut Element,
    refs: &[(String, Affine)],
) -> Result<(), ConvertError> {
    let mut unique: HashMap<&str, Affine> = HashMap::new();
    for (id, transform) in refs {
        match unique.get(id.as_str()) {
            Some(existing) if existing != transform => {
                return Err(ConvertError::Translate(format!(
                    "clip-path #{id} is referenced from two different coordinate systems; the \
                     clip box cannot stay consistent"
                )));
            }
            Some(_) => {}
            None => {
                unique.insert(id.as_str(), *transform);
            }
        }
    }

    for (id, transform) in unique {
        if transform.is_identity() {
            continue;
        }
        let clip_path = find_by_id(root, id)
            .ok_or_else(|| ConvertError::Translate(format!("clip-path #{id} is not defined")))?;
        if clip_path.name != "clipPath" {
            return Err(ConvertError::Translate(format!(
                "clip-path #{id} refers to <{}>, not a <clipPath>",
                clip_path.name
            )));
        }
        let Some(Node::Element(rect)) = clip_path
            .children
            .iter_mut()
            .find(|node| matches!(node, Node::Element(child) if child.name == "rect"))
        else {
            return Err(ConvertError::Translate(format!(
                "clip-path #{id} referenced from a transformed context must contain a <rect> so \
                 the clip box can be baked"
            )));
        };
        bake_shape(rect, transform)?;
    }
    Ok(())
}
