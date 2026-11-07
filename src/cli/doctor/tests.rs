//! Unit tests for `doctor` (`ARCHITECTURE/testing.md`).
//!
//! They are pure: argument parsing, the glob matcher, the manifest reader, corpus
//! discovery, the message texts, the binary lookup against a temporary directory,
//! and the verdict/counting logic. **No fixture corpus and no process spawn** —
//! the end-to-end run against a 3-fixture corpus is `tests/selftest/doctor/`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use super::args::FLAGS;
use super::fixtures;
use super::glob;
use super::run::{Status, Totals};
use super::{
    help_covers_all_flags, no_fixtures_message, repo_fixtures_dir, resolve_fixtures_dir,
    DoctorArgs, CONVERT_STAGE,
};
use crate::cli::parse::Invocation;
use crate::cli::CliError;

/// Parse `doctor` arguments.
fn doctor(args: &[&str]) -> DoctorArgs {
    let mut argv = vec!["doctor".to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    match crate::cli::parse::parse(&argv) {
        Ok(Invocation::Doctor(args)) => *args,
        other => panic!("expected doctor, got {other:?}"),
    }
}

/// A usage error (exit code 2) for `doctor`.
fn usage_error(args: &[&str]) {
    let mut argv = vec!["doctor".to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    match crate::cli::parse::parse(&argv) {
        Err(error) => assert_eq!(error.exit_code(), 2, "usage errors exit 2: {error}"),
        Ok(other) => panic!("expected a usage error, got {other:?}"),
    }
}

/// A fresh temporary directory for one test (no cwd dependency).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "svg-grid-doctor-test-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a temporary directory must be creatable");
    dir
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory");
    }
    fs::write(path, contents).expect("file");
}

#[test]
fn defaults_are_the_documented_ones() {
    let args = doctor(&[]);
    assert_eq!(args.fixtures, None);
    assert_eq!(args.only, None);
    assert_eq!(args.jobs, 1, "the default is one fixture at a time");
    assert!(!args.json);
    assert_eq!(args.keep, None);
}

#[test]
fn every_flag_is_taken_and_stored() {
    let args = doctor(&[
        "--fixtures",
        "/tmp/corpus",
        "--only",
        "ggplot2/*",
        "--jobs",
        "4",
        "--json",
        "--keep",
        "/tmp/keep",
    ]);
    assert_eq!(
        args.fixtures.as_deref().unwrap().to_str(),
        Some("/tmp/corpus")
    );
    assert_eq!(args.only.as_deref(), Some("ggplot2/*"));
    assert_eq!(args.jobs, 4);
    assert!(args.json);
    assert_eq!(args.keep.as_deref().unwrap().to_str(), Some("/tmp/keep"));
}

#[test]
fn out_of_range_and_empty_values_are_usage_errors() {
    for args in [
        vec!["--jobs", "0"],
        vec!["--jobs", "-1"],
        vec!["--jobs", "abc"],
        vec!["--only", ""],
        vec!["--only"],
        vec!["--fixtures"],
        vec!["--keep"],
        vec!["--bogus"],
        vec!["--json", "yes"],
    ] {
        usage_error(&args);
    }
}

/// Every registered flag has a parser arm and is documented: the registry, the
/// parser and `--help` cannot drift apart.
#[test]
fn every_flag_has_a_parser_arm() {
    for (flag, _) in FLAGS {
        let mut argv = vec!["doctor".to_string(), flag.to_string()];
        if *flag != "--json" {
            argv.push("x".to_string());
        }
        if let Err(error) = super::args::parse(&argv[1..]) {
            assert!(
                !error.message().contains("has no parser arm"),
                "{flag} is registered but unimplemented: {}",
                error.message()
            );
        }
    }
}

#[test]
fn help_covers_every_doctor_flag() {
    assert!(help_covers_all_flags(), "--help misses a doctor flag");
    let help = crate::cli::help::HELP;
    for needle in [
        "doctor",
        "svg_grid doctor",
        "--fixtures",
        "--only",
        "--jobs",
        "--json",
        "--keep",
        "summary: N pass, M fail, K skip",
        "PASS|FAIL|SKIP",
    ] {
        assert!(help.contains(needle), "--help lacks {needle:?}");
    }
}

#[test]
fn doctor_help_wins_before_any_value_is_taken() {
    for argv in [
        vec!["doctor", "--help"],
        vec!["doctor", "-h"],
        vec!["doctor", "--fixtures", "/x", "--help"],
        vec!["--fixtures", "--help"],
    ] {
        let argv: Vec<String> = argv.iter().map(|arg| arg.to_string()).collect();
        assert!(
            matches!(crate::cli::parse::parse(&argv), Ok(Invocation::Help)),
            "{argv:?} must print help on stdout and exit 0"
        );
    }
}

#[test]
fn unknown_subcommand_still_points_at_help() {
    let argv: Vec<String> = vec!["doctorr".to_string()];
    let error = crate::cli::parse::parse(&argv).expect_err("no such subcommand");
    assert_eq!(error.exit_code(), 2);
    assert!(error.message().contains("unknown subcommand"));
    assert!(error.message().contains("--help"));
}

#[test]
fn the_exit_code_tiers_are_kept_apart() {
    assert_eq!(CliError::usage("x").exit_code(), 2);
    assert_eq!(
        CliError::config(no_fixtures_message(Path::new("/nope/fixtures"))).exit_code(),
        3,
        "a missing corpus is a configuration error"
    );
    assert_eq!(
        CliError::failure("1 fixture FAILED").exit_code(),
        1,
        "a red doctor run is a runtime failure"
    );
}

/// The first stage is a library call, so there is no "which binary did you run"
/// failure mode left: the report says so instead of printing a path, and the run
/// cannot exit 3 over it (that tier is reserved for the corpus and the fonts).
#[test]
fn the_first_stage_is_in_process_and_has_no_binary_to_miss() {
    assert!(
        CONVERT_STAGE.contains("in-process"),
        "the report line must say the stage is in process: {CONVERT_STAGE}"
    );
    assert!(
        CONVERT_STAGE.contains("svg_grid_convert"),
        "it must still name the converter that ran: {CONVERT_STAGE}"
    );
}

#[test]
fn the_configuration_message_says_fix_and_the_way_out() {
    let corpus = no_fixtures_message(Path::new("/nope/fixtures"));
    assert!(corpus.contains("Fix:"), "{corpus}");
    assert!(corpus.contains("--fixtures"), "{corpus}");
    assert!(corpus.contains("/nope/fixtures"), "{corpus}");
    // The reader must be able to tell which candidate was the wrong one.
    assert!(corpus.contains("SVG_GRID_FIXTURES"), "{corpus}");
    assert!(corpus.contains("svg_grid/fixtures"), "{corpus}");
    assert_eq!(
        corpus.lines().count(),
        1,
        "one sentence, one line: {corpus}"
    );
}

#[test]
fn the_repository_corpus_is_two_levels_up_from_the_binary() {
    let dir = repo_fixtures_dir(Path::new("/repo/tools/svg_grid/target/debug"));
    assert_eq!(
        dir,
        Path::new("/repo/tools/svg_grid/target/debug/../../tests/fixtures")
    );
    // Normalised, that is the repository's corpus.
    assert!(dir.ends_with("tests/fixtures"));
}

#[test]
fn the_corpus_is_resolved_in_the_documented_order() {
    let exe_dir = Path::new("/repo/tools/svg_grid/target/debug");
    let repo = repo_fixtures_dir(exe_dir);
    let home = temp_dir("resolve-order");
    let data_root = home.join("svg_grid").join("fixtures");
    let flag = Path::new("/from/flag");
    let from_env = OsStr::new("/from/env");

    // 4. Nothing installed: the repository's own corpus.
    assert_eq!(
        resolve_fixtures_dir(None, None, Some(home.as_os_str()), exe_dir),
        repo
    );

    // 3. `install.sh` filled the data root: it wins over the repository directory.
    fs::create_dir_all(&data_root).expect("data root");
    assert_eq!(
        resolve_fixtures_dir(None, None, Some(home.as_os_str()), exe_dir),
        data_root
    );
    // An empty value is "not given" — it must not shadow the next candidate.
    assert_eq!(
        resolve_fixtures_dir(None, Some(OsStr::new("")), Some(home.as_os_str()), exe_dir),
        data_root
    );
    // ... and a HOME without a data root falls through to the repository.
    let empty_home = temp_dir("resolve-order-empty-home");
    assert_eq!(
        resolve_fixtures_dir(None, None, Some(empty_home.as_os_str()), exe_dir),
        repo
    );
    assert_eq!(resolve_fixtures_dir(None, None, None, exe_dir), repo);

    // 2. `$SVG_GRID_FIXTURES` outranks the data root and is taken as given.
    assert_eq!(
        resolve_fixtures_dir(None, Some(from_env), Some(home.as_os_str()), exe_dir),
        Path::new("/from/env")
    );

    // 1. `--fixtures` outranks everything else, likewise unexamined.
    assert_eq!(
        resolve_fixtures_dir(Some(flag), Some(from_env), Some(home.as_os_str()), exe_dir),
        flag
    );
}

/// A three-fixture corpus: two expected to pass (one of which is registered as
/// unsupported) plus one file the manifest does not mention.
fn corpus(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    write(
        &dir.join("MANIFEST.toml"),
        "# Corpus for the unit tests.\n\
         [[fixture]]\n\
         id = \"ggplot2/ok\"\n\
         package = \"ggplot2\"\n\
         expect = \"pass\"\n\
         \n\
         [[fixture]]\n\
         id = \"matplotlib/bad\"\n\
         package = \"matplotlib\"\n\
         expect = \"pass\"\n\
         \n\
         [[fixture]]\n\
         # A rotated ancestor is fail-closed in the converter.\n\
         id = \"matplotlib/mathtext\"\n\
         expect = \"unsupported\"\n\
         why = \"<text> under a rotated ancestor\"\n",
    );
    write(
        &dir.join("ggplot2/ok.svg"),
        "<svg width=\"10pt\" height=\"10pt\"/>",
    );
    write(
        &dir.join("matplotlib/bad.svg"),
        "<svg width=\"10pt\" height=\"10pt\"/>",
    );
    write(
        &dir.join("matplotlib/mathtext.svg"),
        "<svg width=\"10pt\" height=\"10pt\"/>",
    );
    write(
        &dir.join("seaborn/extra.svg"),
        "<svg width=\"10pt\" height=\"10pt\"/>",
    );
    dir
}

#[test]
fn the_corpus_keeps_manifest_order_and_appends_unregistered_files() {
    let dir = corpus("corpus-order");
    let loaded = fixtures::load(&dir).expect("the corpus must load");
    let ids: Vec<&str> = loaded.iter().map(|fixture| fixture.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "ggplot2/ok",
            "matplotlib/bad",
            "matplotlib/mathtext",
            "seaborn/extra"
        ],
        "manifest order first, then files the manifest forgot"
    );
    assert_eq!(loaded[0].path, dir.join("ggplot2/ok.svg"));
    assert_eq!(loaded[0].expect, "pass");
    assert_eq!(
        loaded[3].expect, "pass",
        "an unregistered file is expected to pass"
    );
}

#[test]
fn only_keeps_the_matching_fixtures() {
    let dir = corpus("corpus-only");
    let loaded = fixtures::load(&dir).expect("the corpus must load");
    let selected = |pattern: &str| -> Vec<String> {
        loaded
            .iter()
            .filter(|fixture| glob::matches(pattern, &fixture.id))
            .map(|fixture| fixture.id.clone())
            .collect()
    };
    assert_eq!(
        selected("matplotlib/*"),
        vec!["matplotlib/bad", "matplotlib/mathtext"]
    );
    assert_eq!(selected("ggplot2/*"), vec!["ggplot2/ok"]);
    assert_eq!(selected("*mathtext"), vec!["matplotlib/mathtext"]);
    assert!(selected("ggplot2").is_empty(), "matching is anchored");
}

#[test]
fn an_unsupported_fixture_is_a_skip_with_a_printed_reason() {
    let dir = corpus("corpus-skip");
    let loaded = fixtures::load(&dir).expect("the corpus must load");
    let unsupported: Vec<&fixtures::Fixture> = loaded
        .iter()
        .filter(|fixture| fixture.unsupported())
        .collect();
    assert_eq!(unsupported.len(), 1);
    assert_eq!(unsupported[0].id, "matplotlib/mathtext");
    assert_eq!(
        unsupported[0].skip_reason(),
        "<text> under a rotated ancestor"
    );

    // A manifest that forgets the `why` still produces a reason (never silence).
    let dir = temp_dir("corpus-skip-no-why");
    write(
        &dir.join("MANIFEST.toml"),
        "[[fixture]]\nid = \"x/y\"\nexpect = \"unsupported\"\n",
    );
    write(&dir.join("x/y.svg"), "<svg width=\"1pt\" height=\"1pt\"/>");
    let loaded = fixtures::load(&dir).expect("the corpus must load");
    assert!(loaded[0].skip_reason().contains("no `why`"));
}

#[test]
fn the_counts_and_the_status_words_are_the_contract() {
    assert_eq!(Status::Pass.as_str(), "PASS");
    assert_eq!(Status::Fail.as_str(), "FAIL");
    assert_eq!(Status::Skip.as_str(), "SKIP");
    assert_eq!(Status::Pass.as_json(), "pass");

    let totals = Totals {
        pass: 56,
        fail: 1,
        skip: 1,
    };
    assert_eq!(totals.total(), 58);
    assert_eq!(Totals::default().total(), 0);
}

#[test]
fn a_corpus_without_a_manifest_still_runs() {
    let dir = temp_dir("corpus-no-manifest");
    write(&dir.join("a.svg"), "<svg width=\"1pt\" height=\"1pt\"/>");
    write(&dir.join("b.svg"), "<svg width=\"1pt\" height=\"1pt\"/>");
    let loaded = fixtures::load(&dir).expect("a manifest is optional");
    assert_eq!(loaded.len(), 2, "the note is printed, the files are run");
    assert!(loaded.iter().all(|fixture| fixture.expect == "pass"));
}

use super::run::{classify, judge, number, read_stderr_lines, StageResult};

fn stage(convert_rc: i32, plot_rc: i32, ink: Result<u64, String>, stderr: &str) -> StageResult {
    StageResult {
        convert_rc,
        plot_rc,
        tagged_bytes: 100,
        svg_bytes: 100,
        png_bytes: 100,
        ink,
        stderr: stderr.to_string(),
    }
}

#[test]
fn a_clean_run_has_no_problems() {
    let problems = judge(&stage(0, 0, Ok(1234), ""));
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_blank_png_is_a_failure() {
    let problems = judge(&stage(0, 0, Ok(0), ""));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("no ink"), "{problems:?}");
}

#[test]
fn w7_and_w9_fail_the_fixture_but_w1_does_not() {
    let w1 = "svg_grid: warning: 1 input(s) are scaled by the layout (|sx-1| or |sy-1| > 0.5%): \
              #0 sx=0.77778 sy=0.66667 (off by 33.33%).";
    assert!(judge(&stage(0, 0, Ok(10), w1)).is_empty());

    let w9 = "svg_grid: warning: 1 reference(s) point at an id that nothing in the output \
              defines: input #0 <rect fill> -> url(#nope).";
    let problems = judge(&stage(0, 0, Ok(10), w9));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("W9"), "{problems:?}");

    let w7 = "svg_grid: warning: 1 attribute(s) could not be rewritten and were left exactly as \
              written: input #0 <path d> (malformed).";
    let problems = judge(&stage(0, 0, Ok(10), w7));
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("W7"), "{problems:?}");
}

#[test]
fn every_broken_stage_is_named() {
    let problems = judge(&stage(1, 2, Err("nope".to_string()), ""));
    assert!(problems.iter().any(|problem| problem.contains("rc=1")));
    assert!(problems.iter().any(|problem| problem.contains("rc=2")));
    assert!(problems
        .iter()
        .any(|problem| problem.contains("unreadable")));
}

#[test]
fn the_tally_line_is_not_counted_as_a_warning() {
    let stderr = "svg_grid: warning: 1 input is scaled by the layout: #0 sx=1.50000\n\
                  svg_grid: note: font-size is never rescaled\n\
                  svg_grid: warning: non-fatal warning(s) — the exit code and the output \
                  file are unchanged\n";
    let (codes, warnings) = classify(stderr);
    assert_eq!(warnings, 1);
    assert_eq!(codes, vec!["W1"]);
}

#[test]
fn the_converter_warning_is_counted_with_its_own_code() {
    let stderr =
        "svg_grid convert: warning: <style> CSS block is not parsed, so lengths/font-size \
                  there are not rescaled\n";
    let (codes, warnings) = classify(stderr);
    assert_eq!(warnings, 1);
    assert_eq!(codes, vec!["V1"]);
}

#[test]
fn stderr_lines_are_deduplicated_and_capped() {
    let plot = (0..12)
        .map(|index| format!("svg_grid: warning: line {index}\n"))
        .collect::<String>();
    let lines = read_stderr_lines("", &plot);
    assert_eq!(
        lines.len(),
        8,
        "a flooding fixture must not flood the report"
    );
    assert_eq!(lines[0], "svg_grid: warning: line 0");
}

#[test]
fn dimensions_print_as_plot_grid_accepts_them() {
    assert_eq!(number(264.0), "264");
    assert_eq!(number(216.5), "216.500");
}
