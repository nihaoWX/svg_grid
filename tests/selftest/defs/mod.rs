// Coordinate rewrite for `<defs>` payloads: `<mask>`, `<marker>`, `<filter>`.
//
// Each of these is placed by a *unit* attribute, and the two answers are
// opposite: `userSpaceOnUse` means "these numbers are panel coordinates, remap
// them with the geometry", while the default (`objectBoundingBox` for a mask or
// filter box, `strokeWidth` for a marker) means "these numbers are fractions or
// stroke multiples, leave them alone". Getting it backwards is silent — the
// definition is still there, it just describes the wrong box (round-data/R37).

use std::fs;

use svg_grid::{plot_grid, AlignMode, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

/// Compose one panel of `width`x`height` into a `canvas_w`x`canvas_h` grid.
fn compose(name: &str, body: &str, canvas_w: f32, canvas_h: f32) -> String {
    let input = temp_path(name);
    let output = temp_path(&format!("{name}_out"));
    fs::write(
        &input,
        format!(
            r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">{body}</g>
</svg>"##
        ),
    )
    .unwrap();
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: canvas_w,
        height: Some(canvas_h),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
        font_families: None,
    })
    .unwrap();
    let svg = fs::read_to_string(&output).unwrap();
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    svg
}

#[test]
fn a_user_space_mask_box_and_body_follow_the_geometry() {
    // sx = 2, sy = 2 (100x80 into 200x160): every number doubles.
    let svg = compose(
        "mask_user_space.svg",
        "<defs><mask id='m' maskUnits='userSpaceOnUse' x='10' y='10' width='40' height='20'>\
         <rect x='10' y='10' width='20' height='10' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"maskUnits="userSpaceOnUse" x="20" y="20" width="80" height="40""#),
        "a userSpaceOnUse mask box is panel geometry: {svg}"
    );
    assert!(
        svg.contains(r#"<rect x="20" y="20" width="40" height="20" fill="white"/>"#),
        "the mask body is drawn, so it moves with the panel: {svg}"
    );
    assert!(svg.contains("mask=\"url(#svg-grid-cell0-m)\""), "{svg}");
}

#[test]
fn an_object_bounding_box_mask_keeps_its_fractional_box() {
    // The default `maskUnits` is `objectBoundingBox`: `width='0.5'` means half the
    // masked shape, not half a pixel. Only the *content* is user space.
    let svg = compose(
        "mask_bbox.svg",
        "<defs><mask id='m' x='0' y='0' width='0.5' height='1'>\
         <rect x='10' y='10' width='20' height='10' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"x="0" y="0" width="0.5" height="1""#),
        "an objectBoundingBox mask box is a fraction and must stay: {svg}"
    );
    assert!(
        svg.contains(r#"<rect x="20" y="20" width="40" height="20" fill="white"/>"#),
        "the mask content is still user space: {svg}"
    );
}

#[test]
fn a_mask_with_fractional_content_units_keeps_its_body() {
    // `maskContentUnits='objectBoundingBox'` makes the body fractional too, so
    // nothing in the mask may be remapped.
    let svg = compose(
        "mask_content_bbox.svg",
        "<defs><mask id='m' maskUnits='userSpaceOnUse' x='10' y='10' width='40' height='20' \
         maskContentUnits='objectBoundingBox'>\
         <rect x='0' y='0' width='0.5' height='1' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"<rect x="0" y="0" width="0.5" height="1" fill="white"/>"#),
        "objectBoundingBox mask content must not be scaled: {svg}"
    );
    assert!(
        svg.contains(r#"x="20" y="20" width="80" height="40""#),
        "the mask box itself is still user space: {svg}"
    );
}

#[test]
fn a_default_marker_body_is_relative_to_its_reference_point() {
    // Under the default `markerUnits='strokeWidth'` a marker body lives in the
    // marker's own viewport `(0,0,markerWidth,markerHeight)` and is placed at the
    // vertex with `refX`/`refY` subtracted, so its numbers are relative to the
    // reference point, not to the panel. Moving them pushes the arrow out of its
    // own viewport and it disappears; the renderer already scales the body by the
    // stroke width, and `stroke-width` is rewritten, so it follows the panel for
    // free.
    let svg = compose(
        "marker_default.svg",
        "<defs><marker id='k' markerWidth='6' markerHeight='6' refX='0' refY='3' \
         orient='auto'><path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/></marker></defs>\
         <path d='M 10 30 L 90 30' stroke='black' marker-end='url(#k)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"markerWidth="6" markerHeight="6" refX="0" refY="3""#),
        "default marker geometry is relative to its reference point: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 0 0 L 6 3 L 0 6 Z" fill="black"/>"#),
        "the marker body must stay in marker coordinates: {svg}"
    );
    // The referencing path *is* panel geometry and does move.
    assert!(svg.contains(r#"d="M 20 60 L 180 60""#), "{svg}");
}

#[test]
fn a_user_space_marker_follows_the_panel() {
    // `markerUnits='userSpaceOnUse'` makes the viewport, the reference point and
    // the body plain user lengths: nothing else rewrites them, so the composer has
    // to. Measured through the renderer at s=2: xor 622, ink 1952 -> 1459 before
    // the fix (round-data/R43).
    let svg = compose(
        "marker_user_space.svg",
        "<defs><marker id='k' markerUnits='userSpaceOnUse' markerWidth='6' markerHeight='6' \
         refX='0' refY='3' orient='auto'><path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/></marker></defs>\
         <path d='M 10 30 L 90 30' stroke='black' marker-end='url(#k)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"markerWidth="12" markerHeight="12" refX="0" refY="6""#),
        "a user-space marker viewport follows the panel: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 0 0 L 12 6 L 0 12 Z" fill="black"/>"#),
        "a user-space marker body is panel geometry: {svg}"
    );
    assert!(svg.contains(r#"d="M 20 60 L 180 60""#), "{svg}");
}

#[test]
fn a_user_space_filter_box_follows_the_geometry() {
    let svg = compose(
        "filter_user_space.svg",
        "<defs><filter id='f' filterUnits='userSpaceOnUse' x='10' y='10' width='40' height='20'>\
         <feMorphology operator='dilate' radius='2'/></filter></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' filter='url(#f)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"filterUnits="userSpaceOnUse" x="20" y="20" width="80" height="40""#),
        "a userSpaceOnUse filter region is panel geometry: {svg}"
    );
    assert!(
        svg.contains("<feMorphology operator=\"dilate\" radius=\"2\"/>"),
        "filter primitives are not geometry and must not be rewritten: {svg}"
    );
}

#[test]
fn a_default_filter_region_keeps_its_fractional_box() {
    let svg = compose(
        "filter_bbox.svg",
        "<defs><filter id='f' x='0' y='0' width='0.5' height='1'>\
         <feMorphology operator='dilate' radius='2'/></filter></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' filter='url(#f)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"x="0" y="0" width="0.5" height="1""#),
        "the default filter region is a fraction of the bounding box: {svg}"
    );
}

#[test]
fn a_definition_nothing_points_at_is_still_dropped_by_the_converter() {
    // The composer keeps whatever the converter hands it; pruning happens earlier.
    // This pins the split so a future change cannot start dropping *used* ones.
    let svg = compose(
        "defs_kept.svg",
        "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='50' height='80' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>",
        100.0,
        80.0,
    );
    assert!(svg.contains("<mask"), "{svg}");
    assert!(svg.contains("mask=\"url(#svg-grid-cell0-m)\""), "{svg}");
}
