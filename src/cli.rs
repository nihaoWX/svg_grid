//! The command line: parse → execute → exit code (`ARCHITECTURE/cli.md`).
//!
//! `main.rs` only calls [`main`]. The help text lives in [`help`], the parser in
//! [`parse`] (pure, unit-tested), the execution in [`execute`] (`plot-grid`), in
//! [`doctor`] (the corpus self-check) and in [`convert`] (the thin shell over the
//! `svg_grid_convert` library).
//!
//! Errors are typed, not flattened: [`CliError::Usage`] is "you typed it wrong"
//! (exit 2), [`CliError::Config`] is "something is not installed where it is
//! looked for" (exit 3), [`CliError::Failure`] is "the run finished and is red"
//! (exit 1), and [`CliError::SvgGrid`] keeps the library's own exit-code tier
//! (1 = runtime failure, 2 = invalid input, 3 = configuration).

use std::fmt;
use std::process::ExitCode;

mod convert;
mod doctor;
mod execute;
mod help;
mod parse;
#[cfg(test)]
mod tests;

use parse::Invocation;

/// A command line that could not be run, and the exit code it earns.
#[derive(Debug)]
pub(crate) enum CliError {
    /// Bad usage: unknown flag, missing value, out-of-range value, missing output.
    /// Carries the one-line message (the `--help` hint is appended on display).
    Usage(String),
    /// A **configuration** error: something the run needs is not installed or not
    /// where it is looked for (the fixture corpus). Exit 3, like an empty font
    /// library: no input can fix it.
    Config(String),
    /// The run completed and its verdict is "not green" (`doctor`: at least one
    /// fixture FAILED). Exit 1.
    Failure(String),
    /// Anything the library reports; its own tier is preserved.
    SvgGrid(svg_grid::SvgGridError),
}

impl CliError {
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        CliError::Usage(message.into())
    }

    pub(crate) fn config(message: impl Into<String>) -> Self {
        CliError::Config(message.into())
    }

    pub(crate) fn failure(message: impl Into<String>) -> Self {
        CliError::Failure(message.into())
    }

    /// `ARCHITECTURE/cli.md` §退出码: 1 runtime, 2 usage/input, 3 configuration.
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            CliError::Usage(_) => 2,
            CliError::Config(_) => 3,
            CliError::Failure(_) => 1,
            CliError::SvgGrid(error) => error.exit_code(),
        }
    }

    /// The message printed to stderr. Usage errors add the `--help` pointer so the
    /// caller is never left guessing; the whole thing stays one line.
    pub(crate) fn message(&self) -> String {
        match self {
            CliError::Usage(message) => {
                format!("{message} (run `svg_grid --help` for usage)")
            }
            CliError::Config(message) | CliError::Failure(message) => message.clone(),
            CliError::SvgGrid(error) => error.to_string(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl From<svg_grid::SvgGridError> for CliError {
    fn from(error: svg_grid::SvgGridError) -> Self {
        CliError::SvgGrid(error)
    }
}

/// The binary's entry point: parse the process arguments and run them.
pub(crate) fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    run(args)
}

/// Run one command line. `--help` / `--version` print to **stdout** and succeed.
fn run(args: Vec<String>) -> ExitCode {
    let invocation = match parse::parse(&args) {
        Ok(invocation) => invocation,
        Err(error) => return report(&error),
    };

    let result = match invocation {
        Invocation::Help => {
            print!("{}", help::HELP);
            Ok(())
        }
        Invocation::Version => {
            println!("svg_grid {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Invocation::PlotGrid(args) => execute::run(*args),
        Invocation::Doctor(args) => doctor::execute(*args),
        // The subcommand is a thin shell: the converter library prints its own
        // messages and returns its own exit code, so nothing is re-formatted here
        // (that is what keeps `svg_grid convert` byte-identical to the standalone
        // `svg_grid_convert`).
        Invocation::Convert(rest) => return convert::run(rest),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}

/// Print `error` on stderr and turn it into its exit code.
fn report(error: &CliError) -> ExitCode {
    eprintln!("svg_grid: {}", error.message());
    ExitCode::from(error.exit_code())
}
