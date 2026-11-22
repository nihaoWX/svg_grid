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
    check_residue(root, false, &mut residue);
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
///
/// `in_mask_or_marker` marks subtrees the `shapes` stage does not translate
/// (mask/marker content is not part of a supported shape tree): a `<path>` there
/// is untranslated residue even when its `d` uses the flattened form.
///
/// That mask/marker rule is a **deliberate fail-closed** decision (R28), not an
/// oversight. A canonical flattened `<path>` inside a root- or `<g>`-level
/// `<mask>`, or inside a root-level `<marker>`, makes `self_check` return `rc=1`:
/// the serialization cannot prove provenance there, because the stage never
/// emits such a path itself. None of the four supported dialects produces a
/// root-level `<mask>`/`<marker>`, and the 220-fixture corpus showed 0
/// regressions from tightening this check.
///
/// The whitelist is about *provenance*, not about container names: `<a>`,
/// `<switch>` and `<text>` are ordinary traversed containers (as are nested
/// `<g>`/`<defs>`), so a canonical flattened path inside them is accepted. It is
/// not "everything else is residue" — only the untranslated mask/marker payload
/// subtrees are excluded.
fn check_residue(element: &Element, in_mask_or_marker: bool, problems: &mut Vec<String>) {
    match element.name.as_str() {
        // The `shapes` stage emits a flattened multi-contour `<path>` for a fill
        // region with three or more subpaths (a `<polygon>` cannot keep its
        // holes); such a path is a finished product, not dialect residue.
        "path" if !in_mask_or_marker && is_flattened_fill_path(element) => {}
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
    let child_in_mask_or_marker =
        in_mask_or_marker || matches!(element.name.as_str(), "mask" | "marker");
    for child in &element.children {
        if let Node::Element(child) = child {
            check_residue(child, child_in_mask_or_marker, problems);
        }
    }
}

/// True for the `<path>` the `shapes` stage emits on purpose: absolute `M`/`L`
/// straight commands, one explicit `Z` per contour, no transform, and the exact
/// serialization [`joined_closed_path_data`] writes — `M x,y L x,y … Z` contours
/// separated by single spaces, coordinates in `fmt_num` form (an integer or three
/// decimals), no other whitespace.
///
/// A definition container cannot be rejected wholesale: the stage legitimately
/// rewrites paths inside `<pattern>` (hatch fills), and those patterns sit in
/// `<defs>`; mask/marker content, which the stage does not translate, is excluded
/// separately by the caller. Within the shape tree the serialization is the
/// remaining provenance: a hand-written `d` that happens to match it is already a
/// flattened fill region with straight segments, so accepting it is harmless.
/// Anything else — spaces or newlines between coordinates, relative/mixed
/// commands, curves, a missing `Z` — is untranslated residue and fails the check.
/// This is not a container-name rule (see [`check_residue`]): `<a>`, `<switch>`
/// and `<text>` are traversed like `<g>`, so a canonical flattened path inside
/// them still passes.
fn is_flattened_fill_path(element: &Element) -> bool {
    if element.attr("transform").is_some() {
        return false;
    }
    let Some(d) = element.attr("d") else {
        return false;
    };
    let tokens: Vec<&str> = d.split(' ').collect();
    let mut index = 0;
    let mut closed = false;
    while index < tokens.len() {
        if tokens[index] != "M" {
            return false;
        }
        index += 1;
        if index >= tokens.len() || !is_coordinate(tokens[index]) {
            return false;
        }
        index += 1;
        while index < tokens.len() && tokens[index] == "L" {
            if index + 1 >= tokens.len() || !is_coordinate(tokens[index + 1]) {
                return false;
            }
            index += 2;
        }
        if index >= tokens.len() || tokens[index] != "Z" {
            return false;
        }
        index += 1;
        closed = true;
    }
    closed
}

/// One `x,y` pair in `fmt_num` form: optional `-`, digits, and an optional
/// fraction of one to three digits.
fn is_coordinate(token: &str) -> bool {
    match token.split_once(',') {
        Some((x, y)) => is_number(x) && is_number(y),
        None => false,
    }
}

fn is_number(text: &str) -> bool {
    let text = text.strip_prefix('-').unwrap_or(text);
    match text.split_once('.') {
        Some((whole, fraction)) => {
            !whole.is_empty()
                && (1..=3).contains(&fraction.len())
                && whole.bytes().all(|byte| byte.is_ascii_digit())
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()),
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
        // A `<path>` is accepted by the scan (the translator handles it), but an
        // untranslated one — curves, relative commands, or no explicit close —
        // must never reach the output.
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
    fn self_check_accepts_the_flattened_multi_contour_fill_path() {
        // The `shapes` stage emits this exact form on purpose for a fill region
        // with three or more subpaths (a `<polygon>` would fill its holes solid).
        let root = parse(
            "<svg><rect data-panel-box='main' x='0' y='0' width='1' height='1'/>\
             <g data-scale='xy'><path d='M 0,0 L 4,0 L 4,4 Z M 1,1 L 3,1 L 3,3 Z \
             M 8,8 L 9,8 L 9,9 Z' fill-rule='evenodd'/></g></svg>",
        )
        .unwrap();
        if let Err(error) = self_check(&root) {
            panic!("the flattened fill path must pass the self-check, got {error:?}");
        }
    }

    #[test]
    fn self_check_rejects_flattened_paths_inside_masks_and_markers() {
        // The same `d` is a finished product in the shape tree but untranslated
        // residue inside a mask/marker (the `shapes` stage does not translate
        // their content): accepting it by serialization alone would hide a
        // dropped/kept `<mask>`/`<marker>` payload.
        for container in [
            "<defs><mask id='m'><path d='M 0,0 L 4,0 L 4,4 Z'/></mask></defs>",
            "<defs><marker id='k'><path d='M 0,0 L 4,0 L 4,4 Z'/></marker></defs>",
        ] {
            let root = parse(&format!(
                "<svg><rect data-panel-box='main' x='0' y='0' width='1' height='1'/>\
                 <g data-scale='xy'><rect x='0' y='0' width='1' height='1'/>{container}</g></svg>"
            ))
            .unwrap();
            match self_check(&root) {
                Err(ConvertError::SelfCheck(detail)) => {
                    assert!(detail.contains("path"), "{container}: {detail}")
                }
                other => panic!("{container}: expected SelfCheck error, got {other:?}"),
            }
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
