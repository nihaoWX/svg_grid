//! Unit tests for R61: a **finite** source number times an axis factor can still
//! overflow `f32`, and the old code wrote the result back as `inf`/`NaN` — which is
//! not a legal SVG number and made the output unrenderable.
//!
//! The rule under test is uniform: the product must be finite, otherwise the
//! original text is kept **byte for byte** and warning W7 names the input. The
//! overflow corpus and the pixel oracle live in `docs/PITFALLS/round-data/R61/`.

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

/// 8×: `1e38 * 8` overflows `f32`.
const BIG: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 800.0,
    height: 640.0,
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

fn run_with(svg: &str, target: Rect) -> (String, Passthroughs) {
    run_with_source(svg, SRC, target)
}

fn run_with_source(svg: &str, source: Rect, target: Rect) -> (String, Passthroughs) {
    let mut root = dom::parse(svg).unwrap();
    let mut passthroughs = Passthroughs::default();
    transform_svg(&mut root, source, target, 0, &mut passthroughs).unwrap();
    (dom::serialize(&root).unwrap(), passthroughs)
}

/// The **last** element with this name: the panel box comes first in the fixture,
/// the actual body shape last.
fn find<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    let mut found = (element.name == name).then_some(element);
    for child in &element.children {
        if let Node::Element(child) = child {
            if let Some(hit) = find(child, name) {
                found = Some(hit);
            }
        }
    }
    found
}

fn attr(svg: &str, name: &str, attr_name: &str) -> Option<String> {
    let root = dom::parse(svg).unwrap();
    find(&root, name)?.attr(attr_name).map(str::to_string)
}

fn assert_no_new_non_finite(out: &str, source: &str) {
    for token in ["inf", "NaN"] {
        assert!(
            out.matches(token).count() <= source.matches(token).count(),
            "the output introduced {token}: {out}"
        );
    }
}

fn w7(svg: &str, element: &str, attr_name: &str) -> String {
    let (out, passthroughs) = run_with(svg, BIG);
    assert_no_new_non_finite(&out, svg);
    passthroughs
        .offenders()
        .iter()
        .find(|offender| offender.element == element && offender.attr == attr_name)
        .unwrap_or_else(|| panic!("{element} {attr_name} was not reported as W7"))
        .detail
        .clone()
}

// --- P2: computed writebacks must not overflow ------------------------------

#[test]
fn an_overflowing_rect_keeps_all_four_attributes() {
    let body = r##"<rect x="1e38" y="10" width="20" height="20"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    // One bad side invalidates the box: writing `inf`/`NaN` (or half a box) would
    // make the shape unrenderable, so the element stays exactly as written.
    assert_eq!(attr(&out, "rect", "x"), Some("1e38".into()));
    assert_eq!(attr(&out, "rect", "y"), Some("10".into()));
    assert_eq!(attr(&out, "rect", "width"), Some("20".into()));
    assert_eq!(attr(&out, "rect", "height"), Some("20".into()));
    let detail = w7(&svg, "rect", "x");
    assert!(detail.contains("1e38"), "{detail}");
}

#[test]
fn an_overflowing_pair_is_kept_on_both_axes() {
    let body = r##"<image x="1e38" y="10" width="20" height="20"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(attr(&out, "image", "x"), Some("1e38".into()));
    assert_eq!(attr(&out, "image", "y"), Some("10".into()));
    // `image` uses the same box rewrite as `rect`: one bad side keeps all four.
    assert_eq!(attr(&out, "image", "width"), Some("20".into()));
    w7(&svg, "image", "x");
}

#[test]
fn an_overflowing_axis_keeps_that_axis_only() {
    let body = r##"<circle cx="1e38" cy="10" r="40"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(attr(&out, "circle", "cx"), Some("1e38".into()));
    // `cx`/`cy` are written as a pair, so the bad axis keeps both verbatim.
    assert_eq!(attr(&out, "circle", "cy"), Some("10".into()));
    assert_eq!(attr(&out, "circle", "r"), Some("320".into()));
    w7(&svg, "circle", "cx");
}

#[test]
fn an_overflowing_point_list_is_kept_verbatim() {
    let body = r##"<polygon points="1e38,10 20,20 10,30"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "polygon", "points"),
        Some("1e38,10 20,20 10,30".into())
    );
    w7(&svg, "polygon", "points");
}

#[test]
fn an_overflowing_path_is_kept_verbatim() {
    let body = r##"<path d="M 1e38 10 L 20 20 L 10 30 Z"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "path", "d"),
        Some("M 1e38 10 L 20 20 L 10 30 Z".into())
    );
    let detail = w7(&svg, "path", "d");
    assert!(detail.contains("scaled path"), "{detail}");
}

#[test]
fn an_overflowing_filter_pair_is_kept_verbatim() {
    let body = r##"<defs><filter id="f"><feGaussianBlur stdDeviation="3e38"/></filter></defs><rect x="10" y="10" width="20" height="20" filter="url(#f)"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "feGaussianBlur", "stdDeviation"),
        Some("3e38".into())
    );
    w7(&svg, "feGaussianBlur", "stdDeviation");
}

#[test]
fn an_overflowing_displacement_scale_is_kept_verbatim() {
    let body = r##"<defs><filter id="f"><feDisplacementMap in="SourceGraphic" scale="3e38"/></filter></defs><rect x="10" y="10" width="20" height="20" filter="url(#f)"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "feDisplacementMap", "scale"),
        Some("3e38".into())
    );
    w7(&svg, "feDisplacementMap", "scale");
}

#[test]
fn an_overflowing_matrix_is_kept_verbatim() {
    let body = r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" gradientTransform="translate(1e38 0)"/></defs><rect x="10" y="10" width="20" height="20" fill="url(#g)"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "linearGradient", "gradientTransform"),
        Some("translate(1e38 0)".into())
    );
    let detail = w7(&svg, "linearGradient", "gradientTransform");
    assert!(detail.contains("scaled transform"), "{detail}");
}

#[test]
fn an_overflowing_rotate_centre_is_kept_verbatim() {
    let body = r##"<rect x="10" y="10" width="20" height="20" transform="rotate(30 1e38 10)"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "rect", "transform"),
        Some("rotate(30 1e38 10)".into())
    );
    w7(&svg, "rect", "transform");
}

#[test]
fn an_overflowing_style_token_keeps_bytes_and_warns() {
    let body =
        r##"<rect x="10" y="10" width="20" height="20" style="stroke-width:3e38;fill:none"/>"##;
    let svg = doc("xy", body);
    let out = run_in(&svg, BIG);
    assert_eq!(
        attr(&out, "rect", "style"),
        Some("stroke-width:3e38;fill:none".into())
    );
    let detail = w7(&svg, "rect", "stroke-width");
    assert!(detail.contains("style token"), "{detail}");
}

#[test]
fn a_non_finite_factor_keeps_every_length_verbatim() {
    // The source canvas is so tiny that `target / source` is `inf`: the factor
    // itself is not finite, and every writeback must keep its input.
    let source = Rect {
        x: 0.0,
        y: 0.0,
        width: 1e-6,
        height: 1e-6,
    };
    let svg = doc("xy", r##"<rect x="10" y="10" width="20" height="20"/>"##);
    let (out, passthroughs) = run_with_source(
        &svg,
        source,
        Rect {
            x: 0.0,
            y: 0.0,
            width: f32::MAX,
            height: f32::MAX,
        },
    );
    assert_no_new_non_finite(&out, &svg);
    assert_eq!(attr(&out, "rect", "x"), Some("10".into()));
    assert_eq!(attr(&out, "rect", "width"), Some("20".into()));
    assert!(
        passthroughs
            .offenders()
            .iter()
            .any(|offender| offender.detail.contains("non-finite scale factor")),
        "{:?}",
        passthroughs
            .offenders()
            .iter()
            .map(|offender| (&offender.element, &offender.attr, &offender.detail))
            .collect::<Vec<_>>()
    );
}

// --- Counter-proof: finite values still scale -------------------------------

#[test]
fn ordinary_lengths_still_scale() {
    let svg = doc("xy", r##"<rect x="10" y="10" width="20" height="20"/>"##);
    let (out, passthroughs) = run_with(&svg, BIG);
    assert_eq!(attr(&out, "rect", "x"), Some("80".into()));
    assert_eq!(attr(&out, "rect", "y"), Some("80".into()));
    assert_eq!(attr(&out, "rect", "width"), Some("160".into()));
    assert_eq!(attr(&out, "rect", "height"), Some("160".into()));
    assert!(passthroughs.offenders().is_empty());
}

#[test]
fn a_finite_huge_length_still_scales() {
    let body = r##"<rect x="1e9" y="10" width="20" height="20"/>"##;
    let svg = doc("xy", body);
    let (out, passthroughs) = run_with(&svg, BIG);
    assert_eq!(attr(&out, "rect", "x"), Some("8000000000".into()));
    assert!(passthroughs.offenders().is_empty());
}
