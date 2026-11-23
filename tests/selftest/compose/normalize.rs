// Input normalization (`--normalize-input-max-side`): geometry and style lengths.

use std::fs;

use svg_grid::{plot_grid, AlignMode, InputNormalize, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

#[test]
fn plot_grid_can_normalize_large_input_design_scale() {
    let input_a = temp_path("normalize_a.svg");
    let input_b = temp_path("normalize_b.svg");
    let output = temp_path("normalize_combined.svg");
    fs::write(
        &input_a,
        r##"<svg width="400" height="600" viewBox="0 0 400 600" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="400" height="600" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="200" cy="300" r="3" stroke="#000" stroke-width="1"/><text x="200" y="300" font-size="12">A</text></g>
</svg>"##,
    )
    .unwrap();
    fs::write(
        &input_b,
        r##"<svg width="4000" height="6000" viewBox="0 0 4000 6000" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="4000" height="6000" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="2000" cy="3000" r="30" stroke="#000" stroke-width="10"/><text x="2000" y="3000" font-size="120">B</text></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 800.0,
        height: Some(600.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: Some(InputNormalize { max_side: 600.0 }),
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r##"<circle cx="200" cy="300" r="3" stroke="#000" stroke-width="1"/>"##));
    assert!(svg.contains(r#"<text x="200" y="300" font-size="12">A</text>"#));
    assert!(svg.contains(r##"<circle cx="600" cy="300" r="3" stroke="#000" stroke-width="1"/>"##));
    assert!(svg.contains(r#"<text x="600" y="300" font-size="12">B</text>"#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

#[test]
fn normalize_scales_lengths_declared_in_style() {
    // Both dialects write lengths in `style="..."` (matplotlib: font-size and
    // stroke-width; svglite: font-size). Normalization must scale those by the
    // *same* factor as the geometry, or the text:geometry ratio breaks.
    let input = temp_path("normalize_style.svg");
    let output = temp_path("normalize_style_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="none">
  <text x="10" y="20" style="font-size: 10px; fill: #000000">A</text>
  <line x1="0" y1="0" x2="10" y2="10" style="stroke: #000000; stroke-width: 0.8"/>
</g>
</svg>"##,
    )
    .unwrap();

    // Longest side 100 -> 200: factor 2.0 for geometry *and* style lengths.
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
        normalize: Some(InputNormalize { max_side: 200.0 }),
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"viewBox="0 0 200 160""#), "{svg}");
    assert!(
        svg.contains(r#"style="font-size: 20px; fill: #000000""#),
        "font-size in style must scale by the same factor: {svg}"
    );
    assert!(
        svg.contains(r#"style="stroke: #000000; stroke-width: 1.600""#),
        "stroke-width in style must scale by the same factor: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn normalize_scales_text_length_with_a_px_suffix() {
    // svglite writes `textLength="102.4264px"` on every `<text>`: the value is in
    // user units but carries a `px` suffix. Normalization must scale it by the
    // *same* factor as `font-size`, or the run is silently squashed — and the
    // unit suffix must be preserved.
    let input = temp_path("normalize_text_length.svg");
    let output = temp_path("normalize_text_length_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="none"><text x="10" y="20" textLength="100px" style="font-size: 8.8px; fill: #000000">A</text></g>
</svg>"##,
    )
    .unwrap();

    // Longest side 100 -> 200: factor 2.0 for geometry, `textLength` *and* the
    // style lengths. `format_number` renders 17.6 as `17.600`.
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
        normalize: Some(InputNormalize { max_side: 200.0 }),
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(
        svg.contains(r#"textLength="200px""#),
        "textLength must scale by the same factor, keeping `px`: {svg}"
    );
    assert!(
        svg.contains(r#"style="font-size: 17.600px; fill: #000000""#),
        "font-size must scale by the same factor: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn text_length_is_untouched_without_normalization() {
    // Like `font-size`, `textLength` is never scaled by the composer's tag
    // policies: only the input normalization path touches it.
    let input = temp_path("no_normalize_text_length.svg");
    let output = temp_path("no_normalize_text_length_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="none"><text x="10" y="20" textLength="100px" style="font-size: 8.8px; fill: #000000">A</text></g>
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
    assert!(
        svg.contains(r#"textLength="100px""#),
        "without normalization textLength must be byte-identical: {svg}"
    );
    assert!(!svg.contains("textLength=\"200px\""), "{svg}");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// Compose `body` under `data-scale="xy"` with input normalization, and return the
/// output SVG.
fn normalize_panel(name: &str, body: &str, max_side: f32, canvas: f32) -> String {
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
        width: canvas,
        height: Some(canvas * 0.8),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: Some(InputNormalize { max_side }),
        font_families: None,
    })
    .unwrap();
    let svg = fs::read_to_string(&output).unwrap();
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    svg
}

#[test]
fn normalization_keeps_object_bounding_box_pattern_fractions() {
    // `patternUnits`/`patternContentUnits` default to fractions of the referencing
    // element's box, and that box shrinks with the input — so the fractions must
    // stay put. Halving them too made the hatch twice as fine as the panel it
    // decorates (round-data/R46 item D: normalized output xor 18432 against the
    // same panel without normalization).
    let svg = normalize_panel(
        "normalize_obb_pattern.svg",
        "<defs><pattern id='h' width='0.25' height='0.25' patternContentUnits='objectBoundingBox'>\
         <rect x='0' y='0' width='0.1' height='0.1' fill='black'/></pattern></defs>\
         <rect x='10' y='10' width='80' height='60' fill='url(#h)'/>",
        50.0,
        400.0,
    );
    assert!(
        svg.contains(r#"<pattern id="svg-grid-cell0-h" width="0.25" height="0.25""#),
        "the tile is a fraction of the referrer's box: {svg}"
    );
    assert!(
        svg.contains(r#"<rect x="0" y="0" width="0.1" height="0.1" fill="black"/>"#),
        "the body is a fraction of the same box: {svg}"
    );
}

#[test]
fn normalization_leaves_a_stroke_width_marker_box_alone() {
    // The default `markerUnits` is `strokeWidth`: the box is a multiple of the
    // stroke, which already halved with the input, so the multiple must not
    // (round-data/R46 item D: the arrowhead shrank from ink 4880 to 2600).
    let svg = normalize_panel(
        "normalize_marker_units.svg",
        "<defs><marker id='a' markerWidth='10' markerHeight='10' refX='5' refY='5' orient='auto'>\
         <path d='M 0,0 L 10,5 L 0,10 Z' fill='black'/></marker></defs>\
         <polyline points='20,40 80,40' stroke='black' stroke-width='2' marker-end='url(#a)'/>",
        50.0,
        400.0,
    );
    assert!(
        svg.contains(r#"markerWidth="10" markerHeight="10" refX="5" refY="5""#),
        "a stroke-width marker box is not a user length: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 0,0 L 10,5 L 0,10 Z""#),
        "its body is in the marker's own units: {svg}"
    );
}

#[test]
fn normalization_scales_a_user_space_marker_box() {
    // Control: with `markerUnits="userSpaceOnUse"` the box *is* panel geometry, so
    // it halves with the input (10 → 5) and then follows the panel (×8 → 40).
    let svg = normalize_panel(
        "normalize_marker_user_space.svg",
        "<defs><marker id='a' markerUnits='userSpaceOnUse' markerWidth='10' markerHeight='8' \
         refX='4' refY='4' orient='auto'>\
         <path d='M 0,0 L 10,5 L 0,10 Z' fill='black'/></marker></defs>\
         <polyline points='20,40 80,40' stroke='black' stroke-width='2' marker-end='url(#a)'/>",
        50.0,
        400.0,
    );
    assert!(
        svg.contains(r#"markerWidth="40" markerHeight="32" refX="16" refY="16""#),
        "a user-space marker box follows the panel: {svg}"
    );
}
