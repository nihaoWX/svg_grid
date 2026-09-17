//! `svg_grid_convert` CLI: argument parsing, reporting and file orchestration.
//!
//! ```text
//! svg_grid_convert --input PANEL.svg --output PANEL.tagged.svg [--report] [--force]
//! svg_grid_convert --input PANEL.svg --check [--report]
//! ```
//!
//! Exit codes: `0` success/compatible, `1` incompatible input or error.
//! Errors go to stderr; non-fatal warnings also go to stderr.
//!
//! An input that already is a *complete* conversion product (exactly one
//! `data-panel-box="main"` rect **and** a `data-scale` grouping) exits `0`; if
//! `--output` was given it is copied verbatim so a batch pipeline still produces
//! the file the next stage reads. A *half-finished* product (a panel box without
//! the `data-scale` grouping) is a hard error unless `--force` is given.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::error::Finding;
use crate::options::{Conversion, Options, Report};
use crate::pipeline::convert;

/// Binary entry point: parse `std::env::args`, map `Err` to exit code 1.
pub fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("svg_grid_convert: {message}");
            ExitCode::FAILURE
        }
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut options = Options::default();

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--input" | "-i" => input = Some(PathBuf::from(take_value(&args, &mut index, &flag)?)),
            "--output" | "-o" => {
                output = Some(PathBuf::from(take_value(&args, &mut index, &flag)?))
            }
            "--check" => options.check = true,
            "--report" => options.report = true,
            "--force" | "-f" => options.force = true,
            "--help" | "-h" => {
                println!("{}", usage());
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}\n{}", usage())),
        }
        index += 1;
    }

    let input = input.ok_or_else(|| format!("missing required --input <path>\n{}", usage()))?;
    if !options.check && output.is_none() {
        return Err(format!(
            "missing required --output <path> (required unless --check is given)\n{}",
            usage()
        ));
    }

    run(&input, output.as_deref(), &options)
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn usage() -> &'static str {
    "usage: svg_grid_convert --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]

  --input  <path>   source SVG produced by R (svglite/ggplot2)        [required]
  --output <path>   destination tagged SVG                            [required unless --check]
  --check           only run the compatibility check and print a report; write nothing
  --report          print the per-node / conversion report to stdout
  --force           re-run conversion even if the input already looks converted
                    (also completes a half-finished product: a panel box without
                    the matching data-scale grouping)

An already-converted input exits 0; with --output <path> it is copied verbatim,
so `convert --input a.svg --output b.svg` always produces b.svg.

exit codes: 0 = success/compatible, 1 = incompatible or error."
}

/// Read `input`, run the conversion, print reports/warnings, and write `output`
/// (unless `--check`). Returns `Err(message)` on any failure (exit code 1).
pub fn run(input: &Path, output: Option<&Path>, options: &Options) -> Result<(), String> {
    let source = std::fs::read_to_string(input)
        .map_err(|error| format!("failed to read {}: {error}", input.display()))?;

    let conversion = convert(&source, options.force).map_err(|error| error.to_string())?;

    match conversion {
        Conversion::AlreadyConverted { canvas } => {
            if options.check {
                println!(
                    "already converted: {} is a complete conversion product",
                    input.display()
                );
                println!(
                    "  verdict: compatible (already converted) - `svg_grid plot-grid` can consume it as-is"
                );
                print_canvas_hint(&canvas);
                return Ok(());
            }

            if let Some(output) = output {
                if output != input {
                    // The next pipeline stage expects `--output` to exist, so
                    // copy the input verbatim instead of silently writing
                    // nothing.
                    std::fs::write(output, &source).map_err(|error| {
                        format!("failed to write {}: {error}", output.display())
                    })?;
                    println!(
                        "svg_grid_convert: {} is already converted; copied it verbatim to {}",
                        input.display(),
                        output.display()
                    );
                } else {
                    println!(
                        "svg_grid_convert: {} is already converted; --output is the same file, nothing written",
                        input.display()
                    );
                }
            } else {
                println!(
                    "svg_grid_convert: {} is already converted (nothing to do)",
                    input.display()
                );
            }
            if options.report {
                print_canvas_hint(&canvas);
            }
            Ok(())
        }
        Conversion::Incompatible(findings) => {
            if options.check || options.report {
                print_incompat_report(&findings);
            }
            Err(format!(
                "{} is not compatible with `svg_grid plot-grid`; {} problem(s); no output written",
                input.display(),
                findings.len()
            ))
        }
        Conversion::Converted(done) => {
            for warning in &done.warnings {
                eprintln!("svg_grid_convert: warning: {warning}");
            }
            if options.check {
                println!(
                    "compatible: {} can be consumed by `svg_grid plot-grid`",
                    input.display()
                );
                println!("  canvas box:  {}", done.report.canvas);
                println!("  text marks:  {}", done.report.texts);
                println!("  point marks: {}", done.report.circles);
                print_canvas_hint(&done.report.canvas);
                return Ok(());
            }

            let output = output.ok_or_else(|| {
                "missing --output <path> (required unless --check is given)".to_string()
            })?;
            std::fs::write(output, &done.svg)
                .map_err(|error| format!("failed to write {}: {error}", output.display()))?;

            if options.report {
                print_report(&done.report, output);
                print_canvas_hint(&done.report.canvas);
            }
            Ok(())
        }
    }
}

fn print_incompat_report(findings: &[Finding]) {
    println!(
        "compatibility report: NOT compatible ({} problem(s))",
        findings.len()
    );
    for finding in findings {
        println!(
            "  - <{}> #{} at {}: {}",
            finding.tag, finding.ordinal, finding.path, finding.reason
        );
    }
}

fn print_report(report: &Report, output: &Path) {
    println!("converted -> {}", output.display());
    println!("  canvas box:  {}", report.canvas);
    println!("  panel box:   {}", report.panel_box);
    println!("  text marks wrapped in position: {}", report.texts);
    println!("  point marks wrapped in position: {}", report.circles);
    let primitives: Vec<String> = report
        .primitives
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect();
    println!("  primitives:  {}", primitives.join(", "));
}

/// Print the source canvas size plus the registration advice: `font-size`, point
/// radius and `stroke-width` never scale under `position`, so the downstream cell
/// must be close to the source canvas size.
fn print_canvas_hint(canvas: &str) {
    println!(
        "  hint: source canvas is {canvas} (x y width height); compose into a cell close to this size, \
         otherwise text/point proportions change - `font-size`, point radius and `stroke-width` are never \
         rescaled under `data-scale=\"position\"`"
    );
}
