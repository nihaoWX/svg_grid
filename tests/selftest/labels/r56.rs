// R56: close the gaps R54 left in the panel-letter judgement. Every case was
// settled against resvg 0.47 (`d1render`) and cairosvg first; the comment on
// each test names the d1render ink of a single panel (0 = nothing drawn, so not
// a letter).
//
// P1: among equal `!important` declarations resvg keeps the FIRST, so the
//     cascade must not overwrite an earlier important declaration with a later
//     one. Plain declarations still let the later one win, an `!important` one
//     still outranks a plain one, and the keyword stays case-insensitive.
// P2: a zero alpha channel (`#rrggbbaa`, `rgba(...)`) paints nothing, a symbol
//     marked `display:none` is still drawn through `<use>`, and the `href`
//     chain of a gradient has no fixed depth.
// P2.1: a marker that covers nested markers counts one letter per drawn glyph:
//     `<g label="0"><text>A</text><text label="1">A</text></g>` draws two A's
//     and yields "AA" plus "A", so a single panel has no duplicate (R54 made
//     this change; R51 had counted the outer letter as a second "A").

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::*;

fn panel(name: &str, body: &str) -> PathBuf {
    path_panel(name, 100, 80, body)
}

fn styled(style: &str) -> String {
    format!(
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" style="{style}">A</text>"#
    )
}

fn compose(name: &str, inputs: &[PathBuf]) -> (std::process::Output, PathBuf) {
    run_cli(name, inputs, &["--width", "400", "--ncol", "2"])
}

fn compose_png(name: &str, inputs: &[PathBuf]) -> (std::process::Output, PathBuf, PathBuf) {
    let out = temp_path(&format!("{name}.svg"));
    let png = temp_path(&format!("{name}.png"));
    let mut args = vec!["--width".to_string(), "400".to_string()];
    args.push("--ncol".to_string());
    args.push(inputs.len().to_string());
    args.push("--output".to_string());
    args.push(out.to_string_lossy().into_owned());
    args.push("--output-png".to_string());
    args.push(png.to_string_lossy().into_owned());
    let result = run_plot_grid(&args_with_inputs(inputs, &args));
    (result, out, png)
}

fn args_with_inputs(inputs: &[PathBuf], rest: &[String]) -> Vec<String> {
    let mut args = Vec::new();
    for panel in inputs {
        args.push("--input".to_string());
        args.push(panel.to_string_lossy().into_owned());
    }
    args.extend(rest.iter().cloned());
    args
}

fn stderr(result: &std::process::Output) -> String {
    String::from_utf8_lossy(&result.stderr).into_owned()
}

fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Two identical panels carrying `body`: both draw the same glyph, so the tool
/// reporting `letter_text` as a duplicate proves it counted the glyph.
fn assert_drawn(tag: &str, body: &str, letter_text: &str) {
    let a = panel(&format!("r56_{tag}_a.svg"), body);
    let b = panel(&format!("r56_{tag}_b.svg"), body);
    let (result, out) = compose(&format!("r56_{tag}_out.svg"), &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{tag} draws ink: {err}");
    assert!(
        err.contains(&format!(r#"duplicate panel letters: "{letter_text}""#)),
        "{tag}: expected letter {letter_text:?}: {err}"
    );
    assert!(
        !out.exists(),
        "{tag}: rc=2 must not write the output: {err}"
    );
    cleanup(&[a, b, out]);
}

/// Two identical panels carrying `body`: neither draws anything, so the tool
/// must stay silent and write the composed figure.
fn assert_blank(tag: &str, body: &str) {
    let a = panel(&format!("r56_{tag}_a.svg"), body);
    let b = panel(&format!("r56_{tag}_b.svg"), body);
    let (result, out) = compose(&format!("r56_{tag}_out.svg"), &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(0), "{tag} leaves ink 0: {err}");
    assert!(!err.contains("duplicate panel letters"), "{tag}: {err}");
    assert!(out.exists(), "{tag}: rc=0 must write the output: {err}");
    cleanup(&[a, b, out]);
}

#[test]
fn cli_lets_the_first_important_display_declaration_win() {
    // d1render: first important wins, so both of these draw the A (ink 40).
    assert_drawn(
        "r56_first_imp_a",
        &styled("display:block !important;display:none !important"),
        "A",
    );
    // And here the first important hides it (ink 0).
    assert_blank(
        "r56_first_imp_b",
        &styled("display:none !important;display:block !important"),
    );
    // Still the first important, not the first declaration: "block" is plain.
    assert_blank(
        "r56_first_imp_c",
        &styled("display:none !important;display:block;display:block !important"),
    );
}

#[test]
fn cli_keeps_the_important_keyword_case_insensitive() {
    // d1render ink 40: `!IMPORTANT` is an important flag too, so the first one
    // (block) wins.
    assert_drawn(
        "r56_imp_case",
        &styled("display:block !important;display:none !IMPORTANT"),
        "A",
    );
    assert_drawn(
        "r56_imp_case_rev",
        &styled("display:block !IMPORTANT;display:none !important"),
        "A",
    );
}

#[test]
fn cli_lets_the_first_important_visibility_declaration_win() {
    // d1render ink 40 / 0: the same cascade rule for `visibility`.
    assert_drawn(
        "r56_vis_imp",
        &styled("visibility:visible !important;visibility:hidden !important"),
        "A",
    );
    assert_blank(
        "r56_vis_imp_rev",
        &styled("visibility:hidden !important;visibility:visible !important"),
    );
}

#[test]
fn cli_keeps_the_plain_declaration_cascade() {
    // The R54 rules must survive the R56 change: a later plain declaration wins
    // (d1 ink 40 / 0), and a plain one never outranks an important one.
    assert_drawn("r56_plain_win", &styled("display:none;display:block"), "A");
    assert_blank("r56_plain_lose", &styled("display:block;display:none"));
    assert_blank(
        "r56_plain_imp",
        &styled("display:none !important;display:block"),
    );
    assert_drawn(
        "r56_imp_over_plain",
        &styled("display:none;display:block !important"),
        "A",
    );
}

#[test]
fn cli_strips_a_stray_bang_from_a_declaration() {
    // d1render ink 0: resvg drops a trailing `!` even when it is not followed by
    // `important`, so the value is still `none`.
    assert_blank("r56_bang", &styled("display:none !"));
    assert_blank("r56_bang_foo", &styled("display:none ! foo"));
    assert_blank("r56_bang_visibility", &styled("visibility:hidden !"));
}

#[test]
fn cli_ignores_a_zero_alpha_colour() {
    // d1render ink 0 for both spellings; cairosvg still paints `#00000000`
    // (disagreement), and R56 follows resvg as R51 did.
    let plain = |fill: &str| {
        format!(
            r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" fill="{fill}">A</text>"#
        )
    };
    assert_blank("r56_hex8", &plain("#00000000"));
    assert_blank("r56_rgba", &plain("rgba(0,0,0,0)"));
    // A non-zero alpha still paints (ink 40), so the parser must not overreach.
    assert_drawn("r56_hex8_opaque", &plain("#000000ff"), "A");
    assert_drawn("r56_hex8_half", &plain("#00000080"), "A");
    assert_drawn("r56_rgba_half", &plain("rgba(0,0,0,0.5)"), "A");
}

#[test]
fn cli_ignores_a_zero_alpha_gradient_stop() {
    // d1render ink 0: every stop is invisible, so the gradient paints nothing.
    let gradient = |stops: &str| {
        format!(
            r#"<defs><linearGradient id="g">{stops}</linearGradient></defs><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" fill="url(#g)">A</text>"#
        )
    };
    assert_blank(
        "r56_stop_rgba0",
        &gradient(r#"<stop offset="0" stop-color="rgba(0,0,0,0)"/>"#),
    );
    assert_blank(
        "r56_stop_hex0",
        &gradient(r##"<stop offset="0" stop-color="#00000000"/>"##),
    );
    assert_drawn(
        "r56_stop_rgba_half",
        &gradient(r#"<stop offset="0" stop-color="rgba(0,0,0,0.5)"/>"#),
        "A",
    );
}

#[test]
fn cli_follows_a_gradient_href_chain_without_a_depth_limit() {
    // d1render ink 40: a stop ten links down the chain still paints.
    let chain = format!(
        r#"<defs><linearGradient id="g0"><stop offset="0" stop-color="black"/></linearGradient>{}</defs><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" fill="url(#g10)">A</text>"#,
        (1..=10)
            .map(|i| format!(r##"<linearGradient id="g{i}" href="#g{}"/>"##, i - 1))
            .collect::<String>()
    );
    assert_drawn("r56_chain", &chain, "A");
    // A circular chain paints nothing in either renderer (ink 0), and must not
    // hang the walker.
    let circular = r##"<defs><linearGradient id="a" href="#b"/><linearGradient id="b" href="#a"/></defs><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" fill="url(#a)">A</text>"##;
    assert_blank("r56_chain_cycle", circular);
}

#[test]
fn cli_ignores_a_symbol_whose_view_box_clips_the_text_away() {
    // A `<symbol viewBox>` clips its content to that rectangle. Measured with
    // resvg 0.47 on a 100x80 panel: the 6x6 box leaves ink 0 (the text at y=20
    // spans y 11..23), the 20x20 box leaves ink 358.
    let clipped = r##"<defs><symbol id="s" viewBox="0 0 6 6" data-svg-grid-label="0"><text x="4" y="20" font-size="12" font-family="Arial">A</text></symbol></defs><use href="#s"/>"##;
    assert_blank("r56_symbol_clip", clipped);
    let visible = r##"<defs><symbol id="s" viewBox="0 0 20 20" data-svg-grid-label="0"><text x="4" y="20" font-size="12" font-family="Arial">A</text></symbol></defs><use href="#s"/>"##;
    assert_drawn("r56_symbol_vb", visible, "A");
}

#[test]
fn cli_draws_a_marker_on_a_display_none_symbol() {
    // d1render ink 40: `display` does not apply to a `<symbol>` itself, so the
    // text is still drawn through `<use>` (cairosvg hides it — resvg wins).
    let body = r##"<defs><symbol id="s" style="display:none" data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text></symbol></defs><use href="#s"/>"##;
    assert_drawn("r56_symbol_hidden", body, "A");
}

#[test]
fn cli_counts_one_letter_per_drawn_glyph_under_a_covering_marker() {
    if !fonts_available() {
        return;
    }
    let marked = r#"<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text><text data-svg-grid-label="1" x="30" y="14" font-size="12" font-family="Arial">A</text></g>"#;
    let unmarked = r#"<g><text x="4" y="14" font-size="12" font-family="Arial">A</text><text x="30" y="14" font-size="12" font-family="Arial">A</text></g>"#;

    // The markers change nothing about what is drawn: the PNG is byte-identical
    // to the same figure without a single marker (two A glyphs, ink 80).
    let marked_panel = panel("r56_cover_marked.svg", marked);
    let plain_panel = panel("r56_cover_plain.svg", unmarked);
    let (marked_result, marked_out, marked_png) =
        compose_png("r56_cover_marked", std::slice::from_ref(&marked_panel));
    let (plain_result, plain_out, plain_png) =
        compose_png("r56_cover_plain", std::slice::from_ref(&plain_panel));
    assert!(marked_result.status.success(), "{}", stderr(&marked_result));
    assert!(plain_result.status.success(), "{}", stderr(&plain_result));
    assert_eq!(
        read(&marked_png),
        read(&plain_png),
        "the markers must not change the raster: two A glyphs are drawn"
    );

    // One panel, two letters ("AA" and "A"): no duplicate, so rc=0.
    let (single_result, single_out) = run_cli(
        "r56_cover_single.svg",
        std::slice::from_ref(&marked_panel),
        &["--width", "400", "--ncol", "1"],
    );
    assert_eq!(
        single_result.status.code(),
        Some(0),
        "one panel draws two A's; the covering marker is not a second \"A\": {}",
        stderr(&single_result)
    );

    // Two identical panels: each letter repeats across the inputs, so both the
    // covering "AA" and the nested "A" are reported.
    let second = panel("r56_cover_second.svg", marked);
    let (both, both_out) = compose(
        "r56_cover_both.svg",
        &[marked_panel.clone(), second.clone()],
    );
    let err = stderr(&both);
    assert_eq!(both.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "AA""#), "{err}");
    assert!(err.contains(r#""A" at #0 (cell 1)"#), "{err}");
    cleanup(&[
        marked_panel,
        plain_panel,
        marked_out,
        plain_out,
        marked_png,
        plain_png,
        single_out,
        second,
        both_out,
    ]);
}
