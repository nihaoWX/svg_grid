// Layout sizing: derived canvas height, spacers / zero rel, lengths matching.

use std::fs;
use std::path::PathBuf;

use svg_grid::{plot_grid, AlignMode, Margins, PlotGridSpec, SvgInput};

use crate::common::*;

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
        font_families: None,
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
