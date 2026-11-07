//! The panel-letter warnings: W3 (re-lettering an already-lettered composite),
//! W4a (a `--labels` count mismatch), W4b (duplicate letters) and W5 (a panel
//! letter clipped by the canvas).
//!
//! W5's *why* is the shared cause note [`super::NOTE_LABEL_CLIP`]; its own line
//! only names the cell, the ink box and the fix.

use crate::geom::Rect;

use super::{assemble, fit};

/// Warn when an input already carries panel letters but this call would letter it
/// again (the classic nesting mistake: passing `--labels` while assembling a
/// composite/row block).
///
/// The subject is deliberately `panels in input #N`, so the sentence reads the
/// same for one offender and for many and never needs a `1 input(s)` wrapper.
pub fn describe_existing_labels(indexes: &[usize]) -> Option<String> {
    if indexes.is_empty() {
        return None;
    }
    let detailed: Vec<String> = indexes
        .iter()
        .map(|index| format!("input #{index}"))
        .collect();
    let plain: Vec<String> = indexes.iter().map(|index| format!("#{index}")).collect();
    Some(assemble(
        "panels in ",
        &detailed,
        &plain,
        &[
            " already contain panel letters (data-svg-grid-label) but --labels was given again. \
             Fix: pass --labels only in the call that owns the panels as direct inputs.",
        ],
    ))
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
