// Layout unit tests. They live outside `src/layout.rs` so both files stay under
// the 20 KB gate (R8); the module is declared from `src/lib.rs`.

use crate::layout::*;

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
fn a_cell_that_rounds_to_zero_pixels_is_rejected() {
    // `--rel-widths 1e-9,1` is positive, so the old gate accepted it and then
    // laid out a 0x60.8 cell: a panel that can never be drawn. Anything that
    // rounds to 0 px (< 0.5) must be a usage error naming the flag and the raw
    // token, and the message must spell the token the way the user typed it.
    let aspects = vec![Some(ASPECT); 2];
    let run = |rel_widths: &[f32], width: f32| {
        compute_grid(GridSpec {
            count: 2,
            nrow: None,
            ncol: Some(2),
            width,
            height: None,
            margin: Margins::zero(),
            gap: 0.0,
            rel_widths,
            rel_heights: &[],
            aspects: &aspects,
        })
    };
    let error = run(&[1e-9, 1.0], 200.0).unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"invalid input: --rel-widths must leave a cell at least 1 px wide, got "1e-9""#
    );
    // The weight is a multiplier on the column's natural width, so the cell is
    // `200 * r / (r + 1)` px: 1e-3 → 0.2 px (rejected), 3e-3 → 0.6 px (kept).
    assert!(run(&[1e-3, 1.0], 200.0).is_err());
    let ok = run(&[3e-3, 1.0], 200.0).unwrap();
    assert!(
        ok.cells[0].width > 0.5 && ok.cells[0].width < 1.0,
        "a 0.6 px cell is drawable, got {}",
        ok.cells[0].width
    );
    // A tiny canvas collapses the cell even with uniform weights: then the
    // canvas flag is the culprit and is named instead. Wide panels (aspect
    // 0.5) keep the row tall enough that the *width* is what fails.
    let wide = vec![Some(0.5); 2];
    let error = compute_grid(GridSpec {
        count: 2,
        nrow: None,
        ncol: Some(2),
        width: 0.8,
        height: None,
        margin: Margins::zero(),
        gap: 0.0,
        rel_widths: &[],
        rel_heights: &[],
        aspects: &wide,
    })
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"invalid input: --width must leave a cell at least 1 px wide, got "0.8""#
    );
}

#[test]
fn a_row_that_rounds_to_zero_pixels_is_rejected() {
    let aspects = vec![Some(ASPECT); 2];
    let run = |rel_heights: &[f32], height: Option<f32>| {
        compute_grid(GridSpec {
            count: 2,
            nrow: None,
            ncol: Some(1),
            width: 1000.0,
            height,
            margin: Margins::zero(),
            gap: 0.0,
            rel_widths: &[],
            rel_heights,
            aspects: &aspects,
        })
    };
    let error = run(&[1e-9, 1.0], Some(100.0)).unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"invalid input: --rel-heights must leave a cell at least 1 px tall, got "1e-9""#
    );
    let error = run(&[], Some(0.4)).unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"invalid input: --height must leave a cell at least 1 px tall, got "0.4""#
    );
    // The natural (height-less) layout of a 1000-wide canvas is never degenerate.
    assert!(run(&[], None).is_ok());
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
