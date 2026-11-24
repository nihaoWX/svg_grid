//! Unit tests for R59: the four residuals the R58 review left behind.
//!
//! Every expected string was measured with `d1render` before the code was
//! written; the pixel/table evidence lives in
//! `docs/PITFALLS/round-data/R59/test.py`.
//!
//! * `feSpotLight`'s `x`/`y`/`z`/`pointsAtX`/`pointsAtY`/`pointsAtZ` are
//!   **lengths** in primitive units (only `specularExponent` and
//!   `limitingConeAngle` are dimensionless), unlike `feDistantLight`'s
//!   `azimuth`/`elevation`;
//! * a one-axis `data-scale` still has to rewrite a scalar `<number-optional-number>`
//!   on **both** axes: the missing factor is `1`, so `x` turns `2` into `8 2`;
//! * a malformed primitive length is kept verbatim with W7 and the run stays rc=0;
//! * an `objectBoundingBox` `<clipPath>` body keeps its fractional `font-size`.

use super::transform_svg;
use crate::dom::{self, Element, Node};
use crate::geom::Rect;
use crate::warnings::Passthroughs;

const SRC: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 100.0,
    height: 80.0,
};

/// Isotropic 2×.
const DOUBLED: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
};

/// Anisotropic 4×/5×: a square cell for a 100×80 panel.
const STRETCHED: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 400.0,
};

fn run_in(svg: &str, target: Rect) -> String {
    let mut root = dom::parse(svg).unwrap();
    transform_svg(&mut root, SRC, target, 0, &mut Passthroughs::default()).unwrap();
    dom::serialize(&root).unwrap()
}

/// Like [`run_in`], but keeps the W7 offenders for the malformed-value tests.
fn run_with(svg: &str, target: Rect) -> (String, Passthroughs) {
    let mut root = dom::parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, SRC, target, 0, &mut passthroughs).unwrap();
    (dom::serialize(&root).unwrap(), passthroughs)
}

fn doc(policy: &str, body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="{policy}">{body}</g></svg>"##
    )
}

fn find<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    if element.name == name {
        return Some(element);
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            if let Some(found) = find(child, name) {
                return Some(found);
            }
        }
    }
    None
}

fn attr(svg: &str, element: &str, name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find(&root, element)
        .and_then(|element| element.attr(name))
        .map(str::to_string)
}

fn filter(inner: &str) -> String {
    format!(
        r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">{inner}</filter></defs>
<rect x="10" y="10" width="80" height="60" fill="#000000" filter="url(#f)"/>"##
    )
}

fn scaled(policy: &str, inner: &str, target: Rect) -> String {
    run_in(&doc(policy, &filter(inner)), target)
}

// --- E1: feSpotLight positions are user lengths ------------------------------

#[test]
fn spot_light_positions_scale_isotropically() {
    let out = scaled(
        "xy",
        r##"<feDiffuseLighting surfaceScale="2" diffuseConstant="1">
<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0" specularExponent="5" limitingConeAngle="30"/>
</feDiffuseLighting>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "feSpotLight", "x"), Some("40".into()));
    assert_eq!(attr(&out, "feSpotLight", "y"), Some("40".into()));
    assert_eq!(attr(&out, "feSpotLight", "z"), Some("20".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtX"), Some("100".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtY"), Some("80".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtZ"), Some("0".into()));
    // The two dimensionless attributes must not move: turning a cone angle or an
    // exponent into a length factor would silently change the highlight.
    assert_eq!(
        attr(&out, "feSpotLight", "specularExponent"),
        Some("5".into())
    );
    assert_eq!(
        attr(&out, "feSpotLight", "limitingConeAngle"),
        Some("30".into())
    );
}

#[test]
fn spot_light_positions_take_one_factor_per_axis() {
    // sx=4/sy=5: x/y/pointsAtX/pointsAtY follow their own axis, z/pointsAtZ the
    // geometric mean (10 * sqrt(20) = 44.721).
    let out = scaled(
        "xy",
        r##"<feSpecularLighting surfaceScale="2" specularConstant="1" specularExponent="20">
<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0" limitingConeAngle="30"/>
</feSpecularLighting>"##,
        STRETCHED,
    );
    assert_eq!(attr(&out, "feSpotLight", "x"), Some("80".into()));
    assert_eq!(attr(&out, "feSpotLight", "y"), Some("100".into()));
    assert_eq!(attr(&out, "feSpotLight", "z"), Some("44.721".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtX"), Some("200".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtY"), Some("200".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtZ"), Some("0".into()));
    assert_eq!(
        attr(&out, "feSpotLight", "limitingConeAngle"),
        Some("30".into())
    );
}

#[test]
fn distant_light_stays_angular() {
    // The other light source really is angular: azimuth/elevation must not scale,
    // or the sun moves when the panel does.
    let out = scaled(
        "xy",
        r##"<feDiffuseLighting><feDistantLight azimuth="45" elevation="20"/></feDiffuseLighting>"##,
        STRETCHED,
    );
    assert_eq!(attr(&out, "feDistantLight", "azimuth"), Some("45".into()));
    assert_eq!(attr(&out, "feDistantLight", "elevation"), Some("20".into()));
}

// --- E2: a one-axis policy still writes both numbers -------------------------

#[test]
fn x_policy_writes_a_pair_for_a_scalar_blur() {
    // Source 100→400, y untouched: a scalar stdDeviation applies to both
    // directions, so the y half must be written explicitly as 2 (factor 1).
    let out = scaled("x", r##"<feGaussianBlur stdDeviation="2"/>"##, STRETCHED);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("8 2".into())
    );
}

#[test]
fn y_policy_writes_a_pair_for_a_scalar_blur() {
    let out = scaled("y", r##"<feGaussianBlur stdDeviation="2"/>"##, STRETCHED);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("2 10".into())
    );
}

#[test]
fn one_axis_policy_scales_each_source_pair_on_its_own_axis() {
    let x_out = scaled("x", r##"<feGaussianBlur stdDeviation="3 4"/>"##, STRETCHED);
    assert_eq!(
        attr(&x_out, "feGaussianBlur", "stdDeviation"),
        Some("12 4".into())
    );
    let y_out = scaled("y", r##"<feGaussianBlur stdDeviation="3 4"/>"##, STRETCHED);
    assert_eq!(
        attr(&y_out, "feGaussianBlur", "stdDeviation"),
        Some("3 20".into())
    );
}

#[test]
fn one_axis_policy_scales_morphology_and_turbulence_too() {
    let radius = scaled(
        "x",
        r##"<feMorphology operator="dilate" radius="2"/>"##,
        STRETCHED,
    );
    assert_eq!(attr(&radius, "feMorphology", "radius"), Some("8 2".into()));
    // A frequency is divided on the scaled axis and kept on the untouched one.
    let frequency = scaled(
        "x",
        r##"<feTurbulence baseFrequency="0.05" numOctaves="1" seed="3"/>"##,
        STRETCHED,
    );
    assert_eq!(
        attr(&frequency, "feTurbulence", "baseFrequency"),
        Some("0.0125 0.05".into())
    );
    let frequency_y = scaled(
        "y",
        r##"<feTurbulence baseFrequency="0.05" numOctaves="1" seed="3"/>"##,
        STRETCHED,
    );
    assert_eq!(
        attr(&frequency_y, "feTurbulence", "baseFrequency"),
        Some("0.05 0.01".into())
    );
}

// --- E3: a malformed primitive length is kept verbatim + W7 ------------------

fn assert_kept(inner: &str, element: &str, attr_name: &str, value: &str) {
    let (out, passthroughs) = run_with(&doc("xy", &filter(inner)), DOUBLED);
    // `transform_svg` returning `Ok` is the rc=0 contract: a bad primitive value is
    // not a structural input error (R58 returned `InvalidInput` here).
    assert_eq!(attr(&out, element, attr_name), Some(value.into()));
    let offender = passthroughs
        .offenders()
        .iter()
        .find(|offender| offender.element == element && offender.attr == attr_name)
        .unwrap_or_else(|| panic!("{element} {attr_name}={value} was not reported as W7"));
    assert!(
        offender.detail.contains(value),
        "the W7 detail should quote {value:?}: {}",
        offender.detail
    );
}

#[test]
fn malformed_blur_std_deviation_is_kept_verbatim() {
    assert_kept(
        r##"<feGaussianBlur stdDeviation="2px"/>"##,
        "feGaussianBlur",
        "stdDeviation",
        "2px",
    );
    assert_kept(
        r##"<feGaussianBlur stdDeviation="abc"/>"##,
        "feGaussianBlur",
        "stdDeviation",
        "abc",
    );
}

#[test]
fn malformed_offset_radius_and_frequency_are_kept_verbatim() {
    assert_kept(r##"<feOffset dx="2px" dy="5"/>"##, "feOffset", "dx", "2px");
    assert_kept(
        r##"<feMorphology operator="dilate" radius="2px"/>"##,
        "feMorphology",
        "radius",
        "2px",
    );
    assert_kept(
        r##"<feTurbulence baseFrequency="abc" numOctaves="1" seed="3"/>"##,
        "feTurbulence",
        "baseFrequency",
        "abc",
    );
}

// --- E4: an obb clipPath body keeps a fractional font-size -------------------

#[test]
fn object_bounding_box_clip_path_keeps_its_fractional_font_size() {
    let body = r##"<defs><clipPath id="c" clipPathUnits="objectBoundingBox">
<text x="0.05" y="0.7" font-family="Arial" font-size="0.2">AB</text></clipPath></defs>
<rect x="25" y="15" width="50" height="50" fill="#000000" clip-path="url(#c)"/>"##;
    let out = run_in(&doc("xy", body), DOUBLED);
    // 0.2 is a fraction of the bounding box, not a user-space point size: scaling
    // it to 0.4 describes a glyph twice the box height.
    assert_eq!(attr(&out, "text", "font-size"), Some("0.2".into()));
}
