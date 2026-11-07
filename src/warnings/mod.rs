//! Non-fatal warnings for `plot_grid`.
//!
//! The composer must never *silently* produce a result that looks right but is
//! quietly wrong — that is the failure mode this tool exists to avoid. When the
//! layout rescales an input, stretches a cell away from its panel's aspect
//! ratio, re-letters an already-lettered composite, or receives inconsistent
//! labels, the run still succeeds (exit code 0) and the output file is still
//! written byte-for-byte as before; a warning is printed to stderr so an agent
//! driving the tool can notice and fix the invocation.
//!
//! Every message answers three questions: *what happened*, *why it matters*, and
//! *what to do about it* — but the *why* is never repeated from line to line. A
//! cause that several messages share is lifted into one `svg_grid: note:` line,
//! printed at most once per run (see [`NOTE_FONT_SIZE`] and friends); each
//! warning then carries only *what happened*, *which input*, and the `Fix:`.
//! A run with no warnings prints neither a warning nor a note (zero bytes of
//! stderr — that is a regression item).
//!
//! The presentation rules live in `ARCHITECTURE/warnings.md` §1:
//!
//! 1. one printed line is at most [`MAX_LINE`] characters, prefix included;
//! 2. a shared cause is one note per run, and it disappears with the warnings;
//! 3. a single offender is never wrapped in a count (`1 input`, not `1 input(s)`).
//!
//! ## Layout
//!
//! The warning groups live in sibling modules; this file keeps the module
//! documentation, the shared tolerances, the [`Warnings`] collector and the
//! formatting helpers, and re-exports every item so the rest of the crate keeps
//! calling `crate::warnings::…` unchanged.
//!
//! * [`scale`] — W1 ([`describe_scale`]) and W2 ([`describe_stretch`]).
//! * [`labels`] — W3 ([`describe_existing_labels`]), W4a
//!   ([`describe_label_count`]), W4b ([`describe_label_duplicates`]) and W5
//!   ([`describe_clipped_label`]).
//! * [`content`] — W6 ([`describe_content_outside_canvas`]).
//! * [`passthrough`] — W7 ([`Passthroughs`] / [`describe_passthrough`]).
//! * [`fonts`] — W8 ([`describe_missing_font`]).
//! * [`refs`] — W9 ([`dangling_references`] / [`describe_dangling_reference`]).

mod content;
mod fonts;
mod labels;
mod passthrough;
mod refs;
mod scale;

#[cfg(test)]
mod rules_tests;
#[cfg(test)]
mod tests;

pub use content::*;
pub use fonts::*;
pub use labels::*;
pub use passthrough::*;
pub use refs::*;
pub use scale::*;

/// Relative tolerance for "this input was rescaled by the layout"
/// (`|sx - 1|` / `|sy - 1|`).
///
/// 0.5 %: the canonical silent bug — an outer assembly that forgets
/// `--plot-margin 0` — shrinks each row block by `(W - 2·24) / W`, which at
/// `W = 4961` is `0.99032` (a **0.97 %** deviation). A 1 % tolerance would miss
/// exactly the case this warning exists for, so the threshold is 0.5 %. The S22
/// reference pipeline draws every panel at its cell size and is exact to 1e-4,
/// so it never trips this.
pub const SCALE_TOLERANCE: f32 = 0.005;

/// Relative tolerance for "this cell's aspect ratio differs from its panel's".
/// A stretch of more than 1 % is where distortion becomes visible.
pub const ASPECT_TOLERANCE: f32 = 0.01;

/// Tolerance (in canvas units / px) for W6: ink this far outside the input's own
/// canvas box is treated as *inside*. The text run is only a proportional
/// estimate, so a small overshoot (a fraction of a px) is expected and must not
/// be reported.
pub const CONTENT_OUTSIDE_TOLERANCE: f64 = 1.0;

/// "Dust" threshold (px) for the W6 self-priority hint: when *every* reported
/// overflow is below this, the message is flagged with [`DUST_HINT`] and the run
/// also prints [`NOTE_INK_DUST`]. The verdict itself is **not** changed — the
/// 1 px tolerance keeps any real overflow reported (a 1.5 px overshoot is still
/// an overflow).
pub const CONTENT_OUTSIDE_DUST: f64 = 2.0;

/// Flag appended to W6 when every reported overflow is below
/// [`CONTENT_OUTSIDE_DUST`]. What it *means* is [`NOTE_INK_DUST`], printed once.
pub(crate) const DUST_HINT: &str = " (all overflows < 2 px)";

/// How many offending inputs/letters a single grouped warning lists before it
/// collapses the rest into `+N more`.
const MAX_LISTED: usize = 6;

/// Prefix of every warning line. [`MAX_LINE`] includes it.
pub const WARNING_PREFIX: &str = "svg_grid: warning: ";

/// Prefix of a per-run *cause note*: one line, printed at most once per run.
pub const NOTE_PREFIX: &str = "svg_grid: note: ";

/// Hard cap on one printed stderr line, **prefix included**
/// (`ARCHITECTURE/warnings.md` §1 rule 1). Measured with `awk length`.
pub const MAX_LINE: usize = 200;

/// What is left for a warning body once the prefix is printed.
pub(crate) const MAX_BODY: usize = MAX_LINE - WARNING_PREFIX.len();

/// What is left for a note body once the note prefix is printed.
pub(crate) const MAX_NOTE: usize = MAX_LINE - NOTE_PREFIX.len();

/// Cause shared by W1 and W2: the tag protocol is position-only, so `font-size`
/// is never rescaled. Printed once per run; W1/W2 themselves only report numbers.
pub const NOTE_FONT_SIZE: &str =
    "font-size is never rescaled (the tag protocol is position-only), so \
     text keeps its source size while its geometry moves; cells past a 1 % aspect change are \
     stretched.";

/// Cause behind W6: what an ink overflow past the canvas edge actually does.
pub const NOTE_CANVAS_INK: &str = "ink past the canvas edge is DROPPED by the panel's own raster \
     export (its PNG); in a composite a positive margin there shows it flush against the finished \
     edge.";

/// W6's self-priority caveat, printed only when every overflow is "dust".
pub const NOTE_INK_DUST: &str =
    "every overflow above is under 2 px — a margin normally absorbs that, so it is informational only.";

/// Cause behind W7: why a value that cannot be rewritten is passed through.
pub const NOTE_PASSTHROUGH: &str = "a browser is lenient about a malformed or non-finite value, so \
     it is passed through rather than rejected; it was not remapped, though, so its ink may land in \
     the wrong place.";

/// Cause behind W8: why a font fallback is not merely cosmetic.
pub const NOTE_FONT_FALLBACK: &str =
    "a missing family changes glyph widths and line breaks, so the \
     letters no longer match the figure the layout was measured against.";

/// Cause behind W5: what a clipped letter looks like in the output.
pub const NOTE_LABEL_CLIP: &str =
    "a letter whose ink leaves the canvas is cut at the edge; the band \
     reserved when --plot-margin is omitted is always large enough for it.";

/// Cause behind W9: what a dangling `url(#…)` costs, and which `<defs>` elements
/// can define the id.
pub const NOTE_DANGLING: &str = "a dropped paint server (<linearGradient>, <radialGradient> or \
     <pattern>) or <clipPath> paints its shapes with nothing, so the ink is gone while the figure \
     still looks complete.";

/// Warnings and cause notes collected during a single `plot_grid` call. Emitting
/// them together keeps a run with many offenders to a handful of lines instead of
/// one line per offender.
#[derive(Default)]
pub struct Warnings {
    messages: Vec<String>,
    notes: Vec<String>,
}

impl Warnings {
    pub fn add(&mut self, message: String) {
        self.messages.push(message);
    }

    /// Register a cause note (one of the `NOTE_*` constants). It is printed at
    /// most **once per run**, only when the run has at least one warning — a
    /// clean run stays byte-for-byte silent.
    pub fn note(&mut self, note: &str) {
        debug_assert!(
            note.chars().count() <= MAX_NOTE,
            "a note must fit {MAX_LINE} characters: {} ch",
            note.chars().count()
        );
        if !self.notes.iter().any(|existing| existing == note) {
            self.notes.push(note.to_string());
        }
    }

    /// Print every warning to stderr with the uniform `svg_grid: warning:` prefix,
    /// each on its own line, then the cause notes (one line each), then the
    /// uniform footer. The exit code is never affected and no output byte changes.
    ///
    /// The footer carries no count: the number of warnings is the number of lines
    /// above, and a one-warning run must not print `1 non-fatal warning(s)`
    /// (`ARCHITECTURE/warnings.md` §1 rule 3).
    pub fn emit(&self) {
        if self.messages.is_empty() {
            return;
        }
        for message in &self.messages {
            eprintln!("{WARNING_PREFIX}{message}");
        }
        for note in &self.notes {
            eprintln!("{NOTE_PREFIX}{note}");
        }
        eprintln!(
            "{WARNING_PREFIX}non-fatal warning(s) — the exit code and the output file are unchanged"
        );
    }
}

/// `1 input` / `3 inputs`: a single offender is never written as `1 input(s)`
/// (`ARCHITECTURE/warnings.md` §1 rule 3). `singular`/`plural` include the verb
/// when the count is the subject of a sentence (`"input is"` / `"inputs are"`).
pub(crate) fn counted(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// The offender list, with the tail folded into `+N more` once `shown` entries
/// are printed (`shown == 0` keeps the count only).
pub(crate) fn fold(items: &[String], shown: usize) -> String {
    let mut list = items[..shown.min(items.len())].join("; ");
    if shown < items.len() {
        if !list.is_empty() {
            list.push_str("; ");
        }
        list.push_str(&format!("+{} more", items.len() - shown));
    }
    list
}

/// Compose the richest warning body that fits [`MAX_BODY`].
///
/// `detailed` and `plain` are the same offenders at two levels of detail, and
/// `tails` is the closing clause (a `Fix:` first, then shorter fallbacks). The
/// richest rendering that still lists every offender is tried first; the list is
/// folded into `+N more` only when no rendering fits. The last candidate is
/// returned as a last resort, so a pathological input can never break a line —
/// only at worst exceed the cap by its own tail.
pub(crate) fn assemble(
    head: &str,
    detailed: &[String],
    plain: &[String],
    tails: &[&str],
) -> String {
    for shown in (0..=detailed.len()).rev() {
        for items in [detailed, plain] {
            for tail in tails {
                let body = format!("{head}{}{tail}", fold(items, shown));
                if body.chars().count() <= MAX_BODY {
                    return body;
                }
            }
        }
    }
    format!(
        "{head}{}{}",
        fold(plain, 0),
        tails.last().copied().unwrap_or_default()
    )
}

/// Return the first candidate that fits [`MAX_BODY`], else the shortest one.
pub(crate) fn fit(variants: &[String]) -> String {
    if let Some(fitting) = variants
        .iter()
        .find(|candidate| candidate.chars().count() <= MAX_BODY)
    {
        return fitting.clone();
    }
    variants
        .iter()
        .min_by_key(|candidate| candidate.chars().count())
        .cloned()
        .unwrap_or_default()
}
