//! Geometry fixups and tag-protocol insertion.
//!
//! Percentage geometry is made absolute, `<defs>` is left alone, and everything
//! else is tagged with `data-panel-box` / `data-scale` as the composer expects.

use crate::dom::{Element, Node};
use crate::geom::CanvasBox;
use crate::transform::fmt_num;

/// Turn percentage geometry (e.g. svglite's `<rect width='100%' height='100%'/>`)
/// into absolute values, since the composer parses geometry with `parse_f32`.
///
/// Percentages hidden in a `style="..."` declaration are resolved the same way
/// (the composer would otherwise multiply a `%` length by a scale factor and keep
/// the `%`, e.g. `width: 100%` → `width: 200%`).
pub(crate) fn resolve_percentages(element: &mut Element, canvas: &CanvasBox) {
    // Per the SVG spec a percentage radius (and `stroke-*`) is relative to the
    // normalized diagonal `sqrt((w^2 + h^2) / 2)` — not to the width.
    let diagonal = ((canvas.width_value * canvas.width_value
        + canvas.height_value * canvas.height_value)
        / 2.0)
        .sqrt();
    for (name, value) in element.attrs.iter_mut() {
        if !value.contains('%') {
            continue;
        }
        let Some(axis) = percentage_axis(name, canvas, diagonal) else {
            continue;
        };
        if let Some(number) = value.trim().strip_suffix('%') {
            if let Ok(percent) = number.trim().parse::<f32>() {
                *value = fmt_num(percent / 100.0 * axis);
            }
        }
    }
    if let Some(style) = element.attr("style").map(str::to_string) {
        if let Some(resolved) = resolve_style_percentages(&style, canvas, diagonal) {
            element.set_attr("style", resolved);
        }
    }
    for child in &mut element.children {
        if let Node::Element(child) = child {
            resolve_percentages(child, canvas);
        }
    }
}

/// The canvas axis a percentage of `name` is measured against.
fn percentage_axis(name: &str, canvas: &CanvasBox, diagonal: f32) -> Option<f32> {
    match name {
        "x" | "cx" | "x1" | "x2" | "width" => Some(canvas.width_value),
        "y" | "cy" | "y1" | "y2" | "height" => Some(canvas.height_value),
        "r" | "stroke-width" | "stroke-dasharray" | "stroke-dashoffset" => Some(diagonal),
        _ => None,
    }
}

/// Resolve `%` length tokens inside a `style="..."` value, preserving the rest of
/// the declaration list (order, spacing, unrelated declarations) verbatim.
fn resolve_style_percentages(style: &str, canvas: &CanvasBox, diagonal: f32) -> Option<String> {
    let mut out = String::with_capacity(style.len() + 8);
    let mut changed = false;
    let mut rest = style;
    loop {
        let (declaration, remainder) = match rest.split_once(';') {
            Some((declaration, remainder)) => (declaration, Some(remainder)),
            None => (rest, None),
        };
        let rewritten = declaration.split_once(':').and_then(|(name, value)| {
            let axis = percentage_axis(name.trim(), canvas, diagonal)?;
            let value = resolve_percent_tokens(value, axis)?;
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

/// Replace every `N%` token in a declaration value with `N/100 * axis`, keeping
/// the separators and any non-percentage token untouched.
fn resolve_percent_tokens(value: &str, axis: f32) -> Option<String> {
    let mut out = String::with_capacity(value.len() + 8);
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
        match token.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
            Some(percent) => {
                out.push_str(&fmt_num(percent / 100.0 * axis));
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

pub(crate) fn count_panel_boxes(element: &Element) -> usize {
    let mut count = if element.name == "rect" && element.attr("data-panel-box") == Some("main") {
        1
    } else {
        0
    };
    for child in &element.children {
        if let Node::Element(child) = child {
            count += count_panel_boxes(child);
        }
    }
    count
}

/// True when the tree already carries a `data-scale` grouping (the second half
/// of the tag protocol).
pub(crate) fn has_data_scale(element: &Element) -> bool {
    element.attr("data-scale").is_some()
        || element
            .children
            .iter()
            .any(|child| matches!(child, Node::Element(child) if has_data_scale(child)))
}

pub(crate) fn insert_panel_box(root: &mut Element, canvas: &CanvasBox) {
    let rect = Element::new(
        "rect",
        vec![
            ("data-panel-box", "main".to_string()),
            ("x", canvas.x.clone()),
            ("y", canvas.y.clone()),
            ("width", canvas.width.clone()),
            ("height", canvas.height.clone()),
            ("visibility", "hidden".to_string()),
            ("pointer-events", "none".to_string()),
        ],
    );
    root.children.insert(0, Node::Element(rect));
}

/// Wrap every `<text>`/`<circle>` in an in-place `data-scale="position"` group.
/// Nodes nested in `<defs>` (e.g. a `<clipPath>`) are left alone so they inherit
/// the outer `xy` policy together with the geometry they clip.
pub(crate) fn wrap_text_circles(element: &mut Element, in_position: bool, stats: &mut Stats) {
    let mut out = Vec::with_capacity(element.children.len());
    for node in std::mem::take(&mut element.children) {
        let Node::Element(mut child) = node else {
            out.push(node);
            continue;
        };

        if child.name == "defs" {
            out.push(Node::Element(child));
        } else if child.name == "g" && child.attr("data-scale") == Some("position") {
            wrap_text_circles(&mut child, true, stats);
            out.push(Node::Element(child));
        } else if matches!(child.name.as_str(), "text" | "circle") && !in_position {
            if child.name == "text" {
                stats.texts += 1;
            } else {
                stats.circles += 1;
            }
            let group = Element {
                name: "g".to_string(),
                attrs: vec![("data-scale".to_string(), "position".to_string())],
                children: vec![Node::Element(child)],
            };
            out.push(Node::Element(group));
        } else {
            wrap_text_circles(&mut child, in_position, stats);
            out.push(Node::Element(child));
        }
    }
    element.children = out;
}

/// Put everything under the root (except the panel box) into a single
/// `data-scale="xy"` group so both the geometry and the `<clipPath>` rectangles
/// receive the same policy.
pub(crate) fn wrap_root_content_in_xy(root: &mut Element) {
    let mut panel_box: Option<Node> = None;
    let mut rest: Vec<Node> = Vec::new();

    for child in std::mem::take(&mut root.children) {
        if panel_box.is_none() {
            if let Node::Element(element) = &child {
                if element.name == "rect" && element.attr("data-panel-box") == Some("main") {
                    panel_box = Some(child);
                    continue;
                }
            }
        }
        rest.push(child);
    }

    let already_wrapped = {
        let elements: Vec<&Node> = rest
            .iter()
            .filter(|node| matches!(node, Node::Element(_)))
            .collect();
        elements.len() == 1
            && matches!(elements[0], Node::Element(element)
                if element.name == "g" && element.attr("data-scale") == Some("xy"))
    };

    let wrapped = if already_wrapped {
        rest
    } else {
        vec![Node::Element(Element {
            name: "g".to_string(),
            attrs: vec![("data-scale".to_string(), "xy".to_string())],
            children: rest,
        })]
    };

    root.children.clear();
    if let Some(panel_box) = panel_box {
        root.children.push(panel_box);
    }
    root.children.extend(wrapped);
}

#[derive(Default)]
pub(crate) struct Stats {
    pub(crate) texts: usize,
    pub(crate) circles: usize,
}

pub(crate) fn count_primitives(root: &Element) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    walk_count(root, &mut counts);
    counts
}

fn walk_count(element: &Element, counts: &mut Vec<(String, usize)>) {
    if matches!(
        element.name.as_str(),
        "text" | "circle" | "rect" | "line" | "polyline" | "polygon" | "use"
    ) {
        if let Some(entry) = counts.iter_mut().find(|(name, _)| name == &element.name) {
            entry.1 += 1;
        } else {
            counts.push((element.name.clone(), 1));
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            walk_count(child, counts);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::converted;

    #[test]
    fn clip_rect_inside_defs_is_allowed() {
        let source = r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<defs><clipPath id='c'><rect x='1' y='1' width='10' height='10'/></clipPath></defs>
<circle cx='1' cy='1' r='1'/></svg>"#;
        let done = converted(source);
        assert!(done.svg.contains("<clipPath"));
        assert!(done
            .svg
            .contains("<rect x=\"1\" y=\"1\" width=\"10\" height=\"10\"/>"));
    }

    #[test]
    fn style_percentages_use_the_same_axes_as_attributes() {
        // width 100 / height 80 → normalized diagonal sqrt(8200) = 90.5539.
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<circle cx='50%' cy='25%' r='10%' style='stroke-width: 10%; stroke: #000000'/></svg>"#,
        );
        assert!(done.svg.contains(r#"cx="50""#), "{}", done.svg);
        assert!(done.svg.contains(r#"cy="20""#), "{}", done.svg);
        assert!(done.svg.contains(r#"r="9.055""#), "{}", done.svg);
        assert!(
            done.svg.contains(r#"stroke-width: 9.055"#),
            "the style stroke-width percent must use the diagonal: {}",
            done.svg
        );
        assert!(
            !done.svg.contains('%'),
            "no percent may remain: {}",
            done.svg
        );
    }

    #[test]
    fn percentage_radius_uses_normalized_diagonal() {
        // sqrt((100^2 + 80^2) / 2) = sqrt(8200) = 90.5539 → 10% = 9.055
        let done = converted(
            r#"<svg width='100' height='80' viewBox='0 0 100 80' xmlns='http://www.w3.org/2000/svg'>
<circle cx='50' cy='40' r='10%'/></svg>"#,
        );
        assert!(
            done.svg.contains(r#"r="9.055""#),
            "expected normalized-diagonal radius, got: {}",
            done.svg
        );
    }
}
