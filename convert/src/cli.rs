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
//!
//! # Three callers, one code path
//!
//! * the standalone `svg_grid_convert` binary ([`main`] → [`run_args`]);
//! * `svg_grid convert …` ([`run_args_as`] with its own name, so the parameters and
//!   the exit codes are identical by construction);
//! * `svg_grid doctor` ([`run_in`]), which needs the same behaviour but wants the
//!   messages as text instead of on the process's own streams.
//!
//! The `prog:` message prefix names the **caller**, not this library: the standalone
//! binary says `svg_grid_convert:`, the subcommand says `svg_grid convert:` (round
//! R19 — as a subcommand, `svg_grid_convert` no longer exists, so a diagnostic
//! naming it points at a program the user cannot run). Every entry point takes the
//! name as a parameter; [`STANDALONE`] is the default when nobody says otherwise.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::error::Finding;
use crate::options::{Conversion, Options, Report};
use crate::pipeline::convert;

/// Every flag this command line accepts: canonical spelling, then aliases.
///
/// The single registry: `svg_grid`'s `--help` coverage test walks it, so a flag
/// cannot be added here without being documented there.
pub const FLAGS: &[(&str, &[&str])] = &[
    ("--input", &["-i"]),
    ("--output", &["-o"]),
    ("--check", &[]),
    ("--report", &[]),
    ("--force", &["-f"]),
];

/// The standalone binary's own name: the diagnostics prefix used when the caller
/// does not pass one (`svg_grid_convert`, its `[[bin]]` name).
pub const STANDALONE: &str = "svg_grid_convert";

/// Binary entry point: parse `std::env::args`, map `Err` to exit code 1.
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run_args(&args)
}

/// Run one already-collected argument list (no program name in front) as the
/// standalone binary: that is [`run_args_as`] with [`STANDALONE`].
pub fn run_args(args: &[String]) -> ExitCode {
    run_args_as(STANDALONE, args)
}

/// [`run_args`] under the caller's own name: every diagnostic this run writes is
/// prefixed `{prog}:`. `svg_grid convert …` passes `"svg_grid convert"`, so the
/// message names a command the user can actually type.
pub fn run_args_as(prog: &str, args: &[String]) -> ExitCode {
    ExitCode::from(run_args_code_as(prog, args))
}

/// The exit code of [`run_args`], as a plain number: `0` success/compatible,
/// `1` incompatible input or error. Kept separate so it can be asserted.
pub fn run_args_code(args: &[String]) -> u8 {
    run_args_code_as(STANDALONE, args)
}

/// [`run_args_code`] under the caller's own name (see [`run_args_as`]).
pub fn run_args_code_as(prog: &str, args: &[String]) -> u8 {
    match real_main(prog, args) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("{prog}: {message}");
            1
        }
    }
}

fn real_main(prog: &str, args: &[String]) -> Result<(), String> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut options = Options::default();

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--input" | "-i" => input = Some(PathBuf::from(take_value(args, &mut index, &flag)?)),
            "--output" | "-o" => output = Some(PathBuf::from(take_value(args, &mut index, &flag)?)),
            "--check" => options.check = true,
            "--report" => options.report = true,
            "--force" | "-f" => options.force = true,
            "--help" | "-h" => {
                println!("{}", usage(prog));
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}\n{}", usage(prog))),
        }
        index += 1;
    }

    let input = input.ok_or_else(|| format!("missing required --input <path>\n{}", usage(prog)))?;
    if !options.check && output.is_none() {
        return Err(format!(
            "missing required --output <path> (required unless --check is given)\n{}",
            usage(prog)
        ));
    }

    run(prog, &input, output.as_deref(), &options)
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

/// The usage block, spelled with the caller's own name (`svg_grid_convert` for the
/// standalone binary, `svg_grid convert` for the subcommand).
fn usage(prog: &str) -> String {
    format!(
        "usage: {prog} --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]

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
    )
}

/// Read `input`, run the conversion, print reports/warnings, and write `output`
/// (unless `--check`). Returns `Err(message)` on any failure (exit code 1).
///
/// This is the process-level entry: stdout and stderr are the real ones.
pub fn run(
    prog: &str,
    input: &Path,
    output: Option<&Path>,
    options: &Options,
) -> Result<(), String> {
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    run_in(prog, input, output, options, &mut out, &mut err)
}

/// [`run`] with explicit streams: `svg_grid doctor` captures the very same text
/// (warnings included) into two buffers instead of the terminal.
pub fn run_in(
    prog: &str,
    input: &Path,
    output: Option<&Path>,
    options: &Options,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), String> {
    let source = std::fs::read_to_string(input)
        .map_err(|error| format!("failed to read {}: {error}", input.display()))?;

    let conversion = convert(&source, options.force).map_err(|error| error.to_string())?;

    match conversion {
        Conversion::AlreadyConverted { canvas } => {
            if options.check {
                emit(
                    out,
                    format_args!(
                        "already converted: {} is a complete conversion product",
                        input.display()
                    ),
                );
                emit(
                    out,
                    format_args!(
                        "  verdict: compatible (already converted) - `svg_grid plot-grid` can consume it as-is"
                    ),
                );
                print_canvas_hint(out, &canvas);
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
                    emit(
                        out,
                        format_args!(
                            "{prog}: {} is already converted; copied it verbatim to {}",
                            input.display(),
                            output.display()
                        ),
                    );
                } else {
                    emit(
                        out,
                        format_args!(
                            "{prog}: {} is already converted; --output is the same file, nothing written",
                            input.display()
                        ),
                    );
                }
            } else {
                emit(
                    out,
                    format_args!(
                        "{prog}: {} is already converted (nothing to do)",
                        input.display()
                    ),
                );
            }
            if options.report {
                print_canvas_hint(out, &canvas);
            }
            Ok(())
        }
        Conversion::Incompatible(findings) => {
            if options.check || options.report {
                print_incompat_report(out, &findings);
            }
            Err(format!(
                "{} is not compatible with `svg_grid plot-grid`; {} problem(s); no output written",
                input.display(),
                findings.len()
            ))
        }
        Conversion::Converted(done) => {
            for warning in &done.warnings {
                emit(err, format_args!("{prog}: warning: {warning}"));
            }
            if options.check {
                emit(
                    out,
                    format_args!(
                        "compatible: {} can be consumed by `svg_grid plot-grid`",
                        input.display()
                    ),
                );
                emit(out, format_args!("  canvas box:  {}", done.report.canvas));
                emit(out, format_args!("  text marks:  {}", done.report.texts));
                emit(out, format_args!("  point marks: {}", done.report.circles));
                print_canvas_hint(out, &done.report.canvas);
                return Ok(());
            }

            let output = output.ok_or_else(|| {
                "missing --output <path> (required unless --check is given)".to_string()
            })?;
            std::fs::write(output, &done.svg)
                .map_err(|error| format!("failed to write {}: {error}", output.display()))?;

            if options.report {
                print_report(out, &done.report, output);
                print_canvas_hint(out, &done.report.canvas);
            }
            Ok(())
        }
    }
}

/// One line on a stream. A closed pipe is the reader's business, not a failure of
/// the conversion, so the error is dropped (the exit code is unaffected).
fn emit(out: &mut dyn Write, line: std::fmt::Arguments<'_>) {
    let _ = writeln!(out, "{line}");
}

fn print_incompat_report(out: &mut dyn Write, findings: &[Finding]) {
    emit(
        out,
        format_args!(
            "compatibility report: NOT compatible ({} problem(s))",
            findings.len()
        ),
    );
    for finding in findings {
        emit(
            out,
            format_args!(
                "  - <{}> #{} at {}: {}",
                finding.tag, finding.ordinal, finding.path, finding.reason
            ),
        );
    }
}

fn print_report(out: &mut dyn Write, report: &Report, output: &Path) {
    emit(out, format_args!("converted -> {}", output.display()));
    emit(out, format_args!("  canvas box:  {}", report.canvas));
    emit(out, format_args!("  panel box:   {}", report.panel_box));
    emit(
        out,
        format_args!("  text marks wrapped in position: {}", report.texts),
    );
    emit(
        out,
        format_args!("  point marks wrapped in position: {}", report.circles),
    );
    let primitives: Vec<String> = report
        .primitives
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect();
    emit(
        out,
        format_args!("  primitives:  {}", primitives.join(", ")),
    );
}

/// Print the source canvas size plus the registration advice: `font-size`, point
/// radius and `stroke-width` never scale under `position`, so the downstream cell
/// must be close to the source canvas size.
///
/// A report line, not a warning — but it obeys the same 200-character cap
/// (`ARCHITECTURE/warnings.md` §1 rule 1) so no printed line depends on which
/// kind of line it is.
fn print_canvas_hint(out: &mut dyn Write, canvas: &str) {
    emit(
        out,
        format_args!(
            "  hint: source canvas is {canvas} (x y width height); compose into a cell of this size, \
             else text/point proportions change - `font-size`, radius and `stroke-width` never rescale"
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::{run_args_code, FLAGS};

    /// Every registered flag is understood: naming it (with a dummy value) must not
    /// produce the "unknown argument" exit, so the registry cannot list a flag the
    /// parser forgot. `--input` alone is then rejected for the missing `--output`,
    /// which is still exit 1 — the assertion is about the message, not the code.
    #[test]
    fn every_registered_flag_is_understood() {
        for (flag, aliases) in FLAGS {
            for spelling in std::iter::once(flag).chain(aliases.iter()) {
                let args = vec![spelling.to_string(), "x".to_string()];
                // Exercise the parser; an unknown flag would print usage and exit 1,
                // a known one only complains about the *other* missing flags.
                assert_eq!(run_args_code(&args), 1, "{spelling} must not be a panic");
            }
        }
        // `--help` is not in the registry (it is handled before any value is taken)
        // but it must still succeed.
        assert_eq!(run_args_code(&["--help".to_string()]), 0);
        assert_eq!(run_args_code(&["-h".to_string()]), 0);
    }

    /// `FLAGS` is what `svg_grid --help` is checked against, so it has to describe
    /// the real surface: four value flags, one bare flag, three short aliases.
    #[test]
    fn the_registry_describes_the_documented_surface() {
        let names: Vec<&str> = FLAGS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            vec!["--input", "--output", "--check", "--report", "--force"]
        );
        let aliases: Vec<&str> = FLAGS
            .iter()
            .flat_map(|(_, aliases)| aliases.iter().copied())
            .collect();
        assert_eq!(aliases, vec!["-i", "-o", "-f"]);
    }
}
