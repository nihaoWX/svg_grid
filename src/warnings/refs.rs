//! W9: a `url(#id)` reference in the output whose target is not defined.
//!
//! This is the mechanical check for the failure mode that cost the most ink so far
//! (round-data/R16): `<defs>` paint servers (`<linearGradient>` /
//! `<radialGradient>` / `<pattern>`) and `<clipPath>`s are *resources* that no
//! coordinate rewrite can recreate, so whatever stage drops one leaves every
//! `fill="url(#…)"` / `clip-path="url(#…)"` pointing at nothing. Nothing fails,
//! no element is misplaced — the shapes are simply painted with nothing, so the
//! figure "looks fine" while a whole series is missing.
//!
//! The check is deliberately mechanical and cheap: it compares the set of ids
//! defined in the tree against the set of ids referenced by a `url(#…)`, and says
//! so when they do not line up. A dangling `<use href="#missing">` is *not*
//! reported (it draws nothing and the converter already refuses it).

use std::collections::HashSet;

use crate::dom::{Element, Node};

use super::assemble;

/// One `url(#…)` reference whose id is not defined in the same tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingReference {
    /// Which input (its position in the grid) the reference belongs to.
    pub index: usize,
    /// The element's tag name (`rect`, `g`, …).
    pub element: String,
    /// The attribute holding the reference (`fill`, `stroke`, `clip-path`, `style`, …).
    pub attr: String,
    /// The id that is referenced but not defined.
    pub id: String,
}

/// Every `url(#id)` in `root` whose `id` is not defined in `root`.
///
/// Called per input *before* the per-cell id namespacing, so the reported id is
/// the one written in the source file (namespacing prefixes the definitions and
/// the references alike, so it never creates or hides a dangling one). Offenders
/// are deduplicated by `(index, element, attr, id)`.
pub fn dangling_references(root: &Element, index: usize) -> Vec<DanglingReference> {
    let mut ids = HashSet::new();
    collect_ids(root, &mut ids);

    let mut offenders: Vec<DanglingReference> = Vec::new();
    collect_refs(root, index, &ids, &mut offenders);
    offenders
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

fn collect_refs(
    element: &Element,
    index: usize,
    ids: &HashSet<String>,
    offenders: &mut Vec<DanglingReference>,
) {
    for (name, value) in &element.attrs {
        for id in url_ids(value) {
            if ids.contains(id) {
                continue;
            }
            let offender = DanglingReference {
                index,
                element: element.name.clone(),
                attr: name.clone(),
                id: id.to_string(),
            };
            if !offenders.contains(&offender) {
                offenders.push(offender);
            }
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_refs(child, index, ids, offenders);
        }
    }
}

/// Every id inside `url(#id)` occurrences of one attribute value (an inline
/// `style` can carry several, e.g. `fill: url(#a); stroke: url(#b)`).
fn url_ids(value: &str) -> Vec<&str> {
    const MARKER: &str = "url(#";
    let mut ids = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find(MARKER) {
        let after = &rest[start + MARKER.len()..];
        let Some(end) = after.find(')') else { break };
        let id = after[..end].trim();
        if !id.is_empty() {
            ids.push(id);
        }
        rest = &after[end..];
    }
    ids
}

/// Build the W9 warning, or `None` when every reference resolves.
///
/// The line is the *what / where / Fix* triple: which ids are referenced but not
/// defined, on which input/element/attribute, and what to keep. Both the
/// consequence and the fix stay on the line (a dangling reference is the failure
/// mode this check exists for, so it is not folded into the cause note); the
/// offender list degrades to `input #N url(#id)` when the full form would push
/// the line past [`super::MAX_LINE`].
pub fn describe_dangling_reference(offenders: &[DanglingReference]) -> Option<String> {
    if offenders.is_empty() {
        return None;
    }
    let detailed: Vec<String> = offenders
        .iter()
        .map(|offender| {
            format!(
                "input #{} <{} {}> -> url(#{})",
                offender.index, offender.element, offender.attr, offender.id
            )
        })
        .collect();
    let plain: Vec<String> = offenders
        .iter()
        .map(|offender| format!("input #{} url(#{})", offender.index, offender.id))
        .collect();
    // `reference(s)` is part of the message's stable identity: `round-data/R16`
    // quotes `1 reference(s) point at an id that nothing in the output defines`
    // and `2 reference(s)` verbatim, so this count keeps its parenthetical form.
    let head = format!(
        "{} reference(s) point at an id that nothing in the output defines: ",
        offenders.len()
    );
    Some(assemble(
        &head,
        &detailed,
        &plain,
        &[
            ". Fix: keep the <defs>/<linearGradient>; painted with nothing, loses ink.",
            ". Fix: keep the <defs>: painted with nothing, loses ink.",
        ],
    ))
}
