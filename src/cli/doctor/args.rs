//! `doctor`'s flag surface (`ARCHITECTURE/testing.md` §`svg_grid doctor`).
//!
//! Like the `plot-grid` parser, this is a pure function over the arguments: no
//! environment, no cwd, no file system, so every branch is unit-testable.

use std::path::PathBuf;

use crate::cli::parse::{count, take_value};
use crate::cli::CliError;

/// Every flag `doctor` accepts: canonical spelling, then aliases.
///
/// It is the single registry: the parser resolves aliases through it and the
/// `--help` coverage test walks it, so a new flag cannot be added without either
/// documenting it or failing that test.
pub(crate) const FLAGS: &[(&str, &[&str])] = &[
    ("--fixtures", &[]),
    ("--only", &[]),
    ("--jobs", &[]),
    ("--json", &[]),
    ("--keep", &[]),
];

/// Flags that never take a value.
const BARE_FLAGS: &[&str] = &["--json"];

/// Everything `doctor` accepts, with the documented defaults.
#[derive(Debug)]
pub(crate) struct DoctorArgs {
    /// Corpus directory; `None` → `$SVG_GRID_FIXTURES`, then
    /// `$HOME/svg_grid/fixtures`, then `<exe dir>/../../tests/fixtures`
    /// (see `super::resolve_fixtures_dir`).
    pub fixtures: Option<PathBuf>,
    /// Only run fixtures whose id matches this glob.
    pub only: Option<String>,
    /// How many fixtures to run at once (default 1: the report stays readable and
    /// a serial run is the one that reproduces a failure).
    pub jobs: usize,
    /// Machine-readable report on stdout instead of the one-line-per-fixture one.
    pub json: bool,
    /// Keep the intermediate tagged/output files in this directory.
    pub keep: Option<PathBuf>,
}

impl Default for DoctorArgs {
    fn default() -> Self {
        Self {
            fixtures: None,
            only: None,
            jobs: 1,
            json: false,
            keep: None,
        }
    }
}

/// Parse the arguments that follow `doctor`.
pub(crate) fn parse(rest: &[String]) -> Result<DoctorArgs, CliError> {
    let mut args = DoctorArgs::default();
    let mut index = 0;

    while index < rest.len() {
        let raw = rest[index].as_str();
        let Some(flag) = canonical(raw) else {
            return Err(CliError::usage(format!("unknown flag {raw:?} for doctor")));
        };
        if BARE_FLAGS.contains(&flag) {
            args.json = true;
            index += 1;
            continue;
        }
        let value = take_value(rest, index, flag)?;
        match flag {
            "--fixtures" => args.fixtures = Some(PathBuf::from(value)),
            "--only" => {
                if value.trim().is_empty() {
                    return Err(CliError::usage(
                        "--only must be a non-empty glob (e.g. --only 'ggplot2/*')".to_string(),
                    ));
                }
                args.only = Some(value.to_string());
            }
            "--jobs" => args.jobs = count(flag, value)?,
            "--keep" => args.keep = Some(PathBuf::from(value)),
            // Not reachable: `every_flag_has_a_parser_arm` proves that every entry
            // of `FLAGS` has an arm here, and an unknown token never gets this far.
            other => unreachable!("doctor flag {other} has no parser arm"),
        }
        index += 2;
    }

    Ok(args)
}

/// The canonical spelling of `flag`, if `doctor` knows it (aliases included).
fn canonical(flag: &str) -> Option<&'static str> {
    FLAGS
        .iter()
        .find(|(name, aliases)| *name == flag || aliases.contains(&flag))
        .map(|(name, _)| *name)
}
