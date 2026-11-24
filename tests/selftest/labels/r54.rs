// R54: the panel-letter duplicate check must agree with what the renderer
// actually paints. Every case below was settled against resvg 0.47
// (`d1render`) and cairosvg first; the numbers in the comments are the
// d1render ink of a single panel (0 = nothing drawn, so not a letter).
//
// A: an unresolved `url(#...)` reference, or an inherited zero opacity or
//    stroke width, paints nothing — both renderers leave the canvas blank.
// B: inside one `style` attribute the later declaration wins, an
//    `!important` declaration outranks a plain one, and `/* ... */` comments
//    are not part of the declaration.
// C: a marker on a `<symbol>` is drawn once per `<use>`, exactly like a
//    marker on a `<g>`.
// D: a marker's letter is the whole text its subtree draws, nested markers
//    included; the outer marker is dropped only when it adds no character of
//    its own. Bare character data inside a `<g>` is not painted at all.
// E: the fixes must not turn any painted glyph into a non-letter.

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

/// Two identical panels carrying `body`; both draw the same glyph, so the tool
/// reporting `letter_text` as a duplicate proves it counted the glyph.
fn assert_drawn(tag: &str, body: &str, letter_text: &str) {
    let a = panel(&format!("r54_{tag}_a.svg"), body);
    let b = panel(&format!("r54_{tag}_b.svg"), body);
    let (result, out) = compose(&format!("r54_{tag}_out.svg"), &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(2),
        "{tag} draws ink, so it is a letter: {err}"
    );
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

/// Two identical panels carrying `body`; neither draws anything, so the tool
/// must stay silent and write the composed figure.
fn assert_blank(tag: &str, body: &str) {
    let a = panel(&format!("r54_{tag}_a.svg"), body);
    let b = panel(&format!("r54_{tag}_b.svg"), body);
    let (result, out) = compose(&format!("r54_{tag}_out.svg"), &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{tag} leaves ink 0 and is not a letter: {err}"
    );
    assert!(!err.contains("duplicate panel letters"), "{tag}: {err}");
    assert!(out.exists(), "{tag}: rc=0 must write the output: {err}");
    cleanup(&[a, b, out]);
}

fn text_a(attrs: &str) -> String {
    format!(
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial" {attrs}>A</text>"#
    )
}

// ---------------------------------------------------------------- A: paint

#[test]
fn cli_ignores_a_letter_painted_with_an_unresolved_reference() {
    // d1 ink 0 / cairo ink 0 for all four: no paint server, no fallback.
    assert_blank("a1_fill_url", &text_a(r#"fill="url(#missing)""#));
    assert_blank("a2_style_url", &text_a(r#"style="fill:url(#missing)""#));
    assert_blank(
        "a4_stroke_url",
        &text_a(r#"fill="none" stroke="url(#missing)""#),
    );
    assert_blank(
        "a3_group_url",
        &format!(r#"<g fill="url(#missing)">{}</g>"#, text_a("")),
    );
}

#[test]
fn cli_ignores_a_letter_with_zero_opacity() {
    // d1 ink 0 / cairo ink 0: the default fill is fully transparent.
    assert_blank("a5_fill_opacity", &text_a(r#"fill-opacity="0""#));
    assert_blank(
        "a6_group_fill_opacity",
        &format!(r#"<g fill-opacity="0">{}</g>"#, text_a("")),
    );
}

#[test]
fn cli_ignores_a_stroke_with_zero_opacity_or_width() {
    let stroked = text_a(r#"fill="none" stroke="black""#);
    // d1 ink 0 / cairo ink 0: the stroke that would paint is suppressed.
    assert_blank(
        "a7_group_stroke_opacity",
        &format!(r#"<g stroke-opacity="0">{stroked}</g>"#),
    );
    assert_blank(
        "a8_group_stroke_width",
        &format!(r#"<g stroke-width="0">{stroked}</g>"#),
    );
}

// -------------------------------------------------------- B: style cascade

#[test]
fn cli_ignores_display_none_wrapped_in_a_css_comment() {
    // d1 ink 0 (cairosvg keeps the comment as part of the value and draws 53 —
    // divergence; R51 precedent follows resvg).
    assert_blank(
        "b1_comment",
        &format!(r#"<g style="display:none /* c */">{}</g>"#, text_a("")),
    );
}

#[test]
fn cli_lets_the_later_display_declaration_win() {
    // d1 ink 40 / cairo ink 53: the block declaration wins.
    assert_drawn(
        "b2_later_block",
        &format!(
            r#"<g style="display:none;display:block">{}</g>"#,
            text_a("")
        ),
        "A",
    );
    // d1 ink 0 / cairo ink 0: the later none wins.
    assert_blank(
        "b3_later_none",
        &format!(
            r#"<g style="display:block;display:none">{}</g>"#,
            text_a("")
        ),
    );
}

#[test]
fn cli_lets_an_important_display_declaration_win() {
    // d1 ink 0 / cairo ink 0: important none beats a later plain block.
    assert_blank(
        "b4_important_none",
        &format!(
            r#"<g style="display:block;display:none !important">{}</g>"#,
            text_a("")
        ),
    );
    // d1 ink 0 / cairo ink 0: important none also beats an earlier none.
    assert_blank(
        "b5_important_first",
        &format!(
            r#"<g style="display:none !important;display:block">{}</g>"#,
            text_a("")
        ),
    );
}

#[test]
fn cli_lets_an_important_block_win_over_a_plain_none() {
    // d1 ink 40 / cairo ink 53: important block beats the plain none, and the
    // later declaration is not what decides it.
    assert_drawn(
        "b5b_important_block",
        &format!(
            r#"<g style="display:none;display:block !important">{}</g>"#,
            text_a("")
        ),
        "A",
    );
}

#[test]
fn cli_lets_the_later_visibility_declaration_win() {
    // d1 ink 40 / cairo ink 53: the visible declaration wins.
    assert_drawn(
        "b6_later_visible",
        &format!(
            r#"<g style="visibility:hidden;visibility:visible">{}</g>"#,
            text_a("")
        ),
        "A",
    );
}

// ------------------------------------------- C: a marker on a `<symbol>`

#[test]
fn cli_counts_a_marker_on_a_symbol_instanced_by_use() {
    // d1 ink 40 / cairo ink 53: `<use>` draws the symbol, so its marker owns
    // the glyph. R54 used to drop it (rc=0) and miss the duplicate.
    let body = r##"<defs><symbol id="r54_sym" data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text></symbol></defs><use href="#r54_sym"/>"##;
    assert_drawn("c1_symbol_marker", body, "A");
}

#[test]
fn cli_counts_a_marker_on_a_group_instanced_by_use() {
    // Control: the same shape through a `<g>` already worked before R54.
    let body = r##"<defs><g id="r54_grp" data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text></g></defs><use href="#r54_grp"/>"##;
    assert_drawn("c2_group_marker", body, "A");
}

// ------------------------------------------------------- D: nested markers

fn three_siblings() -> String {
    format!(
        r#"<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text>{}<text x="4" y="14" font-size="12" font-family="Arial">C</text></g>"#,
        letter("B", "1")
    )
}

#[test]
fn cli_reads_the_whole_drawn_subtree_into_an_outer_marker() {
    // d1 ink 71: the three glyphs are the same pixels as `<text>ABC</text>`,
    // so the outer marker's letter is "ABC" and a plain "ABC" panel collides.
    let a = panel("r54_d1_outer.svg", &three_siblings());
    let b = panel("r54_d1_abc.svg", &letter("ABC", "0"));
    let (result, out) = compose("r54_d1_out.svg", &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "ABC""#), "{err}");
    assert!(err.contains("#0 (cell 0)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[a, b, out]);
}

#[test]
fn cli_does_not_collide_an_outer_marker_with_its_trimmed_text() {
    // d1 ink 71 vs 45: "ABC" is not "AC", so the outer marker must not report
    // a duplicate against an AC panel (the pre-R54 code compared "AC").
    let a = panel("r54_d2_outer.svg", &three_siblings());
    let b = panel("r54_d2_ac.svg", &letter("AC", "0"));
    let (result, out) = compose("r54_d2_out.svg", &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "the outer marker reads \"ABC\", not \"AC\": {err}"
    );
    assert!(out.exists(), "rc=0 must write the output: {err}");
    cleanup(&[a, b, out]);
}

#[test]
fn cli_collects_only_the_rendered_text_of_a_mixed_marker() {
    // d1/cairo: bare "A" and "C" inside the `<g>` are never drawn; the panel
    // is pixel-identical to `<text>B</text>` (d1 diff 0). The outer marker
    // therefore adds no character of its own and is dropped; the nested
    // marker owns the only glyph.
    let body = format!(r#"<g data-svg-grid-label="0">A{}C</g>"#, letter("B", "1"));
    let a = panel("r54_d3_mixed.svg", &body);
    let (result, out) = compose("r54_d3_out.svg", std::slice::from_ref(&a));
    let err = stderr(&result);
    assert_eq!(
        result.status.code(),
        Some(0),
        "one drawn glyph is not a duplicate: {err}"
    );
    assert!(out.exists(), "rc=0 must write the output: {err}");

    let b = panel("r54_d3_b.svg", &letter("B", "0"));
    let (result, out) = compose("r54_d3_dup.svg", &[a.clone(), b.clone()]);
    let err = stderr(&result);
    assert_eq!(result.status.code(), Some(2), "{err}");
    assert!(err.contains(r#"duplicate panel letters: "B""#), "{err}");
    assert!(err.contains("#0 (cell 1)"), "{err}");
    assert!(err.contains("#1 (cell 0)"), "{err}");
    assert!(!out.exists(), "rc=2 must not write the output: {err}");
    cleanup(&[a, b, out]);
}

// --------------------------------------------------------- E: must not break

#[test]
fn cli_still_counts_a_stroked_letter_with_a_missing_fill_reference() {
    // d1 ink 61 / cairo ink 65: the stroke still paints.
    assert_drawn(
        "e1_stroke_fallback",
        &text_a(r#"fill="url(#missing)" stroke="black""#),
        "A",
    );
}

#[test]
fn cli_still_counts_a_letter_painted_by_a_gradient() {
    // d1 ink 40 / cairo ink 53: black gradient stops paint.
    let body = format!(
        r#"<defs><linearGradient id="r54_g"><stop offset="0" stop-color="black"/><stop offset="1" stop-color="black"/></linearGradient></defs>{}"#,
        text_a(r#"fill="url(#r54_g)""#)
    );
    assert_drawn("e2_gradient", &body, "A");
}

#[test]
fn cli_lets_a_child_restore_zero_fill_opacity() {
    // d1 ink 40 / cairo ink 53: the child's fill-opacity wins.
    let body = format!(
        r#"<g fill-opacity="0">{}</g>"#,
        text_a(r#"fill-opacity="1""#)
    );
    assert_drawn("e3_child_fill_opacity", &body, "A");
}

#[test]
fn cli_lets_a_child_restore_zero_stroke_width() {
    // d1 ink 61 / cairo ink 65: the child's stroke-width wins.
    let body = format!(
        r#"<g stroke-width="0">{}</g>"#,
        text_a(r#"stroke-width="1" fill="none" stroke="black""#)
    );
    assert_drawn("e4_child_stroke_width", &body, "A");
}

#[test]
fn cli_still_counts_a_semi_transparent_letter() {
    // d1 ink 40 / cairo ink 45: half opacity still paints.
    assert_drawn(
        "e5_half_opacity",
        &format!(r#"<g fill-opacity="0.5">{}</g>"#, text_a("")),
        "A",
    );
}

#[test]
fn cli_still_ignores_fill_none() {
    // d1 ink 0 / cairo ink 0, R51 ruling: no fill and no stroke draws nothing.
    assert_blank("e6_fill_none", &text_a(r#"fill="none""#));
}

#[test]
fn cli_still_counts_a_letter_with_stroke_none() {
    // d1 ink 40 / cairo ink 53: `stroke="none"` only removes the stroke; the
    // default fill still paints.
    assert_drawn("e7_stroke_none", &text_a(r#"stroke="none""#), "A");
}

#[test]
fn cli_keeps_uppercase_keywords_visible() {
    // d1 ink 40 for all three: both renderers compare keywords case
    // sensitively, so these are not `none`/`hidden` (R51 ruling).
    assert_drawn(
        "e8_display_attr_upper",
        &format!(r#"<g display="NONE">{}</g>"#, text_a("")),
        "A",
    );
    assert_drawn(
        "e9_display_style_upper",
        &format!(r#"<g style="DISPLAY:none">{}</g>"#, text_a("")),
        "A",
    );
    assert_drawn(
        "e10_visibility_upper",
        &format!(r#"<g visibility="HIDDEN">{}</g>"#, text_a("")),
        "A",
    );
}

#[test]
fn cli_still_honours_every_important_spelling() {
    // d1 ink 0 / cairo ink 0 for all four; R51 fixed the spellings.
    for (tag, style) in [
        ("e11", "display:none !important"),
        ("e12", "display:none !IMPORTANT"),
        ("e13", "display:none ! important"),
        ("e14", "display:none!important"),
    ] {
        assert_blank(tag, &format!(r#"<g style="{style}">{}</g>"#, text_a("")));
    }
}
