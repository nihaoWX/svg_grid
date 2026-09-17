use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use svg_grid::{
    auto_labels, plot_grid, render_svg_to_avif, render_svg_to_png, AlignMode, InputNormalize,
    LabelSize, LabelSpec, Margins, PlotGridSpec, SvgInput,
};

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("svg_grid_test_{}_{}", std::process::id(), name));
    path
}

#[test]
fn plot_grid_writes_an_svg_file() {
    let input = temp_path("single.svg");
    let output = temp_path("combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="xy"><line x1="10" y1="10" x2="90" y2="70" stroke="#000" stroke-width="1"/></g>
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
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"<svg"#));
    assert!(svg.contains(r#"width="200""#));
    assert!(svg.contains(r#"height="160""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_preserves_input_dom_children() {
    let input = temp_path("preserve.svg");
    let output = temp_path("preserve_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<title>Preserve me</title>
<g id="layer-1" data-scale="none">
  <text x="12" y="20">alpha &amp; beta</text>
  <circle cx="30" cy="40" r="5" fill="#336699"/>
</g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 240.0,
        height: Some(180.0),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"<title>Preserve me</title>"#));
    assert!(svg.contains(r#"<g id="svg-grid-cell0-layer-1" data-scale="none">"#));
    assert!(svg.contains(r#"<text x="26" y="32.500">alpha &amp; beta</text>"#));
    assert!(svg.contains(r##"<circle cx="44" cy="52.500" r="5" fill="#336699"/>"##));
    assert!(svg.contains(r#"width="240""#));
    assert!(svg.contains(r#"height="180""#));
    assert!(svg.contains(r#"viewBox="0 0 240 180""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_accepts_svgz_input() {
    let input = temp_path("compressed.svgz");
    let output = temp_path("compressed_combined.svg");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(
            br##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g id="compressed-layer" data-scale="xy"><rect x="10" y="10" width="50" height="30" fill="#ffcc00"/></g>
</svg>"##,
        )
        .unwrap();
    fs::write(&input, encoder.finish().unwrap()).unwrap();

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
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"<g id="svg-grid-cell0-compressed-layer" data-scale="xy">"#));
    assert!(svg.contains(r##"<rect x="20" y="20" width="100" height="60" fill="#ffcc00"/>"##));
    assert!(svg.contains(r#"width="200""#));
    assert!(svg.contains(r#"height="160""#));
    assert!(svg.contains(r#"viewBox="0 0 200 160""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_places_two_inputs_in_two_columns() {
    let input_a = temp_path("a.svg");
    let input_b = temp_path("b.svg");
    let output = temp_path("two_columns.svg");
    let base = |label: &str| {
        format!(
            r#"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><text x="50" y="40" font-size="12">{label}</text></g>
</svg>"#
        )
    };
    fs::write(&input_a, base("A")).unwrap();
    fs::write(&input_b, base("B")).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 220.0,
        height: Some(80.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(">A<"));
    assert!(svg.contains(">B<"));
    assert!(svg.contains(r#"<g data-svg-grid-cell="0""#));
    assert!(svg.contains(r#"<g data-svg-grid-cell="1""#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_output_can_be_used_as_nested_input() {
    let input_a = temp_path("nested_a.svg");
    let input_b = temp_path("nested_b.svg");
    let input_c = temp_path("nested_c.svg");
    let output_ab = temp_path("nested_ab.svg");
    let output_final = temp_path("nested_final.svg");
    let base = |label: &str| {
        format!(
            r#"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><text x="50" y="40" font-size="12">{label}</text></g>
</svg>"#
        )
    };
    fs::write(&input_a, base("A")).unwrap();
    fs::write(&input_b, base("B")).unwrap();
    fs::write(&input_c, base("C")).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output_ab.clone(),
        output_svgz: None,
        width: 220.0,
        height: Some(80.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: Some(LabelSpec {
            labels: vec!["A".to_string(), "B".to_string()],
            x: vec![0.0],
            y: vec![1.0],
            hjust: vec![0.0],
            vjust: vec![0.0],
            size: LabelSize::Fixed(10.0),
            font_family: "sans-serif".to_string(),
            font_face: "bold".to_string(),
            colour: "#000000".to_string(),
        }),
        normalize: None,
    })
    .unwrap();
    let ab_svg = fs::read_to_string(&output_ab).unwrap();
    assert!(ab_svg.contains(r#"<text data-svg-grid-label="0" data-scale="position""#));

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(output_ab.clone()),
            SvgInput::Path(input_c.clone()),
        ],
        output_svg: output_final.clone(),
        output_svgz: None,
        width: 240.0,
        height: Some(200.0),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![1.0, 1.0],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let final_svg = fs::read_to_string(&output_final).unwrap();
    assert!(final_svg.contains(">A<"));
    assert!(final_svg.contains(">B<"));
    assert!(final_svg.contains(">C<"));
    assert!(final_svg.contains(r#"data-svg-grid-cell="0""#));
    assert!(final_svg.contains(r#"data-svg-grid-cell="1""#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(input_c);
    let _ = fs::remove_file(output_ab);
    let _ = fs::remove_file(output_final);
}

#[test]
fn plot_grid_uses_relative_widths() {
    let input_a = temp_path("rel_width_a.svg");
    let input_b = temp_path("rel_width_b.svg");
    let output = temp_path("rel_width_combined.svg");
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000"/></g>
</svg>"##;
    fs::write(&input_a, svg).unwrap();
    fs::write(&input_b, svg).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 300.0,
        height: Some(100.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![1.0, 2.0],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"cx="50""#));
    assert!(combined.contains(r#"cx="200""#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_applies_outer_margin() {
    let input = temp_path("margin.svg");
    let output = temp_path("margin_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="0" cy="0" r="3" fill="#000"/></g>
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
        margin: Margins {
            top: 20.0,
            right: 30.0,
            bottom: 40.0,
            left: 10.0,
        },
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"cx="10""#));
    assert!(svg.contains(r#"cy="20""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_adds_auto_labels() {
    let input_a = temp_path("label_a.svg");
    let input_b = temp_path("label_b.svg");
    let output = temp_path("label_combined.svg");
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000"/></g>
</svg>"##;
    fs::write(&input_a, svg).unwrap();
    fs::write(&input_b, svg).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 220.0,
        height: Some(80.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: Some(LabelSpec {
            labels: auto_labels(2, true),
            x: vec![0.0],
            y: vec![1.0],
            hjust: vec![-0.5],
            vjust: vec![1.5],
            size: LabelSize::Fixed(14.0),
            font_family: "sans-serif".to_string(),
            font_face: "bold".to_string(),
            colour: "#111111".to_string(),
        }),
        normalize: None,
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    let cell_0 = combined.find(r#"<g data-svg-grid-cell="0""#).unwrap();
    let label_0 = combined.find(r#"<text data-svg-grid-label="0""#).unwrap();
    let cell_1 = combined.find(r#"<g data-svg-grid-cell="1""#).unwrap();
    assert!(cell_0 < label_0 && label_0 < cell_1);
    assert!(combined.contains(r#"<text data-svg-grid-label="0" data-scale="position""#));
    assert!(combined.contains(r#"font-weight="bold">A</text>"#));
    assert!(combined.contains(r#"font-weight="bold">B</text>"#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

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
fn plot_grid_writes_optional_svgz_output() {
    let input = temp_path("svgz_out_input.svg");
    let output = temp_path("svgz_out.svg");
    let output_svgz = temp_path("svgz_out.svgz");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="none"><text x="15" y="20" font-size="12">Z</text></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: Some(output_svgz.clone()),
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
    })
    .unwrap();

    let mut decoder = GzDecoder::new(fs::File::open(&output_svgz).unwrap());
    let mut decoded = String::new();
    decoder.read_to_string(&mut decoded).unwrap();
    assert!(decoded.contains(">Z<"));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(output_svgz);
}

#[test]
fn plot_grid_namespaces_defs_ids_per_cell() {
    let input_a = temp_path("defs_a.svg");
    let input_b = temp_path("defs_b.svg");
    let output = temp_path("defs_combined.svg");
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<defs><circle id="pt0" r="3" fill="#000"/></defs>
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><use href="#pt0" x="50" y="40"/></g>
</svg>"##;
    fs::write(&input_a, svg).unwrap();
    fs::write(&input_b, svg).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 220.0,
        height: Some(80.0),
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"id="svg-grid-cell0-pt0""#));
    assert!(combined.contains(r##"href="#svg-grid-cell0-pt0""##));
    assert!(combined.contains(r#"id="svg-grid-cell1-pt0""#));
    assert!(combined.contains(r##"href="#svg-grid-cell1-pt0""##));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

#[test]
fn plot_grid_preserves_off_panel_title_inside_cell() {
    let input = temp_path("title_margin.svg");
    let output = temp_path("title_margin_combined.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="10" y="10" width="80" height="60" fill="none" stroke="none"/>
<g data-scale="position"><text x="50" y="5" font-size="12">Title</text></g>
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
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"<text x="100" y="10" font-size="12">Title</text>"#));

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
fn render_svg_to_png_writes_a_png_preview() {
    let input = temp_path("raster_input.svg");
    let output = temp_path("raster_preview.png");
    fs::write(
        &input,
        r##"<svg width="64" height="48" viewBox="0 0 64 48" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="64" height="48" fill="#FFFFFF"/>
<rect x="8" y="8" width="48" height="32" fill="#336699"/>
</svg>"##,
    )
    .unwrap();

    render_svg_to_png(&input, &output).unwrap();

    let bytes = fs::read(&output).unwrap();
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn render_svg_to_png_preserves_text() {
    let input = temp_path("raster_text_input.svg");
    let output = temp_path("raster_text_preview.png");
    fs::write(
        &input,
        r##"<svg width="160" height="80" viewBox="0 0 160 80" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="160" height="80" fill="#FFFFFF"/>
<text x="20" y="50" font-family="sans-serif" font-size="40" fill="#000000">QC</text>
</svg>"##,
    )
    .unwrap();

    render_svg_to_png(&input, &output).unwrap();

    let file = fs::File::open(&output).unwrap();
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut buffer).unwrap();
    let bytes = &buffer[..frame.buffer_size()];
    let dark_pixels = bytes
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 64 && pixel[1] < 64 && pixel[2] < 64 && pixel[3] > 0)
        .count();
    assert!(
        dark_pixels > 20,
        "expected rendered text to produce dark pixels"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn render_svg_to_avif_writes_an_avif_preview() {
    let input = temp_path("raster_input_avif.svg");
    let output = temp_path("raster_preview.avif");
    fs::write(
        &input,
        r##"<svg width="64" height="48" viewBox="0 0 64 48" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="64" height="48" fill="#FFFFFF"/>
<circle cx="32" cy="24" r="12" fill="#CC5500"/>
</svg>"##,
    )
    .unwrap();

    render_svg_to_avif(&input, &output, 65.0).unwrap();

    let bytes = fs::read(&output).unwrap();
    assert!(bytes.len() > 32);
    assert_eq!(&bytes[4..8], b"ftyp");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn auto_label_size_and_position_default_outside_the_panel() {
    let input = temp_path("label_default_pos.svg");
    let output = temp_path("label_default_pos_out.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##,
    )
    .unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![SvgInput::Path(input.clone())],
        output_svg: output.clone(),
        output_svgz: None,
        width: 1000.0,
        height: Some(800.0),
        nrow: None,
        ncol: Some(1),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 0.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: Some(LabelSpec {
            labels: vec!["A".to_string()],
            x: vec![0.0],
            y: vec![1.0],
            hjust: vec![0.0],
            vjust: vec![-0.25],
            size: LabelSize::Auto,
            font_family: "serif".to_string(),
            font_face: "regular".to_string(),
            colour: "#000000".to_string(),
        }),
        normalize: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    // size = 0.02 / 0.74 * 1000 = 27.027 (auto = canvas width * 2.7 %).
    assert!(svg.contains(r#"font-size="27.027""#), "{svg}");
    // x = panel left edge (0); baseline lifted 0.25 * size = 6.757 above cell top.
    assert!(svg.contains(r#"x="0" y="-6.757""#), "{svg}");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

fn run_plot_grid(args: &[String]) -> std::process::Output {
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_svg_grid"));
    std::process::Command::new(bin)
        .arg("plot-grid")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_labels_flag_semantics() {
    let input = temp_path("cli_labels_input.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##,
    )
    .unwrap();
    let input = input.to_string_lossy().into_owned();

    let render = |name: &str, extra: &[&str]| -> String {
        let out = temp_path(name);
        let out_str = out.to_string_lossy().into_owned();
        let mut args = vec![
            "--input".to_string(),
            input.clone(),
            "--output".to_string(),
            out_str,
            "--width".to_string(),
            "200".to_string(),
            "--height".to_string(),
            "160".to_string(),
        ];
        args.extend(extra.iter().map(|value| value.to_string()));
        let result = run_plot_grid(&args);
        assert!(
            result.status.success(),
            "plot-grid failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let svg = fs::read_to_string(&out).unwrap();
        let _ = fs::remove_file(&out);
        svg
    };

    // (a) omitting --labels draws no letters.
    let absent = render("cli_labels_absent.svg", &[]);
    assert!(
        !absent.contains("data-svg-grid-label"),
        "no --labels must draw no letters: {absent}"
    );

    // (b) a bare --labels means AUTO (A, B, ...).
    let bare = render("cli_labels_bare.svg", &["--labels"]);
    assert!(bare.contains(r#"data-svg-grid-label="0""#), "{bare}");
    assert!(bare.contains(">A<"), "{bare}");

    // (c) a bare --labels directly followed by another flag is still AUTO.
    let bare_before_flag = render("cli_labels_bare_flag.svg", &["--labels", "--gap", "0"]);
    assert!(bare_before_flag.contains(">A<"), "{bare_before_flag}");

    // (d) --labels none turns the letters off.
    let off = render("cli_labels_none.svg", &["--labels", "none"]);
    assert!(
        !off.contains("data-svg-grid-label"),
        "--labels none must draw no letters: {off}"
    );
}

#[test]
fn cli_reserves_label_band_and_warns_on_tight_margin() {
    let input_path = temp_path("label_band_input.svg");
    fs::write(
        &input_path,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##,
    )
    .unwrap();
    let input = input_path.to_string_lossy().into_owned();

    let render = |name: &str, extra: &[&str]| -> (std::process::Output, String) {
        let out = temp_path(name);
        let out_str = out.to_string_lossy().into_owned();
        let mut args = vec![
            "--input".to_string(),
            input.clone(),
            "--output".to_string(),
            out_str,
            "--width".to_string(),
            "1000".to_string(),
            "--height".to_string(),
            "600".to_string(),
        ];
        args.extend(extra.iter().map(|value| value.to_string()));
        let result = run_plot_grid(&args);
        let svg = fs::read_to_string(&out).unwrap_or_default();
        let _ = fs::remove_file(&out);
        (result, svg)
    };

    // (a) --labels without --plot-margin: the band is reserved, so the letter is
    // NOT clipped and no clip warning is emitted. (The 100x80 panel is genuinely
    // upscaled ~9.5x into its ~952 px cell, which the separate scale warning does
    // report — that is the point of that warning, so only the clip warning is
    // checked for absence here.)
    let (reserved, svg) = render("label_band_reserved.svg", &["--labels"]);
    assert!(reserved.status.success());
    let reserved_stderr = String::from_utf8_lossy(&reserved.stderr);
    assert!(
        !reserved_stderr.contains("clipped"),
        "the reserved band must keep the letter inside the canvas: {reserved_stderr}"
    );
    assert!(
        reserved_stderr.contains("scaled by the layout"),
        "upscaling a 100x80 panel ~9.5x must be reported: {reserved_stderr}"
    );
    let label_tag = svg
        .split(r#"data-svg-grid-label="0""#)
        .nth(1)
        .expect("label must be present");
    let baseline: f32 = label_tag
        .split(r#"y=""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|value| value.parse().ok())
        .expect("label y must parse");
    assert!(
        baseline > 0.0,
        "the reserved band must keep the letter baseline below the top: y={baseline}"
    );

    // (b) an explicit, too-small --plot-margin is respected, but warned about; the
    // run itself must still succeed (exit 0).
    let (tight, _) = render("label_band_tight.svg", &["--labels", "--plot-margin", "1"]);
    assert!(
        tight.status.success(),
        "a clipping letter must not fail the run"
    );
    let stderr = String::from_utf8_lossy(&tight.stderr);
    assert!(
        stderr.contains("warning") && stderr.contains("clipped"),
        "a too-small margin must warn: {stderr}"
    );

    let _ = fs::remove_file(&input_path);
}

#[test]
fn cli_supports_spacers_and_zero_rel() {
    let a = temp_path("spacer_a.svg");
    let b = temp_path("spacer_b.svg");
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" fill="#eee"/></g>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##;
    fs::write(&a, svg).unwrap();
    fs::write(&b, svg).unwrap();

    let run = |rel: &str, name: &str| -> (bool, String) {
        let out = temp_path(name);
        let args: Vec<String> = [
            "--input",
            a.to_str().unwrap(),
            "--input",
            "none",
            "--input",
            b.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--width",
            "300",
            "--height",
            "100",
            "--ncol",
            "3",
            "--plot-margin",
            "0",
            "--rel-widths",
            rel,
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();
        let result = run_plot_grid(&args);
        let svg = fs::read_to_string(&out).unwrap_or_default();
        let _ = fs::remove_file(&out);
        (result.status.success(), svg)
    };

    // The spacer occupies the middle cell but draws nothing: cells are 0 and 2
    // only. Columns (aspect 1.25 × rel, spacer = 1 × rel) are 125 | 50 | 125.
    let (ok, svg) = run("1,0.5,1", "spacer_half.svg");
    assert!(ok, "spacer run failed: {svg}");
    assert_eq!(svg.matches("data-svg-grid-cell").count(), 2);
    assert!(svg.contains(r#"data-svg-grid-cell="2""#));
    assert!(!svg.contains(r#"data-svg-grid-cell="1""#));
    // A is at x=0 → circle x = 50*1.25 = 62.5; B is at x=175 → 175 + 50*1.25 = 237.5.
    assert!(svg.contains(r#"cx="62.500""#), "{svg}");
    assert!(svg.contains(r#"cx="237.500""#), "{svg}");

    // A zero multiplier is accepted (here it collapses the spacer column).
    let (ok_zero, _) = run("1,0,1", "spacer_zero.svg");
    assert!(ok_zero, "rel-widths may contain 0");

    let _ = fs::remove_file(&a);
    let _ = fs::remove_file(&b);
}

#[test]
fn plot_grid_derives_canvas_height_without_explicit_height() {
    // Two 100x80 panels (aspect 1.25) in one row of width 300 with a 20 gap:
    // available width 280 → natural row height 280 / (1.25 + 1.25) = 112, which
    // with no leftover margins/gaps is exactly the output canvas height.
    let input_a = temp_path("auto_h_a.svg");
    let input_b = temp_path("auto_h_b.svg");
    let output = temp_path("auto_h.svg");
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" fill="#eee"/></g>
</svg>"##;
    fs::write(&input_a, svg).unwrap();
    fs::write(&input_b, svg).unwrap();

    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input_a.clone()),
            SvgInput::Path(input_b.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 300.0,
        height: None,
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        gap: 20.0,
        margin: Margins::zero(),
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"height="112""#), "{combined}");
    assert!(combined.contains(r#"viewBox="0 0 300 112""#), "{combined}");
    assert!(combined.contains(r#"data-panel-box="main" x="0" y="0" width="300" height="112""#));

    let _ = fs::remove_file(input_a);
    let _ = fs::remove_file(input_b);
    let _ = fs::remove_file(output);
}

#[test]
fn cli_rejects_rel_widths_length_mismatch() {
    // Three panels in one row → three columns. A four-value `--rel-widths` used
    // to be silently discarded (falling back to equal columns); it must now be a
    // loud, non-zero-exit error (fail-closed).
    let inputs: Vec<PathBuf> = (0..3)
        .map(|index| {
            let path = temp_path(&format!("rel_mismatch_{index}.svg"));
            fs::write(
                &path,
                r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##,
            )
            .unwrap();
            path
        })
        .collect();
    let output = temp_path("rel_mismatch_out.svg");

    let mut args: Vec<String> = Vec::new();
    for input in &inputs {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.extend(
        [
            "--output",
            output.to_str().unwrap(),
            "--width",
            "300",
            "--height",
            "100",
            "--ncol",
            "3",
            "--rel-widths",
            "1,1,2,3",
        ]
        .iter()
        .map(|value| value.to_string()),
    );

    let result = run_plot_grid(&args);
    assert!(
        !result.status.success(),
        "a rel-widths length mismatch must fail"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("rel-widths") && stderr.contains("3 columns"),
        "the error must name the flag and the column count: {stderr}"
    );

    for input in inputs {
        let _ = fs::remove_file(input);
    }
    let _ = fs::remove_file(output);
}

// --------------------------------------------------------------------------- #
// Non-fatal warnings (stderr, exit code stays 0, output unchanged).
// --------------------------------------------------------------------------- #

/// A minimal protocol-valid panel with a `width`x`height` canvas.
const PANEL_100X80: &str = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##;

fn write_panels(names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|name| {
            let path = temp_path(&format!("warn_{name}.svg"));
            fs::write(&path, PANEL_100X80).unwrap();
            path
        })
        .collect()
}

fn run_cli(name: &str, inputs: &[PathBuf], extra: &[&str]) -> (std::process::Output, PathBuf) {
    let out = temp_path(name);
    let mut args: Vec<String> = Vec::new();
    for input in inputs {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.push("--output".to_string());
    args.push(out.to_string_lossy().into_owned());
    args.extend(extra.iter().map(|value| value.to_string()));
    let result = run_plot_grid(&args);
    (result, out)
}

#[test]
fn warns_when_an_input_is_rescaled_and_is_silent_at_unit_scale() {
    let panels = write_panels(&["scale"]);

    // (a) A 100x80 panel in a 1000x800 canvas is upscaled 10x: the layout
    // rescales the input, which (font-size is never rescaled) changes its text.
    let (scaled, scaled_out) = run_cli(
        "scale_warn_on.svg",
        &panels,
        &[
            "--width",
            "1000",
            "--height",
            "800",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(scaled.status.success(), "the warning must not fail the run");
    let stderr = String::from_utf8_lossy(&scaled.stderr);
    assert!(
        stderr.contains("scaled by the layout") && stderr.contains("sx=10.00000"),
        "an upscaled input must warn with its scale: {stderr}"
    );

    // (b) The same panel in exactly its own size: sx = sy = 1 -> no warning at all.
    let (unit, unit_out) = run_cli(
        "scale_warn_off.svg",
        &panels,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(unit.status.success());
    assert!(
        unit.stderr.is_empty(),
        "unit scale must be silent: {}",
        String::from_utf8_lossy(&unit.stderr)
    );

    for path in [scaled_out, unit_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn warns_when_a_cell_is_stretched_and_is_silent_for_the_natural_layout() {
    let panels = write_panels(&["stretch_a", "stretch_b"]);

    // --rel-widths 2,1 on two aspect-1.25 panels: cells are 200x120 and 100x120,
    // so each cell's aspect (1.667 / 0.833) misses the panel's (1.25).
    let (stretched, stretched_out) = run_cli(
        "stretch_warn_on.svg",
        &panels,
        &[
            "--width",
            "300",
            "--ncol",
            "2",
            "--rel-widths",
            "2,1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(stretched.status.success());
    let stderr = String::from_utf8_lossy(&stretched.stderr);
    assert!(
        stderr.contains("are stretched") && stderr.contains("stretched +33.3%"),
        "a cell/panel aspect mismatch must warn with the stretch: {stderr}"
    );

    // Omitting --rel-* lets the layout allocate each cell from the panel aspect,
    // so the cells are 150x120 (both aspect 1.25): no stretch.
    let (natural, natural_out) = run_cli(
        "stretch_warn_off.svg",
        &panels,
        &["--width", "300", "--ncol", "2", "--plot-margin", "0"],
    );
    assert!(natural.status.success());
    let stderr = String::from_utf8_lossy(&natural.stderr);
    assert!(
        !stderr.contains("are stretched"),
        "the natural layout must not report a stretch: {stderr}"
    );

    for path in [stretched_out, natural_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn warns_when_relettering_an_already_lettered_composite() {
    let panels = write_panels(&["reletter"]);

    // Step 1: a lettered row block (it carries data-svg-grid-label).
    let (row, row_out) = run_cli(
        "reletter_row.svg",
        &panels,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--labels",
            "A",
        ],
    );
    assert!(row.status.success());
    let row_svg = fs::read_to_string(&row_out).unwrap();
    assert!(row_svg.contains("data-svg-grid-label"), "{row_svg}");

    // Step 2: re-letter the block -> duplicate/misplaced letters.
    let (retagged, retagged_out) = run_cli(
        "reletter_on.svg",
        std::slice::from_ref(&row_out),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--labels",
            "B",
        ],
    );
    assert!(retagged.status.success());
    let stderr = String::from_utf8_lossy(&retagged.stderr);
    assert!(
        stderr.contains("already contain panel letters") && stderr.contains("data-svg-grid-label"),
        "re-lettering a composite must warn: {stderr}"
    );

    // Step 2 without --labels: the correct nesting -> no such warning.
    let (clean, clean_out) = run_cli(
        "reletter_off.svg",
        std::slice::from_ref(&row_out),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(clean.status.success());
    let stderr = String::from_utf8_lossy(&clean.stderr);
    assert!(
        !stderr.contains("already contain panel letters"),
        "not passing --labels must not warn: {stderr}"
    );

    for path in [row_out, retagged_out, clean_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn warns_on_duplicate_and_mismatched_labels() {
    let panels = write_panels(&["labels_a", "labels_b", "labels_c"]);
    let base = ["--width", "300", "--ncol", "3", "--plot-margin", "0"];

    // A,A,B -> "A" is repeated (the count still matches the 3 inputs).
    let mut duplicate_args = base.to_vec();
    duplicate_args.extend(["--labels", "A,A,B"]);
    let (duplicate, duplicate_out) = run_cli("labels_dup.svg", &panels, &duplicate_args);
    assert!(duplicate.status.success());
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(
        stderr.contains("repeats some letters") && stderr.contains("\"A\""),
        "duplicate letters must warn: {stderr}"
    );

    // A,B -> only 2 labels for 3 inputs: a silent count mismatch.
    let mut short_args = base.to_vec();
    short_args.extend(["--labels", "A,B"]);
    let (short, short_out) = run_cli("labels_short.svg", &panels, &short_args);
    assert!(short.status.success());
    let stderr = String::from_utf8_lossy(&short.stderr);
    assert!(
        stderr.contains("entries but the grid has 3 input(s)"),
        "a label-count mismatch must warn: {stderr}"
    );

    // A,B,C -> unique and complete: neither warning.
    let mut clean_args = base.to_vec();
    clean_args.extend(["--labels", "A,B,C"]);
    let (clean, clean_out) = run_cli("labels_clean.svg", &panels, &clean_args);
    assert!(clean.status.success());
    let stderr = String::from_utf8_lossy(&clean.stderr);
    assert!(
        !stderr.contains("repeats some letters") && !stderr.contains("entries but the grid"),
        "unique, complete labels must not warn: {stderr}"
    );

    for path in [duplicate_out, short_out, clean_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn warnings_do_not_change_the_output_bytes() {
    // Warnings only read the resolved layout; they must never touch the output.
    // Run a warning-producing invocation twice and require byte-identical output.
    let panels = write_panels(&["bytes"]);
    let args = [
        "--width",
        "1000",
        "--height",
        "800",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
    ];

    let (first, first_out) = run_cli("bytes_first.svg", &panels, &args);
    let (second, second_out) = run_cli("bytes_second.svg", &panels, &args);
    assert!(first.status.success() && second.status.success());
    assert!(
        String::from_utf8_lossy(&first.stderr).contains("scaled by the layout"),
        "the invocation is expected to warn: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        fs::read(&first_out).unwrap(),
        fs::read(&second_out).unwrap(),
        "the output bytes must be identical regardless of warnings"
    );

    for path in [first_out, second_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}
