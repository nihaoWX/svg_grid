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
    // Corrected by R58: primitive lengths are gated by `primitiveUnits` (default
    // userSpaceOnUse), not by `filterUnits`; the old expectation froze the bug.
    // Measured through d1render at s=2: radius 2 => xor 840, radius 4 => xor 0.
    assert!(
        svg.contains("<feMorphology operator=\"dilate\" radius=\"4\"/>"),
        "primitiveUnits=userSpaceOnUse makes primitive lengths panel geometry: {svg}"
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

/// Compose `inner` (everything after the panel box) into a `canvas_w`x`canvas_h`
/// cell, without wrapping it in a `data-scale` group.
fn compose_raw(name: &str, inner: &str, canvas_w: f32, canvas_h: f32) -> String {
    let input = temp_path(name);
    let output = temp_path(&format!("{name}_out"));
    fs::write(
        &input,
        format!(
            r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
{inner}
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
fn a_definition_written_outside_the_group_follows_its_referrer() {
    // The `<pattern>` sits at the root and inherits `None`, but the only element
    // that points at it lives under `data-scale="xy"`: the renderer reads the tile
    // and the body in *that* element's user space, so both must double
    // (round-data/R46). Before the fix the tile stayed `20` while the body became
    // `49.600` and the fill vanished.
    let svg = compose_raw(
        "defs_root_pattern.svg",
        "<defs><pattern id='p' patternUnits='userSpaceOnUse' x='10' y='10' width='20' height='20'>\
         <rect x='1' y='2' width='2' height='3' fill='black'/></pattern></defs>\
         <g data-scale='xy'><rect x='10' y='10' width='40' height='30' fill='url(#p)'/></g>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"x="20" y="20" width="40" height="40""#),
        "the tile is evaluated in the referrer's user space: {svg}"
    );
    assert!(
        svg.contains(r#"<rect x="2" y="4" width="4" height="6" fill="black"/>"#),
        "the body scales about the tile origin, like every other user length: {svg}"
    );
}

#[test]
fn a_definition_two_policies_reference_is_copied_once_per_policy() {
    // `xy` and `x` disagree about the y axis, and the renderer evaluates the
    // numbers once per referrer, so one definition cannot serve both: the copy
    // keeps the original id, the second gets a deterministic policy suffix, and
    // each referrer is repointed at its own copy (round-data/R46).
    let svg = compose(
        "defs_two_policies.svg",
        "<defs><pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='20'>\
         <rect x='1' y='1' width='2' height='2' fill='black'/></pattern></defs>\
         <rect x='0' y='0' width='20' height='20' fill='url(#p)'/>",
        200.0,
        160.0,
    );
    assert!(svg.contains("id=\"svg-grid-cell0-p\""), "{svg}");
    assert!(
        svg.contains("width=\"20\" height=\"40\""),
        "the `xy` copy scales both axes: {svg}"
    );
    assert!(
        svg.contains(r#"<rect x="2" y="2" width="4" height="4" fill="black"/>"#),
        "the `xy` body doubles on both axes: {svg}"
    );
}

#[test]
fn a_copy_points_at_the_copies_of_what_it_references() {
    // Constraint that makes the fix reach all the way: the pattern fills with a
    // gradient, so the gradient is instantiated under the same policies and each
    // pattern copy must name its own copy — otherwise the `x` pattern would paint
    // with gradient coordinates scaled for `xy`.
    let svg = compose_raw(
        "defs_two_policies_gradient.svg",
        "<defs>\
         <linearGradient id='g' gradientUnits='userSpaceOnUse' x1='0' y1='0' x2='100' y2='100'/>\
         <pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='20'>\
         <rect x='0' y='0' width='10' height='20' fill='url(#g)'/></pattern></defs>\
         <g data-scale='xy'><rect x='0' y='0' width='20' height='20' fill='url(#p)'/></g>\
         <g data-scale='x'><rect x='50' y='0' width='20' height='20' fill='url(#p)'/></g>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"id="svg-grid-cell0-p--svg-grid-x" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20""#),
        "the `x` copy scales x only: {svg}"
    );
    assert!(
        svg.contains(r#"<pattern id="svg-grid-cell0-p--svg-grid-x" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20"><rect x="0" y="0" width="20" height="20" fill="url(#svg-grid-cell0-g--svg-grid-x)"/>"#),
        "the `x` copy points at its own gradient copy: {svg}"
    );
    assert!(
        svg.contains(r#"<linearGradient id="svg-grid-cell0-g--svg-grid-x" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="200" y2="100""#),
        "the copy carries the `x` scaling, not the `xy` one: {svg}"
    );
}

#[test]
fn a_marker_with_a_view_box_scales_its_box_and_not_its_body() {
    // With a `viewBox` the renderer maps the body onto the viewport itself
    // (`usvg-0.47.0/src/parser/marker.rs:183-196`), so scaling the body here would
    // apply the panel scale twice (round-data/R46 item B: ink 489 against 200).
    let svg = compose(
        "marker_view_box.svg",
        "<defs><marker id='a' markerUnits='userSpaceOnUse' markerWidth='10' markerHeight='10' \
         refX='5' refY='5' viewBox='0 0 10 10' orient='auto'>\
         <path d='M 0,0 L 10,5 L 0,10 Z' fill='black'/></marker></defs>\
         <polyline points='10,40 90,40' stroke='black' stroke-width='2' marker-end='url(#a)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"markerWidth="20" markerHeight="20" refX="10" refY="10""#),
        "the viewport is user geometry and follows the panel: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 0,0 L 10,5 L 0,10 Z""#),
        "a viewBox body is already relative to its viewport: {svg}"
    );
}

#[test]
fn a_definition_nothing_scales_keeps_every_number() {
    // No `data-scale` anywhere: the whole page only translates, so a user-space
    // marker must stay verbatim. Scaling it here used to turn a 10-unit arrowhead
    // into 24.800 units (round-data/R46).
    let svg = compose_raw(
        "defs_unscaled_marker.svg",
        "<defs><marker id='a' markerUnits='userSpaceOnUse' markerWidth='10' markerHeight='10' \
         refX='5' refY='5' orient='auto'>\
         <path d='M 0,0 L 10,5 L 0,10 Z' fill='black'/></marker></defs>\
         <polyline points='20,40 80,40' stroke='black' stroke-width='2' marker-end='url(#a)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r#"markerWidth="10" markerHeight="10" refX="5" refY="5""#),
        "nothing scales, so not even the marker box does: {svg}"
    );
    assert!(svg.contains(r#"<path d="M 0,0 L 10,5 L 0,10 Z""#), "{svg}");
    assert!(svg.contains(r#"points="20,40 80,40""#), "{svg}");
}

#[test]
fn a_gradient_reads_its_units_through_the_href_chain() {
    // `d` inherits `gradientUnits="userSpaceOnUse"` from `base`
    // (`usvg-0.47.0/src/parser/paint_server.rs:230-233` resolves through `href`),
    // so the lone `x1="10"` on `d` is a user length and must double — even though
    // `d` itself never spells the units attribute.
    let svg = compose(
        "gradient_units_href.svg",
        "<defs>\
         <linearGradient id='base' gradientUnits='userSpaceOnUse' x1='0' y1='0' x2='100' y2='10'/>\
         <linearGradient id='d' href='#base' x1='10'>\
         <stop offset='0' stop-color='black'/></linearGradient></defs>\
         <rect x='0' y='0' width='20' height='20' fill='url(#d)'/>",
        200.0,
        160.0,
    );
    assert!(
        svg.contains(r##"id="svg-grid-cell0-d" href="#svg-grid-cell0-base" x1="20""##),
        "the inherited units make `x1` a user length: {svg}"
    );
    assert!(
        svg.contains(r#"x1="0" y1="0" x2="200" y2="20""#),
        "the element that declares the units is rewritten too: {svg}"
    );
}
