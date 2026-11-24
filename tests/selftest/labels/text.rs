// R51 C: `normalize_label` used `str::split_whitespace`, which treats U+00A0 as
// layout whitespace — so `A` and `A&#160;` were reported as the same letter and
// the figure was refused (rc=2) although both render to identical pixels
// (d1render/cairosvg: PNG diff 0). SVG's default `xml:space="default"` collapses
// only ASCII whitespace, so U+00A0 stays part of the letter.
//
// The mirror-image hole: zero-width characters add no ink and no advance, so
// `A` and `A\u{200b}` render identically (diff 0) but used to be accepted as two
// different letters (rc=0). They are now removed before comparing.

use std::fs;
use std::path::PathBuf;

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn letter(text: &str) -> String {
    format!(
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">{text}</text>"#
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
fn cli_keeps_a_no_break_space_significant() {
    let plain = panel("r51_nbsp_plain.svg", &letter("A"));
    let nbsp = panel("r51_nbsp_padded.svg", &letter("A&#160;"));
    let (result, out) = compose("r51_nbsp_out.svg", &[plain.clone(), nbsp.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "A and A&#160; are different letters: {err}"
    );
    assert!(!err.contains("duplicate panel letters"), "{err}");
    assert!(out.exists(), "rc=0 must write the output: {err}");

    // Two identical `A&#160;` panels still collide: the character is compared,
    // not dropped.
    let (twice, twice_out) = compose("r51_nbsp_twice.svg", &[nbsp.clone(), nbsp.clone()]);
    let twice_err = stderr(&twice);
    assert_eq!(twice.status.code(), Some(2), "{twice_err}");
    assert!(twice_err.contains("duplicate panel letters"), "{twice_err}");
    assert!(
        !twice_out.exists(),
        "rc=2 must not write the output: {twice_err}"
    );
    cleanup(&[plain, nbsp, out, twice_out]);
}

#[test]
fn cli_strips_zero_width_characters_before_comparing() {
    let plain = panel("r51_zwsp_plain.svg", &letter("A"));
    let padded = panel("r51_zwsp_padded.svg", &letter("A\u{200b}"));
    let (result, out) = compose("r51_zwsp_out.svg", &[plain.clone(), padded.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(2),
        "A and A\\u200b look identical, so they are one letter: {err}"
    );
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[plain, padded, out]);
}
