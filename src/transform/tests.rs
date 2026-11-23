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
    run_in(SVG, target)
}

fn run_in(svg: &str, target: Rect) -> String {
    let mut root = dom::parse(svg).unwrap();
    let source = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 80.0,
    };
    transform_svg(&mut root, source, target, 0, &mut Passthroughs::default()).unwrap();
    dom::serialize(&root).unwrap()
}

/// A panel that doubles on both axes, so every body number has to change when (and
/// only when) its units are user space.
const DOUBLED: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
};

fn doc(patterns: &str, use_markup: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs>{patterns}</defs>
{use_markup}
</g>
</svg>"##
    )
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

/// `patternContentUnits` is read with the SVG's own typed grammar: only the exact
/// token `objectBoundingBox` means bbox fractions. Every other spelling — a
/// different case, surrounding whitespace, an unknown token, an empty string, or a
/// missing attribute — falls back to the `userSpaceOnUse` default (usvg:
/// `convert_units` = `attribute(...).unwrap_or(default)`, no trimming, case
/// sensitive). Treating them as bbox left the body verbatim while the renderer
/// scaled it (round-data/R40, measured xor 448 at s=2).
#[test]
fn invalid_pattern_content_units_scales_like_user_space() {
    for units in [
        "ObjectBoundingBox",
        "objectBoundingBox ",
        " objectBoundingBox",
        "foo",
        "",
    ] {
        let svg = doc(
            &format!(
                r##"<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="{units}" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##
            ),
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
        );
        let out = run_in(&svg, DOUBLED);
        assert_eq!(
            body_rect(pattern(&out, "p")),
            r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
            "patternContentUnits={units:?} is not a bbox token and must scale: {out}"
        );
    }
}

/// A valid `viewBox` puts the body in viewBox units, which the viewBox→tile
/// mapping already applies; scaling the numbers again double-scales them
/// (round-data/R40, measured `2 3 6 4` -> `4 6 12 8`, xor 1792).
#[test]
fn view_box_pattern_body_stays_verbatim() {
    let svg = doc(
        r##"<pattern id="p" patternUnits="userSpaceOnUse" viewBox="0 0 20 20" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    let body = pattern(&out, "p");
    assert_eq!(
        body_rect(body),
        r##"<rect x="2" y="3" width="6" height="4" fill="#000000"/>"##,
        "a viewBox body is in viewBox units and must stay verbatim: {out}"
    );
    assert!(
        body.contains(r#"x="20" y="20" width="40" height="40""#),
        "the tile box itself is user space and still scales: {out}"
    );
}

/// A `viewBox` that does not parse is not a viewBox: usvg keeps the typed `None`
/// and the body stays in user units.
#[test]
fn invalid_view_box_falls_back_to_user_space() {
    for view_box in ["0 0 20 0", "0 0 20", "0 0 20 20e", "a b c d"] {
        let svg = doc(
            &format!(
                r##"<pattern id="p" patternUnits="userSpaceOnUse" viewBox="{view_box}" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##
            ),
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
        );
        let out = run_in(&svg, DOUBLED);
        assert_eq!(
            body_rect(pattern(&out, "p")),
            r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
            "viewBox={view_box:?} does not parse, so the body is user space: {out}"
        );
    }
}

/// usvg resolves `patternContentUnits` along `href`, first explicit value wins.
#[test]
fn pattern_content_units_is_inherited_through_href() {
    let svg = doc(
        r##"<pattern id="a" patternContentUnits="objectBoundingBox"/><pattern id="b" href="#a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    assert_eq!(
        body_rect(pattern(&out, "b")),
        r##"<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>"##,
        "an inherited objectBoundingBox must keep the body verbatim: {out}"
    );
}

/// … and `viewBox` too, through either spelling of the link.
#[test]
fn pattern_view_box_is_inherited_through_href() {
    for link in ["href", "xlink:href"] {
        let svg = doc(
            &format!(
                r##"<pattern id="a" viewBox="0 0 20 20"/><pattern id="b" {link}="#a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##
            ),
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
        );
        let out = run_in(&svg, DOUBLED);
        assert_eq!(
            body_rect(pattern(&out, "b")),
            r##"<rect x="2" y="3" width="6" height="4" fill="#000000"/>"##,
            "a viewBox inherited through {link} keeps the body verbatim: {out}"
        );
    }
}

/// An `href` cycle must terminate with what has been collected so far, not loop.
#[test]
fn pattern_href_cycle_stops_instead_of_panicking() {
    let svg = doc(
        r##"<pattern id="a" href="#b"/><pattern id="b" href="#a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    // Neither pattern declares content units or a viewBox, so the body is user
    // space: scaled, and the walk simply stops at the repeated id.
    assert_eq!(
        body_rect(pattern(&out, "b")),
        r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
        "a cycle resolves to user space and must not hang: {out}"
    );
}

/// A link the composer cannot resolve ends inheritance, so the defaults apply:
/// content units fall back to `userSpaceOnUse` and the body **does** scale
/// (round-data/R41; usvg's `HrefIter` simply stops, it does not keep bbox units).
#[test]
fn unresolvable_pattern_href_falls_back_to_user_space() {
    for link in ["#missing", "other.svg#p", "", "http://example.com/#p"] {
        let svg = doc(
            &format!(
                r##"<pattern id="b" href="{link}" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##
            ),
            r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
        );
        let out = run_in(&svg, DOUBLED);
        assert_eq!(
            body_rect(pattern(&out, "b")),
            r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
            "href={link:?} is unresolvable, so the body stays user space: {out}"
        );
    }
}

/// usvg builds its id map with `links.insert(...)` in document order, so a
/// duplicate id resolves to the **last** definition (round-data/R41).
#[test]
fn duplicate_pattern_ids_resolve_to_the_last_definition() {
    let svg = doc(
        r##"<pattern id="a" patternContentUnits="objectBoundingBox"/><pattern id="a" patternContentUnits="userSpaceOnUse"/><pattern id="b" href="#a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    assert_eq!(
        body_rect(pattern(&out, "b")),
        r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
        "the last duplicate declares user units, so the body scales: {out}"
    );
}

/// `patternUnits` inherits along `href` as well; without the tile's own attribute
/// usvg reads the linked value, and its default is `objectBoundingBox` (round-data/R41).
#[test]
fn pattern_units_is_inherited_through_href() {
    let svg = doc(
        r##"<pattern id="base" patternUnits="userSpaceOnUse"/><pattern id="p" href="#base" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    let tile = pattern(&out, "p");
    for expected in [r#"x="20""#, r#"y="20""#, r#"width="40""#, r#"height="40""#] {
        assert!(
            tile.contains(expected),
            "an inherited userSpaceOnUse tile follows the panel, missing {expected}: {out}"
        );
    }
    assert_eq!(
        body_rect(tile),
        r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
        "the body is plain user space: {out}"
    );

    let svg = doc(
        r##"<pattern id="base" patternUnits="objectBoundingBox"/><pattern id="p" href="#base" x="0.1" y="0.2" width="0.5" height="0.5"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#p)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    let tile = pattern(&out, "p");
    assert!(
        tile.contains(r#"x="0.1""#) && tile.contains(r#"width="0.5""#),
        "an inherited objectBoundingBox tile keeps its fractions: {out}"
    );
}

/// A link that lands on something which is not a `<pattern>` ends the chain, like
/// usvg's `href_iter` — the body is then plain user space.
#[test]
fn pattern_href_to_non_pattern_ends_the_chain() {
    let svg = doc(
        r##"<rect id="notpattern" x="0" y="0" width="1" height="1"/><pattern id="b" href="#notpattern" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"><rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern>"##,
        r##"<rect x="10" y="10" width="40" height="30" fill="url(#b)"/>"##,
    );
    let out = run_in(&svg, DOUBLED);
    assert_eq!(
        body_rect(pattern(&out, "b")),
        r##"<rect x="4" y="6" width="12" height="8" fill="#000000"/>"##,
        "a non-pattern target ends the chain and leaves user units: {out}"
    );
}
