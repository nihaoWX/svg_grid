//! The warning-code table and the stderr classifier `doctor` uses.
//!
//! `ARCHITECTURE/warnings.md` §3 fixes the *ids* (`W1`…`W9`, `V1`) as the stable
//! interface; the phrases below are how a printed line is recognised as that id.
//! They therefore live next to the messages themselves, not in `doctor`: a
//! reworded message must break a test in this module
//! (`every_warning_code_still_matches_its_message`), not silently stop being
//! classified.
//!
//! Two rules keep the table honest:
//!
//! * **one code, several phrases** — a code may be printed in more than one shape
//!   (W5 is either "was moved inside the canvas" or "may be clipped by the
//!   canvas"), so the table maps a code to *all* of its phrases;
//! * **only `svg_grid: warning:` lines are classified** — W3 and W8 are *hard
//!   errors* (`rc=2` / `rc=3`, `ARCHITECTURE/warnings.md` §3), judged by `doctor`
//!   through the exit code, so they are deliberately **not** in this table. A
//!   table entry that can never match would read as "covered" while covering
//!   nothing (that is exactly the drift R45 fixed).

/// Warning codes recognised on stderr, each with every phrase its message uses.
///
/// The ids are the stable interface of `ARCHITECTURE/warnings.md` §3; the phrases
/// are the ones the messages themselves use, so a reworded message is caught by
/// `every_warning_code_still_matches_its_message` in `codes_tests`.
pub const WARNING_CODES: &[(&str, &[&str])] = &[
    ("W1", &["scaled by the layout"]),
    ("W2", &["stretched: "]),
    ("W4a", &["--labels has "]),
    ("W4b", &["repeats some letters"]),
    (
        "W5",
        &[
            "was moved inside the canvas",
            "may be clipped by the canvas",
        ],
    ),
    ("W6", &["draws ink outside its own canvas box"]),
    ("W7", &["could not be rewritten, kept verbatim"]),
    ("W9", &["point at an id that nothing in the output defines"]),
    ("V1", &["<style> CSS block"]),
];

/// Split stderr into (warning codes, warning count).
///
/// Only `svg_grid: warning:` / `svg_grid convert: warning:` lines are considered
/// (hard errors carry their own prefix and are judged by exit code). The uniform
/// tally line (`N non-fatal warning(s) — …`) is a footer, not a warning, so it is
/// neither counted nor classified.
pub fn classify(stderr: &str) -> (Vec<&'static str>, usize) {
    let mut codes: Vec<&'static str> = Vec::new();
    let mut warnings = 0;
    for line in stderr.lines() {
        if !is_warning_line(line) {
            continue;
        }
        warnings += 1;
        if let Some((code, _)) = WARNING_CODES
            .iter()
            .find(|(_, phrases)| phrases.iter().any(|phrase| line.contains(phrase)))
        {
            if !codes.contains(code) {
                codes.push(code);
            }
        }
    }
    (codes, warnings)
}

fn is_warning_line(line: &str) -> bool {
    (line.starts_with("svg_grid: warning:") || line.starts_with("svg_grid convert: warning:"))
        && !line.contains("non-fatal warning(s)")
}
