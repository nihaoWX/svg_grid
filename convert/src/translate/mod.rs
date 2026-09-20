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
//!    exact, curves are flattened to within `DEFAULT_TOLERANCE` source units. A
//!    fill-only path becomes a *single* `<polygon>` (all subpaths concatenated
//!    and explicitly closed, so a glyph's holes survive); a stroked-only path
//!    stays one `<polyline>` per subpath; a path that is both filled and stroked
//!    is split into a fill-only polygon and one stroke-only shape per subpath
//!    when it has several subpaths. The same module rectifies `<clipPath>` bodies
//!    (`<path>` → `<rect>`).
//! 3. **`<use>`/`<defs>` expansion** (`uses`) — each `<use>` becomes a `<g
//!    translate>` wrapper around a copy of the referenced element; unreferenced
//!    `<defs>` content is dropped (only `<clipPath>`/`<style>` survive).
//! 4. **`<image>` baking** (`image`) — a pure translation and any axis-aligned
//!    scale (uniform or not, optionally with a flip) are baked into the box
//!    `x`/`y`/`width`/`height`; a negative scale decodes the embedded PNG, flips
//!    it and re-encodes. Anything with rotation or skew is rejected (fail
//!    closed).
//!
//! Text transforms are left for the existing `normalize_transforms` step; a
//! `<text>` under a pure-translation ancestor transform is baked (the
//! translation is added to its `x`/`y`, its `<tspan>`s and any leaf rotate
//! centre), while a `<text>` under a scaling/rotating ancestor transform is
//! rejected.
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
    // Rectify `<clipPath>` bodies (poppler writes them as `<path>`) before the
    // affine baking runs, so a referencing transform can still move the boxes.
    shapes::rectify_clip_paths(root)?;
    // Normalize svglite's text transforms *before* baking the ancestor
    // translations. Baking first would write x/y onto a
    // `translate(...) rotate(...)` text, after which `normalize_transforms` (which
    // only rewrites a text without x/y) no longer recognises it and the transform
    // survives to fail the self-check. Doing it in this order is exact because
    // `T(e,f) · R(a, cx, cy) == R(a, cx+e, cy+f) · T(e,f)`:
    // normalizing turns the svglite form into `x/y + rotate(a, x, y)`, and baking
    // then adds the ancestor translation to both the x/y and the rotation centre.
    crate::transform::normalize_transforms(root);

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
    use crate::test_support::{converted, incompatible, SVGLITE};

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
    fn non_uniform_axis_aligned_image_scale_is_baked() {
        // A 1xN legend colour band placed with a non-uniform matrix (the poppler
        // `<use>` of an `<image>`). An axis-aligned non-uniform scale only
        // stretches the box, so it must be baked, not rejected.
        let source = r#"<svg width='1000' height='400' viewBox='0 0 1000 400' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
<image x='0' y='0' width='8' height='4' transform='matrix(17.28,0,0,0.288,493.77,149.48)' xlink:href='data:image/png;base64,iVBORw0KGgo='/></svg>"#;
        let done = converted(source);
        assert!(!done.svg.contains("transform="), "{}", done.svg);
        assert!(
            done.svg.contains(r#"preserveAspectRatio="none""#),
            "{}",
            done.svg
        );

        let transform =
            crate::affine::parse_transform("matrix(17.28,0,0,0.288,493.77,149.48)").unwrap();
        let (x0, y0) = transform.apply(0.0, 0.0);
        let (x1, y1) = transform.apply(8.0, 4.0);
        let expected = format!(
            r#"x="{}" y="{}" width="{}" height="{}""#,
            crate::transform::fmt_num(x0.min(x1)),
            crate::transform::fmt_num(y0.min(y1)),
            crate::transform::fmt_num((x1 - x0).abs()),
            crate::transform::fmt_num((y1 - y0).abs()),
        );
        assert!(
            done.svg.contains(&expected),
            "expected {expected} in {}",
            done.svg
        );
    }

    #[test]
    fn text_under_a_translated_group_is_baked() {
        // A pure ancestor translation is baked into the text's x/y *and* its leaf
        // rotate centre (T(e,f)·R(a,cx,cy) == R(a,cx+e,cy+f)·T(e,f)).
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='translate(5,5)'><text x='1' y='2' transform='rotate(-30, 1, 2)'>a</text></g></svg>"#,
        );
        assert!(!done.svg.contains("translate(5,5)"), "{}", done.svg);
        assert!(done.svg.contains(r#"x="6" y="7""#), "{}", done.svg);
        assert!(done.svg.contains("rotate(-30, 6, 7)"), "{}", done.svg);
    }

    #[test]
    fn translate_rotate_text_under_a_translated_group_is_baked() {
        // svglite's `translate(x,y) rotate(a)` text under a translated ancestor
        // group. `normalize_transforms` must run *before* the ancestor bake:
        // baking first writes x/y, after which normalization (which only rewrites
        // a text without x/y) no longer fires and the raw transform survives to
        // fail the self-check. With the correct order the ancestor translation
        // (5,5) is added to the normalized x/y (12,34) and rotate centre exactly.
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='translate(5,5)'><text transform='translate(12,34) rotate(-90)'>y</text></g></svg>"#,
        );
        assert!(!done.svg.contains("translate("), "{}", done.svg);
        assert!(done.svg.contains(r#"x="17" y="39""#), "{}", done.svg);
        assert!(done.svg.contains("rotate(-90, 17, 39)"), "{}", done.svg);
    }

    #[test]
    fn svglite_translate_rotate_text_is_still_normalized() {
        // Reordering must not change the plain svglite idiom (a translate-rotate
        // text under an untransformed ancestor): it is still normalized to
        // `x/y + rotate(a, x, y)`.
        let done = converted(SVGLITE);
        assert!(
            done.svg.contains(r#"x="12.720" y="143.530""#),
            "{}",
            done.svg
        );
        assert!(
            done.svg
                .contains(r#"transform="rotate(-90, 12.720, 143.530)""#),
            "{}",
            done.svg
        );
    }

    #[test]
    fn text_under_a_scaling_group_is_still_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='scale(2)'><text x='1' y='2'>a</text></g></svg>"#,
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
