//! Text-node entity decoding (R47).
//!
//! `quick_xml` hands a text-node reference over as `Event::GeneralRef` without its
//! `&`/`;`. Both DOMs used to resolve only the five predefined names and echo
//! everything else verbatim, so `&#176;` became the literal `&#176;` and the
//! serializer escaped the `&` again ⇒ `&amp;#176;`, which renders as visible text.
//!
//! These tests drive the real binary on both paths (`plot-grid` and `convert`) and
//! assert the decoded text node character for character, plus the fail-closed
//! behaviour for an undefined name and the untouched CDATA literal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PANEL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>
  <text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text>
</svg>"#;

const PLAIN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text>
</svg>"#;

fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).unwrap();
    dir.join(format!("textref_{name}_{}", std::process::id()))
}

fn write(name: &str, template: &str, body: &str) -> PathBuf {
    let path = tmp(name);
    fs::write(&path, template.replace("{BODY}", body)).unwrap();
    path
}

fn plot_grid(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_svg_grid"))
        .args([
            "plot-grid",
            "--width",
            "400",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--input",
        ])
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn convert(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_svg_grid"))
        .arg("convert")
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

/// The character data of every `<text>` element, in document order.
fn text_nodes(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .map(|chunk| {
            let start = chunk.find('>').expect("text start tag") + 1;
            let end = chunk[start..].find("</text>").expect("text end tag") + start;
            chunk[start..end].to_string()
        })
        .collect()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn plot_grid_decodes_character_references_in_text() {
    let input = write("plot_grid.svg", PANEL, "37&#176;C &#215;2 &#177;1");
    let output = tmp("plot_grid.out.svg");
    let result = plot_grid(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    assert_eq!(text_nodes(&read(&output)), ["37°C ×2 ±1"]);
}

#[test]
fn convert_decodes_character_references_in_text() {
    let input = write("convert.svg", PLAIN, "37&#176;C &#215;2 &#177;1");
    let output = tmp("convert.out.svg");
    let result = convert(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    assert_eq!(text_nodes(&read(&output)), ["37°C ×2 ±1"]);
}

#[test]
fn plot_grid_rejects_an_undefined_named_entity_in_text() {
    let input = write("plot_grid_nbsp.svg", PANEL, "a&nbsp;b");
    let output = tmp("plot_grid_nbsp.out.svg");
    let _ = fs::remove_file(&output);
    let result = plot_grid(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("unrecognized entity"), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

#[test]
fn convert_rejects_an_undefined_named_entity_in_text() {
    let input = write("convert_nbsp.svg", PLAIN, "a&nbsp;b");
    let output = tmp("convert_nbsp.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("unrecognized entity"), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

#[test]
fn cdata_content_stays_a_literal() {
    let input = write("cdata.svg", PANEL, "<![CDATA[37&#176;C]]>");
    let output = tmp("cdata.out.svg");
    let result = plot_grid(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    // CDATA is character data, not markup: the `&` is escaped on the way out and
    // the renderer draws the same literal it drew before R47.
    assert_eq!(text_nodes(&read(&output)), ["37&amp;#176;C"]);
}
