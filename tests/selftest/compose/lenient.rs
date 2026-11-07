// --------------------------------------------------------------------------- #
// Lenient `<path>` / non-finite numbers (2026-09-28 R6): these are W7, not
// fail-closed. Structural garbage in a plain attribute stays fail-closed.
// --------------------------------------------------------------------------- #

use std::fs;

use crate::common::*;

#[test]
fn malformed_path_d_is_passed_through_verbatim_with_a_warning() {
    // The exact reproducer from the contract: a moveto with an odd argument
    // count used to abort the whole figure with exit 1. It must now compose
    // (exit 0), say so on stderr (W7) and keep `d` byte-for-byte.
    for (name, d) in [("odd", "M 10 20 30"), ("nan", "M0 0 Lnan nan")] {
        let (result, input, out) = compose_one(
            &format!("pathd_{name}"),
            &format!(r##"<path d="{d}" fill="none" stroke="#000000"/>"##),
        );
        assert!(
            result.status.success(),
            "a malformed d must not fail the run ({d}): {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("input #0"), "{d}: {stderr}");
        assert!(stderr.contains("<path d>"), "{d}: {stderr}");
        assert!(stderr.contains("kept verbatim"), "{d}: {stderr}");
        let svg = fs::read_to_string(&out).unwrap();
        assert_eq!(attr_value(&svg, "d"), d, "d must not change one byte");

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(out);
    }
}

#[test]
fn malformed_path_d_does_not_suppress_w6_for_other_overflowing_geometry() {
    // Leniency is not a licence to ignore an overflow: the malformed path is
    // passed through AND the overflowing rect still raises W6 (before R6 the
    // whole run aborted, so W6 never got a chance to be printed at all).
    let (result, input, out) = compose_one(
        "pathd_overflow",
        r##"<path d="M 10 20 30" fill="none" stroke="#000000"/>
<rect x="0" y="0" width="140" height="80" fill="#000000"/>"##,
    );
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("outside its own canvas box"), "{stderr}");
    assert!(stderr.contains("right by 40.000 px"), "{stderr}");
    assert!(
        stderr.contains("<path d>") && stderr.contains("kept verbatim"),
        "{stderr}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

#[test]
fn non_finite_numeric_attribute_warns_but_keeps_the_existing_bytes() {
    // `<circle cx="nan">` used to write `cx="NaN"` *silently*; now it says so —
    // and the bytes it writes stay exactly the same (exit code stays 0).
    let (result, input, out) = compose_one(
        "circle_nan",
        r##"<circle cx="nan" cy="10" r="5" fill="#000000"/>"##,
    );
    assert!(
        result.status.success(),
        "a non-finite number must not fail the run"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("input #0"), "{stderr}");
    assert!(stderr.contains("<circle cx>"), "{stderr}");
    assert!(stderr.contains("non-finite"), "{stderr}");
    assert!(stderr.contains("kept verbatim"), "{stderr}");
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(
        attr_value(&svg, "cx"),
        "NaN",
        "the bytes are unchanged: {svg}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

#[test]
fn non_finite_length_is_flagged_even_when_the_policy_leaves_it_unscaled() {
    // `r="nan"` under a `position`/`none` policy is not rewritten (so it is
    // already verbatim) — but a non-finite number must still be surfaced.
    let (result, input, out) = compose_one(
        "circle_r_nan",
        r##"<circle cx="10" cy="10" r="nan" fill="#000000"/>"##,
    );
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("<circle r>"), "{stderr}");
    assert!(stderr.contains("non-finite"), "{stderr}");
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(attr_value(&svg, "r"), "nan", "untouched under this policy");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

#[test]
fn a_structurally_garbage_attribute_still_fails_closed() {
    // `rect x="abc"` is a different error (it cannot come from the converter and
    // is not a browser-lenient number), so the contract is unchanged: exit 1.
    let (result, input, out) = compose_one(
        "rect_abc",
        r##"<rect x="abc" y="0" width="10" height="10" fill="#000000"/>"##,
    );
    assert!(
        !result.status.success(),
        "a non-numeric attribute must still fail closed"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid numeric SVG attribute"),
        "the error must name the attribute: {stderr}"
    );
    assert!(!out.exists(), "a failed run must not leave an output");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

#[test]
fn w7_speaks_once_for_repeated_malformed_paths() {
    // Two malformed paths ⇒ a single W7 line that names both (no screen spam).
    let (result, input, out) = compose_one(
        "pathd_dedup",
        r##"<path d="M 10 20 30" fill="none" stroke="#000000"/>
<path d="M0 0 Lnan nan" fill="none" stroke="#000000"/>"##,
    );
    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    let w7_lines = stderr
        .lines()
        .filter(|line| line.contains("could not be rewritten"))
        .count();
    assert_eq!(w7_lines, 1, "W7 must be one line per call: {stderr}");
    // Both offenders must be named. `kept verbatim` is said once (in the head),
    // and each offender keeps the value that tells it apart.
    assert_eq!(
        stderr.matches("kept verbatim").count(),
        1,
        "`kept verbatim` is a per-run statement, not a per-offender one: {stderr}"
    );
    assert_eq!(
        stderr.matches("input #0").count(),
        2,
        "both offenders must be listed: {stderr}"
    );
    assert!(
        stderr.contains("M 10 20 30") && stderr.contains("M0 0 Lnan nan"),
        "each offender keeps its own value: {stderr}"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(out);
}

#[test]
fn w6_dust_hint_is_added_below_two_px_and_suppressed_at_or_above() {
    // 1.5 px: a real overflow (still reported by the 1 px tolerance) but with the
    // "dust" hint; 40 px: no hint.
    let (small, small_in, small_out) = compose_one(
        "dust_small",
        r##"<rect x="0" y="0" width="101.5" height="80" fill="#000000"/>"##,
    );
    assert!(small.status.success());
    let small_stderr = String::from_utf8_lossy(&small.stderr);
    assert!(small_stderr.contains("right by 1.500 px"), "{small_stderr}");
    assert!(
        small_stderr.contains("all overflows < 2 px"),
        "{small_stderr}"
    );

    let (big, big_in, big_out) = compose_one(
        "dust_big",
        r##"<rect x="0" y="0" width="140" height="80" fill="#000000"/>"##,
    );
    assert!(big.status.success());
    let big_stderr = String::from_utf8_lossy(&big.stderr);
    assert!(big_stderr.contains("right by 40.000 px"), "{big_stderr}");
    assert!(!big_stderr.contains("all overflows < 2 px"), "{big_stderr}");

    for file in [small_in, small_out, big_in, big_out] {
        let _ = fs::remove_file(file);
    }
}
