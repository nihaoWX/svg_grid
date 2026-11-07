//! Unit tests for the `<path>` geometry (moved verbatim from `src/path.rs`).

use super::flatten::midpoint;
use super::*;

fn scaled(sx: f32, sy: f32) -> AxisMap {
    AxisMap {
        x_scale: sx,
        y_scale: sy,
        x_offset: 0.0,
        y_offset: 0.0,
        x_length: sx.abs(),
        y_length: sy.abs(),
    }
}

fn identity() -> AxisMap {
    scaled(1.0, 1.0)
}

#[test]
fn identity_rewrite_keeps_the_geometry() {
    let out = rewrite("M 0 0 L 10 0 L 10 10 Z", &identity()).unwrap();
    assert_eq!(out, "M 0 0 L 10 0 L 10 10 Z");
}

#[test]
fn a_uniform_scale_scales_both_axes() {
    let out = rewrite("M 10 20 L 30 40", &scaled(2.0, 2.0)).unwrap();
    assert_eq!(out, "M 20 40 L 60 80");
}

#[test]
fn x_only_scale_leaves_y_alone_and_vice_versa() {
    let map = AxisMap {
        x_scale: 3.0,
        y_scale: 1.0,
        x_offset: 0.0,
        y_offset: 5.0,
        x_length: 3.0,
        y_length: 1.0,
    };
    // H moves only x; V moves only y.
    assert_eq!(rewrite("M 0 0 H 10 V 4", &map).unwrap(), "M 0 5 H 30 V 9");
}

#[test]
fn arc_rotation_and_flags_survive_byte_for_byte() {
    // Uniform scale (sx == sy): the arc stays an `A`; rx/ry scale by axis;
    // the rotation and both flags are copied verbatim.
    let out = rewrite("M 0 0 A 5 8 30.5 1 0 10 0", &scaled(2.0, 2.0)).unwrap();
    assert_eq!(out, "M 0 0 A 10 16 30.5 1 0 20 0");
}

#[test]
fn a_rotation_of_zero_stays_an_arc_under_a_non_uniform_scale() {
    // Non-uniform, but rotation == 0: still faithful.
    let out = rewrite("M 0 0 A 5 8 0 0 1 10 0", &scaled(2.0, 3.0)).unwrap();
    assert_eq!(out, "M 0 0 A 10 24 0 0 1 20 0");
}

#[test]
fn a_rotated_arc_flattens_under_a_non_uniform_scale() {
    let out = rewrite("M 0 0 A 5 5 30 0 1 10 0", &scaled(2.0, 3.0)).unwrap();
    assert!(!out.contains('A') && !out.contains('a'), "{out}");
    assert!(out.starts_with("M 0 0 L "), "{out}");
    // The flattened end point is the (scaled) arc end point.
    assert!(out.trim_end().ends_with("20 0"), "{out}");
}

#[test]
fn flattened_arc_stays_within_the_tolerance() {
    // `rx == ry` makes the arc a circle, so the radial error at each chord's
    // midpoint (its farthest point from the arc) upper-bounds the flattening
    // error; it must stay within the tolerance.
    let subpaths = flatten("M 0 0 A 5 5 30 0 1 10 0", DEFAULT_TOLERANCE).unwrap();
    let points = &subpaths[0].points;
    let center = (5.0f32, 0.0f32);
    assert!(points.len() > 3);
    for chord in points.windows(2) {
        let mid = midpoint(chord[0], chord[1]);
        let radius = ((mid.0 - center.0).powi(2) + (mid.1 - center.1).powi(2)).sqrt();
        assert!(
            (radius - 5.0).abs() <= DEFAULT_TOLERANCE + 1e-3,
            "a chord deviates by {} (radius {radius})",
            (radius - 5.0).abs()
        );
    }
}

#[test]
fn relative_commands_are_vectors_and_implicit_repeats_expand() {
    // "m 1 2" then implicit relative linetos "3 4 5 6" (-> l 3 4, l 5 6).
    let out = rewrite("m 1 2 3 4 5 6", &scaled(2.0, 2.0)).unwrap();
    assert_eq!(out, "m 2 4 l 6 8 l 10 12");

    // Absolute moveto with implicit linetos.
    let out = rewrite("M0,0 1,1 2,2", &scaled(10.0, 10.0)).unwrap();
    assert_eq!(out, "M 0 0 L 10 10 L 20 20");

    // Relative h/v/a.
    let out = rewrite("m 0 0 h 4 v 3 a 2 2 0 0 1 2 2", &scaled(2.0, 2.0)).unwrap();
    assert_eq!(out, "m 0 0 h 8 v 6 a 4 4 0 0 1 4 4");
}

#[test]
fn exponent_and_glued_sign_numbers_parse() {
    let out = rewrite("M1e-5 1E-5 L10-5", &identity()).unwrap();
    // 1e-5 rounds to "0" under the crate's number formatting.
    assert_eq!(out, "M 0 0 L 10 -5");
}

#[test]
fn flatten_straight_lines_are_exact() {
    let subpaths = flatten("M 0 0 L 10 0 L 10 10 z", DEFAULT_TOLERANCE).unwrap();
    assert_eq!(subpaths.len(), 1);
    assert!(subpaths[0].closed);
    assert_eq!(
        subpaths[0].points,
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)]
    );
}

#[test]
fn flatten_finer_tolerance_yields_more_vertices() {
    let d = "M 0 0 C 0 10 10 10 10 0";
    let coarse = flatten(d, 1.0).unwrap()[0].points.len();
    let fine = flatten(d, 0.01).unwrap()[0].points.len();
    assert!(fine > coarse, "fine={fine} coarse={coarse}");
}

#[test]
fn bbox_is_the_extent_of_the_flattened_path() {
    let box_ = bbox("M 10 10 L 20 5 L 15 30").unwrap();
    assert_eq!(box_, (10.0, 5.0, 20.0, 30.0));
    // A lone moveto draws nothing.
    assert!(bbox("M 10 10").is_none());
}

#[test]
fn unknown_command_is_an_error() {
    assert!(rewrite("M 0 0 X 1 1", &identity()).is_err());
    assert!(flatten("M 0 0 X 1 1", DEFAULT_TOLERANCE).is_err());
}
