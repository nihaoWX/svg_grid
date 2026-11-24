//! The guard the `WARNING_CODES` comment promises: every code is recognised from
//! the **real** message its `describe_*` function produces.
//!
//! Hand-written strings cannot catch drift — they only prove the table agrees with
//! the test (that is how W4a/W5/W7 stayed dead for several rounds, R45). So each
//! case below calls the production builder and feeds its output to `classify`.

use crate::convert::{convert, Conversion};
use crate::geom::Rect;

use super::{
    classify, describe_clipped_label, describe_content_outside_canvas, describe_dangling_reference,
    describe_label_count, describe_label_duplicates, describe_moved_label, describe_passthrough,
    describe_scale, describe_stretch, CellStretch, DanglingReference, InputScale, Passthrough,
    WARNING_CODES,
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
    //
    // A code may be printed in more than one shape (W5 is moved *or* clipped), so
    // each code carries every real message it can produce; the phrases of the
    // table are then checked against those messages below.
    let messages: Vec<(&str, Vec<String>)> = vec![
        (
            "W1",
            vec![describe_scale(&[InputScale {
                index: 0,
                sx: 2.0,
                sy: 2.0,
            }])
            .expect("a 2x rescale warns")],
        ),
        (
            "W2",
            vec![describe_stretch(&[CellStretch {
                index: 0,
                cell_aspect: 2.0,
                panel_aspect: 1.0,
            }])
            .expect("a 2:1 stretch warns")],
        ),
        (
            "W4a",
            vec![describe_label_count(3, 2).expect("3 entries for 2 inputs warns")],
        ),
        (
            "W4b",
            vec![
                describe_label_duplicates(&["A".to_string(), "A".to_string()])
                    .expect("a repeated letter warns"),
            ],
        ),
        (
            "W5",
            vec![
                describe_moved_label("A", 0, 10.0, 20.0).expect("a moved letter warns"),
                describe_clipped_label(
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
                .expect("ink left of the canvas clips the letter"),
            ],
        ),
        (
            "W6",
            vec![describe_content_outside_canvas(
                0,
                (-5.0, 0.0, 100.0, 80.0),
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 80.0,
                },
            )
            .expect("ink left of the canvas warns")],
        ),
        (
            "W7",
            vec![describe_passthrough(&[Passthrough {
                index: 0,
                element: "rect".to_string(),
                attr: "x".to_string(),
                detail: "non-finite number \"inf\"".to_string(),
            }])
            .expect("a kept-verbatim attribute warns")],
        ),
        (
            "W9",
            vec![describe_dangling_reference(&[DanglingReference {
                index: 0,
                element: "rect".to_string(),
                attr: "fill".to_string(),
                id: "nope".to_string(),
            }])
            .expect("a dangling url(#…) warns")],
        ),
        ("V1", vec![converter_style_warning()]),
    ];

    for (code, group) in &messages {
        for message in group {
            let codes = codes_of(message);
            assert!(
                codes.contains(code),
                "{code} is not recognised from its own message: {message:?} -> {codes:?}"
            );
        }
    }

    // Every *phrase* of the table must occur in some real message of its code:
    // a phrase added for a shape no builder emits would otherwise read as
    // "covered" while covering nothing (R45 review P3-a).
    for (code, phrases) in WARNING_CODES {
        let group = messages
            .iter()
            .find(|(candidate, _)| candidate == code)
            .map(|(_, group)| group)
            .unwrap_or_else(|| panic!("{code} has no real message in the guard"));
        for phrase in *phrases {
            assert!(
                group.iter().any(|message| message.contains(phrase)),
                "the phrase {phrase:?} of {code} never occurs in a real message: {group:?}"
            );
        }
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

/// The W9 phrase, taken from the table itself so the guard below cannot drift.
fn w9_phrase() -> &'static str {
    WARNING_CODES
        .iter()
        .find(|(code, _)| *code == "W9")
        .expect("W9 is in WARNING_CODES")
        .1[0]
}

/// A W9-shaped line whose dangling id is `id` (the id is printed verbatim).
fn w9_guard_line(id: &str) -> String {
    format!(
        "svg_grid: warning: 1 reference(s) {}: input #0 <rect fill> -> url(#{id})\n",
        w9_phrase()
    )
}

/// The core invariant of R53: W9 prints the dangling id **verbatim**, so user
/// content can carry any other code's phrase. Every phrase of every code is put
/// into that id; W9 must still be classified, or `doctor` fails open.
#[test]
fn a_phrase_inside_a_dangling_id_cannot_mask_w9() {
    // The guard line must keep the production head, or it would prove nothing.
    let production = describe_dangling_reference(&[DanglingReference {
        index: 0,
        element: "rect".to_string(),
        attr: "fill".to_string(),
        id: "nope".to_string(),
    }])
    .expect("a dangling url(#…) warns");
    let head = w9_guard_line("nope");
    let head = head
        .strip_prefix("svg_grid: warning: ")
        .expect("the guard line keeps the CLI prefix")
        .trim_end();
    assert!(
        production.starts_with(head),
        "the guard's W9 shape drifted from the production message: {production:?} vs {head:?}"
    );

    for (code, phrases) in WARNING_CODES {
        for phrase in *phrases {
            let line = w9_guard_line(phrase);
            assert!(
                line.contains(phrase),
                "the guard did not embed {phrase:?} in the dangling id"
            );
            let (codes, warnings) = classify(&line);
            assert_eq!(warnings, 1, "{line:?}");
            assert!(
                codes.contains(&"W9"),
                "a {code} phrase ({phrase:?}) inside the dangling id masked W9: {codes:?}"
            );
        }
    }
}

/// The same invariant on W7: the kept attribute value is printed verbatim, so a
/// phrase of any other code inside it must not steal the W7 verdict.
#[test]
fn a_phrase_inside_a_kept_verbatim_value_cannot_mask_w7() {
    for (code, phrases) in WARNING_CODES {
        for phrase in *phrases {
            let message = describe_passthrough(&[Passthrough {
                index: 0,
                element: "path".to_string(),
                attr: "d".to_string(),
                detail: format!(
                    "unparseable path data \"{phrase}\" (missing argument for path command 'L')"
                ),
            }])
            .expect("a kept-verbatim attribute warns");
            assert!(
                message.contains(phrase),
                "the production W7 message dropped {phrase:?}: {message:?}"
            );
            let codes = codes_of(&message);
            assert!(
                codes.contains(&"W7"),
                "a {code} phrase ({phrase:?}) inside the kept value masked W7: {codes:?}"
            );
        }
    }
}

/// The footer is excluded by its **message**, not by a substring anywhere in the
/// line: a real warning whose body mentions `non-fatal warning(s)` is a warning.
#[test]
fn only_the_footer_is_excluded_not_a_body_mentioning_it() {
    let footer = "svg_grid: warning: non-fatal warning(s) — the exit code and the output \
                  file are unchanged\n";
    assert_eq!(
        classify(footer),
        (Vec::new(), 0),
        "the footer is not a warning"
    );

    let body = w9_guard_line("non-fatal warning(s)");
    let (codes, warnings) = classify(&body);
    assert_eq!(
        warnings, 1,
        "a warning body mentioning the footer is still a warning"
    );
    assert!(
        codes.contains(&"W9"),
        "the W9 body must stay classified: {codes:?}"
    );
}
