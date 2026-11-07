//! `svg_grid doctor`: run the whole fixture corpus through the real two-stage
//! pipeline and give a verdict (`ARCHITECTURE/testing.md` §`svg_grid doctor`).
//!
//! The subcommand is deliberately thin around one idea: **the corpus is the test
//! set**, so the fastest way to know whether a change broke real figures is to run
//! every registered fixture through
//!
//! 1. the converter — `svg_grid_convert`'s library, called **in process** (there is
//!    no second binary to find, and no exit-3 "first stage missing" failure mode);
//! 2. `plot-grid --input <tmp>/tagged.svg --output <tmp>/fig.svg --output-png
//!    <tmp>/fig.png --png-dpi 300`
//!
//! and report one line per fixture. The verdict is mechanical (see [`run::judge`]):
//! both stages succeed, both outputs are non-empty, the PNG actually carries ink,
//! and stderr holds neither W7 nor W9. Everything else is only counted.
//!
//! Module layout (each file stays small):
//!
//! * [`args`] — the flag surface (`--fixtures`, `--only`, `--jobs`, `--json`, `--keep`).
//! * [`fixtures`] — corpus discovery and the minimal `MANIFEST.toml` reader.
//! * [`glob`] — the `--only` matcher.
//! * [`natural`] — a source SVG's natural width/height, in pt.
//! * [`ink`] — "does the produced PNG carry any ink?" (non-white pixel count).
//! * [`run`] — the two stages, the judging, the `--jobs` pool.
//! * [`report`] — the one-line-per-fixture report and its `--json` twin.

mod args;
mod fixtures;
mod glob;
mod ink;
mod natural;
mod report;
mod run;
#[cfg(test)]
mod tests;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use svg_grid::SvgGridError;

use super::CliError;
pub(crate) use args::{parse, DoctorArgs};
use fixtures::Fixture;
use report::Report;
use run::Totals;

/// `plot-grid`'s own default `--plot-margin` (one number, all four sides), in pt.
///
/// `doctor` never passes `--plot-margin`: the corpus is composed at each
/// fixture's natural size so nobody trips W1 for no reason, and the canvas is
/// grown by exactly this margin (`--width = panel width + 2 × margin`), which
/// makes the cell exactly the panel's size (`sx = sy = 1`).
const DEFAULT_MARGIN: f32 = 24.0;

/// What the report says the first stage is. It is a **library** call now, so there
/// is no path to print — naming the crate keeps the line informative.
pub(crate) const CONVERT_STAGE: &str = "in-process (svg_grid_convert library)";

/// Run `doctor`.
pub(crate) fn execute(args: DoctorArgs) -> Result<(), CliError> {
    let exe = std::env::current_exe().map_err(|error| CliError::from(SvgGridError::from(error)))?;
    let exe_dir = exe
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let env_fixtures = std::env::var_os(FIXTURES_ENV);
    // `HOME`, else `USERPROFILE` on Windows (the resolution lives in the library so the
    // font library and the corpus can never disagree about where the data root is).
    let home = svg_grid::home::home_dir();
    let fixtures_dir = resolve_fixtures_dir(
        args.fixtures.as_deref(),
        env_fixtures.as_deref(),
        home.as_deref().map(Path::as_os_str),
        &exe_dir,
    );
    if !fixtures_dir.is_dir() {
        return Err(CliError::config(no_fixtures_message(&fixtures_dir)));
    }

    let registered = fixtures::load(&fixtures_dir)?;
    let total_registered = registered.len();
    let selected: Vec<Fixture> = match &args.only {
        None => registered,
        Some(pattern) => {
            let hits: Vec<Fixture> = registered
                .into_iter()
                .filter(|fixture| glob::matches(pattern, &fixture.id))
                .collect();
            if hits.is_empty() {
                return Err(CliError::usage(format!(
                    "--only {pattern:?} matched none of the {total_registered} fixture(s) in {}",
                    fixtures_dir.display()
                )));
            }
            hits
        }
    };

    let (work, temporary) = work_dir(args.keep.as_deref())?;
    let context = run::Context {
        svg_grid: exe,
        work: work.clone(),
    };
    let outcomes = run::run_all(&context, &selected, args.jobs);
    let totals = Totals::of(&outcomes);

    let report = Report {
        fixtures_dir: &fixtures_dir,
        convert: CONVERT_STAGE,
        jobs: args.jobs,
        only: args.only.as_deref(),
        outcomes: &outcomes,
        totals,
    };
    if args.json {
        print!("{}", report::json(&report));
    } else {
        print!("{}", report::text(&report));
    }

    if temporary {
        let _ = fs::remove_dir_all(&work);
    }

    if totals.fail > 0 {
        return Err(CliError::failure(format!(
            "{} of {} fixture(s) FAILED — the failing criteria and their stderr lines are listed above",
            totals.fail,
            totals.total()
        )));
    }
    Ok(())
}

/// Whether `--help` documents every flag `doctor` parses.
///
/// It lives next to the registry so the test and the text cannot drift: a flag
/// added without a `--help` line fails `help_covers_every_doctor_flag`.
#[cfg(test)]
pub(crate) fn help_covers_all_flags() -> bool {
    args::FLAGS
        .iter()
        .all(|(flag, _)| super::help::HELP.contains(flag))
}

/// `<exe dir>/../../tests/fixtures` — from a `target/debug/` build that is the
/// repository's `tests/fixtures/`; from an installed `~/.local/bin` it is
/// `<prefix>/../tests/fixtures`, which is why it is only the last candidate.
fn repo_fixtures_dir(exe_dir: &Path) -> PathBuf {
    exe_dir.join("..").join("..").join("tests").join("fixtures")
}

/// The environment variable that names the corpus (outranked only by `--fixtures`).
const FIXTURES_ENV: &str = "SVG_GRID_FIXTURES";

/// The installed data root: `install.sh` fills `$HOME/svg_grid/` (corpus and fonts).
const DATA_ROOT: &str = "svg_grid";

/// Where the corpus is, in the documented order (`ARCHITECTURE/testing.md` §语料目录):
///
/// 1. `--fixtures <dir>`
/// 2. `$SVG_GRID_FIXTURES`
/// 3. the data root `$HOME/svg_grid/fixtures` (what `install.sh` syncs there)
/// 4. the repository's `<exe dir>/../../tests/fixtures`
///
/// The first two are *explicit*: they are taken as given, so a mistyped path is a
/// configuration error instead of a silent run of some other corpus. The last two
/// are *candidates*: the first that is a directory wins. Nothing else is probed —
/// a copy of the corpus lying next to some other directory must not be picked up.
///
/// Every input is a parameter (no environment reads) so the order is unit-testable.
fn resolve_fixtures_dir(
    flag: Option<&Path>,
    env_fixtures: Option<&OsStr>,
    home: Option<&OsStr>,
    exe_dir: &Path,
) -> PathBuf {
    if let Some(dir) = flag {
        return dir.to_path_buf();
    }
    if let Some(dir) = env_fixtures.filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    let data_root = home
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(DATA_ROOT).join("fixtures"))
        .filter(|dir| dir.is_dir());
    data_root.unwrap_or_else(|| repo_fixtures_dir(exe_dir))
}

/// The message for a fixture directory that is not there. One sentence (the order it
/// tried, so the reader can tell *which* candidate was wrong), then `Fix:` with the
/// way out (`ARCHITECTURE/warnings.md` §1 rule 1).
fn no_fixtures_message(dir: &Path) -> String {
    format!(
        "there is no fixture directory at {} — doctor takes the corpus from --fixtures <dir>, then \
         $SVG_GRID_FIXTURES, then the data root <home>/svg_grid/fixtures (HOME, or %USERPROFILE% on \
         Windows), then the repository's <exe dir>/../../tests/fixtures. Fix: on Linux/macOS run \
         tools/svg_grid/install.sh (it syncs the corpus into <home>/svg_grid/fixtures); on Windows \
         copy tests\\fixtures there (see docs/install-windows.md); or pass --fixtures <dir> pointing \
         at the directory holding MANIFEST.toml and the generated *.svg files.",
        dir.display()
    )
}

/// Where the intermediate files go. `--keep <dir>` keeps them (and is created on
/// demand); otherwise a per-run directory under the system temp dir is used and
/// removed at the end.
fn work_dir(keep: Option<&Path>) -> Result<(PathBuf, bool), CliError> {
    match keep {
        Some(dir) => {
            fs::create_dir_all(dir).map_err(|error| {
                CliError::from(SvgGridError::at(
                    dir,
                    std::io::Error::other(format!("cannot create the --keep directory: {error}")),
                ))
            })?;
            Ok((dir.to_path_buf(), false))
        }
        None => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir()
                .join(format!("svg_grid-doctor-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&dir).map_err(|error| SvgGridError::at(&dir, error))?;
            Ok((dir, true))
        }
    }
}
