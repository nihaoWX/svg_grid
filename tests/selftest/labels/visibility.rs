// R48: a marker inside a subtree that is not rendered used to take part in the
// duplicate check (rc=2) even though the rendered figure was unchanged. The
// check now follows SVG visibility: `display:none` and `opacity:0` remove the
// subtree for good, `visibility:hidden` can be undone by a descendant.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::*;

fn letter_a(name: &str) -> PathBuf {
    path_panel(
        name,
        100,
        80,
        r#"<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text>"#,
    )
}

fn compose_png(name: &str, inputs: &[PathBuf]) -> (std::process::Output, PathBuf, PathBuf) {
    let out = temp_path(&format!("{name}.svg"));
    let png = temp_path(&format!("{name}.png"));
    let mut args: Vec<String> = Vec::new();
    for input in inputs {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.push("--width".to_string());
    args.push("400".to_string());
    args.push("--ncol".to_string());
    args.push("2".to_string());
    args.push("--output".to_string());
    args.push(out.to_string_lossy().into_owned());
    args.push("--output-png".to_string());
    args.push(png.to_string_lossy().into_owned());
    (run_plot_grid(&args), out, png)
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn markers_in_non_rendered_subtrees_do_not_collide() {
    if !fonts_available() {
        return;
    }
    let visible = letter_a("r48_vis_a.svg");
    let blank = path_panel("r48_vis_blank.svg", 100, 80, "");
    let (base_result, base_out, base_png) =
        compose_png("r48_vis_base", &[blank.clone(), visible.clone()]);
    assert!(
        base_result.status.success(),
        "{}",
        String::from_utf8_lossy(&base_result.stderr)
    );
    let baseline = read(&base_png);

    let forms = [
        (
            "defs",
            r#"<defs><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></defs>"#,
        ),
        (
            "symbol",
            r#"<symbol><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></symbol>"#,
        ),
        (
            "display",
            r#"<g display="none"><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#,
        ),
        (
            "visibility",
            r#"<g visibility="hidden"><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#,
        ),
        (
            "opacity",
            r#"<g opacity="0"><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#,
        ),
    ];
    for (tag, wrapper) in forms {
        let hidden = path_panel(&format!("r48_vis_{tag}.svg"), 100, 80, wrapper);
        let (result, out, png) = compose_png(
            &format!("r48_vis_{tag}_out"),
            &[hidden.clone(), visible.clone()],
        );
        let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
        assert_eq!(result.status.code(), Some(0), "{tag}: {stderr}");
        assert!(out.exists(), "{tag}: an accepted call writes the output");
        assert_eq!(
            read(&png),
            baseline,
            "{tag}: the rendered figure must be identical to the no-marker version"
        );
        for path in [hidden, out, png] {
            let _ = fs::remove_file(path);
        }
    }
    for path in [blank, visible, base_out, base_png] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn visibility_hidden_can_be_restored_by_a_descendant() {
    let restored = r#"<g visibility="hidden"><text data-svg-grid-label="0" visibility="visible" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#;
    let a = path_panel("r48_vis_restored.svg", 100, 80, restored);
    let b = letter_a("r48_vis_restored_b.svg");
    let (result, out) = run_cli(
        "r48_vis_restored_out.svg",
        &[a.clone(), b.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    assert!(!out.exists());
    for path in [a, b] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn display_none_cannot_be_restored_by_a_descendant() {
    if !fonts_available() {
        return;
    }
    let hidden = r#"<g display="none"><text data-svg-grid-label="0" display="inline" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#;
    let a = path_panel("r48_vis_display_child.svg", 100, 80, hidden);
    let b = letter_a("r48_vis_display_child_b.svg");
    let blank = path_panel("r48_vis_display_blank.svg", 100, 80, "");
    let (base_result, base_out, base_png) =
        compose_png("r48_vis_display_base", &[blank.clone(), b.clone()]);
    assert!(base_result.status.success());
    let (result, out, png) = compose_png("r48_vis_display_out", &[a.clone(), b.clone()]);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(
        read(&png),
        read(&base_png),
        "display:none removes the subtree, whatever the child says"
    );
    for path in [a, b, blank, base_out, base_png, out, png] {
        let _ = fs::remove_file(path);
    }
}

// R51 D: an inline declaration may carry `!important`, which both reference
// renderers honour (`display:none !important` leaves no ink). It is stripped
// before the keyword is compared, with optional whitespace and any casing.
// The keywords themselves stay case-sensitive: `style="display:NONE"`,
// `style="DISPLAY:none"`, `display="NONE"` and `visibility="HIDDEN"` *do* draw
// the A in resvg and cairosvg (ink 40), so they must keep taking part in the
// duplicate check. CSS says property names and keywords are case-insensitive;
// both renderers are non-conforming here, and this project follows what is
// actually drawn (R38-R41).
#[test]
fn important_is_stripped_and_uppercase_keywords_stay_visible() {
    if !fonts_available() {
        return;
    }
    let visible = letter_a("r51_vis_important_b.svg");
    let blank = path_panel("r51_vis_important_blank.svg", 100, 80, "");
    let (base_result, base_out, base_png) =
        compose_png("r51_vis_important_base", &[blank.clone(), visible.clone()]);
    assert!(base_result.status.success());
    let baseline = read(&base_png);

    // (a) Every `!important` spelling hides the subtree: no ink, no letter.
    for (tag, declaration) in [
        ("plain", "display:none !important"),
        ("upper", "display:none !IMPORTANT"),
        ("spaced", "display:none ! important"),
        ("tight", "display:none!important"),
    ] {
        let hidden = path_panel(
            &format!("r51_vis_important_{tag}.svg"),
            100,
            80,
            &format!(
                r#"<g style="{declaration}"><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#
            ),
        );
        let (result, out, png) = compose_png(
            &format!("r51_vis_important_{tag}_out"),
            &[hidden.clone(), visible.clone()],
        );
        let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
        assert_eq!(result.status.code(), Some(0), "{tag}: {stderr}");
        assert!(out.exists(), "{tag}: an accepted call writes the output");
        assert_eq!(read(&png), baseline, "{tag}: `{declaration}` hides the A");
        for path in [hidden, out, png] {
            let _ = fs::remove_file(path);
        }
    }

    // (b) Uppercase spellings are *not* hidden by either renderer: the composed
    // PNG matches a plainly visible A, and the duplicate check still fires.
    for (tag, attributes) in [
        ("css_value", r#"style="display:NONE""#),
        ("css_name", r#"style="DISPLAY:none""#),
        ("attr_value", r#"display="NONE""#),
        ("visibility", r#"visibility="HIDDEN""#),
    ] {
        let painted = path_panel(
            &format!("r51_vis_upper_{tag}.svg"),
            100,
            80,
            &format!(
                r#"<g {attributes}><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#
            ),
        );
        let (alone, alone_out, alone_png) = compose_png(
            &format!("r51_vis_upper_{tag}_alone"),
            &[blank.clone(), painted.clone()],
        );
        assert!(
            alone.status.success(),
            "{tag}: {}",
            String::from_utf8_lossy(&alone.stderr)
        );
        assert_eq!(
            read(&alone_png),
            baseline,
            "{tag}: `{attributes}` draws the A (ink 40 in resvg and cairosvg)"
        );

        let (upper, upper_out) = run_cli(
            "r51_vis_upper_out.svg",
            &[painted.clone(), visible.clone()],
            &["--width", "400", "--ncol", "2"],
        );
        let upper_err = String::from_utf8_lossy(&upper.stderr).into_owned();
        assert_eq!(
            upper.status.code(),
            Some(2),
            "{tag} still draws the A, so it is a letter: {upper_err}"
        );
        assert!(upper_err.contains("duplicate panel letters"), "{upper_err}");
        assert!(!upper_out.exists(), "rc=2 must not write the output");
        for path in [painted, alone_out, alone_png, upper_out] {
            let _ = fs::remove_file(path);
        }
    }
    for path in [visible, blank, base_out, base_png] {
        let _ = fs::remove_file(path);
    }
}

// R51 E: a `<text>` that paints neither a fill nor a stroke leaves no ink
// (`fill="none"` alone is ink 0 in both reference renderers), so its marker is
// not a letter. A visible stroke, or a descendant that restores the fill, still
// counts.
#[test]
fn text_that_paints_nothing_is_not_a_letter() {
    if !fonts_available() {
        return;
    }
    let visible = letter_a("r51_paint_visible.svg");
    let blank = path_panel("r51_paint_blank.svg", 100, 80, "");
    let (base_result, base_out, base_png) =
        compose_png("r51_paint_base", &[blank.clone(), visible.clone()]);
    assert!(base_result.status.success());
    let unpainted = path_panel(
        "r51_paint_none.svg",
        100,
        80,
        r#"<text data-svg-grid-label="0" fill="none" x="4" y="14" font-size="12" font-family="Arial">A</text>"#,
    );
    let (result, out, png) =
        compose_png("r51_paint_none_out", &[unpainted.clone(), visible.clone()]);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(0), "{stderr}");
    assert!(out.exists(), "an accepted call writes the output: {stderr}");
    assert_eq!(
        read(&png),
        read(&base_png),
        "fill=\"none\" text leaves no ink, so the two figures match"
    );

    for (tag, body) in [
        (
            "stroke",
            r#"<text data-svg-grid-label="0" fill="none" stroke="black" x="4" y="14" font-size="12" font-family="Arial">A</text>"#,
        ),
        (
            "restored",
            r#"<g fill="none"><text data-svg-grid-label="0" fill="black" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#,
        ),
    ] {
        let painted = path_panel(&format!("r51_paint_{tag}.svg"), 100, 80, body);
        let (painted_result, painted_out) = run_cli(
            "r51_paint_painted_out.svg",
            &[painted.clone(), visible.clone()],
            &["--width", "400", "--ncol", "2"],
        );
        let painted_err = String::from_utf8_lossy(&painted_result.stderr).into_owned();
        assert_eq!(
            painted_result.status.code(),
            Some(2),
            "{tag}: a painted A is a letter: {painted_err}"
        );
        assert!(!painted_out.exists(), "rc=2 must not write the output");
        for path in [painted, painted_out] {
            let _ = fs::remove_file(path);
        }
    }

    let inherited_none = path_panel(
        "r51_paint_inherited.svg",
        100,
        80,
        r#"<g fill="none"><text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></g>"#,
    );
    let (inherited, inherited_out) = run_cli(
        "r51_paint_inherited_out.svg",
        &[inherited_none.clone(), visible.clone()],
        &["--width", "400", "--ncol", "2"],
    );
    assert_eq!(
        inherited.status.code(),
        Some(0),
        "an inherited fill=\"none\" paints nothing either: {}",
        String::from_utf8_lossy(&inherited.stderr)
    );
    assert!(inherited_out.exists());
    for path in [
        visible,
        blank,
        base_out,
        base_png,
        unpainted,
        out,
        png,
        inherited_none,
        inherited_out,
    ] {
        let _ = fs::remove_file(path);
    }
}
