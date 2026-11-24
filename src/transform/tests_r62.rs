//! Unit tests for R62: the **content domain** axis of a definition, and the
//! `<ellipse>` primitive that was never rewritten at all.
//!
//! A definition referenced only from inside a body the renderer already maps (a
//! pattern with a `viewBox` or `patternContentUnits="objectBoundingBox"`, a marker
//! with a `viewBox`) is instantiated in that body's units: its tile, coordinates
//! and body stay verbatim, and a definition those bodies share with panel geometry
//! is copied per domain. The pixel oracles live in `docs/PITFALLS/round-data/R62/`;
//! the ported assertions about `referrers`/`split` are R46's, expressed in
//! [`super::domains::Slot`]. The `<ellipse>` half (and the D-axis contracts) live
//! in the sibling `tests_r62b`.

use std::collections::BTreeMap;

use super::defs::{referrers, split};
use super::domains::Slot;
use super::transform_svg;
use super::ScalePolicy;
use crate::dom::{self, Element, Node};
use crate::geom::Rect;
use crate::warnings::Passthroughs;

const SRC: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 100.0,
    height: 80.0,
};

/// 2×: the probe size of the content-domain cases.
const TILE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
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

fn attr_of(svg: &str, id: &str, attr_name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find_by_id(&root, id)?.attr(attr_name).map(str::to_string)
}

fn nested_attr(svg: &str, id: &str, child: &str, attr_name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    let parent = find_by_id(&root, id)?;
    parent.children.iter().find_map(|node| match node {
        Node::Element(element) if element.name == child => {
            element.attr(attr_name).map(str::to_string)
        }
        _ => None,
    })
}

fn refs_of(svg: &str) -> BTreeMap<String, Vec<Slot>> {
    let root = dom::parse(svg).unwrap();
    referrers(&root)
        .into_iter()
        .map(|(id, set)| (id, set.into_iter().collect()))
        .collect()
}

fn split_text(svg: &str) -> String {
    let mut root = dom::parse(svg).unwrap();
    let _ = split(&mut root);
    dom::serialize(&root).unwrap()
}

fn slot(policy: ScalePolicy) -> Slot {
    Slot::UserSpace(policy)
}

/// A container's `data-scale` must reach its children: the copy a child points
/// at is chosen by the slot the child inherits, not by the document root. This
/// is the R46/R49 regression R62's first cut introduced and the reason those two
/// rounds' evidence is part of the acceptance run.
#[test]
fn a_group_data_scale_reaches_its_children() {
    let svg = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<pattern id="h" patternUnits="userSpaceOnUse" x="0" y="0" width="6" height="6"><rect width="6" height="6" fill="#000"/></pattern>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#h)"/></g>
<g data-scale="x"><rect x="10" y="50" width="40" height="30" fill="url(#h)"/></g></svg>"##;
    let mut root = dom::parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, SRC, TILE, 0, &mut passthroughs).unwrap();
    let out = dom::serialize(&root).unwrap();
    assert!(out.contains(r#"fill="url(#h)""#), "{out}");
    assert!(out.contains(r#"fill="url(#h--svg-grid-x)""#), "{out}");
}

#[test]
fn a_referrers_policy_is_the_policy_of_the_referring_element() {
    let refs = refs_of(
        &doc("x", r#"<rect width="10" height="10" fill="url(#p)"/>"#).replace(
            "</g>",
            "</g><defs><pattern id=\"p\" width=\"2\" height=\"2\"/></defs>",
        ),
    );
    assert_eq!(refs["p"], vec![slot(ScalePolicy::X)]);
}

#[test]
fn a_definition_inherits_the_policies_of_the_definitions_that_reference_it() {
    let refs = refs_of(
        r##"<svg xmlns="http://www.w3.org/2000/svg">
              <defs>
                <pattern id="p" width="2" height="2">
                  <linearGradient id="d" x1="0" x2="100"/>
                  <rect fill="url(#d)"/>
                </pattern>
              </defs>
              <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
              <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
            </svg>"##,
    );
    assert_eq!(
        refs["p"],
        vec![slot(ScalePolicy::Xy), slot(ScalePolicy::X)],
        "the pattern is evaluated under both referrer policies"
    );
    assert_eq!(
        refs["d"], refs["p"],
        "so is the gradient its body fills with"
    );
}

#[test]
fn an_unreferenced_definition_has_no_slot_at_all() {
    let refs = refs_of(
        r##"<svg xmlns="http://www.w3.org/2000/svg">
              <defs><linearGradient id="u" x1="0" x2="100"/></defs>
              <rect width="10" height="10" fill="url(#u)"/>
            </svg>"##,
    );
    assert!(refs.contains_key("u"));
    let refs = refs_of(
        r##"<svg xmlns="http://www.w3.org/2000/svg">
              <defs><linearGradient id="u" x1="0" x2="100"/></defs>
            </svg>"##,
    );
    assert!(!refs.contains_key("u"));
}

#[test]
fn a_single_slot_definition_is_never_copied() {
    let text = split_text(
        r##"<svg xmlns="http://www.w3.org/2000/svg">
              <defs><linearGradient id="g" x1="0" x2="10"/></defs>
              <g data-scale="xy"><rect width="10" height="10" fill="url(#g)"/></g>
            </svg>"##,
    );
    assert!(!text.contains("--svg-grid"), "{text}");
    assert!(text.contains(r#"id="g""#), "{text}");
}

#[test]
fn copies_follow_the_reference_graph_and_get_unique_ids() {
    let text = split_text(
        r##"<svg xmlns="http://www.w3.org/2000/svg">
              <defs>
                <pattern id="p" width="2" height="2"><rect fill="url(#g)"/></pattern>
                <linearGradient id="g" x1="0" x2="10"/>
              </defs>
              <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
              <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
            </svg>"##,
    );
    for id in ["p", "p--svg-grid-x", "g", "g--svg-grid-x"] {
        assert!(
            text.contains(&format!(r#"id="{id}""#)),
            "missing {id}: {text}"
        );
    }
    assert!(
        text.contains(r#"fill="url(#g--svg-grid-x)""#),
        "the x copy must point at its own gradient: {text}"
    );
}

/// A pattern body that is already mapped by the renderer does not hand the panel
/// policy on: the inner definition stays verbatim (dispatch case v1, xor 0).
#[test]
fn a_pattern_inside_a_view_box_body_stays_verbatim() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" fill="url(#outer)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<pattern id="inner" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="4" height="4" fill="red"/></pattern>
<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16" viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#inner)"/></pattern>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "inner", "width").as_deref(), Some("8"));
    assert_eq!(attr_of(&out, "inner", "height").as_deref(), Some("8"));
    assert_eq!(
        nested_attr(&out, "inner", "rect", "width").as_deref(),
        Some("4"),
        "the inner body must not scale either: {out}"
    );
    assert_eq!(attr_of(&out, "outer", "width").as_deref(), Some("32"));
    assert_eq!(
        nested_attr(&out, "outer", "rect", "width").as_deref(),
        Some("16"),
        "the mapped body itself stays verbatim: {out}"
    );
}

/// The same rule for an `objectBoundingBox` body and its user-space gradient
/// (dispatch case v3, xor 0).
#[test]
fn a_gradient_inside_an_object_bounding_box_body_stays_verbatim() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" fill="url(#outer)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0.5" y2="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
<pattern id="outer" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" width="0.2" height="0.2"><rect width="1" height="1" fill="url(#g)"/></pattern>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "g", "x2").as_deref(), Some("0.5"), "{out}");
    assert_eq!(
        attr_of(&out, "outer", "width").as_deref(),
        Some("0.2"),
        "an obb tile is a fraction of the box, so it stays: {out}"
    );
}

/// A marker with a `viewBox` maps its body the same way (dispatch case m1).
#[test]
fn a_marker_view_box_body_keeps_its_gradient() {
    let svg = doc(
        "xy",
        r##"<path d="M 0 40 L 100 40" stroke="black" marker-end="url(#m)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
<marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" markerHeight="10" refX="0" refY="0" viewBox="0 0 10 10"><rect width="10" height="10" fill="url(#g)"/></marker>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "g", "x2").as_deref(), Some("10"), "{out}");
    assert_eq!(
        attr_of(&out, "m", "markerWidth").as_deref(),
        Some("20"),
        "the marker viewport still follows the panel: {out}"
    );
    assert_eq!(
        nested_attr(&out, "m", "rect", "width").as_deref(),
        Some("10"),
        "the marker body is already expanded by the renderer: {out}"
    );
}

/// A marker body that fills with a user-space pattern keeps the pattern verbatim
/// (dispatch case n3, xor 0).
#[test]
fn a_pattern_inside_a_marker_view_box_stays_verbatim() {
    let svg = doc(
        "xy",
        r##"<path d="M 0 40 L 100 40" stroke="black" marker-end="url(#m)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="2" height="2" fill="red"/></pattern>
<marker id="m" markerUnits="userSpaceOnUse" markerWidth="20" markerHeight="20" refX="0" refY="0" viewBox="0 0 20 20"><rect width="20" height="20" fill="url(#p)"/></marker>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "p", "width").as_deref(), Some("4"), "{out}");
    assert_eq!(
        nested_attr(&out, "p", "rect", "width").as_deref(),
        Some("2"),
        "{out}"
    );
    assert_eq!(attr_of(&out, "m", "markerWidth").as_deref(), Some("40"));
}

/// One gradient referenced by panel geometry and by a mapped body is two
/// definitions: the body's copy is verbatim, the panel's copy scales (case v4).
#[test]
fn a_shared_gradient_is_copied_per_domain() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="40" fill="url(#g)"/><rect y="40" width="100" height="40" fill="url(#outer)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16" viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#g)"/></pattern>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(
        attr_of(&out, "g", "x2").as_deref(),
        Some("20"),
        "the panel copy scales: {out}"
    );
    let copies: Vec<&str> = out
        .match_indices(r#"<linearGradient "#)
        .map(|(position, _)| &out[position..])
        .collect();
    assert_eq!(copies.len(), 2, "one copy per domain: {out}");
    assert!(
        copies
            .iter()
            .any(|copy| copy.starts_with(r#"<linearGradient id="g--svg-grid-verbatim""#)),
        "the verbatim copy keeps the body's numbers: {out}"
    );
    assert!(
        out.contains(r#"fill="url(#g--svg-grid-verbatim)""#),
        "the mapped body must point at it: {out}"
    );
}

/// Three domains — panel, viewBox body, obb body — are three copies, two of them
/// verbatim and therefore identical in value (case m3).
#[test]
fn three_domains_are_two_copies_because_both_verbatim_slots_agree() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="26" fill="url(#g)"/><rect y="26" width="100" height="27" fill="url(#pv)"/><rect y="53" width="100" height="27" fill="url(#pb)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
<pattern id="pv" patternUnits="userSpaceOnUse" width="16" height="16" viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#g)"/></pattern>
<pattern id="pb" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" width="0.2" height="0.2"><rect width="1" height="1" fill="url(#g)"/></pattern>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(attr_of(&out, "g", "x2").as_deref(), Some("20"), "{out}");
    assert_eq!(out.matches("<linearGradient").count(), 2, "{out}");
    // Both mapped bodies share the verbatim copy: one clone, not two.
    assert_eq!(out.matches("#g--svg-grid-verbatim").count(), 2, "{out}");
}

/// A nested pattern whose bodies are *not* mapped keeps the old behaviour: the
/// inner tile and body scale with the panel (control case c1, byte-identical).
#[test]
fn a_user_space_nested_pattern_still_scales() {
    let svg = doc(
        "xy",
        r##"<rect width="100" height="80" fill="url(#outer)"/>"##,
    )
    .replace(
        "</svg>",
        r##"<defs>
<pattern id="inner" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="4" height="4" fill="red"/></pattern>
<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16"><rect width="16" height="16" fill="url(#inner)"/></pattern>
</defs></svg>"##,
    );
    let out = run_in(&svg, TILE);
    assert_eq!(
        attr_of(&out, "inner", "width").as_deref(),
        Some("16"),
        "{out}"
    );
    assert_eq!(
        nested_attr(&out, "inner", "rect", "width").as_deref(),
        Some("8"),
        "{out}"
    );
    assert!(
        !out.contains("--svg-grid"),
        "one domain means no copies: {out}"
    );
}
