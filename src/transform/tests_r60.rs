//! Unit tests for R60: the light-source factors and the two malformed-value
//! families the R59 review found.
//!
//! Every expected string was measured with `d1render` before the code was written;
//! the pixel/table evidence lives in `docs/PITFALLS/round-data/R60/test.py`.
//!
//! * a light's `z`/`pointsAtZ` follow the **RMS** of the axis factors
//!   (`sqrt((sx²+sy²)/2)`), not the geometric mean: on a 4×5 cell the wrong factor
//!   left xor 37011 / maxdiff 206;
//! * a light's `x`/`y`/`pointsAtX`/`pointsAtY` are user-space positions, so they
//!   **translate** into the cell (a two-cell figure: xor 5277 / maxdiff 238 before);
//! * lights are user space whatever `primitiveUnits` says, so an
//!   `objectBoundingBox` filter still scales them (xor 93427 / maxdiff 242 before);
//! * `feDisplacementMap scale` is applied **quadratically** by the renderer, so the
//!   rewritten number is the axis factor's square root (`data-scale="x"`, sx=4,
//!   `scale="10"` → `20`; the linear `40` displaced four times too far);
//! * a primitive's own subregion (`x`/`y`/`width`/`height`) follows the panel
//!   (xor 16984 / maxdiff 255 before);
//! * a non-finite length is kept **byte for byte** — never rewritten to
//!   `inf`/`NaN` — and `stdDeviation=""` / `"2 3 4"` are reported instead of being
//!   dropped silently.

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

/// Anisotropic 4×/5×.
const STRETCHED: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 400.0,
};

/// The second cell of a two-column figure: a pure translation.
const SHIFTED: Rect = Rect {
    x: 100.0,
    y: 30.0,
    width: 100.0,
    height: 80.0,
};

fn run_in(svg: &str, target: Rect) -> String {
    let mut root = dom::parse(svg).unwrap();
    transform_svg(&mut root, SRC, target, 0, &mut Passthroughs::default()).unwrap();
    dom::serialize(&root).unwrap()
}

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

/// A `<filter>` with the default `primitiveUnits` (`userSpaceOnUse`) around `inner`.
fn filter(inner: &str) -> String {
    format!(
        r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">{inner}</filter></defs>
<rect x="10" y="10" width="80" height="60" fill="#000000" filter="url(#f)"/>"##
    )
}

fn scaled(policy: &str, inner: &str, target: Rect) -> String {
    run_in(&doc(policy, &filter(inner)), target)
}

fn lighting(inner: &str) -> String {
    format!(
        r##"<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">{inner}</feDiffuseLighting>"##
    )
}

// --- P1-1: z / pointsAtZ are the RMS of the axis factors ---------------------

#[test]
fn spot_light_z_is_the_rms_of_the_axis_factors() {
    // sx=4/sy=5: the renderer measures z with sqrt((16+25)/2) = 4.5276928, so
    // 10 → 45.276928 and 5 → 22.638464. The geometric mean would be 44.721 and
    // 22.361 and left xor 37011 / maxdiff 206 (docs/PITFALLS/round-data/R60).
    let out = scaled(
        "xy",
        &lighting(
            r##"<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="5" limitingConeAngle="30"/>"##,
        ),
        STRETCHED,
    );
    assert_eq!(attr(&out, "feSpotLight", "x"), Some("80".into()));
    assert_eq!(attr(&out, "feSpotLight", "y"), Some("100".into()));
    assert_eq!(attr(&out, "feSpotLight", "z"), Some("45.276928".into()));
    assert_eq!(
        attr(&out, "feSpotLight", "pointsAtZ"),
        Some("22.638464".into())
    );
}

#[test]
fn point_light_z_is_the_rms_of_the_axis_factors() {
    // The point light shares the bug (R58 inherited it too): xor 95008 before.
    let out = scaled(
        "xy",
        &lighting(r##"<fePointLight x="20" y="20" z="10"/>"##),
        STRETCHED,
    );
    assert_eq!(attr(&out, "fePointLight", "z"), Some("45.276928".into()));
}

#[test]
fn light_z_stays_the_plain_factor_on_an_isotropic_cell() {
    // The RMS and the geometric mean agree here, which is why R58/R59 could not see
    // the difference: 10 * 2 = 20.
    let out = scaled(
        "xy",
        &lighting(r##"<fePointLight x="20" y="20" z="10"/>"##),
        DOUBLED,
    );
    assert_eq!(attr(&out, "fePointLight", "z"), Some("20".into()));
}

// --- P1-2: light positions translate into the cell ---------------------------

#[test]
fn light_positions_translate_into_a_shifted_cell() {
    // The second cell of a two-column figure: a light at x=20 must land at 120, not
    // 80 (measured xor 5277 / maxdiff 238 with the scale-only rewrite).
    let out = scaled(
        "xy",
        &lighting(
            r##"<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0"/>"##,
        ),
        SHIFTED,
    );
    assert_eq!(attr(&out, "feSpotLight", "x"), Some("120".into()));
    assert_eq!(attr(&out, "feSpotLight", "y"), Some("50".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtX"), Some("150".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtY"), Some("70".into()));
    // z is a distance: translation must not touch it.
    assert_eq!(attr(&out, "feSpotLight", "z"), Some("10".into()));
}

#[test]
fn point_light_positions_translate_into_a_shifted_cell() {
    let out = scaled(
        "xy",
        &lighting(r##"<fePointLight x="20" y="20"/>"##),
        SHIFTED,
    );
    assert_eq!(attr(&out, "fePointLight", "x"), Some("120".into()));
    assert_eq!(attr(&out, "fePointLight", "y"), Some("50".into()));
}

#[test]
fn an_absent_light_position_is_still_translated() {
    // A light with no position sits at the renderer's default 0, which is the origin
    // of the *source* panel: in the second cell that origin is (100, 30), so the
    // attribute has to appear.
    let out = scaled("xy", &lighting(r##"<fePointLight z="10"/>"##), SHIFTED);
    assert_eq!(attr(&out, "fePointLight", "x"), Some("100".into()));
    assert_eq!(attr(&out, "fePointLight", "y"), Some("30".into()));
}

// --- P1-3: lights are user space even under objectBoundingBox ----------------

#[test]
fn object_bounding_box_lighting_still_scales_the_light() {
    // usvg keeps the six light numbers as plain f32, independent of
    // `primitiveUnits`; skipping the whole lighting rewrite left xor 93427 /
    // maxdiff 242. The genuinely box-relative lengths stay untouched.
    let body = r##"<defs><filter id="f" primitiveUnits="objectBoundingBox" x="-0.1" y="-0.1" width="1.2" height="1.2"><feDiffuseLighting surfaceScale="2"><feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0" limitingConeAngle="30"/></feDiffuseLighting><feGaussianBlur stdDeviation="2"/></filter></defs><rect x="10" y="10" width="80" height="60" fill="#336699" filter="url(#f)"/>"##;
    let out = run_in(&doc("xy", body), STRETCHED);
    assert_eq!(attr(&out, "feSpotLight", "x"), Some("80".into()));
    assert_eq!(attr(&out, "feSpotLight", "y"), Some("100".into()));
    assert_eq!(attr(&out, "feSpotLight", "z"), Some("45.276928".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtX"), Some("200".into()));
    assert_eq!(attr(&out, "feSpotLight", "pointsAtY"), Some("200".into()));
    // An obb fraction is not a length.
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("2".into())
    );
}

// --- P2-1: feDisplacementMap scale takes the square root of the factor -------

#[test]
fn displacement_scale_is_the_square_root_of_the_axis_factor() {
    // The renderer applies `scale` quadratically (`scale²·axis`), so under
    // `data-scale="x"` with sx=4 the exact value for `10` is `20` — measured xor 0
    // against the oracle, while the linear `40` displaced four times too far
    // (xor 6061 / maxdiff 255).
    let out = scaled(
        "x",
        r##"<feDisplacementMap in="SourceGraphic" in2="SourceGraphic" scale="10"/>"##,
        STRETCHED,
    );
    assert_eq!(attr(&out, "feDisplacementMap", "scale"), Some("20".into()));
}

#[test]
fn displacement_scale_under_y_is_the_square_root_of_the_y_factor() {
    // sqrt(5) * 10 = 22.361.
    let out = scaled(
        "y",
        r##"<feDisplacementMap in="SourceGraphic" in2="SourceGraphic" scale="10"/>"##,
        STRETCHED,
    );
    assert_eq!(
        attr(&out, "feDisplacementMap", "scale"),
        Some("22.361".into())
    );
}

#[test]
fn displacement_scale_anisotropically_uses_the_geometric_mean_of_the_exact_values() {
    // No single number is exact when sx ≠ sy: sqrt(40) for x and sqrt(50) for y, so
    // the fallback is their geometric mean, (4·5)^(1/4) * 10 = 21.147 (the old
    // geometric mean of the *factors*, 44.721, displaced 4.5× too far).
    let out = scaled(
        "xy",
        r##"<feDisplacementMap in="SourceGraphic" in2="SourceGraphic" scale="10"/>"##,
        STRETCHED,
    );
    assert_eq!(
        attr(&out, "feDisplacementMap", "scale"),
        Some("21.147".into())
    );
}

// --- P2-2: a primitive's own subregion follows the panel ---------------------

#[test]
fn primitive_subregion_scales_and_translates() {
    // `x`/`y`/`width`/`height` on the primitive are user lengths under
    // `primitiveUnits="userSpaceOnUse"` (usvg resolve_primitive_region). Leaving them
    // alone clipped the primitive to the unscaled box (xor 16984 / maxdiff 255).
    let out = scaled(
        "xy",
        r##"<feGaussianBlur stdDeviation="2" x="10" y="10" width="80" height="60"/>"##,
        DOUBLED,
    );
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("4".into())
    );
    assert_eq!(attr(&out, "feGaussianBlur", "x"), Some("20".into()));
    assert_eq!(attr(&out, "feGaussianBlur", "y"), Some("20".into()));
    assert_eq!(attr(&out, "feGaussianBlur", "width"), Some("160".into()));
    assert_eq!(attr(&out, "feGaussianBlur", "height"), Some("120".into()));
}

#[test]
fn primitive_subregion_stays_a_fraction_under_object_bounding_box() {
    // Under obb the same four numbers are fractions of the filtered box, which already
    // follows the panel; scaling them again would shrink the primitive.
    let body = r##"<defs><filter id="f" primitiveUnits="objectBoundingBox"><feGaussianBlur stdDeviation="0.1" x="0.1" y="0.1" width="0.8" height="0.6"/></filter></defs><rect x="10" y="10" width="80" height="60" filter="url(#f)"/>"##;
    let out = run_in(&doc("xy", body), DOUBLED);
    assert_eq!(attr(&out, "feGaussianBlur", "x"), Some("0.1".into()));
    assert_eq!(attr(&out, "feGaussianBlur", "width"), Some("0.8".into()));
    assert_eq!(attr(&out, "feGaussianBlur", "height"), Some("0.6".into()));
}

// --- P2-4: malformed and non-finite values --------------------------------

fn w7(svg: &str, element: &str, attr_name: &str) -> String {
    let (out, passthroughs) = run_with(svg, DOUBLED);
    // The rewrite must never *introduce* a non-finite number: `inf`/`NaN` may only
    // appear where the author wrote it (kept verbatim), never as a rewrite result.
    for token in ["inf", "NaN"] {
        assert!(
            out.matches(token).count() <= svg.matches(token).count(),
            "the output introduced {token}: {out}"
        );
    }
    let offender = passthroughs
        .offenders()
        .iter()
        .find(|offender| offender.element == element && offender.attr == attr_name)
        .unwrap_or_else(|| panic!("{element} {attr_name} was not reported as W7"));
    offender.detail.clone()
}

#[test]
fn non_finite_primitive_length_is_kept_verbatim() {
    // `1e999` parses to `inf` in Rust and used to be written back as `"inf"`, which is
    // not a legal SVG number — while the warning said "kept verbatim".
    for raw in ["1e999", "NaN", "-inf"] {
        let svg = doc(
            "xy",
            &filter(&format!(r##"<feGaussianBlur stdDeviation="{raw}"/>"##)),
        );
        let detail = w7(&svg, "feGaussianBlur", "stdDeviation");
        assert!(detail.contains(raw), "W7 should quote {raw:?}: {detail}");
        assert_eq!(
            attr(&run_in(&svg, DOUBLED), "feGaussianBlur", "stdDeviation"),
            Some(raw.to_string()),
            "{raw} must survive byte for byte"
        );
    }
}

#[test]
fn non_finite_plain_length_is_kept_verbatim_too() {
    // The same path serves `stroke-width`, `x`, ... (R58 inherited it; the R50
    // baseline already wrote `inf` here).
    let body = r##"<rect x="10" y="10" width="10" height="10" fill="none" stroke="#000000" stroke-width="1e999"/>"##;
    let svg = doc("xy", body);
    let detail = w7(&svg, "rect", "stroke-width");
    assert!(
        detail.contains("1e999"),
        "W7 should quote the value: {detail}"
    );
    let out = run_in(&svg, DOUBLED);
    assert!(out.contains(r##"stroke-width="1e999""##), "{out}");
}

#[test]
fn non_finite_style_length_is_kept_verbatim() {
    let body =
        r##"<rect x="10" y="10" width="10" height="10" fill="none" style="stroke-width:1e999"/>"##;
    let out = run_in(&doc("xy", body), DOUBLED);
    assert!(out.contains("stroke-width:1e999"), "{out}");
    assert!(!out.contains("inf"), "{out}");
}

#[test]
fn malformed_number_optional_number_is_reported_not_dropped() {
    // usvg's `convert_std_dev_attr` takes nothing from either spelling and blurs with
    // zero deviation, so the text is kept — but silently keeping it hid a broken
    // figure.
    for raw in ["", " ", "2 3 4"] {
        let svg = doc(
            "xy",
            &filter(&format!(r##"<feGaussianBlur stdDeviation="{raw}"/>"##)),
        );
        let detail = w7(&svg, "feGaussianBlur", "stdDeviation");
        assert!(
            detail.contains("expected one or two numbers"),
            "W7 should say what the attribute wanted: {detail}"
        );
        assert_eq!(
            attr(&run_in(&svg, DOUBLED), "feGaussianBlur", "stdDeviation"),
            Some(raw.to_string()),
            "{raw:?} must survive byte for byte"
        );
    }
}
