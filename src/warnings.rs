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
//! *what to do about it*.

use crate::geom::Rect;

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

/// How many offending inputs/letters a single grouped warning lists before it
/// collapses the rest into `+N more`.
const MAX_LISTED: usize = 6;

/// Warnings collected during a single `plot_grid` call. Emitting them together
/// keeps a run with many offenders to a handful of lines instead of one line per
/// offender.
#[derive(Default)]
pub struct Warnings {
    messages: Vec<String>,
}

impl Warnings {
    pub fn add(&mut self, message: String) {
        self.messages.push(message);
    }

    /// Print every warning to stderr with the uniform `svg_grid: warning:` prefix,
    /// each on its own line, then a one-line tally. The exit code is never
    /// affected and no output byte changes.
    pub fn emit(&self) {
        for message in &self.messages {
            eprintln!("svg_grid: warning: {message}");
        }
        if !self.messages.is_empty() {
            let count = self.messages.len();
            eprintln!(
                "svg_grid: warning: {count} non-fatal warning(s) — the exit code and the output file are unchanged"
            );
        }
    }
}

/// One input's resolved layout scale: `sx = cell.width / canvas.width`,
/// `sy = cell.height / canvas.height`.
#[derive(Debug, Clone, Copy)]
pub struct InputScale {
    pub index: usize,
    pub sx: f32,
    pub sy: f32,
}

/// One drawn cell's aspect ratio compared with its panel's canvas aspect ratio.
#[derive(Debug, Clone, Copy)]
pub struct CellStretch {
    pub index: usize,
    pub cell_aspect: f32,
    pub panel_aspect: f32,
}

/// Warn when the layout rescales an input away from 1.
///
/// All offending inputs are collapsed into a single message so a wide grid does
/// not flood stderr.
pub fn describe_scale(scales: &[InputScale]) -> Option<String> {
    let offenders: Vec<String> = scales
        .iter()
        .filter(|scale| {
            (scale.sx - 1.0).abs() > SCALE_TOLERANCE || (scale.sy - 1.0).abs() > SCALE_TOLERANCE
        })
        .map(|scale| {
            let deviation = (scale.sx - 1.0).abs().max((scale.sy - 1.0).abs());
            format!(
                "#{} sx={:.5} sy={:.5} (off by {:.2}%)",
                scale.index,
                scale.sx,
                scale.sy,
                deviation * 100.0
            )
        })
        .collect();
    if offenders.is_empty() {
        return None;
    }
    Some(format!(
        "{} input(s) are scaled by the layout (|sx-1| or |sy-1| > {:.1}%): {}. The tag protocol \
         never rescales font-size, so a panel that is not drawn at its cell size keeps its text \
         at the wrong size relative to its geometry. To keep the scale ≈1: draw the panel at its \
         target cell size (--normalize-input-max-side <cell width>), give --plot-margin 0 on the \
         outer assembly, or omit --rel-* so each cell takes its panel's canvas aspect.",
        offenders.len(),
        SCALE_TOLERANCE * 100.0,
        list_items(&offenders),
    ))
}

/// Warn when a cell's aspect ratio does not match its panel's, i.e. the panel is
/// stretched.
pub fn describe_stretch(cells: &[CellStretch]) -> Option<String> {
    let offenders: Vec<String> = cells
        .iter()
        .filter_map(|cell| {
            if cell.panel_aspect <= 0.0 || cell.cell_aspect <= 0.0 {
                return None;
            }
            let stretch = cell.cell_aspect / cell.panel_aspect - 1.0;
            (stretch.abs() > ASPECT_TOLERANCE).then(|| {
                format!(
                    "#{} cell aspect {:.3} vs panel {:.3} (stretched {:+.1}%)",
                    cell.index,
                    cell.cell_aspect,
                    cell.panel_aspect,
                    stretch * 100.0
                )
            })
        })
        .collect();
    if offenders.is_empty() {
        return None;
    }
    Some(format!(
        "{} cell(s) are stretched: the cell's aspect ratio differs from its panel canvas aspect \
         by more than {:.0}%: {}. A stretched cell distorts the panel and, because font-size is \
         never rescaled, detaches its text from its geometry. Fix: omit --rel-* so the layout \
         allocates each cell from its panel's aspect ratio, or give --rel-widths/--rel-heights \
         that match the panels.",
        offenders.len(),
        ASPECT_TOLERANCE * 100.0,
        list_items(&offenders),
    ))
}

/// Warn when an input already carries panel letters but this call would letter it
/// again (the classic nesting mistake: passing `--labels` while assembling a
/// composite/row block).
pub fn describe_existing_labels(indexes: &[usize]) -> Option<String> {
    if indexes.is_empty() {
        return None;
    }
    let listed: Vec<String> = indexes.iter().map(|index| format!("#{index}")).collect();
    Some(format!(
        "{} input(s) already contain panel letters (data-svg-grid-label) but --labels was given \
         again: {}. Re-lettering a composite figure renumbers its panels and shifts the existing \
         letters (duplicates / wrong positions). Add letters only in the call that owns the panels \
         as direct inputs; never pass --labels when assembling a composite / row block.",
        indexes.len(),
        list_items(&listed),
    ))
}

/// Warn when the number of explicit `--labels` entries does not match the number
/// of inputs (empty slots included).
pub fn describe_label_count(count: usize, inputs: usize) -> Option<String> {
    if count == inputs {
        return None;
    }
    Some(format!(
        "--labels has {count} entries but the grid has {inputs} input(s): extra labels are \
         ignored and missing ones leave their panel unlettered. Give exactly one entry per input; \
         use an empty entry (e.g. --labels A,,C) to deliberately skip a cell."
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
    Some(format!(
        "--labels repeats some letters: {}. Duplicate letters make panels indistinguishable; give \
         each panel a unique letter, or use a bare --labels (AUTO) to number the drawn panels \
         A, B, C ... automatically.",
        list_items(&duplicates),
    ))
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
    Some(format!(
        "panel letter {label:?} (cell {index}) may be clipped by the canvas: its ink box \
         x=[{:.3}, {:.3}] y=[{:.3}, {:.3}] falls outside the canvas [0, {canvas_width}] x \
         [0, {canvas_height}]. Fix: enlarge --plot-margin (top ≥ the label size), or omit \
         --plot-margin so the letter band is reserved automatically.",
        ink.x,
        ink.x + ink.width,
        ink.y,
        ink.y + ink.height,
    ))
}

fn list_items(items: &[String]) -> String {
    if items.len() <= MAX_LISTED {
        items.join("; ")
    } else {
        format!(
            "{}; +{} more",
            items[..MAX_LISTED].join("; "),
            items.len() - MAX_LISTED
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale(index: usize, sx: f32, sy: f32) -> InputScale {
        InputScale { index, sx, sy }
    }

    #[test]
    fn scale_warning_fires_for_the_099032_outer_margin_bug() {
        // (W - 2*24)/W at W = 4961 is exactly the silent ~1 % shrink this warning
        // exists for; a 1 % tolerance would miss it.
        let message = describe_scale(&[scale(0, 4913.0 / 4961.0, 4913.0 / 4961.0)])
            .expect("0.99032 must be reported");
        assert!(message.contains("0.99032"), "{message}");
        assert!(message.contains("font-size"), "{message}");
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
        assert_eq!(message.matches("input(s) are scaled").count(), 1);
        assert!(
            message.contains("#0") && message.contains("#1"),
            "{message}"
        );
    }

    fn stretch(index: usize, cell_aspect: f32, panel_aspect: f32) -> CellStretch {
        CellStretch {
            index,
            cell_aspect,
            panel_aspect,
        }
    }

    #[test]
    fn stretch_warning_fires_and_reports_the_percent() {
        let message = describe_stretch(&[stretch(0, 1.738, 1.25)]).expect("stretch must be shown");
        assert!(message.contains("stretched +39.0%"), "{message}");
        assert!(message.contains("--rel-*"), "{message}");
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
        let message = describe_existing_labels(&[0, 3]).unwrap();
        assert!(message.contains("data-svg-grid-label"), "{message}");
        assert!(
            message.contains("#0") && message.contains("#3"),
            "{message}"
        );
    }

    #[test]
    fn label_count_warning_only_on_mismatch() {
        assert!(describe_label_count(3, 3).is_none());
        let message = describe_label_count(2, 3).unwrap();
        assert!(
            message.contains("2 entries") && message.contains("3 input(s)"),
            "{message}"
        );
    }

    #[test]
    fn label_duplicate_warning_ignores_empty_slots() {
        let owned = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(describe_label_duplicates(&owned(&["A", "B", "C"])).is_none());
        assert!(describe_label_duplicates(&owned(&["A", "", "C"])).is_none());
        let message = describe_label_duplicates(&owned(&["A", "", "A"])).unwrap();
        assert!(message.contains("\"A\""), "{message}");
        assert!(message.contains("0, 2"), "{message}");
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
        assert!(message.contains("clipped"), "{message}");
    }
}
