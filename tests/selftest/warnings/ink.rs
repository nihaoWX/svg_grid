//! W6/W5 ink-bound checks: a panel's own ink leaving its canvas, a rotated
//! y-axis label leaving the canvas, and the clean-text negative controls.

use std::path::PathBuf;

use super::*;

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

/// A 100x80 panel whose `body` sits at x=200, i.e. entirely right of the canvas.
fn panel_with(body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
{body}
</svg>"##
    )
}

/// Run one panel through `plot-grid` at its own size, so W6 is the only warning.
fn w6(name: &str, body: &str) -> (std::process::Output, PathBuf, PathBuf) {
    let input = temp_path(&format!("{name}.svg"));
    fs::write(&input, panel_with(body)).unwrap();
    let (result, out) = run_cli(
        &format!("{name}_out.svg"),
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
    (result, input, out)
}

#[test]
fn warns_when_only_a_tspan_paints_outside_the_canvas() {
    // Measured d1 ink 92 / cairosvg 95 at x=200: the outer `<text>` is
    // `fill="none"`, so the old judge saw no ink at all and stayed silent (R66b
    // P1-1). The warning must fire and must name the real overflow.
    let (fired, fired_in, fired_out) = w6(
        "ink_tspan_outside",
        r##"<g data-scale="position"><text x="200" y="40" font-size="20" fill="none"><tspan fill="#000">A</tspan></text></g>"##,
    );
    assert!(fired.status.success(), "W6 must never fail the run");
    let stderr = String::from_utf8_lossy(&fired.stderr);
    assert!(stderr.contains("outside its own canvas box"), "{stderr}");
    assert!(stderr.contains("input #0"), "{stderr}");
    assert!(stderr.contains("overflows right by"), "{stderr}");

    // Negative control: the same run with the tspan's paint removed is silent —
    // the judge follows the layer that carries the glyphs, not the outer tag.
    let (silent, silent_in, silent_out) = w6(
        "ink_tspan_inside",
        r##"<g data-scale="position"><text x="200" y="40" font-size="20" fill="#000"><tspan fill="none">A</tspan></text></g>"##,
    );
    assert!(silent.status.success());
    assert_eq!(
        silent.stderr.len(),
        0,
        "a non-painting tspan must stay silent, got: {}",
        String::from_utf8_lossy(&silent.stderr)
    );

    for path in [fired_in, fired_out, silent_in, silent_out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn warns_when_a_pattern_reaches_its_ink_through_href() {
    // Measured d1 ink 400 / cairosvg 400: `<pattern id="p" href="#b"/>` paints
    // `b`'s solid rect, but the old judge looked only at `p`'s own (empty) children
    // and stayed silent (R66b P1-2).
    let (fired, fired_in, fired_out) = w6(
        "ink_pattern_href_outside",
        r##"<defs><pattern id="b" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="4" height="4" fill="#000"/></pattern><pattern id="p" href="#b"/></defs><g data-scale="position"><rect x="200" y="10" width="20" height="20" fill="url(#p)"/></g>"##,
    );
    assert!(fired.status.success(), "W6 must never fail the run");
    let stderr = String::from_utf8_lossy(&fired.stderr);
    assert!(stderr.contains("outside its own canvas box"), "{stderr}");
    assert!(stderr.contains("overflows right by"), "{stderr}");

    // Negative control: a pattern whose only child paints nothing is silent, even
    // though the child element exists (R66b P2-1's five false positives).
    let (silent, silent_in, silent_out) = w6(
        "ink_pattern_href_inside",
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="4" height="4" fill="none"/></pattern></defs><g data-scale="position"><rect x="200" y="10" width="20" height="20" fill="url(#p)"/></g>"##,
    );
    assert!(silent.status.success());
    assert_eq!(
        silent.stderr.len(),
        0,
        "a non-painting pattern must stay silent, got: {}",
        String::from_utf8_lossy(&silent.stderr)
    );

    for path in [fired_in, fired_out, silent_in, silent_out] {
        let _ = fs::remove_file(path);
    }
}
