//! Unit tests for the ink estimate (moved verbatim from `src/ink.rs`).

use super::*;
use crate::dom;

fn bbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
    ink_bbox(&dom::parse(svg).unwrap())
}

#[test]
fn rect_box_is_exact_and_uses_its_own_coordinates() {
    let box_ = bbox(r##"<svg><rect x="10" y="20" width="30" height="40"/></svg>"##).unwrap();
    assert_eq!(box_, (10.0, 20.0, 40.0, 60.0));
}

#[test]
fn circle_and_line_boxes_are_the_union_of_their_geometry() {
    let box_ =
        bbox(r##"<svg><line x1="0" y1="0" x2="10" y2="5"/><circle cx="50" cy="50" r="4"/></svg>"##)
            .unwrap();
    assert_eq!(box_, (0.0, 0.0, 54.0, 54.0));
}

#[test]
fn a_painted_stroke_expands_the_box_by_half_its_width() {
    let box_ =
        bbox(r##"<svg><line x1="0" y1="0" x2="10" y2="0" stroke="#000" stroke-width="4"/></svg>"##)
            .unwrap();
    assert_eq!(box_, (-2.0, -2.0, 12.0, 2.0));
}

#[test]
fn an_unpainted_stroke_does_not_expand() {
    let box_ = bbox(
        r##"<svg><line x1="0" y1="0" x2="10" y2="0" stroke="none" stroke-width="40"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (0.0, 0.0, 10.0, 0.0));
}

#[test]
fn stroke_paint_and_width_are_inherited_from_a_group() {
    let box_ = bbox(
        r##"<svg><g stroke="#000" stroke-width="2"><line x1="0" y1="0" x2="10" y2="0"/></g></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (-1.0, -1.0, 11.0, 1.0));
}

#[test]
fn text_box_honours_x_y_font_size_and_anchor() {
    // font-size 10, anchor=start at x=20: width = 0.75 * 10 * 2 = 15.
    let box_ = bbox(r##"<svg><text x="20" y="30" font-size="10">AB</text></svg>"##).unwrap();
    assert_eq!(box_, (20.0, 22.5, 35.0, 32.5));

    // anchor=end puts the same run's right edge at x.
    let box_ =
        bbox(r##"<svg><text x="20" y="30" font-size="10" text-anchor="end">AB</text></svg>"##)
            .unwrap();
    assert_eq!(box_, (5.0, 22.5, 20.0, 32.5));
}

#[test]
fn text_dominant_baseline_moves_the_box() {
    let box_ = bbox(
        r##"<svg><text x="0" y="30" font-size="10" dominant-baseline="middle">A</text></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (0.0, 25.0, 7.5, 35.0));
}

#[test]
fn text_font_size_in_style_is_read() {
    let box_ = bbox(r##"<svg><text x="0" y="30" style="font-size: 20px">A</text></svg>"##).unwrap();
    assert_eq!(box_, (0.0, 15.0, 15.0, 35.0));
}

#[test]
fn a_tspan_inside_text_is_folded_into_its_parent() {
    // "ab" is one run of 2 characters at the text's anchor.
    let box_ =
        bbox(r##"<svg><text x="10" y="30" font-size="10">a<tspan x="40">b</tspan></text></svg>"##)
            .unwrap();
    assert_eq!(box_, (10.0, 22.5, 25.0, 32.5));
}

#[test]
fn defs_and_hidden_content_are_ignored() {
    assert!(bbox(r#"<svg><defs><circle cx="999" cy="999" r="50"/></defs></svg>"#).is_none());
    assert!(
        bbox(r#"<svg><rect x="-500" y="-500" width="50" height="50" display="none"/></svg>"#)
            .is_none()
    );
}

#[test]
fn a_path_box_is_its_flattened_extent_not_the_control_hull() {
    // The cubic control points reach y=10, but the flattened curve stays
    // within y≈[0, 7.5]; the box must follow the curve, not the hull.
    let box_ = bbox(r##"<svg><path d="M 0 0 C 0 10 10 10 10 0"/></svg>"##).unwrap();
    assert_eq!(box_.0, 0.0);
    assert!((box_.2 - 10.0).abs() < 1e-4, "{box_:?}");
    assert!(box_.3 < 7.6, "control hull would reach y=10: {box_:?}");
}

#[test]
fn a_referenced_defs_element_is_measured_at_the_use_offset() {
    // The definition sits at the origin; the use lands it at (50, 40).
    let box_ = bbox(
        r##"<svg><defs><polyline id="p" points="0,0 20,0 20,10"/></defs><use href="#p" x="50" y="40"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (50.0, 40.0, 70.0, 50.0));

    // `xlink:href` is accepted too, and the offset still applies.
    let box_ = bbox(
        r##"<svg><defs><rect id="r" x="0" y="0" width="4" height="6"/></defs><use xlink:href="#r" x="10" y="20"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (10.0, 20.0, 14.0, 26.0));
}

#[test]
fn one_definition_referenced_twice_is_measured_at_each_offset() {
    let box_ = bbox(
        r##"<svg><defs><rect id="r" x="0" y="0" width="1" height="1"/></defs><use href="#r" x="10" y="0"/><use href="#r" x="90" y="0"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (10.0, 0.0, 91.0, 1.0));
}

#[test]
fn a_dangling_use_contributes_no_ink() {
    // The id is absent from the document: the `<use>` renders nothing, so it
    // must not contribute its `(x, y)` anchor (that was a false W6).
    assert!(
        bbox(r##"<svg><use href="#missing" x="10" y="20" width="4" height="6"/></svg>"##).is_none()
    );
    // A `<use>` with no `href` is just as inert.
    assert!(bbox(r##"<svg><use x="10" y="20" width="4" height="6"/></svg>"##).is_none());
}

#[test]
fn a_use_of_a_symbol_is_skipped() {
    // `<symbol>` carries its own viewport rules the composer does not carry;
    // only the placement box is counted (the symbol body at 999 is ignored).
    let box_ = bbox(
        r##"<svg><defs><symbol id="s"><circle cx="999" cy="999" r="50"/></symbol></defs><use href="#s" x="1" y="2"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (1.0, 2.0, 1.0, 2.0));
}

#[test]
fn polygon_and_polyline_box_is_the_point_extent() {
    let box_ = bbox(
        r##"<svg><polygon points="0,0 10,0 10,20 0,20"/><polyline points="30 30 40 35"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (0.0, 0.0, 40.0, 35.0));
}

#[test]
fn an_empty_input_has_no_box() {
    assert!(bbox(r##"<svg><title>nothing drawn</title></svg>"##).is_none());
}

#[test]
fn a_rotated_y_axis_label_is_measured_through_its_transform() {
    // The real A1 panel: a y-axis label whose anchor sits just left of a
    // 528 x 187.2 canvas, carrying the converter's canonical
    // `rotate(-90 cx cy)` (cx = x). The rotation maps the label's cap height
    // onto the -x axis, so its ink lands ~7 px left of the canvas edge.
    let box_ = bbox(
        r##"<svg><text style="font-size: 7.2px; font-family: 'DejaVu Serif', serif; text-anchor: middle; fill: #20303a" x="-1.684688" y="85.176" transform="rotate(-90 -1.684688 85.176)">NES</text></svg>"##,
    )
    .unwrap();
    // Local box x=[-9.785, 6.415], y=[79.776, 86.976]; rotated -90° about the
    // anchor gives x' = cx + (y - cy), y' = cy - (x - cx).
    let (min_x, min_y, max_x, max_y) = box_;
    assert!((min_x - -7.084688).abs() < 0.01, "{box_:?}");
    assert!((max_x - 0.115312).abs() < 0.01, "{box_:?}");
    assert!((min_y - 77.076).abs() < 0.01, "{box_:?}");
    assert!((max_y - 93.276).abs() < 0.01, "{box_:?}");
}

#[test]
fn a_zero_rotation_matches_having_no_transform() {
    // `rotate(0 …)` is the identity, not an excuse to widen the box.
    let plain = bbox(r##"<svg><text x="20" y="30" font-size="10">AB</text></svg>"##);
    let zero = bbox(
        r##"<svg><text x="20" y="30" font-size="10" transform="rotate(0 20 30)">AB</text></svg>"##,
    );
    assert_eq!(plain, zero);
}

#[test]
fn translate_scale_and_matrix_move_the_local_box() {
    let moved = bbox(
        r##"<svg><rect x="10" y="20" width="30" height="40" transform="translate(5 7)"/></svg>"##,
    )
    .unwrap();
    assert_eq!(moved, (15.0, 27.0, 45.0, 67.0));

    let scaled =
        bbox(r##"<svg><rect x="10" y="20" width="30" height="40" transform="scale(2)"/></svg>"##)
            .unwrap();
    assert_eq!(scaled, (20.0, 40.0, 80.0, 120.0));

    let matrix = bbox(
        r##"<svg><rect x="1" y="0" width="0" height="0" transform="matrix(2 0 0 2 10 0)"/></svg>"##,
    )
    .unwrap();
    assert_eq!(matrix, (12.0, 0.0, 12.0, 0.0));
}

#[test]
fn a_transform_list_is_composed_left_to_right() {
    // SVG order: `scale(2)` acts on the local point first, then
    // `translate(10, 0)` moves it: (1, 0) -> (2, 0) -> (12, 0).
    let box_ = bbox(
        r##"<svg><line x1="1" y1="0" x2="1" y2="0" transform="translate(10 0) scale(2)"/></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (12.0, 0.0, 12.0, 0.0));
}

#[test]
fn an_unsupported_transform_leaves_the_element_unmeasured() {
    assert!(bbox(
        r##"<svg><rect x="-500" y="0" width="10" height="10" transform="skewX(45)"/></svg>"##
    )
    .is_none());
    assert!(
        bbox(
            r##"<svg><rect x="-500" y="0" width="10" height="10" transform="translate(1) skewY(45)"/></svg>"##
        )
        .is_none(),
        "a list mixing a supported function with an unsupported one is unusable"
    );
    // ...but a well-formed sibling is still measured.
    let sibling = bbox(
        r##"<svg><rect x="0" y="0" width="10" height="10" transform="skewX(45)"/><rect x="20" y="20" width="5" height="5"/></svg>"##,
    )
    .unwrap();
    assert_eq!(sibling, (20.0, 20.0, 25.0, 25.0));
}
