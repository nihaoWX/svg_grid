//! Auto-conversion of untagged inputs (user expectation #11, round R32).
//!
//! An input without `data-panel-box="main"` is converted in memory before
//! composition. These tests pin the safety promises: byte-identical to a manual
//! `svg_grid convert`, source untouched, tagged inputs never converted, and a
//! failed conversion is rc=2 with no output file.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::common::*;

const SVGLITE: &str = r#"<?xml version='1.0' encoding='UTF-8' ?>
<svg xmlns='http://www.w3.org/2000/svg' width='288.00pt' height='216.00pt' viewBox='0 0 288.00 216.00'>
<g class='svglite'>
<defs><style type='text/css'><![CDATA[ .svglite circle { fill: none; stroke: #000000; } ]]></style></defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; stroke: none; fill: #CFA15A;' />
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</g>
</svg>"#;

const MULTI_SUBPATH: &str = r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<path d='M 10 10 L 30 10 L 30 30 L 10 30 Z M 60 60 L 90 60 L 90 90 L 60 90 Z M 10 60 L 25 60 L 25 75 L 10 75 Z' fill='#ff0000' stroke='#000000' stroke-width='2' fill-rule='evenodd'/>
</svg>"#;

const UNSUPPORTED: &str = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<symbol id='s'><rect x='0' y='0' width='10' height='10'/></symbol>
<circle cx='50' cy='40' r='5'/>
</svg>"#;

fn write_input(name: &str, svg: &str) -> PathBuf {
    let path = temp_path(name);
    fs::write(&path, svg).unwrap();
    path
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_svg_grid"))
}

fn convert_manually(input: &Path, output: &Path) -> Output {
    Command::new(binary())
        .arg("convert")
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn plot(input: &Path, output: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "--input".to_string(),
        input.to_string_lossy().into_owned(),
        "--output".to_string(),
        output.to_string_lossy().into_owned(),
    ];
    args.extend(extra.iter().map(|value| value.to_string()));
    run_plot_grid(&args)
}

fn plan(input: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "--input".to_string(),
        input.to_string_lossy().into_owned(),
        "--plan".to_string(),
    ];
    args.extend(extra.iter().map(|value| value.to_string()));
    run_plot_grid(&args)
}

fn cell_geometries(stdout: &str) -> Vec<(f32, f32)> {
    stdout
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| {
            let start = line.find("cell ").expect("cell") + "cell ".len();
            let value = line[start..].split_whitespace().next().expect("size");
            let (w, h) = value.split_once('x').expect("WxH");
            (w.parse().unwrap(), h.parse().unwrap())
        })
        .collect()
}

fn assert_same_as_manual(source: &Path, name: &str, extra: &[&str]) {
    let tagged = temp_path(&format!("{name}.tagged.svg"));
    let manual = temp_path(&format!("{name}.manual.svg"));
    let automatic = temp_path(&format!("{name}.auto.svg"));

    let converted = convert_manually(source, &tagged);
    assert!(
        converted.status.success(),
        "manual convert failed: {}",
        String::from_utf8_lossy(&converted.stderr)
    );
    let manual_run = plot(&tagged, &manual, extra);
    assert!(
        manual_run.status.success(),
        "manual pipeline failed: {}",
        String::from_utf8_lossy(&manual_run.stderr)
    );
    let auto_run = plot(source, &automatic, extra);
    assert!(
        auto_run.status.success(),
        "auto-conversion failed: {}",
        String::from_utf8_lossy(&auto_run.stderr)
    );
    assert_eq!(
        fs::read(&manual).unwrap(),
        fs::read(&automatic).unwrap(),
        "in-memory conversion must equal `svg_grid convert` followed by `plot-grid`"
    );
}

#[test]
fn untagged_svglite_matches_manual_conversion_byte_for_byte() {
    let source = write_input("d4_svglite.svg", SVGLITE);
    assert_same_as_manual(
        &source,
        "d4_svglite",
        &["--width", "500", "--ncol", "1", "--plot-margin", "0"],
    );
    let output = temp_path("d4_svglite.auto.svg");
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"data-svg-grid-cell="0""#));
    assert!(svg.contains("<circle"));
}

#[test]
fn untagged_multi_subpath_panel_keeps_its_holes() {
    let source = write_input("d4_multi.svg", MULTI_SUBPATH);
    assert_same_as_manual(
        &source,
        "d4_multi",
        &["--width", "400", "--ncol", "1", "--plot-margin", "0"],
    );
    let svg = fs::read_to_string(temp_path("d4_multi.auto.svg")).unwrap();
    // A flattened multi-contour path keeps the two inner holes as separate `M`
    // subpaths; a solid single contour (the R28 failure) would have only one.
    assert!(svg.matches(" M ").count() >= 2, "holes were filled: {svg}");
}

#[test]
fn untagged_real_matplotlib_probe_matches_manual_conversion() {
    // A committed matplotlib 3.11 probe: 11 `<path>` glyphs, no panel box.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fig2_mixed/out/probe/probe_imshow_cb.svg");
    assert_same_as_manual(
        &source,
        "d4_mpl_real",
        &["--width", "600", "--ncol", "1", "--plot-margin", "0"],
    );
    let svg = fs::read_to_string(temp_path("d4_mpl_real.auto.svg")).unwrap();
    assert!(svg.contains(r#"data-svg-grid-cell="0""#));
}

#[test]
fn an_input_with_a_panel_box_is_never_auto_converted() {
    // `<symbol>` makes the converter fail closed. The composer must still accept
    // this panel because it already has a panel box and is skipped.
    let source = path_panel(
        "d4_already_tagged.svg",
        100,
        80,
        "<symbol id='decorative'/><g data-scale='position'><circle cx='50' cy='40' r='3'/></g>",
    );
    let output = temp_path("d4_already_tagged.out.svg");
    let run = plot(
        &source,
        &output,
        &[
            "--width",
            "100",
            "--height",
            "80",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--verbose",
        ],
    );
    assert!(
        run.status.success(),
        "a tagged input must not be converted: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!String::from_utf8_lossy(&run.stderr).contains("auto-converted"));
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"data-svg-grid-cell="0""#));
    assert!(svg.contains("<symbol"));
}

#[test]
fn unsupported_untagged_input_fails_closed_without_output() {
    let source = write_input("d4_unsupported.svg", UNSUPPORTED);
    let output = temp_path("d4_unsupported.out.svg");
    let _ = fs::remove_file(&output);
    let run = plot(
        &source,
        &output,
        &["--width", "400", "--ncol", "1", "--plot-margin", "0"],
    );
    assert_eq!(run.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("auto-conversion failed for")
            && stderr.contains("svg/symbol")
            && stderr.contains("svg_grid convert"),
        "the error must name the input, finding and fix: {stderr}"
    );
    assert!(!output.exists(), "a failed conversion must write no output");

    let planned = plan(
        &source,
        &["--width", "400", "--ncol", "1", "--plot-margin", "0"],
    );
    assert_eq!(planned.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&planned.stderr).contains("auto-conversion failed for"));
}

#[test]
fn plan_on_an_untagged_input_matches_the_real_run() {
    let source = write_input("d4_plan.svg", SVGLITE);
    let planned = plan(
        &source,
        &["--width", "500", "--ncol", "1", "--plot-margin", "0"],
    );
    assert!(
        planned.status.success(),
        "{}",
        String::from_utf8_lossy(&planned.stderr)
    );
    assert_eq!(planned.stderr.len(), 0, "plan must stay silent by default");
    let stdout = String::from_utf8_lossy(&planned.stdout);
    assert!(stdout.contains("canvas 500x375"), "{stdout}");
    assert_eq!(cell_geometries(&stdout), vec![(500.0, 375.0)]);
    assert!(stdout.contains("s=1.736x1.736"), "{stdout}");

    let output = temp_path("d4_plan.out.svg");
    let real = plot(
        &source,
        &output,
        &["--width", "500", "--ncol", "1", "--plot-margin", "0"],
    );
    assert!(real.status.success());
    let svg = fs::read_to_string(&output).unwrap();
    assert_eq!(attr_value(&svg, "width"), "500");
    assert_eq!(attr_value(&svg, "height"), "375");
}

#[test]
fn auto_conversion_is_silent_by_default_and_leaves_the_source_untouched() {
    let source = write_input("d4_source.svg", SVGLITE);
    let before = fs::read(&source).unwrap();
    let before_hash = sha256_hex(&before);
    let before_mtime = fs::metadata(&source).unwrap().modified().unwrap();

    let quiet_out = temp_path("d4_source.quiet.svg");
    let quiet = plot(
        &source,
        &quiet_out,
        &["--width", "500", "--ncol", "1", "--plot-margin", "0"],
    );
    assert!(quiet.status.success());
    assert_eq!(
        quiet.stderr.len(),
        0,
        "{}",
        String::from_utf8_lossy(&quiet.stderr)
    );

    let after = fs::read(&source).unwrap();
    assert_eq!(before, after, "the source bytes changed");
    assert_eq!(
        before_hash,
        sha256_hex(&after),
        "the source SHA-256 changed"
    );
    assert_eq!(
        before_mtime,
        fs::metadata(&source).unwrap().modified().unwrap(),
        "the source mtime changed"
    );

    let loud_out = temp_path("d4_source.loud.svg");
    let loud = plot(
        &source,
        &loud_out,
        &[
            "--width",
            "500",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--verbose",
        ],
    );
    assert!(loud.status.success());
    let stderr = String::from_utf8_lossy(&loud.stderr);
    assert!(
        stderr.contains("auto-converted") && stderr.contains("svg_grid: note:"),
        "{stderr}"
    );
    assert_eq!(fs::read(&quiet_out).unwrap(), fs::read(&loud_out).unwrap());
}

#[test]
fn mixed_tagged_and_untagged_inputs_share_one_grid() {
    let tagged = path_panel(
        "d4_mixed_tagged.svg",
        100,
        80,
        "<circle cx='50' cy='40' r='3'/>",
    );
    let untagged = write_input("d4_mixed_untagged.svg", SVGLITE);
    let output = temp_path("d4_mixed.out.svg");

    let mut args = vec![
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--plot-margin".to_string(),
        "0".to_string(),
        "--output".to_string(),
        output.to_string_lossy().into_owned(),
    ];
    for input in [&tagged, &untagged] {
        args.push("--input".to_string());
        args.push(input.to_string_lossy().into_owned());
    }
    let run = run_plot_grid(&args);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let svg = fs::read_to_string(&output).unwrap();
    assert!(svg.contains(r#"data-svg-grid-cell="0""#));
    assert!(svg.contains(r#"data-svg-grid-cell="1""#));

    let planned = vec![
        "--input".to_string(),
        tagged.to_string_lossy().into_owned(),
        "--input".to_string(),
        untagged.to_string_lossy().into_owned(),
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--plot-margin".to_string(),
        "0".to_string(),
        "--plan".to_string(),
    ];
    let planned = run_plot_grid(&planned);
    assert!(
        planned.status.success(),
        "{}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let cells = cell_geometries(&String::from_utf8_lossy(&planned.stdout));
    assert_eq!(cells.len(), 2);
    assert!((cells[0].1 - cells[1].1).abs() < 1e-3);
    assert!((cells[0].0 + cells[1].0 - 800.0).abs() < 1e-3);
}

#[allow(clippy::needless_range_loop)]
fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[test]
fn the_local_sha256_matches_the_standard_abc_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // The NIST two-block vector, so the helper is checked past one 64-byte block.
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}
