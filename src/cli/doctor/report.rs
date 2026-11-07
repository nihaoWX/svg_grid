//! The two reports: one line per fixture, and the `--json` twin for agents/CI.
//!
//! The text form is fixed by `ARCHITECTURE/testing.md` §4:
//!
//! ```text
//! PASS|FAIL|SKIP  <id>  <step>  <ms>  warnings=N
//! summary: N pass, M fail, K skip
//! ```
//!
//! After the summary, every FAIL is repeated with its concrete criteria and the
//! stderr lines that explain them — the point of the subcommand is that a red run
//! says what to fix, not only that something is red.
//!
//! Everything here is English (`ARCHITECTURE/warnings.md` §1 rule 4).

use std::path::Path;

use super::run::{Outcome, Status, Totals};

/// One report, ready to print.
pub(crate) struct Report<'a> {
    pub fixtures_dir: &'a Path,
    /// What the first stage is — a description, not a path: the converter is called
    /// in process, so there is no executable to name (`super::CONVERT_STAGE`).
    pub convert: &'a str,
    pub jobs: usize,
    pub only: Option<&'a str>,
    pub outcomes: &'a [Outcome],
    pub totals: Totals,
}

/// The human-readable report.
pub(crate) fn text(report: &Report<'_>) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "svg_grid doctor: {} fixture(s) in {}\n",
        report.outcomes.len(),
        report.fixtures_dir.display()
    ));
    out.push_str(&format!("convert: {}\n", report.convert));
    match report.only {
        Some(only) => out.push_str(&format!("only: {only}  jobs: {}\n", report.jobs)),
        None => out.push_str(&format!("jobs: {}\n", report.jobs)),
    }

    for outcome in report.outcomes {
        out.push_str(&format!(
            "{:<5} {}  {}  {}  warnings={}\n",
            outcome.status.as_str(),
            outcome.id,
            outcome.step,
            outcome.ms,
            outcome.warnings
        ));
        if outcome.status == Status::Skip {
            if let Some(why) = &outcome.why {
                out.push_str(&format!("      why: {why}\n"));
            }
        }
    }

    out.push_str(&format!(
        "\nsummary: {} pass, {} fail, {} skip\n",
        report.totals.pass, report.totals.fail, report.totals.skip
    ));

    let failures: Vec<&Outcome> = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.status == Status::Fail)
        .collect();
    if !failures.is_empty() {
        out.push_str("\nfailures:\n");
        for outcome in failures {
            out.push_str(&format!(
                "  {}  (step={}, expect={})\n",
                outcome.id, outcome.step, outcome.expect
            ));
            for problem in &outcome.problems {
                out.push_str(&format!("    - {problem}\n"));
            }
            if !outcome.stderr.is_empty() {
                out.push_str("    stderr:\n");
                for line in &outcome.stderr {
                    out.push_str(&format!("      {line}\n"));
                }
            }
        }
    }

    let skips: Vec<&Outcome> = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.status == Status::Skip)
        .collect();
    if !skips.is_empty() {
        out.push_str("\nskips (registered as unsupported, never silently dropped):\n");
        for outcome in skips {
            out.push_str(&format!(
                "  {}  convert_rc={}\n    why: {}\n",
                outcome.id,
                outcome
                    .convert_rc
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                outcome.why.as_deref().unwrap_or("(none)")
            ));
        }
    }

    out
}

/// The machine-readable report (`--json`).
pub(crate) fn json(report: &Report<'_>) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"fixtures_dir\": {},\n",
        quote(&report.fixtures_dir.to_string_lossy())
    ));
    out.push_str(&format!("  \"convert\": {},\n", quote(report.convert)));
    out.push_str(&format!("  \"jobs\": {},\n", report.jobs));
    out.push_str(&format!(
        "  \"only\": {},\n",
        match report.only {
            Some(only) => quote(only),
            None => "null".to_string(),
        }
    ));
    out.push_str(&format!("  \"total\": {},\n", report.totals.total()));
    out.push_str(&format!(
        "  \"summary\": {{\"pass\": {}, \"fail\": {}, \"skip\": {}}},\n",
        report.totals.pass, report.totals.fail, report.totals.skip
    ));
    out.push_str("  \"fixtures\": [\n");
    for (index, outcome) in report.outcomes.iter().enumerate() {
        out.push_str("    {");
        out.push_str(&format!("\"id\": {}, ", quote(&outcome.id)));
        out.push_str(&format!("\"expect\": {}, ", quote(&outcome.expect)));
        out.push_str(&format!(
            "\"status\": {}, ",
            quote(outcome.status.as_json())
        ));
        out.push_str(&format!("\"step\": {}, ", quote(outcome.step)));
        out.push_str(&format!("\"ms\": {}, ", outcome.ms));
        out.push_str(&format!("\"warnings\": {}, ", outcome.warnings));
        out.push_str(&format!(
            "\"warning_codes\": [{}], ",
            outcome
                .codes
                .iter()
                .map(|code| quote(code))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push_str(&format!(
            "\"convert_rc\": {}, ",
            number_or_null(outcome.convert_rc)
        ));
        out.push_str(&format!(
            "\"plot_rc\": {}, ",
            number_or_null(outcome.plot_rc)
        ));
        out.push_str(&format!("\"tagged_svg_bytes\": {}, ", outcome.tagged_bytes));
        out.push_str(&format!("\"svg_bytes\": {}, ", outcome.svg_bytes));
        out.push_str(&format!("\"png_bytes\": {}, ", outcome.png_bytes));
        out.push_str(&format!(
            "\"ink_pixels\": {}, ",
            outcome
                .ink_pixels
                .map(|pixels| pixels.to_string())
                .unwrap_or_else(|| "null".to_string())
        ));
        out.push_str(&format!(
            "\"why\": {}, ",
            outcome
                .why
                .as_deref()
                .map(quote)
                .unwrap_or_else(|| "null".to_string())
        ));
        out.push_str(&format!(
            "\"problems\": [{}], ",
            outcome
                .problems
                .iter()
                .map(|problem| quote(problem))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push_str(&format!(
            "\"stderr\": [{}]",
            outcome
                .stderr
                .iter()
                .map(|line| quote(line))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push('}');
        if index + 1 < report.outcomes.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

fn number_or_null(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

/// A JSON string literal. Only the characters JSON requires are escaped (plus the
/// control characters), so ids and warning lines stay readable in the output.
fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::{json, quote, text, Report};
    use crate::cli::doctor::run::{Outcome, Status, Totals};
    use std::path::Path;

    fn outcome(id: &str, status: Status, step: &'static str) -> Outcome {
        Outcome {
            id: id.to_string(),
            expect: "pass".to_string(),
            status,
            step,
            ms: 12,
            warnings: 1,
            codes: vec!["W1"],
            convert_rc: Some(0),
            plot_rc: Some(0),
            tagged_bytes: 10,
            svg_bytes: 20,
            png_bytes: 30,
            ink_pixels: Some(40),
            why: None,
            problems: Vec::new(),
            stderr: Vec::new(),
        }
    }

    fn report<'a>(outcomes: &'a [Outcome]) -> Report<'a> {
        Report {
            fixtures_dir: Path::new("/corpus"),
            convert: crate::cli::doctor::CONVERT_STAGE,
            jobs: 1,
            only: None,
            outcomes,
            totals: Totals::of(outcomes),
        }
    }

    #[test]
    fn the_line_format_matches_the_contract() {
        let mut failed = outcome("ggplot2/boxplot", Status::Fail, "check");
        failed.problems = vec!["the composed PNG has no ink".to_string()];
        failed.stderr = vec!["svg_grid: warning: 1 reference(s)".to_string()];
        let outcomes = vec![outcome("ggplot2/annotate", Status::Pass, "both"), failed];
        let text = text(&report(&outcomes));
        assert!(
            text.contains("PASS  ggplot2/annotate  both  12  warnings=1"),
            "{text}"
        );
        assert!(
            text.contains("FAIL  ggplot2/boxplot  check  12  warnings=1"),
            "{text}"
        );
        assert!(text.contains("summary: 1 pass, 1 fail, 0 skip"), "{text}");
        assert!(text.contains("failures:"), "{text}");
        assert!(text.contains("    - the composed PNG has no ink"), "{text}");
        assert!(
            text.contains("      svg_grid: warning: 1 reference(s)"),
            "{text}"
        );
    }

    #[test]
    fn a_skip_prints_its_reason() {
        let mut skip = outcome("matplotlib/mathtext", Status::Skip, "convert");
        skip.expect = "unsupported".to_string();
        skip.why = Some("<text> under a rotated ancestor".to_string());
        let outcomes = vec![skip];
        let text = text(&report(&outcomes));
        assert!(
            text.contains("SKIP  matplotlib/mathtext  convert"),
            "{text}"
        );
        assert!(
            text.contains("why: <text> under a rotated ancestor"),
            "a skip must say why: {text}"
        );
        assert!(text.contains("summary: 0 pass, 0 fail, 1 skip"), "{text}");
    }

    #[test]
    fn json_carries_every_field_the_contract_asks_for() {
        let outcomes = vec![outcome("ggplot2/annotate", Status::Pass, "both")];
        let json = json(&report(&outcomes));
        for needle in [
            "\"fixtures_dir\": \"/corpus\"",
            "\"total\": 1",
            "\"summary\": {\"pass\": 1, \"fail\": 0, \"skip\": 0}",
            "\"id\": \"ggplot2/annotate\"",
            "\"status\": \"pass\"",
            "\"step\": \"both\"",
            "\"ms\": 12",
            "\"warnings\": 1",
            "\"warning_codes\": [\"W1\"]",
            "\"convert_rc\": 0",
            "\"plot_rc\": 0",
            "\"ink_pixels\": 40",
            "\"problems\": []",
            "\"stderr\": []",
        ] {
            assert!(json.contains(needle), "missing {needle} in\n{json}");
        }
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
    fn json_escapes_the_characters_that_would_break_it() {
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote("a\\b"), "\"a\\\\b\"");
        assert_eq!(quote("a\nb"), "\"a\\nb\"");
        assert_eq!(quote("plain"), "\"plain\"");
    }
}
