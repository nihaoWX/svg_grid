//! The `doctor` integration tests: a 3-fixture corpus, run end to end.
//!
//! Each test prints a SKIP line with its reason when the environment cannot support
//! it (no font library) — never a silent pass. The converter needs no such check: it
//! is linked into the binary under test.

use crate::common::harness::{
    corpus, doctor, doctor_with_empty_path, font_library_available, summary, svg_grid,
    verdict_lines, Binaries,
};

/// The binary under test.
fn binaries() -> Binaries {
    Binaries::discover()
}

/// A binary **and** a font library, or a printed SKIP reason.
fn ready() -> Option<Binaries> {
    let bins = binaries();
    if !font_library_available() {
        eprintln!(
            "SKIP  doctor integration tests — no font library (run fontlib/setup.sh, or set \
             $SVG_GRID_FONT_DIR); a PNG cannot be rendered without glyphs"
        );
        return None;
    }
    Some(bins)
}

fn path_of(dir: &std::path::Path) -> String {
    dir.to_str().expect("a temporary path is UTF-8").to_string()
}

#[test]
fn a_clean_corpus_passes_with_one_line_per_fixture() {
    let Some(bins) = ready() else { return };
    let dir = corpus("clean", false);
    let report = doctor(&bins, &["--fixtures", &path_of(&dir)]);
    assert_eq!(report.code, 0, "stderr:\n{}", report.stderr);

    let lines = verdict_lines(&report.stdout);
    assert_eq!(lines.len(), 3, "one line per fixture:\n{}", report.stdout);
    assert!(
        lines[0].starts_with("PASS  ggplot2/ok  both  "),
        "the first line must be a full PASS line: {}",
        lines[0]
    );
    assert!(
        lines[0].contains("  warnings="),
        "the line carries the warning count: {}",
        lines[0]
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("SKIP  matplotlib/rotated_text  convert")),
        "the unsupported fixture is a SKIP:\n{}",
        report.stdout
    );
    assert!(
        report
            .stdout
            .contains("why: <text> under a rotated ancestor"),
        "a SKIP must print its reason:\n{}",
        report.stdout
    );
    assert_eq!(summary(&report.stdout), "summary: 2 pass, 0 fail, 1 skip");
}

#[test]
fn a_dangling_reference_fails_the_fixture_and_the_run() {
    let Some(bins) = ready() else { return };
    let dir = corpus("dangling", true);
    let report = doctor(&bins, &["--fixtures", &path_of(&dir)]);
    assert_eq!(
        report.code, 1,
        "a FAIL must make doctor exit 1:\n{}\n{}",
        report.stdout, report.stderr
    );

    let lines = verdict_lines(&report.stdout);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("FAIL  matplotlib/dangling  check")),
        "the dangling fixture must fail at the check step:\n{}",
        report.stdout
    );
    assert!(
        report.stdout.contains("failures:"),
        "the failing fixtures are repeated at the end:\n{}",
        report.stdout
    );
    assert!(
        report.stdout.contains("W9"),
        "the criterion names W9:\n{}",
        report.stdout
    );
    assert!(
        report.stdout.contains("url(#nope)"),
        "the key stderr line is repeated:\n{}",
        report.stdout
    );
    assert_eq!(summary(&report.stdout), "summary: 2 pass, 1 fail, 1 skip");
    assert!(
        report.stderr.contains("1 of 4 fixture(s) FAILED"),
        "stderr carries the one-line verdict:\n{}",
        report.stderr
    );
}

#[test]
fn only_runs_the_matching_fixtures() {
    let Some(bins) = ready() else { return };
    let dir = corpus("only", true);

    let report = doctor(
        &bins,
        &["--fixtures", &path_of(&dir), "--only", "ggplot2/*"],
    );
    assert_eq!(report.code, 0, "stderr:\n{}", report.stderr);
    let lines = verdict_lines(&report.stdout);
    assert_eq!(lines.len(), 2, "{}", report.stdout);
    assert!(lines.iter().all(|line| line.contains("ggplot2/")));
    assert_eq!(summary(&report.stdout), "summary: 2 pass, 0 fail, 0 skip");

    let report = doctor(
        &bins,
        &[
            "--fixtures",
            &path_of(&dir),
            "--only",
            "matplotlib/rotated_*",
        ],
    );
    assert_eq!(report.code, 0, "stderr:\n{}", report.stderr);
    let lines = verdict_lines(&report.stdout);
    assert_eq!(lines.len(), 1, "{}", report.stdout);
    assert!(lines[0].starts_with("SKIP  matplotlib/rotated_text"));
    assert_eq!(summary(&report.stdout), "summary: 0 pass, 0 fail, 1 skip");

    // A glob that matches nothing is a usage error, not a silent "0 fixtures".
    let report = doctor(&bins, &["--fixtures", &path_of(&dir), "--only", "nope/*"]);
    assert_eq!(report.code, 2, "stderr:\n{}", report.stderr);
    assert!(report.stderr.contains("matched none"), "{}", report.stderr);
}

#[test]
fn json_is_machine_readable_and_complete() {
    let Some(bins) = ready() else { return };
    let dir = corpus("json", true);
    let report = doctor(&bins, &["--fixtures", &path_of(&dir), "--json"]);
    assert_eq!(report.code, 1, "stderr:\n{}", report.stderr);

    let json = report.stdout.trim();
    assert!(json.starts_with('{'), "stdout must be JSON only:\n{json}");
    assert!(json.ends_with('}'), "stdout must be JSON only:\n{json}");
    for needle in [
        "\"fixtures_dir\": ",
        "\"convert\": ",
        "\"jobs\": 1",
        "\"only\": null",
        "\"total\": 4",
        "\"summary\": {\"pass\": 2, \"fail\": 1, \"skip\": 1}",
        "\"status\": \"fail\"",
        "\"status\": \"skip\"",
        "\"step\": \"check\"",
        "\"ink_pixels\": ",
        "\"warning_codes\": [\"W9\"]",
        "\"why\": \"<text> under a rotated ancestor",
        "\"stderr\": [\"svg_grid: warning: 1 reference(s)",
    ] {
        assert!(json.contains(needle), "missing {needle} in\n{json}");
    }
    assert_eq!(
        json.matches("\"id\": ").count(),
        4,
        "every fixture is listed"
    );
    assert_eq!(json.matches("\"warning_codes\": ").count(), 4);
    assert_eq!(json.matches("\"ink_pixels\": ").count(), 4);
    assert_eq!(json.matches("\"ms\": ").count(), 4);
    assert_eq!(
        json.matches('{').count(),
        json.matches('}').count(),
        "unbalanced braces:\n{json}"
    );
    assert_eq!(
        json.matches('[').count(),
        json.matches(']').count(),
        "unbalanced brackets:\n{json}"
    );
}

#[test]
fn keep_leaves_the_intermediates_behind() {
    let Some(bins) = ready() else { return };
    let dir = corpus("keep", false);
    let keep = crate::common::harness::temp_dir("keep-out");
    let report = doctor(
        &bins,
        &["--fixtures", &path_of(&dir), "--keep", &path_of(&keep)],
    );
    assert_eq!(report.code, 0, "stderr:\n{}", report.stderr);
    for stem in ["ggplot2_ok", "ggplot2_ok2"] {
        for suffix in ["tagged.svg", "svg", "png"] {
            let file = keep.join(format!("{stem}.{suffix}"));
            assert!(
                file.is_file() && std::fs::metadata(&file).unwrap().len() > 0,
                "--keep must keep {file:?}"
            );
        }
    }
    assert!(
        !keep.join("matplotlib_rotated_text.png").exists(),
        "a SKIPped fixture produces no PNG"
    );
}

#[test]
fn jobs_do_not_change_the_verdict() {
    let Some(bins) = ready() else { return };
    let dir = corpus("jobs", true);
    let serial = doctor(&bins, &["--fixtures", &path_of(&dir), "--jobs", "1"]);
    let parallel = doctor(&bins, &["--fixtures", &path_of(&dir), "--jobs", "4"]);
    assert_eq!(serial.code, parallel.code);
    assert_eq!(summary(&serial.stdout), summary(&parallel.stdout));
    // The millisecond column differs; the verdict, step and warning count do not.
    let shape = |stdout: &str| -> Vec<String> {
        verdict_lines(stdout)
            .iter()
            .map(|line| {
                let fields: Vec<&str> = line.split_whitespace().collect();
                format!("{} {} {} {}", fields[0], fields[1], fields[2], fields[4])
            })
            .collect()
    };
    assert_eq!(shape(&serial.stdout), shape(&parallel.stdout));
    assert_eq!(
        verdict_lines(&parallel.stdout).len(),
        4,
        "order is preserved"
    );
}

/// The first stage is a **library** call: with `PATH` reduced to a directory that
/// holds nothing at all, `doctor` still runs the whole corpus. There is no
/// `svg_grid_convert` to find, so there is no exit-3 "first stage missing" path
/// either (`ARCHITECTURE/cli.md`, R18).
#[test]
fn the_first_stage_needs_no_external_binary() {
    let Some(bins) = ready() else { return };
    let dir = corpus("no-convert", false);
    let report = doctor_with_empty_path(&bins, &["--fixtures", &path_of(&dir)]);
    assert_eq!(
        report.code, 0,
        "an empty PATH must not stop doctor:\n{}\n{}",
        report.stdout, report.stderr
    );
    assert_eq!(summary(&report.stdout), "summary: 2 pass, 0 fail, 1 skip");
    assert!(
        report.stdout.contains("convert: in-process"),
        "the report says the converter was called in process:\n{}",
        report.stdout
    );
}

#[test]
fn a_missing_corpus_says_how_to_point_at_it() {
    let bins = binaries();
    let report = doctor(&bins, &["--fixtures", "/nonexistent/svg-grid-corpus"]);
    assert_eq!(report.code, 3, "stderr:\n{}", report.stderr);
    assert!(
        report.stderr.contains("/nonexistent/svg-grid-corpus"),
        "the message names the directory:\n{}",
        report.stderr
    );
    assert!(
        report.stderr.contains("--fixtures"),
        "the message says how to point elsewhere:\n{}",
        report.stderr
    );
    assert!(report.stderr.contains("Fix:"), "{}", report.stderr);
}

#[test]
fn help_documents_doctor_and_its_flags() {
    let bins = binaries();
    let top = svg_grid(&bins, &["--help"]);
    assert_eq!(top.code, 0);
    assert_eq!(top.stderr, "", "--help writes to stdout only");
    for needle in [
        "svg_grid doctor",
        "--fixtures",
        "--only",
        "--jobs",
        "--json",
        "--keep",
        "summary: N pass, M fail, K skip",
        "PASS|FAIL|SKIP",
    ] {
        assert!(top.stdout.contains(needle), "--help lacks {needle:?}");
    }

    let sub = svg_grid(&bins, &["doctor", "--help"]);
    assert_eq!(sub.code, 0, "`doctor --help` exits 0");
    assert_eq!(sub.stderr, "");
    assert_eq!(
        sub.stdout, top.stdout,
        "`doctor --help` prints the same help text"
    );
}
