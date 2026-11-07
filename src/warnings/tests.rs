//! Unit tests for the non-fatal warning messages.
//!
//! Besides the per-code triggers, these tests pin the three presentation rules of
//! `ARCHITECTURE/warnings.md` §1: every printed line fits [`MAX_LINE`] (prefix
//! included), a shared cause is one note printed once per run, and a single
//! offender is never wrapped in a count (`1 input`, never `1 input(s)`).

use super::*;
use crate::geom::Rect;

/// Printed length of a warning body (prefix included).
pub(super) fn line(body: &str) -> usize {
    WARNING_PREFIX.chars().count() + body.chars().count()
}

/// Printed length of a note body (prefix included).
pub(super) fn note_line(note: &str) -> usize {
    NOTE_PREFIX.chars().count() + note.chars().count()
}

pub(super) fn scale(index: usize, sx: f32, sy: f32) -> InputScale {
    InputScale { index, sx, sy }
}

pub(super) fn stretch(index: usize, cell_aspect: f32, panel_aspect: f32) -> CellStretch {
    CellStretch {
        index,
        cell_aspect,
        panel_aspect,
    }
}

/// No warning body may contain the parenthetical plural of a count
/// (`warnings.md` §1 rule 3).
fn assert_no_count_wrapper(message: &str) {
    for wrapper in ["input(s)", "cell(s)", "attribute(s)", "reference(s)"] {
        // `reference(s)` is the one frozen exception: `round-data/R16` quotes
        // `1 reference(s) point at an id that nothing in the output defines`
        // verbatim, so W9 keeps that spelling (see `refs.rs`).
        if wrapper == "reference(s)" {
            continue;
        }
        assert!(!message.contains(wrapper), "{wrapper} in {message}");
    }
    assert!(
        !message.starts_with("1 ") || !message.contains("(s)"),
        "a single offender must not be counted: {message}"
    );
}

#[test]
fn scale_warning_fires_for_the_099032_outer_margin_bug() {
    // (W - 2*24)/W at W = 4961 is exactly the silent ~1 % shrink this warning
    // exists for; a 1 % tolerance would miss it.
    let message = describe_scale(&[scale(0, 4913.0 / 4961.0, 4913.0 / 4961.0)])
        .expect("0.99032 must be reported");
    assert!(message.contains("0.99032"), "{message}");
    assert!(message.contains("scaled by the layout"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn scale_warning_is_silent_at_unit_scale_and_within_tolerance() {
    assert!(describe_scale(&[scale(0, 1.0, 1.0)]).is_none());
    assert!(describe_scale(&[scale(0, 1.004, 0.997)]).is_none()); // within 0.5 %
    assert!(describe_scale(&[scale(0, 1.006, 1.0)]).is_some()); // just over
}

#[test]
fn scale_warning_groups_multiple_inputs_into_one_message() {
    let message = describe_scale(&[scale(0, 0.99, 0.99), scale(1, 2.0, 2.0)]).unwrap();
    assert_eq!(message.matches("scaled by the layout").count(), 1);
    assert!(
        message.contains("#0") && message.contains("#1"),
        "{message}"
    );
    assert!(message.contains("2 inputs are scaled"), "{message}");
    // `sx = sy` prints one number, not two.
    assert!(message.contains("#0 sx=0.99000"), "{message}");
}

#[test]
fn a_single_offender_is_never_counted_as_1_inputs() {
    let message = describe_scale(&[scale(0, 1.5, 1.5)]).unwrap();
    assert!(message.contains("1 input is scaled"), "{message}");
    assert_no_count_wrapper(&message);
}

#[test]
fn stretch_warning_fires_and_reports_the_percent() {
    let message = describe_stretch(&[stretch(0, 1.738, 1.25)]).expect("stretch must be shown");
    assert!(message.contains("stretched +39.0%"), "{message}");
    assert!(message.contains("--rel-*"), "{message}");
    assert!(message.contains("1 cell is stretched"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn stretch_warning_is_silent_when_aspects_match_or_are_close() {
    assert!(describe_stretch(&[stretch(0, 1.25, 1.25)]).is_none());
    assert!(describe_stretch(&[stretch(0, 1.255, 1.25)]).is_none()); // within 1 %
    assert!(describe_stretch(&[stretch(0, 1.30, 1.25)]).is_some()); // over 1 %
}

#[test]
fn existing_label_warning_only_when_an_input_is_lettered() {
    assert!(describe_existing_labels(&[]).is_none());
    let one = describe_existing_labels(&[0]).unwrap();
    assert!(one.contains("already contain panel letters"), "{one}");
    assert!(one.contains("data-svg-grid-label"), "{one}");
    assert!(one.contains("input #0"), "{one}");
    assert_no_count_wrapper(&one);
    let message = describe_existing_labels(&[0, 3]).unwrap();
    assert!(
        message.contains("#0") && message.contains("#3"),
        "{message}"
    );
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn label_count_warning_only_on_mismatch() {
    assert!(describe_label_count(3, 3).is_none());
    let message = describe_label_count(2, 3).unwrap();
    assert!(
        message.contains("2 entries") && message.contains("3 inputs"),
        "{message}"
    );
    // A single input never prints the parenthetical plural.
    let single = describe_label_count(2, 1).unwrap();
    assert!(single.contains("the grid has 1 input:"), "{single}");
    assert_no_count_wrapper(&single);
}

#[test]
fn label_duplicate_warning_ignores_empty_slots() {
    let owned = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(describe_label_duplicates(&owned(&["A", "B", "C"])).is_none());
    assert!(describe_label_duplicates(&owned(&["A", "", "C"])).is_none());
    let message = describe_label_duplicates(&owned(&["A", "", "A"])).unwrap();
    assert!(message.contains("\"A\""), "{message}");
    assert!(message.contains("0, 2"), "{message}");
    assert!(message.contains("repeats some letters"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn clipped_label_warning_respects_the_canvas_bounds() {
    let inside = Rect {
        x: 1.0,
        y: 1.0,
        width: 10.0,
        height: 10.0,
    };
    assert!(describe_clipped_label("A", 0, inside, 100.0, 100.0).is_none());
    let outside = Rect {
        x: -5.0,
        y: 1.0,
        width: 10.0,
        height: 10.0,
    };
    let message = describe_clipped_label("A", 0, outside, 100.0, 100.0).unwrap();
    assert!(
        message.contains("may be clipped by the canvas"),
        "{message}"
    );
    assert!(message.contains("Fix:"), "{message}");
    assert!(message.contains("--plot-margin"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

fn canvas() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
    }
}

#[test]
fn content_outside_warning_reports_the_side_the_amount_and_the_input() {
    // A text starting 1.7 units to the LEFT of the canvas.
    let message = describe_content_outside_canvas(0, (-1.7, 5.0, 30.0, 20.0), canvas())
        .expect("ink left of the canvas must be reported");
    assert!(message.contains("outside its own canvas box"), "{message}");
    assert!(message.contains("input #0"), "{message}");
    assert!(message.contains("left by 1.700 px"), "{message}");
    assert!(message.contains("-1.700"), "{message}");
    // The fix must be on the line; *why* it matters is the shared note.
    assert!(message.contains("Fix:"), "{message}");
    assert!(!message.contains("raster"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn content_outside_warning_respects_the_one_px_tolerance() {
    // Within 1 px on every side: silent.
    assert!(describe_content_outside_canvas(0, (-0.5, -0.5, 100.5, 80.5), canvas()).is_none());
    // Exactly on the tolerance: still silent.
    assert!(describe_content_outside_canvas(0, (-1.0, 5.0, 100.0, 80.0), canvas()).is_none());
    // Just over on the bottom: reported.
    let message = describe_content_outside_canvas(0, (0.0, 0.0, 100.0, 81.5), canvas())
        .expect("ink below the canvas must be reported");
    assert!(message.contains("bottom by 1.500 px"), "{message}");
}

#[test]
fn content_outside_warning_adds_the_dust_hint_only_below_two_px() {
    // 1.5 px — under 2 px: the verdict stands and the hint is appended.
    let small = describe_content_outside_canvas(0, (0.0, 0.0, 100.0, 81.5), canvas()).unwrap();
    assert!(small.contains("bottom by 1.500 px"), "{small}");
    assert!(small.contains(DUST_HINT), "{small}");

    // Exactly 2 px is no longer "dust": the hint is suppressed.
    let two = describe_content_outside_canvas(0, (0.0, 0.0, 100.0, 82.0), canvas()).unwrap();
    assert!(!two.contains(DUST_HINT), "{two}");

    // One 40 px overflow suppresses the hint even if another is dust.
    let big = describe_content_outside_canvas(0, (0.0, 0.0, 100.0, 120.0), canvas()).unwrap();
    assert!(!big.contains(DUST_HINT), "{big}");
    let mixed = describe_content_outside_canvas(0, (-1.5, 0.0, 140.0, 80.0), canvas()).unwrap();
    assert!(!mixed.contains(DUST_HINT), "{mixed}");
}

#[test]
fn content_outside_warning_lists_every_side_and_still_fits() {
    // All four sides overflow: the line must still fit. The richest rendering that
    // fits lists every side; when none does, the worst side is named and the rest
    // are counted (never a silently truncated list).
    let message = describe_content_outside_canvas(0, (-12.5, -30.25, 140.0, 120.0), canvas())
        .expect("every side overflows");
    assert!(message.contains("by 40.000 px"), "{message}");
    assert!(message.contains("worst of 3 more sides"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn passthrough_warning_names_the_input_the_element_and_the_attribute() {
    let mut seen = Passthroughs::default();
    seen.record(
        0,
        "path",
        "d",
        "unparseable path data (missing argument for path command 'L')".to_string(),
    );
    let message = describe_passthrough(seen.offenders()).expect("must report");
    assert!(message.contains("input #0"), "{message}");
    assert!(message.contains("<path d>"), "{message}");
    assert!(message.contains("kept verbatim"), "{message}");
    assert!(message.contains("could not be rewritten"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
    assert_no_count_wrapper(&message);
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );

    assert!(describe_passthrough(&[]).is_none());
}

#[test]
fn passthrough_warning_deduplicates_and_groups_offenders_into_one_message() {
    let mut seen = Passthroughs::default();
    let detail = "non-finite number \"nan\"".to_string();
    seen.record(0, "circle", "cx", detail.clone());
    seen.record(0, "circle", "cx", detail.clone()); // identical → deduplicated
    seen.record(1, "circle", "cx", detail); // a different input survives
    assert_eq!(seen.offenders().len(), 2, "{:?}", seen.offenders());

    let message = describe_passthrough(seen.offenders()).unwrap();
    // One message, both offenders listed, one head: `kept verbatim` is said once.
    assert!(
        message.contains("input #0") && message.contains("input #1"),
        "{message}"
    );
    assert_eq!(
        message.matches("could not be rewritten").count(),
        1,
        "{message}"
    );
    // With two offenders the value is what the line keeps (the reason would not
    // fit); the cause note carries the "malformed or non-finite" explanation.
    assert!(message.contains("\"nan\""), "{message}");
    assert_eq!(message.matches("kept verbatim").count(), 1, "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn passthrough_warning_still_tells_two_offenders_of_one_attribute_apart() {
    // Two malformed `d`s of the same input: the line must say *which* values
    // (the value is what distinguishes them) and still fit the cap.
    let mut seen = Passthroughs::default();
    seen.record(
        0,
        "path",
        "d",
        "unparseable path data \"M 10 20 30\" (missing argument for path command 'L')".to_string(),
    );
    seen.record(
        0,
        "path",
        "d",
        "unparseable path data \"M0 0 Lnan nan\" (missing argument for path command 'L')"
            .to_string(),
    );
    let message = describe_passthrough(seen.offenders()).unwrap();
    assert!(message.contains("2 attributes"), "{message}");
    assert!(message.contains("kept verbatim"), "{message}");
    assert!(
        message.contains("M 10 20 30") && message.contains("M0 0 Lnan nan"),
        "{message}"
    );
    assert_eq!(message.matches("input #0").count(), 2, "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

pub(super) fn dangling(index: usize, element: &str, attr: &str, id: &str) -> DanglingReference {
    DanglingReference {
        index,
        element: element.to_string(),
        attr: attr.to_string(),
        id: id.to_string(),
    }
}

#[test]
fn dangling_reference_warning_names_the_input_the_element_and_the_id() {
    let message = describe_dangling_reference(&[dangling(0, "rect", "fill", "pat-0")])
        .expect("a dangling url(#…) must be reported");
    assert!(message.contains("input #0"), "{message}");
    assert!(message.contains("<rect fill>"), "{message}");
    assert!(message.contains("url(#pat-0)"), "{message}");
    assert!(message.contains("loses ink"), "{message}");
    assert!(message.contains("painted with nothing"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
    assert!(message.contains("<defs>"), "{message}");
    // The fix still names the kind of resource a paint server must be.
    assert!(message.contains("<linearGradient>"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );

    assert!(describe_dangling_reference(&[]).is_none());
}

#[test]
fn dangling_reference_warning_keeps_every_id_and_still_fits_the_cap() {
    let two = vec![
        dangling(0, "rect", "clip-path", "no-such-clip"),
        dangling(0, "rect", "style", "no-such-fill"),
    ];
    let message = describe_dangling_reference(&two).unwrap();
    assert!(message.contains("2 reference(s)"), "{message}");
    assert!(
        message.contains("url(#no-such-clip)") && message.contains("url(#no-such-fill)"),
        "{message}"
    );
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );

    let many: Vec<DanglingReference> = (0..8)
        .map(|n| dangling(0, "rect", "fill", &format!("pat-{n}")))
        .collect();
    let message = describe_dangling_reference(&many).unwrap();
    assert_eq!(
        message.matches("reference(s) point at").count(),
        1,
        "{message}"
    );
    assert!(message.contains("pat-0"), "{message}");
    assert!(message.contains("more"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
}

#[test]
fn dangling_references_are_collected_per_attribute_and_deduplicated() {
    let root = crate::dom::parse(
        r#"<svg><defs><linearGradient id="kept"/></defs>
<rect fill="url(#kept)" stroke="url(#gone)" style="fill: url(#gone)"/>
<rect fill="url(#gone)"/></svg>"#,
    )
    .unwrap();
    let found = dangling_references(&root, 2);
    // `#kept` resolves; `#gone` is reported once per (element, attr) — never per
    // occurrence, and never twice for the same attribute.
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found.iter().all(|offender| offender.id == "gone"),
        "{found:?}"
    );
    assert!(
        found.iter().all(|offender| offender.index == 2),
        "{found:?}"
    );
    let attrs: Vec<&str> = found.iter().map(|o| o.attr.as_str()).collect();
    assert_eq!(attrs, vec!["stroke", "style", "fill"], "{found:?}");

    let clean = crate::dom::parse(
        r#"<svg><defs><linearGradient id="kept"/></defs><rect fill="url(#kept)"/></svg>"#,
    )
    .unwrap();
    assert!(dangling_references(&clean, 0).is_empty());
}

#[test]
fn missing_font_warning_fits_the_cap_and_says_where_it_lands() {
    let available = vec!["Arial".to_string()];
    let message = describe_missing_font("NoSuchFont_xyz", &available).unwrap();
    assert!(message.contains("not in the font library"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
    assert!(message.contains("--list-fonts"), "{message}");
    assert!(
        line(&message) <= MAX_LINE,
        "{} ch: {message}",
        line(&message)
    );
    // A family the library holds is never reported.
    assert!(describe_missing_font("Arial", &available).is_none());
}
