// `patternTransform` / `gradientTransform` are written in the user space of the
// panel, so they must follow its scale (round-data/R55).
//
// The rewrite is the conjugation `T' = S·T·S⁻¹`: the tile box, the pattern body
// and the gradient's coordinates are already mapped by `S`, so a transform that
// has to keep describing the same geometry has to be pulled through `S` as well.
// Under an isotropic scale that only multiplies translation vectors, which keeps
// the keyword spelling; under an anisotropic scale a rotation or shear stops
// being expressible with the keywords and becomes `matrix(...)`. A transform
// that commutes with `S` (a `scale()`, a `skewX` under an isotropic scale) is
// not touched at all — the attribute keeps its exact input bytes.
//
// Every expected string below is the value the reference renderer proved correct
// (d1render = resvg 0.47), not a value derived from the implementation: the
// four-column table with the before/after pixel numbers is in
// `docs/PITFALLS/round-data/R55/test.py`, which re-measures them.

use std::fs;

use svg_grid::{plot_grid, AlignMode, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

/// Compose a `panel`-sized source panel into a `cell`-sized canvas (1 column, no
/// margins) and return the composed SVG.
fn compose(name: &str, panel: (u32, u32), cell: (f32, f32), body: &str) -> String {
    let input = temp_path(&format!("r55_{name}.svg"));
    let output = temp_path(&format!("r55_{name}_out.svg"));
    fs::write(
        &input,
        format!(
            r##"<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="{w}" height="{h}" fill="none" stroke="none"/>
<g data-scale="xy">
{body}
</g>
</svg>"##,
            w = panel.0,
            h = panel.1
        ),
    )
    .unwrap();
    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: cell.0,
        height: Some(cell.1),
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

/// A user-space pattern `id` whose `patternTransform` is `transform`.
fn pattern_body(id: &str, transform: &str, extra: &str) -> String {
    format!(
        r##"<defs>
<pattern id="{id}" patternUnits="userSpaceOnUse" x="10" y="10" width="10" height="10" patternTransform="{transform}"{extra}>
<rect x="0" y="0" width="4" height="3" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="60" height="50" fill="url(#{id})"/>"##
    )
}

#[test]
fn an_isotropic_scale_multiplies_a_pattern_transform_translation() {
    // 100x80 -> 200x160, s = 2. Measured: verbatim xor 6080/maxdiff 255, this
    // value xor 0/0.
    let svg = compose(
        "pat_iso_translate",
        (100, 80),
        (200.0, 160.0),
        &pattern_body("p", "translate(3 4)", ""),
    );
    assert!(
        svg.contains(r#"patternTransform="translate(6, 8)""#),
        "a translated pattern must follow the panel scale: {svg}"
    );
}

#[test]
fn an_isotropic_scale_multiplies_a_rotation_centre() {
    // `rotate(a cx cy)` rotates about a point, so the point is a translation and
    // scales; the angle is dimensionless and stays. Measured xor 6442 -> 0.
    let svg = compose(
        "pat_iso_rotate_centre",
        (100, 80),
        (200.0, 160.0),
        &pattern_body("p", "rotate(30 3 4)", ""),
    );
    assert!(
        svg.contains(r#"patternTransform="rotate(30, 6, 8)""#),
        "the rotation centre must move with the panel: {svg}"
    );
}

#[test]
fn an_isotropic_scale_multiplies_a_matrix_translation() {
    // Measured xor 6080 -> 0. The matrix keeps its spelling; only e/f move.
    let svg = compose(
        "pat_iso_matrix",
        (100, 80),
        (200.0, 160.0),
        &pattern_body("p", "matrix(1 0 0 1 3 4)", ""),
    );
    assert!(
        svg.contains(r#"patternTransform="matrix(1 0 0 1 6 8)""#),
        "a matrix's translation column is a translation too: {svg}"
    );
}

#[test]
fn a_pattern_transform_that_commutes_with_the_scale_stays_byte_identical() {
    // `scale(2 3)` is dimensionless and `skewX(20)` is a dimensionless linear
    // map; under an isotropic scale both already render correctly (measured xor
    // 0/0 before the fix), so rewriting them would be a regression. Note the
    // *values* (2/3, 20) are exactly the ones that would look wrong if scaled.
    let svg = compose(
        "pat_keep",
        (100, 80),
        (200.0, 160.0),
        &format!(
            "{}\n{}",
            pattern_body("p1", "scale(2 3)", ""),
            pattern_body("p2", "skewX(20)", "")
        ),
    );
    assert!(
        svg.contains(r#"patternTransform="scale(2 3)""#),
        "a diagonal scale is invariant under an isotropic conjugation: {svg}"
    );
    assert!(
        svg.contains(r#"patternTransform="skewX(20)""#),
        "a shear is invariant under an isotropic conjugation: {svg}"
    );
}

#[test]
fn an_anisotropic_scale_scales_a_pattern_transform_translation_per_axis() {
    // 60x40 -> 240x200, sx = 4, sy = 5. Measured xor 11520/maxdiff 255 -> 0/0.
    let svg = compose(
        "pat_aniso_translate",
        (60, 40),
        (240.0, 200.0),
        &pattern_body("p", "translate(3 4)", ""),
    );
    assert!(
        svg.contains(r#"patternTransform="translate(12, 20)""#),
        "each axis of a translation follows its own scale: {svg}"
    );
}

#[test]
fn an_anisotropic_scale_turns_a_pattern_rotation_into_a_matrix() {
    // S = diag(4,5): S·R(30)·S⁻¹ = [0.866 -0.4; 0.625 0.866]. No keyword can
    // express that, so the value must become `matrix(...)`. The renderer proves
    // the conjugation is far closer than the verbatim value (12469 -> 2788 on a
    // tile pattern; it cannot be exact because a rotated tile lattice is not
    // rectangular).
    let svg = compose(
        "pat_aniso_rotate",
        (60, 40),
        (240.0, 200.0),
        &pattern_body("p", "rotate(30)", ""),
    );
    assert!(
        svg.contains(r#"patternTransform="matrix(0.866 0.625 -0.400 0.866 0 0)""#),
        "an anisotropic conjugation of a rotation has no keyword form: {svg}"
    );
}

#[test]
fn an_object_bounding_box_pattern_transform_is_conjugated_too() {
    // `patternUnits="objectBoundingBox"` only changes the tile box; the transform
    // is still user space (measured: this value xor 0, verbatim 4425/255), so it
    // must be conjugated while the tile stays a fraction.
    let svg = compose(
        "pat_obb",
        (100, 80),
        (200.0, 160.0),
        r##"<defs>
<pattern id="p" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" x="0" y="0" width="0.2" height="0.2" patternTransform="translate(3 4)">
<rect x="0" y="0" width="0.2" height="0.2" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="60" height="50" fill="url(#p)"/>"##,
    );
    assert!(
        svg.contains(r#"patternTransform="translate(6, 8)""#),
        "the transform of an objectBoundingBox pattern is user space: {svg}"
    );
    assert!(
        svg.contains(r#"width="0.2" height="0.2""#),
        "the tile box must stay a bbox fraction: {svg}"
    );
}

#[test]
fn a_view_box_pattern_transform_is_conjugated_too() {
    // A viewBox makes the *body* verbatim, not the transform: measured verbatim
    // xor 6080/maxdiff 255, conjugated xor 0.
    let svg = compose(
        "pat_viewbox",
        (100, 80),
        (200.0, 160.0),
        &pattern_body("p", "translate(3 4)", " viewBox=\"0 0 10 10\""),
    );
    assert!(
        svg.contains(r#"patternTransform="translate(6, 8)""#),
        "a viewBox pattern's transform is still user space: {svg}"
    );
    assert!(svg.contains(r#"viewBox="0 0 10 10""#), "{svg}");
}

#[test]
fn nested_patterns_each_scale_for_the_level_that_references_them() {
    // Measured: both verbatim 7680/255; inner only 6400; outer only 3840; both
    // conjugated 0/0. Accumulating (the old comment's rule) would double-scale
    // the inner one to 16.
    let svg = compose(
        "pat_nested",
        (100, 80),
        (200.0, 160.0),
        r##"<defs>
<pattern id="pin" patternUnits="userSpaceOnUse" x="0" y="0" width="10" height="10" patternTransform="translate(2 2)">
<rect x="0" y="0" width="4" height="3" fill="#000000"/>
</pattern>
<pattern id="pout" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20" patternTransform="translate(4 4)">
<rect x="0" y="0" width="10" height="10" fill="url(#pin)"/>
</pattern>
</defs>
<rect x="10" y="10" width="60" height="50" fill="url(#pout)"/>"##,
    );
    assert!(
        svg.contains(r#"patternTransform="translate(4, 4)""#),
        "the inner pattern scales with its own reference: {svg}"
    );
    assert!(
        svg.contains(r#"patternTransform="translate(8, 8)""#),
        "the outer pattern scales once, not twice: {svg}"
    );
}

#[test]
fn an_unparsable_pattern_transform_is_kept_verbatim_with_a_warning() {
    // A malformed value is *not* fail-closed; the renderer can still use it, so
    // it stays exactly as written and is reported as W7.
    let input = temp_path("r55_pat_bad.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" width="10" height="10" patternTransform="wobble(1)">
<rect x="0" y="0" width="4" height="3" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="60" height="50" fill="url(#p)"/>
</g>
</svg>"##,
    )
    .unwrap();
    let (result, out) = run_cli(
        "r55_pat_bad_out.svg",
        std::slice::from_ref(&input),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--verbose",
        ],
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    let svg = fs::read_to_string(&out).unwrap();
    assert!(
        svg.contains(r#"patternTransform="wobble(1)""#),
        "an unparsable transform is kept verbatim: {svg}"
    );
    assert!(
        stderr.contains("patternTransform") && stderr.contains("kept verbatim"),
        "the kept transform must be reported as W7: {stderr}"
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

/// A user-space gradient (`gradientUnits="userSpaceOnUse"`, coordinate 10) over
/// a 60x40 panel, with `gradientTransform` on the element.
fn gradient_body(kind: &str, transform: &str) -> String {
    let (tag, geometry) = if kind == "radial" {
        ("radialGradient", r#"cx="10" cy="10" r="6""#)
    } else {
        ("linearGradient", r#"x1="0" y1="0" x2="10" y2="0""#)
    };
    format!(
        r##"<defs>
<{tag} id="g" gradientUnits="userSpaceOnUse" {geometry} gradientTransform="{transform}"><stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ff0000"/></{tag}>
</defs>
<rect x="0" y="0" width="60" height="40" fill="url(#g)"/>"##
    )
}

#[test]
fn an_isotropic_scale_multiplies_a_user_space_gradient_transform() {
    // Measured: verbatim xor 3680/maxdiff 39, translate(6, 8) xor 0/0. The
    // coordinates keep scaling on their own (x2 10 -> 20), so the transform must
    // only be conjugated — prepending a second `scale()` would scale twice
    // (measured xor 46830).
    let svg = compose(
        "grad_iso",
        (60, 40),
        (120.0, 80.0),
        &gradient_body("linear", "translate(3 4)"),
    );
    assert!(
        svg.contains(r#"gradientTransform="translate(6, 8)""#),
        "a user-space gradient transform must follow the panel: {svg}"
    );
    assert!(
        svg.contains(r#"x2="20""#),
        "the coordinates still scale: {svg}"
    );
}

#[test]
fn a_radial_gradient_transform_is_conjugated_by_the_same_rule() {
    let svg = compose(
        "grad_radial",
        (60, 40),
        (120.0, 80.0),
        &gradient_body("radial", "translate(3 4)"),
    );
    assert!(
        svg.contains(r#"gradientTransform="translate(6, 8)""#),
        "radial gradients share the rule: {svg}"
    );
}

#[test]
fn an_anisotropic_scale_scales_a_gradient_translation_per_axis() {
    // sx = 4, sy = 5. Measured xor 9800/maxdiff 58 -> 0/0.
    let svg = compose(
        "grad_aniso_translate",
        (60, 40),
        (240.0, 200.0),
        &gradient_body("linear", "translate(3 4)"),
    );
    assert!(
        svg.contains(r#"gradientTransform="translate(12, 20)""#),
        "{svg}"
    );
}

#[test]
fn an_anisotropic_scale_turns_a_gradient_rotation_into_a_matrix() {
    // Measured: verbatim xor 2268/maxdiff 50, conjugated matrix xor 12/maxdiff 1
    // (exact up to antialiasing; unlike a pattern tile the gradient has no
    // rectangular lattice to break).
    let svg = compose(
        "grad_aniso_rotate",
        (60, 40),
        (240.0, 200.0),
        &gradient_body("linear", "rotate(30)"),
    );
    assert!(
        svg.contains(r#"gradientTransform="matrix(0.866 0.625 -0.400 0.866 0 0)""#),
        "{svg}"
    );
}

#[test]
fn an_object_bounding_box_gradient_transform_is_left_alone() {
    // Default `gradientUnits` means the transform is in bbox units, so the panel
    // scale must not touch it. Measured: verbatim xor 0/0, a conjugated matrix
    // 36665/83 — the exact opposite of the user-space case.
    let svg = compose(
        "grad_obb",
        (60, 40),
        (240.0, 200.0),
        r##"<defs>
<linearGradient id="g" x1="0" y1="0" x2="1" y2="0" gradientTransform="rotate(45)"><stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ff0000"/></linearGradient>
</defs>
<rect x="0" y="0" width="60" height="40" fill="url(#g)"/>"##,
    );
    assert!(
        svg.contains(r#"gradientTransform="rotate(45)""#),
        "an objectBoundingBox gradient transform is a box fraction: {svg}"
    );
}

#[test]
fn a_paint_server_without_a_transform_gets_no_transform_attribute() {
    // The rewrite must never invent an attribute: no `*Transform` in, none out
    // (this is what keeps the corpus byte-identical).
    let svg = compose(
        "paint_none",
        (100, 80),
        (200.0, 160.0),
        r##"<defs>
<linearGradient id="lg" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="#000000"/></linearGradient>
<pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" width="10" height="10"><rect x="0" y="0" width="4" height="3" fill="#000000"/></pattern>
</defs>
<rect x="0" y="0" width="50" height="40" fill="url(#lg)"/>
<rect x="10" y="10" width="60" height="50" fill="url(#p)"/>"##,
    );
    assert!(!svg.contains("gradientTransform"), "{svg}");
    assert!(!svg.contains("patternTransform"), "{svg}");
}
