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

/// The composer reads `patternContentUnits` and `viewBox` with the renderer's own
/// typed grammar, and follows `href` the way usvg does (round-data/R40):
///
/// * only the exact token `objectBoundingBox` means bbox fractions — a different
///   case, surrounding whitespace or an unknown value is the `userSpaceOnUse`
///   default and the body must scale;
/// * a valid `viewBox` puts the body in viewBox units, which the viewBox→tile
///   mapping already applies, so the body must stay verbatim;
/// * both attributes are inherited through `href`/`xlink:href`, and a cycle ends
///   the chain instead of hanging.
#[test]
fn pattern_body_units_are_strict_and_inherited_through_href() {
    let input = temp_path("pattern_units.svg");
    let output = temp_path("pattern_units_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="invalid" patternUnits="userSpaceOnUse" patternContentUnits="ObjectBoundingBox" x="10" y="10" width="20" height="20">
<rect x="2" y="3" width="6" height="4" fill="#000000"/>
</pattern>
<pattern id="vb" patternUnits="userSpaceOnUse" viewBox="0 0 20 20" x="10" y="10" width="20" height="20">
<rect x="2" y="3" width="6" height="4" fill="#000000"/>
</pattern>
<pattern id="base_obb" patternContentUnits="objectBoundingBox"/>
<pattern id="inherited_obb" href="#base_obb" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>
</pattern>
<pattern id="base_vb" viewBox="0 0 20 20"/>
<pattern id="inherited_vb" xlink:href="#base_vb" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="2" y="3" width="6" height="4" fill="#000000"/>
</pattern>
<pattern id="cycle_a" href="#cycle_b"/>
<pattern id="cycle_b" href="#cycle_a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="2" y="3" width="6" height="4" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#invalid)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#vb)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#inherited_obb)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#inherited_vb)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#cycle_b)"/>
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
    // `ObjectBoundingBox` is not the bbox token, so the body scales like user space.
    assert!(
        svg.contains(r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##),
        "a non-canonical patternContentUnits value must fall back to user space: {svg}"
    );
    // A valid viewBox keeps the body in viewBox units; the tile box still scales.
    let verbatim = r##"<rect x="2" y="3" width="6" height="4" fill="#000000"/>"##;
    assert!(
        svg.matches(verbatim).count() >= 2,
        "a viewBox body (direct and inherited) must stay verbatim: {svg}"
    );
    assert!(
        svg.contains(r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##),
        "an objectBoundingBox inherited through href must stay verbatim: {svg}"
    );
    assert!(
        svg.contains(r#"viewBox="0 0 20 20""#),
        "the viewBox itself stays untouched: {svg}"
    );
    assert!(
        svg.contains(r#"x="20" y="20" width="40" height="40""#),
        "every tile box is user-space geometry and still scales: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// The `href` chain also decides *how much* a `<pattern>` scales (round-data/R41):
///
/// * a link that does not resolve inside this document ends inheritance, so the
///   defaults apply and the body scales like plain user space;
/// * duplicate ids resolve to the last definition, like usvg's id map;
/// * `patternUnits` inherits too, and its `objectBoundingBox` default keeps the
///   tile fractions relative to the (already scaled) box.
#[test]
fn pattern_href_chain_follows_the_renderers_inheritance() {
    let input = temp_path("pattern_href_chain.svg");
    let output = temp_path("pattern_href_chain_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="broken" href="other.svg#p" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="3" y="4" width="5" height="2" fill="#000000"/>
</pattern>
<pattern id="dup" patternContentUnits="objectBoundingBox"/>
<pattern id="dup" patternContentUnits="userSpaceOnUse"/>
<pattern id="last_wins" href="#dup" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="4" y="5" width="6" height="3" fill="#000000"/>
</pattern>
<pattern id="tile_user" patternUnits="userSpaceOnUse"/>
<pattern id="inherited_tile" href="#tile_user" x="12" y="14" width="16" height="18">
<rect x="5" y="6" width="7" height="4" fill="#000000"/>
</pattern>
<pattern id="tile_box" patternUnits="objectBoundingBox"/>
<pattern id="inherited_box" href="#tile_box" x="0.1" y="0.2" width="0.5" height="0.5">
<rect x="5" y="6" width="7" height="4" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#broken)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#last_wins)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#inherited_tile)"/>
<rect x="10" y="10" width="40" height="30" fill="url(#inherited_box)"/>
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
    // The link leaves the document, so the content units fall back to the
    // `userSpaceOnUse` default and the body scales.
    assert!(
        svg.contains(r##"<rect x="6" y="8" width="10" height="4" fill="#000000"/>"##),
        "a broken href falls back to user space: {svg}"
    );
    // The last `<pattern id="dup">` declares user units, so the inherited body scales.
    assert!(
        svg.contains(r##"<rect x="8" y="10" width="12" height="6" fill="#000000"/>"##),
        "a duplicate id resolves to the last definition: {svg}"
    );
    // `patternUnits` is inherited: the tile follows the panel and its body is user space.
    assert!(
        svg.contains(r#"x="24" y="28" width="32" height="36""#),
        "the inherited userSpaceOnUse tile scales: {svg}"
    );
    assert!(
        svg.contains(r##"<rect x="10" y="12" width="14" height="8" fill="#000000"/>"##),
        "the inherited userSpaceOnUse body scales: {svg}"
    );
    // The bbox default keeps tile fractions and body numbers.
    assert!(
        svg.contains(r#"x="0.1" y="0.2" width="0.5" height="0.5""#),
        "an inherited objectBoundingBox tile keeps its fractions: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// A reference may carry spaces around the fragment (`href=" #base"`), and the
/// renderer accepts it. Id namespacing has to rewrite that form too, or the
/// reference keeps pointing at the un-prefixed id while `id` itself gained the
/// cell prefix — the fill then silently disappears (measured: ink 2160 -> 0,
/// round-data/R43).
#[test]
fn a_space_padded_pattern_reference_survives_id_namespacing() {
    let input = temp_path("pattern_space_padded_href.svg");
    let output = temp_path("pattern_space_padded_href_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="base" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="1" y="2" width="3" height="4" fill="black"/>
</pattern>
<pattern id="p" href=" #base"/>
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
        svg.contains(r##"href=" #svg-grid-cell0-base""##),
        "the padded reference must be namespaced like the id it points at: {svg}"
    );
    assert!(svg.contains(r##"id="svg-grid-cell0-p""##), "{svg}");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
