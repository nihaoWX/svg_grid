//! Thin entry point: `main()` and nothing else (`ARCHITECTURE/cli.md`).
//!
//! The command line — parsing, help text, execution, exit codes — lives in
//! `src/cli.rs`.

mod cli;

fn main() -> std::process::ExitCode {
    cli::main()
}
