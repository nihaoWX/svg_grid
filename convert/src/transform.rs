//! Transform classification and normalization.

use crate::dom::{Element, Node};

enum TransformKind {
    /// `rotate(a, x, y)` — rewritten by the composer.
    Rotate3,
    /// svglite's `translate(x,y) rotate(a)` on a text with no x/y — we rewrite
    /// it into `x/y + rotate(a, x, y)` which is mathematically identical.
    TranslateRotate { angle: f32, x: f32, y: f32 },
    /// A bare `translate(tx[, ty])` — on a `<text>` we fold it into `x`/`y` and
    /// drop the transform. Kept distinct from [`TransformKind::TranslateRotate`]
    /// (which also carries a rotation).
    Translate { x: f32, y: f32 },
    /// Anything else (`scale`, `matrix`, 1-arg `rotate`, …).
    Unsupported,
}

fn classify_transform(value: &str) -> TransformKind {
    let value = value.trim();
    if let Some(inner) = value
        .strip_prefix("rotate(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return if split_numbers(inner).len() == 3 {
            TransformKind::Rotate3
        } else {
            TransformKind::Unsupported
        };
    }
    if let Some(index) = value.find("rotate(") {
        let prefix = value[..index].trim();
        let rest = &value[index..];
        let Some(inner) = rest
            .strip_prefix("rotate(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return TransformKind::Unsupported;
        };
        let angle = split_numbers(inner);
        if angle.len() != 1 {
            return TransformKind::Unsupported;
        }
        let Some(tinner) = prefix
            .strip_prefix("translate(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return TransformKind::Unsupported;
        };
        let translate = split_numbers(tinner);
        if translate.len() != 2 {
            return TransformKind::Unsupported;
        }
        return TransformKind::TranslateRotate {
            angle: angle[0],
            x: translate[0],
            y: translate[1],
        };
    }
    if let Some(inner) = value
        .strip_prefix("translate(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let translate = split_numbers(inner);
        return match translate.as_slice() {
            [x] => TransformKind::Translate { x: *x, y: 0.0 },
            [x, y] => TransformKind::Translate { x: *x, y: *y },
            _ => TransformKind::Unsupported,
        };
    }
    TransformKind::Unsupported
}

fn split_numbers(value: &str) -> Vec<f32> {
    value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
}

pub(crate) fn transform_supported(element: &Element) -> bool {
    let Some(transform) = element.attr("transform") else {
        return true;
    };
    match classify_transform(transform) {
        TransformKind::Rotate3 => true,
        // Only exact for a text anchored at its local origin, i.e. no x/y.
        TransformKind::TranslateRotate { .. } => {
            element.name == "text" && element.attr("x").is_none() && element.attr("y").is_none()
        }
        // A bare translation is folded into the text's x/y (with or without an
        // existing x/y), so it is always fine for a `<text>`.
        TransformKind::Translate { .. } => element.name == "text",
        TransformKind::Unsupported => false,
    }
}

/// Rewrite svglite's `translate(x,y) rotate(a)` into the composer's supported
/// `x`/`y` + `rotate(a, x, y)` form (identical rendering, see module docs), and a
/// bare `translate(tx, ty)` into an offset of the text's own `x`/`y` (there is
/// nothing to rotate, so the transform can simply be dropped).
pub(crate) fn normalize_transforms(element: &mut Element) {
    if let Some(transform) = element.attr("transform").map(str::to_string) {
        match classify_transform(&transform) {
            TransformKind::TranslateRotate { angle, x, y } => {
                if element.name == "text"
                    && element.attr("x").is_none()
                    && element.attr("y").is_none()
                {
                    element.set_attr("x", fmt_num(x));
                    element.set_attr("y", fmt_num(y));
                    element.set_attr(
                        "transform",
                        format!("rotate({}, {}, {})", fmt_num(angle), fmt_num(x), fmt_num(y)),
                    );
                }
            }
            TransformKind::Translate { x, y } => {
                if element.name == "text" {
                    let base_x = optional_number(element, "x");
                    let base_y = optional_number(element, "y");
                    element.set_attr("x", fmt_num(base_x + x));
                    element.set_attr("y", fmt_num(base_y + y));
                    element.remove_attr("transform");
                }
            }
            TransformKind::Rotate3 | TransformKind::Unsupported => {}
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            normalize_transforms(child);
        }
    }
}

/// Read an optional numeric attribute, defaulting to `0` when absent (SVG's
/// default for `x`/`y`).
fn optional_number(element: &Element, attr: &str) -> f32 {
    element
        .attr(attr)
        .map(|value| value.trim().parse::<f32>().unwrap_or(0.0))
        .unwrap_or(0.0)
}

/// Format a coordinate with the crate's canonical short form: integers drop the
/// fractional part, everything else keeps three decimals. Both the transform
/// normalizer and the percentage resolver must agree on this so the tagged
/// output stays byte-stable.
pub(crate) fn fmt_num(value: f32) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.3}")
    }
}

#[cfg(test)]
mod tests {
    use crate::options::Conversion;
    use crate::pipeline::convert;
    use crate::test_support::converted;

    #[test]
    fn rotate_with_center_is_allowed() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text x='5' y='40' transform='rotate(-45 10 10)'>Y</text>
</svg>"#;
        assert!(matches!(
            convert(source, false).unwrap(),
            Conversion::Converted(_)
        ));
    }

    #[test]
    fn bare_translate_transform_is_baked() {
        // A bare `<g transform>` used to be rejected; the translation stage now
        // bakes it into the leaf coordinates.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='translate(10,20)'><circle cx='1' cy='1' r='1'/></g>
</svg>"#;
        let Conversion::Converted(done) = convert(source, false).unwrap() else {
            panic!("expected Converted");
        };
        assert!(!done.svg.contains("transform"), "{}", done.svg);
        assert!(done.svg.contains(r#"cx="11" cy="21""#), "{}", done.svg);
    }

    #[test]
    fn bare_translate_on_text_is_folded_into_xy() {
        // A bare `translate(tx, ty)` on a `<text>` with an existing x/y is folded
        // into the x/y and the transform is dropped.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text x='10' y='20' transform='translate(5, 7)'>a</text></svg>"#;
        let done = converted(source);
        assert!(!done.svg.contains("transform"), "{}", done.svg);
        assert!(done.svg.contains(r#"x="15" y="27""#), "{}", done.svg);
    }

    #[test]
    fn bare_translate_on_text_without_xy_is_folded_into_xy() {
        // No x/y present: SVG would place the text at the (0,0) origin and then
        // translate it, which is exactly `x = tx`, `y = ty`.
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text transform='translate(5, 7)'>a</text></svg>"#;
        let done = converted(source);
        assert!(!done.svg.contains("transform"), "{}", done.svg);
        assert!(done.svg.contains(r#"x="5" y="7""#), "{}", done.svg);
    }
}
