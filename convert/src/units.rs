//! Unit-suffix adaptation at the translation boundary.
//!
//! RDKit's `MolDraw2DSVG` emits unit-bearing lengths: the root is
//! `width='300px' height='200px'` and every stroke carries `stroke-width:2.0px`.
//! A `px` length is *exactly* a user unit when the viewport and the user
//! coordinate system map 1:1 — which is the case RDKit emits, because the root
//! size (`300px` × `200px`) matches `viewBox='0 0 300 200'`. The composer only
//! ever parses plain numbers, so the fix belongs here, in the adapter: the `px`
//! suffix is dropped and the number is kept verbatim (no unit *conversion* is
//! done — at 1:1 the number already is the user unit).
//!
//! This is deliberately *not* a unit converter. When 1:1 cannot be proven — a
//! `%`/`pt` root, a root size that disagrees with the `viewBox`, or a missing
//! numeric root size — nothing is stripped and the existing fail-closed scan
//! reports the unit-bearing length as-is. Silently treating a non-1:1 viewport as
//! 1:1 would rescale the whole panel, which is exactly the silent failure this
//! tool exists to prevent.

use crate::dom::{Element, Node};

/// Length attributes the composer may rewrite. `font-size` and `textLength` are
/// deliberately absent: the composer never rewrites them (`textLength` is
/// rescaled by the kernel's `normalize` step), so this adapter must leave their
/// `px` alone rather than silently changing behavior outside this fix.
const LENGTH_ATTRS: [&str; 14] = [
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "width",
    "height",
    "stroke-width",
];

/// Length declarations only ever seen in an inline `style="..."` (never as a
/// presentation attribute in the dialects we accept).
const LENGTH_STYLE_ONLY: [&str; 2] = ["stroke-dasharray", "stroke-dashoffset"];

/// The plain-number text and value of a length that is either a bare number or a
/// `px` length. Any other unit (`pt`, `%`, …) yields `None`.
///
/// Shared with `geom::canvas_box`, which accepts a `px` root size when there is
/// no `viewBox` (there the viewport *is* the user space, so a `px` value is a
/// plain number by definition).
pub(crate) fn plain_number(value: &str) -> Option<(&str, f32)> {
    let value = value.trim();
    let number = value.strip_suffix("px").unwrap_or(value).trim();
    number.parse::<f32>().ok().map(|value| (number, value))
}

/// True only when the root viewport and the user coordinate system provably map
/// 1:1, so a `px` length's number *is* its user unit.
///
/// With a `viewBox`, the numeric root `width`/`height` (a bare number or a `px`
/// length) must equal the `viewBox` extent on each axis. Without a `viewBox`, the
/// SVG viewport itself is the user space (1 user unit = 1 px), so a numeric root
/// size is 1:1 by definition. Anything else — a `%`/`pt` root, a size that
/// disagrees with the `viewBox`, an absent size, a malformed `viewBox` — is not
/// provably 1:1 → `false`.
///
/// Callers must evaluate this on the *original* root, before `resolve_percentages`
/// rewrites a `100%` width into its absolute value (which would otherwise look
/// like a 1:1 viewport).
pub(crate) fn viewport_allows_px(root: &Element) -> bool {
    let width = root.attr("width").and_then(plain_number);
    let height = root.attr("height").and_then(plain_number);
    match root.attr("viewBox") {
        Some(view_box) => match view_box_extent(view_box) {
            Some((view_box_width, view_box_height)) => matches!(
                (width, height),
                (Some((_, width)), Some((_, height)))
                    if width == view_box_width && height == view_box_height
            ),
            // A malformed viewBox is already rejected by `canvas_box`; be
            // conservative so a bug there never becomes a silent rescale.
            None => false,
        },
        None => width.is_some() && height.is_some(),
    }
}

/// The `(width, height)` extent of a well-formed four-number `viewBox`.
fn view_box_extent(view_box: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = view_box.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }
    Some((parts[2].parse().ok()?, parts[3].parse().ok()?))
}

/// Strip the `px` suffix from every length the composer may rewrite, in the whole
/// tree. Only called when [`viewport_allows_px`] holds, so the number is the user
/// unit and is kept verbatim.
pub(crate) fn strip_px_lengths(root: &mut Element) {
    walk(root);
}

fn walk(element: &mut Element) {
    for (name, value) in element.attrs.iter_mut() {
        if !LENGTH_ATTRS.contains(&name.as_str()) {
            continue;
        }
        if let Some(number) = strip_px(value) {
            *value = number;
        }
    }
    if let Some(style) = element.attr("style").map(str::to_string) {
        if let Some(rewritten) = strip_style_px(&style) {
            element.set_attr("style", rewritten);
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            walk(child);
        }
    }
}

/// `Some(number)` when `value` is a `px` length whose numeric part parses: the
/// suffix alone is dropped, the number is returned verbatim.
fn strip_px(value: &str) -> Option<String> {
    let value = value.trim();
    let number = value.strip_suffix("px")?.trim();
    number.parse::<f32>().ok()?;
    Some(number.to_string())
}

/// Strip `px` from the length declarations of an inline `style`, preserving the
/// rest of the declaration list (order, spacing, unrelated declarations) exactly.
fn strip_style_px(style: &str) -> Option<String> {
    let mut out = String::with_capacity(style.len());
    let mut changed = false;
    let mut rest = style;
    loop {
        let (declaration, remainder) = match rest.split_once(';') {
            Some((declaration, remainder)) => (declaration, Some(remainder)),
            None => (rest, None),
        };
        let rewritten = declaration.split_once(':').and_then(|(name, value)| {
            if !is_length_name(name.trim()) {
                return None;
            }
            let value = strip_px_tokens(value)?;
            Some(format!("{name}:{value}"))
        });
        match rewritten {
            Some(rewritten) => {
                out.push_str(&rewritten);
                changed = true;
            }
            None => out.push_str(declaration),
        }
        match remainder {
            Some(remainder) => {
                out.push(';');
                rest = remainder;
            }
            None => break,
        }
    }
    if changed {
        Some(out)
    } else {
        None
    }
}

/// True for a `style` declaration name this adapter may rewrite: the geometry /
/// line lengths plus the stroke dash lengths.
fn is_length_name(name: &str) -> bool {
    LENGTH_ATTRS.contains(&name) || LENGTH_STYLE_ONLY.contains(&name)
}

/// Strip `px` from every whitespace/comma-separated token of a length list (e.g.
/// `stroke-dasharray: 5px, 3px`), keeping the separators untouched.
fn strip_px_tokens(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut changed = false;
    let mut rest = value;
    while !rest.is_empty() {
        let separator_len = rest
            .find(|ch: char| !(ch.is_whitespace() || ch == ','))
            .unwrap_or(rest.len());
        out.push_str(&rest[..separator_len]);
        rest = &rest[separator_len..];
        if rest.is_empty() {
            break;
        }
        let token_len = rest
            .find(|ch: char| ch.is_whitespace() || ch == ',')
            .unwrap_or(rest.len());
        let token = &rest[..token_len];
        match strip_px(token) {
            Some(number) => {
                out.push_str(&number);
                changed = true;
            }
            None => out.push_str(token),
        }
        rest = &rest[token_len..];
    }
    if changed {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::options::Conversion;
    use crate::pipeline::convert;
    use crate::test_support::{converted, SVGLITE};

    /// The RDKit shape: a `px` root that matches the `viewBox`, and a stroked
    /// `<path>` whose line width is a `px` length inside `style`.
    const RDKIT: &str = r#"<svg width='300px' height='200px' viewBox='0 0 300 200' xmlns='http://www.w3.org/2000/svg'>
<path d='M 10 10 L 100 100' style='fill:none;stroke:#000000;stroke-width:2.0px'/></svg>"#;

    /// The `px` length must not be silently converted away: reject any outcome
    /// other than a plain, successful conversion. (The failure surfaces either as
    /// an `Incompatible` finding, when a geometry *attribute* carries the unit, or
    /// as the post-conversion `SelfCheck` error, when the unit is in `style` and
    /// only reaches an attribute during translation.)
    fn assert_rejected(source: &str) {
        match convert(source, false) {
            Ok(Conversion::Converted(_)) | Ok(Conversion::AlreadyConverted { .. }) => {
                panic!("the unit-bearing length must not convert silently: {source}")
            }
            Ok(Conversion::Incompatible(_)) | Err(_) => {}
        }
    }

    #[test]
    fn rdkit_px_lengths_are_stripped_and_converted() {
        let done = converted(RDKIT);
        // The number is kept verbatim; only the unit suffix is dropped.
        assert!(done.svg.contains(r#"stroke-width="2.0""#), "{}", done.svg);
        assert!(!done.svg.contains("px"), "no unit may remain: {}", done.svg);
        // `fill:none` ⇒ a stroke-only shape, emitted as a `<polyline>`.
        assert!(done.svg.contains("<polyline"), "{}", done.svg);
        assert!(!done.svg.contains("<path"), "{}", done.svg);
    }

    #[test]
    fn geometry_px_attributes_are_stripped_verbatim() {
        // A `px` root that matches the `viewBox`, plus `px` geometry attributes:
        // the suffix is dropped, the numbers are kept.
        let done = converted(
            r#"<svg width='100px' height='80px' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<rect x='1px' y='2px' width='10px' height='20px'/></svg>"#,
        );
        assert!(
            done.svg.contains(r#"x="1" y="2" width="10" height="20""#),
            "{}",
            done.svg
        );
        assert!(!done.svg.contains("px"), "{}", done.svg);
        assert_eq!(done.report.canvas, "0 0 100 80");
    }

    #[test]
    fn style_dash_lengths_lose_their_unit_too() {
        // `stroke-dasharray` is a length *list*; each token loses its unit and the
        // separators are preserved.
        let done = converted(
            r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<line x1='0' y1='0' x2='10' y2='10' stroke='#000000' stroke-width='1'
      style='stroke-dasharray: 5px, 3px; stroke-dashoffset: 2px'/></svg>"#,
        );
        assert!(!done.svg.contains("px"), "{}", done.svg);
        assert!(done.svg.contains("stroke-dasharray: 5, 3"), "{}", done.svg);
        assert!(done.svg.contains("stroke-dashoffset: 2"), "{}", done.svg);
    }

    #[test]
    fn non_one_to_one_viewport_still_fails_closed() {
        // `width='100%'` cannot be proven 1:1 with the `viewBox`, and this tool
        // does not convert units: the `px` length must stay and be rejected.
        assert_rejected(
            r#"<svg width='100%' height='200' viewBox='0 0 300 200' xmlns='http://www.w3.org/2000/svg'>
<path d='M 10 10 L 100 100' style='fill:none;stroke:#000000;stroke-width:2.0px'/></svg>"#,
        );
        // A `pt` viewport is equally not 1:1, on an attribute this time.
        assert_rejected(
            r#"<svg width='288pt' height='216pt' viewBox='0 0 288 216' xmlns='http://www.w3.org/2000/svg'>
<line x1='0' y1='0' x2='10' y2='10' stroke='#000000' stroke-width='2.0px'/></svg>"#,
        );
        // A root size that disagrees with the `viewBox` is not 1:1 either.
        assert_rejected(
            r#"<svg width='288px' height='200px' viewBox='0 0 300 200' xmlns='http://www.w3.org/2000/svg'>
<line x1='0' y1='0' x2='10' y2='10' stroke='#000000' stroke-width='2.0px'/></svg>"#,
        );
    }

    #[test]
    fn font_size_and_text_length_px_are_left_untouched() {
        // The svglite dialect (a `pt` viewport) keeps its `font-size: 8.80px`.
        let done = converted(SVGLITE);
        assert!(done.svg.contains("font-size: 8.80px"), "{}", done.svg);

        // The stricter check is a *1:1* viewport: only `stroke-width` may be
        // stripped, `font-size`/`textLength` must survive verbatim.
        let done = converted(
            r#"<svg width='100' height='100' viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
<text x='1' y='2' textLength='40px' style='font-size: 8px'>a</text></svg>"#,
        );
        assert!(done.svg.contains("font-size: 8px"), "{}", done.svg);
        assert!(done.svg.contains(r#"textLength="40px""#), "{}", done.svg);
    }
}
