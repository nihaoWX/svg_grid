//! W7: attributes the composer could not rewrite and therefore kept verbatim.
//!
//! The line names the input, the element/attribute and why it could not be
//! rewritten; *why passing it through is safe* is the shared cause note
//! [`super::NOTE_PASSTHROUGH`], printed once per run.

use super::{assemble, counted};

/// One attribute the composer could not rewrite and therefore left **exactly** as
/// the input wrote it — the offender behind warning W7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passthrough {
    /// Which input (its position in the grid) the attribute belongs to.
    pub index: usize,
    /// The element's tag name (`path`, `circle`, …).
    pub element: String,
    /// The attribute name (`d`, `cx`, …).
    pub attr: String,
    /// Why it could not be rewritten.
    pub detail: String,
}

/// Offenders collected while rewriting the inputs of a single `plot_grid` call.
///
/// W7 speaks **once per call**: offenders are deduplicated by
/// `(index, element, attr, detail)` and the survivors are rendered into a single
/// message. A figure with fifty identically malformed paths therefore still
/// prints one line.
#[derive(Default)]
pub struct Passthroughs {
    offenders: Vec<Passthrough>,
}

impl Passthroughs {
    /// Record that `attr` on `element` in input `index` was kept verbatim.
    pub fn record(&mut self, index: usize, element: &str, attr: &str, detail: String) {
        let offender = Passthrough {
            index,
            element: element.to_string(),
            attr: attr.to_string(),
            detail,
        };
        if !self.offenders.contains(&offender) {
            self.offenders.push(offender);
        }
    }

    pub fn offenders(&self) -> &[Passthrough] {
        &self.offenders
    }
}

/// The offending value a detail string quotes, if it quotes one:
/// `unparseable path data "M 0 0 Lnan" (…)` and `non-finite number "nan"` both
/// yield `Some("…")`.
fn quoted_value(detail: &str) -> Option<&str> {
    let rest = &detail[detail.find('"')? + 1..];
    let end = rest.find('"')?;
    (!rest[..end].is_empty()).then(|| &rest[..end])
}

/// Build the W7 warning for every attribute that was kept verbatim during the
/// call, or `None` when there is nothing to report.
///
/// A malformed `d` (structure, a missing argument, a `nan`/`inf` number, …) and a
/// non-finite numeric attribute (`nan`/`inf`) are *not* fatal: a browser is
/// lenient about both, and rejecting the whole figure would narrow what used to
/// work. The run keeps its exit code (0) and those bytes are unchanged; the point
/// of the warning is only that the composer could not remap the value.
pub fn describe_passthrough(offenders: &[Passthrough]) -> Option<String> {
    if offenders.is_empty() {
        return None;
    }
    let detail: Vec<String> = offenders
        .iter()
        .map(|offender| {
            format!(
                "input #{} <{} {}>: {}",
                offender.index, offender.element, offender.attr, offender.detail
            )
        })
        .collect();
    // The middle tier keeps the offending *value* (it is what tells two offenders
    // of the same element/attribute apart) but drops the explanation; the last
    // tier drops the value too.
    let valued: Vec<String> = offenders
        .iter()
        .map(|offender| match quoted_value(&offender.detail) {
            Some(value) => format!(
                "input #{} <{} {}> \"{value}\"",
                offender.index, offender.element, offender.attr
            ),
            None => format!(
                "input #{} <{} {}>",
                offender.index, offender.element, offender.attr
            ),
        })
        .collect();
    let bare: Vec<String> = offenders
        .iter()
        .map(|offender| {
            format!(
                "input #{} <{} {}>",
                offender.index, offender.element, offender.attr
            )
        })
        .collect();
    // `kept verbatim` is said once, in the head — never once per offender.
    let head = format!(
        "{} could not be rewritten, kept verbatim: ",
        counted(offenders.len(), "attribute", "attributes")
    );
    // Three levels of detail — the full reason, the offending value alone, then the
    // bare input/element/attribute — and two fixes, tried richest first. Every tier
    // lists every offender (folded only at `MAX_LISTED`).
    let tails = [
        ". Fix: correct the value at the plotting end (or let the converter normalise it).",
        ". Fix: correct the value at the plotting end.",
    ];
    for items in [&detail, &valued, &bare] {
        let body = super::fold(items, items.len().min(super::MAX_LISTED));
        for tail in &tails {
            let candidate = format!("{head}{body}{tail}");
            if candidate.chars().count() <= super::MAX_BODY {
                return Some(candidate);
            }
        }
    }
    Some(assemble(&head, &bare, &bare, &tails[1..]))
}
