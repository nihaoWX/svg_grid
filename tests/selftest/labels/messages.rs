// R48: the duplicate-letter error must stay actionable when many sites collide,
// must report the real cell, and must not tell the user to do something that
// cannot work (`pass unique --labels here` always trips W3).

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn letter_a(name: &str) -> PathBuf {
    panel(
        name,
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text>"#,
    )
}

fn assert_rejected(result: &std::process::Output, out: &Path) -> String {
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    assert!(
        stderr.lines().all(|line| line.chars().count() <= 200),
        "one 200-char stderr line, prefix included: {stderr}"
    );
    assert!(!out.exists(), "rc=2 must not write the output: {stderr}");
    stderr
}

#[test]
fn six_colliding_sites_keep_an_explicit_remainder() {
    let inputs: Vec<PathBuf> = (0..6)
        .map(|i| letter_a(&format!("r48_six_{i}.svg")))
        .collect();
    let (result, out) = run_cli(
        "r48_six_out.svg",
        &inputs,
        &["--width", "1200", "--ncol", "6"],
    );
    let stderr = assert_rejected(&result, &out);
    assert!(stderr.contains("\"A\""), "{stderr}");
    assert!(
        stderr.contains("+3 more") || stderr.contains("(6 places)"),
        "the omitted sites must be counted, not silently dropped: {stderr}"
    );
    for path in inputs {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn the_message_reports_the_cell_inside_a_nested_composite() {
    let p1 = path_panel("r48_cell_p1.svg", 100, 80, "");
    let p2 = path_panel("r48_cell_p2.svg", 100, 80, "");
    let (inner, inner_out) = run_cli(
        "r48_cell_inner.svg",
        &[p1.clone(), p2.clone()],
        &["--width", "400", "--ncol", "2", "--labels", ",A"],
    );
    assert!(
        inner.status.success(),
        "{}",
        String::from_utf8_lossy(&inner.stderr)
    );
    let a = letter_a("r48_cell_a.svg");
    let (result, out) = run_cli(
        "r48_cell_out.svg",
        &[inner_out.clone(), a.clone()],
        &["--width", "800", "--ncol", "2"],
    );
    let stderr = assert_rejected(&result, &out);
    assert!(
        stderr.contains("#0 (cell 1)"),
        "the letter sits in cell 1 of input #0: {stderr}"
    );
    assert!(
        stderr.contains("#1 (cell 0)"),
        "the other letter sits in cell 0 of input #1: {stderr}"
    );
    for path in [p1, p2, inner_out, a] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn the_fix_text_is_something_the_user_can_do() {
    let body = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan></text>"#;
    let a = panel("r48_fix_a.svg", body);
    let b = panel("r48_fix_b.svg", body);
    let (result, out) = run_cli(
        "r48_fix_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    let stderr = assert_rejected(&result, &out);
    assert!(
        stderr.contains("drop the inputs' data-svg-grid-label markers"),
        "the fix must name the marker the user can remove: {stderr}"
    );
    assert!(
        !stderr.contains("pass unique --labels"),
        "any non-empty --labels over a lettered input trips W3: {stderr}"
    );
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn explicit_labels_collapse_internal_whitespace_like_inherited_ones() {
    let spaced = r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A  B</text>"#;
    let a = panel("r48_space_a.svg", spaced);
    let b = path_panel("r48_space_b.svg", 100, 80, "");
    let (result, out) = run_cli(
        "r48_space_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2", "--labels", ",A B"],
    );
    let stderr = assert_rejected(&result, &out);
    assert!(stderr.contains("\"A B\""), "{stderr}");
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn an_explicit_letter_on_a_spacer_cell_still_collides() {
    // The spacer draws no letter of its own, but `--labels ,A` draws one at the
    // blank cell (R48 verified with d1render: 25 ink pixels), so it is a real
    // duplicate of the inherited A.
    let a = letter_a("r48_spacer_a.svg");
    let out = temp_path("r48_spacer_out.svg");
    let result = run_plot_grid(&[
        "--input".to_string(),
        a.to_string_lossy().into_owned(),
        "--input".to_string(),
        "none".to_string(),
        "--width".to_string(),
        "400".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--labels".to_string(),
        ",A".to_string(),
        "--output".to_string(),
        out.to_string_lossy().into_owned(),
    ]);
    let stderr = assert_rejected(&result, &out);
    assert!(stderr.contains("\"A\""), "{stderr}");
    assert!(
        stderr.contains("#1"),
        "the spacer cell is input #1: {stderr}"
    );
    let _ = fs::remove_file(a);
}
