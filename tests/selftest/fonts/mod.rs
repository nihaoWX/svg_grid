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
        // These runs assert diagnostics; without this flag even a W8 would be silent.
        "--verbose",
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
fn a_missing_label_family_is_a_hard_font_error() {
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

    // W8 is "necessary information" since D5: a missing family fails every output
    // (SVG included) with the existing font-error tier and writes no file.
    assert_eq!(
        result.status.code(),
        Some(3),
        "a missing label family must be a font error: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains(r#""NoSuchFont_xyz""#) && stderr.contains("is not in the font library"),
        "the missing family must be named: {stderr}"
    );
    assert!(
        stderr.contains("available:") && stderr.contains("Arial"),
        "the available families must be listed: {stderr}"
    );
    assert!(
        stderr.contains("--list-fonts") && stderr.contains("fontlib/"),
        "the error must say how to fix it: {stderr}"
    );
    assert!(
        !std::path::Path::new(&out).exists(),
        "a failed font lookup must not leave an output file"
    );

    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

/// `--label-fontfamily` is a CSS fallback chain: the first *available* family wins,
/// not the first non-generic one. A chain whose head is missing but whose tail is in
/// the library must run silently, and the user's string must reach the output verbatim.
#[test]
fn a_fallback_chain_wins_with_its_first_available_family() {
    if !fonts_available() {
        return;
    }
    let panels = write_panels(&["fonts_chain_tail"]);
    let chain = "Helvetica, Arial, sans-serif";
    let (result, out) = run_cli(
        "fonts_chain_tail_out.svg",
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
            chain,
            "--verbose",
        ],
    );
    assert!(
        result.status.success(),
        "a chain with an available tail must succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        result.stderr.len(),
        0,
        "a satisfied chain must be silent, got: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = fs::read_to_string(&out).unwrap();
    assert!(
        svg.contains(&format!(r#"font-family="{chain}""#)),
        "the chain must be written verbatim, not rewritten to the resolved family"
    );

    // A generic family anywhere in the chain is always satisfiable: the renderer
    // resolves it, so the run must not fail even when no named family is installed.
    let (generic, generic_out) = run_cli(
        "fonts_chain_generic_out.svg",
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
            "serif, NoSuchFont_xyz",
            "--verbose",
        ],
    );
    assert!(
        generic.status.success(),
        "a chain containing a generic family must succeed: {}",
        String::from_utf8_lossy(&generic.stderr)
    );
    assert_eq!(generic.stderr.len(), 0);

    for path in [out, generic_out] {
        let _ = fs::remove_file(path);
    }
    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

/// Only a chain whose every name is unavailable is a font error, and the message
/// must name the whole chain, list the library, and stay inside the 200-char cap.
#[test]
fn a_wholly_unavailable_chain_is_a_hard_font_error() {
    if !fonts_available() {
        return;
    }
    let panels = write_panels(&["fonts_chain_dead"]);
    let chain = "Helvetica, Liberation Sans, DejaVu Sans";
    let (result, out) = run_cli(
        "fonts_chain_dead_out.svg",
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
            chain,
        ],
    );
    assert_eq!(
        result.status.code(),
        Some(3),
        "a wholly unavailable chain must be a font error: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Helvetica") && stderr.contains("is not in the font library"),
        "the chain must be named: {stderr}"
    );
    assert!(
        stderr.contains("no family in the chain is usable"),
        "the message must say the whole chain failed: {stderr}"
    );
    assert!(
        stderr.contains("available:") && stderr.contains("Arial"),
        "the available families must be listed: {stderr}"
    );
    assert!(
        stderr.contains("--list-fonts"),
        "the error must say how to fix it: {stderr}"
    );
    for line in stderr.lines() {
        assert!(
            line.chars().count() <= 200,
            "every stderr line must stay within 200 chars ({}): {line}",
            line.chars().count()
        );
    }
    assert!(
        !std::path::Path::new(&out).exists(),
        "a failed font lookup must not leave an output file"
    );

    for panel in panels {
        let _ = fs::remove_file(panel);
    }
}

/// A pathologically long chain must still produce a single line inside the cap:
/// the echo is clamped, and when even that is not enough the message degrades
/// further instead of spilling over 200 characters.
#[test]
fn a_pathologically_long_chain_still_fits_the_error_cap() {
    if !fonts_available() {
        return;
    }
    let panels = write_panels(&["fonts_chain_long"]);
    for (name, chain) in [
        ("fonts_chain_long_81", "X".repeat(81)),
        ("fonts_chain_long_200", "X".repeat(200)),
        (
            "fonts_chain_long_many",
            (0..200)
                .map(|i| format!("N{i}"))
                .collect::<Vec<_>>()
                .join(","),
        ),
    ] {
        let (result, out) = run_cli(
            &format!("{name}_out.svg"),
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
                &chain,
            ],
        );
        assert_eq!(
            result.status.code(),
            Some(3),
            "a dead chain must be a font error: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        for line in stderr.lines() {
            assert!(
                line.chars().count() <= 200,
                "every stderr line must stay within 200 chars ({}): {line}",
                line.chars().count()
            );
        }
        assert!(
            stderr.contains("is not in the font library")
                && stderr.contains("available:")
                && stderr.contains("--list-fonts"),
            "the message must stay actionable: {stderr}"
        );
        assert!(
            !std::path::Path::new(&out).exists(),
            "a failed font lookup must not leave an output file"
        );
    }

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
