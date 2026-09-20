//! Top-level conversion orchestration.

use crate::dom::{parse, serialize};
use crate::error::ConvertError;
use crate::geom::canvas_box;
use crate::options::{Conversion, Converted, Report};
use crate::scan::scan;
use crate::self_check::self_check;
use crate::stroke;
use crate::tag::{
    count_panel_boxes, count_primitives, has_data_scale, insert_panel_box, resolve_percentages,
    wrap_root_content_in_xy, wrap_text_circles, Stats,
};
use crate::translate::translate_dialect;
use crate::warnings::collect_warnings;

/// Convert one SVG document. Pure: no I/O, nothing written.
///
/// The order is deliberate: **validate first, then decide idempotency, then
/// convert** — the "already converted" shortcut must never skip validation.
pub fn convert(source: &str, force: bool) -> Result<Conversion, ConvertError> {
    let mut root = parse(source)?;

    // 1. Validate the root element and the canvas box (never short-circuited).
    let canvas = match canvas_box(&root) {
        Ok(canvas) => canvas,
        Err(findings) => return Ok(Conversion::Incompatible(findings)),
    };
    resolve_percentages(&mut root, &canvas);

    // 2. Full fail-closed scan of the whole tree.
    let findings = scan(&root);
    if !findings.is_empty() {
        return Ok(Conversion::Incompatible(findings));
    }

    // 3. Idempotency, only for a *complete* conversion product.
    let boxes = count_panel_boxes(&root);
    if !force && boxes >= 1 {
        let has_scale = has_data_scale(&root);
        if boxes == 1 && has_scale {
            return Ok(Conversion::AlreadyConverted {
                canvas: canvas.as_string(),
            });
        }
        let detail = if has_scale {
            format!(
                "input contains {boxes} `data-panel-box=\"main\"` rect(s) but a complete conversion \
                 product must have exactly one; re-run with --force to rebuild it"
            )
        } else {
            format!(
                "input already contains {boxes} `data-panel-box=\"main\"` rect(s) but no `data-scale` \
                 grouping, so it is a half-finished conversion; re-run with --force to complete it \
                 (or remove the panel box)"
            )
        };
        return Err(ConvertError::Incomplete(detail));
    }

    // 4. Convert: translate the matplotlib dialect into the composer's
    // vocabulary (affine baking, path flattening, use expansion, image baking).
    // `translate_dialect` also normalizes the remaining (svglite) text
    // transforms, in the order that lets a translate-rotate text under a
    // translated ancestor be baked exactly.
    let warnings = collect_warnings(&root);
    translate_dialect(&mut root)?;
    // Give every stroked primitive an explicit `stroke-width` so the composer can
    // rescale it; a shape relying on the SVG default would stay a hairline.
    stroke::materialize_stroke_width(&mut root);

    let had_panel = boxes >= 1;
    if !had_panel {
        insert_panel_box(&mut root, &canvas);
    }

    let mut stats = Stats::default();
    wrap_text_circles(&mut root, false, &mut stats);
    wrap_root_content_in_xy(&mut root);

    self_check(&root)?;

    let svg = serialize(&root);
    let report = Report {
        canvas: canvas.as_string(),
        panel_box: if had_panel {
            "kept (already present)".to_string()
        } else {
            "inserted".to_string()
        },
        texts: stats.texts,
        circles: stats.circles,
        primitives: count_primitives(&root),
    };

    Ok(Conversion::Converted(Box::new(Converted {
        svg,
        report,
        warnings,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;
    use crate::options::Conversion;
    use crate::self_check::self_check;
    use crate::test_support::{convert_with, converted, incompatible, SVGLITE};

    #[test]
    fn svglite_fixture_converts_and_is_tagged() {
        let done = converted(SVGLITE);

        // Panel box inserted first, values taken from the canvas box.
        assert!(done.svg.contains(r#"data-panel-box="main""#));
        assert!(done.svg.contains(r#"width="288.00""#));
        assert!(done.svg.contains(r#"height="216.00""#));

        // Geometry group uses xy; text/circle are wrapped in place with position.
        assert!(done.svg.contains(r#"<g data-scale="xy">"#));
        assert!(done
            .svg
            .contains(r#"<g data-scale="position"><circle cx="60.82" cy="135.96""#));
        assert!(done
            .svg
            .contains(r#"<g data-scale="position"><text x="147.29" y="18.33""#));

        // Exactly one panel box, and text/circle counts.
        assert_eq!(done.report.texts, 2);
        assert_eq!(done.report.circles, 2);
        assert_eq!(done.report.panel_box, "inserted");

        // The clip rect lives in <defs> and is NOT given its own tag.
        assert!(done.svg.contains(r#"<defs>"#));
        assert!(!done.svg.contains(r#"<g data-scale="position"><rect"#));

        // The 100% background rect was resolved to absolute canvas values.
        assert!(!done.svg.contains("100%"));

        // svglite's rotate idiom was normalized to the composer's form.
        assert!(done
            .svg
            .contains(r#"transform="rotate(-90, 12.720, 143.530)""#));
        assert!(done.svg.contains(r#"x="12.720" y="143.530""#));

        // The `<style>` CSS block is still reported (the composer does not parse
        // CSS). The fixture's inline `font-size`/`stroke-width` lengths are now
        // rescaled, so they are no longer flagged.
        assert!(done.warnings.iter().any(|w| w.contains("<style>")));
        assert!(!done
            .warnings
            .iter()
            .any(|w| w.contains("will NOT be rescaled")));

        // The output re-passes the self-check.
        let reparsed = parse(&done.svg).unwrap();
        self_check(&reparsed).unwrap();
    }

    #[test]
    fn already_converted_input_is_detected() {
        assert!(matches!(
            convert(SVGLITE, false)
                .and_then(|_| convert(&converted(SVGLITE).svg, false))
                .unwrap(),
            Conversion::AlreadyConverted { .. }
        ));
    }

    #[test]
    fn half_converted_input_is_an_error_without_force() {
        let half = r#"<svg width='288' height='216' viewBox='0 0 288 216' xmlns='http://www.w3.org/2000/svg'>
<rect data-panel-box='main' x='0' y='0' width='288' height='216' visibility='hidden' pointer-events='none'/>
<circle cx='10' cy='10' r='1'/></svg>"#;
        assert!(matches!(
            convert(half, false),
            Err(ConvertError::Incomplete(_))
        ));
        // `--force` completes it.
        assert!(matches!(
            convert(half, true).unwrap(),
            Conversion::Converted(_)
        ));
    }

    #[test]
    fn force_conversion_is_stable() {
        let once = converted(SVGLITE).svg;
        let twice = convert_with(&once, true).svg;
        assert_eq!(once, twice);
    }

    #[test]
    fn converted_looking_input_is_still_validated() {
        // A panel box *and* a data-scale group, but a node the composer cannot
        // rewrite: validation must not be short-circuited by the idempotency
        // check, so this is Incompatible (not AlreadyConverted).
        let source = r#"<svg width='288' height='216' viewBox='0 0 288 216' xmlns='http://www.w3.org/2000/svg'>
<rect data-panel-box='main' x='0' y='0' width='288' height='216' visibility='hidden' pointer-events='none'/>
<g data-scale='xy'><symbol id='s'><rect x='0' y='0' width='10' height='10'/></symbol></g></svg>"#;
        let findings = incompatible(source);
        assert!(findings.iter().any(|f| f.tag == "symbol"));
    }

    #[test]
    fn panel_box_does_not_hide_a_bad_canvas() {
        // A panel box + data-scale that "looks converted" but whose canvas
        // (no viewBox, `pt` units) is not a pure number must still be rejected.
        let source = r#"<svg width='100pt' height='80pt' xmlns='http://www.w3.org/2000/svg'>
<rect data-panel-box='main' x='0' y='0' width='100' height='80' visibility='hidden' pointer-events='none'/>
<g data-scale='xy'><circle cx='1' cy='1' r='1'/></g></svg>"#;
        let findings = incompatible(source);
        assert!(
            findings[0].reason.contains("not a pure number"),
            "{:?}",
            findings[0]
        );
    }
}
