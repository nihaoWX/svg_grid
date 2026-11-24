//! Unit tests for R64: `<use href>`/`xlink:href` targets that are **not** one of
//! the seven definition tags.
//!
//! A `<rect id="r">` or `<g id="g">` in `<defs>` is never drawn where it sits
//! either: `<use>` instantiates it in the referrer's user space, so its numbers
//! have to follow the panel exactly like a `<pattern>`'s. Before this round only
//! [`DEFINITION_TAGS`](super::defs::DEFINITION_TAGS) could carry a referrer policy,
//! so the reference graph and the per-policy copies existed (R46) but the walk never
//! applied them to a plain element.
//!
//! The pixel oracles live in `docs/PITFALLS/round-data/R64/` (u2/u3/u5/u17 at
//! `xor 0`, `ink` equal). The two **controls** — a `<symbol>` with a `viewBox`
//! instantiated by `<use>`, and a `<use>` inside a `viewBox` pattern body — must
//! stay byte-identical, so each is asserted as its **exact serialized document**
//! (a golden string): spot-checking a few attributes cannot catch a walk that
//! stops materialising the defaults (`x="0" y="0"`) the old pass wrote. u16/u4 are
//! attribute assertions because a single-axis `data-scale` has no uniform
//! `d1render` oracle (the source cannot be rendered at `(2, 1)`); u7 is the
//! registered residual (target in the drawable tree), also pinned.

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

/// 2x: the probe size of every R64 case.
const TILE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
};

fn run(body: &str) -> String {
    let svg = format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
{body}</svg>"##
    );
    let mut root = dom::parse(&svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, SRC, TILE, 0, &mut passthroughs).unwrap();
    dom::serialize(&root).unwrap()
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

fn attr_of(svg: &str, id: &str, name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find_by_id(&root, id)?.attr(name).map(str::to_string)
}

fn assert_rect(svg: &str, id: &str, width: &str, height: &str) {
    assert_eq!(attr_of(svg, id, "width").as_deref(), Some(width), "{svg}");
    assert_eq!(attr_of(svg, id, "height").as_deref(), Some(height), "{svg}");
}

/// u2: a bare `<rect>` in `<defs>` is instantiated in the `<use>`'s user space, so
/// its size doubles with the panel. Base: rect stays `10x10` (xor 300 / maxdiff 255,
/// ink 100 against 400); fixed: `20x20` (xor 0, ink 400).
#[test]
fn a_use_referenced_rect_in_defs_follows_the_referrer() {
    let out = run(
        r##"<defs><rect id="r" width="10" height="10" fill="#000"/></defs>
<g data-scale="xy"><use href="#r" x="10" y="10"/></g>"##,
    );
    assert_rect(&out, "r", "20", "20");
    assert_eq!(attr_of(&out, "r", "fill").as_deref(), Some("#000"));
}

/// u3: a `<g>` target scales its whole subtree, not just its own (absent) numbers.
/// Base: rect `10x10` / circle `15,5 r=3` (xor 408, ink 133 against 524); fixed:
/// `20x20` and `30,10 r=6` (xor 0, ink 524).
#[test]
fn a_use_referenced_group_scales_its_children() {
    let out = run(
        r##"<defs><g id="g"><rect width="10" height="10" fill="#000"/>
<circle cx="15" cy="5" r="3" fill="#000"/></g></defs>
<g data-scale="xy"><use href="#g" x="10" y="10"/></g>"##,
    );
    let root = dom::parse(&out).unwrap();
    let Group { rect, circle } = group_children(&root, "g");
    assert_eq!(rect, ("20".into(), "20".into()));
    assert_eq!(circle, ("30".into(), "10".into(), "6".into()));
}

/// u5: the `<use>` **inside** the referenced definition has its own `x`/`y` in the
/// same user space, so they scale too. Base: leaf `10x10`, inner use `2,2` (xor 372,
/// ink 100 against 400); fixed: `20x20` and `4,4` (xor 0, ink 400).
#[test]
fn a_use_inside_a_referenced_definition_scales_its_offsets() {
    let out = run(r##"<defs><rect id="r" width="10" height="10" fill="#000"/>
<g id="g"><use href="#r" x="2" y="2"/></g></defs>
<g data-scale="xy"><use href="#g" x="10" y="10"/></g>"##);
    assert_rect(&out, "r", "20", "20");
    let root = dom::parse(&out).unwrap();
    let inner = find_by_id(&root, "r")
        .and_then(|_| first_use(&root))
        .expect("the inner <use> must survive");
    assert_eq!(attr(inner, "x").as_deref(), Some("4"));
    assert_eq!(attr(inner, "y").as_deref(), Some("4"));
}

/// u17: two `<use>`s of one definition share one copy, and that copy scales once.
/// Base: `10x10` (xor 600, ink 200 against 800); fixed: `20x20` (xor 0, ink 800).
#[test]
fn two_uses_of_one_definition_share_the_scaled_copy() {
    let out = run(
        r##"<defs><rect id="r" width="10" height="10" fill="#000"/></defs>
<g data-scale="xy"><use href="#r" x="10" y="10"/><use href="#r" x="40" y="10"/></g>"##,
    );
    assert_rect(&out, "r", "20", "20");
}

/// u16: `data-scale="x"` scales only the width, and the `<use>` offset only in `x`.
/// No uniform pixel oracle exists for a `(2, 1)` placement, so this is asserted.
#[test]
fn a_single_axis_policy_scales_only_that_axis() {
    let out = run(
        r##"<defs><rect id="r" width="10" height="10" fill="#000"/></defs>
<g data-scale="x"><use href="#r" x="10" y="10"/></g>"##,
    );
    assert_rect(&out, "r", "20", "10");
    let root = dom::parse(&out).unwrap();
    let used = first_use(&root).expect("the <use> must survive");
    assert_eq!(attr(used, "x").as_deref(), Some("20"));
    assert_eq!(attr(used, "y").as_deref(), Some("10"));
}

/// u4: a definition shared by two policies is copied per policy (R46) and **each
/// copy** takes its own policy — `xy` gives `20x20`, `y` gives `10x20`. Base: both
/// copies stayed `10x10` (xor 800, ink 200 against 800 for the uniform-mixed
/// composition); after R64 the references are repointed at the copies that scale.
#[test]
fn each_per_policy_copy_of_a_use_target_scales_its_own_way() {
    let out = run(
        r##"<defs><rect id="r" width="10" height="10" fill="#000"/></defs>
<g data-scale="xy"><use href="#r" x="10" y="10"/></g>
<g data-scale="y"><use href="#r" x="10" y="40"/></g>"##,
    );
    assert_rect(&out, "r", "20", "20");
    assert_rect(&out, "r--svg-grid-y", "10", "20");
    assert!(out.contains(r##"<use href="#r--svg-grid-y""##), "{out}");
}

/// u6 control: a `<symbol viewBox="0 0 10 10">` is already mapped onto the
/// instance's `width`/`height` by the renderer, so its body must stay verbatim
/// (xor 0 / ink 1600 before and after). Only the `<use>` box scales.
///
/// Asserted as the **whole serialized document**: the body keeps
/// `width="10" height="10"` (and the `x="0" y="0"` the walk materialises), while
/// the instance box becomes `20/20/40/40`. R64b: an attribute-only check let the
/// "skip the byte-preserving pass" mutation (removing the `is_definition` guard in
/// `walk.rs`) pass `cargo test`; the golden string makes it red.
#[test]
fn a_symbol_with_a_viewbox_keeps_its_body_verbatim() {
    let out = run(
        r##"<defs><symbol id="s" viewBox="0 0 10 10"><rect width="10" height="10" fill="#000"/></symbol></defs>
<g data-scale="xy"><use href="#s" x="10" y="10" width="20" height="20"/></g>"##,
    );
    let expected = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<defs><symbol id="s" viewBox="0 0 10 10"><rect width="10" height="10" fill="#000" x="0" y="0"/></symbol></defs>
<g data-scale="xy"><use href="#s" x="20" y="20" width="40" height="40"/></g></svg>"##;
    assert_eq!(out, expected);
}

/// u15 control: a `<use>` inside a pattern body that has a `viewBox` lives in
/// viewBox units, so neither the `<use>` nor its target may be rewritten
/// (xor 0 before and after); only the tile box scales.
///
/// Asserted as the **whole serialized document**: `<rect id="r">` keeps its
/// numbers *and* the `x="0" y="0"` the walk materialises, the inner `<use>` keeps
/// `x="0" y="0"`, and only the tile becomes `20×20`. This is the case R64b pins:
/// dropping the `is_definition` guard makes the early return swallow that
/// materialisation and turns the golden comparison red.
#[test]
fn a_use_inside_a_viewbox_pattern_body_stays_verbatim() {
    let out = run(r##"<defs><rect id="r" width="10" height="10" fill="#000"/>
<pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10" viewBox="0 0 10 10">
<use href="#r" x="0" y="0"/></pattern></defs>
<g data-scale="xy"><rect width="100" height="80" fill="url(#p)"/></g>"##);
    let expected = r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<defs><rect id="r" width="10" height="10" fill="#000" x="0" y="0"/>
<pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20" viewBox="0 0 10 10">
<use href="#r" x="0" y="0"/></pattern></defs>
<g data-scale="xy"><rect width="200" height="160" fill="url(#p)" x="0" y="0"/></g></svg>"##;
    assert_eq!(out, expected);
}

/// u7, **registered residual** (not fixed by R64): a `<use>` whose target is an
/// ordinary `<g>` in the drawable tree is not scaled. The element is drawn where it
/// sits as well, so giving it a referrer policy would scale that first drawing too
/// (double), and copying it per policy would draw the body twice. Measured: xor 600,
/// ink 200 against 800. The workaround is to put the target in `<defs>`.
#[test]
fn a_use_target_in_the_drawable_tree_keeps_its_own_policy() {
    let out = run(r##"<g id="g"><rect width="10" height="10" fill="#000"/></g>
<g data-scale="xy"><use href="#g" x="10" y="10"/></g>"##);
    let root = dom::parse(&out).unwrap();
    let group = find_by_id(&root, "g").expect("the group must survive");
    let rect = group
        .children
        .iter()
        .find_map(|node| match node {
            Node::Element(element) => Some(element),
            Node::Text(_) => None,
        })
        .expect("the body must survive");
    assert_eq!(attr(rect, "width").as_deref(), Some("10"));
}

struct Group {
    rect: (String, String),
    circle: (String, String, String),
}

fn group_children(root: &Element, id: &str) -> Group {
    let group = find_by_id(root, id).expect("the group must survive");
    let mut rect = None;
    let mut circle = None;
    for child in &group.children {
        let Node::Element(child) = child else {
            continue;
        };
        match child.name.as_str() {
            "rect" => {
                rect = Some((
                    attr(child, "width").unwrap(),
                    attr(child, "height").unwrap(),
                ))
            }
            "circle" => {
                circle = Some((
                    attr(child, "cx").unwrap(),
                    attr(child, "cy").unwrap(),
                    attr(child, "r").unwrap(),
                ))
            }
            _ => {}
        }
    }
    Group {
        rect: rect.expect("a rect child"),
        circle: circle.expect("a circle child"),
    }
}

fn first_use(root: &Element) -> Option<&Element> {
    root.children.iter().find_map(|node| match node {
        Node::Element(element) if element.name == "use" => Some(element),
        Node::Element(element) => first_use(element),
        Node::Text(_) => None,
    })
}

fn attr(element: &Element, name: &str) -> Option<String> {
    element.attr(name).map(str::to_string)
}
