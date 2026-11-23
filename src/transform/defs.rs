//! Definitions are drawn where they are **referenced**, not where they are written.
//!
//! A `<pattern>`, `<marker>`, `<mask>`, `<filter>` or gradient inside `<defs>` is
//! evaluated in the user space of the element that points at it. The same is true
//! of their numbers: a tile of `20` user units is 20 user units *of the referrer*,
//! so it must follow the referrer's `data-scale`, not the policy that happens to be
//! inherited where the `<defs>` block sits.
//!
//! The composer used to recurse into `<defs>` like any other subtree, so a
//! definition written outside the `data-scale` group inherited the root's `None`
//! and stayed put while the shapes that referenced it scaled: the tile kept
//! `width="20"` while the body became `M 0 49.600 …`, the body fell outside its own
//! tile window and the fill vanished (round-data/R46).
//!
//! A definition may be referenced by shapes under *different* policies, and then it
//! really is several definitions: the renderer evaluates the numbers once per
//! referrer. Picking the widest policy would hand the narrower referrer numbers
//! scaled on the wrong axes, which is the same defect in a smaller size, so instead
//! the subtree is **copied once per policy**, the ids get a policy suffix, and every
//! reference is repointed at the copy for its own policy. A single-policy definition
//! — the shape the defect actually takes — is never copied, so untouched input keeps
//! its bytes.
//!
//! Copies follow the reference graph transitively: if `<pattern id="p">` fills with
//! `url(#g)`, then `g` is evaluated under the same policies as `p`, so `g` is copied
//! too and each `p` copy points at its own `g` copy. Otherwise `p`'s `x` copy would
//! still use the `xy`-scaled gradient and the fix would only reach half way.
//!
//! A definition nothing references gets no policy of its own: it falls back to
//! the `data-scale` inherited where it is written, which for the root — the shape
//! this defect takes — is `None` (translate only). `url(#…)` text inside `<style>`
//! is not parsed by this tool, so such a definition also counts as unreferenced —
//! an existing, documented limitation, not a new failure mode.

#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::{BTreeSet, HashMap, HashSet};

use super::hoist::hoist_outside;
use super::links::{internal_target, map_url_references};
use super::{parse_policy, ScalePolicy};
use crate::dom::{Element, Node};

/// Tags whose numbers are read in the user space of whoever references them.
const DEFINITION_TAGS: [&str; 7] = [
    "pattern",
    "marker",
    "mask",
    "filter",
    "linearGradient",
    "radialGradient",
    "clipPath",
];

pub(super) fn is_definition(element: &Element) -> bool {
    DEFINITION_TAGS.contains(&element.name.as_str())
}

/// What the transform walk needs to know about definitions.
pub(super) struct DefsPlan {
    /// Definition id -> the one policy it is evaluated under (copies included).
    policies: HashMap<String, ScalePolicy>,
}

impl DefsPlan {
    /// The policy a definition element's numbers are read in, if a referrer
    /// decides it.
    ///
    /// `None` means "no referrer says anything": the walk then falls back to the
    /// `data-scale` inherited where the definition is written, which is exactly
    /// what this composer did before the round started. Definitions placed at the
    /// root (the shape the defect takes) inherit `None` either way, while a
    /// definition written inside a scale group keeps behaving as it always did —
    /// so untouched input keeps its bytes.
    pub(super) fn policy_of(&self, element: &Element) -> Option<ScalePolicy> {
        if !is_definition(element) {
            return None;
        }
        element
            .attr("id")
            .and_then(|id| self.policies.get(id).copied())
    }
}

/// Copy every definition that more than one policy references, then repoint the
/// references. Definitions with zero or one referrer are left structurally alone.
pub(super) fn split(root: &mut Element) -> DefsPlan {
    let refs = referrers(root);
    // A nested definition that is also referenced from outside its enclosing
    // definition cannot be instantiated correctly inside each outer copy: the
    // outside reference needs its own copy at the top level. Move it there once
    // (round-data/R49).
    hoist_outside(root, &refs);
    let mut policies: HashMap<String, ScalePolicy> = HashMap::new();
    let mut wanted: HashMap<String, Vec<ScalePolicy>> = HashMap::new();
    let mut names: HashMap<(String, ScalePolicy), String> = HashMap::new();
    let mut used = ids_of(root);

    let mut ordered_ids: Vec<&String> = refs.keys().collect();
    ordered_ids.sort();
    for id in ordered_ids {
        let set = &refs[id];
        let mut ordered: Vec<ScalePolicy> = set.iter().copied().collect();
        ordered.sort_by_key(|policy| rank(*policy));
        let Some((first, rest)) = ordered.split_first() else {
            continue;
        };
        policies.insert(id.clone(), *first);
        if rest.is_empty() {
            continue;
        }
        for policy in rest {
            let name = unique_name(id, *policy, &mut used);
            names.insert((id.clone(), *policy), name.clone());
            policies.insert(name, *policy);
        }
        wanted.insert(id.clone(), ordered);
    }

    let plan = DefsPlan { policies };
    if !wanted.is_empty() {
        let mut copies = Copies {
            wanted,
            names,
            used,
        };
        split_children(root, ScalePolicy::None, &mut copies, None);
    }
    plan
}

/// The policies of the elements that reference each definition, transitively.
pub(super) fn referrers(root: &Element) -> HashMap<String, BTreeSet<ScalePolicy>> {
    let mut direct: HashMap<String, BTreeSet<ScalePolicy>> = HashMap::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    collect(root, ScalePolicy::None, None, &mut direct, &mut edges);

    // `refs[D]` is `direct[D]` plus the referrer set of every definition that
    // points at `D`. Only grows, so the fixpoint terminates.
    let mut refs = direct;
    loop {
        let mut changed = false;
        for (owner, target) in &edges {
            let Some(inherited) = refs.get(owner).cloned() else {
                continue;
            };
            if inherited.is_empty() {
                continue;
            }
            let entry = refs.entry(target.clone()).or_default();
            for policy in inherited {
                changed |= entry.insert(policy);
            }
        }
        if !changed {
            break;
        }
    }
    refs
}

fn collect(
    element: &Element,
    inherited: ScalePolicy,
    enclosing: Option<&str>,
    direct: &mut HashMap<String, BTreeSet<ScalePolicy>>,
    edges: &mut Vec<(String, String)>,
) {
    let policy = element
        .attr("data-scale")
        .map(parse_policy)
        .unwrap_or(inherited);
    let owner = element
        .attr("id")
        .filter(|_| is_definition(element))
        .map(str::to_string);
    // References inside a definition belong to its whole subtree, not to the
    // leaf element that happens to spell the `url(#…)`.
    let enclosing = owner.as_deref().or(enclosing);
    for (name, value) in &element.attrs {
        for id in referenced_ids(name, value) {
            match enclosing {
                // The reference belongs to a definition: it is evaluated under
                // whatever policies that definition is instantiated with.
                Some(owner) => edges.push((owner.to_string(), id)),
                None => {
                    direct.entry(id).or_default().insert(policy);
                }
            }
        }
    }
    for child in &element.children {
        if let Node::Element(child) = child {
            collect(child, policy, enclosing, direct, edges);
        }
    }
}

/// Every internal id `value` points at: `url(#a)` anywhere, or a bare `#a` link.
fn referenced_ids(name: &str, value: &str) -> Vec<String> {
    if name == "href" || name == "xlink:href" {
        return link_of_text(value);
    }
    let mut ids = Vec::new();
    let mut rest = value;
    while let Some(position) = rest.find("url(") {
        let tail = &rest[position + 4..];
        let Some(close) = tail.find(')') else { break };
        let inner = &tail[..close];
        if let Some(id) = inner.strip_prefix('#') {
            if !id.is_empty() {
                ids.push(id.to_string());
            }
        }
        rest = &tail[close + 1..];
    }
    ids
}

/// A bare link value (`href="#a"`) as an id, if the renderer can follow it.
fn link_of_text(value: &str) -> Vec<String> {
    if !value.starts_with('#') && !value.starts_with(' ') {
        return Vec::new();
    }
    internal_target(value)
        .map(|id| vec![id.to_string()])
        .unwrap_or_default()
}

/// Everything needed to copy definition subtrees and repoint references.
struct Copies {
    wanted: HashMap<String, Vec<ScalePolicy>>,
    names: HashMap<(String, ScalePolicy), String>,
    used: HashSet<String>,
}

impl Copies {
    /// The id `id` has inside a definition evaluated under `policy`.
    fn name(&self, id: &str, policy: ScalePolicy) -> String {
        self.names
            .get(&(id.to_string(), policy))
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    /// The id `id` has inside a **non-first** copy for `policy`. A definition
    /// nothing references was never named by `split`, but it still cannot keep
    /// its id in two copies, so it gets one here (round-data/R49).
    fn name_or_allocate(&mut self, id: &str, policy: ScalePolicy) -> String {
        if let Some(name) = self.names.get(&(id.to_string(), policy)) {
            return name.clone();
        }
        let name = unique_name(id, policy, &mut self.used);
        self.names.insert((id.to_string(), policy), name.clone());
        name
    }
}

fn split_children(
    parent: &mut Element,
    inherited: ScalePolicy,
    copies: &mut Copies,
    narrow: Option<(ScalePolicy, bool)>,
) {
    let mut index = 0;
    while index < parent.children.len() {
        let Node::Element(current) = &parent.children[index] else {
            index += 1;
            continue;
        };
        let mut current = current.clone();
        if let Some((policy, first)) = narrow {
            // Inside a copy of an outer definition every nested definition is
            // instantiated under **this copy's** policy only. Splitting it by its
            // full referrer set would emit one copy per outer copy and leave
            // duplicate ids behind (round-data/R49).
            if is_definition(&current) {
                if let Some(id) = current.attr("id").map(str::to_string) {
                    let name = if first {
                        copies.name(&id, policy)
                    } else {
                        copies.name_or_allocate(&id, policy)
                    };
                    if name != id {
                        current.set_attr("id", name);
                    }
                }
            }
            rewrite_references(&mut current, policy, copies);
            split_children(&mut current, policy, copies, Some((policy, first)));
            parent.children[index] = Node::Element(current);
            index += 1;
            continue;
        }
        let policy = current
            .attr("data-scale")
            .map(parse_policy)
            .unwrap_or(inherited);
        let id = current.attr("id").map(str::to_string);
        let list = id.as_ref().and_then(|id| copies.wanted.get(id)).cloned();
        match list {
            Some(list) if list.len() > 1 => {
                let mut insert_at = index;
                for (position, policy) in list.iter().enumerate() {
                    let mut version = current.clone();
                    if position > 0 {
                        let id = id.clone().unwrap_or_default();
                        version.set_attr("id", copies.name(&id, *policy));
                    }
                    rewrite_references(&mut version, *policy, copies);
                    split_children(
                        &mut version,
                        *policy,
                        copies,
                        Some((*policy, position == 0)),
                    );
                    if position == 0 {
                        parent.children[index] = Node::Element(version);
                    } else {
                        insert_at += 1;
                        parent.children.insert(insert_at, Node::Element(version));
                    }
                }
            }
            _ => {
                rewrite_references(&mut current, policy, copies);
                split_children(&mut current, policy, copies, None);
                parent.children[index] = Node::Element(current);
            }
        }
        index += 1;
    }
}

/// Point every reference of `element` at the copy valid for its own `policy`.
fn rewrite_references(element: &mut Element, policy: ScalePolicy, copies: &Copies) {
    for (name, value) in &mut element.attrs {
        if name == "href" || name == "xlink:href" {
            if let Some(id) = internal_target(value) {
                let renamed = copies.name(id, policy);
                if renamed != id {
                    *value = format!("#{renamed}");
                }
            }
            continue;
        }
        if !value.contains("url(") {
            continue;
        }
        let rewritten = map_url_references(value, |id| copies.name(id, policy));
        if rewritten != *value {
            *value = rewritten;
        }
    }
}

/// Every `id` the document already uses, so a copy can never shadow one.
fn ids_of(root: &Element) -> HashSet<String> {
    let mut ids = HashSet::new();
    fn walk(element: &Element, ids: &mut HashSet<String>) {
        if let Some(id) = element.attr("id") {
            ids.insert(id.to_string());
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                walk(child, ids);
            }
        }
    }
    walk(root, &mut ids);
    ids
}

/// `{id}--svg-grid-{tag}`, plus `-2`, `-3`, … when that name is already taken.
fn unique_name(id: &str, policy: ScalePolicy, used: &mut HashSet<String>) -> String {
    let base = format!("{id}--svg-grid-{}", tag(policy));
    if used.insert(base.clone()) {
        return base;
    }
    for counter in 2.. {
        let candidate = format!("{base}-{counter}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("the counter reaches a free name")
}

/// The suffix that names a policy. Deterministic, and readable in a diff.
fn tag(policy: ScalePolicy) -> &'static str {
    match policy {
        ScalePolicy::Xy => "xy",
        ScalePolicy::X => "x",
        ScalePolicy::Y => "y",
        ScalePolicy::Position => "position",
        ScalePolicy::None => "none",
        ScalePolicy::Scale => "scale",
        ScalePolicy::ScaleX => "scalex",
        ScalePolicy::ScaleY => "scaley",
    }
}

/// A fixed order so the copy that keeps the original id — and the ids of all the
/// other copies — do not depend on hash iteration.
fn rank(policy: ScalePolicy) -> u8 {
    match policy {
        ScalePolicy::Xy => 0,
        ScalePolicy::X => 1,
        ScalePolicy::Y => 2,
        ScalePolicy::Position => 3,
        ScalePolicy::None => 4,
        ScalePolicy::Scale => 5,
        ScalePolicy::ScaleX => 6,
        ScalePolicy::ScaleY => 7,
    }
}

/// Sorted view of the reference sets, for tests and for the report's determinism.
#[cfg(test)]
fn sorted(refs: &HashMap<String, BTreeSet<ScalePolicy>>) -> BTreeMap<String, Vec<ScalePolicy>> {
    refs.iter()
        .map(|(id, set)| (id.clone(), set.iter().copied().collect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    fn refs_of(svg: &str) -> BTreeMap<String, Vec<ScalePolicy>> {
        let root = parse(svg).unwrap();
        sorted(&referrers(&root))
    }

    #[test]
    fn a_referrers_policy_is_the_policy_of_the_referring_element() {
        let refs = refs_of(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs><pattern id="p" width="2" height="2"/></defs>
                  <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        );
        assert_eq!(refs["p"], vec![ScalePolicy::X]);
    }

    #[test]
    fn a_definition_inherits_the_policies_of_the_definitions_that_reference_it() {
        // `d` is used under `xy` and `x`; `base` is used only by `d`, so it is
        // instantiated under the same two policies (round-data/R46 constraint 1).
        let refs = refs_of(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <linearGradient id="base" x1="0" x2="100"/>
                    <linearGradient id="d" href="#base" x1="10"/>
                    <pattern id="p" width="2" height="2"><rect fill="url(#d)"/></pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        );
        assert_eq!(refs["p"], vec![ScalePolicy::Xy, ScalePolicy::X]);
        assert_eq!(refs["d"], vec![ScalePolicy::Xy, ScalePolicy::X]);
        assert_eq!(refs["base"], vec![ScalePolicy::Xy, ScalePolicy::X]);
    }

    #[test]
    fn an_unreferenced_definition_has_no_policy_at_all() {
        let refs = refs_of(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs><pattern id="p" width="2" height="2"/></defs>
                </svg>"##,
        );
        assert!(refs.is_empty(), "nothing references the pattern: {refs:?}");
    }

    #[test]
    fn a_single_policy_definition_is_never_copied() {
        let mut root = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs><pattern id="p" width="2" height="2"/></defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        )
        .unwrap();
        let plan = split(&mut root);
        assert_eq!(plan.policies.len(), 1, "no copy must be created");
        assert_eq!(plan.policies["p"], ScalePolicy::Xy);
    }

    #[test]
    fn copies_follow_the_reference_graph_and_get_unique_ids() {
        let mut root = parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
                  <defs>
                    <linearGradient id="g" x1="0" x2="100"/>
                    <pattern id="p" width="2" height="2"><rect fill="url(#g)"/></pattern>
                  </defs>
                  <g data-scale="xy"><rect width="10" height="10" fill="url(#p)"/></g>
                  <g data-scale="x"><rect width="10" height="10" fill="url(#p)"/></g>
                </svg>"##,
        )
        .unwrap();
        let plan = split(&mut root);
        let text = crate::dom::serialize(&root).unwrap();
        assert!(
            text.contains(r#"<pattern id="p" "#)
                && text.contains(r#"<pattern id="p--svg-grid-x" "#),
            "both copies must exist: {text}"
        );
        assert!(
            text.contains(r#"<linearGradient id="g--svg-grid-x""#),
            "the gradient must be copied too: {text}"
        );
        assert!(
            text.contains(r##"<pattern id="p--svg-grid-x" width="2" height="2"><rect fill="url(#g--svg-grid-x)"/>"##),
            "the x copy must point at its own gradient: {text}"
        );
        assert_eq!(plan.policies["p--svg-grid-x"], ScalePolicy::X);
    }
}
