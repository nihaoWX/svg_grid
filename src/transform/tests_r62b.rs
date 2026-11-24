//! R62, second half: `<ellipse>` (never rewritten before this round) and the
//! contracts that pin what the composer must **not** do (independent axes).
//!
//! The content-domain tests live in the sibling `tests_r62`; the pixel oracles for
//! both halves are in `docs/PITFALLS/round-data/R62/`.

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

/// 2×: the probe size of the D-axis contracts.
const TILE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
};

/// 8×: the probe size of the `<ellipse>` cases.
const BIG: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 800.0,
    height: 640.0,
};

/// Anisotropic: `sx = 2`, `sy = 3`. With `sx == sy` a mutant that swaps the two
/// axes is invisible (the R62 review's `ry` mutant survived every `BIG` case).
const ANISO: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 240.0,
};

fn doc(policy: &str, body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="{policy}">{body}</g></svg>"##
    )
}

fn run_in(svg: &str, target: Rect) -> String {
    run_with_source(svg, SRC, target).0
}

fn run_with_source(svg: &str, source: Rect, target: Rect) -> (String, Passthroughs) {
    let mut root = dom::parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, source, target, 0, &mut passthroughs).unwrap();
    (dom::serialize(&root).unwrap(), passthroughs)
}

fn find_by_id<'a>(element: &'a Element, id: &str) -> Option<&'a Element> {
    if element.attr("id") == Some(id) {
        return Some(element);
    }
    element.children.iter().find_map(|child| match child {
        Node::Element(child) => find_by_id(child, id),
        Node::Text(_) => None,
    })
}

/// The **last** element with this name (the panel box comes first in `doc`).
fn find_last<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    let mut found = (element.name == name).then_some(element);
    for child in &element.children {
        if let Node::Element(child) = child {
            if let Some(hit) = find_last(child, name) {
                found = Some(hit);
            }
        }
    }
    found
}

fn attr_by_name(svg: &str, name: &str, attr_name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find_last(&root, name)?.attr(attr_name).map(str::to_string)
}

fn attr_of(svg: &str, id: &str, attr_name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find_by_id(&root, id)?.attr(attr_name).map(str::to_string)
}

#[test]
fn an_ellipse_scales_its_centre_and_both_radii() {
    let svg = doc(
        "xy",
        r#"<ellipse cx="20" cy="20" rx="10" ry="5" fill="red"/>"#,
    );
    let out = run_in(&svg, BIG);
    assert_eq!(attr_by_name(&out, "ellipse", "cx").as_deref(), Some("160"));
    assert_eq!(attr_by_name(&out, "ellipse", "cy").as_deref(), Some("160"));
    assert_eq!(attr_by_name(&out, "ellipse", "rx").as_deref(), Some("80"));
    assert_eq!(attr_by_name(&out, "ellipse", "ry").as_deref(), Some("40"));
}

#[test]
fn an_ellipse_scales_each_radius_with_its_own_axis() {
    let svg = doc(
        "xy",
        r#"<ellipse cx="20" cy="20" rx="10" ry="5" fill="red"/>"#,
    );
    let out = run_in(&svg, ANISO);
    assert_eq!(attr_by_name(&out, "ellipse", "cx").as_deref(), Some("40"));
    assert_eq!(attr_by_name(&out, "ellipse", "cy").as_deref(), Some("60"));
    assert_eq!(
        attr_by_name(&out, "ellipse", "rx").as_deref(),
        Some("20"),
        "rx follows x (sx = 2): {out}"
    );
    assert_eq!(
        attr_by_name(&out, "ellipse", "ry").as_deref(),
        Some("15"),
        "ry follows y (sy = 3): {out}"
    );

    // Only `rx` is spelled: the renderer's implicit ry is the *source* rx and
    // must be materialised on the y axis (10 * 3), not the x axis.
    let svg = doc("xy", r#"<ellipse cx="20" cy="20" rx="10" fill="red"/>"#);
    let out = run_in(&svg, ANISO);
    assert_eq!(attr_by_name(&out, "ellipse", "rx").as_deref(), Some("20"));
    assert_eq!(
        attr_by_name(&out, "ellipse", "ry").as_deref(),
        Some("30"),
        "the implied ry is filled by its own axis (10 * 3): {out}"
    );

    // Only `ry` is spelled: the implicit rx is 10 * 2, not 10 * 3.
    let svg = doc("xy", r#"<ellipse cx="20" cy="20" ry="10" fill="red"/>"#);
    let out = run_in(&svg, ANISO);
    assert_eq!(
        attr_by_name(&out, "ellipse", "rx").as_deref(),
        Some("20"),
        "the implied rx is filled by its own axis (10 * 2): {out}"
    );
    assert_eq!(attr_by_name(&out, "ellipse", "ry").as_deref(), Some("30"));
}

#[test]
fn an_ellipse_with_one_radius_materialises_the_equal_one() {
    let svg = doc("xy", r#"<ellipse cx="20" cy="20" rx="10" fill="red"/>"#);
    let out = run_in(&svg, BIG);
    assert_eq!(attr_by_name(&out, "ellipse", "rx").as_deref(), Some("80"));
    assert_eq!(
        attr_by_name(&out, "ellipse", "ry").as_deref(),
        Some("80"),
        "the implicit ry is the old rx and must scale with y: {out}"
    );

    let svg = doc("xy", r#"<ellipse cx="20" cy="20" ry="10" fill="red"/>"#);
    let out = run_in(&svg, BIG);
    assert_eq!(attr_by_name(&out, "ellipse", "rx").as_deref(), Some("80"));
    assert_eq!(attr_by_name(&out, "ellipse", "ry").as_deref(), Some("80"));
}

#[test]
fn an_ellipse_with_one_radius_under_a_single_axis_policy_keeps_the_other() {
    let svg = doc("x", r#"<ellipse cx="20" cy="20" rx="10" fill="red"/>"#);
    let out = run_in(&svg, BIG);
    assert_eq!(attr_by_name(&out, "ellipse", "cx").as_deref(), Some("160"));
    assert_eq!(attr_by_name(&out, "ellipse", "cy").as_deref(), Some("20"));
    assert_eq!(attr_by_name(&out, "ellipse", "rx").as_deref(), Some("80"));
    assert_eq!(
        attr_by_name(&out, "ellipse", "ry").as_deref(),
        Some("10"),
        "y is not scaled, so the equal radius keeps the source value: {out}"
    );
}

#[test]
fn an_ellipse_with_one_radius_under_none_keeps_its_bytes() {
    let svg = doc("none", r#"<ellipse cx="20" cy="20" rx="10" fill="red"/>"#);
    let out = run_in(&svg, BIG);
    assert!(attr_by_name(&out, "ellipse", "ry").is_none(), "{out}");
}

#[test]
fn an_ellipse_with_only_a_style_radius_keeps_the_pair_verbatim() {
    // A radius in inline `style` wins over the attribute, so materialising the
    // missing one as an attribute would invent geometry: d1render draws neither
    // spelling, and the case is registered (not fixed) in round-data/R62.
    let svg = doc(
        "xy",
        r#"<ellipse cx="20" cy="20" style="rx:10" fill="red"/>"#,
    );
    let out = run_in(&svg, BIG);
    assert_eq!(attr_by_name(&out, "ellipse", "cx").as_deref(), Some("160"));
    let style = attr_by_name(&out, "ellipse", "style").unwrap_or_default();
    assert!(style.contains("rx:10"), "{out}");
    assert!(attr_by_name(&out, "ellipse", "ry").is_none(), "{out}");
}

#[test]
fn an_ellipse_with_a_malformed_radius_fails_closed() {
    let svg = doc("xy", r#"<ellipse cx="20" cy="20" rx="abc" fill="red"/>"#);
    let mut root = dom::parse(&svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    let result = transform_svg(&mut root, SRC, BIG, 0, &mut passthroughs);
    assert!(result.is_err(), "garbage geometry must not be composed");
}

/// D: a gradient that spells only `x2` gets no synthesised coordinates.
#[test]
fn a_gradient_with_one_coordinate_gets_no_pair_filled() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" fill="url(#g)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x2="16"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "g", "x2").as_deref(), Some("32"), "{out}");
    for attr_name in ["x1", "y1", "y2"] {
        assert!(
            attr_of(&out, "g", attr_name).is_none(),
            "must not invent {attr_name}: {out}"
        );
    }
}

/// D: a pattern tile's `x`/`y` are independent of its `width`/`height`.
#[test]
fn a_pattern_tile_scales_only_what_it_spells() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" fill="url(#p)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" x="2" width="16"><rect width="4" height="4" fill="red"/></pattern></defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "p", "x").as_deref(), Some("4"), "{out}");
    assert_eq!(
        attr_of(&out, "p", "y").as_deref(),
        Some("0"),
        "x and y are one point: {out}"
    );
    assert_eq!(attr_of(&out, "p", "width").as_deref(), Some("32"), "{out}");
    assert!(
        attr_of(&out, "p", "height").is_none(),
        "width and height stay independent: {out}"
    );
}

/// D: a filter/mask region scales only the sides it spells.
#[test]
fn a_filter_region_scales_only_what_it_spells() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" filter="url(#f)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="10" width="20"><feGaussianBlur stdDeviation="2"/></filter></defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "f", "x").as_deref(), Some("20"), "{out}");
    assert_eq!(attr_of(&out, "f", "width").as_deref(), Some("40"), "{out}");
    assert!(attr_of(&out, "f", "y").is_none(), "{out}");
    assert!(attr_of(&out, "f", "height").is_none(), "{out}");
}
