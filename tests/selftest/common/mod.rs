// Shared fixtures and CLI helpers used by every theme file below.

pub(crate) mod harness;

use std::fs;
use std::path::PathBuf;

pub(crate) fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("svg_grid_test_{}_{}", std::process::id(), name));
    path
}

pub(crate) fn run_plot_grid(args: &[String]) -> std::process::Output {
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_svg_grid"));
    std::process::Command::new(bin)
        .arg("plot-grid")
        .args(args)
        .output()
        .unwrap()
}

/// `true` when this checkout's font library holds fonts.
///
/// An empty `fontlib/` is a hard error by design (fail-closed), so the tests that
/// need real glyphs skip instead of failing: they print why, which points at
/// `fontlib/setup.sh`. Everything that composes pure vectors or fails closed on
/// purpose runs unconditionally.
pub(crate) fn fonts_available() -> bool {
    match svg_grid::font_families(&svg_grid::default_font_dir()) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("skipping (needs glyphs): {error}");
            false
        }
    }
}

/// A minimal protocol-valid panel with a `width`x`height` canvas.
pub(crate) const PANEL_100X80: &str = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"##;

pub(crate) fn write_panels(names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|name| {
            let path = temp_path(&format!("warn_{name}.svg"));
            fs::write(&path, PANEL_100X80).unwrap();
            path
        })
        .collect()
}

pub(crate) fn run_cli(
    name: &str,
    inputs: &[PathBuf],
    extra: &[&str],
) -> (std::process::Output, PathBuf) {
    let out = temp_path(name);
    let mut args: Vec<String> = Vec::new();
    for input in inputs {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    args.push("--output".to_string());
    args.push(out.to_string_lossy().into_owned());
    args.extend(extra.iter().map(|value| value.to_string()));
    let result = run_plot_grid(&args);
    (result, out)
}

/// A `width`x`height` panel with a full-canvas panel box and `body` as content.
pub(crate) fn path_panel(name: &str, width: u32, height: u32, body: &str) -> PathBuf {
    let path = temp_path(name);
    fs::write(
        &path,
        format!(
            r##"<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="{width}" height="{height}" fill="none" stroke="none"/>
{body}
</svg>"##
        ),
    )
    .unwrap();
    path
}

/// The value of the first `attr="..."` in `svg`.
pub(crate) fn attr_value(svg: &str, attr: &str) -> String {
    let needle = format!("{attr}=\"");
    let start = svg
        .find(&needle)
        .unwrap_or_else(|| panic!("no {attr} attribute in output: {svg}"))
        + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"').expect("unterminated attribute");
    rest[..end].to_string()
}

/// Every numeric token in a `points`/`d` value (`"M 20 20 L 180 20"` -> the
/// numbers; command letters are ignored).
pub(crate) fn numbers(value: &str) -> Vec<f32> {
    value
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter_map(|token| token.parse::<f32>().ok())
        .collect()
}

/// A 100x80 panel with a body of content; the CLI composes it 1:1 (`--width 100
/// --height 80 --ncol 1 --plot-margin 0`) so the only stderr is what is under
/// test.
pub(crate) fn compose_one(name: &str, body: &str) -> (std::process::Output, PathBuf, PathBuf) {
    let input = path_panel(&format!("r6_{name}.svg"), 100, 80, body);
    let (result, out) = run_cli(
        &format!("r6_{name}_out.svg"),
        std::slice::from_ref(&input),
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    (result, input, out)
}
