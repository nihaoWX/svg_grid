use super::*;

fn between<'a>(svg: &'a str, start: &str, end: &str) -> &'a str {
    let from = svg.find(start).unwrap();
    let to = from + svg[from..].find(end).unwrap();
    &svg[from..to]
}

/// `markerUnits="userSpaceOnUse"` makes the viewport, the reference point and the
/// body plain user lengths, and nothing else rewrites them: without this the
/// arrowhead is clipped to a 12×12 box that no longer covers it (measured at s=2:
/// xor 622, ink 1952 -> 1459, round-data/R43).
#[test]
fn user_space_marker_viewport_and_body_follow_the_panel() {
    let svg = run_in(
        &doc(
            r##"<marker id="k" markerUnits="userSpaceOnUse" markerWidth="6" markerHeight="6" refX="0" refY="3" orient="auto"><path d="M 0 0 L 6 3 L 0 6 Z" fill="black"/></marker>"##,
            r##"<path d="M 10 30 L 90 30" stroke="black" marker-end="url(#k)"/>"##,
        ),
        DOUBLED,
    );
    let marker = between(&svg, "<marker", "</marker>");
    assert!(
        marker.contains(r#"markerWidth="12" markerHeight="12""#),
        "a user-space marker viewport follows the panel: {svg}"
    );
    assert!(marker.contains(r#"refY="6""#), "{svg}");
    assert!(
        marker.contains(r#"d="M 0 0 L 12 6 L 0 12 Z""#),
        "a user-space marker body is panel geometry: {svg}"
    );
}

/// The default `markerUnits="strokeWidth"` is the opposite case: the body is
/// relative to the vertex and the renderer scales it by the stroke width that the
/// composer already rewrote, so touching it would only clip the arrow away.
#[test]
fn default_marker_stays_in_marker_coordinates() {
    let svg = run_in(
        &doc(
            r##"<marker id="k" markerWidth="6" markerHeight="6" refX="0" refY="3" orient="auto"><path d="M 0 0 L 6 3 L 0 6 Z" fill="black"/></marker>"##,
            r##"<path d="M 10 30 L 90 30" stroke="black" marker-end="url(#k)"/>"##,
        ),
        DOUBLED,
    );
    let marker = between(&svg, "<marker", "</marker>");
    assert!(
        marker.contains(r#"markerWidth="6" markerHeight="6" refX="0" refY="3""#),
        "a strokeWidth marker must stay in marker coordinates: {svg}"
    );
    assert!(marker.contains(r#"d="M 0 0 L 6 3 L 0 6 Z""#), "{svg}");
}

/// `maskContentUnits` defaults to `userSpaceOnUse`, and an invalid value falls back
/// to that default: the body is user geometry and must be rewritten. Measured at
/// s=2 with `maskContentUnits="foo"`: xor 6800, ink 6400 -> 1600 (round-data/R43).
#[test]
fn invalid_mask_content_units_scales_the_body() {
    let svg = run_in(
        &doc(
            r##"<mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="foo" x="10" y="10" width="40" height="20"><rect x="1" y="2" width="3" height="4" fill="white"/></mask>"##,
            r##"<rect x="10" y="10" width="40" height="20" fill="black" mask="url(#m)"/>"##,
        ),
        DOUBLED,
    );
    assert!(
        svg.contains(r#"<rect x="2" y="4" width="6" height="8" fill="white"/>"#),
        "an invalid maskContentUnits means user space, so the body scales: {svg}"
    );
    assert!(
        svg.contains(r#"x="20" y="20" width="80" height="40""#),
        "the user-space mask region still follows the panel: {svg}"
    );
}

#[test]
fn strict_object_bounding_box_mask_content_stays_verbatim() {
    let svg = run_in(
        &doc(
            r##"<mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="objectBoundingBox" x="10" y="10" width="40" height="20"><rect x="0" y="0" width="0.5" height="1" fill="white"/></mask>"##,
            r##"<rect x="10" y="10" width="40" height="20" fill="black" mask="url(#m)"/>"##,
        ),
        DOUBLED,
    );
    assert!(
        svg.contains(r#"<rect x="0" y="0" width="0.5" height="1" fill="white"/>"#),
        "objectBoundingBox mask content is already relative to the box: {svg}"
    );
}

/// The renderer resolves each tile number along the `href` chain and then
/// interprets it in the **resolved** `patternUnits` — so a tile half-declared on a
/// bbox pattern and half-inherited by a user-space one is all user space. Before
/// the fix, the inherited numbers stayed unscaled (review: xor 1104, ink 1152).
#[test]
fn pattern_tile_numbers_are_resolved_along_href_in_the_resolved_units() {
    let svg = run_in(
        &doc(
            r##"<pattern id="base" x="2" y="3" width="20" height="18"><rect x="0" y="0" width="10" height="10" fill="#000000"/></pattern><pattern id="p" href="#base" patternUnits="userSpaceOnUse" x="10"><rect x="1" y="1" width="2" height="2" fill="#000000"/></pattern>"##,
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
        ),
        DOUBLED,
    );
    let p = pattern(&svg, "p");
    assert!(
        p.contains(r#"x="20" y="6" width="40" height="36""#),
        "the inherited tile numbers are user space too: {svg}"
    );
    assert!(
        p.contains(r##"<rect x="2" y="2" width="4" height="4" fill="#000000"/>"##),
        "the body is still user space and scales: {svg}"
    );
}

/// The renderer accepts `href=" #base"` (it skips literal spaces around the
/// fragment), so the composer must follow it too: before the fix the reference
/// dangled and the fill vanished (measured: ink 2160 -> 0, round-data/R43).
#[test]
fn href_with_leading_spaces_is_still_followed() {
    let svg = run_in(
        &doc(
            r##"<pattern id="base" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" x="10" y="10" width="20" height="20"><rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/></pattern><pattern id="p" href=" #base"><rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/></pattern>"##,
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
        ),
        DOUBLED,
    );
    let p = pattern(&svg, "p");
    assert!(
        p.contains(r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##),
        "a spaced IRI is still an internal reference, so the bbox body is inherited: {svg}"
    );
    assert!(
        p.contains(r#"x="20" y="20" width="40" height="40""#),
        "and the inherited user-space tile is rewritten: {svg}"
    );
}

/// `#a b` is not an IRI the renderer can follow (the fragment stops at the first
/// space and the rest must be spaces), so inheritance stops and the defaults —
/// user space content — apply.
#[test]
fn href_with_an_invalid_fragment_ends_the_chain() {
    let svg = run_in(
        &doc(
            r##"<pattern id="base" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" x="10" y="10" width="20" height="20"><rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/></pattern><pattern id="p" href="#base extra" x="10" y="10" width="20" height="20"><rect x="1" y="2" width="3" height="4" fill="#000000"/></pattern>"##,
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
        ),
        DOUBLED,
    );
    let p = pattern(&svg, "p");
    assert!(
        p.contains(r##"<rect x="2" y="4" width="6" height="8" fill="#000000"/>"##),
        "the invalid IRI stops the chain, so the body is user space and scales: {svg}"
    );
}
