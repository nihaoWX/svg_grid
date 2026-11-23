//! What a paint server inherits from the element its `href` points at.
//!
//! A gradient, pattern or filter may leave an attribute out and inherit it from
//! another element of the same family. `usvg-0.47.0` resolves such an attribute
//! with `resolve_attr` (`src/parser/paint_server.rs:417-540`), and the tags it is
//! willing to follow are **not uniform**:
//!
//! * gradient **coordinates** (`x1/y1/x2/y2` for a linear gradient, `cx/cy/r/fx/fy`
//!   for a radial one) come only from a link of the *same* gradient kind;
//! * `gradientUnits`, `spreadMethod` and `gradientTransform` come from *either*
//!   gradient kind, because they mean the same thing on both;
//! * a `<pattern>` or `<filter>` follows the chain only while the link is the same
//!   tag (`resolve_pattern_attr`, `resolve_filter_attr`);
//! * a `<mask>` follows **no** chain at all: `src/parser/mask.rs:20-30` reads
//!   `node.attribute(AId::MaskUnits)` directly, so `maskUnits`/`maskContentUnits`
//!   are *not* inherited. Do not "unify" this with the gradient families.
//!
//! And the units are resolved *before* the numbers they apply to are read
//! (`convert_units` calls `resolve_attr`, `resolve_number` calls `resolve_attr`),
//! so a `<linearGradient href="#base" x1="10">` whose `base` says
//! `gradientUnits="userSpaceOnUse"` has a `10` in **user space**, even though the
//! element that spells `x1` does not say so itself (round-data/R46).

use std::collections::{HashMap, HashSet};

use super::units::{convert_units, Units};
use crate::dom::{Element, Node};

/// Every element that owns an `id`, with the handful of attributes a definition
/// family can inherit, collected once per transform.
#[derive(Default)]
pub(super) struct IdIndex {
    elements: HashMap<String, Indexed>,
}

/// A snapshot of one element: what [`IdIndex`] can answer about it.
struct Indexed {
    name: String,
    href: Option<String>,
    attrs: Vec<(String, String)>,
}

impl Indexed {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

impl IdIndex {
    /// Index every element with an `id`. Duplicate ids keep the **last**
    /// occurrence, like the renderer's id map (`usvg-0.47.0/src/parser/
    /// svgtree/parse.rs:117-121`).
    pub(super) fn collect(root: &Element) -> Self {
        let mut index = Self::default();
        index.walk(root);
        index
    }

    fn walk(&mut self, element: &Element) {
        if let Some(id) = element.attr("id") {
            self.elements.insert(
                id.to_string(),
                Indexed {
                    name: element.name.clone(),
                    href: link_of(element).map(str::to_string),
                    attrs: element
                        .attrs
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                },
            );
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                self.walk(child);
            }
        }
    }

    fn get(&self, id: &str) -> Option<&Indexed> {
        self.elements.get(id)
    }
}

/// The value the renderer reads for `name` on `element`, following `href` with
/// exactly the tag rules of `resolve_attr` (see the module docs).
///
/// Returns `None` when neither the element nor a followable link declares it —
/// the caller then applies the attribute's own default.
pub(super) fn resolved_attr(element: &Element, index: &IdIndex, name: &str) -> Option<String> {
    if let Some(value) = element.attr(name) {
        return Some(value.to_string());
    }
    let owner = element.name.as_str();
    let mut link = link_of(element).map(str::to_string);
    let mut visited: HashSet<String> = HashSet::new();
    while let Some(href) = link {
        let Some(id) = internal_target(&href) else {
            break;
        };
        if !visited.insert(id.to_string()) {
            // A `href` cycle: the renderer stops too (`HrefIter` yields each
            // element once), so the chain ends with what has been resolved.
            break;
        }
        let Some(target) = index.get(id) else { break };
        if !follows(owner, &target.name, name) {
            break;
        }
        if let Some(value) = target.attr(name) {
            return Some(value.to_string());
        }
        link = target.href.clone();
    }
    None
}

/// The units of `name`, resolved along `href` first (`convert_units` in usvg).
pub(super) fn resolved_units(
    element: &Element,
    index: &IdIndex,
    name: &str,
    default: Units,
) -> Units {
    convert_units(resolved_attr(element, index, name).as_deref(), default)
}

/// Whether `resolve_attr` may step from a `owner` element to a `target` element
/// of another tag while looking for `name`.
fn follows(owner: &str, target: &str, name: &str) -> bool {
    match owner {
        "linearGradient" => match name {
            "x1" | "y1" | "x2" | "y2" => target == "linearGradient",
            "gradientUnits" | "spreadMethod" | "gradientTransform" => {
                matches!(target, "linearGradient" | "radialGradient")
            }
            // Any other (attribute, tag) pair makes `resolve_lg_attr` `break`.
            _ => false,
        },
        "radialGradient" => match name {
            "cx" | "cy" | "r" | "fx" | "fy" => target == "radialGradient",
            "gradientUnits" | "spreadMethod" | "gradientTransform" => {
                matches!(target, "linearGradient" | "radialGradient")
            }
            _ => false,
        },
        "pattern" => target == "pattern",
        "filter" => target == "filter",
        _ => false,
    }
}

/// The raw `href` of a definition, in either spelling.
pub(super) fn link_of(element: &Element) -> Option<&str> {
    element.attr("href").or_else(|| element.attr("xlink:href"))
}

/// The `#id` a link points at, if the link is an internal IRI the renderer can
/// follow, parsed like `svgtypes::Stream::parse_iri`:
///
/// * leading and trailing **spaces** are skipped (`' '` only: a tab belongs to
///   the id, so `"#\tbase"` names a different, unresolvable element);
/// * the fragment runs to the first literal space;
/// * whatever follows it must be spaces only.
///
/// A URI with a scheme or a path (`other.svg#q`, `http://example.com/#p`) is not
/// an internal target, and neither is an empty fragment (round-data/R43). An id
/// carrying a tab or a line break cannot name an element this tool can follow, so
/// such a fragment is rejected as well.
pub(super) fn internal_target(href: &str) -> Option<&str> {
    let fragment = href.trim_start_matches(' ').strip_prefix('#')?;
    let end = fragment.find(' ').unwrap_or(fragment.len());
    let (id, tail) = fragment.split_at(end);
    if id.is_empty() || !tail.bytes().all(|byte| byte == b' ') {
        return None;
    }
    if id.contains(['\t', '\n', '\r']) {
        return None;
    }
    Some(id)
}

/// Rewrite every `url(#id)` in `value` through `map`, leaving everything else —
/// including a `url()` that does not hold an internal IRI — verbatim.
pub(super) fn map_url_references(value: &str, mut map: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(position) = rest.find("url(") {
        let after = position + 4;
        out.push_str(&rest[..after]);
        let tail = &rest[after..];
        let Some(close) = tail.find(')') else {
            out.push_str(tail);
            return out;
        };
        let inner = &tail[..close];
        match inner.strip_prefix('#') {
            Some(id) if !id.is_empty() => {
                out.push('#');
                out.push_str(&map(id));
            }
            _ => out.push_str(inner),
        }
        out.push(')');
        rest = &tail[close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{internal_target, map_url_references};

    #[test]
    fn internal_target_follows_the_iri_grammar() {
        assert_eq!(internal_target("#base"), Some("base"));
        assert_eq!(internal_target("  #base  "), Some("base"));
        assert_eq!(
            internal_target("#base extra"),
            None,
            "only spaces may follow the fragment"
        );
        assert_eq!(internal_target("#base\t"), None, "a tab is not an id");
        assert_eq!(internal_target("#"), None, "an empty fragment is not an id");
        assert_eq!(internal_target("other.svg#q"), None);
    }

    #[test]
    fn url_references_are_rewritten_one_by_one() {
        assert_eq!(
            map_url_references("url(#a)", |id| format!("{id}-x")),
            "url(#a-x)"
        );
        assert_eq!(
            map_url_references("url(#a) url( #b )", |id| format!("{id}-x")),
            "url(#a-x) url( #b )",
            "only an immediate `#` is an internal IRI"
        );
        assert_eq!(map_url_references("none", |id| id.to_string()), "none");
    }
}
