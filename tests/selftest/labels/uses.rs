// R51 B: `<defs>` is in `NON_RENDERED`, so a definition's marker used to be
// skipped even when a `<use>` instanced it — a rendered glyph that never reached
// the duplicate check (base `1b7ef3d` was rc=2, R48 onward was rc=0). A
// definition now counts once per `<use>` instantiation; an unreferenced
// definition still counts zero times, and a circular `<use>` expands once.

use std::fs;
use std::path::PathBuf;

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn letter(text: &str, marker: &str) -> String {
    format!(
        r#"<text data-svg-grid-label="{marker}" x="4" y="14" font-size="12" font-family="Arial">{text}</text>"#
    )
}

fn definition() -> String {
    r#"<defs><text id="L" data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></defs>"#.to_string()
}

fn compose(name: &str, inputs: &[PathBuf]) -> (std::process::Output, PathBuf) {
    run_cli(name, inputs, &["--width", "400", "--ncol", "2"])
}

fn stderr(result: &std::process::Output) -> String {
    String::from_utf8_lossy(&result.stderr).into_owned()
}

fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn cli_counts_a_definition_once_per_use() {
    let instanced = panel(
        "r51_use_once.svg",
        &format!("{}<use href=\"#L\"/>", definition()),
    );
    let visible = panel("r51_use_visible.svg", &letter("A", "0"));
    // One instanced A plus one drawn A: the rendered figure really has two.
    let (result, out) = compose(
        "r51_use_once_out.svg",
        &[instanced.clone(), visible.clone()],
    );
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");

    // The same input without the `<use>` draws nothing: no collision.
    let unused = panel("r51_use_unreferenced.svg", &definition());
    let (clean, clean_out) = compose("r51_use_unused_out.svg", &[unused.clone(), visible.clone()]);
    assert_eq!(clean.status.code(), Some(0), "{}", stderr(&clean));
    assert!(clean_out.exists(), "rc=0 must write the output");
    cleanup(&[instanced, visible, out, unused, clean_out]);
}

#[test]
fn cli_counts_two_use_instantiations_separately() {
    let twice = panel(
        "r51_use_twice.svg",
        &format!(
            "{}<use href=\"#L\"/><use href=\"#L\" x=\"30\"/>",
            definition()
        ),
    );
    let (result, out) = compose("r51_use_twice_out.svg", std::slice::from_ref(&twice));
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "A""#), "{err}");
    assert!(err.contains("2 times inside #0 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[twice, out]);
}

#[test]
fn cli_follows_a_circular_use_only_once() {
    // A `<use>` that points at an ancestor is a circular reference: the
    // renderer draws one A (ink 40, d1render and cairosvg agree), so only the
    // outer marker is a letter.
    let self_reference = panel(
        "r51_use_self.svg",
        r##"<g id="L" data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text><use href="#L" x="30"/></g>"##,
    );
    let (result, out) = compose(
        "r51_use_self_out.svg",
        std::slice::from_ref(&self_reference),
    );
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(0), "{err}");
    assert!(out.exists(), "rc=0 must write the output: {err}");

    let mutual = panel(
        "r51_use_mutual.svg",
        r##"<defs><g id="A" data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text><use href="#B"/></g><g id="B"><use href="#A"/></g></defs><use href="#A"/>"##,
    );
    let (mutual_result, mutual_out) =
        compose("r51_use_mutual_out.svg", std::slice::from_ref(&mutual));
    let mutual_err = stderr(&mutual_result);
    assert_eq!(mutual_result.status.code(), Some(0), "{mutual_err}");
    assert!(
        mutual_out.exists(),
        "rc=0 must write the output: {mutual_err}"
    );
    cleanup(&[self_reference, out, mutual, mutual_out]);
}

#[test]
fn cli_draws_a_symbol_only_through_use() {
    let symbol = format!(
        "<defs><symbol id=\"S\">{}</symbol></defs>",
        letter("A", "0")
    );
    // Unreferenced: a `<symbol>` is not drawn where it sits.
    let unused = panel("r51_symbol_unused.svg", &symbol);
    let (clean, clean_out) = compose("r51_symbol_unused_out.svg", std::slice::from_ref(&unused));
    assert_eq!(clean.status.code(), Some(0), "{}", stderr(&clean));
    assert!(clean_out.exists(), "rc=0 must write the output");

    // Instanced once next to a drawn A: two rendered A's.
    let instanced = panel("r51_symbol_use.svg", &format!("{symbol}<use href=\"#S\"/>"));
    let visible = panel("r51_symbol_visible.svg", &letter("A", "0"));
    let (result, out) = compose(
        "r51_symbol_use_out.svg",
        &[instanced.clone(), visible.clone()],
    );
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[unused, clean_out, instanced, visible, out]);
}
