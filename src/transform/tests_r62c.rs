//! R62 follow-up: a `href`/`xlink:href` between two definitions is a reference
//! like `url(#…)` is — the copy it points at follows the **referrer's own
//! content domain**, not the slot the referrer was instantiated in, and the
//! switch is transitive along the chain.
//!
//! Geometry is the same probe as the evidence: source 100×80, target 200×160
//! (2×), so a user-space tile of 20 becomes 40 while a verbatim copy keeps its
//! numbers. The hand-fixed products in `docs/PITFALLS/round-data/R62/` are 0/0
//! against the source rendered at 2×.

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

const TILE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 160.0,
};

fn run_text(svg: &str) -> String {
    let mut root = dom::parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, SRC, TILE, 0, &mut passthroughs).unwrap();
    dom::serialize(&root).unwrap()
}

fn panel(policy: &str, defs: &str, body: &str) -> String {
    format!(
        r##"<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<defs>{defs}</defs><g data-scale="{policy}">{body}</g></svg>"##
    )
}

/// The definition a `href` points at must be resolved by the **referrer's own
/// content domain**, not by the slot the referrer was instantiated in: a pattern
/// with a `viewBox` is scaled as a tile for a user-space rect, yet everything it
/// inherits through `href` lives in its viewBox units and must stay verbatim
/// (hand-fixed product: xor 0, before: 12800/255).
#[test]
fn a_definition_href_follows_its_own_content_domain() {
    let text = run_text(&panel(
        "xy",
        r##"<pattern id="q" patternUnits="userSpaceOnUse" width="10" height="10">
              <rect width="10" height="10" fill="#0a0"/>
            </pattern>
            <pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20"
                     viewBox="0 0 20 20" href="#q"/>"##,
        r##"<rect width="50" height="80" fill="url(#q)"/>
            <rect x="50" width="50" height="80" fill="url(#p)"/>"##,
    ));
    // `q` is a tile in panel user space (10 → 20) and a verbatim tile for `p`'s
    // body, which reads it in viewBox units (10 stays 10).
    assert!(
        text.contains(r##"id="q" patternUnits="userSpaceOnUse" width="20" height="20""##),
        "the panel-centred copy of `q` still scales: {text}"
    );
    assert!(
        text.contains(
            r##"id="q--svg-grid-verbatim" patternUnits="userSpaceOnUse" width="10" height="10""##
        ),
        "the verbatim copy of the tile must exist: {text}"
    );
    // `p` is instantiated in user space (its tile scales), but its `href` edge is
    // an edge of its mapped body: it must point at the verbatim copy of `q`.
    assert!(
        text.contains(r##"id="p" patternUnits="userSpaceOnUse" width="40" height="40""##)
            && text.contains(r##"viewBox="0 0 20 20" href="#q--svg-grid-verbatim""##),
        "the mapped tile must inherit from the verbatim copy: {text}"
    );
}

/// A gradient in `objectBoundingBox` units reads the attributes it inherits from
/// `href` in bounding-box units too, so that copy must not be scaled for the
/// panel (hand-fixed product: xor 0, before: 16000/126).
#[test]
fn a_bounding_box_gradient_href_stays_verbatim() {
    let text = run_text(&panel(
        "xy",
        r##"<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0.5" y2="0">
              <stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/>
            </linearGradient>
            <linearGradient id="h" gradientUnits="objectBoundingBox" href="#g"/>"##,
        r##"<rect width="50" height="80" fill="url(#g)"/>
            <rect x="50" width="50" height="80" fill="url(#h)"/>"##,
    ));
    // The panel reads `g` in user space (x2 0.5 → 1); `h` reads it in
    // bounding-box units, where the source value is already the final one.
    assert!(
        text.contains(r##"id="g--svg-grid-verbatim" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0.5" y2="0""##),
        "an unscaled copy of the inherited gradient must exist: {text}"
    );
    assert!(
        text.contains(r##"id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="1" y2="0""##),
        "the panel's own copy stays scaled: {text}"
    );
    assert!(
        text.contains(r##"id="h" gradientUnits="objectBoundingBox" href="#g--svg-grid-verbatim""##),
        "the bounding-box gradient must inherit the verbatim copy: {text}"
    );
}

/// The domain switch is transitive: the middle definition is instantiated in both
/// domains (a user-space rect fills it directly, a `viewBox` body inherits it), and
/// each copy must point at the copy of the last link for **its** domain
/// (hand-fixed: xor 0, before: 2304/255).
#[test]
fn a_href_chain_is_transitive() {
    let text = run_text(&panel(
        "xy",
        r##"<pattern id="r" patternUnits="userSpaceOnUse" width="4" height="4">
              <rect width="4" height="4" fill="#0a0"/>
            </pattern>
            <pattern id="q" patternUnits="userSpaceOnUse" width="8" height="8" href="#r"/>
            <pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20"
                     viewBox="0 0 20 20" href="#q"/>"##,
        r##"<rect width="40" height="80" fill="url(#q)"/>
            <rect x="40" width="60" height="80" fill="url(#p)"/>"##,
    ));
    assert!(
        text.contains(r##"viewBox="0 0 20 20" href="#q--svg-grid-verbatim""##),
        "level two must inherit the verbatim copy: {text}"
    );
    assert!(
        text.contains(r##"id="q--svg-grid-verbatim" patternUnits="userSpaceOnUse" width="8" height="8" href="#r--svg-grid-verbatim""##),
        "level three must inherit the verbatim copy too: {text}"
    );
    assert!(
        text.contains(r##"id="q" patternUnits="userSpaceOnUse" width="16" height="16" href="#r""##),
        "the user-space copy keeps the user-space link: {text}"
    );
}

/// The gradient sibling of the transitive case, three layers deep: `h3` is an
/// `objectBoundingBox` gradient, so **its own body** is mapped even when the
/// composer instantiates the `h3` copy in panel user space. Its first `href`
/// edge therefore already points at `h2`'s verbatim copy, and that copy's edge
/// at `g`'s. R62's first cut (cc66d4b) decided the edge from the referrer's
/// **slot** (`h3` is user space here), so it left `h3 → h2` scaled — the R62
/// review found the three-layer assertion had been reduced to one layer.
#[test]
fn a_three_layer_gradient_href_chain_switches_every_copy() {
    let text = run_text(&panel(
        "xy",
        r##"<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0">
              <stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/>
            </linearGradient>
            <linearGradient id="h2" gradientUnits="userSpaceOnUse" href="#g" x1="2"/>
            <linearGradient id="h3" gradientUnits="objectBoundingBox" href="#h2"/>
            <pattern id="pv" patternUnits="userSpaceOnUse" width="20" height="20"
                     viewBox="0 0 10 10"><rect width="10" height="10" fill="url(#h3)"/></pattern>"##,
        r##"<rect width="30" height="80" fill="url(#h3)"/>
            <rect x="30" width="30" height="80" fill="url(#h2)"/>
            <rect x="60" width="40" height="80" fill="url(#pv)"/>"##,
    ));
    // The panel-side copy of h3 is still drawn in bounding-box units, so its
    // first edge is an edge of a mapped body and must switch to the verbatim
    // copy of h2 (this is the assertion cc66d4b fails).
    assert!(
        text.contains(
            r##"id="h3" gradientUnits="objectBoundingBox" href="#h2--svg-grid-verbatim""##
        ),
        "the bbox gradient must inherit h2's verbatim copy: {text}"
    );
    assert!(
        text.contains(
            r##"id="h3--svg-grid-verbatim" gradientUnits="objectBoundingBox" href="#h2--svg-grid-verbatim""##
        ),
        "its verbatim copy must inherit the same: {text}"
    );
    assert!(
        text.contains(
            r##"id="h2--svg-grid-verbatim" gradientUnits="userSpaceOnUse" href="#g--svg-grid-verbatim""##
        ),
        "the second edge must end in g's verbatim copy: {text}"
    );
    assert!(
        text.contains(r##"id="h2" gradientUnits="userSpaceOnUse" href="#g""##),
        "the user-space copy keeps the user-space link: {text}"
    );
    assert!(
        text.contains(
            r##"id="g--svg-grid-verbatim" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0""##
        ),
        "the verbatim gradient keeps its numbers: {text}"
    );
}
