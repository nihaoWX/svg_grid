// Degenerate cells: a grid that would round a cell down to 0 px (R66b P3-4).

use std::fs;

use super::grid_args;
use crate::common::*;

#[test]
fn a_cell_that_collapses_to_zero_pixels_is_rejected_by_run_and_plan() {
    // `--rel-widths 1e-9,1` is positive, so it used to be accepted and then laid
    // out a 0x60.8 cell (R66 P3-4): a panel that can never be drawn. After parsing
    // a cell narrower than 0.5 px must be a usage error naming the flag and the raw
    // token, from both the real run and `--plan`.
    let panels = write_panels(&["collapse_0", "collapse_1"]);
    let extra = ["--rel-widths", "1e-9,1"];

    let out = temp_path("collapse_out.svg");
    let run = run_plot_grid(&grid_args(&panels, &extra, Some(&out), false));
    assert_eq!(
        run.status.code(),
        Some(2),
        "a collapsed cell must be a usage error, stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!out.exists(), "an invalid grid must write nothing");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains(r#"--rel-widths must leave a cell at least 1 px wide, got "1e-9""#),
        "the error must name the flag and the raw token: {stderr}"
    );

    let plan = run_plot_grid(&grid_args(&panels, &extra, None, true));
    assert_eq!(
        plan.status.code(),
        Some(2),
        "--plan must reject it too, stderr: {}",
        String::from_utf8_lossy(&plan.stderr)
    );
    assert_eq!(
        plan.stderr, run.stderr,
        "--plan and the real run must report the same grid error"
    );

    // 1e-3 on a 1000-wide canvas is ≈1 px, which is drawable: still accepted.
    let fine = run_plot_grid(&grid_args(
        &panels,
        &["--rel-widths", "1e-3,1"],
        Some(&out),
        false,
    ));
    assert!(
        fine.status.success(),
        "a 1 px cell must stay accepted, stderr: {}",
        String::from_utf8_lossy(&fine.stderr)
    );
    assert!(out.exists());

    let _ = fs::remove_file(&out);
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}
