//! The geometry half of a `plot-grid` call: read the inputs, resolve the grid, and
//! report what the composition would do — **without writing a file**
//! (`ARCHITECTURE/cli.md` §`--plan`, `ARCHITECTURE/layout.md` §格子尺寸由工具算).
//!
//! [`plot_grid`](crate::plot_grid) renders what [`prepare`] returns and [`plan_grid`]
//! only reports it: both go through the same `prepare`, and the scale they talk about
//! is [`geom::cell_scale`] — that is what keeps "the plan says s = 1.5" and "the
//! figure really scaled that panel by 1.5" one statement instead of two.

use std::path::Path;

use svg_grid_convert::{Conversion, ConvertError, Finding};

use crate::dom;
use crate::error::{Result, SvgGridError};
use crate::geom;
use crate::input;
use crate::layout;
use crate::normalize;
use crate::warnings;
use crate::{InputNormalize, LabelSpec, Margins, SvgInput};

/// A parsed input SVG together with the geometry the composer needs from it.
pub(crate) struct PreparedInput {
    pub(crate) root: dom::Element,
    pub(crate) panel: geom::Rect,
    pub(crate) canvas: geom::Rect,
    /// `Some` when the source had no panel box and was converted in memory; the
    /// converter's non-fatal warnings travel with it so the CLI can emit them
    /// through the same `--verbose` channel as W1–W9.
    pub(crate) auto_converted: Option<Vec<String>>,
}

/// The layout half of a `plot-grid` call: everything that decides the geometry, and
/// **nothing that writes**.
///
/// [`plot_grid`] builds one of these from its [`PlotGridSpec`] and hands it to
/// [`prepare`]; [`plan_grid`] takes one directly. That is what makes `--plan` and the
/// real run resolve the same cells and the same `sx`/`sy` — there is no second copy
/// of the layout arithmetic (`ARCHITECTURE/layout.md` §格子尺寸由工具算).
#[derive(Debug, Clone)]
pub struct GridRequest {
    pub inputs: Vec<SvgInput>,
    pub width: f32,
    /// `None` → derive the canvas height from the panels' natural heights.
    pub height: Option<f32>,
    pub nrow: Option<usize>,
    pub ncol: Option<usize>,
    pub rel_widths: Vec<f32>,
    pub rel_heights: Vec<f32>,
    pub margin: Margins,
    pub gap: f32,
    pub normalize: Option<InputNormalize>,
}

/// The resolved, side-effect-free layout of a `plot-grid` call — what
/// `plot-grid --plan` prints.
///
/// Every number here comes from [`prepare`], the same function the real run uses, so
/// the plan cannot drift from the figure it describes.
#[derive(Debug, Clone)]
pub struct GridPlan {
    /// The output SVG's `width`, and thus the PNG pixel width.
    pub width: f32,
    /// The output SVG's `height` (derived when `--height` is omitted).
    pub height: f32,
    /// One entry per input, in input order.
    pub inputs: Vec<PlannedInput>,
}

/// One input's resolved place in the layout.
#[derive(Debug, Clone)]
pub struct PlannedInput {
    pub index: usize,
    /// The input itself: a path or a blank [`SvgInput::Spacer`].
    pub input: SvgInput,
    /// The input's own canvas (`width`, `height`) **after** `--normalize-input-max-side`;
    /// `None` = a blank cell, which has no canvas.
    pub canvas: Option<(f32, f32)>,
    /// The cell it lands in (`width`, `height`).
    pub cell: (f32, f32),
    /// `sx`, `sy` exactly as [`geom::scale_panel_to_cell`] applies them; `None` = blank.
    pub scale: Option<(f32, f32)>,
}

impl PlannedInput {
    /// `sx / sy − 1`: the non-uniform part of the scale. `0` means the panel keeps its
    /// aspect ratio (circles stay circles); anything else is a stretch.
    pub fn distortion(&self) -> Option<f32> {
        self.scale
            .map(|(sx, sy)| if sy == 0.0 { 0.0 } else { sx / sy - 1.0 })
    }

    /// Whether this input has to be re-emitted at its cell size first — judged by
    /// exactly [`warnings::SCALE_TOLERANCE`], the threshold W1 uses, so the plan and
    /// the run never disagree about "is this input at its cell size".
    pub fn needs_normalize(&self) -> bool {
        match self.scale {
            Some((sx, sy)) => {
                (sx - 1.0).abs() > warnings::SCALE_TOLERANCE
                    || (sy - 1.0).abs() > warnings::SCALE_TOLERANCE
            }
            None => false,
        }
    }

    /// Whether this input is stretched: the cell's aspect ratio differs from the
    /// panel's by more than [`warnings::ASPECT_TOLERANCE`] — W2's own threshold.
    pub fn is_stretched(&self) -> bool {
        self.distortion()
            .is_some_and(|distortion| distortion.abs() > warnings::ASPECT_TOLERANCE)
    }

    /// The `--normalize-input-max-side` target for this input: the long side of its
    /// cell. `None` for a blank cell.
    pub fn normalize_target(&self) -> Option<f32> {
        self.canvas.is_some().then(|| self.cell.0.max(self.cell.1))
    }
}

/// Read every input, apply `--normalize-input-max-side`, and resolve the grid.
///
/// This is the whole geometry of a `plot-grid` call, and it touches nothing but its
/// inputs' files: [`plot_grid`] renders what it returns, [`plan_grid`] reports it.
pub(crate) fn prepare(
    request: &GridRequest,
) -> Result<(Vec<Option<PreparedInput>>, layout::GridLayout)> {
    if request.inputs.is_empty() {
        return Err(SvgGridError::InvalidInput(
            "plot_grid requires at least one input SVG".to_string(),
        ));
    }

    // Every input is parsed up front: the layout reads each panel's canvas
    // aspect ratio to derive natural column widths and row heights.
    let mut prepared: Vec<Option<PreparedInput>> = Vec::with_capacity(request.inputs.len());
    let mut aspects: Vec<Option<f32>> = Vec::with_capacity(request.inputs.len());
    for input in &request.inputs {
        let input_path = match input {
            SvgInput::Path(path) => path,
            SvgInput::Spacer => {
                prepared.push(None);
                aspects.push(None);
                continue;
            }
        };
        let svg = input::read_svg(input_path)?;
        let mut root = dom::parse(&svg)?;
        // Untagged inputs (an R/matplotlib SVG straight from the graphics device)
        // are converted **in memory** first. The source file is only ever read:
        // `convert` takes the text and returns text, so there is no temp file and
        // no write-back. An input that already carries a panel box skips this
        // branch entirely, which is what keeps composites and doctor's second
        // stage byte-identical.
        let auto_converted = if geom::find_panel_box_opt(&root)?.is_none() {
            let (converted_svg, converted_warnings) = auto_convert(input_path, &svg)?;
            root = dom::parse(&converted_svg).map_err(|error| {
                SvgGridError::InvalidInput(format!(
                    "auto-conversion of {} produced an unparseable SVG: {error}",
                    input_path.display()
                ))
            })?;
            if geom::find_panel_box_opt(&root)?.is_none() {
                return Err(SvgGridError::InvalidInput(format!(
                    "auto-conversion of {} produced no <rect data-panel-box=\"main\" .../> \
                     element; refusing to compose a panel whose geometry cannot be located",
                    input_path.display()
                )));
            }
            Some(converted_warnings)
        } else {
            None
        };
        if let Some(normalize) = &request.normalize {
            normalize::normalize_svg(&mut root, normalize.max_side)?;
        }
        let panel = geom::find_panel_box(&root)?;
        let canvas = geom::find_canvas_box(&root)?;
        if canvas.width <= 0.0 || canvas.height <= 0.0 {
            return Err(SvgGridError::InvalidInput(
                "input SVG canvas must have positive width and height".to_string(),
            ));
        }
        aspects.push(Some(canvas.width / canvas.height));
        prepared.push(Some(PreparedInput {
            root,
            panel,
            canvas,
            auto_converted,
        }));
    }

    let grid = layout::compute_grid(layout::GridSpec {
        count: request.inputs.len(),
        nrow: request.nrow,
        ncol: request.ncol,
        width: request.width,
        height: request.height,
        margin: request.margin,
        gap: request.gap,
        rel_widths: &request.rel_widths,
        rel_heights: &request.rel_heights,
        aspects: &aspects,
    })?;
    Ok((prepared, grid))
}

/// The layout this call would produce, with **no file written and nothing
/// rasterised** (`svg_grid plot-grid --plan`).
///
/// It runs [`prepare`] — the same reading, the same `--normalize-input-max-side`, the
/// same cell arithmetic — and only reports the result, so the numbers it prints are
/// the numbers the real run applies.
pub fn plan_grid(request: GridRequest) -> Result<GridPlan> {
    plan_grid_checked(request, None, false)
}

/// [`plan_grid`] with the same explicit diagnostics switch as
/// [`plot_grid_with_diagnostics`](crate::plot_grid_with_diagnostics): when
/// `verbose` is true, the converter warnings from an in-memory auto-conversion
/// are printed to stderr. The **stdout** `--plan` contract is untouched.
pub fn plan_grid_with_diagnostics(request: GridRequest, verbose: bool) -> Result<GridPlan> {
    plan_grid_checked(request, None, verbose)
}

/// [`plan_grid`] that also validates the label request (D5 W3): `--plan` must
/// fail exactly like the real run when it would re-letter an already-lettered
/// composite. The plan itself is built only after the labels pass, so a failing
/// call prints no plan.
pub fn plan_grid_with_labels(
    request: GridRequest,
    labels: Option<&LabelSpec>,
    verbose: bool,
) -> Result<GridPlan> {
    plan_grid_checked(request, labels, verbose)
}

fn plan_grid_checked(
    request: GridRequest,
    labels: Option<&LabelSpec>,
    verbose: bool,
) -> Result<GridPlan> {
    let (prepared, grid) = prepare(&request)?;
    if let Some(labels) = labels {
        crate::validate_label_conflicts(&prepared, labels)?;
    }
    let mut warnings = warnings::Warnings::default();
    collect_auto_conversion_diagnostics(&request.inputs, &prepared, &mut warnings);
    let mut inputs = Vec::with_capacity(request.inputs.len());
    for (index, item) in prepared.iter().enumerate() {
        let cell = grid.cells[index];
        let cell_size = (cell.width, cell.height);
        match item {
            None => inputs.push(PlannedInput {
                index,
                input: SvgInput::Spacer,
                canvas: None,
                cell: cell_size,
                scale: None,
            }),
            Some(item) => {
                let (sx, sy) = geom::cell_scale(cell, item.canvas)?;
                inputs.push(PlannedInput {
                    index,
                    input: request.inputs[index].clone(),
                    canvas: Some((item.canvas.width, item.canvas.height)),
                    cell: cell_size,
                    scale: Some((sx, sy)),
                });
            }
        }
    }
    let plan = GridPlan {
        width: grid.width,
        height: grid.height,
        inputs,
    };
    warnings.emit(verbose);
    Ok(plan)
}

/// Convert one untagged input in memory.
///
/// The source is passed as text and the result is a string: no path is written,
/// so `plot-grid` can never modify (or leave a temporary file next to) the panel
/// the user pointed at. A failure is rephrased as an actionable `InvalidInput`:
/// name the input, show the first few converter findings, and point at the
/// standalone command that prints the full report.
fn auto_convert(input: &Path, source: &str) -> Result<(String, Vec<String>)> {
    match svg_grid_convert::convert(source, false) {
        Ok(Conversion::Converted(converted)) => {
            let converted = *converted;
            Ok((converted.svg, converted.warnings))
        }
        Ok(Conversion::Incompatible(findings)) => {
            Err(auto_conversion_error(input, &describe_findings(&findings)))
        }
        Ok(Conversion::AlreadyConverted { .. }) => Err(SvgGridError::InvalidInput(format!(
            "auto-conversion of {} reported an already-converted input although no \
             <rect data-panel-box=\"main\" .../> element was found; refusing to guess which \
             layer is the panel",
            input.display()
        ))),
        Err(error) => {
            let detail = match error {
                ConvertError::Incompatible(findings) => describe_findings(&findings),
                other => format!("  converter error: {}", clamp(&other.to_string(), 400)),
            };
            Err(auto_conversion_error(input, &detail))
        }
    }
}

fn auto_conversion_error(input: &Path, detail: &str) -> SvgGridError {
    SvgGridError::InvalidInput(format!(
        "auto-conversion failed for {}: the input has no <rect data-panel-box=\"main\" .../> \
         element, so `plot-grid` tried `svg_grid convert` in memory and that failed.\n{detail}\n\
         fix: check that the input is one of the supported dialects, or run `svg_grid convert \
         --input {} --check --report` to read the full compatibility report.",
        input.display(),
        input.display()
    ))
}

fn describe_findings(findings: &[Finding]) -> String {
    let mut text = format!("  converter reported {} problem(s):", findings.len());
    for finding in findings.iter().take(3) {
        let line = format!(
            "  - <{}> at {}: {}",
            finding.tag,
            finding.path,
            clamp(&finding.reason, 180)
        );
        text.push('\n');
        text.push_str(&clamp(&line, 200));
    }
    if findings.len() > 3 {
        text.push_str(&format!("\n  - … and {} more", findings.len() - 3));
    }
    text
}

/// Add the auto-conversion diagnostics to the run's collector.
///
/// One line per converter warning, prefixed with the input it belongs to; when
/// the converter itself was quiet, a single summary line still records that the
/// input was converted, so the companion note is not the only trace under
/// `--verbose`. Messages are clamped to the shared stderr line budget.
pub(crate) fn collect_auto_conversion_diagnostics(
    inputs: &[SvgInput],
    prepared: &[Option<PreparedInput>],
    warnings: &mut warnings::Warnings,
) {
    let mut any = false;
    for (index, item) in prepared.iter().enumerate() {
        let Some(PreparedInput {
            auto_converted: Some(converted_warnings),
            ..
        }) = item
        else {
            continue;
        };
        let SvgInput::Path(path) = &inputs[index] else {
            continue;
        };
        any = true;
        let prefix = format!("auto-converted {}: ", path.display());
        if converted_warnings.is_empty() {
            let message = format!(
                "{prefix}no data-panel-box; converted in memory (the source file was not modified)"
            );
            warnings.add(fit_warning(message));
        } else {
            for warning in converted_warnings {
                warnings.add(fit_warning(format!("{prefix}{}", clamp(warning, 400))));
            }
        }
    }
    if any {
        warnings.note(warnings::NOTE_AUTO_CONVERTED);
    }
}

fn fit_warning(message: String) -> String {
    clamp(&message, warnings::MAX_BODY)
}

/// Collapse a multi-line value to one line and cap it by **characters** (not
/// bytes), so the stderr line budget holds for non-ASCII paths and reasons too.
fn clamp(value: &str, max: usize) -> String {
    let one_line = value.replace(['\n', '\r', '\t'], " ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut clipped: String = one_line.chars().take(max.saturating_sub(1)).collect();
        // Prefer a word boundary: an auto-conversion warning ends with its own
        // `Fix: …` sentence, and cutting mid-word ("Fix: wr…") reads like a bug.
        if let Some(space) = clipped.rfind(char::is_whitespace) {
            if space >= clipped.len() / 2 {
                clipped.truncate(space);
            }
        }
        clipped.push('…');
        clipped
    }
}
