//! `<use>` → `<defs>` expansion, `<defs>` pruning and the dangling-reference gate.
//!
//! Each `<use>` becomes a `<g translate>` wrapper around a copy of the
//! referenced element. `<defs>` content that nothing points at is dropped, but
//! *what counts as a reference* is decided by the tree, not by a list of element
//! names: any `url(#id)` in any attribute (including an inline `style`) and any
//! `href="#id"` keeps the definition with that id, transitively (`<pattern>` →
//! `<linearGradient>`). `<style>` and the five [`REFERENCE_TARGETS`] are kept
//! unconditionally, because a paint server or clip path whose loss is silent is
//! exactly the failure this stage exists to prevent (round-data/R16).
//!
//! Pruning is followed by [`check_dangling_references`]: a reference whose target
//! no longer exists is a hard error, not a silently unpainted shape.

use std::collections::{HashMap, HashSet};

use crate::dom::{Element, Node};
use crate::error::ConvertError;
use crate::transform::fmt_num;

use super::number;
use super::style::{parse_style, serialize_style};

/// Presentation attributes inherited from a `<use>` onto its expansion.
const INHERITED_ATTRS: [&str; 14] = [
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-opacity",
    "stroke-width",
    "stroke-linejoin",
    "stroke-linecap",
    "stroke-dasharray",
    "stroke-dashoffset",
    "opacity",
    "clip-path",
    "fill-rule",
    "clip-rule",
];

pub(super) fn collect_ids(element: &Element, ids: &mut HashMap<String, Element>) {
    if let Some(id) = element.attr("id") {
        ids.entry(id.to_string()).or_insert_with(|| element.clone());
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_ids(child, ids);
        }
    }
}

pub(super) fn expand_uses(
    children: &mut Vec<Node>,
    ids: &HashMap<String, Element>,
    depth: u32,
) -> Result<(), ConvertError> {
    if depth > 32 {
        return Err(ConvertError::Translate(
            "<use> expansion exceeded 32 levels (likely a reference cycle)".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(children.len());
    for node in std::mem::take(children) {
        match node {
            Node::Element(child) if child.name == "use" => {
                out.push(Node::Element(expand_one(&child, ids, depth)?));
            }
            Node::Element(mut child) => {
                expand_uses(&mut child.children, ids, depth)?;
                out.push(Node::Element(child));
            }
            other => out.push(other),
        }
    }
    *children = out;
    Ok(())
}

fn expand_one(
    use_element: &Element,
    ids: &HashMap<String, Element>,
    depth: u32,
) -> Result<Element, ConvertError> {
    let href = use_element
        .attr("xlink:href")
        .or_else(|| use_element.attr("href"))
        .unwrap_or("");
    let id = href.strip_prefix('#').ok_or_else(|| {
        ConvertError::Translate(format!("<use> href {href:?} is not a `#id` reference"))
    })?;
    let referenced = ids
        .get(id)
        .ok_or_else(|| ConvertError::Translate(format!("<use> references unknown id #{id}")))?;

    let mut clone = referenced.clone();
    strip_ids(&mut clone);
    expand_uses(&mut clone.children, ids, depth + 1)?;
    merge_presentation(use_element, &mut clone);

    let x = number(use_element, "x")?;
    let y = number(use_element, "y")?;
    let mut transform = String::new();
    if x != 0.0 || y != 0.0 {
        transform = format!("translate({},{}) ", fmt_num(x), fmt_num(y));
    }
    if let Some(own) = use_element.attr("transform") {
        transform.push_str(own);
    }

    let mut attrs = Vec::new();
    if !transform.is_empty() {
        attrs.push(("transform".to_string(), transform));
    }
    for (name, value) in &use_element.attrs {
        if matches!(
            name.as_str(),
            "x" | "y" | "href" | "xlink:href" | "width" | "height" | "transform" | "style" | "id"
        ) {
            continue;
        }
        clone.set_attr(name, value.clone());
    }

    Ok(Element {
        name: "g".to_string(),
        attrs,
        children: vec![Node::Element(clone)],
    })
}

fn strip_ids(element: &mut Element) {
    element.remove_attr("id");
    for child in &mut element.children {
        if let Node::Element(child) = child {
            strip_ids(child);
        }
    }
}

/// Merge the `<use>`'s presentation onto the cloned content. The clone's own
/// declarations win (an explicit property on the referenced element has higher
/// specificity than the inherited value from the `<use>`).
fn merge_presentation(use_element: &Element, clone: &mut Element) {
    if let Some(use_style) = use_element.attr("style") {
        let existing = clone.attr("style").map(str::to_string).unwrap_or_default();
        let mut merged = parse_style(&existing);
        for (property, value) in parse_style(use_style) {
            if !merged.iter().any(|(key, _)| *key == property) {
                merged.push((property, value));
            }
        }
        if !merged.is_empty() {
            clone.set_attr("style", serialize_style(&merged));
        }
    }

    for name in INHERITED_ATTRS {
        if clone.attr(name).is_none() {
            if let Some(value) = use_element.attr(name) {
                clone.set_attr(name, value.to_string());
            }
        }
    }
}

/// `<defs>` children that are *not* drawn where they sit but are the ink of the
/// shapes that reference them, so they must survive even when the reference
/// scanner cannot see a reference (a CSS rule in `<style>` may be the only thing
/// pointing at a paint server).
///
/// `style` is kept for the CSS report, `clipPath` for clipping; the paint servers
/// are the ones whose loss is silent — a `fill="url(#pat-0)"` whose
/// `<linearGradient>`/`<radialGradient>`/`<pattern>` is gone paints **nothing**.
/// Everything else (`<mask>`, `<marker>`, `<filter>`, …) survives only when
/// something actually references it.
const REFERENCE_TARGETS: [&str; 5] = [
    "clipPath",
    "style",
    "linearGradient",
    "radialGradient",
    "pattern",
];

/// Drop every `<defs>` child that nothing references.
///
/// The reference set is computed from the tree *outside* `<defs>` first, then
/// closed over the definitions that are kept: a `<pattern>` that survives may
/// itself point at a `<linearGradient>`, which must survive with it. A
/// definition that is only reached from an already-dropped definition stays
/// dropped.
pub(super) fn prune_defs(element: &mut Element) {
    let mut referenced = HashSet::new();
    collect_references_outside_defs(element, &mut referenced);

    loop {
        let mut grown = false;
        visit_defs(element, &mut |child| {
            let Some(id) = child.attr("id") else {
                return;
            };
            if !referenced.contains(id) {
                return;
            }
            let before = referenced.len();
            collect_references(child, &mut referenced);
            grown |= referenced.len() != before;
        });
        if !grown {
            break;
        }
    }

    prune_defs_with(element, &referenced);
}

fn prune_defs_with(element: &mut Element, referenced: &HashSet<String>) {
    let mut out = Vec::with_capacity(element.children.len());
    for node in std::mem::take(&mut element.children) {
        if let Node::Element(mut child) = node {
            prune_defs_with(&mut child, referenced);
            if child.name == "defs" {
                child.children = keep_definitions(std::mem::take(&mut child.children), referenced);
                if child.children.is_empty() {
                    continue;
                }
            }
            out.push(Node::Element(child));
        } else {
            out.push(node);
        }
    }
    element.children = out;
}

/// The definitions a `<defs>` subtree still needs, hoisted out of the wrappers
/// that carry them.
///
/// `<defs>` children are not required to be definitions: `<defs><g><linearGradient
/// id="g">` is legal, and so is `<defs><symbol id="s"><linearGradient …>` or
/// `<defs><defs><pattern …>`. Dropping such a wrapper would drop the definition
/// with it, and the *silent* failure mode is the one this module exists to
/// prevent: the shape paints nothing. So a wrapper (`<g>`, `<symbol>`, a nested
/// `<defs>`, `<a>`, `<switch>`) is not kept as a container — it is unlaced and
/// its kept descendants are spliced into the `<defs>` itself. The wrapper was
/// never drawn where it sat, so nothing moves; only the reachability of the ids
/// inside it matters.
///
/// A node whose *own* id is referenced stays verbatim (a `<mask id="m">` is the
/// definition), and so does every [`REFERENCE_TARGETS`] element.
fn keep_definitions(children: Vec<Node>, referenced: &HashSet<String>) -> Vec<Node> {
    let mut out = Vec::with_capacity(children.len());
    for node in children {
        let Node::Element(mut child) = node else {
            continue;
        };
        let kept_by_id = child.attr("id").is_some_and(|id| referenced.contains(id));
        if REFERENCE_TARGETS.contains(&child.name.as_str()) || kept_by_id {
            out.push(Node::Element(child));
        } else if is_wrapper(&child.name) {
            out.append(&mut keep_definitions(
                std::mem::take(&mut child.children),
                referenced,
            ));
        }
    }
    out
}

/// Containers that a `<defs>` child may be wrapped in without being a definition
/// itself (see [`keep_definitions`]).
fn is_wrapper(name: &str) -> bool {
    matches!(name, "g" | "symbol" | "defs" | "a" | "switch")
}

/// Call `visit` on every element inside every `<defs>` in the tree, at any
/// depth.
///
/// The scan and the prune must agree on what a definition is: a `<linearGradient>`
/// wrapped in `<g>`, a `<mask>` inside a `<symbol>`, a paint server in a nested
/// `<defs>` is just as reachable by `url(#…)` as a direct child. Visiting only
/// the direct children (R37) made the prune drop those definitions while the
/// reference survived, and the new dangling-reference gate then hard-failed a
/// document the renderers handle fine (R38).
fn visit_defs(element: &Element, visit: &mut impl FnMut(&Element)) {
    for child in &element.children {
        let Node::Element(child) = child else {
            continue;
        };
        if child.name == "defs" {
            visit_defs_contents(child, visit);
        } else {
            visit_defs(child, visit);
        }
    }
}

fn visit_defs_contents(element: &Element, visit: &mut impl FnMut(&Element)) {
    for child in &element.children {
        if let Node::Element(child) = child {
            visit(child);
            visit_defs_contents(child, visit);
        }
    }
}

/// Every id referenced anywhere in `element`'s subtree.
fn collect_references(element: &Element, ids: &mut HashSet<String>) {
    collect_own_references(element, ids);
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_references(child, ids);
        }
    }
}

/// Like [`collect_references`], but a `<defs>` subtree contributes nothing: a
/// definition that is itself dropped must not keep its own targets alive.
fn collect_references_outside_defs(element: &Element, ids: &mut HashSet<String>) {
    if element.name == "defs" {
        return;
    }
    collect_own_references(element, ids);
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_references_outside_defs(child, ids);
        }
    }
}

fn collect_own_references(element: &Element, ids: &mut HashSet<String>) {
    for (name, value) in &element.attrs {
        for id in url_ids(value) {
            ids.insert(id.to_string());
        }
        if let Some(id) = href_id(element, name, value) {
            ids.insert(id.to_string());
        }
    }
}

/// The ids inside every `url(#id)` of one attribute value. Whitespace inside the
/// parentheses (`url( #id )`) and a quoted argument (`url('#id')`) are accepted:
/// they are valid CSS, and treating them as "no reference" would silently drop
/// the definition they point at.
fn url_ids(value: &str) -> Vec<&str> {
    let mut ids = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("url(") {
        let after = &rest[start + "url(".len()..];
        let Some(end) = after.find(')') else {
            break;
        };
        let inner = after[..end].trim();
        let inner = inner
            .strip_prefix('\'')
            .and_then(|inner| inner.strip_suffix('\''))
            .or_else(|| {
                inner
                    .strip_prefix('"')
                    .and_then(|inner| inner.strip_suffix('"'))
            })
            .unwrap_or(inner);
        if let Some(id) = inner.trim().strip_prefix('#') {
            let id = id.trim();
            if !id.is_empty() {
                ids.push(id);
            }
        }
        rest = &after[end..];
    }
    ids
}

/// The id of a `href="#id"` / `xlink:href="#id"` attribute. Only those two
/// attribute names are considered: `fill="#fff"` also starts with `#`, and it is
/// a colour, not a reference.
///
/// An `<a href="#…">` is a *hyperlink* to an anchor in the document, not a
/// rendering reference, so it is skipped: a missing anchor is not a broken
/// picture.
fn href_id<'a>(element: &Element, name: &str, value: &'a str) -> Option<&'a str> {
    if !matches!(name, "href" | "xlink:href") || element.name == "a" {
        return None;
    }
    let id = value.trim().strip_prefix('#')?.trim();
    (!id.is_empty()).then_some(id)
}

/// Fail closed when a `url(#id)` or `href="#id"` points at an id that nothing in
/// the tree defines.
///
/// This is the second half of the `<defs>` fix: keeping a definition alive is
/// only useful if the reference still resolves. A dangling `mask`/`marker`/
/// `filter` reference is *not* a cosmetic problem — the renderer either drops the
/// whole shape or paints it unmasked — so it is reported with the element, the
/// attribute and the missing id instead of being written out.
///
/// `clip-path` is checked here too, for *existence* only: whether the target is
/// really a `<clipPath>` and whether its box can be baked is
/// [`super::bake::apply_clip_refs`]'s job, and that stage only looks at clips
/// referenced from a transformed context — a dangling `clip-path` on an
/// untransformed shape would otherwise slip through untouched.
pub(super) fn check_dangling_references(root: &Element) -> Result<(), ConvertError> {
    let mut defined = HashSet::new();
    collect_defined_ids(root, &mut defined);

    let mut problems: Vec<String> = Vec::new();
    walk_references(root, &defined, &mut problems);
    if problems.is_empty() {
        return Ok(());
    }
    Err(ConvertError::Translate(format!(
        "{} reference(s) point at an id that nothing in the input defines: {}; add the missing \
         definition or remove the reference",
        problems.len(),
        problems.join(", ")
    )))
}

fn collect_defined_ids(element: &Element, ids: &mut HashSet<String>) {
    if let Some(id) = element.attr("id") {
        ids.insert(id.to_string());
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect_defined_ids(child, ids);
        }
    }
}

fn walk_references(element: &Element, defined: &HashSet<String>, problems: &mut Vec<String>) {
    for (name, value) in &element.attrs {
        let mut missing: Vec<&str> = url_ids(value)
            .into_iter()
            .chain(href_id(element, name, value))
            .filter(|id| !defined.contains(*id))
            .collect();
        missing.sort_unstable();
        missing.dedup();
        for id in missing {
            let problem = format!("<{}> {name}={value:?} -> #{id}", element.name);
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            walk_references(child, defined, problems);
        }
    }
}
