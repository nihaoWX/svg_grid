//! Unit tests (moved verbatim from the old flat `shapes.rs`).

use crate::dom::{parse, Element, Node};
use crate::error::ConvertError;
use crate::pipeline::convert;
use crate::test_support::converted;

use super::path_form::{own_fill, own_stroke};

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
