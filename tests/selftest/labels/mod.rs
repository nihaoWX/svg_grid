// Panel letters: auto labels, size/position, the `--labels` CLI semantics.

use std::fs;

use svg_grid::{
    auto_labels, plot_grid, AlignMode, LabelSize, LabelSpec, Margins, PlotGridSpec, SvgInput,
};

use crate::common::*;

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
        font_families: None,
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
        margin: Margins {
            top: 40.0,
            ..Margins::zero()
        },
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
        font_families: None,
    })
    .unwrap();

    let svg = fs::read_to_string(&output).unwrap();
    // size = 0.02 / 0.74 * 1000 = 27.027 (auto = canvas width * 2.7 %).
    assert!(svg.contains(r#"font-size="27.027""#), "{svg}");
    // x = panel left edge (0); cell top is the 40 px margin, and the baseline is
    // lifted 0.25 * size = 6.757 above it (still inside the canvas).
    assert!(svg.contains(r#"x="0" y="33.243""#), "{svg}");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
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
        // These assertions inspect the warning surface; opt in explicitly.
        args.push("--verbose".to_string());
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
        // These assertions inspect the warning surface; opt in explicitly.
        args.push("--verbose".to_string());
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
        stderr.contains("warning") && stderr.contains("moved inside the canvas"),
        "a too-small margin must warn that the letter moved inward: {stderr}"
    );

    let _ = fs::remove_file(&input_path);
}

/// The text of every `data-svg-grid-label` element, indexed by its slot.
fn label_texts(svg: &str) -> Vec<Option<String>> {
    let mut labels = Vec::new();
    for index in 0..8 {
        let needle = format!(r#"<text data-svg-grid-label="{index}""#);
        let Some(start) = svg.find(&needle) else {
            labels.push(None);
            continue;
        };
        let mut tail = &svg[start..];
        if let Some(gt) = tail.find('>') {
            tail = &tail[gt + 1..];
        }
        let text = tail.split("</text>").next().unwrap_or_default().to_string();
        labels.push(Some(text));
    }
    labels
}

#[test]
fn auto_labels_skip_null_and_explicit_labels_can_fill_the_empty_cell() {
    let panels = write_panels(&["auto_null_a", "auto_null_b", "auto_null_c", "auto_null_d"]);
    let out = temp_path("labels_null_out.svg");

    let mut args: Vec<String> = Vec::new();
    for input in [&panels[0], &panels[1]] {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.push("--input".to_string());
    args.push("NULL".to_string());
    for input in [&panels[2], &panels[3]] {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.extend(
        [
            "--output",
            out.to_str().unwrap(),
            "--width",
            "500",
            "--height",
            "300",
            "--ncol",
            "2",
            "--labels",
        ]
        .iter()
        .map(|value| value.to_string()),
    );

    // Bare --labels: NULL does not consume a letter.
    let auto = run_plot_grid(&args);
    assert!(
        auto.status.success(),
        "{}",
        String::from_utf8_lossy(&auto.stderr)
    );
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(
        label_texts(&svg),
        vec![
            Some("A".to_string()),
            Some("B".to_string()),
            None,
            Some("C".to_string()),
            Some("D".to_string()),
            None,
            None,
            None,
        ],
        "{svg}"
    );

    // Explicit list: slot 2 is NULL, but the letter X is still drawn in its cell.
    let mut explicit = args.clone();
    explicit.pop(); // the bare --labels flag
    explicit.extend(
        ["--labels", "A,B,X,,E"]
            .iter()
            .map(|value| value.to_string()),
    );
    let explicit = run_plot_grid(&explicit);
    assert!(
        explicit.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(
        label_texts(&svg),
        vec![
            Some("A".to_string()),
            Some("B".to_string()),
            Some("X".to_string()),
            None,
            Some("E".to_string()),
            None,
            None,
            None,
        ],
        "{svg}"
    );
    // The empty explicit slot skipped D's cell; no letter is attached to it.
    assert!(!svg.contains(r#"data-svg-grid-label="3""#), "{svg}");

    for path in [out].iter().chain(panels.iter()) {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn margin_zero_moves_every_letter_inside_the_canvas() {
    let panels = write_panels(&[
        "margin_zero_a",
        "margin_zero_b",
        "margin_zero_c",
        "margin_zero_d",
        "margin_zero_e",
    ]);
    let out = temp_path("labels_margin_zero_out.svg");
    let mut args: Vec<String> = Vec::new();
    for input in &panels {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.extend(
        [
            "--output",
            out.to_str().unwrap(),
            "--width",
            "500",
            "--height",
            "160",
            "--ncol",
            "5",
            "--plot-margin",
            "0",
            "--labels",
            "--verbose",
        ]
        .iter()
        .map(|value| value.to_string()),
    );

    let result = run_plot_grid(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = fs::read_to_string(&out).unwrap();
    for index in 0..5 {
        let needle = format!(r#"<text data-svg-grid-label="{index}""#);
        let start = svg.find(&needle).expect("every letter must be drawn");
        let tag = &svg[start..start + svg[start..].find('>').unwrap()];
        let y: f32 = tag
            .split(r#"y=""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .and_then(|value| value.parse().ok())
            .expect("label y must parse");
        assert!(
            y >= 0.0,
            "letter {index} must be inside the canvas: y={y}\n{svg}"
        );
    }
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("was moved inside the canvas"),
        "the moved letter must be explained: {stderr}"
    );

    let _ = fs::remove_file(out);
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

mod duplicates;
mod messages;
mod recursive;
mod visibility;
