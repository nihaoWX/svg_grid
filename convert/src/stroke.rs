//! Stroke-width materialization.
//!
//! The composer only scales a `stroke-width` that is *declared* on an element
//! (attribute or `style="..."`); a shape that relies on the SVG default width of
//! `1` keeps that default after the panel is scaled up, so its outline collapses
//! to a hairline (the reported polyline/polygon bug). This pass walks the tagged
//! tree and gives every stroked primitive an explicit, scale-able `stroke-width`,
//! taking the value from its own declaration or from the nearest stroking ancestor.
//!
//! It deliberately never *adds* a stroke: an unstroked shape (SVG's default
//! `stroke="none"`, or an explicit `none` / `transparent`) is left untouched.

use crate::dom::{Element, Node};
use crate::translate::style::style_value;

/// Renderable primitives that can carry a stroke. `<text>` is handled by the
/// `position` policy instead and is intentionally not "materialized" here.
const STROKED_SHAPES: [&str; 6] = ["circle", "rect", "line", "polyline", "polygon", "ellipse"];

pub(crate) fn materialize_stroke_width(root: &mut Element) {
    walk(root, None, None);
}

fn walk(element: &mut Element, inherited_stroke: Option<String>, inherited_width: Option<String>) {
    let own_stroke = own_property(element, "stroke");
    let own_width = own_property(element, "stroke-width");
    let stroke = own_stroke.or_else(|| inherited_stroke.clone());
    let width = own_width.clone().or_else(|| inherited_width.clone());

    if STROKED_SHAPES.contains(&element.name.as_str())
        && is_stroked(stroke.as_deref())
        && own_width.is_none()
    {
        // The SVG default stroke width is `1`; the inherited value (if any) is
        // written down verbatim so the composer rescales *this* element.
        element.set_attr(
            "stroke-width",
            width.clone().unwrap_or_else(|| "1".to_string()),
        );
    }

    for child in &mut element.children {
        if let Node::Element(child) = child {
            walk(child, stroke.clone(), width.clone());
        }
    }
}

/// The element's own value for `name`, preferring the presentation attribute and
/// falling back to the inline `style="..."` declaration list.
fn own_property(element: &Element, name: &str) -> Option<String> {
    if let Some(value) = element.attr(name) {
        return Some(value.to_string());
    }
    style_value(element.attr("style")?, name)
}

/// SVG's initial `stroke` is `none`; an explicit `none`/`transparent` is not a
/// stroke. Shared with the `<path>` rewrite, which splits a filled *and* stroked
/// path so no shape paints both.
pub(crate) fn is_stroked(stroke: Option<&str>) -> bool {
    match stroke {
        Some(value) => {
            let value = value.trim();
            !value.is_empty()
                && !value.eq_ignore_ascii_case("none")
                && !value.eq_ignore_ascii_case("transparent")
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::converted;

    #[test]
    fn a_stroked_primitive_without_a_width_gets_the_default_written_down() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<path d='M 0 0 L 10 10 L 20 0' fill='none' stroke='#000000'/></svg>"#,
        );
        assert!(
            done.svg.contains(r#"stroke-width="1""#),
            "a stroked polyline must carry an explicit default width: {}",
            done.svg
        );
    }

    #[test]
    fn an_inherited_width_is_materialized_onto_the_stroked_primitive() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g style='stroke-width: 2.5'><line x1='0' y1='0' x2='10' y2='10' stroke='#000000'/></g></svg>"#,
        );
        assert!(
            done.svg.contains(r#"stroke-width="2.5""#),
            "the inherited width must be explicit on the primitive: {}",
            done.svg
        );
    }

    #[test]
    fn an_unstroked_primitive_is_not_given_a_stroke() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<rect x='1' y='1' width='10' height='10' fill='#336699'/>
<circle cx='5' cy='5' r='2' style='stroke: none; fill: #000000;'/></svg>"#,
        );
        assert!(
            !done.svg.contains("stroke-width"),
            "a fill-only primitive must not gain a stroke: {}",
            done.svg
        );
    }

    #[test]
    fn an_own_width_is_left_verbatim() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<line x1='0' y1='0' x2='10' y2='10' stroke='#000000' stroke-width='0.4'/></svg>"#,
        );
        assert!(done.svg.contains(r#"stroke-width="0.4""#), "{}", done.svg);
    }
}
