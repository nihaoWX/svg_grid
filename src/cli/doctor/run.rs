//! Running the corpus: the two stages, the verdict, and the `--jobs` pool.
//!
//! One fixture is one *complete* pipeline run, exactly as a user would run it
//! (`ARCHITECTURE/testing.md` §`svg_grid doctor`):
//!
//! ```text
//! <converter, in process>  <fixture.svg> -> <work>/tagged.svg
//! svg_grid plot-grid --input <work>/tagged.svg --output <work>/fig.svg \
//!           --output-png <work>/fig.png --png-dpi 300 \
//!           --width <natural width + 2*margin> --height <natural height + 2*margin>
//! ```
//!
//! The first stage is a **library call**, not a subprocess: the converter is linked
//! into this binary (`ARCHITECTURE/cli.md` §子命令与参数面), so `doctor` needs nothing
//! on `PATH` and cannot fail because a second executable was not installed. It runs
//! the same code and produces the same text — the warnings are captured into a
//! buffer instead of the terminal, so W7/W9 are judged on exactly what a user would
//! have seen on stderr.
//!
//! `--plot-margin` is deliberately **not** passed: with the canvas grown by the
//! default margin on both sides, each cell is exactly the panel's natural size
//! (`sx = sy = 1`), so no fixture trips W1 merely because `doctor` resized it. A
//! fixture that really is rescaled by the layout still prints its W1, and is
//! counted — not failed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use super::fixtures::Fixture;
use super::natural;
use super::{ink, DEFAULT_MARGIN};

/// One run's fixed pieces: which binary, and where the intermediates go.
pub(crate) struct Context {
    /// The `svg_grid` executable, re-entered for `plot-grid` (stage 2). The
    /// converter needs no entry here: it is this very process.
    pub svg_grid: PathBuf,
    pub work: PathBuf,
}

/// The verdict of one fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    /// The word the report prints (`PASS` / `FAIL` / `SKIP`).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Skip => "SKIP",
        }
    }

    /// The lowercase word `--json` prints.
    pub(crate) fn as_json(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

/// Everything the report needs about one fixture.
#[derive(Debug)]
pub(crate) struct Outcome {
    pub id: String,
    pub expect: String,
    pub status: Status,
    /// `read` | `convert` | `plot` | `check` for a failure; `both` for a pass;
    /// `convert` for a skip (that is where an unsupported fixture stops).
    pub step: &'static str,
    pub ms: u64,
    /// Warning lines on the two stages' stderr (`W1`–`W9` plus the converter's).
    pub warnings: usize,
    /// The warning codes seen, deduplicated and in warning order.
    pub codes: Vec<&'static str>,
    pub convert_rc: Option<i32>,
    pub plot_rc: Option<i32>,
    pub tagged_bytes: u64,
    pub svg_bytes: u64,
    pub png_bytes: u64,
    /// `None` = stage 2 did not run (a skip).
    pub ink_pixels: Option<u64>,
    /// Why an `unsupported` fixture is skipped (printed verbatim).
    pub why: Option<String>,
    /// The failed criteria, in the order they were judged.
    pub problems: Vec<String>,
    /// The key stderr lines, for the failure (`FAIL`) / skip (`SKIP`) sections.
    pub stderr: Vec<String>,
}

/// The counts of the summary line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Totals {
    pub pass: usize,
    pub fail: usize,
    pub skip: usize,
}

impl Totals {
    pub(crate) fn of(outcomes: &[Outcome]) -> Self {
        let mut totals = Totals::default();
        for outcome in outcomes {
            match outcome.status {
                Status::Pass => totals.pass += 1,
                Status::Fail => totals.fail += 1,
                Status::Skip => totals.skip += 1,
            }
        }
        totals
    }

    pub(crate) fn total(&self) -> usize {
        self.pass + self.fail + self.skip
    }
}

/// Run every fixture, at most `jobs` at a time, and return the verdicts **in
/// fixture order** (so a `--jobs 4` report reads like a `--jobs 1` report).
pub(crate) fn run_all(context: &Context, fixtures: &[Fixture], jobs: usize) -> Vec<Outcome> {
    let jobs = jobs.max(1).min(fixtures.len().max(1));
    if jobs == 1 {
        return fixtures
            .iter()
            .map(|fixture| run_one(context, fixture))
            .collect();
    }

    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Outcome>>> =
        Mutex::new((0..fixtures.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(fixture) = fixtures.get(index) else {
                    break;
                };
                let outcome = run_one(context, fixture);
                if let Ok(mut slots) = results.lock() {
                    slots[index] = Some(outcome);
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect()
}

/// The two stages of one fixture, then the verdict.
pub(crate) fn run_one(context: &Context, fixture: &Fixture) -> Outcome {
    let started = Instant::now();
    let stem = fixture.id.replace(['/', '\\'], "_");
    let tagged = context.work.join(format!("{stem}.tagged.svg"));
    let out_svg = context.work.join(format!("{stem}.svg"));
    let out_png = context.work.join(format!("{stem}.png"));

    let mut outcome = Outcome {
        id: fixture.id.clone(),
        expect: fixture.expect.clone(),
        status: Status::Fail,
        step: "read",
        ms: 0,
        warnings: 0,
        codes: Vec::new(),
        convert_rc: None,
        plot_rc: None,
        tagged_bytes: 0,
        svg_bytes: 0,
        png_bytes: 0,
        ink_pixels: None,
        why: None,
        problems: Vec::new(),
        stderr: Vec::new(),
    };

    // The natural size of the SOURCE file: it decides the canvas, so a fixture
    // without one cannot be composed at its own size and is a failed criterion.
    let size = match read_size(&fixture.path) {
        Ok(size) => size,
        Err(problem) => {
            outcome.problems.push(problem);
            return finish(outcome, started);
        }
    };

    // Stage 1: dialect SVG → tag protocol, **in process**. `run_in` is the exact
    // function the `convert` subcommand and the standalone binary use; only the
    // streams differ (two buffers here), so `convert_stderr` is the text a user
    // would have seen and the W-code classification below is unchanged.
    let (convert_rc, convert_stderr) = convert_in_process(&fixture.path, &tagged);
    outcome.convert_rc = Some(convert_rc);
    outcome.tagged_bytes = file_len(&tagged);
    let (codes, warnings) = classify(&convert_stderr);
    outcome.codes = codes;
    outcome.warnings = warnings;

    // An `unsupported` fixture is a SKIP, not a failure: the converter is expected
    // to refuse it, and the manifest explains why. It never runs stage 2.
    if fixture.unsupported() {
        outcome.status = Status::Skip;
        outcome.step = "convert";
        outcome.why = Some(fixture.skip_reason());
        outcome.stderr = read_stderr_lines(&convert_stderr, "");
        return finish(outcome, started);
    }

    // A converter that failed on a fixture the manifest expects to pass stops here:
    // there is no tagged SVG to compose, and pretending otherwise would only add
    // "cannot read the input" noise to the real diagnosis.
    if outcome.convert_rc != Some(0) {
        outcome.step = "convert";
        outcome.problems.push(format!(
            "stage 1 (svg_grid_convert) exited with rc={}",
            outcome.convert_rc.unwrap_or(-1)
        ));
        if outcome.tagged_bytes == 0 {
            outcome
                .problems
                .push("the converted SVG is empty".to_string());
        }
        outcome.stderr = read_stderr_lines(&convert_stderr, "");
        return finish(outcome, started);
    }

    // Stage 2: compose + rasterise at the fixture's own size.
    let (width, height) = (
        size.width + 2.0 * DEFAULT_MARGIN,
        size.height + 2.0 * DEFAULT_MARGIN,
    );
    let plot_stderr = match run_stage(
        &context.svg_grid,
        &[
            "plot-grid",
            "--input",
            &tagged.to_string_lossy(),
            "--output",
            &out_svg.to_string_lossy(),
            "--output-png",
            &out_png.to_string_lossy(),
            "--png-dpi",
            "300",
            "--width",
            &number(width),
            "--height",
            &number(height),
        ],
    ) {
        Ok(output) => {
            outcome.plot_rc = Some(exit_code(&output));
            String::from_utf8_lossy(&output.stderr).into_owned()
        }
        Err(problem) => {
            outcome.problems.push(problem);
            outcome.step = "plot";
            return finish(outcome, started);
        }
    };

    let stage = StageResult {
        convert_rc: outcome.convert_rc.unwrap_or(-1),
        plot_rc: outcome.plot_rc.unwrap_or(-1),
        tagged_bytes: outcome.tagged_bytes,
        svg_bytes: file_len(&out_svg),
        png_bytes: file_len(&out_png),
        ink: ink::ink_pixels(&out_png).map_err(|error| error.to_string()),
        stderr: format!("{convert_stderr}{plot_stderr}"),
    };
    outcome.svg_bytes = stage.svg_bytes;
    outcome.png_bytes = stage.png_bytes;
    outcome.ink_pixels = stage.ink.clone().ok();
    let (codes, warnings) = classify(&stage.stderr);
    outcome.codes = codes;
    outcome.warnings = warnings;

    let problems = judge(&stage);
    outcome.step = step_of(&stage);
    outcome.status = if problems.is_empty() {
        outcome.step = "both";
        Status::Pass
    } else {
        Status::Fail
    };
    outcome.problems = problems;
    if outcome.status != Status::Pass {
        outcome.stderr = read_stderr_lines(&convert_stderr, &plot_stderr);
    }
    finish(outcome, started)
}

/// Stamp the elapsed time once, at every exit of [`run_one`].
fn finish(mut outcome: Outcome, started: Instant) -> Outcome {
    outcome.ms = elapsed_ms(started);
    outcome
}

/// The quantities the verdict is made of.
pub(crate) struct StageResult {
    pub convert_rc: i32,
    pub plot_rc: i32,
    pub tagged_bytes: u64,
    pub svg_bytes: u64,
    pub png_bytes: u64,
    /// Non-white pixels of the produced PNG; `Err` = it could not be decoded.
    pub ink: Result<u64, String>,
    /// Both stages' stderr, concatenated.
    pub stderr: String,
}

/// The judging function: every criterion of `ARCHITECTURE/testing.md` §3.
///
/// It is a pure function over the run's numbers and stderr, so the criteria can be
/// unit-tested without spawning a binary.
pub(crate) fn judge(stage: &StageResult) -> Vec<String> {
    let mut problems = Vec::new();
    if stage.convert_rc != 0 {
        problems.push(format!(
            "stage 1 (svg_grid_convert) exited with rc={}",
            stage.convert_rc
        ));
    }
    if stage.plot_rc != 0 {
        problems.push(format!(
            "stage 2 (plot-grid) exited with rc={}",
            stage.plot_rc
        ));
    }
    if stage.tagged_bytes == 0 {
        problems.push("the converted SVG is empty".to_string());
    }
    if stage.svg_bytes == 0 {
        problems.push("the composed SVG is empty".to_string());
    }
    if stage.png_bytes == 0 {
        problems.push("the composed PNG is empty".to_string());
    }
    match &stage.ink {
        Err(error) => problems.push(format!("the composed PNG is unreadable: {error}")),
        Ok(0) => problems.push(
            "the composed PNG has no ink: every pixel is pure white (nothing was painted)"
                .to_string(),
        ),
        Ok(_) => {}
    }
    let (codes, _) = classify(&stage.stderr);
    if codes.contains(&"W7") {
        problems.push(
            "stderr contains W7 (an attribute could not be rewritten and was kept verbatim)"
                .to_string(),
        );
    }
    if codes.contains(&"W9") {
        problems.push(
            "stderr contains W9 (a url(#…) reference points at an id nothing defines)".to_string(),
        );
    }
    problems
}

/// Which step a failure happened at.
fn step_of(stage: &StageResult) -> &'static str {
    if stage.convert_rc != 0 {
        "convert"
    } else if stage.plot_rc != 0 {
        "plot"
    } else {
        "check"
    }
}

/// Warning codes recognised on stderr.
///
/// The ids are the stable interface of `ARCHITECTURE/warnings.md` §3; the phrases
/// are the ones the messages themselves use, so a reworded message is caught by
/// `every_warning_code_still_matches_its_message` in the unit tests.
pub(crate) const WARNING_CODES: &[(&str, &str)] = &[
    ("W1", "scaled by the layout"),
    ("W2", "stretched: "),
    ("W3", "already contain panel letters"),
    ("W4a", "`--labels` has "),
    ("W4b", "repeats some letters"),
    ("W5", "may be clipped by the canvas"),
    ("W6", "draws ink outside its own canvas box"),
    (
        "W7",
        "could not be rewritten and were left exactly as written",
    ),
    ("W8", "is not in the font library"),
    ("W9", "point at an id that nothing in the output defines"),
    ("V1", "<style> CSS block"),
];

/// Split stderr into (warning codes, warning count).
///
/// The uniform tally line (`N non-fatal warning(s) — …`) is a footer, not a
/// warning, so it is not counted.
pub(crate) fn classify(stderr: &str) -> (Vec<&'static str>, usize) {
    let mut codes: Vec<&'static str> = Vec::new();
    let mut warnings = 0;
    for line in stderr.lines() {
        if !is_warning_line(line) {
            continue;
        }
        warnings += 1;
        if let Some((code, _)) = WARNING_CODES
            .iter()
            .find(|(_, phrase)| line.contains(phrase))
        {
            if !codes.contains(code) {
                codes.push(code);
            }
        }
    }
    (codes, warnings)
}

fn is_warning_line(line: &str) -> bool {
    (line.starts_with("svg_grid: warning:") || line.starts_with("svg_grid convert: warning:"))
        && !line.contains("non-fatal warning(s)")
}

/// The stderr lines worth repeating under a FAIL/SKIP: warnings first (they are
/// the diagnosis), then anything else, deduplicated, capped so one misbehaving
/// fixture cannot flood the report.
pub(super) fn read_stderr_lines(convert_stderr: &str, plot_stderr: &str) -> Vec<String> {
    const MAX_LINES: usize = 8;
    const MAX_CHARS: usize = 600;
    let mut lines: Vec<String> = Vec::new();
    for stderr in [convert_stderr, plot_stderr] {
        for line in stderr.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("non-fatal warning(s)") {
                continue;
            }
            let mut line = line.to_string();
            if line.chars().count() > MAX_CHARS {
                line = line.chars().take(MAX_CHARS).collect::<String>() + " …";
            }
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    lines.truncate(MAX_LINES);
    lines
}

/// Stage 1, in process: the converter library with its messages captured.
///
/// Returns `(exit code, stderr)` where the code is exactly what the standalone
/// binary would have exited with (`0` success/already converted, `1` otherwise) and
/// the text is exactly what it would have written to stderr — warnings included.
fn convert_in_process(input: &Path, output: &Path) -> (i32, String) {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let options = svg_grid_convert::Options::default();
    let result = svg_grid_convert::cli::run_in(
        super::super::convert::SUBCOMMAND,
        input,
        Some(output),
        &options,
        &mut out,
        &mut err,
    );
    let code = match result {
        Ok(()) => 0,
        Err(_) => 1,
    };
    (code, String::from_utf8_lossy(&err).into_owned())
}

/// Run one binary and capture its output. A spawn failure (a missing binary, no
/// execute permission) becomes a criterion, never a panic.
fn run_stage(program: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("cannot start {}: {error}", program.display()))
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

/// A canvas dimension as `plot-grid` expects it: an integer when it is one.
pub(super) fn number(value: f32) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.3}")
    }
}

/// Read a fixture's natural size, mapping the two failure modes onto one sentence.
fn read_size(path: &Path) -> Result<natural::Size, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read the fixture {}: {error}", path.display()))?;
    natural::natural_size(&text).map_err(|error| {
        format!(
            "cannot determine the natural size of {}: {error}",
            path.display()
        )
    })
}
