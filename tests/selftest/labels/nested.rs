// R51 A: a marker nested inside another marker used to be counted twice — the
// outer marker read the inner marker's text as part of its own letter, and the
// inner marker was collected as well, so one glyph produced two sites and a
// false duplicate (rc=2 on a figure with a single A; base `1b7ef3d` was rc=0).
// The outer marker now stops at a nested marker: the inner one owns its text.

use std::fs;
use std::path::PathBuf;

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn letter(text: &str, marker: &str) -> String {
    format!(
        r#"<text data-svg-grid-label="{marker}" x="4" y="14" font-size="12" font-family="Arial">{text}</text>"#
    )
}

fn compose(name: &str, inputs: &[PathBuf]) -> (std::process::Output, PathBuf) {
    run_cli(name, inputs, &["--width", "400", "--ncol", "2"])
}

fn stderr(result: &std::process::Output) -> String {
    String::from_utf8_lossy(&result.stderr).into_owned()
}

fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_counts_a_nested_marker_once() {
    let body = format!(r#"<g data-svg-grid-label="0">{}</g>"#, letter("A", "1"));
    let input = panel("r51_nested_once.svg", &body);
    let (result, out) = compose("r51_nested_once_out.svg", std::slice::from_ref(&input));
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "one A is not a duplicate: {err}"
    );
    assert!(!err.contains("duplicate panel letters"), "{err}");
    assert!(out.exists(), "rc=0 must write the output: {err}");
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(
        svg.matches(">A</text>").count(),
        1,
        "exactly one glyph is composed: {svg}"
    );
    cleanup(&[input, out]);
}

#[test]
fn cli_reads_the_outer_marker_text_next_to_a_nested_one() {
    // The outer marker carries "A" of its own and the nested marker "B". A
    // second input is labelled "AB": if the outer marker still swallowed the
    // nested text it would collide with "AB" and fail.
    let body = format!(
        r#"<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text>{}</g>"#,
        letter("B", "1")
    );
    let nested = panel("r51_outer_a.svg", &body);
    let ab = panel("r51_ab.svg", &letter("AB", "0"));
    let (result, out) = compose("r51_outer_a_out.svg", &[nested.clone(), ab.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "the outer marker reads only its own \"A\": {err}"
    );
    assert!(out.exists(), "rc=0 must write the output: {err}");
    cleanup(&[nested, ab, out]);
}

#[test]
fn cli_rejects_a_real_duplicate_between_an_outer_and_a_nested_marker() {
    let body = format!(
        r#"<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text>{}</g>"#,
        letter("A", "1")
    );
    let input = panel("r51_outer_inner_a.svg", &body);
    let (result, out) = compose("r51_outer_inner_a_out.svg", std::slice::from_ref(&input));
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#0 (cell 1)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[input, out]);
}

#[test]
fn cli_counts_only_the_innermost_marker_of_three_levels() {
    let body = format!(
        r#"<g data-svg-grid-label="0"><g data-svg-grid-label="1">{}</g></g>"#,
        letter("A", "2")
    );
    let input = panel("r51_three_levels.svg", &body);
    let (result, out) = compose("r51_three_levels_out.svg", std::slice::from_ref(&input));
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "one A is not a duplicate: {err}"
    );
    assert!(out.exists(), "rc=0 must write the output: {err}");
    // The innermost marker is the one that owns the glyph: pairing it with a
    // plain "A" panel must collide at cell 2, not cell 0 or 1.
    let plain_a = panel("r51_plain_a.svg", &letter("A", "0"));
    let (collides, dup_out) = compose(
        "r51_three_levels_dup.svg",
        &[input.clone(), plain_a.clone()],
    );
    let dup = stderr(&collides);
    assert_eq!(collides.status.code(), Some(2), "{dup}");
    assert!(dup.contains("#0 (cell 2)"), "{dup}");
    assert!(!dup_out.exists(), "rc=2 must not write the output: {dup}");
    cleanup(&[input, out, plain_a, dup_out]);
}

#[test]
fn cli_rejects_the_same_nested_letter_across_inputs() {
    let body = format!(r#"<g data-svg-grid-label="0">{}</g>"#, letter("A", "1"));
    let a = panel("r51_cross_a.svg", &body);
    let b = panel("r51_cross_b.svg", &body);
    let (result, out) = compose("r51_cross_out.svg", &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("#0 (cell 1)"), "{err}");
    assert!(err.contains("#1 (cell 1)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[a, b, out]);
}
