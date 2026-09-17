//! `<use>` → `<defs>` expansion.
//!
//! Each `<use>` becomes a `<g translate>` wrapper around a copy of the
//! referenced element; unreferenced `<defs>` content is dropped (only
//! `<clipPath>`/`<style>` survive).

use std::collections::HashMap;

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

pub(super) fn prune_defs(element: &mut Element) {
    let mut out = Vec::with_capacity(element.children.len());
    for node in std::mem::take(&mut element.children) {
        if let Node::Element(mut child) = node {
            prune_defs(&mut child);
            if child.name == "defs" {
                child.children.retain(|node| {
                    matches!(node, Node::Element(child)
                        if child.name == "clipPath" || child.name == "style")
                });
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
