//! The guard the `WARNING_CODES` comment promises: every code is recognised from
//! the **real** message its `describe_*` function produces.
//!
//! Hand-written strings cannot catch drift — they only prove the table agrees with
//! the test (that is how W4a/W5/W7 stayed dead for several rounds, R45). So each
//! case below calls the production builder and feeds its output to `classify`.

use crate::convert::{convert, Conversion};
use crate::geom::Rect;

use super::{
    classify, describe_content_outside_canvas, describe_dangling_reference, describe_label_count,
    describe_label_duplicates, describe_moved_label, describe_passthrough, describe_scale,
    describe_stretch, CellStretch, DanglingReference, InputScale, Passthrough, WARNING_CODES,
};

/// Wrap a message the way the CLI prints it, then classify it.
fn codes_of(message: &str) -> Vec<&'static str> {
    classify(&format!("svg_grid: warning: {message}\n")).0
}

/// The real V1 text: the converter's own warning for a `<style>` block.
fn converter_style_warning() -> String {
    let source = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"80\" \
                  viewBox=\"0 0 100 80\"><style>.a{stroke-linejoin:round}</style>\
                  <rect x=\"10\" y=\"10\" width=\"80\" height=\"60\" fill=\"#000000\"/></svg>";
    match convert(source, false).expect("a plain SVG converts") {
        Conversion::Converted(done) => done
            .warnings
            .iter()
            .find(|warning| warning.contains("<style>"))
            .expect("a <style> block must be reported")
            .clone(),
        other => panic!("expected a conversion, got {other:?}"),
    }
}

#[test]
fn every_warning_code_still_matches_its_message() {
    // W3 and W8 are deliberately absent: they are hard errors (`rc=2` / `rc=3`,
    // `ARCHITECTURE/warnings.md` §3) printed with `svg_grid: invalid input:` /
    // `svg_grid: font library error:`, so `doctor` judges them by exit code and
    // `classify` never sees them. A table entry for them could never match.
    let messages: Vec<(&str, String)> = vec![
        (
            "W1",
            describe_scale(&[InputScale {
                index: 0,
                sx: 2.0,
                sy: 2.0,
            }])
            .expect("a 2x rescale warns"),
        ),
        (
            "W2",
            describe_stretch(&[CellStretch {
                index: 0,
                cell_aspect: 2.0,
                panel_aspect: 1.0,
            }])
            .expect("a 2:1 stretch warns"),
        ),
        (
            "W4a",
            describe_label_count(3, 2).expect("3 entries for 2 inputs warns"),
        ),
        (
            "W4b",
            describe_label_duplicates(&["A".to_string(), "A".to_string()])
                .expect("a repeated letter warns"),
        ),
        (
            "W5",
            describe_moved_label("A", 0, 10.0, 20.0).expect("a moved letter warns"),
        ),
        (
            "W6",
            describe_content_outside_canvas(
                0,
                (-5.0, 0.0, 100.0, 80.0),
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 80.0,
                },
            )
            .expect("ink left of the canvas warns"),
        ),
        (
            "W7",
            describe_passthrough(&[Passthrough {
                index: 0,
                element: "rect".to_string(),
                attr: "x".to_string(),
                detail: "non-finite number \"inf\"".to_string(),
            }])
            .expect("a kept-verbatim attribute warns"),
        ),
        (
            "W9",
            describe_dangling_reference(&[DanglingReference {
                index: 0,
                element: "rect".to_string(),
                attr: "fill".to_string(),
                id: "nope".to_string(),
            }])
            .expect("a dangling url(#…) warns"),
        ),
        ("V1", converter_style_warning()),
    ];

    for (code, message) in &messages {
        let codes = codes_of(message);
        assert!(
            codes.contains(code),
            "{code} is not recognised from its own message: {message:?} -> {codes:?}"
        );
    }

    // The table has no entry that no message can produce: every id in it is
    // covered above, and every covered id is in it.
    let covered: Vec<&str> = messages.iter().map(|(code, _)| *code).collect();
    let table: Vec<&str> = WARNING_CODES.iter().map(|(code, _)| *code).collect();
    assert_eq!(
        table, covered,
        "WARNING_CODES and the guard test must list the same codes"
    );
}

/// W5 has two shapes (moved inside the canvas / may be clipped); both must be
/// recognised, and neither may be dropped to keep the table short.
#[test]
fn a_code_with_two_shapes_recognises_both() {
    let clipped = super::describe_clipped_label(
        "A",
        0,
        Rect {
            x: -10.0,
            y: 0.0,
            width: 20.0,
            height: 10.0,
        },
        100.0,
        80.0,
    )
    .expect("ink left of the canvas clips the letter");
    assert!(
        codes_of(&clipped).contains(&"W5"),
        "the clipped shape must classify as W5: {clipped:?}"
    );
    let moved = describe_moved_label("A", 0, 10.0, 20.0).expect("a moved letter warns");
    assert!(
        codes_of(&moved).contains(&"W5"),
        "the moved shape must classify as W5: {moved:?}"
    );
}

/// A hard error is not a warning: its line must not be counted or classified.
#[test]
fn hard_error_lines_are_not_classified_as_warnings() {
    let stderr = "svg_grid: invalid input: panels in input #0 (letter \"A\") already carry \
                  letters (data-svg-grid-label); lettering again duplicates them.\n\
                  svg_grid: font library error: --label-fontfamily \"X\" is not in the font \
                  library (available: Arial).\n";
    let (codes, warnings) = classify(stderr);
    assert_eq!(warnings, 0, "hard errors are not warnings: {codes:?}");
    assert!(codes.is_empty(), "{codes:?}");
}
