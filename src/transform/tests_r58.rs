//! Unit tests for R58: filter primitive lengths (`primitiveUnits`) and the
//! `font-size` of a definition body.
//!
//! Every expected number here was measured with `d1render` before the code was
//! written (oracle = the source panel rendered at the target size); the pixel
//! evidence lives in `docs/PITFALLS/round-data/R58/test.py`. These tests pin the
//! *attribute* form the renderer needs, so a refactor that changes one of these
//! strings is a rendering change, not a cosmetic one.
//!
//! Two gates are easy to invert and are covered from both sides:
//!
//! * `primitiveUnits` (default `userSpaceOnUse`) decides what a primitive's numbers
//!   mean — `filterUnits` only decides what the filter *region* means, so a
//!   `filterUnits="objectBoundingBox"` filter still scales its `stdDeviation`;
//! * `font-size` scales only **inside a definition body**, never on panel text: an
//!   ordinary `<text>` in a `data-scale="xy"` group moves but keeps its point size
//!   (`ARCHITECTURE/tag-protocol.md`).

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

/// Anisotropic 4×/5× (the cell is square, the panel is 100×80), which is where a
/// two-value attribute is required and a geometric mean is measurably wrong.
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

/// A source panel in the tag protocol: panel box plus one scaling group.
fn doc(body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">{body}</g></svg>"##
    )
}

fn position_doc(body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position">{body}</g></svg>"##
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

fn find_by_id<'a>(element: &'a Element, id: &str) -> Option<&'a Element> {
    if element.attr("id") == Some(id) {
        return Some(element);
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            if let Some(found) = find_by_id(child, id) {
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

fn attr_of_id(svg: &str, id: &str, name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find_by_id(&root, id)
        .and_then(|element| element.attr(name))
        .map(str::to_string)
}

fn filter(primitive_units: &str, primitive: &str) -> String {
    format!(
        r##"<defs><filter id="f"{primitive_units}>{primitive}</filter></defs>
<rect x="10" y="10" width="60" height="40" fill="#000000" filter="url(#f)"/>"##
    )
}

fn scaled(primitive_units: &str, primitive: &str, target: Rect) -> String {
    run_in(&doc(&filter(primitive_units, primitive)), target)
}

// --- A: `primitiveUnits` gates the primitive lengths, not `filterUnits` -------

#[test]
fn blur_std_deviation_follows_the_panel_by_default() {
    let out = scaled("", r##"<feGaussianBlur stdDeviation="2"/>"##, DOUBLED);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("4".into())
    );
}

#[test]
fn blur_std_deviation_stays_in_object_bounding_box_units() {
    let out = scaled(
        r##" primitiveUnits="objectBoundingBox""##,
        r##"<feGaussianBlur stdDeviation="2"/>"##,
        DOUBLED,
    );
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("2".into())
    );
}

#[test]
fn blur_std_deviation_gets_two_values_under_anisotropy() {
    let out = scaled("", r##"<feGaussianBlur stdDeviation="2"/>"##, STRETCHED);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("8 10".into())
    );
}

#[test]
fn offset_moves_each_axis_by_its_own_factor() {
    let out = scaled("", r##"<feOffset dx="3" dy="4"/>"##, STRETCHED);
    assert_eq!(attr(&out, "feOffset", "dx"), Some("12".into()));
    assert_eq!(attr(&out, "feOffset", "dy"), Some("20".into()));
}

#[test]
fn morphology_radius_gets_two_values_under_anisotropy() {
    let out = scaled("", r##"<feMorphology radius="2"/>"##, STRETCHED);
    assert_eq!(attr(&out, "feMorphology", "radius"), Some("8 10".into()));
}

#[test]
fn drop_shadow_scales_offset_and_blur() {
    let out = scaled(
        "",
        r##"<feDropShadow dx="2" dy="3" stdDeviation="1"/>"##,
        STRETCHED,
    );
    assert_eq!(attr(&out, "feDropShadow", "dx"), Some("8".into()));
    assert_eq!(attr(&out, "feDropShadow", "dy"), Some("15".into()));
    assert_eq!(
        attr(&out, "feDropShadow", "stdDeviation"),
        Some("4 5".into())
    );
}

#[test]
fn turbulence_base_frequency_is_divided() {
    let out = scaled("", r##"<feTurbulence baseFrequency="0.05"/>"##, DOUBLED);
    assert_eq!(
        attr(&out, "feTurbulence", "baseFrequency"),
        Some("0.025".into())
    );
}

#[test]
fn turbulence_base_frequency_keeps_two_values_per_axis() {
    let out = scaled("", r##"<feTurbulence baseFrequency="0.05"/>"##, STRETCHED);
    // 0.05/4 and 0.05/5, written with full precision: the 3-decimal writer would
    // collapse 0.0125 to 0.013 and the noise field then misses the oracle entirely
    // (measured xor 68684 vs 2).
    assert_eq!(
        attr(&out, "feTurbulence", "baseFrequency"),
        Some("0.0125 0.01".into())
    );
}

#[test]
fn turbulence_two_source_values_are_divided_per_axis() {
    let out = scaled(
        "",
        r##"<feTurbulence baseFrequency="0.04 0.05"/>"##,
        STRETCHED,
    );
    assert_eq!(
        attr(&out, "feTurbulence", "baseFrequency"),
        Some("0.01 0.01".into())
    );
}

#[test]
fn image_region_scales_all_four_attributes() {
    let out = scaled(
        "",
        r##"<feImage href="#x" x="2" y="3" width="10" height="5"/>"##,
        STRETCHED,
    );
    assert_eq!(attr(&out, "feImage", "x"), Some("8".into()));
    assert_eq!(attr(&out, "feImage", "y"), Some("15".into()));
    assert_eq!(attr(&out, "feImage", "width"), Some("40".into()));
    assert_eq!(attr(&out, "feImage", "height"), Some("25".into()));
}

#[test]
fn point_light_scales_all_three_axes() {
    let out = scaled(
        "",
        r##"<feDiffuseLighting surfaceScale="2"><fePointLight x="10" y="20" z="5"/></feDiffuseLighting>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "fePointLight", "x"), Some("20".into()));
    assert_eq!(attr(&out, "fePointLight", "y"), Some("40".into()));
    assert_eq!(attr(&out, "fePointLight", "z"), Some("10".into()));
    // Dimensionless: the surface normal's scale is not a user length.
    assert_eq!(
        attr(&out, "feDiffuseLighting", "surfaceScale"),
        Some("2".into())
    );
}

#[test]
fn specular_lighting_scales_its_light() {
    let out = scaled(
        "",
        r##"<feSpecularLighting specularExponent="20"><fePointLight x="6" y="8" z="4"/></feSpecularLighting>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "fePointLight", "x"), Some("12".into()));
    assert_eq!(attr(&out, "fePointLight", "z"), Some("8".into()));
    assert_eq!(
        attr(&out, "feSpecularLighting", "specularExponent"),
        Some("20".into())
    );
}

#[test]
fn convolve_matrix_is_not_a_user_length() {
    let out = scaled(
        "",
        r##"<feConvolveMatrix order="3" kernelMatrix="0 1 0 1 -4 1 0 1 0" divisor="1" bias="0.5" targetX="1" targetY="1" edgeMode="duplicate"/>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "feConvolveMatrix", "order"), Some("3".into()));
    assert_eq!(
        attr(&out, "feConvolveMatrix", "kernelMatrix"),
        Some("0 1 0 1 -4 1 0 1 0".into())
    );
    assert_eq!(attr(&out, "feConvolveMatrix", "bias"), Some("0.5".into()));
    assert_eq!(attr(&out, "feConvolveMatrix", "targetX"), Some("1".into()));
    assert_eq!(
        attr(&out, "feConvolveMatrix", "edgeMode"),
        Some("duplicate".into())
    );
}

#[test]
fn colours_compositing_and_flood_carry_no_user_length() {
    let out = scaled(
        "",
        r##"<feColorMatrix type="matrix" values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 1 0"/><feFlood flood-color="#123456" flood-opacity="0.5"/><feComposite operator="in" k1="0.1"/><feBlend mode="multiply"/><feMerge><feMergeNode/></feMerge><feTile/><feComponentTransfer><feFuncR type="linear" slope="2"/></feComponentTransfer>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "feFlood", "flood-color"), Some("#123456".into()));
    assert_eq!(attr(&out, "feComposite", "operator"), Some("in".into()));
    assert_eq!(attr(&out, "feFuncR", "slope"), Some("2".into()));
    assert_eq!(attr(&out, "feBlend", "mode"), Some("multiply".into()));
}

#[test]
fn spot_light_positions_scale_but_distant_light_angles_do_not() {
    // Corrected by R59: spot positions are user lengths; measured xor 18003/205 -> 0/0.
    let out = scaled(
        "",
        r##"<feSpecularLighting><feSpotLight x="10" y="20" z="30" pointsAtX="1" pointsAtY="2" specularExponent="10" limitingConeAngle="30"/></feSpecularLighting><feDiffuseLighting><feDistantLight azimuth="45" elevation="20"/></feDiffuseLighting>"##,
        DOUBLED,
    );
    for (name, want) in [
        ("x", "20"),
        ("y", "40"),
        ("z", "60"),
        ("pointsAtX", "2"),
        ("pointsAtY", "4"),
        ("specularExponent", "10"),
        ("limitingConeAngle", "30"),
    ] {
        assert_eq!(attr(&out, "feSpotLight", name), Some(want.into()), "{name}");
    }
    assert_eq!(attr(&out, "feDistantLight", "azimuth"), Some("45".into()));
    assert_eq!(attr(&out, "feDistantLight", "elevation"), Some("20".into()));
}

#[test]
fn primitive_units_is_inherited_through_href() {
    let body = r##"<defs><filter id="base" primitiveUnits="objectBoundingBox"><feGaussianBlur stdDeviation="2"/></filter>
<filter id="f" href="#base"/></defs>
<rect x="10" y="10" width="60" height="40" fill="#000000" filter="url(#f)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    let root = dom::parse(&out).unwrap();
    let base = find_by_id(&root, "base").unwrap();
    let blur = find(base, "feGaussianBlur").unwrap();
    assert_eq!(blur.attr("stdDeviation"), Some("2"));
}

#[test]
fn filter_units_object_bounding_box_does_not_block_primitive_scaling() {
    // The two units are independent: an obb *region* with the default
    // `primitiveUnits` still needs its primitives scaled.
    let out = scaled(
        r##" filterUnits="objectBoundingBox""##,
        r##"<feGaussianBlur stdDeviation="2"/>"##,
        DOUBLED,
    );
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("4".into())
    );
}

// --- C: a filter/mask region is only rewritten where it is written -----------

#[test]
fn filter_without_a_region_does_not_invent_a_zero_box() {
    // `filterUnits="userSpaceOnUse"` and no x/y/width/height: the renderer's
    // defaults (the bbox) must survive. Writing width="0" makes the filter empty
    // and the content disappears (measured ink 9532 -> 0).
    let out = scaled(
        r##" filterUnits="userSpaceOnUse""##,
        r##"<feGaussianBlur stdDeviation="2"/>"##,
        DOUBLED,
    );
    assert_eq!(attr(&out, "filter", "x"), None);
    assert_eq!(attr(&out, "filter", "y"), None);
    assert_eq!(attr(&out, "filter", "width"), None);
    assert_eq!(attr(&out, "filter", "height"), None);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("4".into())
    );
}

#[test]
fn filter_href_only_does_not_gain_the_inherited_region() {
    let body = r##"<defs><filter id="base" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80"><feGaussianBlur stdDeviation="2"/></filter>
<filter id="f" href="#base"/></defs>
<rect x="0" y="0" width="100" height="80" fill="#000000" filter="url(#f)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    let derived = attr_of_id(&out, "f", "width");
    assert_eq!(derived, None, "the href child must not write a region");
    assert_eq!(attr_of_id(&out, "base", "width"), Some("200".into()));
    assert_eq!(attr_of_id(&out, "base", "height"), Some("160".into()));
}

#[test]
fn filter_own_region_still_scales() {
    let body = r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80"><feGaussianBlur stdDeviation="2"/></filter></defs>
<rect x="0" y="0" width="100" height="80" fill="#000000" filter="url(#f)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "filter", "width"), Some("200".into()));
    assert_eq!(attr(&out, "filter", "height"), Some("160".into()));
}

#[test]
fn filter_object_bounding_box_region_stays() {
    let body = r##"<defs><filter id="f" x="0" y="0" width="1" height="1"><feGaussianBlur stdDeviation="2"/></filter></defs>
<rect x="0" y="0" width="100" height="80" fill="#000000" filter="url(#f)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "filter", "width"), Some("1".into()));
    assert_eq!(attr(&out, "filter", "height"), Some("1".into()));
}

#[test]
fn mask_without_a_region_does_not_invent_a_zero_box() {
    let body = r##"<defs><mask id="m" maskUnits="userSpaceOnUse"><rect width="100" height="80" fill="white"/></mask></defs>
<rect id="target" width="100" height="80" fill="#000000" mask="url(#m)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "mask", "x"), None);
    assert_eq!(attr(&out, "mask", "width"), None);
    assert_eq!(attr(&out, "mask", "height"), None);
    assert_eq!(attr_of_id(&out, "target", "width"), Some("200".into()));
}

#[test]
fn mask_own_region_still_scales() {
    let body = r##"<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="80"><rect width="100" height="80" fill="white"/></mask></defs>
<rect width="100" height="80" fill="#000000" mask="url(#m)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "mask", "width"), Some("200".into()));
    assert_eq!(attr(&out, "mask", "height"), Some("160".into()));
}

#[test]
fn mask_object_bounding_box_region_stays() {
    let body = r##"<defs><mask id="m" x="0" y="0" width="1" height="1"><rect width="100" height="80" fill="white"/></mask></defs>
<rect width="100" height="80" fill="#000000" mask="url(#m)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "mask", "width"), Some("1".into()));
}

// --- B: a definition body scales its own `font-size` ------------------------

fn pattern_body(body: &str) -> String {
    format!(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20">{body}</pattern></defs>
<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>"##
    )
}

#[test]
fn pattern_body_text_font_size_scales() {
    let body = pattern_body(r##"<text x="2" y="14" font-size="12" font-family="Arial">A</text>"##);
    let out = run_in(&doc(&body), DOUBLED);
    assert_eq!(attr(&out, "text", "font-size"), Some("24".into()));
    assert_eq!(attr(&out, "text", "x"), Some("4".into()));
}

#[test]
fn pattern_body_style_font_size_scales() {
    let body = pattern_body(
        r##"<text x="2" y="14" style="font-size: 12px; fill: #000000" font-family="Arial">A</text>"##,
    );
    let out = run_in(&doc(&body), DOUBLED);
    assert_eq!(
        attr(&out, "text", "style"),
        Some("font-size: 24px; fill: #000000".into())
    );
}

#[test]
fn pattern_body_tspan_font_size_scales() {
    let body = pattern_body(
        r##"<text x="2" y="14" font-family="Arial"><tspan font-size="12">A</tspan></text>"##,
    );
    let out = run_in(&doc(&body), DOUBLED);
    assert_eq!(attr(&out, "tspan", "font-size"), Some("24".into()));
}

#[test]
fn mask_body_text_font_size_scales() {
    let body = r##"<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="80"><text x="2" y="14" font-size="12" font-family="Arial" fill="white">A</text></mask></defs>
<rect width="100" height="80" fill="#000000" mask="url(#m)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "text", "font-size"), Some("24".into()));
}

#[test]
fn clip_path_body_text_font_size_scales() {
    let body = r##"<defs><clipPath id="c" clipPathUnits="userSpaceOnUse"><text x="2" y="14" font-size="12" font-family="Arial">A</text></clipPath></defs>
<rect width="100" height="80" fill="#000000" clip-path="url(#c)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "text", "font-size"), Some("24".into()));
}

#[test]
fn view_box_pattern_body_text_font_size_stays() {
    // A viewBox body is already expanded by the renderer; scaling it again is the
    // double scaling R40 fixed.
    let body = r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20" viewBox="0 0 40 20"><text x="2" y="14" font-size="12" font-family="Arial">A</text></pattern></defs>
<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>"##;
    let out = run_in(&doc(body), DOUBLED);
    assert_eq!(attr(&out, "text", "font-size"), Some("12".into()));
}

#[test]
fn panel_text_font_size_is_never_scaled() {
    // The tag protocol: `xy` remaps the coordinates, the glyph keeps its point size
    // (the paper normalisation handles the rest). Scaling it here would double-scale
    // exactly the text the protocol protects.
    let out = run_in(
        &doc(r##"<text x="10" y="40" font-size="12" font-family="Arial">Hello</text>"##),
        DOUBLED,
    );
    assert_eq!(attr(&out, "text", "font-size"), Some("12".into()));
    assert_eq!(attr(&out, "text", "x"), Some("20".into()));
}

#[test]
fn panel_text_style_font_size_is_never_scaled() {
    let out = run_in(
        &doc(r##"<text x="10" y="40" style="font-size: 12px" font-family="Arial">Hello</text>"##),
        DOUBLED,
    );
    assert_eq!(attr(&out, "text", "style"), Some("font-size: 12px".into()));
}

#[test]
fn position_panel_does_not_scale_a_pattern_body_font_size() {
    // `position` never changes a size, so the body (and its glyph) stays verbatim.
    let body = pattern_body(r##"<text x="2" y="14" font-size="12" font-family="Arial">A</text>"##);
    let out = run_in(&position_doc(&body), DOUBLED);
    assert_eq!(attr(&out, "text", "font-size"), Some("12".into()));
}

#[test]
fn position_panel_does_not_scale_filter_primitives() {
    let body = filter("", r##"<feGaussianBlur stdDeviation="2"/>"##);
    let out = run_in(&position_doc(&body), DOUBLED);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("2".into())
    );
}
