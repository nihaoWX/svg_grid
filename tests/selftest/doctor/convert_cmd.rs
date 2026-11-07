//! `svg_grid convert …`: the converter as a subcommand of the one binary.
//!
//! These run the real `svg_grid` executable, so the process-level exit codes and
//! the files it writes are exercised. The *conversion* itself is covered by the
//! converter crate's own `tests/cli.rs` (it drives `svg_grid_convert`); what is
//! asserted here is the seam — the subcommand reaches the same library, writes the
//! same file, keeps the `0`/`1` exit codes, and prefixes its diagnostics with its
//! own name (`svg_grid convert:`, not the standalone binary's `svg_grid_convert:`).

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::common::harness::{temp_dir, write, Binaries};

/// A dialect SVG the converter rewrites (svglite's `<style>` block, a text mark and
/// a circle: enough to produce both a report and a W-code warning).
const SVGLITE_PANEL: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' width='288pt' height='216pt' viewBox='0 0 288 216'>
<defs><style type='text/css'><![CDATA[ .svglite circle { fill: none; } ]]></style></defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; fill: #CFA15A;'/>
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</svg>"#;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn convert(bins: &Binaries, args: &[&str]) -> Run {
    let output = Command::new(&bins.svg_grid)
        .arg("convert")
        .args(args)
        .output()
        .expect("the svg_grid binary must be runnable");
    Run {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn path_of(path: &Path) -> String {
    path.to_str()
        .expect("a temporary path is UTF-8")
        .to_string()
}

#[test]
fn the_subcommand_writes_the_tagged_svg() {
    let bins = Binaries::discover();
    let dir = temp_dir("convert-write");
    let input = dir.join("panel.svg");
    let output = dir.join("tagged.svg");
    write(&input, SVGLITE_PANEL);

    let run = convert(
        &bins,
        &["--input", &path_of(&input), "--output", &path_of(&output)],
    );
    assert_eq!(run.code, 0, "stderr:\n{}", run.stderr);
    let tagged = fs::read_to_string(&output).expect("the tagged SVG must exist");
    assert!(
        tagged.contains("data-panel-box=\"main\""),
        "the panel box is tagged:\n{tagged}"
    );
    assert!(
        tagged.contains("data-scale"),
        "the scale groups are tagged:\n{tagged}"
    );
}

#[test]
fn check_reports_without_writing_and_exits_zero() {
    let bins = Binaries::discover();
    let dir = temp_dir("convert-check");
    let input = dir.join("panel.svg");
    let output = dir.join("never.svg");
    write(&input, SVGLITE_PANEL);

    let run = convert(
        &bins,
        &[
            "--input",
            &path_of(&input),
            "--output",
            &path_of(&output),
            "--check",
        ],
    );
    assert_eq!(run.code, 0, "stderr:\n{}", run.stderr);
    assert!(
        run.stdout
            .contains("can be consumed by `svg_grid plot-grid`"),
        "stdout:\n{}",
        run.stdout
    );
    assert!(!output.exists(), "--check writes nothing");
}

/// The diagnostics name the **caller**: `svg_grid convert:` for the subcommand, so
/// the line points at a command the reader can run (the standalone binary says
/// `svg_grid_convert:` — asserted in the converter crate's `tests/cli.rs`). The exit
/// code stays the converter's one bit: 1, not the CLI's 2/3 tiers.
#[test]
fn the_diagnostics_name_the_subcommand_and_keep_the_one_bit_exit_code() {
    let bins = Binaries::discover();
    let dir = temp_dir("convert-errors");
    let input = dir.join("panel.svg");
    write(&input, SVGLITE_PANEL);

    // A bad flag: exit 1 (not 2) and the subcommand's own prefix.
    let run = convert(&bins, &["--bogus"]);
    assert_eq!(run.code, 1, "stdout:\n{}", run.stdout);
    assert!(
        run.stderr.starts_with("svg_grid convert: "),
        "stderr:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("unknown argument: --bogus"),
        "stderr:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("usage: svg_grid convert --input"),
        "the usage block names the same command:\n{}",
        run.stderr
    );

    // No --input at all: the same.
    let run = convert(&bins, &[]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("missing required --input"),
        "stderr:\n{}",
        run.stderr
    );

    // A converter refusal is exit 1 too, and it names the path it rejected.
    let run = convert(
        &bins,
        &["--input", &path_of(&dir.join("nope.svg")), "--check"],
    );
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("failed to read"),
        "stderr:\n{}",
        run.stderr
    );
}

#[test]
fn help_wins_over_the_subcommand_and_documents_it() {
    let bins = Binaries::discover();
    let output = Command::new(&bins.svg_grid)
        .args(["convert", "--help"])
        .output()
        .expect("the svg_grid binary must be runnable");
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&output.stdout);
    for needle in ["svg_grid convert --input", "--check", "--report", "--force"] {
        assert!(help.contains(needle), "--help lacks {needle:?}");
    }
    assert!(
        output.stderr.is_empty(),
        "--help writes to stdout only: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
