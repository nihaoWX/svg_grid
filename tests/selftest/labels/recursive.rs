// R48: a panel letter that is split across a `<tspan>`, wrapped in a `<g>`, or
// padded with whitespace used to slip past the duplicate check (rc=0, silent
// duplicate). The marker's whole subtree is now read in document order and
// normalized like an explicit `--labels` entry.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn compose(name: &str, inputs: &[PathBuf], extra: &[&str]) -> (std::process::Output, PathBuf) {
    run_cli(name, inputs, extra)
}

fn assert_duplicate(result: &std::process::Output, out: &Path, letter: &str) {
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    assert!(stderr.contains(&format!("\"{letter}\"")), "{stderr}");
    assert!(stderr.contains("#0") && stderr.contains("#1"), "{stderr}");
    assert!(
        stderr.lines().all(|line| line.chars().count() <= 200),
        "one 200-char stderr line, prefix included: {stderr}"
    );
    assert!(!out.exists(), "rc=2 must not write the output: {stderr}");
}

#[test]
fn cli_rejects_a_letter_split_into_a_tspan() {
    let body = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan></text>"#;
    let a = panel("r48_tspan_a.svg", body);
    let b = panel("r48_tspan_b.svg", body);
    let (result, out) = compose(
        "r48_tspan_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_duplicate(&result, &out, "A");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_rejects_a_letter_wrapped_in_a_group_marker() {
    let body = r#"<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#;
    let a = panel("r48_group_a.svg", body);
    let b = panel("r48_group_b.svg", body);
    let (result, out) = compose(
        "r48_group_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_duplicate(&result, &out, "A");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_concatenates_the_marker_subtree_in_document_order() {
    let inline = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A<tspan>B</tspan></text>"#;
    let nested = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan><tspan>B</tspan></text>"#;
    let a = panel("r48_order_a.svg", inline);
    let b = panel("r48_order_b.svg", nested);
    let (result, out) = compose(
        "r48_order_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_duplicate(&result, &out, "AB");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_decodes_an_entity_once_and_compares_the_text() {
    let entity = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">&#65;</text>"#;
    let plain =
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text>"#;
    let a = panel("r48_entity_a.svg", entity);
    let b = panel("r48_entity_b.svg", plain);
    let (result, out) = compose(
        "r48_entity_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_duplicate(&result, &out, "A");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_normalizes_surrounding_whitespace() {
    let padded = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"> A </text>"#;
    let plain =
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text>"#;
    let a = panel("r48_pad_a.svg", padded);
    let b = panel("r48_pad_b.svg", plain);
    let (result, out) = compose(
        "r48_pad_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_duplicate(&result, &out, "A");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_ignores_a_marker_that_normalizes_to_nothing() {
    let empty = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">   </text>"#;
    let a = panel("r48_empty_a.svg", empty);
    let b = panel(
        "r48_empty_b.svg",
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text>"#,
    );
    let (result, out) = compose(
        "r48_empty_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(0), "stderr: {stderr}");
    assert!(out.exists(), "an accepted call writes the output");
    for path in [a, b, out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn a_rejected_call_does_not_truncate_an_existing_output() {
    let body = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan></text>"#;
    let a = panel("r48_keep_a.svg", body);
    let b = panel("r48_keep_b.svg", body);
    let out = temp_path("r48_keep_out.svg");
    fs::write(&out, "SENTINEL\n").unwrap();
    let result = run_plot_grid(&[
        "--input".to_string(),
        a.to_string_lossy().into_owned(),
        "--input".to_string(),
        b.to_string_lossy().into_owned(),
        "--width".to_string(),
        "400".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--output".to_string(),
        out.to_string_lossy().into_owned(),
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&out).unwrap(), "SENTINEL\n");
    for path in [a, b, out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn plan_fails_without_printing_a_plan() {
    let body = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan></text>"#;
    let a = panel("r48_plan_a.svg", body);
    let b = panel("r48_plan_b.svg", body);
    let result = run_plot_grid(&[
        "--input".to_string(),
        a.to_string_lossy().into_owned(),
        "--input".to_string(),
        b.to_string_lossy().into_owned(),
        "--width".to_string(),
        "400".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--plan".to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(2), "stderr: {stderr}");
    assert!(result.stdout.is_empty(), "no plan for a rejected figure");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}
