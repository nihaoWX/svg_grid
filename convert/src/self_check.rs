//! Post-conversion invariant self-check (detects internal bugs).

use crate::dom::{Element, Node};
use crate::error::ConvertError;
use crate::scan::scan;
use crate::tag::count_panel_boxes;

/// Assert the invariants the composer relies on:
/// (a) exactly one panel box; (b) no unsupported primitives; (c) no dialect
/// residue the translation stage is supposed to have removed; (d) every `<text>`
/// and `<circle>` that is *not* inside `<defs>`/`<clipPath>` sits under a
/// `position` policy (matching what `wrap_text_circles` tags).
pub fn self_check(root: &Element) -> Result<(), ConvertError> {
    let boxes = count_panel_boxes(root);
    if boxes != 1 {
        return Err(ConvertError::SelfCheck(format!(
            "expected exactly 1 `data-panel-box=\"main\"` rect, found {boxes}"
        )));
    }

    let findings = scan(root);
    if !findings.is_empty() {
        return Err(ConvertError::SelfCheck(format!(
            "output still contains {} unsupported node(s)",
            findings.len()
        )));
    }

    let mut residue = Vec::new();
    check_residue(root, &mut residue);
    if !residue.is_empty() {
        return Err(ConvertError::SelfCheck(format!(
            "output still contains untranslated dialect nodes: {}",
            residue.join("; ")
        )));
    }

    let mut problems = Vec::new();
    check_positions(root, None, &mut problems, false);
    if !problems.is_empty() {
        return Err(ConvertError::SelfCheck(problems.join("; ")));
    }

    Ok(())
}

/// The translation stage must leave none of these behind.
fn check_residue(element: &Element, problems: &mut Vec<String>) {
    match element.name.as_str() {
        "path" => problems.push("a <path>".to_string()),
        "use" => problems.push("a <use>".to_string()),
        "symbol" => problems.push("a <symbol>".to_string()),
        "g" if element.attr("transform").is_some() => {
            problems.push("a <g transform=...>".to_string());
        }
        "image" if element.attr("transform").is_some() => {
            problems.push("an <image transform=...>".to_string());
        }
        "text" => {
            if let Some(transform) = element.attr("transform") {
                if !is_normalized_text_transform(transform) {
                    problems.push(format!(
                        "a <text transform={transform:?}> that is not `rotate(a x y)`"
                    ));
                }
            }
        }
        _ => {}
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            check_residue(child, problems);
        }
    }
}

/// True for the composer's supported text rotation form `rotate(a, x, y)`.
fn is_normalized_text_transform(value: &str) -> bool {
    value
        .trim()
        .strip_prefix("rotate(")
        .and_then(|value| value.strip_suffix(')'))
        .map(|inner| {
            let parts: Vec<&str> = inner
                .split([',', ' '])
                .filter(|part| !part.is_empty())
                .collect();
            parts.len() == 3 && parts.iter().all(|part| part.parse::<f32>().is_ok())
        })
        .unwrap_or(false)
}

fn check_positions(
    element: &Element,
    inherited: Option<&str>,
    problems: &mut Vec<String>,
    in_skipped_scope: bool,
) {
    let policy = element.attr("data-scale").or(inherited);
    if !in_skipped_scope
        && matches!(element.name.as_str(), "text" | "circle")
        && policy != Some("position")
    {
        problems.push(format!(
            "<{}> is not under a data-scale=\"position\" group (effective policy {:?})",
            element.name, policy
        ));
    }
    let child_scope = in_skipped_scope || matches!(element.name.as_str(), "defs" | "clipPath");
    for child in &element.children {
        if let Node::Element(child) = child {
            check_positions(child, policy, problems, child_scope);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    #[test]
    fn self_check_detects_missing_or_duplicate_panel_box() {
        // No panel box → self-check fails.
        let no_box = parse("<svg><circle cx='1' cy='1' r='1'/></svg>").unwrap();
        assert!(matches!(
            self_check(&no_box),
            Err(ConvertError::SelfCheck(_))
        ));

        // Two panel boxes → self-check fails.
        let two_boxes = parse(
            "<svg><rect data-panel-box='main' x='0' y='0' width='1' height='1'/>\
             <rect data-panel-box='main' x='0' y='0' width='1' height='1'/></svg>",
        )
        .unwrap();
        assert!(matches!(
            self_check(&two_boxes),
            Err(ConvertError::SelfCheck(_))
        ));
    }

    #[test]
    fn self_check_detects_dialect_residue() {
        // A `<path>` is accepted by the scan (the translator handles it), but the
        // *output* must never contain one — the residue check enforces that.
        let root = parse(
            "<svg><rect data-panel-box='main' x='0' y='0' width='1' height='1'/>\
             <g data-scale='xy'><path d='M 0 0 L 1 1'/></g></svg>",
        )
        .unwrap();
        match self_check(&root) {
            Err(ConvertError::SelfCheck(detail)) => assert!(detail.contains("path")),
            other => panic!("expected SelfCheck error, got {other:?}"),
        }
    }

    #[test]
    fn self_check_detects_text_outside_position() {
        let root = parse(
            "<svg><rect data-panel-box='main' x='0' y='0' width='1' height='1'/>\
             <text x='1' y='1'>oops</text></svg>",
        )
        .unwrap();
        match self_check(&root) {
            Err(ConvertError::SelfCheck(detail)) => assert!(detail.contains("position")),
            other => panic!("expected SelfCheck error, got {other:?}"),
        }
    }
}
