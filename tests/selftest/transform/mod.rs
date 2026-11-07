// Coordinate rewrite: `data-scale` policies, tspan/image/rotated anchors.

use std::fs;

use svg_grid::{plot_grid, AlignMode, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

#[test]
fn data_scale_position_moves_points_but_keeps_radius() {
    let input = temp_path("position.svg");
    let output = temp_path("position_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000"/></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(160.0),
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
    assert!(svg.contains(r#"cx="100""#));
    assert!(svg.contains(r#"cy="80""#));
    assert!(svg.contains(r#"r="3""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn data_scale_position_moves_tspan_coordinates_with_the_text() {
    // A `<tspan>`'s x/y are *absolute* positions in the current user coordinate
    // system (they override the parent `<text>`), so the composer must move them
    // by the same translation as the `<text>` anchor. Otherwise matplotlib's
    // per-run coordinates (mathtext) would silently stay behind.
    let input = temp_path("tspan_position.svg");
    let output = temp_path("tspan_position_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><text x="20" y="30">a<tspan x="25" y="30">b</tspan><tspan x="30" y="30">c</tspan></text></g>
</svg>"##,
    )
    .unwrap();

    // Margins l/r=10, t/b=5 on a 100x80 panel drawn into a 120x90 canvas: the
    // panel box maps to (10,5,100,80), i.e. exactly a translation of (+10, +5)
    // with scale 1 (so a pure "offset preserved" check is meaningful).
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 120.0,
        height: Some(90.0),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins {
            top: 5.0,
            right: 10.0,
            bottom: 5.0,
            left: 10.0,
        },
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    // The anchor and both runs move by the same translation (+10, +5).
    assert!(svg.contains(r#"<text x="30" y="35">"#), "{svg}");
    assert!(svg.contains(r#"<tspan x="35" y="35">"#), "{svg}");
    assert!(svg.contains(r#"<tspan x="40" y="35">"#), "{svg}");
    // Correctness key: the tspan offsets from the anchor are unchanged
    // (25-20=5, 30-20=10 on x; 0 on y).
    assert!(
        svg.contains(r#"<text x="30" y="35">a<tspan x="35" y="35">b</tspan>"#),
        "offsets must be preserved: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn data_scale_position_rewrites_a_tspan_with_only_x() {
    // `<tspan x="25" dy="1.2em">` (no y) is the standard multi-line form. The x
    // must still be moved even though there is no y to pair it with — requiring
    // both axes (as `rewrite_pair` does for `<text>`) would skip it silently.
    // `dy` is an incremental offset and must be left untouched.
    let input = temp_path("tspan_x_only.svg");
    let output = temp_path("tspan_x_only_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><text x="20" y="30">a<tspan x="25" dy="1.2em">b</tspan></text></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 120.0,
        height: Some(90.0),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins {
            top: 5.0,
            right: 10.0,
            bottom: 5.0,
            left: 10.0,
        },
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
        font_families: None,
    })
    .unwrap();

    // Same (+10, +5) translation: the x-only tspan's x moves, `dy` is untouched.
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"<text x="30" y="35">"#), "{svg}");
    assert!(
        svg.contains(r#"<tspan x="35" dy="1.2em">b</tspan>"#),
        "{svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn data_scale_xy_scales_circle_radius_by_geometric_mean() {
    let input = temp_path("circle_xy.svg");
    let output = temp_path("circle_xy_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><circle cx="50" cy="40" r="4" fill="#000"/></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(160.0),
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
    assert!(svg.contains(r#"cx="100""#));
    assert!(svg.contains(r#"cy="80""#));
    assert!(svg.contains(r#"r="8""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn data_scale_x_scales_use_width_and_line_stroke_width() {
    let input = temp_path("size_x.svg");
    let output = temp_path("size_x_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="x">
  <use href="#symbol" x="20" y="30" width="6" height="8"/>
  <line x1="10" y1="10" x2="90" y2="70" stroke="#000" stroke-width="2"/>
</g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(80.0),
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
    assert!(svg
        .contains(r##"<use href="#svg-grid-cell0-symbol" x="40" y="30" width="12" height="8"/>"##));
    assert!(svg.contains(r##"stroke-width="4"/>"##));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn data_scale_policy_scales_style_stroke_width() {
    // `rewrite_scaled_length` must cover a `stroke-width` declared inside
    // `style="..."` (the known "hairline" bug), with the policy unchanged:
    // `xy` scales by the geometric mean, `position` does not scale at all.
    let input = temp_path("style_stroke.svg");
    let output = temp_path("style_stroke_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><line x1="0" y1="0" x2="10" y2="10" style="stroke: #000000; stroke-width: 2"/></g>
<g data-scale="position"><line x1="0" y1="0" x2="10" y2="10" style="stroke: #000000; stroke-width: 2"/></g>
</svg>"##,
    )
    .unwrap();

    // sx = 200/100 = 2, sy = 80/80 = 1 -> xy uniform factor = sqrt(2) = 1.4142.
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(80.0),
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
    assert!(
        svg.contains(r#"style="stroke: #000000; stroke-width: 2.828""#),
        "xy must scale the style stroke-width by sqrt(2): {svg}"
    );
    assert!(
        svg.contains(r#"style="stroke: #000000; stroke-width: 2""#),
        "position must leave the style stroke-width at 2: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_rewrites_rotated_text_anchor() {
    let input = temp_path("rotated_text.svg");
    let output = temp_path("rotated_text_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><text x="5" y="40" transform="rotate(270, 5, 40)">Y</text></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(160.0),
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

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"x="10" y="80" transform="rotate(270, 10, 80)""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_rewrites_image_box_like_a_rect() {
    // An embedded `<image>` is placed by its `x`/`y`/`width`/`height` box, so the
    // composer must rewrite it exactly like a `<rect>` — each axis scaled
    // independently — instead of leaving the bitmap at its source coordinates.
    let input = temp_path("image_box.svg");
    let output = temp_path("image_box_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><image x="10" y="20" width="50" height="30" preserveAspectRatio="none" xlink:href="data:image/png;base64,AA=="/></g>
</svg>"##,
    )
    .unwrap();

    // sx = 200/100 = 2, sy = 80/80 = 1: the two axes are scaled independently.
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(80.0),
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
    assert!(
        svg.contains(
            r##"<image x="20" y="20" width="100" height="30" preserveAspectRatio="none" xlink:href="data:image/png;base64,AA=="/>"##
        ),
        "image box must be rescaled like a rect: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn paint_server_coordinates_follow_a_user_space_reference() {
    // A `*Units="userSpaceOnUse"` paint server is placed in the *referencing*
    // element's user space — the same space every other coordinate of this panel
    // lives in — so its numbers must be remapped with the geometry. Left alone, the
    // gradient keeps describing the unscaled panel: the shape is still painted, but
    // with the wrong part of the ramp (and a `<pattern>` tile with the wrong size),
    // which is exactly the kind of quiet change this tool must not make
    // (round-data/R16).
    let input = temp_path("paint_server.svg");
    let output = temp_path("paint_server_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<linearGradient id="lg" gradientUnits="userSpaceOnUse" x1="10" y1="20" x2="40" y2="60"><stop offset="0" stop-color="#fff"/></linearGradient>
<radialGradient id="rg" gradientUnits="userSpaceOnUse" cx="30" cy="40" r="5" fx="30" fy="40" fr="1"><stop offset="1" stop-color="#000"/></radialGradient>
<linearGradient id="box" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff"/></linearGradient>
<pattern id="pat" patternUnits="userSpaceOnUse" x="10" y="10" width="4" height="6">
<rect x="0" y="0" width="4" height="6" fill="#ff0000"/>
<path d="M 0 0 L 4 6" style="fill: none; stroke: #000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#lg)"/>
</g>
</svg>"##,
    )
    .unwrap();

    // sx = 2, sy = 1 (100x80 panel into a 200x80 canvas), so the two axes and the
    // uniform factor (sqrt(2)) stay distinguishable.
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 200.0,
        height: Some(80.0),
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
    // Each axis follows its own scale; the ids are namespaced with the references.
    assert!(svg.contains(r#"x1="20" y1="20" x2="80" y2="60""#), "{svg}");
    // `cx`/`cy`/`fx`/`fy` likewise, and the radii use the geometric mean (√2).
    assert!(
        svg.contains(r#"cx="60" cy="40""#) && svg.contains(r#"fx="60" fy="40""#),
        "{svg}"
    );
    assert!(svg.contains(r#"id="svg-grid-cell0-rg""#), "{svg}");
    assert!(
        svg.contains("r=\"7.071\""),
        "a radius under a non-uniform scale follows <circle r> (geometric mean): {svg}"
    );
    assert!(svg.contains("fr=\"1.414\""), "{svg}");
    // `objectBoundingBox` is the default and must NOT be touched.
    assert!(
        svg.contains(r#"x1="0" y1="0" x2="1" y2="1""#),
        "objectBoundingBox coordinates are fractions and must stay: {svg}"
    );
    // A `<pattern>` tile box moves like a `<rect>`.
    assert!(
        svg.contains(r#"patternUnits="userSpaceOnUse" x="20" y="10" width="8" height="6""#),
        "{svg}"
    );
    // ... and so does its body (user space by default). A hand-fed `<path>` stays a
    // `<path>` here (form conversion is the converter's job), but its `d` moved.
    assert!(
        svg.contains(r##"<rect x="0" y="0" width="8" height="6" fill="#ff0000"/>"##),
        "pattern body must scale with the tile: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 0 0 L 8 6""#),
        "the pattern body's path moves with the tile: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
