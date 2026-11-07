// --------------------------------------------------------------------------- #
// Non-fatal warnings (stderr, exit code stays 0, output unchanged).
// --------------------------------------------------------------------------- #

use std::fs;

use crate::common::*;

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
    // Measured: both cells are 150x120 (sx = sy = 1.50000) and their aspect
    // (1.250) equals the panel's, so the natural layout does emit W1 (the panel is
    // drawn 1.5x larger than its canvas) and must NOT emit W2. Only warning lines
    // are inspected: the shared cause note may mention that cells *can* be
    // stretched.
    let stretch_lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("svg_grid: warning:") && line.contains("stretched"))
        .collect();
    assert!(
        stretch_lines.is_empty(),
        "the natural layout must not report a stretch: {stretch_lines:?}"
    );
    assert!(
        stderr.contains("scaled by the layout"),
        "the natural layout does rescale the inputs: {stderr}"
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
        stderr.contains("entries but the grid has 3 inputs"),
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

#[test]
fn warns_when_input_ink_leaves_its_own_canvas_box() {
    // W6: a `<text>` whose ink starts 1.7 units to the LEFT of the 100x80 canvas.
    // The warning must name the input and the overflow, and must not fail the run.
    let outside = temp_path("ink_outside.svg");
    fs::write(
        &outside,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><text x="-1.7" y="40" font-size="10">A</text></g>
</svg>"##,
    )
    .unwrap();

    // Same size as the canvas -> no scale/stretch warning, so W6 is the only line.
    let args = [
        "--width",
        "100",
        "--height",
        "80",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
    ];
    let (fired, fired_out) = run_cli("ink_outside_out.svg", std::slice::from_ref(&outside), &args);
    assert!(fired.status.success(), "W6 must never fail the run");
    let stderr = String::from_utf8_lossy(&fired.stderr);
    assert!(stderr.contains("outside its own canvas box"), "{stderr}");
    assert!(stderr.contains("input #0"), "{stderr}");
    assert!(stderr.contains("left by 1.700 px"), "{stderr}");
    assert!(stderr.contains("-1.700"), "{stderr}");

    // Negative control: move the very same text inside -> W6 vanishes.
    let inside = temp_path("ink_inside.svg");
    fs::write(
        &inside,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><text x="10" y="40" font-size="10">A</text></g>
</svg>"##,
    )
    .unwrap();
    let (silent, silent_out) = run_cli("ink_inside_out.svg", std::slice::from_ref(&inside), &args);
    assert!(silent.status.success());
    assert!(
        silent.stderr.is_empty(),
        "content inside the canvas must be silent: {}",
        String::from_utf8_lossy(&silent.stderr)
    );

    for path in [outside, inside, fired_out, silent_out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn warns_when_a_rotated_y_axis_label_leaves_its_canvas() {
    // W6 for the A1 flagship case: a matplotlib-rotated y-axis label whose anchor
    // sits just left of the canvas. Ignoring `transform` used to miss this
    // entirely; the `rotate(-90 …)` now widens the label's ink box past the left
    // edge. The exact element (and its fixed twin) is taken verbatim from
    // `research/…/S39_S40/figure_panels/S39_panelC_depth_v1.svg`.
    let outside = temp_path("rotated_label_outside.svg");
    fs::write(
        &outside,
        r##"<svg width="528" height="187.2" viewBox="0 0 528 187.2" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="18.48" y="11.232" width="417.12" height="147.888" fill="none" stroke="none"/>
<text style="font-size: 7.2px; font-family: 'DejaVu Serif', serif; text-anchor: middle; fill: #20303a" x="-1.684688" y="85.176" transform="rotate(-90 -1.684688 85.176)">NES</text>
</svg>"##,
    )
    .unwrap();

    // Same size as the canvas -> no scale/stretch warning, so W6 is the only line.
    let args = [
        "--width",
        "528",
        "--height",
        "187.2",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
    ];
    let (fired, fired_out) = run_cli(
        "rotated_label_outside_out.svg",
        std::slice::from_ref(&outside),
        &args,
    );
    assert!(fired.status.success(), "W6 must never fail the run");
    let stderr = String::from_utf8_lossy(&fired.stderr);
    assert!(stderr.contains("outside its own canvas box"), "{stderr}");
    assert!(stderr.contains("input #0"), "{stderr}");
    assert!(
        stderr.contains("left by"),
        "the rotated label overflows to the LEFT: {stderr}"
    );
    let amount: f64 = stderr
        .split("left by ")
        .nth(1)
        .and_then(|rest| rest.split(" px").next())
        .and_then(|value| value.parse().ok())
        .expect("the overflow amount must parse");
    assert!(
        (1.5..12.0).contains(&amount),
        "the left overflow is ≈ the label's cap height (a few px): {amount} in {stderr}"
    );

    // Negative control: the fixed panel moves x (and the rotate centre with it) to
    // 11.747625 -> the very same label sits inside the canvas, so stderr is empty.
    let inside = temp_path("rotated_label_inside.svg");
    fs::write(
        &inside,
        r##"<svg width="528" height="187.2" viewBox="0 0 528 187.2" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="18.48" y="11.232" width="417.12" height="147.888" fill="none" stroke="none"/>
<text style="font-size: 7.2px; font-family: 'DejaVu Serif', serif; text-anchor: middle; fill: #20303a" x="11.747625" y="85.176" transform="rotate(-90 11.747625 85.176)">NES</text>
</svg>"##,
    )
    .unwrap();
    let (silent, silent_out) = run_cli(
        "rotated_label_inside_out.svg",
        std::slice::from_ref(&inside),
        &args,
    );
    assert!(silent.status.success());
    assert_eq!(
        silent.stderr.len(),
        0,
        "the fixed panel must emit nothing on stderr, got: {}",
        String::from_utf8_lossy(&silent.stderr)
    );

    for path in [outside, inside, fired_out, silent_out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn ink_warning_is_silent_for_a_clean_text_input() {
    // A text well inside its canvas: W6 must put ZERO bytes on stderr (proof that
    // the estimate does not false-positive). The args match the canvas size, so no
    // scale warning is produced either.
    let input = temp_path("ink_clean.svg");
    fs::write(
        &input,
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><text x="10" y="40" font-size="10">A</text></g>
</svg>"##,
    )
    .unwrap();

    let (result, out) = run_cli(
        "ink_clean_out.svg",
        std::slice::from_ref(&input),
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
    assert!(result.status.success());
    assert_eq!(
        result.stderr.len(),
        0,
        "a clean input must emit nothing on stderr, got: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    for path in [input, out] {
        let _ = fs::remove_file(path);
    }
}
