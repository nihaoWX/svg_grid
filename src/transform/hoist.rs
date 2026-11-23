//! Nested definitions that are also referenced from outside their enclosing
//! definition are moved to the top level before the copies are made.
//!
//! `defs::split` copies a definition once per policy and repoints every reference
//! at the copy for its own policy. A definition nested inside another one is
//! instantiated inside each outer copy, so it cannot be split independently: if
//! an outside reference also points at it, the outside reference needs a copy at
//! the top level, and leaving it nested would either duplicate its id (one copy
//! per outer copy) or leave the outside reference dangling (round-data/R49).
//!
//! Hoisting is the smallest change that keeps both references correct: the
//! definition is split once at the top level, and each outer copy keeps a
//! reference to the copy for its own policy.

use std::collections::{BTreeSet, HashMap};

use super::defs::is_definition;
use super::ScalePolicy;
use crate::dom::{Element, Node};

/// Move every nested definition that is also referenced from outside its
/// enclosing definition to the top level.
pub(super) fn hoist_outside(root: &mut Element, refs: &HashMap<String, BTreeSet<ScalePolicy>>) {
    loop {
        let mut moved = Vec::new();
        take_outside(root, refs, None, &mut moved);
        if moved.is_empty() {
            return;
        }
        root.children.extend(moved.into_iter().map(Node::Element));
    }
}

fn take_outside(
    parent: &mut Element,
    refs: &HashMap<String, BTreeSet<ScalePolicy>>,
    enclosing: Option<&str>,
    moved: &mut Vec<Element>,
) {
    let mut index = 0;
    while index < parent.children.len() {
        let (definition, id) = match &parent.children[index] {
            Node::Element(child) => (is_definition(child), child.attr("id").map(str::to_string)),
            _ => (false, None),
        };
        if definition {
            if let (Some(owner), Some(id)) = (enclosing, id.as_deref()) {
                if refers_outside(refs, id, owner) {
                    if let Node::Element(child) = parent.children.remove(index) {
                        moved.push(child);
                    }
                    continue;
                }
            }
        }
        let owner = if definition { id.as_deref() } else { enclosing };
        if let Node::Element(child) = &mut parent.children[index] {
            take_outside(child, refs, owner, moved);
        }
        index += 1;
    }
}

/// Whether `id` is referenced under a policy its enclosing definition is not.
fn refers_outside(refs: &HashMap<String, BTreeSet<ScalePolicy>>, id: &str, owner: &str) -> bool {
    let Some(own) = refs.get(id) else {
        return false;
    };
    if own.is_empty() {
        return false;
    }
    match refs.get(owner) {
        Some(outer) => own != outer,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{parse, serialize};
    use crate::transform::defs::{referrers, split};

    fn split_text(svg: &str) -> String {
        let mut root = parse(svg).unwrap();
        let _ = split(&mut root);
        serialize(&root).unwrap()
    }

    fn id_counts(text: &str) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for part in text.split(r#"id=""#).skip(1) {
            let id = part.split('"').next().unwrap_or_default();
            *counts.entry(id.to_string()).or_insert(0) += 1;
        }
        counts
    }

    fn assert_unique(text: &str) {
        let duplicates: Vec<_> = id_counts(text)
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .collect();
        assert!(
            duplicates.is_empty(),
            "duplicate ids {duplicates:?}: {text}"
        );
    }

    #[test]
    fn a_nested_definition_is_copied_once_per_outer_copy() {
        let text = split_text(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <pattern id="p" width="2" height="2">
                      <defs><linearGradient id="g" x1="0" x2="100"/></defs>
                      <rect fill="url(#g)"/>
                    </pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="y"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        );
        assert_unique(&text);
        for id in ["p", "p--svg-grid-y", "g", "g--svg-grid-y"] {
            assert!(
                text.contains(&format!(r#"id="{id}""#)),
                "missing {id}: {text}"
            );
        }
        assert!(
            text.contains(r#"fill="url(#g--svg-grid-y)""#),
            "the y copy must point at its own gradient: {text}"
        );
    }

    #[test]
    fn an_unreferenced_nested_definition_still_gets_a_unique_id() {
        // `g` is not referenced by the pattern body, so it has no policy of its
        // own; it still cannot keep its id in two copies (round-data/R49).
        let text = split_text(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <pattern id="p" width="2" height="2">
                      <defs><linearGradient id="g" x1="0" x2="100"/></defs>
                      <rect fill="#000"/>
                    </pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        );
        assert_unique(&text);
        assert!(text.contains(r#"<linearGradient id="g""#), "{text}");
        assert!(
            text.contains(r#"<linearGradient id="g--svg-grid-x""#),
            "{text}"
        );
    }

    #[test]
    fn two_nesting_levels_are_copied_once_per_policy() {
        let text = split_text(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <pattern id="p" width="2" height="2">
                      <pattern id="g" width="1" height="1">
                        <linearGradient id="h" x1="0" x2="10"/>
                        <rect fill="url(#h)"/>
                      </pattern>
                      <rect fill="url(#g)"/>
                    </pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="y"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        );
        assert_unique(&text);
        for id in [
            "p",
            "p--svg-grid-y",
            "g",
            "g--svg-grid-y",
            "h",
            "h--svg-grid-y",
        ] {
            assert!(
                text.contains(&format!(r#"id="{id}""#)),
                "missing {id}: {text}"
            );
        }
    }

    #[test]
    fn a_nested_definition_referenced_from_outside_is_hoisted() {
        let text = split_text(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <pattern id="p" width="2" height="2">
                      <defs><linearGradient id="g" x1="0" x2="100"/></defs>
                      <rect fill="url(#g)"/>
                    </pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="y"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="x"><rect width="10" height="10" fill="url(#g)"/></g>
                </svg>"##,
        );
        let start = text.find(r#"<pattern id="p""#).unwrap();
        let end = text[start..].find("</pattern>").unwrap() + start;
        assert!(
            !text[start..end].contains("<linearGradient"),
            "the gradient must leave the pattern: {text}"
        );
        assert!(
            text.contains(r#"<linearGradient id="g--svg-grid-x""#),
            "{text}"
        );
        assert!(
            text.contains(r#"<linearGradient id="g--svg-grid-y""#),
            "{text}"
        );
    }

    #[test]
    fn a_nested_definition_with_the_same_policies_stays_nested() {
        let mut root = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <pattern id="p" width="2" height="2">
                      <defs><linearGradient id="g" x1="0" x2="100"/></defs>
                      <rect fill="url(#g)"/>
                    </pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="y"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        )
        .unwrap();
        let refs = referrers(&root);
        hoist_outside(&mut root, &refs);
        let text = serialize(&root).unwrap();
        let start = text.find(r#"<pattern id="p""#).unwrap();
        let end = text[start..].find("</pattern>").unwrap() + start;
        assert!(
            text[start..end].contains("<linearGradient"),
            "nothing references the gradient from outside: {text}"
        );
    }
}
