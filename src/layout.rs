use crate::error::{Result, SvgGridError};
use crate::geom::Rect;

#[derive(Debug, Clone)]
pub struct GridLayout {
    pub cells: Vec<Rect>,
    /// Resolved canvas size. When `GridSpec::height` is `None` the height is
    /// derived from the panels' natural heights and *is* the output canvas height.
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Margins {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

pub struct GridSpec<'a> {
    pub count: usize,
    pub nrow: Option<usize>,
    pub ncol: Option<usize>,
    pub width: f32,
    /// `Some(h)` → fill the given canvas height; `None` → auto height (the sum of
    /// the rows' natural heights plus the margins and row gaps), which then *is*
    /// the output canvas height.
    pub height: Option<f32>,
    pub margin: Margins,
    pub gap: f32,
    pub rel_widths: &'a [f32],
    pub rel_heights: &'a [f32],
    /// Canvas aspect ratio (`width / height`) of every input, in row-major order.
    /// `None` marks a spacer / absent cell, which behaves as a unit 1:1 blank.
    pub aspects: &'a [Option<f32>],
}

/// cowplot's near-square default when neither `--nrow` nor `--ncol` is given:
/// `cols = ceil(sqrt(count))`, `rows = ceil(count / cols)`. `count` counts input
/// *slots* (an explicit `NULL`/spacer is a declared empty cell), so a short last
/// row is legal and its missing trailing slots are never emitted.
pub(crate) fn default_grid(count: usize) -> (usize, usize) {
    if count == 0 {
        return (0, 0);
    }
    let cols = (count as f64).sqrt().ceil() as usize;
    (count.div_ceil(cols), cols)
}

pub fn compute_grid(spec: GridSpec<'_>) -> Result<GridLayout> {
    // Validate the requested grid *before* any arithmetic on it: `div_ceil(0)`
    // panics, and a panic (`rc=101`) tells the caller the tool is broken instead
    // of that its argument was (`ARCHITECTURE/cli.md` §退出码).
    if spec.nrow == Some(0) || spec.ncol == Some(0) {
        return Err(SvgGridError::InvalidInput(
            "nrow and ncol must be greater than zero".to_string(),
        ));
    }

    let (rows, cols) = match (spec.nrow, spec.ncol) {
        (Some(rows), Some(cols)) => (rows, cols),
        (Some(rows), None) => (rows, spec.count.div_ceil(rows)),
        (None, Some(cols)) => (spec.count.div_ceil(cols), cols),
        (None, None) => default_grid(spec.count),
    };

    if rows == 0 || cols == 0 {
        return Err(SvgGridError::InvalidInput(
            "nrow and ncol must be greater than zero".to_string(),
        ));
    }
    let capacity = rows
        .checked_mul(cols)
        .ok_or_else(|| SvgGridError::InvalidInput("grid dimensions are too large".to_string()))?;
    if capacity < spec.count {
        return Err(SvgGridError::InvalidInput(
            "grid does not contain enough cells for inputs".to_string(),
        ));
    }
    // A grid whose *last* row has no cell at all is just as wrong as one that is
    // too small: `(rows-1)*cols >= count` means every input fits in the rows above
    // the last one, so the last row would be drawn completely empty while every
    // preceding row is full. cowplot's grid never has an empty row; reject it with
    // the two concrete ways to fix the invocation rather than silently shrinking
    // the canvas (`ARCHITECTURE/layout.md`, D6).
    let cells_above_last = capacity - cols;
    if spec.count > 0 && cells_above_last >= spec.count {
        let min_rows = spec.count.div_ceil(cols);
        let missing = capacity - spec.count;
        let occupied_rows = cells_above_last / cols;
        return Err(SvgGridError::InvalidInput(format!(
            "grid leaves the last row empty: a {rows} x {cols} grid has {capacity} cells, but the \
             {count} input slot(s) all fit in the first {occupied_rows} row(s), so row {rows} has \
             no panel; fix: reduce --nrow to {min_rows}, or add {missing} `--input NULL` \
             placeholder(s) to fill all {capacity} cells",
            count = spec.count,
        )));
    }

    // `rel_*` are multipliers on each axis' *natural* size (cowplot semantics);
    // omitted → all 1 → pure natural. Validated fail-closed.
    let rel_widths = weights(cols, spec.rel_widths, "widths", "columns")?;
    let rel_heights = weights(rows, spec.rel_heights, "heights", "rows")?;

    if spec.margin.top < 0.0
        || spec.margin.right < 0.0
        || spec.margin.bottom < 0.0
        || spec.margin.left < 0.0
    {
        return Err(SvgGridError::InvalidInput(
            "plot margins must be non-negative".to_string(),
        ));
    }
    let available_w = spec.width
        - spec.margin.left
        - spec.margin.right
        - spec.gap * cols.saturating_sub(1) as f32;
    if available_w <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "gap and plot margins leave no drawable grid area".to_string(),
        ));
    }
    // Natural column width ∝ the canvas aspect of the column's first real panel
    // (a column of spacers falls back to a unit 1:1 blank).
    let natural_w = natural_column_weights(rows, cols, spec.aspects);
    let natural_w_sum: f32 = natural_w.iter().sum();
    if natural_w_sum <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "rel-widths/rel-heights leave no drawable cells".to_string(),
        ));
    }
    let column_signal: Vec<f32> = natural_w
        .iter()
        .zip(&rel_widths)
        .map(|(natural, multiplier)| natural * multiplier)
        .collect();

    // Cell size is decided by the *grid*, never by how many panels a row happens
    // to hold: a row is always measured as if it filled all `cols` columns.  So
    // the natural row height is `available_w ÷ Σ(natural column weights)` — the
    // same weights that distribute the column widths — and is shared by every
    // row.  A short last row therefore keeps the same cell size as the rows above
    // it instead of stretching its lone panel across the full width.
    //
    // The `rel_widths` multipliers are deliberately *not* part of this sum: they
    // are ratios ("1,1,2" ≡ "3.2,3.2,6.4", ARCHITECTURE/layout.md §1), and feeding
    // them into the height signal would make scaling every weight by a constant
    // change the derived canvas height.
    let natural_row_h = available_w / natural_w_sum;
    let row_signal: Vec<f32> = rel_heights
        .iter()
        .map(|multiplier| natural_row_h * multiplier)
        .collect();

    let sum_w: f32 = column_signal.iter().sum();
    let sum_h: f32 = row_signal.iter().sum();
    if sum_w <= 0.0 || sum_h <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "rel-widths/rel-heights leave no drawable cells".to_string(),
        ));
    }

    let row_gaps = spec.gap * rows.saturating_sub(1) as f32;
    // Resolve the canvas height first. Explicit → fill the given canvas; omitted
    // → the row stack's natural height, which becomes the output canvas height.
    // Either way the rows may only occupy the space left by the margins and gaps.
    let total_height = match spec.height {
        Some(height) => height,
        None => spec.margin.top + spec.margin.bottom + row_gaps + sum_h,
    };
    let available_h = total_height - spec.margin.top - spec.margin.bottom - row_gaps;
    if available_h <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "gap and plot margins leave no drawable grid area".to_string(),
        ));
    }

    let mut cells = Vec::with_capacity(spec.count);
    let mut y = spec.margin.top;
    for (row, row_weight) in row_signal.iter().take(rows).enumerate() {
        let h = available_h * row_weight / sum_h;
        if h < MIN_CELL_SIDE {
            // Name the input that collapsed the cell: the relative weight if one
            // was given, else the explicit height, else the width the row height
            // was derived from.
            let (flag, value) = match spec.rel_heights.get(row) {
                Some(value) => ("--rel-heights", *value),
                None => match spec.height {
                    Some(height) => ("--height", height),
                    None => ("--width", spec.width),
                },
            };
            return Err(degenerate_cell(flag, value, "tall"));
        }
        let mut x = spec.margin.left;
        for (column, column_weight) in column_signal.iter().take(cols).enumerate() {
            let w = available_w * column_weight / sum_w;
            if w < MIN_CELL_SIDE {
                let (flag, value) = match spec.rel_widths.get(column) {
                    Some(value) => ("--rel-widths", *value),
                    None => ("--width", spec.width),
                };
                return Err(degenerate_cell(flag, value, "wide"));
            }
            if cells.len() < spec.count {
                cells.push(Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                });
            }
            x += w + spec.gap;
        }
        y += h + spec.gap;
    }

    Ok(GridLayout {
        cells,
        width: spec.width,
        height: total_height,
    })
}

/// A cell narrower or shorter than this rounds to zero pixels in every renderer,
/// so the panel could never be drawn where the layout puts it.
const MIN_CELL_SIDE: f32 = 0.5;

/// Reject a grid whose resolved cell has collapsed. `--rel-widths 1e-9,1` used to
/// be accepted (the weight is positive) and then produced a 0x60.8 cell — a
/// silently broken panel — so the gate names the flag and the offending raw token
/// instead. A relative weight is not the only way to collapse a cell: with tiny
/// canvases the `--width`/`--height` itself is the culprit, and then that flag and
/// value are named.
fn degenerate_cell(flag: &str, value: f32, span: &str) -> SvgGridError {
    SvgGridError::InvalidInput(format!(
        "{flag} must leave a cell at least 1 px {span}, got \"{value:?}\""
    ))
}

/// The natural width weight of each column: the canvas aspect of the column's
/// first real panel, or `1.0` when every cell in the column is a spacer/absent.
fn natural_column_weights(rows: usize, cols: usize, aspects: &[Option<f32>]) -> Vec<f32> {
    (0..cols)
        .map(|column| {
            (0..rows)
                .map(|row| row * cols + column)
                .find_map(|index| aspects.get(index).copied().flatten())
                .unwrap_or(1.0)
        })
        .collect()
}

/// Resolve one axis' relative weights, fail-closed.
///
/// An empty `supplied` means "not given" → uniform weights, exactly as before.
/// Anything else is validated: a length that does not match the axis would
/// otherwise be silently ignored, and a negative weight is meaningless.
fn weights(count: usize, supplied: &[f32], axis: &str, noun: &str) -> Result<Vec<f32>> {
    if supplied.is_empty() {
        return Ok(vec![1.0; count]);
    }
    if supplied.len() != count {
        return Err(SvgGridError::InvalidInput(format!(
            "rel-{axis} has {} value(s) but the grid has {count} {noun}",
            supplied.len()
        )));
    }
    if supplied.iter().any(|value| *value < 0.0) {
        return Err(SvgGridError::InvalidInput(format!(
            "rel-{axis} must be non-negative"
        )));
    }
    if supplied.iter().sum::<f32>() <= 0.0 {
        return Err(SvgGridError::InvalidInput(format!(
            "rel-{axis} must contain at least one positive value"
        )));
    }
    Ok(supplied.to_vec())
}
