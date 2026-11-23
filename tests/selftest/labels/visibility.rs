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
