//! R66: the ink judge must only count content that actually paints.
//!
//! Every rule below was measured against both reference renderers before it was
//! written (d1render and cairosvg 2.9.1, 100x80 canvas, a 20x20 shape at
//! (10, 10)); the comment on each test names the measured case. The old judge
//! tracked only `stroke`/`stroke-width`, so a `fill="none"` or hidden element
//! outside the canvas raised a false W6.

use super::*;
use crate::dom;

fn bbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
    ink_bbox(&dom::parse(svg).unwrap())
}

/// A 20x20 shape at (10, 10) with `attrs` spliced into its start tag.
fn rect(attrs: &str) -> Option<(f64, f64, f64, f64)> {
    bbox(&format!(
        r##"<svg><rect x="10" y="10" width="20" height="20" {attrs}/></svg>"##
    ))
}

#[test]
fn a_fill_none_shape_contributes_no_ink() {
    // Measured: fill="none" leaves both canvases blank for rect/circle/polygon/
    // path, and for an open path too (fill still applies to open subpaths).
    assert!(rect(r##"fill="none""##).is_none());
    assert!(bbox(r##"<svg><circle cx="10" cy="10" r="5" fill="none"/></svg>"##).is_none());
    assert!(bbox(r##"<svg><polygon points="0,0 10,0 10,10" fill="none"/></svg>"##).is_none());
    assert!(bbox(r##"<svg><path d="M 0 0 L 10 0 L 10 10" fill="none"/></svg>"##).is_none());
}

#[test]
fn a_fill_none_shape_with_a_painted_stroke_still_inks() {
    // Measured: fill="none" + stroke="#000" inks (d1 320 px, cairo 156 px).
    let box_ = rect(r##"fill="none" stroke="#000""##).unwrap();
    assert_eq!(box_, (9.5, 9.5, 30.5, 30.5));
}

#[test]
fn a_stroke_none_shape_is_still_filled() {
    // The default fill is black: `stroke="none"` alone must not silence a shape.
    assert_eq!(
        rect(r##"stroke="none""##).unwrap(),
        (10.0, 10.0, 30.0, 30.0)
    );
}

#[test]
fn a_line_needs_a_painted_stroke() {
    // Measured: a `<line>` is never filled — `fill="#000000"` leaves both
    // canvases blank, while a painted stroke inks it.
    assert!(bbox(r##"<svg><line x1="0" y1="0" x2="10" y2="0"/></svg>"##).is_none());
    assert!(bbox(r##"<svg><line x1="0" y1="0" x2="10" y2="0" fill="#000"/></svg>"##).is_none());
    assert!(bbox(r##"<svg><line x1="0" y1="0" x2="10" y2="0" stroke="#000"/></svg>"##).is_some());
}

#[test]
fn a_zero_fill_opacity_silences_the_fill() {
    // Measured: fill-opacity="0" is blank, "0.5" still inks (grey).
    assert!(rect(r##"fill="#000" fill-opacity="0""##).is_none());
    assert!(rect(r##"fill="#000" fill-opacity="0.5""##).is_some());
    // `fill-opacity` is inherited, and a child may restore it.
    assert!(
        bbox(
            r##"<svg><g fill-opacity="0"><rect x="0" y="0" width="1" height="1" fill-opacity="1"/></g></svg>"##
        )
        .is_some()
    );
}

#[test]
fn a_zero_stroke_opacity_silences_the_stroke() {
    // Measured: fill="none" + stroke-opacity="0" is blank, "0.5" still inks.
    assert!(rect(r##"fill="none" stroke="#000" stroke-opacity="0""##).is_none());
    assert!(rect(r##"fill="none" stroke="#000" stroke-opacity="0.5""##).is_some());
}

#[test]
fn a_non_positive_stroke_width_silences_the_stroke() {
    // Measured: stroke-width="0" and "-1" are blank. A tiny *positive* width is
    // kept on purpose: it rasterizes to nothing at 1e-9, but the judge must not
    // treat a declared non-zero width as zero (fail-open).
    assert!(rect(r##"fill="none" stroke="#000" stroke-width="0""##).is_none());
    assert!(rect(r##"fill="none" stroke="#000" stroke-width="-1""##).is_none());
    assert!(rect(r##"fill="none" stroke="#000" stroke-width="1e-9""##).is_some());
}

#[test]
fn fill_transparent_paints_nothing() {
    // Measured: `fill="transparent"` is blank; `fill="none"` likewise.
    assert!(rect(r##"fill="transparent""##).is_none());
}

#[test]
fn an_ancestor_stroke_still_inks_a_fill_none_child() {
    // Measured: `<g stroke="#000"><rect fill="none"/></g>` inks (d1 320 px).
    // The fix must not over-tighten: the stroke is inherited from the group.
    let box_ = bbox(
        r##"<svg><g stroke="#000"><rect x="10" y="10" width="20" height="20" fill="none"/></g></svg>"##,
    )
    .unwrap();
    assert_eq!(box_, (9.5, 9.5, 30.5, 30.5));
}

#[test]
fn visibility_hidden_silences_the_subtree_but_a_child_can_restore_it() {
    // Measured: hidden (attribute or style) is blank; a `visibility="visible"`
    // child inside a hidden `<g>` inks again.
    assert!(rect(r##"visibility="hidden""##).is_none());
    assert!(rect(r##"style="visibility: hidden""##).is_none());
    assert!(bbox(
        r##"<svg><g visibility="hidden"><rect x="0" y="0" width="1" height="1"/></g></svg>"##
    )
    .is_none());
    assert!(
        bbox(
            r##"<svg><g visibility="hidden"><rect x="0" y="0" width="1" height="1" visibility="visible"/></g></svg>"##
        )
        .is_some()
    );
    assert!(
        bbox(
            r##"<svg><g visibility="hidden"><rect x="0" y="0" width="1" height="1" style="visibility: visible"/></g></svg>"##
        )
        .is_some()
    );
}

#[test]
fn visibility_collapse_is_hidden() {
    // Measured: d1render (resvg) treats `collapse` as hidden; cairosvg draws it.
    // The judge follows d1render, like the panel-letter judge does.
    assert!(rect(r##"visibility="collapse""##).is_none());
}

#[test]
fn visibility_keywords_are_case_sensitive() {
    // Measured: `visibility="HIDDEN"` still inks in d1render (resvg matches the
    // keyword case-sensitively), so the judge must not lowercase it.
    assert!(rect(r##"visibility="HIDDEN""##).is_some());
}

#[test]
fn a_zero_opacity_hides_the_whole_subtree_and_cannot_be_restored() {
    // Measured: `opacity="0"` on a `<g>` hides a child that sets `opacity="1"`
    // or `visibility="visible"`; `opacity="0.5"` still inks (grey).
    assert!(rect(r##"opacity="0""##).is_none());
    assert!(rect(r##"opacity="-1""##).is_none());
    assert!(rect(r##"opacity="0.5""##).is_some());
    assert!(bbox(
        r##"<svg><g opacity="0"><rect x="0" y="0" width="1" height="1" opacity="1"/></g></svg>"##
    )
    .is_none());
    assert!(
        bbox(
            r##"<svg><g opacity="0"><rect x="0" y="0" width="1" height="1" visibility="visible"/></g></svg>"##
        )
        .is_none()
    );
}

#[test]
fn an_image_inks_unless_hidden_or_transparent() {
    // Measured: a black 1x1 image inks; `opacity="0"` and `visibility="hidden"`
    // are blank. An `<image>` has no fill or stroke, so only opacity/visibility
    // gate it.
    let image = r##"<image x="10" y="10" width="20" height="20" href="data:image/png;base64,iVBORw0KGgo="/>"##;
    assert!(bbox(&format!("<svg>{image}</svg>")).is_some());
    assert!(
        bbox(
            r##"<svg><image x="10" y="10" width="20" height="20" opacity="0" href="data:image/png;base64,iVBORw0KGgo="/></svg>"##
        )
        .is_none()
    );
    assert!(
        bbox(
            r##"<svg><image x="10" y="10" width="20" height="20" visibility="hidden" href="data:image/png;base64,iVBORw0KGgo="/></svg>"##
        )
        .is_none()
    );
}

#[test]
fn a_url_paint_that_does_not_resolve_paints_nothing() {
    // Measured: `fill="url(#missing)"` is blank; a fallback colour after it
    // inks; a gradient needs one painting stop; an empty pattern paints nothing.
    assert!(rect(r##"fill="url(#missing)""##).is_none());
    assert!(rect(r##"fill="url(#missing) #000""##).is_some());
    assert!(
        bbox(
            r##"<svg><defs><linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#000"/></linearGradient></defs><rect x="0" y="0" width="1" height="1" fill="url(#g)"/></svg>"##
        )
        .is_some()
    );
    assert!(
        bbox(
            r##"<svg><defs><linearGradient id="g"><stop offset="0" stop-color="transparent"/></linearGradient></defs><rect x="0" y="0" width="1" height="1" fill="url(#g)"/></svg>"##
        )
        .is_none()
    );
    assert!(
        bbox(
            r##"<svg><defs><pattern id="p"></pattern></defs><rect x="0" y="0" width="1" height="1" fill="url(#p)"/></svg>"##
        )
        .is_none()
    );
    assert!(
        bbox(
            r##"<svg><defs><pattern id="p"><rect width="1" height="1"/></pattern></defs><rect x="0" y="0" width="1" height="1" fill="url(#p)"/></svg>"##
        )
        .is_some()
    );
}

#[test]
fn a_use_of_non_painting_content_contributes_no_ink() {
    // Measured: `<use>` of a `fill="none"` rect is blank. The old fallback
    // counted the placement box whenever the referenced box was empty, which
    // turned this into a false W6.
    assert!(
        bbox(
            r##"<svg><defs><rect id="r" x="0" y="0" width="20" height="20" fill="none"/></defs><use href="#r" x="10" y="10"/></svg>"##
        )
        .is_none()
    );
    // A stroke on the `<use>` is inherited into the instance and inks it.
    assert!(
        bbox(
            r##"<svg><defs><rect id="r" x="0" y="0" width="20" height="20"/></defs><use href="#r" x="10" y="10" fill="none" stroke="#000"/></svg>"##
        )
        .is_some()
    );
}

#[test]
fn a_hidden_use_contributes_no_ink() {
    // Measured: `visibility="hidden"` and `opacity="0"` on a `<use>` are blank;
    // a `visibility="visible"` use inside a hidden `<g>` inks again.
    let defs = r##"<defs><rect id="r" x="0" y="0" width="20" height="20"/></defs>"##;
    assert!(bbox(&format!(
        r##"<svg>{defs}<use href="#r" x="10" y="10" visibility="hidden"/></svg>"##
    ))
    .is_none());
    assert!(bbox(&format!(
        r##"<svg>{defs}<use href="#r" x="10" y="10" opacity="0"/></svg>"##
    ))
    .is_none());
    assert!(
        bbox(&format!(
            r##"<svg><g visibility="hidden">{defs}<use href="#r" x="10" y="10" visibility="visible"/></g></svg>"##
        ))
        .is_some()
    );
}

#[test]
fn a_hidden_symbol_use_contributes_no_ink() {
    // The placement-box fallback for an unsupported `<symbol>` target must not
    // fire when the `<use>` itself is hidden.
    assert!(
        bbox(
            r##"<svg><defs><symbol id="s"><circle cx="0" cy="0" r="5"/></symbol></defs><use href="#s" x="10" y="10" visibility="hidden"/></svg>"##
        )
        .is_none()
    );
}

#[test]
fn text_needs_a_painted_fill_or_stroke() {
    // Measured: `fill="none"` and `fill-opacity="0"` leave the canvas blank;
    // `fill="none" stroke="#000"` inks the glyph outline.
    assert!(
        bbox(r##"<svg><text x="10" y="40" font-size="20" fill="none">A</text></svg>"##).is_none()
    );
    assert!(
        bbox(r##"<svg><text x="10" y="40" font-size="20" fill-opacity="0">A</text></svg>"##)
            .is_none()
    );
    assert!(bbox(
        r##"<svg><text x="10" y="40" font-size="20" fill="none" stroke="#000">A</text></svg>"##
    )
    .is_some());
    assert!(bbox(
        r##"<svg><text x="10" y="40" font-size="20" visibility="hidden">A</text></svg>"##
    )
    .is_none());
}

#[test]
fn a_zero_opacity_group_hides_its_text() {
    // Measured: `<g opacity="0"><text>A</text></g>` is blank.
    assert!(
        bbox(r##"<svg><g opacity="0"><text x="10" y="40" font-size="20">A</text></g></svg>"##)
            .is_none()
    );
}
