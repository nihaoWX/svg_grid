//! `<path>` → `<polyline>`/`<polygon>` rewriting, plus `<clipPath>` rectification.
//!
//! Straight commands are exact; curves are flattened to within
//! [`DEFAULT_TOLERANCE`] source units, after which the (already baked) transform
//! is applied to every point.
//!
//! # Draw form: fill, stroke and closure decide the elements
//!
//! One `<path>` may carry both a fill and a stroke, and the two are painted
//! separately, so the emitted shape depends on whether the path is *filled*,
//! whether it is *stroked* (`stroke::is_stroked`) and how many subpaths it has:
//!
//! | filled | stroked | subpaths | output |
//! |---|---|---|---|
//! | yes | no | any | one `<polygon>`: every subpath concatenated and explicitly closed |
//! | yes | yes | 1 | a `<polygon>` if the subpath is closed, else a `<polyline>` (unchanged from before) |
//! | yes | yes | >1 | one **fill-only** `<polygon>` (concatenated + closed) plus one **stroke-only** shape per subpath |
//! | no | any | any | one `<polyline>` per subpath |
//!
//! A **filled** path is a single fill region, so all its subpaths (a glyph's outer
//! contour *and* its holes) belong to one `<polygon>` — one polygon per subpath
//! would fill every counter (`O`, `0`, `8`, `B`) solid. Concatenating them,
//! however, introduces *bridges* between the contours, so each concatenated
//! subpath is **explicitly closed** (see below) and a stroked path is **split**:
//! a `<polygon>` that kept the source `stroke` would paint those bridges.
//!
//! ## Why each concatenated subpath is explicitly closed
//!
//! A `<polygon>` implicitly draws an edge from its last point back to its first,
//! and the concatenated list also has an edge from one subpath's end to the next
//! subpath's start. For an *open* subpath (`M` without `Z` — exactly the
//! pdftocairo glyph shape) those bridge edges are real edges that change the
//! winding number and can add or erase fill. Appending the first point to every
//! open subpath makes each outbound bridge and its return the same segment walked
//! in opposite directions, so their net winding contribution is zero: the fill
//! region then matches the source's "all subpaths share one `fill-rule`"
//! semantics, and nested rings keep their holes.
//!
//! Because a `<polygon>`/`<polyline>` cannot paint the fill along the concatenated
//! outline and the stroke along each subpath at once, the several-subpath stroked
//! case is split into two kinds of element, each with the *other* paint removed
//! thoroughly (an inline `style` beats a presentation attribute, and the other
//! paint could also be inherited — so both a `style` edit and a
//! `stroke="none"`/`fill="none"` attribute are needed).

use crate::affine::Affine;
use crate::dom::{Element, Node};
use crate::error::ConvertError;
use crate::path::{flatten_path, Subpath, DEFAULT_TOLERANCE};
use crate::stroke::is_stroked;
use crate::transform::fmt_num;

use super::style::{parse_style, style_without};

pub(super) fn path_shapes(
    element: &Element,
    transform: Affine,
) -> Result<Vec<Element>, ConvertError> {
    let d = element.attr("d").unwrap_or("");
    let subpaths = flatten_path(d, DEFAULT_TOLERANCE, &transform).map_err(|error| {
        ConvertError::Translate(format!("failed to flatten <path d={d:?}>: {error}"))
    })?;
    if subpaths.is_empty() {
        // Only degenerate subpaths (e.g. a lone `M x y`): nothing to draw, exactly
        // as before (no empty `<polygon>` is emitted).
        return Ok(Vec::new());
    }

    let style = element.attr("style").unwrap_or("");
    let properties = parse_style(style);
    let fill = property(&properties, element, "fill").unwrap_or_else(|| "black".to_string());
    let filled = !fill.trim().eq_ignore_ascii_case("none");
    let stroked = is_stroked(property(&properties, element, "stroke").as_deref());
    let stroke_width = property(&properties, element, "stroke-width");
    let remaining_style = style_without(style, "stroke-width");

    let mut shapes = Vec::new();
    if !filled {
        // Stroked-only path: one `<polyline>` per subpath, presentation copied
        // verbatim (a zero-area contour must not be merged or dropped).
        for subpath in &subpaths {
            let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
            shape.name = "polyline".to_string();
            shape.set_attr("points", join_points(subpath.points.iter()));
            shapes.push(shape);
        }
    } else if !stroked {
        // Fill-only path: the whole region in one `<polygon>`; the source
        // `fill-rule` (if any) is carried over by `shape_skeleton` so the holes
        // keep their meaning, and every subpath is explicitly closed (see the
        // module docs).
        let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
        shape.name = "polygon".to_string();
        shape.set_attr("points", join_closed_points(&subpaths));
        shapes.push(shape);
    } else if subpaths.len() == 1 {
        // Filled *and* stroked with a single subpath (the common patch/bar): keep
        // the pre-existing shape exactly — a `<polygon>` when closed, a
        // `<polyline>` otherwise, carrying both paints.
        let subpath = &subpaths[0];
        let mut shape = shape_skeleton(element, &stroke_width, &remaining_style);
        shape.name = if subpath.closed {
            "polygon"
        } else {
            "polyline"
        }
        .to_string();
        shape.set_attr("points", join_points(subpath.points.iter()));
        shapes.push(shape);
    } else {
        // Filled *and* stroked with several subpaths: the fill is one region but
        // the stroke follows each subpath, and a single primitive cannot paint
        // both (it would stroke the bridges). Split into a fill-only `<polygon>`
        // and one stroke-only shape per subpath. Each paint is removed
        // *thoroughly*: the declaration is dropped from the inline `style` and the
        // opposite paint is forced with a presentation attribute, which overrides
        // both the source's own declaration and anything inherited.
        let fill_style = remaining_style
            .as_deref()
            .and_then(|style| style_without(style, "stroke"));
        let mut fill_shape = shape_skeleton(element, &None, &fill_style);
        fill_shape.name = "polygon".to_string();
        fill_shape.remove_attr("stroke-width");
        fill_shape.set_attr("stroke", "none");
        fill_shape.set_attr("points", join_closed_points(&subpaths));
        shapes.push(fill_shape);

        let stroke_style = remaining_style
            .as_deref()
            .and_then(|style| style_without(style, "fill"));
        for subpath in &subpaths {
            let mut shape = shape_skeleton(element, &stroke_width, &stroke_style);
            shape.name = if subpath.closed {
                "polygon"
            } else {
                "polyline"
            }
            .to_string();
            shape.set_attr("fill", "none");
            shape.set_attr("points", join_points(subpath.points.iter()));
            shapes.push(shape);
        }

        debug_assert!(
            shapes
                .iter()
                .all(|shape| !(own_fill(shape) && own_stroke(shape))),
            "a split <path> must not produce a shape that both fills and strokes"
        );
    }
    Ok(shapes)
}

/// The element's own value for a presentation property: an inline `style`
/// declaration wins over the presentation attribute (CSS), as the browser
/// resolves it.
fn property(properties: &[(String, String)], element: &Element, name: &str) -> Option<String> {
    properties
        .iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
        .or_else(|| element.attr(name).map(str::to_string))
}

/// [`property`] for an already-built element, reparsing its serialized style.
/// Used by the split assertion (and its tests).
fn own_property(element: &Element, name: &str) -> Option<String> {
    let style = element.attr("style").unwrap_or("");
    property(&parse_style(style), element, name)
}

/// Whether the element paints a fill of its own: its `fill` resolves to something
/// other than `none` (a missing `fill` is the SVG default black).
fn own_fill(element: &Element) -> bool {
    !matches!(
        own_property(element, "fill").as_deref(),
        Some(value) if value.trim().eq_ignore_ascii_case("none")
    )
}

/// Whether the element paints a stroke of its own (the SVG default is `none`).
fn own_stroke(element: &Element) -> bool {
    is_stroked(own_property(element, "stroke").as_deref())
}

fn join_points<'a>(points: impl Iterator<Item = &'a (f32, f32)>) -> String {
    points
        .map(|(x, y)| format!("{},{}", fmt_num(*x), fmt_num(*y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Concatenate every subpath into one point string, **explicitly closing** each:
/// a subpath whose last point is not its first gets the first point appended, so
/// the bridges between subpaths cancel (see the module docs).
fn join_closed_points(subpaths: &[Subpath]) -> String {
    let mut points = Vec::new();
    for subpath in subpaths {
        points.extend(subpath.points.iter().copied());
        if let (Some(first), Some(last)) = (subpath.points.first(), subpath.points.last()) {
            if first != last {
                points.push(*first);
            }
        }
    }
    join_points(points.iter())
}

/// A fresh shape carrying the path's non-`d` presentation (including `fill-rule`,
/// which must be preserved verbatim — we never invent `evenodd`), the resolved
/// `stroke-width` and the leftover inline style.
fn shape_skeleton(
    element: &Element,
    stroke_width: &Option<String>,
    remaining_style: &Option<String>,
) -> Element {
    let mut shape = Element {
        name: String::new(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    for (attr, value) in &element.attrs {
        if matches!(attr.as_str(), "d" | "style" | "transform" | "id") {
            continue;
        }
        shape.set_attr(attr, value.clone());
    }
    if let Some(stroke_width) = stroke_width {
        shape.set_attr("stroke-width", stroke_width.clone());
    }
    if let Some(remaining_style) = remaining_style {
        shape.set_attr("style", remaining_style.clone());
    }
    shape
}

// --------------------------------------------------------------------------- //
// `<clipPath>` rectification
// --------------------------------------------------------------------------- //

/// Rewrite the `<path>` children of every `<clipPath>` into `<rect>` elements.
///
/// `pdftocairo -svg` writes clip boxes as `<path d="M x1 y1 L x2 y1 … Z"/>`
/// (often with a trailing degenerate `M x y` subpath), but the composer can only
/// bake an axis-aligned `<rect>` clip box. Every subpath must be an axis-aligned
/// rectangle, otherwise this fails closed.
pub(super) fn rectify_clip_paths(element: &mut Element) -> Result<(), ConvertError> {
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

#[cfg(test)]
mod tests {
    use crate::dom::{parse, Element, Node};
    use crate::error::ConvertError;
    use crate::pipeline::convert;
    use crate::test_support::converted;

    use super::{own_fill, own_stroke};

    /// Every `<polygon>`/`<polyline>` in the converted document, in order.
    fn shapes_in(svg: &str) -> Vec<Element> {
        fn walk(element: &Element, out: &mut Vec<Element>) {
            if matches!(element.name.as_str(), "polygon" | "polyline") {
                out.push(element.clone());
            }
            for child in &element.children {
                if let Node::Element(child) = child {
                    walk(child, out);
                }
            }
        }
        let root = parse(svg).expect("the converted svg must parse");
        let mut out = Vec::new();
        walk(&root, &mut out);
        out
    }

    #[test]
    fn filled_path_with_a_hole_becomes_one_polygon() {
        // Outer contour + inner contour in one `<path>`, no `Z`, no explicit
        // fill (i.e. the pdftocairo glyph shape). O/0/8/B holes must survive as a
        // single fill region, so exactly one `<polygon>` is emitted.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10 L 0 10 M 2 2 L 2 8 L 8 8 L 8 2'/></svg>"#;
        let done = converted(source);
        assert_eq!(done.svg.matches("<polygon").count(), 1, "{}", done.svg);
        assert!(!done.svg.contains("<polyline"), "{}", done.svg);
        // Every open subpath is explicitly closed before it is concatenated.
        let shapes = shapes_in(&done.svg);
        let points = shapes[0].attr("points").expect("the polygon has points");
        assert!(points.contains("0,10 0,0"), "{points}");
        assert!(points.contains("8,2 2,2"), "{points}");
    }

    #[test]
    fn filled_and_stroked_disjoint_contours_split_fill_from_stroke() {
        // Two *disjoint* closed rectangles in one `<path>` carrying both a fill and
        // a stroke: concatenating them into a single `<polygon>` would bridge the
        // gap, and that bridge would be stroked. The shape must split into one
        // fill-only `<polygon>` plus one stroke-only element per subpath.
        let source = r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<path d='M 10 10 L 30 10 L 30 30 L 10 30 Z M 60 60 L 90 60 L 90 90 L 60 90 Z' fill='#ff0000' stroke='#000000' stroke-width='2' fill-rule='evenodd'/></svg>"#;
        let done = converted(source);
        let shapes = shapes_in(&done.svg);
        // Exactly three elements: one fill-only polygon plus one stroke-only
        // element per subpath (closed subpaths → `<polygon>`).
        assert_eq!(shapes.len(), 3, "{}", done.svg);

        let fills: Vec<&Element> = shapes
            .iter()
            .filter(|s| s.attr("fill").is_some_and(|value| value != "none"))
            .collect();
        let strokes: Vec<&Element> = shapes
            .iter()
            .filter(|s| s.attr("stroke").is_some_and(|value| value != "none"))
            .collect();
        assert_eq!(fills.len(), 1, "one fill-only element: {}", done.svg);
        assert_eq!(strokes.len(), 2, "one stroke-only element: {}", done.svg);

        let fill_shape = fills[0];
        assert_eq!(fill_shape.name, "polygon", "{}", done.svg);
        assert_eq!(fill_shape.attr("stroke"), Some("none"), "{}", done.svg);
        assert_eq!(fill_shape.attr("fill"), Some("#ff0000"), "{}", done.svg);
        assert_eq!(
            fill_shape.attr("fill-rule"),
            Some("evenodd"),
            "fill-rule must stay on the fill: {}",
            done.svg
        );
        for shape in &strokes {
            assert_eq!(shape.name, "polygon", "{}", done.svg);
            assert_eq!(shape.attr("fill"), Some("none"), "{}", done.svg);
            assert_eq!(shape.attr("stroke"), Some("#000000"), "{}", done.svg);
        }

        // The split invariant: no element paints both a fill and a stroke.
        assert!(
            shapes
                .iter()
                .all(|shape| !(own_fill(shape) && own_stroke(shape))),
            "{}",
            done.svg
        );
        assert_eq!(
            done.svg.matches(r#"stroke="none""#).count(),
            1,
            "{}",
            done.svg
        );
        assert_eq!(
            done.svg.matches(r#"fill="none""#).count(),
            2,
            "{}",
            done.svg
        );
    }

    #[test]
    fn filled_and_stroked_single_subpath_keeps_its_pre_fix_shape() {
        // The common patch/bar: filled *and* stroked with a single subpath. It must
        // stay exactly as before — an open subpath a `<polyline>` carrying both
        // paints …
        let open = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10' fill='#ff0000' stroke='#000000' stroke-width='2'/></svg>"#,
        );
        assert_eq!(open.svg.matches("<polyline").count(), 1, "{}", open.svg);
        assert!(!open.svg.contains("<polygon"), "{}", open.svg);
        assert!(open.svg.contains(r##"fill="#ff0000""##), "{}", open.svg);
        assert!(open.svg.contains(r##"stroke="#000000""##), "{}", open.svg);
        assert!(open.svg.contains(r#"stroke-width="2""#), "{}", open.svg);

        // … and a closed subpath a `<polygon>` carrying both paints.
        let closed = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10 Z' fill='#ff0000' stroke='#000000' stroke-width='2'/></svg>"#,
        );
        assert_eq!(closed.svg.matches("<polygon").count(), 1, "{}", closed.svg);
        assert!(!closed.svg.contains("<polyline"), "{}", closed.svg);
        assert!(closed.svg.contains(r##"fill="#ff0000""##), "{}", closed.svg);
        assert!(
            closed.svg.contains(r##"stroke="#000000""##),
            "{}",
            closed.svg
        );
    }

    #[test]
    fn fill_only_open_subpaths_are_explicitly_closed_in_one_polygon() {
        // Fill-only (no stroke) with several *open* subpaths: one `<polygon>` whose
        // concatenated points close every subpath, so the bridges contribute no
        // winding.
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10 M 20 20 L 30 20 L 30 30' fill='#ff0000'/></svg>"#,
        );
        assert_eq!(done.svg.matches("<polygon").count(), 1, "{}", done.svg);
        assert!(!done.svg.contains("<polyline"), "{}", done.svg);
        let shapes = shapes_in(&done.svg);
        let points = shapes[0].attr("points").expect("the polygon has points");
        assert!(
            points.contains("10,10 0,0"),
            "first subpath closed: {points}"
        );
        assert!(
            points.contains("30,30 20,20"),
            "second subpath closed: {points}"
        );
        assert_eq!(points.split(' ').count(), 8, "{points}");
    }

    #[test]
    fn stroked_path_keeps_one_polyline_per_subpath() {
        // No fill → every subpath stays its own `<polyline>` (a stroked-only
        // zero-area contour must not be dropped or merged).
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 M 0 0 L 0 10' fill='none' stroke='#000000'/></svg>"#;
        let done = converted(source);
        assert_eq!(done.svg.matches("<polyline").count(), 2, "{}", done.svg);
        assert!(!done.svg.contains("<polygon"), "{}", done.svg);
    }

    #[test]
    fn source_fill_rule_is_preserved() {
        // The source `fill-rule` is copied verbatim; the converter never invents
        // `evenodd` (that would change the union semantics of overlapping
        // subpaths).
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10 L 0 10 Z' fill-rule='evenodd'/></svg>"#;
        let done = converted(source);
        assert!(done.svg.contains(r#"fill-rule="evenodd""#), "{}", done.svg);
        assert!(done.svg.contains("<polygon"), "{}", done.svg);
    }

    #[test]
    fn clip_path_rectangle_is_rectified() {
        // poppler's clip box: an axis-aligned rectangle plus a degenerate
        // `M x y` subpath (dropped by the flattener). The `<path>` must be gone
        // and a `<rect>` must remain, otherwise the self-check fails.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><clipPath id='c'><path clip-rule='nonzero' d='M 10 20 L 30 20 L 30 40 L 10 40 Z M 5 5'/></clipPath></defs>
<g clip-path='url(#c)'><circle cx='20' cy='30' r='5'/></g></svg>"#;
        let done = converted(source);
        assert!(!done.svg.contains("<path"), "{}", done.svg);
        assert!(
            done.svg
                .contains(r#"<rect x="10" y="20" width="20" height="20""#),
            "{}",
            done.svg
        );
    }

    #[test]
    fn non_rectangular_clip_path_fails_closed() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><clipPath id='c'><path d='M 10 20 L 30 30 L 30 40 Z'/></clipPath></defs>
<g clip-path='url(#c)'><circle cx='20' cy='30' r='5'/></g></svg>"#;
        match convert(source, false) {
            Err(ConvertError::Translate(message)) => {
                assert!(message.contains("clip"), "{message}");
                assert!(message.contains("axis-aligned"), "{message}");
            }
            other => panic!("expected a fail-closed translate error, got {other:?}"),
        }
    }

    #[test]
    fn every_clip_rect_follows_a_transformed_reference() {
        // A clip path with *several* rects referenced from a translated group:
        // all of them must move with the referencing transform, not just the
        // first.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><clipPath id='c'><path d='M 0 0 L 4 0 L 4 4 L 0 4 Z M 6 6 L 9 6 L 9 9 L 6 9 Z'/></clipPath></defs>
<g transform='translate(10,20)'><path d='M 0 0 L 50 0' fill='none' clip-path='url(#c)'/></g></svg>"#;
        let done = converted(source);
        assert!(
            done.svg
                .contains(r#"<rect x="10" y="20" width="4" height="4""#),
            "{}",
            done.svg
        );
        assert!(
            done.svg
                .contains(r#"<rect x="16" y="26" width="3" height="3""#),
            "{}",
            done.svg
        );
    }
}
