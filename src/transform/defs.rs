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

use std::collections::{BTreeSet, HashMap, HashSet};

use super::domains::{self, Slot};
use super::hoist::hoist_outside;
use super::links::{internal_target, map_url_references, IdIndex};
use super::patterns::PatternIndex;
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
    /// Definition id -> the one slot it is instantiated in (copies included).
    slots: HashMap<String, Slot>,
}

impl DefsPlan {
    /// The slot an element's numbers are read in, if a referrer decides it.
    /// `Slot::Verbatim` means the walk must leave the whole subtree untouched.
    ///
    /// `reference_only` marks an element that is **only drawn through a reference**:
    /// one of [`DEFINITION_TAGS`], or any element written inside `<defs>` or inside
    /// another definition's body. That is the population whose `id` a `<use href>`
    /// (or a `url(#…)`) can point at, and a `<rect id="r">` in `<defs>` is scaled in
    /// the referrer's user space exactly like a `<pattern>` is (round-data/R64) — so
    /// the gate cannot be "is one of the seven definition tags" alone.
    ///
    /// `None` means "no referrer says anything": the walk then falls back to the
    /// `data-scale` inherited where the element is written, which is exactly what
    /// this composer did before the round started. Definitions placed at the root
    /// (the shape the defect takes) inherit `None` either way, while one written
    /// inside a scale group keeps behaving as it always did — so untouched input
    /// keeps its bytes.
    pub(super) fn slot_of(&self, element: &Element, reference_only: bool) -> Option<Slot> {
        if !is_definition(element) && !reference_only {
            return None;
        }
        element
            .attr("id")
            .and_then(|id| self.slots.get(id).copied())
    }
}

/// Copy every definition that more than one policy references, then repoint the
/// references. Definitions with zero or one referrer are left structurally alone.
pub(super) fn split(root: &mut Element) -> DefsPlan {
    let patterns = PatternIndex::collect(root);
    let links = IdIndex::collect(root);
    let refs = referrers(root, &patterns, &links);
    // A nested definition that is also referenced from outside its enclosing
    // definition cannot be instantiated correctly inside each outer copy: the
    // outside reference needs its own copy at the top level. Move it there once
    // (round-data/R49).
    hoist_outside(root, &refs);
    let mut slots: HashMap<String, Slot> = HashMap::new();
    let mut wanted: HashMap<String, Vec<Slot>> = HashMap::new();
    let mut names: HashMap<(String, Slot), String> = HashMap::new();
    let mut used = ids_of(root);

    let mut ordered_ids: Vec<&String> = refs.keys().collect();
    ordered_ids.sort();
    for id in ordered_ids {
        let set = &refs[id];
        let mut ordered: Vec<Slot> = set.iter().copied().collect();
        ordered.sort_by_key(|slot| slot.rank());
        let Some((first, rest)) = ordered.split_first() else {
            continue;
        };
        slots.insert(id.clone(), *first);
        if rest.is_empty() {
            continue;
        }
        for slot in rest {
            let name = unique_name(id, slot.tag(), &mut used);
            names.insert((id.clone(), *slot), name.clone());
            slots.insert(name, *slot);
        }
        wanted.insert(id.clone(), ordered);
    }

    if !wanted.is_empty() {
        let mut copies = Copies {
            wanted,
            names,
            used,
            slots: &slots,
            patterns: &patterns,
            links: &links,
        };
        split_children(root, Slot::UserSpace(ScalePolicy::None), &mut copies, None);
    }
    DefsPlan { slots }
}

/// The slots of the elements that reference each definition, transitively.
pub(super) fn referrers(
    root: &Element,
    patterns: &PatternIndex,
    links: &IdIndex,
) -> HashMap<String, BTreeSet<Slot>> {
    let mut gather = Gather::default();
    collect(
        root,
        ScalePolicy::None,
        None,
        false,
        patterns,
        links,
        &mut gather,
    );

    // `refs[D]` is `direct[D]` plus the referrer set of every definition that
    // points at `D`. A mapped body maps every incoming slot to `Verbatim` before
    // passing it on: the outer slot decides where the body lands, not what its
    // numbers mean (round-data/R62). Only grows, so the fixpoint terminates.
    let Gather {
        direct,
        edges,
        bodies,
    } = gather;
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
            let verbatim = bodies.get(owner).copied().unwrap_or(false);
            let entry = refs.entry(target.clone()).or_default();
            for slot in inherited {
                let slot = if verbatim { Slot::Verbatim } else { slot };
                changed |= entry.insert(slot);
            }
        }
        if !changed {
            break;
        }
    }
    refs
}

/// What one pass over the tree gathers: direct referrers, definition-to-definition
/// edges, and which definition ids have a body the renderer already maps.
#[derive(Default)]
struct Gather {
    direct: HashMap<String, BTreeSet<Slot>>,
    edges: Vec<(String, String)>,
    bodies: HashMap<String, bool>,
}

/// `in_defs` is true for anything written inside a `<defs>` block, which — like a
/// definition body — makes an `id` a possible reference target even on an ordinary
/// `<rect>`/`<g>` (round-data/R64).
fn collect(
    element: &Element,
    inherited: ScalePolicy,
    enclosing: Option<&str>,
    in_defs: bool,
    patterns: &PatternIndex,
    links: &IdIndex,
    gather: &mut Gather,
) {
    let policy = element
        .attr("data-scale")
        .map(parse_policy)
        .unwrap_or(inherited);
    // A `<rect id="r">` in `<defs>` is a reference target just like a
    // `<pattern id="p">`: it is never drawn where it sits, but instantiated in the
    // user space of whoever points at it (round-data/R64).
    let reference_only = in_defs || enclosing.is_some();
    let owner = element
        .attr("id")
        .filter(|_| is_definition(element) || reference_only)
        .map(str::to_string);
    if let Some(id) = owner.as_deref() {
        gather.bodies.insert(
            id.to_string(),
            domains::body_is_verbatim(element, patterns, links),
        );
    }
    // References inside a definition belong to its whole subtree, not to the
    // leaf element that happens to spell the `url(#…)`.
    let enclosing = owner.as_deref().or(enclosing);
    for (name, value) in &element.attrs {
        for id in referenced_ids(name, value) {
            match enclosing {
                // The reference belongs to a definition: it is evaluated under
                // whatever slots that definition is instantiated in.
                Some(owner) => gather.edges.push((owner.to_string(), id)),
                None => {
                    gather
                        .direct
                        .entry(id)
                        .or_default()
                        .insert(Slot::UserSpace(policy));
                }
            }
        }
    }
    // Children of a `<defs>` block (and of a nested `<defs>`) are reference-only too,
    // even when no enclosing definition owns them yet.
    let children_in_defs = in_defs || element.name == "defs";
    for child in &element.children {
        if let Node::Element(child) = child {
            collect(
                child,
                policy,
                enclosing,
                children_in_defs,
                patterns,
                links,
                gather,
            );
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
struct Copies<'a> {
    wanted: HashMap<String, Vec<Slot>>,
    names: HashMap<(String, Slot), String>,
    used: HashSet<String>,
    slots: &'a HashMap<String, Slot>,
    patterns: &'a PatternIndex,
    links: &'a IdIndex,
}

impl Copies<'_> {
    /// The id `id` has inside a definition evaluated under `slot`.
    fn name(&self, id: &str, slot: Slot) -> String {
        self.names
            .get(&(id.to_string(), slot))
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    /// The id `id` has inside a **non-first** copy for `slot`. A definition
    /// nothing references was never named by `split`, but it still cannot keep
    /// its id in two copies, so it gets one here (round-data/R49).
    fn name_or_allocate(&mut self, id: &str, slot: Slot) -> String {
        if let Some(name) = self.names.get(&(id.to_string(), slot)) {
            return name.clone();
        }
        let name = unique_name(id, slot.tag(), &mut self.used);
        self.names.insert((id.to_string(), slot), name.clone());
        name
    }

    /// The slot `element`'s own references are repointed with — which is also the
    /// slot its children inherit.
    ///
    /// A body the renderer maps decides this **before** the slot the element was
    /// instantiated in: a `<pattern>` with a `viewBox` is a tile scaled in panel
    /// user space, but everything reached from inside that body — `url(#…)` and
    /// `href` alike — lives in viewBox units. Choosing the slot here instead of at
    /// each `url()` is what makes the `href` edge follow the same rule as a paint
    /// reference (round-data/R62 group E). A definition instantiated verbatim and
    /// every other element keep the R46 rule (its `data-scale`, else the inherited
    /// slot), so untouched input keeps its bytes.
    fn out_slot(&self, element: &Element, inherited: Slot) -> Slot {
        if domains::body_is_verbatim(element, self.patterns, self.links) {
            return Slot::Verbatim;
        }
        if is_definition(element) {
            let verbatim = element
                .attr("id")
                .and_then(|id| self.slots.get(id))
                .is_some_and(|slot| slot.is_verbatim());
            if verbatim {
                return Slot::Verbatim;
            }
        }
        match element.attr("data-scale") {
            Some(value) if !inherited.is_verbatim() => Slot::UserSpace(parse_policy(value)),
            _ => inherited,
        }
    }
}

fn split_children(
    parent: &mut Element,
    inherited: Slot,
    copies: &mut Copies<'_>,
    narrow: Option<(Slot, bool)>,
) {
    let mut index = 0;
    while index < parent.children.len() {
        let Node::Element(current) = &parent.children[index] else {
            index += 1;
            continue;
        };
        let mut current = current.clone();
        // Read before any reference is rewritten: `body_is_verbatim` resolves the
        // original `href` chain. A mapped body belongs to the verbatim domain
        // whatever the slot around it says (round-data/R62).
        if let Some((slot, first)) = narrow {
            let out = copies.out_slot(&current, slot);
            // Inside a copy of an outer definition every nested definition is
            // instantiated under **this copy's** domain only. Splitting it by its
            // full referrer set would emit one copy per outer copy and leave
            // duplicate ids behind (round-data/R49).
            if is_definition(&current) {
                if let Some(id) = current.attr("id").map(str::to_string) {
                    let name = if first {
                        copies.name(&id, slot)
                    } else {
                        copies.name_or_allocate(&id, slot)
                    };
                    if name != id {
                        current.set_attr("id", name);
                    }
                }
            }
            rewrite_references(&mut current, out, copies);
            split_children(&mut current, out, copies, Some((out, first)));
            parent.children[index] = Node::Element(current);
            index += 1;
            continue;
        }
        let own = copies.out_slot(&current, inherited);
        let id = current.attr("id").map(str::to_string);
        let list = id.as_ref().and_then(|id| copies.wanted.get(id)).cloned();
        match list {
            Some(list) if list.len() > 1 => {
                let mut insert_at = index;
                for (position, slot) in list.iter().enumerate() {
                    let mut version = current.clone();
                    if position > 0 {
                        let id = id.clone().unwrap_or_default();
                        version.set_attr("id", copies.name(&id, *slot));
                    }
                    let out = copies.out_slot(&version, *slot);
                    rewrite_references(&mut version, out, copies);
                    split_children(&mut version, out, copies, Some((out, position == 0)));
                    if position == 0 {
                        parent.children[index] = Node::Element(version);
                    } else {
                        insert_at += 1;
                        parent.children.insert(insert_at, Node::Element(version));
                    }
                }
            }
            _ => {
                rewrite_references(&mut current, own, copies);
                split_children(&mut current, own, copies, None);
                parent.children[index] = Node::Element(current);
            }
        }
        index += 1;
    }
}

/// Point every reference of `element` at the copy valid for its own `slot`.
fn rewrite_references(element: &mut Element, slot: Slot, copies: &Copies<'_>) {
    for (name, value) in &mut element.attrs {
        if name == "href" || name == "xlink:href" {
            if let Some(id) = internal_target(value) {
                let renamed = copies.name(id, slot);
                if renamed != id {
                    *value = format!("#{renamed}");
                }
            }
            continue;
        }
        if !value.contains("url(") {
            continue;
        }
        let rewritten = map_url_references(value, |id| copies.name(id, slot));
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
fn unique_name(id: &str, tag: &str, used: &mut HashSet<String>) -> String {
    let base = format!("{id}--svg-grid-{tag}");
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
