// --------------------------------------------------------------------------- #
// `<path>` support (2026-09-28 R5): one geometry written three ways must reach
// the same W6 verdict, the rewrite must match `polyline`, and the two arc rules.
// --------------------------------------------------------------------------- #

use std::fs;

use crate::common::*;

#[test]
fn three_spellings_of_one_geometry_agree_on_the_overflow() {
    // The *same* drawn geometry (x 10..172.5, y 10..40) written three ways.
    // Before R5 only the polyline was measured: the path and the `<defs>`+`<use>`
    // spelling drew the identical overflow and said nothing. All three must now
    // report the same 72.5 px right overflow.
    let polyline = path_panel(
        "spell_polyline.svg",
        100,
        80,
        r##"<polyline points="10,10 172.5,10 172.5,40" fill="#000000"/>"##,
    );
    let path = path_panel(
        "spell_path.svg",
        100,
        80,
        r##"<path d="M 10 10 L 172.5 10 L 172.5 40" fill="#000000"/>"##,
    );
    let defs_use = path_panel(
        "spell_defs.svg",
        100,
        80,
        r##"<defs><polyline id="g" points="0,0 162.5,0 162.5,30" fill="#000000"/></defs>
<use href="#g" x="10" y="10"/>"##,
    );

    let args = [
        "--width",
        "100",
        "--height",
        "80",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
    ];
    for (label, input) in [
        ("polyline", &polyline),
        ("path", &path),
        ("defs+use", &defs_use),
    ] {
        let (result, out) = run_cli(
            &format!("spell_{label}_out.svg"),
            std::slice::from_ref(input),
            &args,
        );
        assert!(result.status.success(), "{label} must compose");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("outside its own canvas box"),
            "the {label} spelling must warn (W6): {stderr}"
        );
        assert!(
            stderr.contains("right by 72.5"),
            "the {label} spelling must report the same right overflow: {stderr}"
        );
        let _ = fs::remove_file(out);
    }

    for path in [polyline, path, defs_use] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn path_rewrite_matches_polyline_coordinates() {
    // The same geometry composed at 2x: the polyline's `points` and the path's
    // `d` must decode to the very same numbers.
    let polyline = path_panel(
        "match_polyline.svg",
        100,
        80,
        r##"<g data-scale="xy"><polyline points="10,10 90,10 90,40" fill="#000000"/></g>"##,
    );
    let path = path_panel(
        "match_path.svg",
        100,
        80,
        r##"<g data-scale="xy"><path d="M 10 10 L 90 10 L 90 40" fill="#000000"/></g>"##,
    );

    let args = [
        "--width",
        "200",
        "--height",
        "160",
        "--ncol",
        "1",
        "--plot-margin",
        "0",
    ];

    let (pol_result, pol_out) = run_cli(
        "match_polyline_out.svg",
        std::slice::from_ref(&polyline),
        &args,
    );
    assert!(pol_result.status.success());
    let pol_svg = fs::read_to_string(&pol_out).unwrap();

    let (path_result, path_out) = run_cli("match_path_out.svg", std::slice::from_ref(&path), &args);
    assert!(path_result.status.success());
    let path_svg = fs::read_to_string(&path_out).unwrap();

    let polyline_points = numbers(&attr_value(&pol_svg, "points"));
    let path_points = numbers(&attr_value(&path_svg, "d"));
    assert_eq!(
        polyline_points, path_points,
        "polyline {pol_svg}\npath {path_svg}"
    );
    assert_eq!(polyline_points, vec![20.0, 20.0, 180.0, 20.0, 180.0, 80.0]);

    for file in [polyline, path, pol_out, path_out] {
        let _ = fs::remove_file(file);
    }
}

#[test]
fn arc_is_kept_for_a_uniform_scale_and_flattened_for_a_rotated_non_uniform_scale() {
    // (a) rotation 0 under a non-uniform scale: rx/ry scale per axis and the arc
    // stays an `A` (its rotation and flags are copied verbatim).
    let arc = path_panel(
        "arc_keep.svg",
        100,
        80,
        r##"<g data-scale="xy"><path d="M 10 40 A 10 20 0 0 1 50 40" fill="none" stroke="#000000"/></g>"##,
    );
    let (kept, kept_out) = run_cli(
        "arc_keep_out.svg",
        std::slice::from_ref(&arc),
        &[
            "--width",
            "200",
            "--height",
            "100",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(kept.status.success());
    let kept_svg = fs::read_to_string(&kept_out).unwrap();
    let kept_d = attr_value(&kept_svg, "d");
    assert!(
        kept_d.contains("A 20 25 0 0 1 100 50"),
        "a rotation-0 arc must stay an `A` with per-axis radii: {kept_d}"
    );

    // (b) the *same* arc rotated, under the same non-uniform scale: flattened to a
    // polyline (no `A` remains) so the shape is not silently wrong.
    let rotated = path_panel(
        "arc_flatten.svg",
        100,
        80,
        r##"<g data-scale="xy"><path d="M 10 40 A 10 20 30 0 1 50 40" fill="none" stroke="#000000"/></g>"##,
    );
    let (flattened, flattened_out) = run_cli(
        "arc_flatten_out.svg",
        std::slice::from_ref(&rotated),
        &[
            "--width",
            "200",
            "--height",
            "100",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(flattened.status.success());
    let flattened_svg = fs::read_to_string(&flattened_out).unwrap();
    let flattened_d = attr_value(&flattened_svg, "d");
    assert!(
        !flattened_d.contains('A') && !flattened_d.contains('a'),
        "a rotated arc under a non-uniform scale must be flattened: {flattened_d}"
    );
    assert!(flattened_d.starts_with("M 20 50 L "), "{flattened_d}");
    assert!(
        flattened_d.ends_with("100 50"),
        "the polyline must end at the mapped arc end: {flattened_d}"
    );

    for file in [arc, rotated, kept_out, flattened_out] {
        let _ = fs::remove_file(file);
    }
}

#[test]
fn relative_and_implicit_path_commands_are_moved() {
    // Lower-case commands are vectors: deltas scale, and `m` + implicit pairs
    // expand to relative linetos.
    let relative = path_panel(
        "rel_path.svg",
        100,
        80,
        r##"<g data-scale="xy"><path d="m 1 2 3 4 h 5 v 6" fill="#000000"/></g>"##,
    );
    let (rel, rel_out) = run_cli(
        "rel_path_out.svg",
        std::slice::from_ref(&relative),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(rel.status.success());
    let rel_svg = fs::read_to_string(&rel_out).unwrap();
    assert_eq!(attr_value(&rel_svg, "d"), "m 2 4 l 6 8 h 10 v 12");

    // Absolute `M` with implicit (absolute) linetos.
    let implicit = path_panel(
        "implicit_path.svg",
        100,
        80,
        r##"<g data-scale="xy"><path d="M0,0 1,1 2,2" fill="#000000"/></g>"##,
    );
    let (imp, imp_out) = run_cli(
        "implicit_path_out.svg",
        std::slice::from_ref(&implicit),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
        ],
    );
    assert!(imp.status.success());
    let imp_svg = fs::read_to_string(&imp_out).unwrap();
    assert_eq!(attr_value(&imp_svg, "d"), "M 0 0 L 2 2 L 4 4");

    for file in [relative, implicit, rel_out, imp_out] {
        let _ = fs::remove_file(file);
    }
}

#[test]
fn a_path_that_stays_inside_is_silent() {
    // Negative control: a path well inside its canvas puts ZERO bytes on stderr
    // (proof that the flattened box does not false-positive).
    let input = path_panel(
        "path_inside.svg",
        100,
        80,
        r##"<path d="M 10 10 L 90 10 L 90 70 Z" fill="none" stroke="#000000"/>"##,
    );
    let (result, out) = run_cli(
        "path_inside_out.svg",
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
    assert!(result.status.success());
    assert_eq!(
        result.stderr.len(),
        0,
        "a path inside the canvas must emit nothing on stderr, got: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    for file in [input, out] {
        let _ = fs::remove_file(file);
    }
}

#[test]
fn normalize_scales_path_coordinates() {
    // `--normalize-input-max-side` scales the whole design; the path geometry must
    // travel with it, or it would silently desync from the rescaled viewBox.
    // Canvas 100x80 has max side 100; normalizing to 200 is a factor of 2, and the
    // cell then matches the normalized canvas (no further scaling).
    let input = path_panel(
        "normalize_path.svg",
        100,
        80,
        r##"<path d="M 10 10 L 90 10 L 90 70" fill="none" stroke="#000000"/>"##,
    );
    let (result, out) = run_cli(
        "normalize_path_out.svg",
        std::slice::from_ref(&input),
        &[
            "--width",
            "200",
            "--height",
            "160",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--normalize-input-max-side",
            "200",
        ],
    );
    assert!(result.status.success());
    let svg = fs::read_to_string(&out).unwrap();
    assert_eq!(attr_value(&svg, "d"), "M 20 20 L 180 20 L 180 140");

    for file in [input, out] {
        let _ = fs::remove_file(file);
    }
}
