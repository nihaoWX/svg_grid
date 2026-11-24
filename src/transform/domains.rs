//! The second axis: the **content domain** a definition's numbers are read in.
//!
//! R46 decides that a definition is evaluated in the user space of whoever
//! references it, and copies it once per `data-scale` policy. That is only half of
//! the story. A body the renderer already maps — a `<pattern>` with a valid
//! `viewBox` or `patternContentUnits="objectBoundingBox"`, a `<marker>` with a
//! valid `viewBox` — is drawn in its **own** units: viewBox units, or fractions of
//! the referencing element's box. Those units already follow the panel, so a
//! definition referenced from such a body is *not* in panel user space: its tile,
//! coordinates and body stay verbatim. Passing the panel policy through the edge
//! scales it a second time and the motif is doubled (measured: an inner pattern
//! tile `8` became `16` and its body `4` became `8`, xor 12160 / maxdiff 255 at
//! s=2; with the inner definition kept verbatim, xor 0 — round-data/R62).
//!
//! So a definition is instantiated in a [`Slot`]: either the panel user space under
//! one policy (the R46 axis), or the verbatim domain of a mapped body. The set of
//! slots is propagated along the reference graph exactly like R46's policy set,
//! except that a verbatim body maps **every** incoming slot to [`Slot::Verbatim`]
//! — the outer policy decides where the body's box lands, not what its numbers
//! mean. A definition two domains share is cloned once per slot, still with the
//! `--svg-grid-<tag>` naming, and a single-slot definition is never copied.
//!
//! Whether a body is mapped is a property of the definition *itself*, not of its
//! referrers: a `<pattern>` with a `viewBox` stays a viewBox body under every
//! policy. That is why this module answers the question from the element (and the
//! inherited units along `href`, see [`super::patterns`]) and never from the walk's
//! current policy.

use super::patterns::{self, PatternIndex};
use super::ScalePolicy;
use crate::dom::Element;

/// How one definition is instantiated: the two axes R46 and R62.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum Slot {
    /// Referenced from panel geometry, or from a body drawn in the panel's user
    /// space: the numbers are panel user units and follow this policy.
    UserSpace(ScalePolicy),
    /// Referenced from inside a body the renderer already maps (`viewBox` or
    /// `objectBoundingBox`): the numbers are that body's units and stay verbatim.
    Verbatim,
}

impl Slot {
    /// A fixed order so the copy that keeps the original id — and the ids of all
    /// the other copies — do not depend on hash iteration. Verbatim sorts last,
    /// after every user-space policy, so adding the domain never renames an
    /// existing R46 copy.
    pub(super) fn rank(self) -> u8 {
        match self {
            Slot::UserSpace(policy) => policy_rank(policy),
            Slot::Verbatim => 8,
        }
    }

    /// The suffix that names a copy. Deterministic, and readable in a diff.
    pub(super) fn tag(self) -> &'static str {
        match self {
            Slot::UserSpace(policy) => policy_tag(policy),
            Slot::Verbatim => "verbatim",
        }
    }

    pub(super) fn is_verbatim(self) -> bool {
        matches!(self, Slot::Verbatim)
    }
}

/// The suffix that names a user-space policy copy.
fn policy_tag(policy: ScalePolicy) -> &'static str {
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

/// The rank of a user-space policy, unchanged from R46.
fn policy_rank(policy: ScalePolicy) -> u8 {
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

/// Whether `element`'s body is drawn in a domain the renderer already maps, so a
/// definition referenced from it must stay verbatim.
///
/// Only the two constructs with a measured defect are listed: a `<pattern>` whose
/// resolved content units are `objectBoundingBox` or whose resolved `viewBox`
/// parses, and a `<marker>` with its own valid `viewBox`. An unmapped body
/// (`patternContentUnits="userSpaceOnUse"` without a `viewBox`, an objectBoundingBox
/// mask/clipPath body) keeps the R46 propagation: its numbers *are* panel units
/// (round-data/R62 registers the obb clipPath/mask residue as untouched).
pub(super) fn body_is_verbatim(element: &Element, patterns: &PatternIndex) -> bool {
    if element.name == "pattern" {
        return patterns::pattern_body_is_verbatim(element, patterns);
    }
    element.name == "marker" && marker_body_is_verbatim(element)
}

/// Whether a `<marker>`'s body is already expanded to its viewport by the
/// renderer (`usvg`'s `ViewBox::to_transform`), so its numbers must not scale.
pub(super) fn marker_body_is_verbatim(element: &Element) -> bool {
    element
        .attr("viewBox")
        .is_some_and(patterns::valid_view_box)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    fn root(svg: &str) -> Element {
        parse(svg).unwrap()
    }

    #[test]
    fn a_view_box_pattern_body_is_verbatim() {
        let root = root(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
            <pattern id="p" width="4" height="4" viewBox="0 0 8 8"><rect width="8" height="8"/></pattern>
        </svg>"#,
        );
        let index = PatternIndex::collect(&root);
        let pattern = root.children.iter().find_map(|node| match node {
            crate::dom::Node::Element(element) if element.name == "pattern" => Some(element),
            _ => None,
        });
        assert!(body_is_verbatim(pattern.unwrap(), &index));
    }

    #[test]
    fn an_unmapped_pattern_body_is_not_verbatim() {
        let root = root(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
            <pattern id="p" width="4" height="4" patternContentUnits="userSpaceOnUse"><rect width="8" height="8"/></pattern>
        </svg>"#,
        );
        let index = PatternIndex::collect(&root);
        let pattern = root.children.iter().find_map(|node| match node {
            crate::dom::Node::Element(element) if element.name == "pattern" => Some(element),
            _ => None,
        });
        assert!(!body_is_verbatim(pattern.unwrap(), &index));
    }

    #[test]
    fn a_view_box_marker_body_is_verbatim_and_a_bare_marker_is_not() {
        let root = root(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
            <marker id="a" viewBox="0 0 10 10"/><marker id="b"/>
        </svg>"#,
        );
        let index = PatternIndex::collect(&root);
        let markers: Vec<&Element> = root
            .children
            .iter()
            .filter_map(|node| match node {
                crate::dom::Node::Element(element) if element.name == "marker" => Some(element),
                _ => None,
            })
            .collect();
        assert!(body_is_verbatim(markers[0], &index));
        assert!(!body_is_verbatim(markers[1], &index));
    }

    #[test]
    fn the_verbatim_slot_sorts_after_every_user_space_policy() {
        let mut slots = vec![
            Slot::Verbatim,
            Slot::UserSpace(ScalePolicy::Xy),
            Slot::UserSpace(ScalePolicy::ScaleY),
        ];
        slots.sort_by_key(|slot| slot.rank());
        assert_eq!(
            slots,
            vec![
                Slot::UserSpace(ScalePolicy::Xy),
                Slot::UserSpace(ScalePolicy::ScaleY),
                Slot::Verbatim,
            ]
        );
        assert_eq!(Slot::Verbatim.tag(), "verbatim");
        assert_eq!(Slot::UserSpace(ScalePolicy::Xy).tag(), "xy");
    }
}
