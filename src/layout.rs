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
fn default_grid(count: usize) -> (usize, usize) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every grid below uses 100×80 panels (canvas aspect 1.25) and a 1000-wide
    /// canvas with no margins or gaps, so the expected numbers stay readable.
    const ASPECT: f32 = 1.25;

    fn fit(count: usize, ncol: Option<usize>, nrow: Option<usize>) -> GridLayout {
        let aspects = vec![Some(ASPECT); count];
        compute_grid(GridSpec {
            count,
            nrow,
            ncol,
            width: 1000.0,
            height: None,
            margin: Margins::zero(),
            gap: 0.0,
            rel_widths: &[],
            rel_heights: &[],
            aspects: &aspects,
        })
        .unwrap()
    }

    fn fit_default(count: usize) -> GridLayout {
        let aspects = vec![Some(ASPECT); count];
        compute_grid(GridSpec {
            count,
            nrow: None,
            ncol: None,
            width: 1000.0,
            height: None,
            margin: Margins::zero(),
            gap: 0.0,
            rel_widths: &[],
            rel_heights: &[],
            aspects: &aspects,
        })
        .unwrap()
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn short_last_row_keeps_the_full_grid_cell_size() {
        // 5 panels in 2 columns → 3 rows (2 + 2 + 1).  The lone last-row panel
        // must not be stretched across the row: every cell is 500×400 and the
        // rows sit at y = 0 / 400 / 800.
        let grid = fit(5, Some(2), None);
        assert_close(grid.height, 1200.0);
        for (index, cell) in grid.cells.iter().enumerate() {
            assert_close(cell.width, 500.0);
            assert_close(cell.height, 400.0);
            assert_close(cell.x, if index % 2 == 0 { 0.0 } else { 500.0 });
            assert_close(cell.y, (index / 2) as f32 * 400.0);
        }
        // The first and last cells (the latter alone in its row) are identical,
        // and the last row starts in the left-most column.
        assert_close(grid.cells[4].width, grid.cells[0].width);
        assert_close(grid.cells[4].height, grid.cells[0].height);
        assert_close(grid.cells[4].x, grid.cells[0].x);
    }

    #[test]
    fn short_last_row_is_measured_as_if_it_filled_all_columns() {
        // 5 panels in 3 columns → 2 rows (3 + 2).  Both rows share one height and
        // the two last-row cells keep the same width as the three above them.
        let grid = fit(5, Some(3), None);
        assert_close(grid.height, 533.333);
        for cell in &grid.cells {
            assert_close(cell.width, 333.333);
            assert_close(cell.height, 266.667);
        }
        assert_close(grid.cells[3].y - grid.cells[0].y, grid.cells[0].height);
        assert_close(grid.cells[4].y, grid.cells[3].y);
        assert_close(grid.cells[3].x, 0.0);
        assert_close(grid.cells[4].x, 333.333);

        // `--nrow 2` derives the same 3 columns and must land on the same grid.
        let by_row = fit(5, None, Some(2));
        assert_close(by_row.height, grid.height);
        for (left, right) in grid.cells.iter().zip(&by_row.cells) {
            assert_close(left.x, right.x);
            assert_close(left.y, right.y);
            assert_close(left.width, right.width);
            assert_close(left.height, right.height);
        }
    }

    #[test]
    fn complete_rows_keep_the_exact_geometry_they_had_before() {
        // The fix only applies to the "row measured by its own panel count"
        // signal; grids whose rows are all complete must not move.  These are the
        // sizes the 100×80 / 1000-wide fixtures produced before D2.
        for (count, cols, expected_h) in [
            (4, 2, 800.0),
            (6, 2, 1200.0),
            (9, 3, 800.0),
            (16, 4, 800.0),
            (15, 5, 480.0),
        ] {
            let grid = fit(count, Some(cols), None);
            assert_close(grid.height, expected_h);
            let cell_w = 1000.0 / cols as f32;
            let cell_h = cell_w / ASPECT;
            for cell in &grid.cells {
                assert_close(cell.width, cell_w);
                assert_close(cell.height, cell_h);
            }
        }
    }

    #[test]
    fn rel_heights_only_scale_the_shared_row_height() {
        // 5 panels in 2 columns with --rel-heights 1,1,2: the short last row is
        // twice the height of the rows above, but still one column wide.
        let aspects = vec![Some(ASPECT); 5];
        let grid = compute_grid(GridSpec {
            count: 5,
            nrow: None,
            ncol: Some(2),
            width: 1000.0,
            height: None,
            margin: Margins::zero(),
            gap: 0.0,
            rel_widths: &[],
            rel_heights: &[1.0, 1.0, 2.0],
            aspects: &aspects,
        })
        .unwrap();
        assert_close(grid.height, 1600.0);
        assert_close(grid.cells[0].height, 400.0);
        assert_close(grid.cells[2].height, 400.0);
        assert_close(grid.cells[4].height, 800.0);
        assert_close(grid.cells[4].width, 500.0);
    }

    #[test]
    fn rel_widths_are_ratios_and_do_not_change_the_derived_height() {
        // `1,2` and `2,4` are the same layout (ARCHITECTURE/layout.md §1).  The
        // height signal deliberately ignores the multipliers, so scaling every
        // weight must leave the canvas and the cells unchanged.
        let aspects = vec![Some(ASPECT); 4];
        let run = |rel_widths: &[f32]| {
            compute_grid(GridSpec {
                count: 4,
                nrow: None,
                ncol: Some(2),
                width: 1000.0,
                height: None,
                margin: Margins::zero(),
                gap: 0.0,
                rel_widths,
                rel_heights: &[],
                aspects: &aspects,
            })
            .unwrap()
        };
        let unit = run(&[1.0, 2.0]);
        let doubled = run(&[2.0, 4.0]);
        // The absolute geometry is pinned as well: the multipliers may only
        // distribute the width, never feed the height signal.
        assert_close(unit.height, 800.0);
        assert_close(unit.cells[0].width, 1000.0 / 3.0);
        assert_close(unit.cells[1].width, 2000.0 / 3.0);
        assert_close(unit.cells[0].height, 400.0);
        assert_close(doubled.height, unit.height);
        for (left, right) in unit.cells.iter().zip(&doubled.cells) {
            assert_close(left.x, right.x);
            assert_close(left.width, right.width);
            assert_close(left.height, right.height);
        }
    }

    #[test]
    fn default_grid_is_the_near_square_cowplot_one() {
        // `cols = ceil(sqrt(n))`, `rows = ceil(n / cols)`; the last row may be short.
        for (count, expected) in [
            (1, (1, 1)),
            (2, (1, 2)),
            (3, (2, 2)),
            (4, (2, 2)),
            (5, (2, 3)),
            (6, (2, 3)),
            (7, (3, 3)),
            (8, (3, 3)),
            (9, (3, 3)),
            (10, (3, 4)),
            (11, (3, 4)),
            (12, (3, 4)),
            (13, (4, 4)),
            (14, (4, 4)),
            (15, (4, 4)),
            (16, (4, 4)),
            (17, (4, 5)),
        ] {
            assert_eq!(default_grid(count), expected, "count {count}");
        }
    }

    #[test]
    fn default_grid_inherits_the_d2_row_geometry() {
        // 5 panels, no --ncol/--nrow → 2×3: equal 266.667-high rows, 333.333-wide cells.
        let five = fit_default(5);
        assert_close(five.height, 1600.0 / 3.0);
        assert_eq!(five.cells.len(), 5);
        for index in 0..5 {
            assert_close(five.cells[index].width, 1000.0 / 3.0);
            assert_close(five.cells[index].height, (1000.0 / 3.0) / ASPECT);
            assert_close(five.cells[index].height, 266.666_67);
            let row = index / 3;
            let column = index % 3;
            assert_close(five.cells[index].x, column as f32 * 1000.0 / 3.0);
            assert_close(five.cells[index].y, row as f32 * 266.666_67);
        }

        // 15 panels → 4×4: the full grid stays exactly as D2 pinned it.
        let fifteen = fit_default(15);
        assert_close(fifteen.height, 800.0);
        for cell in &fifteen.cells {
            assert_close(cell.width, 250.0);
            assert_close(cell.height, 200.0);
        }

        // 1 and 2 panels keep their old one-row geometry byte-for-byte in shape.
        let one = fit_default(1);
        assert_close(one.height, 800.0);
        assert_close(one.cells[0].width, 1000.0);
        assert_close(one.cells[0].height, 800.0);
        let two = fit_default(2);
        assert_close(two.height, 400.0);
        for cell in &two.cells {
            assert_close(cell.width, 500.0);
            assert_close(cell.height, 400.0);
        }
    }
}
