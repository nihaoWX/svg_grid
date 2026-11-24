use super::*;
use crate::dom::{parse, serialize};

fn normalized(svg: &str, max_side: f32) -> String {
    let mut root = parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    normalize_svg(&mut root, max_side, 0, &mut passthroughs).unwrap();
    serialize(&root).unwrap()
}

#[test]
fn an_object_bounding_box_pattern_keeps_its_own_stroke_width() {
    let text = normalized(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
              <pattern id="h" width="0.25" height="0.25"
                       patternContentUnits="objectBoundingBox" stroke-width="0.1">
                <rect x="0" y="0" width="0.1" height="0.1" fill="none" stroke="#000"/>
              </pattern>
              <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
            </svg>"##,
        50.0,
    );
    assert!(text.contains(r#"stroke-width="0.1""#), "{text}");
    assert!(!text.contains(r#"stroke-width="0.050""#), "{text}");
}

#[test]
fn a_user_space_pattern_scales_its_own_stroke_width() {
    let text = normalized(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
              <pattern id="h" patternUnits="userSpaceOnUse"
                       patternContentUnits="userSpaceOnUse" x="0" y="0"
                       width="20" height="20" stroke-width="2">
                <rect x="0" y="0" width="6" height="6" fill="none" stroke="#000"/>
              </pattern>
              <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
            </svg>"##,
        50.0,
    );
    assert!(text.contains(r#"stroke-width="1""#), "{text}");
}

#[test]
fn a_pattern_with_a_view_box_keeps_its_own_stroke_width() {
    let text = normalized(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
              <pattern id="h" patternUnits="userSpaceOnUse"
                       patternContentUnits="userSpaceOnUse" x="0" y="0"
                       width="20" height="20" viewBox="0 0 20 20" stroke-width="2">
                <rect x="0" y="0" width="6" height="6" fill="none" stroke="#000"/>
              </pattern>
              <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
            </svg>"##,
        50.0,
    );
    assert!(text.contains(r#"stroke-width="2""#), "{text}");
}

#[test]
fn an_overflowing_coordinate_is_kept_verbatim_with_w7() {
    let mut root = parse(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
              <rect x="1e38" y="10" width="20" height="20"/>
            </svg>"##,
    )
    .unwrap();
    let mut passthroughs = Passthroughs::default();
    normalize_svg(&mut root, 400.0, 0, &mut passthroughs).unwrap();
    let text = serialize(&root).unwrap();
    assert!(text.contains(r#"x="1e38""#), "{text}");
    assert!(!text.contains("inf") && !text.contains("NaN"), "{text}");
    assert!(
        passthroughs
            .offenders()
            .iter()
            .any(|offender| offender.attr == "x"),
        "{:?}",
        passthroughs.offenders()
    );
}

#[test]
fn a_non_finite_normalization_factor_skips_the_input() {
    let mut root = parse(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1e-45" height="1e-45">
              <rect x="10" y="10" width="20" height="20"/>
            </svg>"##,
    )
    .unwrap();
    let mut passthroughs = Passthroughs::default();
    normalize_svg(&mut root, 400.0, 3, &mut passthroughs).unwrap();
    let text = serialize(&root).unwrap();
    assert!(text.contains(r#"x="10""#), "{text}");
    assert!(!text.contains("inf") && !text.contains("NaN"), "{text}");
    let offender = passthroughs
        .offenders()
        .iter()
        .find(|offender| offender.detail.contains("non-finite normalization factor"))
        .expect("non-finite factor must be reported");
    assert_eq!(offender.index, 3);
    assert_eq!(offender.element, "svg");
}

#[test]
fn ordinary_coordinates_still_scale() {
    let mut root = parse(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
              <rect x="10" y="10" width="20" height="20"/>
            </svg>"##,
    )
    .unwrap();
    let mut passthroughs = Passthroughs::default();
    normalize_svg(&mut root, 200.0, 0, &mut passthroughs).unwrap();
    let text = serialize(&root).unwrap();
    assert!(text.contains(r#"x="20""#), "{text}");
    assert!(text.contains(r#"width="40""#), "{text}");
    assert!(passthroughs.offenders().is_empty());
}
