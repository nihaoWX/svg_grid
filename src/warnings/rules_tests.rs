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

/// The cap holds for pathological inputs too: a long single name, a library whose
/// own family names are long, and a chain so long that even the clamped echo has
/// to give way. Each case must still name the chain, the library and the fix.
#[test]
fn a_pathological_font_chain_still_fits_the_error_cap() {
    let prefix = "svg_grid: font library error: ".len();
    let short_library = vec!["Arial".to_string(), "Times New Roman".to_string()];
    let long_library: Vec<String> = ["L", "M", "N", "O", "P", "Q"]
        .iter()
        .map(|letter| letter.repeat(100))
        .collect();
    let long_chain = (0..200)
        .map(|index| format!("N{index}"))
        .collect::<Vec<_>>()
        .join(",");

    for (chain, library) in [
        ("X".repeat(200), &short_library),
        ("X".repeat(81), &short_library),
        ("NoSuchFont_xyz".to_string(), &long_library),
        (long_chain.clone(), &short_library),
        (long_chain, &long_library),
    ] {
        let message = describe_missing_font_error(&chain, library)
            .unwrap_or_else(|| panic!("a dead chain must fail: {} ch", chain.len()));
        let total = prefix + message.chars().count();
        assert!(
            total <= MAX_LINE,
            "{total} ch (cap {MAX_LINE}) for a {}-char chain: {message}",
            chain.len()
        );
        assert!(
            message.contains("is not in the font library"),
            "the doctor's W8 phrase must survive: {message}"
        );
        assert!(
            message.contains("available:"),
            "the library must be listed: {message}"
        );
        assert!(
            message.contains("--list-fonts"),
            "the fix must be actionable: {message}"
        );
    }

    // The echo is clamped, not thrown away: a long single name keeps a
    // recognisable prefix instead of collapsing to `"…"`.
    let message = describe_missing_font_error(&"X".repeat(81), &short_library).unwrap();
    assert!(
        message.matches('X').count() >= 20,
        "the chain echo must keep a recognisable prefix: {message}"
    );

    // A long library name is shortened rather than dropped, and the chain echo
    // keeps enough room to stay recognisable.
    let message = describe_missing_font_error("NoSuchFont_xyz", &long_library).unwrap();
    assert!(
        message.contains("NoSuchFont_xyz"),
        "the chain must survive a long library: {message}"
    );
    assert!(
        message.contains('…'),
        "a long family name must be shortened: {message}"
    );
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

/// A one-character chain must not let the library list fall off the line: this is
/// the exact shape that regressed once, because the budget for `available:` was
/// hand-counted one character short and every rendering rung was skipped.
#[test]
fn a_single_character_chain_keeps_the_available_list() {
    let prefix = "svg_grid: font library error: ".len();
    let library: Vec<String> = ["A", "B", "C", "F0", "F1", "F2", "F3"]
        .iter()
        .map(|name| format!("{name}{}", "x".repeat(100)))
        .collect();

    let message = describe_missing_font_error("X", &library).expect("a dead chain must fail");
    let total = prefix + message.chars().count();
    assert!(total <= MAX_LINE, "{total} ch (cap {MAX_LINE}): {message}");
    assert!(
        message.contains("available:"),
        "the library must survive a one-character chain: {message}"
    );
    assert!(
        message.contains("\"X\""),
        "the chain itself must still be named: {message}"
    );
    assert!(
        message.contains(" (available: "),
        "the list must be separated from the phrase by a space: {message}"
    );
}

/// A short chain must keep its whole echo even when the library holds very long
/// family names: the echo is worth more than the tail of the list, so it is never
/// collapsed to `"…"` while it would still fit.
#[test]
fn a_short_echo_is_never_collapsed_by_a_long_library() {
    let prefix = "svg_grid: font library error: ".len();
    let library: Vec<String> = (0..7)
        .map(|index| {
            format!(
                "Noto Sans CJK JP Display Condensed {index}{}",
                "y".repeat(10)
            )
        })
        .collect();

    for chain in ["Noto", "AB", "ABC", "ABCD"] {
        let message = describe_missing_font_error(chain, &library).expect("a dead chain must fail");
        let total = prefix + message.chars().count();
        assert!(total <= MAX_LINE, "{total} ch (cap {MAX_LINE}): {message}");
        assert!(
            message.contains(&format!("\"{chain}\"")),
            "the echo must keep the whole {chain:?}: {message}"
        );
        assert!(
            message.contains("available:"),
            "the library must survive too: {message}"
        );
    }
}

/// `with_clamped_chain` is the single place that rejects an over-long rendering:
/// a `fixed` part that leaves no room for even `"…"` must yield `None` instead of
/// an over-cap line.
#[test]
fn with_clamped_chain_rejects_a_fixed_part_that_leaves_no_room() {
    let fixed = format!(" (available: {}); Fix: --list-fonts.", "z".repeat(200));
    assert!(
        fonts::with_clamped_chain("\"X\"", &fixed).is_none(),
        "an impossible fixed part must be rejected, not truncated silently"
    );
}

/// The echo clamp keeps its arithmetic honest: for every budget that can hold an
/// echo at all, the clamped string is within that budget.
#[test]
fn every_clamped_echo_fits_its_budget() {
    let chains = [
        "\"X\"".to_string(),
        "\"Noto Sans CJK JP Display Condensed Medium Oblique Extra Light\"".to_string(),
        "\"Helvetica\", \"Liberation Sans\", \"DejaVu Sans\"".to_string(),
        format!("\"{}\"", "q".repeat(400)),
        "\"A, B, C, D, E, F, G, H, I, J, K, L\"".to_string(),
    ];
    for chain in &chains {
        for budget in 3..=120 {
            let clamped = fonts::clamp_quoted(chain, budget);
            let width = clamped.chars().count();
            assert!(
                width <= budget,
                "budget {budget} produced {width} chars: {clamped}"
            );
        }
    }
}

/// Below the ellipsis width there is nothing to echo, and `clamp_quoted` says so
/// instead of inventing a shorter marker; `with_clamped_chain` then rejects it.
/// This is the one `budget` where the result is *wider* than asked for, so the
/// caller's guard is load-bearing rather than decorative.
#[test]
fn clamp_quoted_never_shrinks_below_the_ellipsis() {
    for budget in 0..3 {
        assert_eq!(fonts::clamp_quoted("\"Arial\"", budget), "\"…\"");
        assert_eq!(
            fonts::with_clamped_chain("\"Arial\"", &"x".repeat(200)),
            None,
            "a fixed part that leaves {budget} characters must be rejected"
        );
    }
}

/// The last-resort form is the unconditional backstop: whatever the ladder emits,
/// the cap holds, and a body that is already short is returned untouched.
#[test]
fn the_last_resort_cap_truncates_only_when_it_must() {
    let long = format!("{}{}", "a".repeat(300), "b".repeat(100));
    let cut = fonts::truncate_body(&long);
    assert!(
        cut.chars().count() <= MAX_ERROR_BODY,
        "{} ch > {MAX_ERROR_BODY}",
        cut.chars().count()
    );
    assert!(cut.ends_with('…'), "{cut}");

    let short = "short body";
    assert_eq!(fonts::truncate_body(short), short);
}

/// A fuzz over the shapes that used to break the budget: a chain of any length, a
/// library of any length, and family names longer than the whole line. Every
/// failure must stay ≤ 200 characters and must still list the library.
#[test]
fn every_chain_and_library_shape_stays_within_the_cap() {
    let prefix = "svg_grid: font library error: ".len();
    let chains: Vec<String> = vec![
        "X".to_string(),
        "AB".to_string(),
        "思源".to_string(),
        "A".repeat(500),
        (0..60)
            .map(|index| format!("Fam{index}"))
            .collect::<Vec<_>>()
            .join(","),
        (0..30)
            .map(|index| format!("Family Name With Spaces {index}"))
            .collect::<Vec<_>>()
            .join(", "),
    ];
    let libraries: Vec<Vec<String>> = vec![
        vec![],
        vec!["Arial".to_string()],
        vec!["Short".to_string(), "MediumName".to_string()],
        (0..7)
            .map(|index| format!("L{index}{}", "z".repeat(100)))
            .collect(),
        (0..40)
            .map(|index| format!("Fam{index}{}", "q".repeat(60)))
            .collect(),
        vec!["x".repeat(400)],
    ];
    for chain in &chains {
        for library in &libraries {
            let Some(message) = describe_missing_font_error(chain, library) else {
                continue;
            };
            let total = prefix + message.chars().count();
            assert!(total <= MAX_LINE, "{total} ch (cap {MAX_LINE}): {message}");
            assert!(
                message.contains("available:"),
                "the library must always be listed: {message}"
            );
            assert!(message.contains("--list-fonts"), "{message}");
        }
    }
}
