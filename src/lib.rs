mod dom;
mod error;
mod geom;
mod input;
mod layout;
mod normalize;
mod output;
mod raster;
mod style;
mod transform;
mod warnings;

use std::path::PathBuf;

pub use error::{Result, SvgGridError};
pub use layout::Margins;
pub use raster::{render_svg_to_avif, render_svg_to_png, render_svg_to_png_with_dpi};

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
/// ≈0.73 em; rounded up for safety).
const LABEL_CAP_HEIGHT_RATIO: f32 = 0.75;
/// Extra clear space kept around the letter's ink, as a fraction of the size.
const LABEL_BAND_PAD_RATIO: f32 = 0.05;
/// Ink width of one capital letter as a fraction of the size (upper bound across
/// the serif capitals; `A` alone is ≈0.74).
const LABEL_INK_WIDTH_RATIO: f32 = 0.75;

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

/// A parsed input SVG together with the geometry the composer needs from it.
struct PreparedInput {
    root: dom::Element,
    panel: geom::Rect,
    canvas: geom::Rect,
}

pub fn plot_grid(spec: PlotGridSpec) -> Result<()> {
    if spec.inputs.is_empty() {
        return Err(SvgGridError::InvalidInput(
            "plot_grid requires at least one input SVG".to_string(),
        ));
    }

    // Every input is parsed up front: the layout now reads each panel's canvas
    // aspect ratio to derive natural column widths and row heights.
    let mut prepared: Vec<Option<PreparedInput>> = Vec::with_capacity(spec.inputs.len());
    let mut aspects: Vec<Option<f32>> = Vec::with_capacity(spec.inputs.len());
    for input in &spec.inputs {
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
        if let Some(normalize) = &spec.normalize {
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
        }));
    }
    let drawn: Vec<bool> = prepared.iter().map(Option::is_some).collect();

    let grid = layout::compute_grid(layout::GridSpec {
        count: spec.inputs.len(),
        nrow: spec.nrow,
        ncol: spec.ncol,
        width: spec.width,
        height: spec.height,
        margin: spec.margin,
        gap: spec.gap,
        rel_widths: &spec.rel_widths,
        rel_heights: &spec.rel_heights,
        aspects: &aspects,
    })?;
    // The resolved grid size IS the output canvas (root width/height/viewBox, and
    // thus the PNG pixel count): `--width`/`--height` are never a "logical" size
    // that differs from the output.
    let canvas_width = grid.width;
    let canvas_height = grid.height;

    // Non-fatal diagnostics: a run that "looks successful but is quietly wrong"
    // must still exit 0 and write its output, but must tell the caller on stderr.
    // Gathered before the inputs are consumed and flushed after writing.
    let mut warnings = warnings::Warnings::default();
    collect_warnings(
        &spec,
        &prepared,
        &grid.cells,
        &drawn,
        canvas_width,
        canvas_height,
        &mut warnings,
    );

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
        let Some(PreparedInput {
            mut root,
            panel,
            canvas,
        }) = input
        else {
            // A spacer draws nothing; the white background is already there.
            continue;
        };
        let cell = grid.cells[index];
        let target_panel = geom::scale_panel_to_cell(panel, canvas, cell)?;
        transform::transform_svg(&mut root, panel, target_panel)?;
        namespace_ids(&mut root, &format!("svg-grid-cell{index}-"));

        root.name = "g".to_string();
        root.set_attr("data-svg-grid-cell", index.to_string());
        root.remove_attr("width");
        root.remove_attr("height");
        root.remove_attr("viewBox");
        root.remove_attr("xmlns");
        if let Some(labels) = &spec.labels {
            if let Some(label) = build_label(index, cell, spec.width, labels) {
                root.children.push(dom::Node::Element(label));
            }
        }
        output.children.push(dom::Node::Element(root));
    }

    let svg = dom::serialize(&output)?;
    output::write_outputs(&svg, &spec.output_svg, spec.output_svgz.as_deref())?;

    // Flush diagnostics only once the output is on disk: they never change the
    // exit code nor the output bytes.
    warnings.emit();

    Ok(())
}

pub fn auto_labels(count: usize, uppercase: bool) -> Vec<String> {
    (0..count)
        .map(|index| alphabet_label(index, uppercase))
        .collect()
}

pub(crate) fn format_number(value: f32) -> String {
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
        scales.push(warnings::InputScale {
            index,
            sx: cell.width / item.canvas.width,
            sy: cell.height / item.canvas.height,
        });
        stretches.push(warnings::CellStretch {
            index,
            cell_aspect: cell.width / cell.height,
            panel_aspect: item.canvas.width / item.canvas.height,
        });
    }
    if let Some(message) = warnings::describe_scale(&scales) {
        warnings.add(message);
    }
    if let Some(message) = warnings::describe_stretch(&stretches) {
        warnings.add(message);
    }

    let Some(labels) = &spec.labels else { return };
    if labels.labels.is_empty() {
        return; // `--labels none`: nothing is drawn and nothing can be mislabelled
    }

    // (3) A composite input that already carries letters is being lettered again.
    let relettered: Vec<usize> = prepared
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let item = item.as_ref()?;
            let label = labels.labels.get(index)?;
            (!label.is_empty() && contains_data_svg_grid_label(&item.root)).then_some(index)
        })
        .collect();
    if let Some(message) = warnings::describe_existing_labels(&relettered) {
        warnings.add(message);
    }

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
        let ink = label_ink_box(label, *cell, index, labels, size);
        if let Some(message) =
            warnings::describe_clipped_label(label, index, ink, canvas_width, canvas_height)
        {
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
    let x = cell.x + label_value(&labels.x, index, 0.0) * cell.width
        - label_value(&labels.hjust, index, 0.0) * size;
    let y = cell.y
        + (1.0 - label_value(&labels.y, index, 1.0)) * cell.height
        + label_value(&labels.vjust, index, -0.25) * size;
    let characters = label.chars().count().max(1) as f32;
    geom::Rect {
        x,
        y: y - LABEL_CAP_HEIGHT_RATIO * size,
        width: LABEL_INK_WIDTH_RATIO * size * characters,
        height: (LABEL_CAP_HEIGHT_RATIO + LABEL_BAND_PAD_RATIO) * size,
    }
}

/// Whether an input subtree already contains a `data-svg-grid-label` marker, i.e.
/// it is (part of) a figure that a previous `plot_grid` call already lettered.
fn contains_data_svg_grid_label(element: &dom::Element) -> bool {
    element.attr("data-svg-grid-label").is_some()
        || element.children.iter().any(|child| match child {
            dom::Node::Element(child) => contains_data_svg_grid_label(child),
            dom::Node::Text(_) => false,
        })
}

fn build_label(
    index: usize,
    cell: geom::Rect,
    canvas_width: f32,
    labels: &LabelSpec,
) -> Option<dom::Element> {
    let label = labels.labels.get(index)?;
    if label.is_empty() {
        return None;
    }
    let size = labels.size.resolve(canvas_width);
    // Defaults keep the letter just outside the panel's top-left corner: `hjust=0`
    // aligns its start with the panel's left edge, `vjust=-0.25` lifts its baseline
    // above the panel's top edge (the panel's box starts at `cell.y`).
    let x = cell.x + label_value(&labels.x, index, 0.0) * cell.width
        - label_value(&labels.hjust, index, 0.0) * size;
    let y = cell.y
        + (1.0 - label_value(&labels.y, index, 1.0)) * cell.height
        + label_value(&labels.vjust, index, -0.25) * size;
    let mut attrs = vec![
        ("data-svg-grid-label".to_string(), index.to_string()),
        ("data-scale".to_string(), "position".to_string()),
        ("x".to_string(), format_number(x)),
        ("y".to_string(), format_number(y)),
        ("font-family".to_string(), labels.font_family.clone()),
        ("font-size".to_string(), format_number(size)),
        ("fill".to_string(), labels.colour.clone()),
    ];
    apply_font_face(&mut attrs, &labels.font_face);
    Some(dom::Element {
        name: "text".to_string(),
        attrs,
        children: vec![dom::Node::Text(label.clone())],
    })
}

fn label_value(values: &[f32], index: usize, fallback: f32) -> f32 {
    values
        .get(index)
        .copied()
        .or_else(|| values.first().copied())
        .unwrap_or(fallback)
}

fn apply_font_face(attrs: &mut Vec<(String, String)>, font_face: &str) {
    let normalized = font_face.replace(['_', '-', '.'], "").to_lowercase();
    if normalized.contains("bold") {
        attrs.push(("font-weight".to_string(), "bold".to_string()));
    }
    if normalized.contains("italic") {
        attrs.push(("font-style".to_string(), "italic".to_string()));
    }
}

fn alphabet_label(index: usize, uppercase: bool) -> String {
    let mut number = index + 1;
    let mut chars = Vec::new();
    while number > 0 {
        number -= 1;
        let base = if uppercase { b'A' } else { b'a' };
        chars.push((base + (number % 26) as u8) as char);
        number /= 26;
    }
    chars.iter().rev().collect()
}

fn namespace_ids(element: &mut dom::Element, prefix: &str) {
    for (name, value) in &mut element.attrs {
        if name == "id" {
            *value = format!("{prefix}{value}");
        } else if (name == "href" || name == "xlink:href") && value.starts_with('#') {
            *value = format!("#{prefix}{}", &value[1..]);
        } else if value.contains("url(#") {
            *value = value.replace("url(#", &format!("url(#{prefix}"));
        }
    }
    for child in &mut element.children {
        if let dom::Node::Element(child) = child {
            namespace_ids(child, prefix);
        }
    }
}
