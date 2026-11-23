//! `<defs>` reference handling (round-data/R37).
//!
//! Three properties are pinned here, because each one used to fail silently:
//!
//! * what survives pruning is decided by **references**, not by element names —
//!   a used `<mask>`/`<marker>`/`<filter>` stays, an unused one still goes;
//! * a reference that resolves to nothing is a **hard error** naming the element,
//!   the attribute and the missing id (the renderer would otherwise drop the
//!   shape or paint it unmasked, at rc 0);
//! * a `<mask>`/`<marker>` body is drawn, so its geometry is translated like any
//!   other geometry — a `<path>` in there must not survive as a `<path>`.

use crate::error::ConvertError;
use crate::options::Conversion;
use crate::pipeline::convert;
use crate::test_support::converted;

const HEAD: &str = "<svg width='100' height='80' viewBox='0 0 100 80' \
                    xmlns='http://www.w3.org/2000/svg' \
                    xmlns:xlink='http://www.w3.org/1999/xlink'>";

fn svg(body: &str) -> String {
    format!("{HEAD}{body}</svg>")
}

/// The `Translate` detail of a conversion that must fail closed.
fn translate_error(source: &str) -> String {
    match convert(source, false) {
        Err(ConvertError::Translate(detail)) => detail,
        other => panic!("expected a Translate error, got {other:?}"),
    }
}

#[test]
fn a_referenced_mask_marker_and_filter_all_survive_pruning() {
    let done = converted(&svg("<defs>\
         <mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='50' height='80' fill='white'/></mask>\
         <marker id='k' markerWidth='6' markerHeight='6' refX='0' refY='3' orient='auto'>\
         <path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/></marker>\
         <filter id='f' filterUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <feMorphology operator='dilate' radius='2'/></filter>\
         </defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>\
         <path d='M 10 60 L 90 60' stroke='black' marker-end='url(#k)'/>\
         <rect x='60' y='10' width='20' height='20' fill='black' filter='url(#f)'/>"));
    for id in ["m", "k", "f"] {
        assert!(
            done.svg.contains(&format!("id=\"{id}\"")),
            "a referenced <defs> child must survive: {id} missing from {}",
            done.svg
        );
    }
    assert!(done.svg.contains("<feMorphology"), "{}", done.svg);
}

#[test]
fn an_unreferenced_mask_marker_and_filter_are_still_dropped() {
    // Pruning is by reference, not "keep everything": an unused definition is
    // dead weight in every output file.
    let done = converted(&svg("<defs>\
         <mask id='dead' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='10' height='10' fill='white'/></mask>\
         <marker id='deadk' markerWidth='6' markerHeight='6' refX='0' refY='3'>\
         <path d='M 0 0 L 6 3 L 0 6 Z'/></marker>\
         <filter id='deadf'><feGaussianBlur stdDeviation='2'/></filter>\
         </defs>\
         <rect x='10' y='10' width='40' height='20' fill='black'/>"));
    for id in ["dead", "deadk", "deadf"] {
        assert!(
            !done.svg.contains(&format!("id=\"{id}\"")),
            "an unreferenced <defs> child must still be dropped: {id} in {}",
            done.svg
        );
    }
}

#[test]
fn a_reference_closure_keeps_the_paint_server_a_pattern_points_at() {
    // The `<pattern>` is referenced from a shape; the gradient is referenced only
    // from *inside* the pattern. A one-level scan would drop the gradient and the
    // hatch would paint nothing.
    let done = converted(&svg("<defs>\
         <linearGradient id='g' gradientUnits='userSpaceOnUse' x1='0' y1='0' x2='10' y2='10'>\
         <stop offset='0' stop-color='#ffffff'/></linearGradient>\
         <pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='10'>\
         <rect x='0' y='0' width='10' height='10' fill='url(#g)'/></pattern>\
         </defs>\
         <rect x='0' y='0' width='40' height='40' fill='url(#p)'/>"));
    assert!(done.svg.contains("<pattern"), "{}", done.svg);
    assert!(done.svg.contains("<linearGradient"), "{}", done.svg);
    assert!(done.svg.contains("url(#g)"), "{}", done.svg);
}

#[test]
fn an_href_reference_keeps_its_target() {
    // `xlink:href="#id"` is a reference too (gradient inheritance), and it is not
    // a `url(#…)`, so the pruning scan has to look at both spellings.
    let done = converted(&svg("<defs>\
         <linearGradient id='base'><stop offset='0' stop-color='#ffffff'/></linearGradient>\
         <linearGradient id='derived' xlink:href='#base' x1='0' y1='0' x2='1' y2='0'/>\
         </defs>\
         <rect x='0' y='0' width='40' height='40' fill='url(#derived)'/>"));
    assert!(done.svg.contains("id=\"base\""), "{}", done.svg);
    assert!(done.svg.contains("id=\"derived\""), "{}", done.svg);
    assert!(done.svg.contains("xlink:href=\"#base\""), "{}", done.svg);
}

#[test]
fn a_dangling_mask_reference_is_a_hard_error() {
    let detail = translate_error(&svg(
        "<rect x='10' y='10' width='40' height='20' fill='black' mask='url(#nope)'/>",
    ));
    // The message has to name all three: the element, the attribute and the id.
    assert!(detail.contains("<rect>"), "{detail}");
    assert!(detail.contains("mask="), "{detail}");
    assert!(detail.contains("#nope"), "{detail}");
    assert!(detail.contains("add the missing definition"), "{detail}");
}

#[test]
fn a_dangling_clip_path_on_an_untransformed_shape_is_a_hard_error() {
    // `apply_clip_refs` only sees clips referenced from a *transformed* context,
    // so an untransformed shape with a dangling clip used to convert at rc 0 and
    // then render unclipped.
    let detail = translate_error(&svg(
        "<rect x='10' y='10' width='40' height='20' fill='black' clip-path='url(#gone)'/>",
    ));
    assert!(detail.contains("clip-path="), "{detail}");
    assert!(detail.contains("#gone"), "{detail}");
}

#[test]
fn a_dangling_href_reference_is_a_hard_error() {
    let detail = translate_error(&svg(
        "<defs><linearGradient id='derived' xlink:href='#missing'/></defs>\
         <rect x='0' y='0' width='40' height='40' fill='url(#derived)'/>",
    ));
    assert!(detail.contains("<linearGradient>"), "{detail}");
    assert!(detail.contains("#missing"), "{detail}");
}

#[test]
fn an_anchor_href_is_not_a_render_reference() {
    // `<a href="#section">` is a hyperlink, not a paint/clip reference: treating it
    // as one would reject perfectly valid documents.
    let done = converted(&svg(
        "<a href='#nowhere'><rect x='0' y='0' width='40' height='40' fill='black'/></a>",
    ));
    assert!(done.svg.contains("<rect"), "{}", done.svg);
}

#[test]
fn a_url_reference_with_inner_spaces_still_resolves() {
    // `url( #m )` is legal CSS and appears in hand-written SVGs.
    let done = converted(&svg(
        "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='50' height='80' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url( #m )'/>",
    ));
    assert!(done.svg.contains("id=\"m\""), "{}", done.svg);
}

#[test]
fn a_self_referential_pattern_does_not_loop() {
    // The closure walk must terminate on a cycle instead of spinning.
    let done = converted(&svg(
        "<defs><pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='10'>\
         <rect x='0' y='0' width='10' height='10' fill='url(#p)'/></pattern></defs>\
         <rect x='0' y='0' width='40' height='40' fill='url(#p)'/>",
    ));
    assert!(done.svg.contains("<pattern"), "{}", done.svg);
}

#[test]
fn duplicate_ids_do_not_break_pruning() {
    // Duplicate ids are invalid SVG but common in generated files; the converter
    // keeps both definitions rather than failing on the ambiguity.
    let done = converted(&svg(
        "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='50' height='80' fill='white'/></mask>\
         <mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='20' height='80' fill='white'/></mask></defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>",
    ));
    assert_eq!(done.svg.matches("id=\"m\"").count(), 2, "{}", done.svg);
}

#[test]
fn mask_and_marker_bodies_are_translated_like_any_other_geometry() {
    // Both bodies are drawn, so a `<path>` in them must become a polygon/polyline;
    // leaving it would also trip the converter's own residue check.
    let done = converted(&svg("<defs>\
         <mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <path d='M 0 0 L 50 0 L 50 80 Z' fill='white'/></mask>\
         <marker id='k' markerWidth='6' markerHeight='6' refX='0' refY='3' orient='auto'>\
         <path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/></marker>\
         </defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>\
         <path d='M 10 60 L 90 60' stroke='black' marker-end='url(#k)'/>"));
    assert!(!done.svg.contains("<path"), "{}", done.svg);
    assert!(done.svg.contains("<polygon"), "{}", done.svg);
}

#[test]
fn a_filter_body_is_left_alone_and_its_path_stays_residue() {
    // A `<filter>` body is a list of filter primitives, not geometry: baking a
    // `<path>` in there would be meaningless, so the stage does not descend and the
    // self-check refuses the output instead of writing something half-translated.
    match convert(
        &svg(
            "<defs><filter id='f'><feMorphology operator='dilate' radius='2'/>\
             <path d='M 0 0 L 10 10'/></filter></defs>\
             <rect x='10' y='10' width='40' height='20' fill='black' filter='url(#f)'/>",
        ),
        false,
    ) {
        Err(ConvertError::SelfCheck(detail)) => {
            assert!(detail.contains("untranslated dialect nodes"), "{detail}");
            assert!(detail.contains("<path>"), "{detail}");
        }
        other => panic!("expected a SelfCheck error, got {other:?}"),
    }
}

#[test]
fn a_converted_output_keeps_the_defs_it_needs_and_nothing_else() {
    // End-to-end shape of the fix: one used and one unused definition of each
    // kind, so the pruning decision is visible in a single output.
    let done = converted(&svg(
        "<defs>\
         <mask id='used' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='50' height='80' fill='white'/></mask>\
         <mask id='unused' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>\
         <rect x='0' y='0' width='10' height='10' fill='white'/></mask>\
         </defs>\
         <rect x='10' y='10' width='40' height='20' fill='black' mask='url(#used)'/>",
    ));
    assert!(done.svg.contains("id=\"used\""), "{}", done.svg);
    assert!(!done.svg.contains("id=\"unused\""), "{}", done.svg);
    assert!(matches!(
        convert(&done.svg, false),
        Ok(Conversion::AlreadyConverted { .. })
    ));
}

/// A paint server defined under one wrapper level per nesting shape: `<defs>`
/// children are not required to be definitions, and a `<rect fill="url(#g)">`
/// that survives pruning while its gradient does not paints **nothing** at rc 0.
///
/// The wrapper is not kept as a container: it was never drawn where it sat, so
/// its kept descendants are hoisted into the `<defs>` and the wrapper dropped.
#[test]
fn a_definition_nested_in_a_container_survives_pruning() {
    let gradient = "<linearGradient id='g'>\
         <stop offset='0' stop-color='#f00'/><stop offset='1' stop-color='#00f'/>\
         </linearGradient>";
    let rect = "<rect x='10' y='10' width='80' height='60' fill='url(#g)'/>";
    let shapes = [
        ("<defs>{DEF}</defs>", "defs"),
        ("<defs><g>{DEF}</g></defs>", "defs/g"),
        ("<defs><g id='grp'>{DEF}</g></defs>", "defs/g[id]"),
        ("<g><defs>{DEF}</defs></g>", "g/defs"),
        ("<defs><defs>{DEF}</defs></defs>", "defs/defs"),
        ("<defs><symbol id='s'>{DEF}</symbol></defs>", "defs/symbol"),
        (
            "<defs><g><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' \
          width='100' height='80'><rect x='0' y='0' width='50' height='80' \
          fill='white'/></mask></g></defs>",
            "defs/g[mask]",
        ),
    ];
    for (shape, label) in shapes {
        let mut body = shape.replace("{DEF}", gradient);
        body.push_str(rect);
        // The mask shape references its own definition, not the gradient.
        if label == "defs/g[mask]" {
            body = body.replace("fill='url(#g)'", "fill='black' mask='url(#m)'");
        }
        let source = svg(&body);
        let done = converted(&source);
        let id = if label == "defs/g[mask]" { "m" } else { "g" };
        assert!(
            done.svg.contains(&format!("id=\"{id}\"")),
            "{label}: the nested definition must survive, got {}",
            done.svg
        );
        assert!(
            !done.svg.contains("<symbol"),
            "{label}: a <symbol> wrapper is not kept, got {}",
            done.svg
        );
        // The output must be self-consistent enough to be re-read as converted.
        assert!(
            matches!(
                convert(&done.svg, false),
                Ok(Conversion::AlreadyConverted { .. })
            ),
            "{label}: output is not recognised as converted: {}",
            done.svg
        );
    }
}

/// The reference closure has to cross the wrapper too: a `<pattern>` nested in a
/// `<g>` points at a gradient nested next to it, and both must survive together.
#[test]
fn the_reference_closure_follows_a_wrapped_definition() {
    let done = converted(&svg("<defs><g>\
         <linearGradient id='stops'><stop offset='0' stop-color='#f00'/>\
         <stop offset='1' stop-color='#00f'/></linearGradient>\
         <pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='10'>\
         <rect x='0' y='0' width='10' height='10' fill='url(#stops)'/></pattern>\
         </g></defs>\
         <rect x='5' y='5' width='60' height='40' fill='url(#p)'/>"));
    for id in ["stops", "p"] {
        assert!(done.svg.contains(&format!("id=\"{id}\"")), "{}", done.svg);
    }
    assert!(!done.svg.contains("<g>"), "{}", done.svg);
}
