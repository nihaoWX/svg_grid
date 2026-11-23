//! Unit tests for the `<pattern>` branch of the walker (round-data/R39).
//!
//! `patternContentUnits` decides what a body's numbers mean: user units of the
//! tile (the default `userSpaceOnUse`, which the composer scales about the tile
//! origin) or fractions of the **referencing element's box** (`objectBoundingBox`,
//! which already scales with the panel and must therefore stay verbatim). Scaling
//! those fractions by `(sx, sy)` again is a double scaling: at s=0.5 the hatch
//! shrank from ink 16 to ink 4 against an xor-0 reference.

use crate::dom;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

use super::transform_svg;

const SVG: &str = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p_obb" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" x="10" y="10" width="20" height="20">
<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>
</pattern>
<pattern id="p_user" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">
<rect x="1" y="2" width="2" height="3" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p_obb)"/>
</g>
</svg>"##;

fn run(target: Rect) -> String {
    let mut root = dom::parse(SVG).unwrap();
    let source = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
    };
    transform_svg(&mut root, source, target, 0, &mut Passthroughs::default()).unwrap();
    dom::serialize(&root).unwrap()
}

fn pattern<'a>(svg: &'a str, id: &str) -> &'a str {
    let start = svg.find(&format!("<pattern id=\"{id}\"")).unwrap();
    let end = start + svg[start..].find("</pattern>").unwrap();
    &svg[start..end]
}

fn body_rect(pattern_markup: &str) -> &str {
    let start = pattern_markup.find("<rect x=").unwrap();
    let end = start + pattern_markup[start..].find("/>").unwrap() + 2;
    &pattern_markup[start..end]
}

/// A unit-scale panel that merely moves: every body number is verbatim, and the
/// two content-unit modes are indistinguishable here — this is the degenerate
/// case, kept only to show it is *not* what the s=2 test relies on.
#[test]
fn object_bounding_box_pattern_body_is_verbatim_at_unit_scale() {
    let svg = run(Rect {
        x: 50.0,
        y: 40.0,
        width: 100.0,
        height: 80.0,
    });
    let obb = pattern(&svg, "p_obb");
    assert_eq!(
        body_rect(obb),
        r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##,
        "an objectBoundingBox body must not move or scale when s=1: {svg}"
    );
    // The tile box itself still moves into the cell.
    assert!(
        obb.contains(r#"x="60" y="50" width="20" height="20""#),
        "the tile box follows the panel even when the content is bbox relative: {svg}"
    );
}

/// The non-degenerate case: s=(2, 1.5). The `objectBoundingBox` body keeps every
/// fraction exactly as written (the bbox already scaled), while the default
/// `userSpaceOnUse` body scales about the tile origin without translating. If the
/// obb fractions were scaled, `0.1` would read `0.2`/`0.15` and `0.5` would read
/// `1`/`0.75` — the double scaling R39 fixed.
#[test]
fn object_bounding_box_pattern_body_is_verbatim_under_scale() {
    let svg = run(Rect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 120.0,
    });

    let obb = pattern(&svg, "p_obb");
    assert_eq!(
        body_rect(obb),
        r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##,
        "an objectBoundingBox body is bbox relative and must stay verbatim: {svg}"
    );
    // The tile box is user-space geometry and still maps into the cell.
    assert!(
        obb.contains(r#"x="30" y="35" width="40" height="30""#),
        "the tile box must still be rewritten: {svg}"
    );

    // Control: the default content units are user units and *do* scale (about the
    // tile origin, never translated).
    let user = pattern(&svg, "p_user");
    assert_eq!(
        body_rect(user),
        r##"<rect x="2" y="3" width="4" height="4.500" fill="#000000"/>"##,
        "a userSpaceOnUse body must still scale with the panel: {svg}"
    );
}

/// `stroke-width` on a `<pattern>` is inherited by its body, so under
/// `objectBoundingBox` content units it is a fraction too and must not be scaled;
/// the default mode keeps scaling it (geometric mean).
#[test]
fn object_bounding_box_pattern_does_not_scale_inherited_lengths() {
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>
<pattern id="p_obb" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" stroke-width="0.1" x="10" y="10" width="20" height="20">
<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>
</pattern>
</defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p_obb)"/>
</g>
</svg>"##;
    let mut root = dom::parse(svg).unwrap();
    let source = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
    };
    let target = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 160.0,
    };
    transform_svg(&mut root, source, target, 0, &mut Passthroughs::default()).unwrap();
    let out = dom::serialize(&root).unwrap();
    assert!(
        pattern(&out, "p_obb").contains(r#"stroke-width="0.1""#),
        "an objectBoundingBox pattern's inherited lengths stay verbatim: {out}"
    );
}
