//! `svg_grid_convert` binary entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    svg_grid_convert::cli::main()
}
