//! R66b: which layer of a text run or a pattern tile actually paints.
//!
//! Every rule below was measured before it was written. The ink oracle is
//! d1render (resvg 0.47) on a 300x80 canvas with the painted content at x=200;
//! cairosvg 2.9.1 is the cross-check, and the few places where the two disagree
//! (`<tspan opacity>`) are called out where they occur — this tool follows resvg,
//! as R51/R54 did.

use super::*;
use crate::dom;

fn bbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
    ink_bbox(&dom::parse(svg).unwrap())
}

/// `body` inside a bare canvas, so only the body's own paint matters.
fn inks(body: &str) -> bool {
    bbox(&format!("<svg>{body}</svg>")).is_some()
}

/// A 20x20 rect at (10, 10) whose paint is decided by `attrs`.
fn rect(attrs: &str) -> bool {
    inks(&format!(
        r##"<rect x="10" y="10" width="20" height="20" {attrs}/>"##
    ))
}

#[test]
fn a_tspan_restores_a_paint_the_outer_text_lacks() {
    // Measured d1 92 / cairo 95 for the fill, d1 158 / cairo 155 for the stroke,
    // 92/95 for a nested tspan that re-enables the fill, and 92/95 for a gradient
    // reference. The old judge read only the outer `<text>`, whose `fill="none"`
    // silenced all four, so real ink outside the canvas produced no W6 (R66b
    // P1-1: five such false negatives).
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="none"><tspan fill="#000">A</tspan></text>"##
    ));
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="none"><tspan stroke="#000" stroke-width="2">A</tspan></text>"##
    ));
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="none"><tspan fill="none"><tspan fill="#000">A</tspan></tspan></text>"##
    ));
    assert!(inks(
        r##"<svg><defs><linearGradient id="g"><stop offset="0" stop-color="#000"/></linearGradient></defs><text x="10" y="40" font-size="20" fill="none"><tspan fill="url(#g)">A</tspan></text></svg>"##
    ));
}

#[test]
fn a_hidden_text_is_inked_by_its_visible_tspan() {
    // Measured d1 92 / cairo 95: `visibility` is inherited, and a descendant may
    // restore it, so the outer `hidden` decides nothing for the tspan's glyphs.
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="#000" visibility="hidden"><tspan visibility="visible">A</tspan></text>"##
    ));
    // And a hidden tspan stays hidden even when the text paints (measured 0/0).
    assert!(!inks(
        r##"<text x="10" y="40" font-size="20" fill="none"><tspan fill="#000" visibility="hidden">A</tspan></text>"##
    ));
}

#[test]
fn a_non_painting_group_around_a_text_does_not_hide_a_painted_tspan() {
    // Measured d1 92 / cairo 95 with `<g fill="none">` above the `<text>`: the
    // tspan's own fill is what paints.
    assert!(inks(
        r##"<g fill="none"><text x="10" y="40" font-size="20"><tspan fill="#000">A</tspan></text></g>"##
    ));
}

#[test]
fn a_text_whose_only_glyphs_paint_nothing_contributes_no_ink() {
    // Measured 0 ink in both renderers for every case: the outer `<text>` paints
    // but no glyph-bearing run does. These were false W6 before R66b (P2-1's
    // text twin), and the old judge could not see them at all.
    for run in [
        r##"<tspan fill="none">A</tspan>"##,
        r##"<tspan fill="transparent">A</tspan>"##,
        r##"<tspan fill-opacity="0">A</tspan>"##,
        r##"<tspan style="fill-opacity:0">A</tspan>"##,
        r##"<tspan visibility="hidden">A</tspan>"##,
        r##"<tspan display="none">A</tspan>"##,
    ] {
        let markup =
            format!(r##"<svg><text x="10" y="40" font-size="20" fill="#000">{run}</text></svg>"##);
        assert!(bbox(&markup).is_none(), "{run} draws nothing");
    }
    // An inherited `fill-opacity="0"` on the text also silences a tspan that does
    // not restore it (measured 0/0).
    assert!(!inks(
        r##"<text x="10" y="40" font-size="20" fill-opacity="0"><tspan fill="#000">A</tspan></text>"##
    ));
    // Text with no characters at all is not ink either.
    assert!(!inks(
        r##"<text x="10" y="40" font-size="20"><tspan fill="#000"> </tspan></text>"##
    ));
}

#[test]
fn a_text_run_inks_when_any_of_its_characters_paint() {
    // Measured d1 92 / cairo 95: direct text next to a non-painting tspan keeps
    // the run visible, and so does a tspan nested in a painted one.
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="#000">A<tspan fill="none">BB</tspan></text>"##
    ));
}

#[test]
fn a_tspan_opacity_is_ignored_but_a_group_opacity_is_honoured() {
    // Measured: `<tspan opacity="0">` still inks (d1 92; cairosvg 0 — the two
    // renderers disagree, and this tool follows resvg). An `opacity` on a `<g>`
    // *inside* the text does hide its glyphs (d1 0), so only `tspan` is exempt.
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="#000"><tspan opacity="0">A</tspan></text>"##
    ));
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="none"><tspan fill="#000" opacity="0">A</tspan></text>"##
    ));
    assert!(!inks(
        r##"<text x="10" y="40" font-size="20" fill="#000"><g opacity="0"><tspan>A</tspan></g></text>"##
    ));
    // A sibling outside that group still inks (measured d1 124).
    assert!(inks(
        r##"<text x="10" y="40" font-size="20" fill="#000"><g opacity="0"><tspan>A</tspan></g>B</text>"##
    ));
}

#[test]
fn a_use_of_a_text_run_is_judged_by_its_tspan() {
    // Measured d1 92 / cairo 95 through `<use>`; and the reverse case — a black
    // text whose only tspan is `fill="none"` — is 0/0 through `<use>` as well,
    // because the instance inherits the tspan's override.
    assert!(inks(
        r##"<defs><text id="t" x="10" y="40" font-size="20" fill="none"><tspan fill="#000">A</tspan></text></defs><use href="#t"/>"##
    ));
    assert!(!inks(
        r##"<defs><text id="t" x="10" y="40" font-size="20" fill="#000"><tspan fill="none">A</tspan></text></defs><use href="#t"/>"##
    ));
}

#[test]
fn current_color_resolves_against_the_inherited_color() {
    // Measured: with `color="transparent"` the 400 px of fill vanish in both
    // renderers; with `color="#000"` they are all there. `color` inherits, and a
    // `!important` style declaration wins over the attribute (measured 400).
    assert!(!rect(r##"fill="currentColor" color="transparent""##));
    assert!(rect(r##"fill="currentColor" color="#000""##));
    assert!(!rect(r##"style="fill:currentColor;color:transparent""##));
    assert!(rect(
        r##"fill="currentColor" color="transparent" style="color:#000 !important""##
    ));
    assert!(!inks(
        r##"<g color="transparent"><rect x="10" y="10" width="20" height="20" fill="currentColor"/></g>"##
    ));
    // The keyword is case-sensitive: `currentcolor` is an invalid colour and both
    // renderers draw it black (measured 400), so it must not resolve to `color`.
    assert!(rect(r##"fill="currentcolor" color="transparent""##));
    // `stroke` uses the same keyword (measured 0 for a transparent `color`).
    assert!(!rect(
        r##"fill="none" stroke="currentColor" color="transparent""##
    ));
}

#[test]
fn a_pattern_reaches_its_content_through_href() {
    // Measured d1 400 / cairo 400 for `href`, `xlink:href` and a two-hop chain;
    // the old judge looked only at the pattern's own children, so an empty
    // `<pattern href="#b"/>` silenced the W6 entirely (R66b P1-2).
    let solid = r##"<pattern id="b" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="4" height="4" fill="#000"/></pattern>"##;
    for pattern in [
        r##"<pattern id="p" href="#b"/>"##,
        r##"<pattern id="p" xlink:href="#b"/>"##,
        r##"<pattern id="p" href="#m"/><pattern id="m" href="#b"/>"##,
        // A `<title>` child is not content, so the href chain is still followed
        // (measured d1 400).
        r##"<pattern id="p" href="#b"><title>t</title></pattern>"##,
    ] {
        let markup = format!(
            r##"<svg xmlns:xlink="http://www.w3.org/1999/xlink"><defs>{solid}{pattern}</defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/></svg>"##
        );
        assert!(bbox(&markup).is_some(), "{pattern} inherits b's ink");
    }
    // A missing target leaves the tile empty (measured 0/0).
    assert!(!inks(
        r##"<defs><pattern id="p" href="#nope"/></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
}

#[test]
fn a_pattern_with_non_painting_content_contributes_no_ink() {
    // Measured 0 ink in both renderers for all six: the child exists but paints
    // nothing. These were the five false W6 P2-1 named, plus an empty `<g>`.
    for child in [
        r##"<rect width="4" height="4" fill="none"/>"##,
        r##"<rect width="4" height="4" fill="transparent"/>"##,
        r##"<rect width="4" height="4" fill-opacity="0"/>"##,
        r##"<rect width="4" height="4" opacity="0"/>"##,
        r##"<rect width="4" height="4" visibility="hidden"/>"##,
        r##"<g fill="none"><rect width="4" height="4"/></g>"##,
        r##"<g/>"##,
        r##"<rect width="4" height="4" fill="currentColor" color="transparent"/>"##,
    ] {
        let markup = format!(
            r##"<svg><defs><pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4">{child}</pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/></svg>"##
        );
        assert!(bbox(&markup).is_none(), "{child} paints nothing");
    }
    // A child that restores a paint inside the non-painting group still inks
    // (measured d1 400 / cairo 400).
    assert!(inks(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4"><g fill="none"><rect width="4" height="4" fill="#000"/></g></pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
    // A display:none child is content that paints nothing, so the tile does not
    // fall back to its href (measured d1 0, unlike the `<title>` case above).
    assert!(!inks(
        r##"<defs><pattern id="b" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="4" height="4" fill="#000"/></pattern><pattern id="p" href="#b"><rect width="4" height="4" display="none"/></pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
}

#[test]
fn a_pattern_tile_is_judged_with_the_ink_walk_itself() {
    // The tile is a rendered subtree, so a `<text>` child and a `<use>` child are
    // measured exactly as they are outside a pattern (measured d1 70 and 400).
    assert!(inks(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="40"><text x="0" y="20" font-size="20" fill="#000">A</text></pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
    assert!(inks(
        r##"<defs><rect id="r" width="4" height="4" fill="#000"/><pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4"><use href="#r"/></pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
}

#[test]
fn a_self_referential_pattern_terminates_and_paints_nothing() {
    // resvg draws nothing for a tile whose only content paints the tile itself
    // (measured d1 0; cairosvg 2.9.1 dies with exit 1). The judge must already
    // answer `false` instead of recursing until the stack dies, for the direct
    // form and for a cycle between two patterns (measured 0/0).
    assert!(!inks(
        r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="4" height="4" fill="url(#p)"/></pattern></defs><rect x="10" y="10" width="20" height="20" fill="url(#p)"/>"##
    ));
    assert!(!inks(
        r##"<defs><pattern id="a" href="#b"/><pattern id="b" href="#a"/></defs><rect x="10" y="10" width="20" height="20" fill="url(#a)"/>"##
    ));
    // A cycle with real content on one leg still paints (measured d1 400).
    assert!(inks(
        r##"<defs><pattern id="a" patternUnits="userSpaceOnUse" width="4" height="4" href="#b"><rect width="4" height="4" fill="#000"/></pattern><pattern id="b" href="#a"/></defs><rect x="10" y="10" width="20" height="20" fill="url(#a)"/>"##
    ));
}
