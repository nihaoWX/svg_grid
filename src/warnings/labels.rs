//! The panel-letter diagnostics: W3 (re-lettering an already-lettered composite,
//! now a hard error), W4a (a `--labels` count mismatch), W4b (duplicate letters)
//! and W5 (a panel letter clipped by, or moved inside, the canvas).
//!
//! W5's *why* is the shared cause note [`super::NOTE_LABEL_CLIP`]; its own line
//! only names the cell, the ink box and the fix.

use crate::geom::Rect;

use super::{assemble_within, fit};

/// Build the W3 hard error when an input already carries panel letters but this
/// call would letter it again (the classic nesting mistake: passing `--labels`
/// while assembling a composite/row block).
///
/// W3 used to be a warning; since D5 it is a hard `rc=2` error (`INSTRUCTIONS.md`
/// lists re-lettering under "必要信息"): the letters would be doubled and only the
/// layer that owns the panel can place them correctly.
pub fn describe_existing_labels(offenders: &[(usize, String)]) -> Option<String> {
    if offenders.is_empty() {
        return None;
    }
    let detailed: Vec<String> = offenders
        .iter()
        .map(|(index, letter)| format!("input #{index} (letter {letter:?})"))
        .collect();
    let plain: Vec<String> = offenders
        .iter()
        .map(|(index, letter)| format!("#{index} ({letter:?})"))
        .collect();
    Some(assemble_within(
        "panels in ",
        &detailed,
        &plain,
        &[
            " already carry letters (data-svg-grid-label); lettering again duplicates \
             them. Fix: --labels only at the layer that owns the panels.",
        ],
        super::MAX_ERROR_BODY,
    ))
}

/// Build the W5 diagnostic for a letter that had to be moved down into the
/// canvas because an explicit `--plot-margin` left no room above its cell.
pub fn describe_moved_label(label: &str, index: usize, from: f32, to: f32) -> Option<String> {
    if to <= from + 0.0001 {
        return None;
    }
    let head = format!("panel letter {label:?} (cell {index}) was moved inside the canvas: ");
    let body = format!(
        "baseline {from:.3} is above it, clamped to {to:.3}. \
         Fix: omit --plot-margin to reserve the label band, or enlarge its top."
    );
    Some(fit(&[format!("{head}{body}")]))
}

/// Warn when the number of explicit `--labels` entries does not match the number
/// of inputs (empty slots included).
pub fn describe_label_count(count: usize, inputs: usize) -> Option<String> {
    if count == inputs {
        return None;
    }
    // `1 input` / `3 inputs`: a single input never prints the `1 input(s)` form
    // (`ARCHITECTURE/warnings.md` §1 rule 3).
    let grid = if inputs == 1 {
        "1 input".to_string()
    } else {
        format!("{inputs} inputs")
    };
    Some(format!(
        "--labels has {count} entries but the grid has {grid}: extras are ignored, missing ones stay \
         unlettered. Fix: one entry per input; an empty entry (A,,C) skips a cell."
    ))
}

/// Warn when `--labels` repeats a letter (empty entries are skip slots and are
/// not counted as duplicates).
pub fn describe_label_duplicates(labels: &[String]) -> Option<String> {
    let mut seen: Vec<(&str, Vec<usize>)> = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        if label.is_empty() {
            continue;
        }
        match seen.iter_mut().find(|(name, _)| *name == label.as_str()) {
            Some((_, positions)) => positions.push(index),
            None => seen.push((label.as_str(), vec![index])),
        }
    }
    let duplicates: Vec<String> = seen
        .iter()
        .filter(|(_, positions)| positions.len() > 1)
        .map(|(name, positions)| {
            let positions: Vec<String> = positions.iter().map(usize::to_string).collect();
            format!("{name:?} at cells {}", positions.join(", "))
        })
        .collect();
    if duplicates.is_empty() {
        return None;
    }
    let head = "--labels repeats some letters: ";
    let fix = " Fix: give each panel its own letter, or a bare --labels (AUTO).";
    Some(fit(&[
        format!(
            "{head}{}. Duplicate letters make panels indistinguishable.{fix}",
            super::fold(&duplicates, duplicates.len())
        ),
        format!("{head}{}.{fix}", super::fold(&duplicates, duplicates.len())),
        format!("{head}{}.{fix}", super::fold(&duplicates, 1)),
    ]))
}

/// Build the letter-clip warning (if any) for one cell, given the estimated ink
/// box.
pub fn describe_clipped_label(
    label: &str,
    index: usize,
    ink: Rect,
    canvas_width: f32,
    canvas_height: f32,
) -> Option<String> {
    if ink.x >= 0.0
        && ink.y >= 0.0
        && ink.x + ink.width <= canvas_width
        && ink.y + ink.height <= canvas_height
    {
        return None;
    }
    let head = format!("panel letter {label:?} (cell {index}) may be clipped by the canvas: ");
    let tails = [
        " Fix: enlarge --plot-margin (top >= label size), or omit it so the band is reserved.",
        " Fix: enlarge --plot-margin (top >= label size), or omit it.",
    ];
    let caged = format!(
        "{head}ink x=[{:.3}, {:.3}] y=[{:.3}, {:.3}] vs canvas {canvas_width} x {canvas_height}.",
        ink.x,
        ink.x + ink.width,
        ink.y,
        ink.y + ink.height,
    );
    let plain = format!("{head}its ink leaves the {canvas_width} x {canvas_height} canvas.");
    let mut variants = Vec::new();
    for body in [&caged, &plain] {
        for tail in &tails {
            variants.push(format!("{body}{tail}"));
        }
    }
    Some(fit(&variants))
}

/// One place a panel letter appears in the finished figure (R44).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetterSite {
    /// Input index (0-based), i.e. the cell it lands in.
    pub input: usize,
    /// Cell index the letter belongs to: the input's own
    /// `data-svg-grid-label` value for an inherited letter (the cell *inside*
    /// that input), or the slot this call writes for an explicit `--labels`
    /// entry. `None` when a hand-edited marker is not a number.
    pub cell: Option<usize>,
    /// `Some(marker)` when the letter comes from the input's own
    /// `data-svg-grid-label`; `None` when this call writes it.
    pub inner: Option<String>,
}

impl LetterSite {
    pub(crate) fn inherited(&self) -> bool {
        self.inner.is_some()
    }

    fn place(&self) -> String {
        match self.cell {
            Some(cell) => format!("#{} (cell {})", self.input, cell),
            None => format!("#{}", self.input),
        }
    }
}

/// R44: the figure would carry the same panel letter twice, and at least one of
/// those places is an input's own letter (an explicit `--labels A,A` collision is
/// W4b's warning, not this error).
///
/// The message names the letter and every input/cell pair it lands in. A figure
/// with many colliding sites is folded to "first N + `+M more`" so the omitted
/// count is always explicit; the count-only fallback (`"A" (6 places)`) is used
/// only when even one site per letter does not fit the 200-character line.
pub fn describe_duplicate_panel_letters(
    collisions: &[(String, Vec<LetterSite>)],
) -> Option<String> {
    if collisions.is_empty() {
        return None;
    }
    let head = "duplicate panel letters: ";
    let tail = ". Fix: drop the inputs' data-svg-grid-label markers, or letter each panel once.";
    let max_sites = collisions
        .iter()
        .map(|(_, sites)| sites.len())
        .max()
        .unwrap_or(1);
    let mut variants: Vec<String> = Vec::new();
    // Richest first: list every site of every letter, then fold the site lists
    // one place at a time. `describe_sites` keeps the omitted count explicit.
    for shown in (1..=max_sites).rev() {
        let parts: Vec<String> = collisions
            .iter()
            .map(|(letter, sites)| describe_sites(letter, sites, shown))
            .collect();
        variants.push(format!("{head}{}{tail}", parts.join("; ")));
    }
    // Still too long: fold whole letters, but keep each letter's place count.
    let counts: Vec<String> = collisions
        .iter()
        .map(|(letter, sites)| format!("{letter:?} ({} places)", sites.len()))
        .collect();
    for shown in (1..=counts.len()).rev() {
        variants.push(format!("{head}{}{tail}", super::fold(&counts, shown)));
    }
    Some(super::fit_within(&variants, super::MAX_ERROR_BODY))
}

/// One letter's sites, listing at most `shown` distinct places and naming the
/// number of places left out.
fn describe_sites(letter: &str, sites: &[LetterSite], shown: usize) -> String {
    // One input can carry the same letter twice at the same marker value; then
    // there is only one place to name and the count is what explains it.
    let mut places: Vec<String> = Vec::new();
    for site in sites {
        let place = site.place();
        if !places.contains(&place) {
            places.push(place);
        }
    }
    if places.len() == 1 {
        return format!("{letter:?} {} times inside {}", sites.len(), places[0]);
    }
    let listed = places[..shown.min(places.len())].join(" and ");
    if shown < places.len() {
        format!("{letter:?} at {listed}; +{} more", places.len() - shown)
    } else {
        format!("{letter:?} at {listed}")
    }
}
