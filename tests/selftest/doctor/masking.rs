//! The R53 gate tests: user content must not be able to steal the W9/W7 verdict.
//!
//! W9 prints the dangling id verbatim and W7 prints the kept attribute value
//! verbatim, so a fixture can carry another code's phrase. Before R53 `classify`
//! stopped at the first table match, and these three fixtures passed `doctor`
//! (`fail-open`). Each corpus holds exactly one fixture, so its `rc` is biting:
//! PASS before the fix, FAIL after it.

use std::path::{Path, PathBuf};

use crate::common::harness::{doctor, font_library_available, temp_dir, write, Binaries};

fn binaries() -> Binaries {
    Binaries::discover()
}

fn ready() -> Option<Binaries> {
    let bins = binaries();
    if !font_library_available() {
        eprintln!(
            "SKIP  masking integration tests — no font library (run fontlib/setup.sh, or set \
             $SVG_GRID_FONT_DIR); a PNG cannot be rendered without glyphs"
        );
        return None;
    }
    Some(bins)
}

fn path_of(dir: &std::path::Path) -> String {
    dir.to_str().expect("a temporary path is UTF-8").to_string()
}

/// One fixture, registered `expect = "pass"`, in a fresh directory.
fn one_fixture(name: &str, id: &str, svg: &str) -> PathBuf {
    let dir = temp_dir(name);
    write(&dir.join(format!("{id}.svg")), svg);
    write(
        &dir.join("MANIFEST.toml"),
        &format!("[[fixture]]\nid = \"{id}\"\npackage = \"ggplot2\"\nexpect = \"pass\"\n"),
    );
    dir
}

/// A tagged panel whose fill is a dangling `url(#…)` with the given id.
fn dangling_panel(id: &str) -> String {
    format!(
        "<svg width=\"100pt\" height=\"80pt\" viewBox=\"0 0 100 80\" \
         xmlns=\"http://www.w3.org/2000/svg\">\n\
         <rect data-panel-box=\"main\" x=\"0\" y=\"0\" width=\"100\" height=\"80\" \
         fill=\"none\" stroke=\"none\"/>\n\
         <g data-scale=\"xy\"><rect x=\"10\" y=\"10\" width=\"40\" height=\"30\" \
         fill=\"url(#{id})\"/></g>\n\
         </svg>\n"
    )
}

/// The W7 panel (`x="inf"` is kept verbatim) plus a path whose `d` carries the
/// `--labels has ` phrase; the path data itself is kept verbatim too.
const W7_MASKED_PANEL: &str = "\
<svg width=\"100pt\" height=\"80pt\" viewBox=\"0 0 100 80\" xmlns=\"http://www.w3.org/2000/svg\">
<rect data-panel-box=\"main\" x=\"0\" y=\"0\" width=\"100\" height=\"80\" fill=\"none\" stroke=\"none\"/>
<g data-scale=\"xy\"><rect x=\"10\" y=\"10\" width=\"40\" height=\"30\" fill=\"#3366cc\"/>
<rect x=\"inf\" y=\"10\" width=\"20\" height=\"20\" fill=\"#000000\"/>
<path d=\"M0 0 --labels has 3\" fill=\"none\" stroke=\"#000000\"/></g>
</svg>
";

/// Assert the whole verdict of a one-fixture corpus: rc, the PASS/FAIL line, the
/// `failures:` entry and the `--json` code set.
fn assert_verdict(dir: &Path, id: &str, codes: &str) {
    let Some(bins) = ready() else { return };
    let report = doctor(&bins, &["--fixtures", &path_of(dir)]);
    assert_eq!(
        report.code, 1,
        "the fixture must FAIL the run (rc=1):\n{}\n{}",
        report.stdout, report.stderr
    );
    assert!(
        report.stdout.contains(&format!("FAIL  {id}  check")),
        "the fixture fails at the check step:\n{}",
        report.stdout
    );
    assert!(
        report
            .stdout
            .contains(&format!("\nfailures:\n  {id}  (step=check, expect=pass)")),
        "the failures block names the fixture:\n{}",
        report.stdout
    );
    assert!(
        report.stdout.contains("stderr contains W9")
            || report.stdout.contains("stderr contains W7"),
        "the failures block names the stolen criterion:\n{}",
        report.stdout
    );

    let json = doctor(&bins, &["--fixtures", &path_of(dir), "--json"]);
    assert_eq!(json.code, 1, "{}", json.stderr);
    assert!(
        json.stdout
            .contains(&format!("\"warning_codes\": [{codes}]")),
        "the machine report carries every matched code, in table order:\n{}",
        json.stdout
    );
}

/// `<rect fill="url(#--labels has 3 entries)"/>`: W9's id carries W4a's phrase.
#[test]
fn a_w4a_phrase_inside_a_dangling_id_cannot_mask_w9() {
    let dir = one_fixture(
        "w9-masked-by-w4a",
        "ggplot2/dangling_label_count",
        &dangling_panel("--labels has 3 entries"),
    );
    assert_verdict(&dir, "ggplot2/dangling_label_count", "\"W4a\", \"W9\"");
}

/// `url(#was moved inside the canvas)`: W9's id carries W5's moved-label phrase.
#[test]
fn a_w5_phrase_inside_a_dangling_id_cannot_mask_w9() {
    let dir = one_fixture(
        "w9-masked-by-w5",
        "ggplot2/dangling_moved",
        &dangling_panel("was moved inside the canvas"),
    );
    assert_verdict(&dir, "ggplot2/dangling_moved", "\"W5\", \"W9\"");
}

/// A W7 fixture that also carries `--labels has 3` in a kept path value: the W4a
/// phrase must not turn the W7 failure into a pass.
#[test]
fn a_w4a_phrase_inside_a_kept_value_cannot_mask_w7() {
    let dir = one_fixture("w7-masked-by-w4a", "ggplot2/kept_verbatim", W7_MASKED_PANEL);
    assert_verdict(&dir, "ggplot2/kept_verbatim", "\"W6\", \"W4a\", \"W7\"");
}
