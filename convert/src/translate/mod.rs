//! Matplotlib-dialect translation stage.
//!
//! Runs *after* the fail-closed scan and *before* the tag protocol is applied.
//! It rewrites the constructs the composer cannot handle into ones it can:
//!
//! 1. **Affine baking** (`bake`) — every `<g transform>` (and any transform on a
//!    leaf) is accumulated down the tree and baked into the leaf coordinate
//!    attributes; the output contains no `<g transform>`. A `<clipPath>` that is
//!    referenced from a transformed context is transformed by the *same* matrix
//!    so the clip box stays in the coordinate system of the geometry it clips.
//! 2. **`<path>` → `<polyline>`/`<polygon>`** (`shapes`) — straight commands are
//!    exact, curves are flattened to within `DEFAULT_TOLERANCE` source units.
//! 3. **`<use>`/`<defs>` expansion** (`uses`) — each `<use>` becomes a `<g
//!    translate>` wrapper around a copy of the referenced element; unreferenced
//!    `<defs>` content is dropped (only `<clipPath>`/`<style>` survive).
//! 4. **`<image>` baking** (`image`) — pure translation and axis-aligned flips
//!    are baked by decoding the embedded PNG, flipping it and re-encoding;
//!    anything with rotation, skew or non-uniform scale is rejected (fail
//!    closed).
//!
//! Text transforms are left for the existing `normalize_transforms` step; a
//! `<text>` under a non-identity ancestor transform is rejected.
//!
//! The stage is split by responsibility: this module is the coordinator plus the
//! small shared helpers; `bake`, `shapes`, `uses`, `image` and `style` hold the
//! individual rewrites.

mod bake;
mod image;
mod shapes;
pub(crate) mod style;
mod uses;

use std::collections::HashMap;

use crate::affine::Affine;
use crate::dom::{Element, Node};
use crate::error::ConvertError;

pub(crate) fn translate_dialect(root: &mut Element) -> Result<(), ConvertError> {
    let mut ids = HashMap::new();
    uses::collect_ids(root, &mut ids);
    uses::expand_uses(&mut root.children, &ids, 0)?;
    uses::prune_defs(root);

    let mut clip_refs: Vec<(String, Affine)> = Vec::new();
    bake::bake_children(&mut root.children, Affine::IDENTITY, &mut clip_refs)?;
    bake::apply_clip_refs(root, &clip_refs)?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// small helpers (shared by the submodules)
// --------------------------------------------------------------------------- //

fn url_id(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
}

fn number(element: &Element, attr: &str) -> Result<f32, ConvertError> {
    element
        .attr(attr)
        .unwrap_or("0")
        .trim()
        .parse::<f32>()
        .map_err(|_| {
            ConvertError::Translate(format!(
                "attribute {attr}={:?} on <{}> is not a plain number",
                element.attr(attr).unwrap_or("0"),
                element.name
            ))
        })
}

fn find_by_id<'a>(element: &'a mut Element, id: &str) -> Option<&'a mut Element> {
    if element.attr("id") == Some(id) {
        return Some(element);
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            if let Some(found) = find_by_id(child, id) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::options::Conversion;
    use crate::pipeline::convert;
    use crate::test_support::{converted, incompatible};

    #[test]
    fn use_is_expanded_and_defs_pruned() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><path id='m1' d='M 0 0 L 0 4' style='stroke: #000000; stroke-width: 0.8'/></defs>
<use href='#m1' x='10' y='20' style='fill: none'/></svg>"#,
        );
        assert!(!done.svg.contains("<use"));
        assert!(!done.svg.contains("id=\"m1\""));
        assert!(done.svg.contains("<polyline"), "{}", done.svg);
        // The use's x/y was baked into the polyline points.
        assert!(done.svg.contains("10,20"), "{}", done.svg);
    }

    #[test]
    fn clip_rect_follows_a_transformed_reference() {
        // The clip rect is referenced from inside a translated group; baking the
        // content but not the clip box would clip in the wrong place.
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><clipPath id='c'><rect x='0' y='0' width='10' height='10'/></clipPath></defs>
<g transform='translate(5,7)'><path d='M 0 0 L 20 0' clip-path='url(#c)'/></g></svg>"#,
        );
        assert!(
            done.svg
                .contains(r#"<rect x="5" y="7" width="10" height="10"/>"#),
            "clip rect must move with its content: {}",
            done.svg
        );
    }

    #[test]
    fn image_flip_is_baked_into_pixels() {
        let done = converted(IMAGE_FIXTURE);
        assert!(!done.svg.contains("transform="), "{}", done.svg);
        assert!(done.svg.contains(r#"preserveAspectRatio="none""#));
        // scale(1 -1) translate(0 -4) maps y=-4..0 to 8..4, so the box is y=4..8.
        assert!(
            done.svg.contains(r#"x="0" y="4" width="8" height="4""#),
            "{}",
            done.svg
        );
        // A pixel-level flip must have happened: the embedded PNG changed.
        assert!(
            !done.svg.contains(SRC_B64),
            "the bitmap must be re-encoded flipped"
        );
    }

    #[test]
    fn rotated_image_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
<image x='0' y='0' width='8' height='4' transform='rotate(30)' xlink:href='data:image/png;base64,AA=='/></svg>"#,
        );
        assert!(findings.iter().any(|f| f.tag == "image"), "{findings:?}");
    }

    #[test]
    fn translation_only_image_keeps_its_bitmap() {
        let source = IMAGE_FIXTURE.replace("scale(1 -1) translate(0 -4)", "translate(3,5)");
        let done = converted(&source);
        assert!(
            done.svg.contains(SRC_B64),
            "no flip should re-encode the bitmap"
        );
        assert!(
            done.svg.contains(r#"x="3" y="1" width="8" height="4""#),
            "{}",
            done.svg
        );
    }

    /// 8x4 RGB PNG with a red top row and a blue bottom row (a clear vertical
    /// pattern, so a baked flip is observable).
    const SRC_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAECAIAAAA8r+mnAAAAFUlEQVR4nGP4z8CAFeEQ/o9PAocMADp+H+E2jTGuAAAAAElFTkSuQmCC";

    const IMAGE_FIXTURE: &str = concat!(
        r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>"#,
        r#"<image x='0' y='-4' width='8' height='4' transform='scale(1 -1) translate(0 -4)' xlink:href='data:image/png;base64,"#,
        "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAECAIAAAA8r+mnAAAAFUlEQVR4nGP4z8CAFeEQ/o9PAocMADp+H+E2jTGuAAAAAElFTkSuQmCC",
        r#"'/></svg>"#,
    );

    #[test]
    fn text_under_a_transform_group_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='translate(5,5)'><text x='1' y='2'>a</text></g></svg>"#,
        );
        assert!(findings.iter().any(|f| f.tag == "text"), "{findings:?}");
    }

    #[test]
    fn translate_rotate_text_is_still_accepted() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text transform='translate(12,34) rotate(-90)'>y</text></svg>"#;
        assert!(matches!(
            convert(source, false).unwrap(),
            Conversion::Converted(_)
        ));
        let done = converted(source);
        assert!(done.svg.contains(r#"rotate(-90, 12, 34)"#), "{}", done.svg);
    }
}
