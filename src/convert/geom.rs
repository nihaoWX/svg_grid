//! Canvas box resolution.
//!
//! Mirrors `geom::find_canvas_box`, but validates instead of falling back
//! silently / producing a non-numeric panel box.

use crate::convert::dom::Element;
use crate::convert::error::{svg_finding, Finding};
use crate::convert::units::plain_number;

pub(crate) struct CanvasBox {
    pub(crate) x: String,
    pub(crate) y: String,
    pub(crate) width: String,
    pub(crate) height: String,
    pub(crate) width_value: f32,
    pub(crate) height_value: f32,
}

impl CanvasBox {
    pub(crate) fn as_string(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.width, self.height)
    }
}

/// Resolve the source canvas box exactly like `geom::find_canvas_box` would,
/// but *fail closed* on anything the composer would silently mishandle:
///
/// * the root element must be `<svg>`;
/// * a `viewBox` attribute that is present must be four whitespace-separated
///   numbers (a comma-separated `viewBox` makes the composer fall back to
///   `width`/`height` and changes the geometry);
/// * with no usable `viewBox`, `width`/`height` must be plain numbers *or* `px`
///   lengths (the composer parses them with `parse_f32`, so `288.00pt` would
///   fail; a `px` length is a user unit by definition here, so its suffix is
///   dropped and the number is used);
/// * the resulting width/height must be positive.
pub(crate) fn canvas_box(root: &Element) -> Result<CanvasBox, Vec<Finding>> {
    if root.name != "svg" {
        return Err(vec![Finding {
            tag: root.name.clone(),
            ordinal: 1,
            path: root.name.clone(),
            reason: format!(
                "the root element must be <svg>, found <{}>; `svg_grid plot-grid` only consumes SVG documents",
                root.name
            ),
        }]);
    }

    // The root `width`/`height` are copied verbatim into the product even when a
    // `viewBox` decides the canvas, so a non-finite one is an illegal number in
    // the output no renderer may see. A unit-bearing size (`504pt`, `100%`) is
    // left to the branches below: it is not a number this gate can judge.
    for attr in ["width", "height"] {
        if let Some(value) = root.attr(attr) {
            if let Some((_, number)) = plain_number(value) {
                if !number.is_finite() {
                    return Err(vec![svg_finding(format!(
                        "root {attr}={value:?} is not a finite number; the composer copies it into \
                         the product verbatim, where `inf`/`NaN` is not a legal SVG length"
                    ))]);
                }
            }
        }
    }

    if let Some(view_box) = root.attr("viewBox") {
        let parts: Vec<&str> = view_box.split_whitespace().collect();
        let values: Option<Vec<f32>> = if parts.len() == 4 {
            parts.iter().map(|part| part.parse::<f32>().ok()).collect()
        } else {
            None
        };
        let Some(values) = values else {
            return Err(vec![svg_finding(format!(
                "viewBox={view_box:?} is present but is not four whitespace-separated numbers; \
                 the composer would silently fall back to the root width/height and change the \
                 geometry (e.g. a comma-separated `0,0,288,216` silently scales by 0.5)"
            ))]);
        };
        let width = values[2];
        let height = values[3];
        if !values.iter().all(|value| value.is_finite()) {
            return Err(vec![svg_finding(format!(
                "viewBox={view_box:?} contains a number that is not finite; the composer would \
                 write `inf`/`NaN` into the panel box, which no renderer accepts"
            ))]);
        }
        if width <= 0.0 || height <= 0.0 {
            return Err(vec![svg_finding(format!(
                "canvas from viewBox={view_box:?} has non-positive size ({width} x {height}); \
                 the composer requires positive width and height"
            ))]);
        }
        return Ok(CanvasBox {
            x: parts[0].to_string(),
            y: parts[1].to_string(),
            width: parts[2].to_string(),
            height: parts[3].to_string(),
            width_value: width,
            height_value: height,
        });
    }

    let width_raw = root.attr("width").unwrap_or("0").trim();
    let height_raw = root.attr("height").unwrap_or("0").trim();
    let (Some((width, width_value)), Some((height, height_value))) =
        (plain_number(width_raw), plain_number(height_raw))
    else {
        return Err(vec![svg_finding(format!(
            "canvas size is not a pure number (width={width_raw:?}, height={height_raw:?}) and there \
             is no usable viewBox; the composer parses width/height with parse_f32, so a unit suffix \
             such as `pt` would be rejected"
        ))]);
    };
    // A non-finite size was already rejected by the root `width`/`height` loop
    // above (it runs for every document, with or without a `viewBox`).
    if width_value <= 0.0 || height_value <= 0.0 {
        return Err(vec![svg_finding(format!(
            "canvas size must have positive width and height, got {width_value} x {height_value}"
        ))]);
    }
    Ok(CanvasBox {
        x: "0".to_string(),
        y: "0".to_string(),
        width: width.to_string(),
        height: height.to_string(),
        width_value,
        height_value,
    })
}

#[cfg(test)]
mod tests {
    use crate::convert::test_support::incompatible;

    #[test]
    fn non_svg_root_is_rejected() {
        let findings = incompatible("<html><body><p>not an svg</p></body></html>");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].path, "html");
        assert!(findings[0].reason.contains("must be <svg>"));
    }

    #[test]
    fn zero_sized_canvas_is_rejected() {
        let findings = incompatible(
            r#"<svg width='0' height='0' viewBox='0 0 0 0' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].reason.contains("positive"), "{:?}", findings[0]);
    }

    #[test]
    fn unit_canvas_without_viewbox_is_rejected() {
        let findings = incompatible(
            r#"<svg width='288.00pt' height='216.00pt' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].reason.contains("not a pure number"),
            "{:?}",
            findings[0]
        );
    }

    #[test]
    fn px_canvas_without_viewbox_is_a_plain_number() {
        // No `viewBox`: the SVG viewport *is* the user space (1 user unit = 1 px),
        // so `width='300px'` is a plain canvas of 300×200 — not a unit error.
        let done = crate::convert::test_support::converted(
            r#"<svg width='300px' height='200px' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(done.report.canvas, "0 0 300 200", "{}", done.svg);
        assert!(done.svg.contains(r#"width="300""#), "{}", done.svg);
        assert!(!done.svg.contains("px"), "{}", done.svg);
    }

    #[test]
    fn comma_separated_viewbox_is_rejected() {
        let findings = incompatible(
            r#"<svg width='288' height='216' viewBox='0,0,288,216' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].reason.contains("viewBox"), "{:?}", findings[0]);
    }

    /// R65: `1e999` parses to `inf`, so a finite check cannot live in `parse_f32`.
    /// The composer copies the root `width`/`height` into the product verbatim even
    /// when a `viewBox` decides the canvas, so a non-finite root size is fatal.
    #[test]
    fn non_finite_root_size_is_rejected() {
        let findings = incompatible(
            r#"<svg width='1e999' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].reason.contains("width") && findings[0].reason.contains("1e999"),
            "{:?}",
            findings[0]
        );
    }

    #[test]
    fn non_finite_viewbox_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 1e999 80' xmlns='http://www.w3.org/2000/svg'>
<circle cx='1' cy='1' r='1'/></svg>"#,
        );
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].reason.contains("viewBox") && findings[0].reason.contains("not finite"),
            "{:?}",
            findings[0]
        );
    }

    #[test]
    fn non_finite_canvas_without_viewbox_is_rejected() {
        for value in ["NaN", "-inf", "1e999"] {
            let source = format!(
                "<svg width='{value}' height='80' xmlns='http://www.w3.org/2000/svg'>\
<circle cx='1' cy='1' r='1'/></svg>"
            );
            let findings = incompatible(&source);
            assert_eq!(findings.len(), 1, "{value}: {findings:?}");
            assert!(
                findings[0].reason.contains("not a finite number")
                    || findings[0].reason.contains("not finite"),
                "{value}: {:?}",
                findings[0]
            );
        }
    }
}
