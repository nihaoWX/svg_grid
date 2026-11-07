// Basic `plot_grid` composition: output files, nesting, namespace/cell ids.

use std::fs;
use std::io::Read;
use std::io::Write;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use svg_grid::{
    plan_grid, plot_grid, AlignMode, GridRequest, LabelSize, LabelSpec, Margins, PlotGridSpec,
    SvgInput,
};

use crate::common::*;

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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"cx="10""#));
    assert!(svg.contains(r#"cy="20""#));

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
        font_families: None,
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
        font_families: None,
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
        font_families: None,
    })
    .unwrap();

    let combined = fs::read_to_string(&output).unwrap();
    assert!(combined.contains(r#"<text x="100" y="10" font-size="12">Title</text>"#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

/// `--plan` is the geometry of the real run, not a second opinion: the cell size and
/// `s` it reports are the ones the composed SVG actually applies
/// (`ARCHITECTURE/layout.md` §格子尺寸由工具算). Both go through the same
/// `prepare`/`geom::cell_scale`, and this pins that down against real output bytes.
#[test]
fn plan_reports_the_geometry_the_composition_applies() {
    let input = temp_path("plan_scale.svg");
    let second = temp_path("plan_scale_b.svg");
    let output = temp_path("plan_scale_combined.svg");
    // A panel whose own canvas is 100×80 with a marker dead centre.
    let panel = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##;
    fs::write(&input, panel).unwrap();
    fs::write(&second, panel).unwrap();

    let plan = plan_grid(GridRequest {
        inputs: vec![
            SvgInput::Path(input.clone()),
            SvgInput::Path(second.clone()),
        ],
        width: 300.0,
        height: None,
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        margin: Margins::zero(),
        gap: 0.0,
        normalize: None,
    })
    .unwrap();
    assert_eq!((plan.width, plan.height), (300.0, 120.0));
    let planned = &plan.inputs[0];
    assert_eq!(planned.cell, (150.0, 120.0), "one cell of a 2-column row");
    let (sx, sy) = planned.scale.unwrap();
    assert!((sx - 1.5).abs() < 1e-4 && (sy - 1.5).abs() < 1e-4);
    assert_eq!(
        planned.distortion().unwrap(),
        0.0,
        "a natural cell is not stretched"
    );
    assert!(planned.needs_normalize(), "s = 1.5 is not its cell size");

    // The same call, for real: the centre markers must land at cell_x + 50 · sx.
    plot_grid(PlotGridSpec {
        inputs: vec![
            SvgInput::Path(input.clone()),
            SvgInput::Path(second.clone()),
        ],
        output_svg: output.clone(),
        output_svgz: None,
        width: 300.0,
        height: None,
        nrow: None,
        ncol: Some(2),
        rel_widths: vec![],
        rel_heights: vec![],
        margin: Margins::zero(),
        gap: 0.0,
        align: AlignMode::Panels,
        labels: None,
        normalize: None,
        font_families: None,
    })
    .unwrap();
    let composed = fs::read_to_string(&output).unwrap();
    assert!(
        composed.contains(r#"<circle cx="75" cy="60" r="4.500""#),
        "marker 1 sits where the plan says (50·1.5 = 75, 40·1.5 = 60)"
    );
    assert!(
        composed.contains(r#"<circle cx="225" cy="60" r="4.500""#),
        "marker 2 sits at cell 2 (150 + 50·1.5 = 225)"
    );
    assert!(
        composed.contains(r#"width="300" height="120""#),
        "canvas = plan canvas"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(second);
    let _ = fs::remove_file(output);
}
