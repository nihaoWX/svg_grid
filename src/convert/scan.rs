//! Fail-closed compatibility scan of the whole tree.
//!
//! This is the single validation gate: it runs *before* the translation stage
//! and reports every construct the converter cannot turn into something the
//! composer can place correctly. The matplotlib dialect is now translated (see
//! [`crate::convert::translate`]), so `<path>`, `<use>`, `<image>` and `<g transform>` are
//! accepted — but only when they can actually be baked.

use std::collections::HashSet;

use crate::convert::affine::{parse_transform, Affine};
use crate::convert::dom::{Element, Node};
use crate::convert::error::Finding;
use crate::convert::transform::transform_supported;

pub(crate) fn scan(root: &Element) -> Vec<Finding> {
    let mut ids = HashSet::new();
    collect_ids(root, &mut ids);

    let mut findings = Vec::new();
    let mut order = 0usize;
    walk_scan(
        root,
        "",
        &mut findings,
        &mut order,
        true,
        false,
        Affine::IDENTITY,
        &ids,
    );
    findings
}

fn collect_ids(element: &Element, ids: &mut HashSet<String>) {
    if let Some(id) = element.attr("id") {
        ids.insert(id.to_string());
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_ids(child, ids);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_scan(
    element: &Element,
    parent_path: &str,
    findings: &mut Vec<Finding>,
    order: &mut usize,
    is_root: bool,
    in_defs: bool,
    acc: Affine,
    ids: &HashSet<String>,
) {
    *order += 1;
    let ordinal = *order;
    let path = if parent_path.is_empty() {
        element.name.clone()
    } else {
        format!("{parent_path}/{}", element.name)
    };

    macro_rules! push {
        ($tag:expr, $reason:expr $(,)?) => {
            findings.push(Finding {
                tag: $tag.to_string(),
                ordinal,
                path: path.clone(),
                reason: $reason,
            })
        };
    }

    match element.name.as_str() {
        // `<symbol>` inside `<defs>` is never drawn where it sits: the prune
        // hoists the definitions it carries and drops the wrapper (R38). Only a
        // symbol in the drawable tree is placed by the composer without its own
        // coordinate system being rewritten.
        "symbol" if !in_defs => push!(
            "symbol",
            "<symbol> establishes its own nested coordinate system; the composer does not rewrite \
             it, so the content would be placed incorrectly"
                .to_string(),
        ),
        "use" => {
            let href = element
                .attr("xlink:href")
                .or_else(|| element.attr("href"))
                .unwrap_or("");
            match href.strip_prefix('#') {
                Some(id) if ids.contains(id) => {}
                _ => push!(
                    "use",
                    format!(
                        "<use> href {href:?} does not resolve to a defined element, so it cannot be \
                         expanded"
                    ),
                ),
            }
        }
        // `<foreignObject>` carries its own viewport and non-SVG content. The
        // renderer the composer targets (resvg through `d1render`) draws nothing for
        // it — measured: a 20x20 `<foreignObject>` with a black `<rect>` inside
        // contributes 0 ink, and the composer's own pass leaves its `x`/`y`/`width`/
        // `height` unscaled. Accepting it would drop the author's content without a
        // word, which is exactly the silence this gate exists to prevent (R64).
        "foreignObject" => push!(
            "foreignObject",
            "<foreignObject> holds foreign (usually HTML) content in its own viewport; the \
             renderer draws none of it, so the content would be silently lost"
                .to_string(),
        ),
        // `<switch>` draws exactly one child (the first whose `requiredFeatures`/
        // `systemLanguage`/... pass). The composer cannot evaluate those conditions
        // and rewrites every child as ordinary panel geometry, so which child ends up
        // drawn — and whether its numbers were scaled for that role — is decided by
        // the renderer, not by the converter (R64).
        "switch" => push!(
            "switch",
            "<switch> draws only the first child whose conditions the renderer supports; the \
             composer cannot evaluate them and would rewrite every child, so the output is not \
             guaranteed to be the child the browser draws"
                .to_string(),
        ),
        "svg" if !is_root => push!(
            "svg",
            "nested <svg> establishes its own viewport and coordinate system, which the composer \
             does not rewrite; only the root <svg> may appear"
                .to_string(),
        ),
        _ => {}
    }

    // Parse the element's own transform (a syntax error is fatal), then check
    // that the accumulated transform can actually be baked for this element.
    let local = match element.attr("transform") {
        Some(value) => match parse_transform(value) {
            Some(transform) => transform,
            None => {
                push!(
                    element.name.as_str(),
                    format!(
                        "unsupported transform {value:?}: only matrix/translate/scale/rotate/skewX/\
                         skewY are understood"
                    ),
                );
                Affine::IDENTITY
            }
        },
        None => Affine::IDENTITY,
    };
    let world = acc.mul(local);

    match element.name.as_str() {
        "text" => {
            if !transform_supported(element) {
                push!(
                    "text",
                    format!(
                        "unsupported transform {:?} on <text>: only `rotate(angle, x, y)` (plus \
                         svglite's `translate(x,y) rotate(angle)` on a text without x/y, and a bare \
                         `translate(x[, y])` which is normalized into x/y) is rewritten by the \
                         composer",
                        element.attr("transform").unwrap_or("")
                    ),
                );
            } else if !acc.is_translation() {
                push!(
                    "text",
                    format!(
                        "<text> sits under a non-identity ancestor transform ({acc:?}) with \
                         rotation or scale; only a pure translation can be baked into text, so the \
                         label would be placed incorrectly"
                    ),
                );
            }
        }
        "image" => {
            let bakeable = world
                .as_axis_scale_translate()
                .map(|(sx, sy, _, _)| sx.abs() > 1e-9 && sy.abs() > 1e-9)
                .unwrap_or(false);
            if !bakeable {
                push!(
                    "image",
                    format!(
                        "unsupported <image> transform {:?}: only an axis-aligned, non-degenerate \
                         scale (with an optional flip, e.g. `matrix(17.28,0,0,0.288,…)` or \
                         `scale(1 -1)`) or a pure translation can be baked into the bitmap",
                        element.attr("transform").unwrap_or("")
                    ),
                );
            }
        }
        "circle" if world.as_uniform_scale_translate().is_none() => {
            push!(
                "circle",
                format!(
                    "unsupported <circle> transform {world:?}: a circle under a rotation, skew or \
                     non-uniform scale becomes an ellipse, which cannot be baked"
                ),
            );
        }
        "rect" if world.as_axis_scale_translate().is_none() => {
            push!(
                "rect",
                format!(
                    "unsupported <rect> transform {world:?}: a `rect` under a rotation or skew is \
                     no longer axis-aligned"
                ),
            );
        }
        _ => {}
    }

    for bad in non_numeric_geometry(element) {
        push!(
            element.name.as_str(),
            format!(
                "geometry attribute {bad} is not a plain number, so the composer cannot rewrite it"
            ),
        );
    }

    let children_in_defs = in_defs || element.name == "defs";
    for child in &element.children {
        if let Node::Element(child) = child {
            walk_scan(
                child,
                &path,
                findings,
                order,
                false,
                children_in_defs,
                world,
                ids,
            );
        }
    }
}

/// Attributes the composer parses with `geom::parse_f32`. Anything present but
/// not a plain number (after `%` resolution) would make the composer bail out,
/// so we fail closed instead of emitting a broken file.
fn non_numeric_geometry(element: &Element) -> Vec<String> {
    let geometry: &[&str] = match element.name.as_str() {
        "circle" => &["cx", "cy", "r"],
        "ellipse" => &["cx", "cy", "rx", "ry"],
        "text" => &["x", "y"],
        "use" => &["x", "y", "width", "height"],
        "line" => &["x1", "y1", "x2", "y2"],
        "rect" => &["x", "y", "width", "height"],
        "image" => &["x", "y", "width", "height"],
        _ => &[],
    };

    let mut bad = Vec::new();
    for attr in geometry
        .iter()
        .copied()
        .chain(std::iter::once("stroke-width"))
    {
        if let Some(value) = element.attr(attr) {
            if value.trim().parse::<f32>().is_err() {
                bad.push(attr.to_string());
            }
        }
    }

    if matches!(element.name.as_str(), "polyline" | "polygon") {
        if let Some(points) = element.attr("points") {
            'outer: for pair in points.split_whitespace() {
                if let Some((x, y)) = pair.split_once(',') {
                    if x.trim().parse::<f32>().is_err() || y.trim().parse::<f32>().is_err() {
                        bad.push("points".to_string());
                        break 'outer;
                    }
                }
            }
        }
    }

    bad
}

#[cfg(test)]
mod tests {
    use crate::convert::options::Conversion;
    use crate::convert::pipeline::convert;
    use crate::convert::test_support::{converted, incompatible, MATPLOTLIB};

    #[test]
    fn matplotlib_fixture_is_now_supported() {
        let done = converted(MATPLOTLIB);
        assert!(!done.svg.contains("<path"));
        assert!(!done.svg.contains("<image transform"));
    }

    #[test]
    fn non_numeric_geometry_is_rejected() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<rect x='0' y='0' width='5em' height='80'/>
</svg>"#;
        match convert(source, false).unwrap() {
            Conversion::Incompatible(findings) => {
                assert!(findings.iter().any(|f| f.reason.contains("width")));
            }
            other => panic!("expected Incompatible, got {other:?}"),
        }
    }

    #[test]
    fn a_non_numeric_ellipse_radius_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<ellipse cx='20' cy='20' rx='abc' ry='5'/></svg>"#,
        );
        assert!(
            findings
                .iter()
                .any(|f| f.tag == "ellipse" && f.reason.contains("rx")),
            "{findings:?}"
        );
    }

    /// `<foreignObject>` content and `<switch>`'s conditional children are not
    /// modelled: the renderer draws none of the former and exactly one of the
    /// latter, while the composer would keep and rescale all of it. Accepting them
    /// silently — the state before R64 — drops content or draws the wrong child.
    #[test]
    fn foreign_object_and_switch_are_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<foreignObject x='0' y='0' width='10' height='10'><rect width='10' height='10'/></foreignObject>
<switch><rect width='10' height='10'/></switch>
</svg>"#,
        );
        let tags: Vec<&str> = findings.iter().map(|f| f.tag.as_str()).collect();
        assert!(tags.contains(&"foreignObject"), "{findings:?}");
        assert!(tags.contains(&"switch"), "{findings:?}");
    }

    #[test]
    fn nested_svg_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<svg x='0' y='0' width='10' height='10' viewBox='0 0 10 10'><circle cx='1' cy='1' r='1'/></svg>
</svg>"#,
        );
        assert!(findings
            .iter()
            .any(|f| f.tag == "svg" && f.path == "svg/svg"));
    }

    #[test]
    fn symbol_in_the_drawable_tree_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<symbol id='s'><rect x='1' y='1' width='2' height='2'/></symbol></svg>"#,
        );
        assert!(findings.iter().any(|f| f.tag == "symbol"));
    }

    #[test]
    fn symbol_inside_defs_is_not_a_placement_hazard() {
        // `<defs>` is never drawn where it sits; the prune hoists what the
        // symbol carries and drops the wrapper (R38). A symbol that reaches the
        // drawable tree (a `<use>` pulls it out) is still rejected.
        let converted = crate::convert::test_support::converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><symbol id='s'><linearGradient id='g'><stop offset='0' stop-color='#f00'/><stop offset='1' stop-color='#00f'/></linearGradient></symbol></defs>
<rect x='1' y='1' width='2' height='2' fill='url(#g)'/></svg>"#,
        );
        assert!(converted.svg.contains("<linearGradient"));
        assert!(!converted.svg.contains("<symbol"), "{}", converted.svg);
    }

    #[test]
    fn use_with_unknown_reference_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<use x='0' y='0' href='#s'/></svg>"#,
        );
        assert!(findings.iter().any(|f| f.tag == "use"));
    }

    #[test]
    fn defined_use_is_expanded_not_rejected() {
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><circle id='s' cx='1' cy='1' r='1'/></defs>
<use x='10' y='20' href='#s'/></svg>"#,
        );
        assert!(!done.svg.contains("<use"));
        // The circle was expanded, translated, and kept as a circle (position tag).
        assert!(done.svg.contains(r#"cx="11""#), "{}", done.svg);
    }

    #[test]
    fn defs_primitive_is_dropped() {
        // An unreferenced renderable primitive inside <defs> does not render and
        // is simply deleted (it used to be a hard error).
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><circle cx='1' cy='1' r='1'/></defs><circle cx='5' cy='5' r='1'/></svg>"#;
        let done = converted(source);
        assert!(!done.svg.contains("cx=\"1\""), "{}", done.svg);
        assert!(done.svg.contains("cx=\"5\""), "{}", done.svg);
    }

    #[test]
    fn tspan_with_coordinates_is_accepted() {
        // A `<tspan>` with absolute x/y is now accepted: the composer only moves
        // the `<text>`'s own x/y and carries the `<tspan>`s along, and a pure
        // ancestor translation is baked into both (see the C1/C2 notes).
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text x='1' y='2'><tspan x='5' y='6'>a</tspan></text></svg>"#;
        let done = converted(source);
        assert!(done.svg.contains(r#"<tspan x="5" y="6">"#), "{}", done.svg);
    }

    #[test]
    fn tspan_absolute_coordinates_survive_a_translated_ancestor() {
        // The `<g transform>` is baked away; the `<text>` anchor moves to the
        // translated origin and the `<tspan>` x/y are carried into the same
        // coordinate system (so the runs stay exactly where they rendered).
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<g transform='translate(30 40)'><text><tspan x='0' y='0' style='font-size: 10px'>a</tspan><tspan x='5' y='0'>b</tspan></text></g></svg>"#;
        let done = converted(source);
        assert!(
            !done.svg.contains("transform=\"translate(30 40)\""),
            "{}",
            done.svg
        );
        assert!(!done.svg.contains("transform="), "{}", done.svg);
        assert!(done.svg.contains(r#"<text x="30" y="40">"#), "{}", done.svg);
        assert!(done.svg.contains(r#"<tspan x="30" y="40""#), "{}", done.svg);
        assert!(
            done.svg.contains(r#"<tspan x="35" y="40">"#),
            "{}",
            done.svg
        );
    }

    #[test]
    fn tspan_without_coordinates_is_allowed() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<text x='1' y='2'><tspan>a</tspan></text></svg>"#;
        assert!(matches!(
            convert(source, false).unwrap(),
            Conversion::Converted(_)
        ));
    }

    #[test]
    fn rotated_rect_is_rejected() {
        let findings = incompatible(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<rect x='1' y='1' width='2' height='2' transform='rotate(30)'/></svg>"#,
        );
        assert!(findings.iter().any(|f| f.tag == "rect"));
    }
}
