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

pub fn compute_grid(spec: GridSpec<'_>) -> Result<GridLayout> {
    let (rows, cols) = match (spec.nrow, spec.ncol) {
        (Some(rows), Some(cols)) => (rows, cols),
        (Some(rows), None) => (rows, spec.count.div_ceil(rows)),
        (None, Some(cols)) => (spec.count.div_ceil(cols), cols),
        (None, None) => (1, spec.count),
    };

    if rows == 0 || cols == 0 {
        return Err(SvgGridError::InvalidInput(
            "nrow and ncol must be greater than zero".to_string(),
        ));
    }
    if rows * cols < spec.count {
        return Err(SvgGridError::InvalidInput(
            "grid does not contain enough cells for inputs".to_string(),
        ));
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
    let column_signal: Vec<f32> = natural_w
        .iter()
        .zip(&rel_widths)
        .map(|(natural, multiplier)| natural * multiplier)
        .collect();

    // Natural row height = available width ÷ Σ(aspects of that row's panels);
    // a spacer contributes a unit 1:1 blank.  Scaling every column width by its
    // aspect then makes each cell's width/height equal the panel's own aspect.
    let row_aspect_sum = row_aspect_sums(rows, cols, spec.count, spec.aspects);
    let row_signal: Vec<f32> = row_aspect_sum
        .iter()
        .zip(&rel_heights)
        .map(|(sum, multiplier)| {
            let natural = if *sum > 0.0 { available_w / *sum } else { 0.0 };
            natural * multiplier
        })
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
    for row_weight in row_signal.iter().take(rows) {
        let h = available_h * row_weight / sum_h;
        let mut x = spec.margin.left;
        for column_weight in column_signal.iter().take(cols) {
            let w = available_w * column_weight / sum_w;
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

/// Σ(canvas aspects) for every row, counting a spacer/an absent cell as a unit
/// blank (`1.0`) and absent trailing cells as `0.0`.
fn row_aspect_sums(rows: usize, cols: usize, count: usize, aspects: &[Option<f32>]) -> Vec<f32> {
    (0..rows)
        .map(|row| {
            (0..cols)
                .filter_map(|column| {
                    let index = row * cols + column;
                    (index < count).then(|| aspects.get(index).copied().flatten().unwrap_or(1.0))
                })
                .sum()
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
