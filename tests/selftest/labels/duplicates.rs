// R44: a figure whose panels repeat a letter must fail closed (blocker report
// `docs/PITFALLS/blocker-svg_grid-duplicate-letters-across-composites.md`).

use std::fs;

use crate::common::*;

/// R44: two composites lettered separately (A,B and A) must not compose into a
/// figure where two panels answer to "A" — that used to be rc=0 with an empty
/// stderr (report `docs/PITFALLS/blocker-svg_grid-duplicate-letters-across-composites.md`).
#[test]
fn cli_rejects_duplicate_letters_across_composites() {
    let p1 = path_panel("r44_p1.svg", 100, 80, "");
    let p2 = path_panel("r44_p2.svg", 100, 80, "");
    let p3 = path_panel("r44_p3.svg", 100, 80, "");

    // Step 1 + 2 from the report: two composites, each lettered on its own.
    let (ab, ab_out) = run_cli(
        "r44_ab.svg",
        &[p1.clone(), p2.clone()],
        &["--width", "600", "--ncol", "2", "--labels"],
    );
    assert!(
        ab.status.success(),
        "{}",
        String::from_utf8_lossy(&ab.stderr)
    );
    let (c, c_out) = run_cli(
        "r44_c.svg",
        std::slice::from_ref(&p3),
        &["--width", "600", "--ncol", "1", "--labels"],
    );
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));

    // Step 3: compose them with no --labels. This is the silent duplicate.
    let out = temp_path("r44_abc.svg");
    let _ = fs::remove_file(&out);
    let result = run_plot_grid(&[
        "--input".to_string(),
        ab_out.to_string_lossy().into_owned(),
        "--input".to_string(),
        c_out.to_string_lossy().into_owned(),
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--output".to_string(),
        out.to_string_lossy().into_owned(),
    ]);
    let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
    assert_eq!(result.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    assert!(
        stderr.contains("\"A\""),
        "the letter must be named: {stderr}"
    );
    assert!(
        stderr.contains("#0") && stderr.contains("#1"),
        "both inputs must be named: {stderr}"
    );
    assert!(
        stderr.lines().all(|line| line.chars().count() <= 200),
        "one 200-char stderr line, prefix included: {stderr}"
    );
    assert!(
        !out.exists(),
        "a rejected call must not write the output file"
    );

    // `--plan` fails the same way and prints no plan.
    let plan = run_plot_grid(&[
        "--input".to_string(),
        ab_out.to_string_lossy().into_owned(),
        "--input".to_string(),
        c_out.to_string_lossy().into_owned(),
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--plan".to_string(),
    ]);
    assert_eq!(plan.status.code(), Some(2), "{plan:?}");
    assert!(
        plan.stdout.is_empty(),
        "--plan must not print a plan for a rejected figure: {}",
        String::from_utf8_lossy(&plan.stdout)
    );
    assert!(
        String::from_utf8_lossy(&plan.stderr).contains("duplicate panel letters"),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );

    // An existing output file must not be truncated by the rejected call.
    let keep = temp_path("r44_keep.svg");
    fs::write(&keep, "SENTINEL").unwrap();
    let clobber = run_plot_grid(&[
        "--input".to_string(),
        ab_out.to_string_lossy().into_owned(),
        "--input".to_string(),
        c_out.to_string_lossy().into_owned(),
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--output".to_string(),
        keep.to_string_lossy().into_owned(),
    ]);
    assert_eq!(clobber.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&keep).unwrap(), "SENTINEL");
    let _ = fs::remove_file(&keep);

    // (a) distinct letters across composites are fine.
    let (c_only, c_only_out) = run_cli(
        "r44_c_only.svg",
        std::slice::from_ref(&p3),
        &["--width", "600", "--ncol", "1", "--labels", "C"],
    );
    assert!(c_only.status.success());
    let (ok, ok_out) = run_cli(
        "r44_ok.svg",
        &[ab_out.clone(), c_only_out],
        &["--width", "800", "--ncol", "2"],
    );
    assert!(
        ok.status.success(),
        "A,B + C must compose: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let _ = fs::remove_file(&ok_out);

    // (b) a lettered composite next to a plain panel is fine.
    let (mixed, mixed_out) = run_cli(
        "r44_mixed.svg",
        &[ab_out.clone(), p1.clone()],
        &["--width", "800", "--ncol", "2"],
    );
    assert!(
        mixed.status.success(),
        "A,B + plain must compose: {}",
        String::from_utf8_lossy(&mixed.stderr)
    );
    let _ = fs::remove_file(&mixed_out);

    // (c) a single already-lettered composite on its own is fine.
    let (alone, alone_out) = run_cli(
        "r44_alone.svg",
        std::slice::from_ref(&ab_out),
        &["--width", "600"],
    );
    assert!(
        alone.status.success(),
        "a lone composite must compose: {}",
        String::from_utf8_lossy(&alone.stderr)
    );
    let _ = fs::remove_file(&alone_out);

    // (d) `--labels A,A` on plain inputs stays the W4b warning (rc=0): the user
    // wrote that collision on purpose.
    let (w4b, w4b_out) = run_cli(
        "r44_w4b.svg",
        &[p1.clone(), p2.clone()],
        &["--width", "600", "--ncol", "2", "--labels", "A,A"],
    );
    assert!(
        w4b.status.success(),
        "--labels A,A is the user's own choice: {}",
        String::from_utf8_lossy(&w4b.stderr)
    );
    let _ = fs::remove_file(&w4b_out);

    // (e) W3 keeps its own message when this call letters a lettered composite.
    let (w3, _w3_out) = run_cli(
        "r44_w3.svg",
        &[ab_out.clone(), c_out.clone()],
        &["--width", "800", "--ncol", "2", "--labels", "B,C"],
    );
    assert_eq!(w3.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&w3.stderr).contains("already carry letters"),
        "{}",
        String::from_utf8_lossy(&w3.stderr)
    );

    for path in [ab_out, c_out, p1, p2, p3] {
        let _ = fs::remove_file(path);
    }
}

/// The same collision on the repository's own figures: `A_heatmap.svg` + `B_A.svg`
/// compose to A,B, `B_B.svg` alone letters to A again.
#[test]
fn cli_rejects_duplicate_letters_in_the_tagged_corpus() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tagged = root.join("tests/fig2_mixed/out/tagged");
    let a = tagged.join("A_heatmap.svg");
    let b = tagged.join("B_A.svg");
    let bee = tagged.join("B_B.svg");
    if !a.exists() || !b.exists() || !bee.exists() {
        eprintln!("skipping: tests/fig2_mixed/out/tagged is not populated");
        return;
    }
    let (ab, ab_out) = run_cli(
        "r44_corpus_ab.svg",
        &[a, b],
        &["--width", "800", "--ncol", "2", "--labels"],
    );
    assert!(
        ab.status.success(),
        "{}",
        String::from_utf8_lossy(&ab.stderr)
    );
    let (c, c_out) = run_cli(
        "r44_corpus_c.svg",
        &[bee],
        &["--width", "600", "--ncol", "1", "--labels"],
    );
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    let combined = run_plot_grid(&[
        "--input".to_string(),
        ab_out.to_string_lossy().into_owned(),
        "--input".to_string(),
        c_out.to_string_lossy().into_owned(),
        "--width".to_string(),
        "800".to_string(),
        "--ncol".to_string(),
        "2".to_string(),
        "--plan".to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&combined.stderr).into_owned();
    assert_eq!(combined.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("duplicate panel letters"), "{stderr}");
    let _ = fs::remove_file(ab_out);
    let _ = fs::remove_file(c_out);
}
