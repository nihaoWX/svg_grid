//! `svg_grid convert`: the dialect converter as a **subcommand**.
//!
//! It is a thin shell and nothing else — parse, call the library, map the exit
//! code (`ARCHITECTURE/cli.md` §子命令与参数面). No conversion logic lives in this
//! crate: `convert/` is still a crate of its own, linked here as a library, so
//! there is one implementation and one installed binary.
//!
//! ```text
//! svg_grid convert --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]
//! ```
//!
//! The parameter surface, the messages and the exit codes are those of the
//! standalone `svg_grid_convert` binary — [`svg_grid_convert::cli::run_args_as`] is
//! literally the function its `main` calls (with [`SUBCOMMAND`] as the program
//! name), so `svg_grid convert …` and `svg_grid_convert …` behave identically.
//! Diagnostics are prefixed with the caller's own name (`svg_grid convert:` here,
//! `svg_grid_convert:` for the standalone binary): the line must name a command the
//! reader can actually type. Exit codes: `0` success / already converted, `1`
//! incompatible input or error (`--help` prints the `svg_grid` help on stdout and
//! exits 0 — the `-h/--help` rule is applied before the subcommand dispatch).

use std::process::ExitCode;

/// How this entry point spells itself in diagnostics (`svg_grid convert: …`).
pub(crate) const SUBCOMMAND: &str = "svg_grid convert";

/// Every flag the subcommand accepts: canonical spelling, then aliases.
///
/// Re-exported from the converter's own registry so `svg_grid --help`'s coverage
/// test (`cli/tests.rs`) walks the real list instead of a copy that can drift. It is
/// only read by that test, hence `cfg(test)`.
#[cfg(test)]
pub(crate) use svg_grid_convert::cli::FLAGS;

/// Run `svg_grid convert <rest>`: hand the tail to the converter's own parser and
/// return its exit code unchanged.
pub(crate) fn run(rest: Vec<String>) -> ExitCode {
    svg_grid_convert::cli::run_args_as(SUBCOMMAND, &rest)
}
