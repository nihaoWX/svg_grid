//! End-to-end CLI tests: exit codes, "no output on failure", idempotency and the
//! half-finished-product / byte-copy semantics.
//!
//! These invoke the actual `svg_grid_convert` binary so the process-level exit
//! codes and the file-write behaviour are exercised, not just the library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SVGLITE: &str = r#"<?xml version='1.0' encoding='UTF-8' ?>
<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='288.00pt' height='216.00pt' viewBox='0 0 288.00 216.00'>
<g class='svglite'>
<defs>
  <style type='text/css'><![CDATA[
    .svglite circle { fill: none; }
  ]]></style>
</defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<defs>
  <clipPath id='cp1'><rect x='17.92' y='24.17' width='258.74' height='238.72' /></clipPath>
</defs>
<g clip-path='url(#cp1)'>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; fill: #CFA15A;' />
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
<text transform='translate(12.72,143.53) rotate(-90)' style='font-size: 8.80px;'>ylabel</text>
</g>
</g>
</svg>"#;

const MATPLOTLIB: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='640' height='480' viewBox='0 0 640 480'>
<g transform='translate(80.0,48.0) scale(1,-1)'>
<path d='M 0 0 L 10 10' fill='none'/>
</g>
<image x='0' y='0' width='10' height='10' xlink:href='data:image/png;base64,iVBORw0KGgo='/>
</svg>"#;

/// Otherwise-compatible document whose only offender is an `<image>`: it must be
/// rejected, so the assertion can actually fail.
const IMAGE_ONLY: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='288' height='216' viewBox='0 0 288 216'>
<circle cx='60.82' cy='135.96' r='0.65'/>
<image x='0' y='0' width='10' height='10' xlink:href='data:image/png;base64,iVBORw0KGgo='/>
</svg>"#;

/// A half-finished product: a panel box but no `data-scale` grouping.
const HALF_CONVERTED: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' width='288' height='216' viewBox='0 0 288 216'>
<rect data-panel-box='main' x='0' y='0' width='288' height='216' visibility='hidden' pointer-events='none'/>
<circle cx='10' cy='10' r='1'/>
</svg>"#;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_svg_grid_convert"))
}

fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).unwrap();
    dir.join(format!("{name}_{}", std::process::id()))
}

fn write_fixture(name: &str, contents: &str) -> PathBuf {
    let path = tmp(name);
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn converts_svglite_fixture_with_exit_zero() {
    let input = write_fixture("cli_svglite.svg", SVGLITE);
    let output = tmp("cli_svglite_out.svg");

    let status = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .arg("--report")
        .status()
        .unwrap();

    assert!(status.success(), "expected exit 0, got {status:?}");

    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"data-panel-box="main""#));
    assert!(svg.contains(r#"<g data-scale="xy">"#));
    assert!(svg.contains(r#"<g data-scale="position"><circle"#));
    assert!(svg.contains(r#"<g data-scale="position"><text"#));
    // Constructive check: text is preserved as text (not rasterized away).
    assert!(svg.contains(">Title<"), "text payload must survive: {svg}");
    assert!(
        svg.matches("<text").count() >= 2,
        "both text runs must survive: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn embedded_image_is_converted_with_exit_zero() {
    // An `<image>` used to be rejected; the translator now bakes its box and
    // writes `preserveAspectRatio="none"` so the composer can stretch it.
    let input = write_fixture("cli_image_only.svg", IMAGE_ONLY);
    let output = tmp("cli_image_only_out.svg");
    let _ = fs::remove_file(&output);

    let result = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(result.success(), "an embedded <image> must now convert");
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains("<image"), "the image must survive: {svg}");
    assert!(
        svg.contains(r#"preserveAspectRatio="none""#),
        "the image must be stretched to its box: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn converts_matplotlib_fixture_with_exit_zero() {
    let input = write_fixture("cli_mpl.svg", MATPLOTLIB);
    let output = tmp("cli_mpl_out.svg");
    let _ = fs::remove_file(&output);

    let result = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(result.success(), "the matplotlib fixture must now convert");
    let svg = fs::read_to_string(&output).unwrap();
    assert!(!svg.contains("<path"), "no <path> may remain: {svg}");
    assert!(
        !svg.contains("<g transform"),
        "no <g transform> may remain: {svg}"
    );
    assert!(
        svg.contains("<polyline"),
        "the path became a polyline: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn check_mode_reports_incompatibility_without_writing() {
    // A nested <svg> is still rejected (fail closed).
    let input = write_fixture(
        "cli_check_bad.svg",
        r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<svg x='0' y='0' width='10' height='10' viewBox='0 0 10 10'><circle cx='1' cy='1' r='1'/></svg>
</svg>"#,
    );
    let result = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--check", "--report"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("<svg>"), "stdout: {stdout}");
    let _ = fs::remove_file(input);
}

#[test]
fn check_mode_passes_on_compatible_input() {
    let input = write_fixture("cli_check_ok.svg", SVGLITE);
    let result = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .arg("--check")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("compatible"), "stdout: {stdout}");
    assert!(stdout.contains("canvas box"), "stdout: {stdout}");
    let _ = fs::remove_file(input);
}

#[test]
fn second_run_is_idempotent_and_leaves_content_unchanged() {
    let input = write_fixture("cli_idem.svg", SVGLITE);
    let output = tmp("cli_idem_out.svg");

    let first = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(first.success());
    let after_first = fs::read_to_string(&output).unwrap();

    // Re-running the converter on its own output (same file) must skip.
    let second = Command::new(bin())
        .args(["--input"])
        .arg(&output)
        .args(["--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(second.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("already converted"), "stdout: {stdout}");
    assert_eq!(after_first, fs::read_to_string(&output).unwrap());

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn already_converted_with_explicit_output_is_copied_verbatim() {
    // Reproduces the pipeline hazard: `convert --input a --output b` on an
    // already-converted `a` used to exit 0 while never creating `b`.
    let input = write_fixture("cli_copy_src.svg", SVGLITE);
    let tagged = tmp("cli_copy_tagged.svg");
    let status = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&tagged)
        .status()
        .unwrap();
    assert!(status.success());
    let tagged_bytes = fs::read(&tagged).unwrap();

    let copy = tmp("cli_copy_dup.svg");
    let _ = fs::remove_file(&copy);
    let result = Command::new(bin())
        .args(["--input"])
        .arg(&tagged)
        .args(["--output"])
        .arg(&copy)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(
        copy.exists(),
        "already-converted input with an explicit --output must produce the file"
    );
    assert_eq!(
        tagged_bytes,
        fs::read(&copy).unwrap(),
        "the copied output must be byte-identical to the input"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(tagged);
    let _ = fs::remove_file(copy);
}

#[test]
fn check_on_already_converted_input_prints_stdout_verdict() {
    let input = write_fixture("cli_check_conv.svg", SVGLITE);
    let tagged = tmp("cli_check_conv_tagged.svg");
    let status = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&tagged)
        .status()
        .unwrap();
    assert!(status.success());

    let result = Command::new(bin())
        .args(["--input"])
        .arg(&tagged)
        .arg("--check")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("already converted"), "stdout: {stdout}");
    assert!(stdout.contains("compatible"), "stdout: {stdout}");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(tagged);
}

#[test]
fn half_converted_without_force_fails_and_with_force_succeeds() {
    let input = write_fixture("cli_half.svg", HALF_CONVERTED);
    let output = tmp("cli_half_out.svg");

    // Without --force: hard error (exit 1), no output.
    let _ = fs::remove_file(&output);
    let without = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(
        without.status.code(),
        Some(1),
        "half-finished product must fail without --force"
    );
    let stderr = String::from_utf8_lossy(&without.stderr);
    assert!(
        stderr.contains("force"),
        "stderr should suggest --force: {stderr}"
    );
    assert!(!output.exists());

    // With --force: completes the tagging.
    let with_force = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .arg("--force")
        .status()
        .unwrap();
    assert!(with_force.success(), "--force must complete a half product");
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"data-scale="xy""#));
    assert!(svg.contains(r#"data-scale="position""#));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn rotate_with_center_is_accepted() {
    let input = write_fixture(
        "cli_rotate.svg",
        r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text x='5' y='40' transform='rotate(-45 10 10)'>Y</text>
</svg>"#,
    );
    let output = tmp("cli_rotate_out.svg");
    let status = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success(), "rotate(a x y) must be accepted");
    let svg = fs::read_to_string(&output).unwrap();
    // `rotate(a x y)` is already in the composer's supported form: it is kept
    // verbatim (the composer rewrites the rotation center at compose time).
    assert!(svg.contains("rotate(-45 10 10)"));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn missing_output_argument_is_an_error() {
    let input = write_fixture("cli_noout.svg", SVGLITE);
    let result = Command::new(bin())
        .args(["--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let _ = fs::remove_file(input);
}
