// R51 A + R54 D: a marker owns every glyph its subtree draws, including the
// glyph of a nested marker, because that is what the renderer paints. The
// outer marker is dropped only when its whole subtree contributes no character
// of its own (it is just a wrapper around the nested marker), so a bare
// wrapper still yields one site while `<g label="0">A<text label="1">B</text>C`
// yields "ABC" for the outer and "B" for the inner.

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
fn cli_reads_the_whole_subtree_into_the_outer_marker() {
    // The outer marker draws "A" and the nested marker draws "B", so the
    // renderer paints "AB" for the outer site. A second panel labelled "AB"
    // is therefore a real duplicate, not a false positive.
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
        Some(2),
        "the outer marker paints \"AB\" and collides with an AB panel: {err}"
    );
    assert!(err.contains(r#"duplicate panel letters: "AB""#), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[nested, ab, out]);
}

#[test]
fn cli_counts_a_nested_glyph_once_inside_a_covering_outer_marker() {
    // The outer marker contributes no character of its own, so it is dropped:
    // one panel holds a single letter "A" and is not a duplicate by itself.
    let body = format!(r#"<g data-svg-grid-label="0">{}</g>"#, letter("A", "1"));
    let input = panel("r51_outer_inner_a.svg", &body);
    let (result, out) = compose("r51_outer_inner_a_out.svg", std::slice::from_ref(&input));
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "the covering outer marker adds no letter of its own: {err}"
    );
    assert!(!err.contains("duplicate panel letters"), "{err}");
    assert!(out.exists(), "rc=0 must write the output: {err}");

    // Two panels that both draw that one "A" are a real duplicate, reported at
    // the nested marker (cell 1), not at the covering outer marker.
    let second = panel("r51_outer_inner_a2.svg", &body);
    let (result, out) = compose(
        "r51_outer_inner_a_dup.svg",
        &[input.clone(), second.clone()],
    );
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("#0 (cell 1)"), "{err}");
    assert!(err.contains("#1 (cell 1)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[input, second, out]);
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
