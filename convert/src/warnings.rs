//! Non-fatal warnings (style-based geometry the composer will not rescale).

use crate::dom::{Element, Node};

/// Presentational *geometry* properties the composer reads from attributes when
/// it rewrites an element's box.
///
/// Length properties the composer understands — `font-size`, `stroke-width`,
/// `stroke-dasharray`, `stroke-dashoffset` — are now rescaled from `style` too, so
/// they are no longer flagged. Coordinate/size geometry written in `style` is only
/// partially rewritten (e.g. `<rect>`/`<image>` boxes come from attributes), so it
/// is still worth a note.
const GEOMETRY_STYLE_PROPS: [&str; 5] = ["r", "width", "height", "x", "y"];

pub(crate) fn collect_warnings(root: &Element) -> Vec<String> {
    let mut has_style_block = false;
    let mut style_geometry = 0usize;
    walk_warnings(root, &mut has_style_block, &mut style_geometry);

    let mut warnings = Vec::new();
    if style_geometry > 0 {
        warnings.push(format!(
            "{style_geometry} element(s) put geometry in a CSS `style=\"...\"` attribute (e.g. `width`, \
             `r`); the composer rescales the length properties it understands (`font-size`, \
             `stroke-width`, `stroke-dasharray`) from `style`, but such geometry may still not be rescaled"
        ));
    }
    if has_style_block {
        warnings.push(
            "document contains a <style> CSS block; the composer does not parse CSS, so geometry/typography \
             declared there is not rescaled"
                .to_string(),
        );
    }
    warnings
}

fn walk_warnings(element: &Element, has_style_block: &mut bool, style_geometry: &mut usize) {
    if element.name == "style" {
        *has_style_block = true;
    }
    if let Some(style) = element.attr("style") {
        if GEOMETRY_STYLE_PROPS
            .iter()
            .any(|prop| style_declares(style, prop))
        {
            *style_geometry += 1;
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            walk_warnings(child, has_style_block, style_geometry);
        }
    }
}

/// True when a CSS declaration list (the value of a `style="..."` attribute)
/// contains the property `prop`. Matching on the declaration name (instead of a
/// raw substring) avoids false positives such as `x` matching `text-anchor`.
fn style_declares(style: &str, prop: &str) -> bool {
    style.split(';').any(|declaration| {
        declaration
            .split_once(':')
            .is_some_and(|(name, _)| name.trim() == prop)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::converted;

    #[test]
    fn style_warning_covers_more_properties() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<rect x='1' y='1' width='10' height='10' style='width: 5; font-size: 3px;'/></svg>"#,
        );
        assert!(
            done.warnings.iter().any(|w| w.contains("style")),
            "warnings: {:?}",
            done.warnings
        );
    }

    #[test]
    fn style_property_matching_is_not_a_bare_substring() {
        // `text-anchor` contains `x`; it must NOT be mistaken for the geometry
        // property `x`.
        assert!(!style_declares("text-anchor: middle;", "x"));
        assert!(style_declares("stroke-width: 0.77;", "stroke-width"));
        assert!(style_declares("fill: none; font-size: 8.8px", "font-size"));
    }
}
