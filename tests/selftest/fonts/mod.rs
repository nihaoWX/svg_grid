// The font library (`fontlib/`): the default label family, `--list-fonts`, W8 for a
// family the library does not have, and the fail-closed empty-library contract.
// See `ARCHITECTURE/fonts.md`.

use std::fs;
use std::path::PathBuf;

use crate::common::*;

/// A fresh, empty directory to point `--font-dir` at.
fn empty_font_dir(name: &str) -> PathBuf {
    let dir = temp_path(&format!("fonts_{name}_dir"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn panel_letters_default_to_arial() {
    if !fonts_available() {
        return;
    }
    let panels = write_panels(&["fonts_default"]);
    let base = [
        "--width",
        "100",
        "--height",
        "80",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
        // Keep the letter inside the canvas (default vjust lifts it above the panel
        // edge, which would add the W5 clip warning on top of what is under test).
        "--vjust",
        "1",
        "--labels",
        "A",
    ];

    // (a) No --label-fontfamily: the letter appears with the default family.
    let (result, out) = run_cli("fonts_default_out.svg", &panels, &base);
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        result.stderr.len(),
        0,
        "a satisfied request must be silent, got: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = fs::read_to_string(&out).unwrap();
    let label_start = svg
        .find(r#"<text data-svg-grid-label="0""#)
        .unwrap_or_else(|| panic!("no panel letter in the output: {svg}"));
    let label = &svg[label_start..];
    let label_end = label.find('>').expect("unterminated <text");
    assert!(
        label[..label_end].contains(r#"font-family="Arial""#),
        "the default label family must be Arial: {}",
        &label[..label_end]
    );

    // (b) An explicit CSS fallback chain whose first family is in the library is
    // just as silent (the first available family wins).
    let mut chain_args = base.to_vec();
    chain_args.extend([
        "--label-fontfamily",
        r#""Arial", "Liberation Sans", sans-serif"#,
    ]);
    let (chain, chain_out) = run_cli("fonts_chain_out.svg", &panels, &chain_args);
    assert!(chain.status.success());
    assert_eq!(
        chain.stderr.len(),
        0,
        "a fallback chain led by a family the library has must be silent, got: {}",
        String::from_utf8_lossy(&chain.stderr)
    );

    for path in [out, chain_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn list_fonts_prints_the_deduplicated_sorted_library() {
    if !fonts_available() {
        return;
    }
    // --list-fonts composes nothing, so no input / output is needed.
    let result = run_plot_grid(&["--list-fonts".to_string()]);
    assert!(
        result.status.success(),
        "--list-fonts must succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    let families: Vec<&str> = stdout.lines().collect();
    assert!(!families.is_empty(), "--list-fonts printed nothing");
    assert!(
        families.contains(&"Arial"),
        "the font library must contain Arial: {stdout}"
    );

    let mut sorted = families.clone();
    sorted.sort_by_key(|family| family.to_lowercase());
    assert_eq!(families, sorted, "--list-fonts must be sorted: {stdout}");
    let mut deduplicated = families.clone();
    deduplicated.dedup();
    assert_eq!(
        families.len(),
        deduplicated.len(),
        "--list-fonts must be deduplicated: {stdout}"
    );
}

#[test]
fn warns_when_the_label_family_is_not_in_the_library() {
    if !fonts_available() {
        return;
    }
    let panels = write_panels(&["fonts_unknown"]);
    let (result, out) = run_cli(
        "fonts_unknown_out.svg",
        &panels,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--vjust",
            "1",
            "--labels",
            "A",
            "--label-fontfamily",
            "NoSuchFont_xyz",
        ],
    );

    // W8 is non-fatal: exit code 0 and the figure is still written.
    assert!(
        result.status.success(),
        "W8 must not fail the run: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains(r#""NoSuchFont_xyz""#) && stderr.contains("is not in the font library"),
        "the missing family must be named: {stderr}"
    );
    // The consequence is the shared cause note (printed once per run), not part
    // of every W8 line.
    assert!(
        stderr.contains("glyph widths and line breaks"),
        "the consequence must be stated: {stderr}"
    );
    assert!(stderr.contains("Fix:"), "W8 must carry a fix: {stderr}");
    assert!(
        stderr.contains("non-fatal warning(s)"),
        "W8 uses the shared footer: {stderr}"
    );
    // The SVG carries the requested family verbatim (vector output keeps the name).
    let svg = fs::read_to_string(&out).unwrap();
    assert!(
        svg.contains(r#"font-family="NoSuchFont_xyz""#),
        "the requested family is written to the SVG as given: {svg}"
    );

    let _ = fs::remove_file(out);
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn empty_font_dir_fails_closed_when_a_png_is_requested() {
    let panels = write_panels(&["fonts_empty_png"]);
    let dir = empty_font_dir("empty_png");
    let png = temp_path("fonts_empty_png_out.png");
    let _ = fs::remove_file(&png);

    let (result, out) = run_cli(
        "fonts_empty_png_out.svg",
        &panels,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--font-dir",
            dir.to_str().unwrap(),
            "--output-png",
            png.to_str().unwrap(),
        ],
    );

    assert!(
        !result.status.success(),
        "an empty font library plus a PNG request must fail the run"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("font library error"), "{stderr}");
    assert!(
        stderr.contains("setup.sh"),
        "the message must say how to fill the library: {stderr}"
    );
    assert!(
        !png.exists(),
        "no PNG may be produced from an empty font library"
    );

    let _ = fs::remove_file(out);
    let _ = fs::remove_dir_all(dir);
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

#[test]
fn empty_font_dir_is_fine_for_a_pure_vector_run() {
    // No PNG/AVIF and no --labels: nothing needs glyphs, so an empty library is not
    // an error (and it is not even loaded).
    let panels = write_panels(&["fonts_vector"]);
    let dir = empty_font_dir("empty_vector");
    let (result, out) = run_cli(
        "fonts_vector_out.svg",
        &panels,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--font-dir",
            dir.to_str().unwrap(),
        ],
    );
    assert!(
        result.status.success(),
        "a pure-vector run must not need fonts: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stderr.len(), 0);

    let _ = fs::remove_file(out);
    let _ = fs::remove_dir_all(dir);
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}
