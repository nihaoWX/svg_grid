//! Transform classification and normalization.

use crate::dom::{Element, Node};

enum TransformKind {
    /// `rotate(a, x, y)` — rewritten by the composer.
    Rotate3,
    /// svglite's `translate(x,y) rotate(a)` on a text with no x/y — we rewrite
    /// it into `x/y + rotate(a, x, y)` which is mathematically identical.
    TranslateRotate { angle: f32, x: f32, y: f32 },
    /// Anything else (`scale`, `matrix`, bare `translate`, 1-arg `rotate`, …).
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
        TransformKind::Unsupported => false,
    }
}

/// Rewrite svglite's `translate(x,y) rotate(a)` into the composer's supported
/// `x`/`y` + `rotate(a, x, y)` form (identical rendering, see module docs).
pub(crate) fn normalize_transforms(element: &mut Element) {
    if let Some(transform) = element.attr("transform").map(str::to_string) {
        if let TransformKind::TranslateRotate { angle, x, y } = classify_transform(&transform) {
            if element.name == "text" && element.attr("x").is_none() && element.attr("y").is_none()
            {
                element.set_attr("x", fmt_num(x));
                element.set_attr("y", fmt_num(y));
                element.set_attr(
                    "transform",
                    format!("rotate({}, {}, {})", fmt_num(angle), fmt_num(x), fmt_num(y)),
                );
            }
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            normalize_transforms(child);
        }
    }
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
}
