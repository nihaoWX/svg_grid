pub mod convert;
mod dom;
mod error;
mod geom;
pub mod home;
mod ink;
mod input;
mod label_conflicts;
mod labels;
mod layout;
mod normalize;
mod ns;
mod output;
mod path;
mod plan;
mod raster;
mod style;
mod transform;
mod warnings;

use std::path::PathBuf;

use plan::{collect_auto_conversion_diagnostics, prepare, PreparedInput};

pub use error::{Result, SvgGridError};
pub(crate) use label_conflicts::validate_label_conflicts;
pub use layout::Margins;
pub use plan::{
    plan_grid, plan_grid_with_diagnostics, plan_grid_with_labels, GridPlan, GridRequest,
    PlannedInput,
};
pub use raster::{
    data_root_font_dir, default_font_dir, font_families, load_font_database, render_svg_to_avif,
    render_svg_to_avif_with_fonts, render_svg_to_png, render_svg_to_png_with_dpi,
    render_svg_to_png_with_dpi_with_fonts, render_svg_to_png_with_fonts, DEFAULT_FONT_FAMILY,
    FONT_DIR_ENV,
};

#[derive(Debug, Clone)]
pub enum SvgInput {
    Path(PathBuf),
    /// A blank grid cell (cowplot's `NULL`): it occupies its cell in the layout
    /// but draws nothing, so it can leave intentional whitespace.
    Spacer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMode {
    None,
    Panels,
}

#[derive(Debug, Clone)]
pub struct PlotGridSpec {
    pub inputs: Vec<SvgInput>,
    pub output_svg: PathBuf,
    pub output_svgz: Option<PathBuf>,
    pub width: f32,
    /// `None` → derive the canvas height from the panels' natural heights.
    pub height: Option<f32>,
    pub nrow: Option<usize>,
    pub ncol: Option<usize>,
    pub rel_widths: Vec<f32>,
    pub rel_heights: Vec<f32>,
    pub margin: Margins,
    pub gap: f32,
    pub align: AlignMode,
    pub labels: Option<LabelSpec>,
    pub normalize: Option<InputNormalize>,
    /// Families the font library (`fontlib/`, or `--font-dir`) actually holds, as
    /// `--list-fonts` prints them.
    ///
    /// Only the caller can know the library, so it is passed in: it is what W8
    /// compares `--label-fontfamily` against. `None` = no library was consulted
    /// (a library-level call that never rasterises), so W8 is not judged.
    pub font_families: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct InputNormalize {
    pub max_side: f32,
}

/// Ink width of a serif capital `A` as a fraction of the nominal font size
/// (per `tools/manuscripts_skills/general-figure-guide`, DejaVu Serif).
const SERIF_A_INK_WIDTH_RATIO: f32 = 0.74;

/// The manuscript rule: the panel letter's ink width is ≈2 % of the canvas width.
const LABEL_INK_WIDTH_OF_CANVAS: f32 = 0.02;

/// Nominal font size realising that rule: `0.02 / 0.74 ≈ 0.027` of the canvas
/// width (`A` ink width ≈ 0.74 × nominal size ≈ 0.02 × canvas width).
const AUTO_LABEL_SIZE_RATIO: f32 = LABEL_INK_WIDTH_OF_CANVAS / SERIF_A_INK_WIDTH_RATIO;

/// How the panel-letter font size is chosen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelSize {
    /// ≈2.7 % of the canvas width (see [`AUTO_LABEL_SIZE_RATIO`]).
    Auto,
    /// An explicit size, in canvas units.
    Fixed(f32),
}

impl LabelSize {
    pub fn resolve(self, canvas_width: f32) -> f32 {
        match self {
            LabelSize::Auto => AUTO_LABEL_SIZE_RATIO * canvas_width,
            LabelSize::Fixed(size) => size,
        }
    }
}

/// Cap height of a capital letter as a fraction of the font size (DejaVu Serif
/// ≈0.73 em; rounded up for safety). Shared with the W6 ink estimate.
pub(crate) const LABEL_CAP_HEIGHT_RATIO: f32 = 0.75;
/// Extra clear space kept around the letter's ink, as a fraction of the size.
const LABEL_BAND_PAD_RATIO: f32 = 0.05;
/// Ink width of one capital letter as a fraction of the size (upper bound across
/// the serif capitals; `A` alone is ≈0.74). Shared with the W6 ink estimate.
pub(crate) const LABEL_INK_WIDTH_RATIO: f32 = 0.75;

/// Top margin needed above a panel row so its default-placed letter is not
/// clipped: the glyph rises `≈cap height` above its baseline, and the default
/// `vjust` lifts the baseline above the cell's top edge, plus a small pad.
pub fn auto_label_top_band(size: f32, vjust: &[f32]) -> f32 {
    let lift = vjust.iter().copied().fold(0.0_f32, f32::min).min(0.0).abs();
    size * (LABEL_CAP_HEIGHT_RATIO + lift + LABEL_BAND_PAD_RATIO)
}

#[derive(Debug, Clone)]
pub struct LabelSpec {
    pub labels: Vec<String>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub hjust: Vec<f32>,
    pub vjust: Vec<f32>,
    pub size: LabelSize,
    pub font_family: String,
    pub font_face: String,
    pub colour: String,
}

/// Compose the panels and write the requested outputs. Non-fatal diagnostics are
/// collected but **not printed**: stderr stays empty on a successful run, matching
/// the CLI default. Use [`plot_grid_with_diagnostics`] to opt into the W1–W9
/// warning surface (`--verbose`).
pub fn plot_grid(spec: PlotGridSpec) -> Result<()> {
    plot_grid_with_diagnostics(spec, false)
}

/// [`plot_grid`] with explicit control over the non-fatal diagnostics: when
/// `verbose` is true, W1–W9 and their shared cause notes are printed to stderr
/// after the output is written. The exit code and the output bytes never change.
pub fn plot_grid_with_diagnostics(spec: PlotGridSpec, verbose: bool) -> Result<()> {
    let (prepared, grid) = prepare(&GridRequest {
        inputs: spec.inputs.clone(),
        width: spec.width,
        height: spec.height,
        nrow: spec.nrow,
        ncol: spec.ncol,
        rel_widths: spec.rel_widths.clone(),
        rel_heights: spec.rel_heights.clone(),
        margin: spec.margin,
        gap: spec.gap,
        normalize: spec.normalize.clone(),
    })?;
    let drawn: Vec<bool> = prepared.iter().map(Option::is_some).collect();

    // W3 is a hard error now, and so is W8: both are "necessary information" in
    // `INSTRUCTIONS.md`. They are checked before any output path is opened, so a
    // failing call leaves no file behind.
    if let Some(labels) = &spec.labels {
        validate_label_conflicts(&prepared, labels)?;
        if let Some(available) = &spec.font_families {
            if labels.labels.iter().any(|label| !label.is_empty()) {
                if let Some(message) =
                    warnings::describe_missing_font_error(&labels.font_family, available)
                {
                    return Err(SvgGridError::Fonts(message));
                }
            }
        }
    }

    // The resolved grid size IS the output canvas (root width/height/viewBox, and
    // thus the PNG pixel count): `--width`/`--height` are never a "logical" size
    // that differs from the output.
    let canvas_width = grid.width;
    let canvas_height = grid.height;

    // Non-fatal diagnostics: a run that "looks successful but is quietly wrong"
    // must still exit 0 and write its output, but must tell the caller on stderr.
    // Gathered before the inputs are consumed and flushed after writing.
    let mut warnings = warnings::Warnings::default();
    // In-memory auto-conversion of untagged inputs happened inside `prepare`; its
    // converter warnings join the same `--verbose` channel as W1–W9.
    collect_auto_conversion_diagnostics(&spec.inputs, &prepared, &mut warnings);
    collect_warnings(
        &spec,
        &prepared,
        &grid.cells,
        &drawn,
        canvas_width,
        canvas_height,
        &mut warnings,
    );

    // Attributes the composer could not rewrite but deliberately kept verbatim
    // while transforming the inputs (W7: a malformed `<path d>` or a non-finite
    // numeric attribute). Gathered across every input so the warning speaks once
    // per call, and flushed with W1–W6 after the output is written.
    let mut passthroughs = warnings::Passthroughs::default();

    // `url(#…)` references the composed output cannot resolve (W9): a paint server
    // or a clip path that a stage above dropped. Nothing fails and nothing moves —
    // the shapes are just painted with nothing — so this is the one mechanical
    // check that keeps that silent ink loss from shipping (round-data/R16).
    let mut dangling = Vec::new();

    let mut output = dom::Element {
        name: "svg".to_string(),
        attrs: vec![
            ("width".to_string(), format_number(canvas_width)),
            ("height".to_string(), format_number(canvas_height)),
            (
                "viewBox".to_string(),
                format!(
                    "0 0 {} {}",
                    format_number(canvas_width),
                    format_number(canvas_height)
                ),
            ),
            (
                "xmlns".to_string(),
                "http://www.w3.org/2000/svg".to_string(),
            ),
        ],
        children: Vec::new(),
    };
    output.children.push(dom::Node::Element(dom::Element {
        name: "rect".to_string(),
        attrs: vec![
            ("data-panel-box".to_string(), "main".to_string()),
            ("x".to_string(), "0".to_string()),
            ("y".to_string(), "0".to_string()),
            ("width".to_string(), format_number(canvas_width)),
            ("height".to_string(), format_number(canvas_height)),
            ("visibility".to_string(), "hidden".to_string()),
            ("pointer-events".to_string(), "none".to_string()),
        ],
        children: Vec::new(),
    }));
    output.children.push(dom::Node::Element(dom::Element {
        name: "rect".to_string(),
        attrs: vec![
            ("x".to_string(), "0".to_string()),
            ("y".to_string(), "0".to_string()),
            ("width".to_string(), format_number(canvas_width)),
            ("height".to_string(), format_number(canvas_height)),
            ("fill".to_string(), "#FFFFFF".to_string()),
            ("stroke".to_string(), "none".to_string()),
        ],
        children: Vec::new(),
    }));

    for (index, input) in prepared.into_iter().enumerate() {
        let cell = grid.cells[index];
        let Some(PreparedInput {
            mut root,
            panel,
            canvas,
            auto_converted: _,
        }) = input
        else {
            // A spacer draws nothing, but an explicit `--labels` entry for this
            // slot is still honoured: `NULL` occupies a cell, so its letter is
            // drawn at the empty cell's top-left (INSTRUCTIONS.md `--labels`).
            if let Some(labels) = &spec.labels {
                if let Some(label) = labels::build(index, cell, spec.width, labels) {
                    output.children.push(dom::Node::Element(label));
                }
            }
            continue;
        };
        let target_panel = geom::scale_panel_to_cell(panel, canvas, cell)?;
        transform::transform_svg(&mut root, panel, target_panel, index, &mut passthroughs)?;
        // W9 is checked on the input's own subtree, before the ids are namespaced,
        // so the message names the id exactly as the source file spells it.
        dangling.extend(warnings::dangling_references(&root, index));
        ns::namespace_ids(&mut root, &format!("svg-grid-cell{index}-"));

        root.name = "g".to_string();
        root.set_attr("data-svg-grid-cell", index.to_string());
        root.remove_attr("width");
        root.remove_attr("height");
        root.remove_attr("viewBox");
        root.remove_attr("xmlns");
        if let Some(labels) = &spec.labels {
            if let Some(label) = labels::build(index, cell, spec.width, labels) {
                root.children.push(dom::Node::Element(label));
            }
        }
        output.children.push(dom::Node::Element(root));
    }

    // W7: attributes kept verbatim (malformed `<path d>`, non-finite numbers).
    // One aggregated message for the whole call; added to the same collector as
    // W1–W6 so it is flushed in the same place, after the output is written.
    if let Some(message) = warnings::describe_passthrough(passthroughs.offenders()) {
        warnings.note(warnings::NOTE_PASSTHROUGH);
        warnings.add(message);
    }
    // W9: `url(#…)` references nothing in the output defines.
    if let Some(message) = warnings::describe_dangling_reference(&dangling) {
        warnings.note(warnings::NOTE_DANGLING);
        warnings.add(message);
    }

    let svg = dom::serialize(&output)?;
    output::write_outputs(&svg, &spec.output_svg, spec.output_svgz.as_deref())?;

    // Flush diagnostics only once the output is on disk, and only for a verbose
    // run: they never change the exit code nor the output bytes.
    warnings.emit(verbose);

    Ok(())
}

pub fn auto_labels(count: usize, uppercase: bool) -> Vec<String> {
    (0..count)
        .map(|index| labels::alphabet_label(index, uppercase))
        .collect()
}

/// The number format of every coordinate in the composed SVG, and therefore of every
/// number `plot-grid --plan` prints — the plan shows the digits the figure contains.
pub fn format_number(value: f32) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.3}")
    }
}

/// Gather every non-fatal warning for this run (see the [`warnings`] module).
///
/// Called before the inputs are consumed and before the output is written, so it
/// can only read the layout: it never affects the exit code or the output bytes.
fn collect_warnings(
    spec: &PlotGridSpec,
    prepared: &[Option<PreparedInput>],
    cells: &[geom::Rect],
    drawn: &[bool],
    canvas_width: f32,
    canvas_height: f32,
    warnings: &mut warnings::Warnings,
) {
    // (1) Inputs rescaled by the layout and (2) cells stretched away from their
    // panel's aspect ratio are both read straight from the resolved scale, before
    // any coordinate rewrite.
    let mut scales = Vec::new();
    let mut stretches = Vec::new();
    for (index, item) in prepared.iter().enumerate() {
        let Some(item) = item else { continue };
        let Some(cell) = cells.get(index) else {
            continue;
        };
        // The one definition of the layout scale (`geom::cell_scale`), so W1 and
        // `--plan` report the number the composition actually applies.
        let Ok((sx, sy)) = geom::cell_scale(*cell, item.canvas) else {
            continue;
        };
        scales.push(warnings::InputScale { index, sx, sy });
        stretches.push(warnings::CellStretch {
            index,
            cell_aspect: cell.width / cell.height,
            panel_aspect: item.canvas.width / item.canvas.height,
        });
    }
    if let Some(message) = warnings::describe_scale(&scales) {
        warnings.note(warnings::NOTE_FONT_SIZE);
        warnings.add(message);
    }
    if let Some(message) = warnings::describe_stretch(&stretches) {
        warnings.note(warnings::NOTE_FONT_SIZE);
        warnings.add(message);
    }

    // (2b) Content ink that leaves the input's OWN canvas box. This is read from
    // the (possibly normalized) input and its matching canvas box, so the two
    // quantities always come from the same stage. It does not depend on
    // `--labels`, so it runs before the label-only early return below.
    for (index, item) in prepared.iter().enumerate() {
        let Some(item) = item else { continue };
        let Some(ink) = ink::ink_bbox(&item.root) else {
            continue; // nothing measurable is drawn
        };
        if let Some(message) = warnings::describe_content_outside_canvas(index, ink, item.canvas) {
            warnings.note(warnings::NOTE_CANVAS_INK);
            if message.contains(warnings::DUST_HINT) {
                warnings.note(warnings::NOTE_INK_DUST);
            }
            warnings.add(message);
        }
    }

    let Some(labels) = &spec.labels else { return };
    if labels.labels.is_empty() {
        return; // `--labels none`: nothing is drawn and nothing can be mislabelled
    }

    // (3) W3 and W8 are hard errors since D5: they were validated before the
    // output was opened, so only the remaining non-fatal label diagnostics live
    // here.

    // (4) Duplicate letters and a label count that does not match the input count
    // (empty slots included).
    if let Some(message) = warnings::describe_label_count(labels.labels.len(), spec.inputs.len()) {
        warnings.add(message);
    }
    if let Some(message) = warnings::describe_label_duplicates(&labels.labels) {
        warnings.add(message);
    }

    // (5) Letters whose ink would be clipped: an explicit `--plot-margin` is
    // respected, so a too-small one is warned about here. With the automatic
    // reservation the band is grown to fit the letters, so this stays silent.
    let size = labels.size.resolve(canvas_width);
    for (index, cell) in cells.iter().enumerate() {
        if !drawn.get(index).copied().unwrap_or(false) {
            continue; // spacer / absent cell: no letter is drawn
        }
        let Some(label) = labels.labels.get(index) else {
            continue;
        };
        if label.is_empty() {
            continue;
        }
        let raw = labels::raw_baseline(*cell, index, labels, size);
        let placed = labels::baseline(*cell, index, labels, size);
        if let Some(message) = warnings::describe_moved_label(label, index, raw, placed) {
            warnings.note(warnings::NOTE_LABEL_CLIP);
            warnings.add(message);
        }
        let ink = label_ink_box(label, *cell, index, labels, size);
        if let Some(message) =
            warnings::describe_clipped_label(label, index, ink, canvas_width, canvas_height)
        {
            warnings.note(warnings::NOTE_LABEL_CLIP);
            warnings.add(message);
        }
    }
}

/// Estimated ink box (in canvas coordinates) of the panel letter placed at
/// `cell`.
fn label_ink_box(
    label: &str,
    cell: geom::Rect,
    index: usize,
    labels: &LabelSpec,
    size: f32,
) -> geom::Rect {
    let x = cell.x + labels::value(&labels.x, index, 0.0) * cell.width
        - labels::value(&labels.hjust, index, 0.0) * size;
    let y = labels::baseline(cell, index, labels, size);
    let characters = label.chars().count().max(1) as f32;
    geom::Rect {
        x,
        y: y - LABEL_CAP_HEIGHT_RATIO * size,
        width: LABEL_INK_WIDTH_RATIO * size * characters,
        height: (LABEL_CAP_HEIGHT_RATIO + LABEL_BAND_PAD_RATIO) * size,
    }
}
