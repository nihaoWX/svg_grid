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

/// D2: a short last row must keep the same cell size as the rows above it.  The
/// plan and the real run share `prepare`, so pinning the plan pins the figure.
#[test]
fn plan_grid_keeps_a_short_last_row_at_the_full_grid_cell_size() {
    use svg_grid::{plan_grid, GridRequest};

    let inputs: Vec<PathBuf> = (0..5)
        .map(|index| {
            let path = temp_path(&format!("short_row_{index}.svg"));
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

    let plan = plan_grid(GridRequest {
        inputs: inputs.iter().cloned().map(SvgInput::Path).collect(),
        width: 1000.0,
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

    // 5 panels / 2 columns → 3 rows of 400; only the last row has one panel.
    assert!((plan.height - 1200.0).abs() < 1e-3, "{}", plan.height);
    for input in &plan.inputs {
        assert_eq!(input.cell, (500.0, 400.0), "cell of input {}", input.index);
        let (sx, sy) = input.scale.unwrap();
        assert!((sx - sy).abs() < 1e-4, "input {} is stretched", input.index);
    }
    assert_eq!(plan.inputs[4].cell, plan.inputs[0].cell);
    assert_eq!(plan.inputs[1].cell, plan.inputs[3].cell);
    assert!(
        plan.inputs.iter().all(|input| !input.is_stretched()),
        "no panel should be stretched once rows are equal"
    );

    for input in inputs {
        let _ = fs::remove_file(input);
    }
}

/// D3: with no `--nrow/--ncol`, the grid is cowplot's near-square one
/// (`cols = ceil(sqrt(n))`, `rows = ceil(n / cols)`) and the plan's numbers are
/// exactly what the real run applies.
#[test]
fn default_grid_is_near_square_and_the_plan_matches_the_run() {
    use svg_grid::{plan_grid, GridRequest};

    let inputs: Vec<PathBuf> = (0..5)
        .map(|index| {
            let path = temp_path(&format!("default_grid_{index}.svg"));
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

    let plan = plan_grid(GridRequest {
        inputs: inputs.iter().cloned().map(SvgInput::Path).collect(),
        width: 1000.0,
        height: None,
        nrow: None,
        ncol: None,
        rel_widths: vec![],
        rel_heights: vec![],
        margin: Margins::zero(),
        gap: 0.0,
        normalize: None,
    })
    .unwrap();

    // 5 → 2 rows × 3 columns (3+2); all cells keep the D2 size.
    assert!((plan.height - 1600.0 / 3.0).abs() < 1e-3, "{}", plan.height);
    for input in &plan.inputs {
        assert!((input.cell.0 - 1000.0 / 3.0).abs() < 1e-3);
        assert!((input.cell.1 - (1000.0 / 3.0) / 1.25).abs() < 1e-3);
        assert!(!input.is_stretched());
    }

    let output = temp_path("default_grid_out.svg");
    plot_grid(PlotGridSpec {
        inputs: inputs.iter().cloned().map(SvgInput::Path).collect(),
        output_svg: output.clone(),
        output_svgz: None,
        width: 1000.0,
        height: None,
        nrow: None,
        ncol: None,
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
    // The real run's canvas and cell origins agree with the plan.
    assert!(svg.contains(r#"height="533.333""#), "{svg}");
    assert!(svg.contains(r#"viewBox="0 0 1000 533.333""#), "{svg}");
    for index in 0..5 {
        assert!(
            svg.contains(&format!(r#"data-svg-grid-cell="{index}""#)),
            "missing cell {index}: {svg}"
        );
    }
    assert!(!svg.contains(r#"data-svg-grid-cell="5""#), "{svg}");
    // #4 is the first (and only) panel of the short last row, one cell to the
    // right of #3; its geometry is translated by (333.333, 266.667).
    assert!(svg.contains(r#"<circle cx="500" cy="400""#), "{svg}");

    for input in inputs {
        let _ = fs::remove_file(input);
    }
    let _ = fs::remove_file(output);
}

/// D3: `count` is the number of input *slots*, so an explicit `NULL` takes part
/// in the near-square derivation (it is a declared empty cell, not an absence).
#[test]
fn explicit_null_counts_as_a_slot_in_the_default_grid() {
    use svg_grid::{plan_grid, GridRequest};

    let inputs: Vec<PathBuf> = (0..4)
        .map(|index| {
            let path = temp_path(&format!("default_grid_null_{index}.svg"));
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

    let mut slots: Vec<SvgInput> = inputs.iter().cloned().map(SvgInput::Path).collect();
    slots.push(SvgInput::Spacer);

    let plan = plan_grid(GridRequest {
        inputs: slots,
        width: 1000.0,
        height: None,
        nrow: None,
        ncol: None,
        rel_widths: vec![],
        rel_heights: vec![],
        margin: Margins::zero(),
        gap: 0.0,
        normalize: None,
    })
    .unwrap();

    // 5 slots → 2×3, not 4 slots → 2×2.
    assert!((plan.height - 1600.0 / 3.0).abs() < 1e-3, "{}", plan.height);
    assert_eq!(plan.inputs.len(), 5);
    assert!(matches!(plan.inputs[4].input, SvgInput::Spacer));
    assert!(plan.inputs[4].scale.is_none());
    assert!((plan.inputs[4].cell.0 - 1000.0 / 3.0).abs() < 1e-3);
    assert!((plan.inputs[0].cell.1 - 266.666_66).abs() < 1e-3);

    for input in inputs {
        let _ = fs::remove_file(input);
    }
}

/// Build a `plot-grid` invocation for `panels`, optionally with `--plan`.
fn grid_args(
    panels: &[PathBuf],
    extra: &[&str],
    out: Option<&std::path::Path>,
    plan: bool,
) -> Vec<String> {
    let mut args = Vec::new();
    for panel in panels {
        args.push("--input".to_string());
        args.push(panel.to_string_lossy().into_owned());
    }
    args.extend(["--width", "1000", "--plot-margin", "0"].map(String::from));
    for value in extra {
        args.push((*value).to_string());
    }
    if plan {
        args.push("--plan".to_string());
    } else if let Some(out) = out {
        args.push("--output".to_string());
        args.push(out.to_string_lossy().into_owned());
    }
    args
}

#[test]
fn an_all_empty_last_row_is_rejected_by_run_and_plan() {
    let panels = write_panels(&[
        "emptyrow_0",
        "emptyrow_1",
        "emptyrow_2",
        "emptyrow_3",
        "emptyrow_4",
        "emptyrow_5",
    ]);

    // (inputs, explicit grid, smallest --nrow that still has a panel in its last
    // row, NULL placeholders that would fill the requested grid).
    let cases: &[(usize, &[&str], usize, usize)] = &[
        (5, &["--nrow", "4"], 3, 3),
        (5, &["--ncol", "3", "--nrow", "3"], 2, 4),
        (6, &["--ncol", "3", "--nrow", "3"], 2, 3),
        (3, &["--ncol", "3", "--nrow", "3"], 1, 6),
    ];

    for (index, (count, extra, min_rows, missing)) in cases.iter().enumerate() {
        let out = temp_path(&format!("emptyrow_out_{index}.svg"));
        let run = run_plot_grid(&grid_args(&panels[..*count], extra, Some(&out), false));
        assert_eq!(
            run.status.code(),
            Some(2),
            "case {extra:?} must be a usage error, stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(!out.exists(), "an invalid grid must write nothing");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(
            stderr.contains("grid leaves the last row empty"),
            "the error must name the empty row: {stderr}"
        );
        assert!(
            stderr.contains(&format!("reduce --nrow to {min_rows}")),
            "the error must offer the concrete rows count: {stderr}"
        );
        assert!(
            stderr.contains(&format!("add {missing} `--input NULL`")),
            "the error must offer the NULL padding: {stderr}"
        );

        // `--plan` resolves the same grid through `compute_grid`: same rc, same text.
        let plan = run_plot_grid(&grid_args(&panels[..*count], extra, None, true));
        assert_eq!(
            plan.status.code(),
            Some(2),
            "--plan must reject it too, stderr: {}",
            String::from_utf8_lossy(&plan.stderr)
        );
        assert_eq!(
            plan.stderr, run.stderr,
            "--plan and the real run must report the same grid error"
        );
    }

    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn short_last_rows_and_default_grids_are_not_mistaken_for_empty_rows() {
    let panels = write_panels(&[
        "notempty_0",
        "notempty_1",
        "notempty_2",
        "notempty_3",
        "notempty_4",
        "notempty_5",
        "notempty_6",
        "notempty_7",
        "notempty_8",
    ]);

    // A short last row is legal as long as it has *a* cell: 4 in a 2×3 grid ends
    // 3+1, 7 in 3×3 ends 3+3+1, and 5 with only --ncol 3 derives two rows.
    for (count, extra) in [
        (4usize, vec!["--ncol", "3", "--nrow", "2"]),
        (7, vec!["--ncol", "3", "--nrow", "3"]),
        (5, vec!["--ncol", "3"]),
    ] {
        let run = run_plot_grid(&grid_args(&panels[..count], &extra, None, true));
        assert!(
            run.status.success(),
            "{count} inputs {extra:?} must be accepted, stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }

    // Explicit NULLs count as slots: 5 panels + 3 NULL = 8 slots in a 4×2 grid,
    // which is completely full and must be accepted.
    let mut nulled: Vec<String> = Vec::new();
    for panel in &panels[..5] {
        nulled.push("--input".to_string());
        nulled.push(panel.to_string_lossy().into_owned());
    }
    for _ in 0..3 {
        nulled.push("--input".to_string());
        nulled.push("none".to_string());
    }
    nulled.extend(
        [
            "--width",
            "1000",
            "--plot-margin",
            "0",
            "--ncol",
            "4",
            "--nrow",
            "2",
            "--plan",
        ]
        .map(String::from),
    );
    let nulled_run = run_plot_grid(&nulled);
    assert!(
        nulled_run.status.success(),
        "5 panels + 3 explicit NULLs in 4×2 must be accepted, stderr: {}",
        String::from_utf8_lossy(&nulled_run.stderr)
    );

    // Every default grid is near-square, so it can never have an empty last row.
    for count in 1..=9 {
        let run = run_plot_grid(&grid_args(&panels[..count], &[], None, true));
        assert!(
            run.status.success(),
            "default grid for {count} input(s) must be accepted, stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }

    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}
