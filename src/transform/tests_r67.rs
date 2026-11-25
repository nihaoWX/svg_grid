//! R67: an `objectBoundingBox` clipPath body is fractions of the referencing
//! element's box, so the composer must keep it verbatim — the same policy as an
//! obb pattern body. Before this round the body was still walked with the panel
//! policy: `width="1" height="0.5"` became `width="2" height="1"` and a
//! `<rect>`'s defaults were materialised as `x="0" y="0"`, i.e. a clip that
//! covers the whole element (measured: source ink 16000 vs product 32000; the
//! verbatim body is xor 0/0 — round-data/R67).
//!
//! The pixel oracles live in `docs/PITFALLS/round-data/R67/`; these tests pin
//! the serialized body so a regression cannot pass by scaling only one axis.

use super::transform_svg;
use crate::dom;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

const SRC: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 100.0,
    height: 80.0,
};

/// 2x: the probe size of every R67 case.
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

#[test]
fn an_object_bounding_box_clip_body_is_verbatim() {
    let out = run(
        r##"<defs><clipPath id="c" clipPathUnits="objectBoundingBox">
<rect width="1" height="0.5" fill="white"/></clipPath></defs>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" clip-path="url(#c)"/></g>"##,
    );
    assert!(
        out.contains(r#"<rect width="1" height="0.5" fill="white"/>"#),
        "{out}"
    );
    // The old pass scaled the fractions and materialised the rect defaults.
    assert!(!out.contains(r#"width="2""#), "{out}");
    assert!(!out.contains(r#"x="0" y="0" width="1""#), "{out}");
}

#[test]
fn a_user_space_clip_body_is_still_rewritten() {
    let out = run(
        r##"<defs><clipPath id="c"><rect x="10" y="10" width="20" height="20"/></clipPath></defs>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" clip-path="url(#c)"/></g>"##,
    );
    assert!(
        out.contains(r#"x="20" y="20" width="40" height="40""#),
        "{out}"
    );
}

#[test]
fn an_object_bounding_box_clip_body_keeps_its_font_size() {
    // R59: an obb body's `font-size` is a fraction of the box, so scaling it
    // would describe a glyph twice the box height.
    let out = run(
        r##"<defs><clipPath id="c" clipPathUnits="objectBoundingBox">
<text x="0.1" y="0.2" font-size="0.2">a</text></clipPath></defs>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" clip-path="url(#c)"/></g>"##,
    );
    assert!(out.contains(r#"font-size="0.2""#), "{out}");
    assert!(out.contains(r#"x="0.1""#), "{out}");
}

#[test]
fn a_pattern_referenced_from_an_object_bounding_box_clip_still_scales() {
    // The clip body is verbatim, but the pattern is a sibling definition and
    // keeps its user-space slot: its tile must still follow the panel (8→16),
    // or the pattern case would stop matching the source (round-data/R67).
    let out = run(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8">
<rect width="4" height="4" fill="white"/></pattern>
<clipPath id="c" clipPathUnits="objectBoundingBox">
<rect width="1" height="0.5" fill="url(#p)"/></clipPath></defs>
<g data-scale="xy"><rect x="0" y="0" width="100" height="80" clip-path="url(#c)"/></g>"##,
    );
    assert!(out.contains(r#"width="16" height="16""#), "{out}");
    assert!(out.contains(r#"width="1" height="0.5""#), "{out}");
}
