// A `<pattern>` body is tile local, so the composer scales it but never
// translates it (round-data/R38). `patternTransform` is already tile relative
// and stays verbatim.

use std::fs;

use svg_grid::{plot_grid, AlignMode, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

/// A `<pattern>` body is **tile local** in both renderers (resvg 0.47 and cairosvg
/// 2.9.1 agree pixel for pixel): the tile box is mapped into the cell, but the body's
/// numbers are placed relative to the tile origin. So the body scales with the panel
/// and keeps its own `x`/`y` — translating it as well would move it out of its own
/// tile window and the fill would vanish silently (round-data/R38).
#[test]
fn pattern_body_scales_but_keeps_its_tile_local_origin() {
    let input = temp_path("pattern_phase.svg");
    let output = temp_path("pattern_phase_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" width="4" height="6">
<rect x="1" y="2" width="2" height="3" fill="#000000"/>
<path d="M 1 2 L 3 5" style="fill: none; stroke: #000000; stroke-width: 1"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>
</g>
</svg>"##,
    )
    .unwrap();

    // sx = sy = 2 (100x80 panel into a 200x160 canvas).
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
    // The tile box moves and scales like any other geometry.
    assert!(
        svg.contains(r#"patternUnits="userSpaceOnUse" x="20" y="20" width="8" height="12""#),
        "{svg}"
    );
    // The body keeps its tile-local origin and only scales.
    assert!(
        svg.contains(r##"<rect x="2" y="4" width="4" height="6" fill="#000000"/>"##),
        "the pattern body must scale about the tile origin, not move with the tile: {svg}"
    );
    assert!(
        svg.contains(r#"<path d="M 2 4 L 6 10""#),
        "the pattern body's path must scale without moving: {svg}"
    );
    // `stroke-width` follows the geometric mean (√4 = 2), like any other length.
    assert!(
        svg.contains("stroke-width: 2"),
        "the pattern body's stroke width scales: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// `patternContentUnits="objectBoundingBox"` makes every body number a fraction of
/// the **referencing element's box**, and that box already scales with the panel.
/// Scaling the fractions by `(sx, sy)` again is a double scaling: at s=0.5 the
/// hatch shrank from ink 16 to 4 while the R38 base still rendered xor 0
/// (round-data/R39). So the whole body stays verbatim at every scale — the numbers
/// must not even be reformatted (`0.1` must not become `0.200` at s=2 or `0.05` at
/// s=0.5), while the tile box keeps mapping into the cell.
#[test]
fn object_bounding_box_pattern_body_is_verbatim_at_every_scale() {
    let input = temp_path("pattern_obb.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" x="10" y="10" width="4" height="6">
<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>
</g>
</svg>"##,
    )
    .unwrap();

    // s=1 and s=2 differ by a factor of two, so an accidental re-scale is visible
    // in the numbers (`0.1` would read `0.200`); the body must read the same.
    for (width, height, tag) in [(100.0, 80.0, "identity"), (200.0, 160.0, "double")] {
        let output = temp_path(&format!("pattern_obb_{tag}_combined.svg"));
        plot_grid(PlotGridSpec {
            inputs: vec![SvgInput::Path(input.clone())],
            output_svg: output.clone(),
            output_svgz: None,
            width,
            height: Some(height),
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
            svg.contains(r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##),
            "an objectBoundingBox body must stay verbatim at {tag} scale: {svg}"
        );
        // The tile box is user-space geometry and still maps like a `<rect>`.
        let (x, y, w, h) = if tag == "identity" {
            ("10", "10", "4", "6")
        } else {
            ("20", "20", "8", "12")
        };
        assert!(
            svg.contains(&format!(r#"x="{x}" y="{y}" width="{w}" height="{h}""#)),
            "the tile box must still follow the panel at {tag} scale: {svg}"
        );

        let _ = fs::remove_file(output);
    }
    let _ = fs::remove_file(input);
}

/// A panel that lands in a cell of exactly its own size must leave the pattern body
/// byte for byte alone: the scale is 1, so "scale only" and "leave alone" coincide.
#[test]
fn pattern_body_is_verbatim_when_the_panel_does_not_scale() {
    let input = temp_path("pattern_identity.svg");
    let output = temp_path("pattern_identity_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" width="4" height="6">
<rect x="1" y="2" width="2" height="3" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>
</g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 100.0,
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
        svg.contains(r##"<rect x="1" y="2" width="2" height="3" fill="#000000"/>"##),
        "a unit-scale panel leaves the pattern body alone: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// `patternTransform` is already relative to the tile, so it keeps working verbatim.
/// The composer never writes one; if it ever has to, it must *accumulate* on the
/// existing value rather than replace it (round-data/R38).
#[test]
fn pattern_transform_is_kept_verbatim() {
    let input = temp_path("pattern_transform.svg");
    let output = temp_path("pattern_transform_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p" patternUnits="userSpaceOnUse" patternTransform="translate(3 4) rotate(30)" x="10" y="10" width="4" height="6">
<rect x="1" y="2" width="2" height="3" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>
</g>
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
        svg.contains(r#"patternTransform="translate(3 4) rotate(30)""#),
        "patternTransform is tile relative and must stay verbatim: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
