//! Unit tests (moved verbatim from the old flat `shapes.rs`).

use crate::dom::{parse, Element, Node};
use crate::error::ConvertError;
use crate::pipeline::convert;
use crate::test_support::converted;

use super::path_form::{own_fill, own_stroke};

/// Every element with one of `names` in the converted document, in order.
fn elements_in(svg: &str, names: &[&str]) -> Vec<Element> {
    fn walk(element: &Element, names: &[&str], out: &mut Vec<Element>) {
        if names.contains(&element.name.as_str()) {
            out.push(element.clone());
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                walk(child, names, out);
            }
        }
    }
    let root = parse(svg).expect("the converted svg must parse");
    let mut out = Vec::new();
    walk(&root, names, &mut out);
    out
}

/// Every `<polygon>`/`<polyline>` in the converted document, in order.
fn shapes_in(svg: &str) -> Vec<Element> {
    elements_in(svg, &["polygon", "polyline"])
}

/// Every `<path>` in the converted document, in order.
fn paths_in(svg: &str) -> Vec<Element> {
    elements_in(svg, &["path"])
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

#[test]
fn fill_only_three_subpaths_become_one_flattened_path_not_a_polygon() {
    // A plate with two holes (outer contour + two inner contours, the pdftocairo
    // glyph shape). Concatenating the three contours into a `<polygon>` would make
    // its implicit closing edge turn the bridges into a real ring and fill both
    // holes solid, so the fill region must become one `<path>` with three
    // explicitly closed `M/L/Z` contours.
    let source = r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 60 0 L 60 40 L 0 40 Z M 10 10 L 30 10 L 30 30 L 10 30 Z M 40 10 L 50 10 L 50 30 L 40 30 Z' fill='#123456' fill-rule='nonzero'/></svg>"#;
    let done = converted(source);
    assert_eq!(done.svg.matches("<path").count(), 1, "{}", done.svg);
    assert!(!done.svg.contains("<polygon"), "{}", done.svg);
    assert!(!done.svg.contains("<polyline"), "{}", done.svg);

    let paths = paths_in(&done.svg);
    assert_eq!(paths.len(), 1, "{}", done.svg);
    let path = &paths[0];
    let d = path.attr("d").expect("the path has d");
    assert_eq!(d.matches('M').count(), 3, "three contours: {d}");
    assert_eq!(d.matches('Z').count(), 3, "every contour closed: {d}");
    assert!(d.contains("M 10,10"), "second contour start: {d}");
    assert!(d.contains("M 40,10"), "third contour start: {d}");
    assert!(
        !d.contains('C') && !d.contains('A'),
        "already flattened: {d}"
    );
    assert_eq!(path.attr("fill"), Some("#123456"), "{}", done.svg);
    assert_eq!(path.attr("fill-rule"), Some("nonzero"), "{}", done.svg);
}

#[test]
fn two_donuts_with_evenodd_keep_their_holes_as_one_path() {
    // Four subpaths (two outer contours, one hole each) with `evenodd`: still one
    // fill region, still one element — but a `<path>`, and `fill-rule` must be
    // carried over verbatim (`evenodd` and `nonzero` differ for nested rings).
    let source = r#"<svg width='120' height='60' viewBox='0 0 120 60' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 40 0 L 40 40 L 0 40 Z M 10 10 L 30 10 L 30 30 L 10 30 Z M 60 0 L 100 0 L 100 40 L 60 40 Z M 70 10 L 90 10 L 90 30 L 70 30 Z' fill='black' fill-rule='evenodd'/></svg>"#;
    let done = converted(source);
    assert_eq!(done.svg.matches("<path").count(), 1, "{}", done.svg);
    assert!(!done.svg.contains("<polygon"), "{}", done.svg);
    let paths = paths_in(&done.svg);
    let d = paths[0].attr("d").expect("the path has d");
    assert_eq!(d.matches('M').count(), 4, "four contours: {d}");
    assert_eq!(d.matches('Z').count(), 4, "four closed contours: {d}");
    assert_eq!(
        paths[0].attr("fill-rule"),
        Some("evenodd"),
        "evenodd must survive the rewrite: {}",
        done.svg
    );
}

#[test]
fn fill_and_stroke_three_subpaths_split_fill_path_from_strokes() {
    // Filled *and* stroked with three subpaths: the fill becomes one flattened
    // `<path>` (the strokes must not paint the bridges), and each subpath keeps
    // its own stroke-only shape.
    let source = r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<path d='M 10 10 L 30 10 L 30 30 L 10 30 Z M 60 60 L 90 60 L 90 90 L 60 90 Z M 10 60 L 25 60 L 25 75 L 10 75 Z' fill='#ff0000' stroke='#000000' stroke-width='2' fill-rule='evenodd'/></svg>"#;
    let done = converted(source);
    let paths = paths_in(&done.svg);
    let shapes = shapes_in(&done.svg);
    assert_eq!(paths.len(), 1, "one fill path: {}", done.svg);
    assert_eq!(
        shapes.len(),
        3,
        "one stroke shape per subpath: {}",
        done.svg
    );
    let fill = &paths[0];
    assert_eq!(fill.attr("stroke"), Some("none"), "{}", done.svg);
    assert_eq!(fill.attr("fill"), Some("#ff0000"), "{}", done.svg);
    assert_eq!(fill.attr("fill-rule"), Some("evenodd"), "{}", done.svg);
    let d = fill.attr("d").expect("the fill path has d");
    assert_eq!(d.matches('M').count(), 3, "{d}");
    for shape in &shapes {
        assert_eq!(shape.attr("fill"), Some("none"), "{}", done.svg);
        assert_eq!(shape.attr("stroke"), Some("#000000"), "{}", done.svg);
        assert_eq!(shape.name, "polygon", "{}", done.svg);
    }
}

#[test]
fn two_subpaths_still_take_the_single_polygon_path() {
    // The N=2 case is exact as a `<polygon>` (both bridges are the same segment
    // walked in opposite directions, zero area), so it must keep the old output.
    let done = converted(
        r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 0 L 10 10 L 0 10 Z M 2 2 L 8 2 L 8 8 L 2 8 Z' fill='black' fill-rule='evenodd'/></svg>"#,
    );
    assert_eq!(done.svg.matches("<polygon").count(), 1, "{}", done.svg);
    assert!(!done.svg.contains("<path"), "{}", done.svg);
    let shapes = shapes_in(&done.svg);
    assert_eq!(shapes.len(), 1, "{}", done.svg);
    let points = shapes[0].attr("points").expect("the polygon has points");
    assert_eq!(points.split(' ').count(), 10, "{points}");
    assert_eq!(shapes[0].attr("fill-rule"), Some("evenodd"), "{}", done.svg);
}
