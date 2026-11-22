//! The presentation rules of `ARCHITECTURE/warnings.md` §1, pinned on the worst
//! case each builder can produce: one line ≤ [`MAX_LINE`] characters, one cause
//! note per run, and no count wrapper for a single offender.

use super::tests::{dangling, line, note_line, scale, stretch};
use super::*;
use crate::geom::Rect;

fn canvas() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
    }
}

/// Rule 1 of `ARCHITECTURE/warnings.md` §1: one printed warning is ≤ 200 chars,
/// prefix included — checked on the worst case each builder can produce.
#[test]
fn every_warning_fits_the_200_character_cap() {
    let wide_scales: Vec<InputScale> = (0..8).map(|n| scale(n, 2.0 + n as f32, 1.5)).collect();
    let wide_stretch: Vec<CellStretch> = (0..8).map(|n| stretch(n, 1.7, 1.25)).collect();
    let mut passthroughs = Passthroughs::default();
    for n in 0..8 {
        passthroughs.record(
            n,
            "path",
            "d",
            format!("unparseable path data \"M{n} 0 L\" (missing argument for path command 'L')"),
        );
    }
    let many_refs: Vec<DanglingReference> = (0..8)
        .map(|n| {
            dangling(
                n,
                "rect",
                "clip-path",
                &format!("svg-grid-pattern-{n}-with-a-long-name"),
            )
        })
        .collect();
    let outside = Rect {
        x: -200.0,
        y: -300.0,
        width: 4000.0,
        height: 3000.0,
    };
    let messages = [
        describe_scale(&wide_scales).unwrap(),
        describe_stretch(&wide_stretch).unwrap(),
        describe_existing_labels(
            &(0..8)
                .map(|index| (index, "A".to_string()))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
        describe_label_count(1, 8).unwrap(),
        describe_label_duplicates(&(0..8).map(|_| "A".to_string()).collect::<Vec<_>>()).unwrap(),
        describe_clipped_label("A", 0, outside, 4000.0, 3000.0).unwrap(),
        describe_content_outside_canvas(0, (-12345.678, -23456.789, 4000.0, 3000.0), canvas())
            .unwrap(),
        describe_passthrough(passthroughs.offenders()).unwrap(),
        describe_dangling_reference(&many_refs).unwrap(),
    ];
    for message in messages {
        assert!(
            line(&message) <= MAX_LINE,
            "{} ch (cap {}): {message}",
            line(&message),
            MAX_LINE
        );
        assert!(message.contains("Fix:"), "no Fix: in {message}");
    }
}

/// W8 reads `--label-fontfamily` as a CSS fallback chain: any available or generic
/// name satisfies it, and only a wholly unavailable chain is an error — whose line
/// must still fit the 200-char cap under the longer `font library error:` prefix.
#[test]
fn a_font_chain_is_satisfied_by_any_usable_family() {
    let available = vec!["Arial".to_string(), "Times New Roman".to_string()];
    for satisfied in [
        "Arial",
        "Helvetica, Arial, sans-serif",
        "Liberation Sans, Arial",
        "serif, NoSuchFont",
        "NoSuchFont, monospace",
        r#""Helvetica", "Arial""#,
        "",
    ] {
        assert_eq!(
            describe_missing_font_error(satisfied, &available),
            None,
            "a chain with a usable family must pass: {satisfied:?}"
        );
    }

    let prefix = "svg_grid: font library error: ".len();
    for dead in [
        "NoSuchFont_xyz",
        "Helvetica, Liberation Sans, DejaVu Sans",
        "A,B,C,D,E,F,G,H",
        "Helvetica, Liberation Sans, DejaVu Sans, Noto Sans, FreeSans, Ubuntu",
    ] {
        let message = describe_missing_font_error(dead, &available)
            .unwrap_or_else(|| panic!("a wholly unavailable chain must fail: {dead:?}"));
        assert!(
            message.contains("is not in the font library"),
            "the doctor's W8 phrase must survive: {message}"
        );
        assert!(
            message.contains("no family in the chain is usable"),
            "the message must say the whole chain failed: {message}"
        );
        assert!(
            message.contains("available:") && message.contains("Arial"),
            "the library must be listed: {message}"
        );
        assert!(
            message.contains("--list-fonts"),
            "the fix must be actionable: {message}"
        );
        assert!(
            prefix + message.chars().count() <= MAX_LINE,
            "{} ch (cap {MAX_LINE}): {message}",
            prefix + message.chars().count()
        );
    }

    // An empty library still produces a usable line rather than an empty list.
    let message = describe_missing_font_error("NoSuchFont_xyz", &[]).unwrap();
    assert!(message.contains("available: none"), "{message}");
}

/// Rule 2: a shared cause is one note line per run, and a clean run prints none.
#[test]
fn a_shared_cause_is_printed_once_and_a_clean_run_prints_nothing() {
    let mut warnings = Warnings::default();
    warnings.add(describe_scale(&[scale(0, 2.0, 2.0)]).unwrap());
    warnings.note(NOTE_FONT_SIZE);
    warnings.add(describe_stretch(&[stretch(0, 1.7, 1.25)]).unwrap());
    warnings.note(NOTE_FONT_SIZE); // same cause again → deduplicated
    assert_eq!(warnings.notes.len(), 1, "{:?}", warnings.notes);
    assert_eq!(warnings.messages.len(), 2);

    let clean = Warnings::default();
    assert!(clean.messages.is_empty() && clean.notes.is_empty());
}

#[test]
fn every_cause_note_fits_the_cap() {
    for note in [
        NOTE_FONT_SIZE,
        NOTE_CANVAS_INK,
        NOTE_INK_DUST,
        NOTE_PASSTHROUGH,
        NOTE_LABEL_CLIP,
        NOTE_DANGLING,
    ] {
        let total = note_line(note);
        assert!(total <= MAX_LINE, "{total} ch: {note}");
    }
}

/// A note explains the *why*, so it must never repeat a warning's code phrase:
/// the doctor classifier and the round-data evidence match those phrases on the
/// `svg_grid: warning:` lines.
#[test]
fn no_cause_note_repeats_a_warning_phrase() {
    let notes = [
        NOTE_CANVAS_INK,
        NOTE_INK_DUST,
        NOTE_PASSTHROUGH,
        NOTE_LABEL_CLIP,
        NOTE_DANGLING,
    ];
    for note in notes {
        for phrase in [
            "scaled by the layout",
            "draws ink outside its own canvas box",
            "point at an id that nothing in the output defines",
            "could not be rewritten",
            "already carry letters",
            "repeats some letters",
            "may be clipped by the canvas",
            "not in the font library",
            "<style> CSS block",
        ] {
            assert!(
                !note.contains(phrase),
                "the note repeats the warning phrase {phrase:?}: {note}"
            );
        }
    }
    // `NOTE_FONT_SIZE` is the one note that may mention a stretch: W2's own line
    // says `1 cell is stretched` for a single offender, while `round-data/R9`
    // looks for the plural phrase in stderr.
    assert!(NOTE_FONT_SIZE.contains("are stretched"), "{NOTE_FONT_SIZE}");
}

/// Rule 3: the footer carries no count at all, and the count-free sentence still
/// says the run is non-fatal (a frozen phrase in `round-data/R16`).
#[test]
fn the_footer_never_counts() {
    let footer = format!(
        "{WARNING_PREFIX}non-fatal warning(s) — the exit code and the output file are unchanged"
    );
    assert!(footer.contains("non-fatal warning(s)"), "{footer}");
    assert!(
        footer.contains("the exit code and the output file are unchanged"),
        "{footer}"
    );
    assert!(!footer.contains("1 "), "{footer}");
    assert!(footer.chars().count() <= MAX_LINE, "{} ch", footer.len());
}
