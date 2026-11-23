//! Unit attributes resolved the way the reference renderer resolves them.
//!
//! Every `*Units` attribute in SVG is a **typed** attribute, and `usvg-0.47.0`
//! reads it as one (`src/parser/paint_server.rs:230-233`):
//!
//! ```text
//! pub(crate) fn convert_units(node: SvgNode, name: AId, def: Units) -> Units {
//!     let node = resolve_attr(node, name);
//!     node.attribute(name).unwrap_or(def)
//! }
//! ```
//!
//! `node.attribute::<Units>(...)` only accepts the exact tokens
//! `userSpaceOnUse` / `objectBoundingBox` — no trimming, no case folding — and
//! anything else (an unknown word, a stray space, an empty value) falls back to
//! that attribute's default. So `maskContentUnits=" objectBoundingBox "` is a
//! *user space* mask for the renderer, and `" userSpaceOnUse "` is an
//! `objectBoundingBox` one: the opposite of what a trimmed comparison concludes
//! (round-data/R43).
//!
//! Defaults, all read off `usvg-0.47.0`:
//!
//! | attribute            | default             | where                                  |
//! |----------------------|---------------------|----------------------------------------|
//! | `patternUnits`       | `objectBoundingBox` | `paint_server.rs:168`                  |
//! | `patternContentUnits`| `userSpaceOnUse`    | `paint_server.rs:169`                  |
//! | `gradientUnits`      | `objectBoundingBox` | `paint_server.rs:56`, `:93`            |
//! | `filterUnits`        | `objectBoundingBox` | `filter.rs:170`                        |
//! | `primitiveUnits`     | `userSpaceOnUse`    | `filter.rs:171`                        |
//! | `maskUnits`          | `objectBoundingBox` | `mask.rs:20-30` (no `href` chain)      |
//! | `maskContentUnits`   | `userSpaceOnUse`    | `mask.rs:20-30` (no `href` chain)      |
//! | `markerUnits`        | `strokeWidth`       | `marker.rs:222` (literal comparison)   |
//!
//! `markerUnits` is special: `usvg` does not parse it as [`Units`] at all, it
//! compares the attribute to the literal `userSpaceOnUse` and treats everything
//! else as the default `strokeWidth` (`marker.rs:222`), so it gets its own
//! predicate here.

use crate::dom::Element;

/// The two values SVG defines for the `*Units` attributes that take a coordinate
/// system (`usvg::tree::Units`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

/// `convert_units`: parse `value` type-wise, falling back to `default`.
pub(super) fn convert_units(value: Option<&str>, default: Units) -> Units {
    match value {
        Some("userSpaceOnUse") => Units::UserSpaceOnUse,
        Some("objectBoundingBox") => Units::ObjectBoundingBox,
        _ => default,
    }
}

/// The units of `element`'s own `attr`, with `default` when the attribute is
/// missing or does not parse.
pub(super) fn units_of(element: &Element, attr: &str, default: Units) -> Units {
    convert_units(element.attr(attr), default)
}

/// `markerUnits="userSpaceOnUse"` holds only for the exact token; every other
/// value means the default `strokeWidth` (`usvg-0.47.0/src/parser/marker.rs:222`).
pub(super) fn marker_units_are_user_space(element: &Element) -> bool {
    element.attr("markerUnits") == Some("userSpaceOnUse")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_units_accepts_only_the_two_exact_tokens() {
        assert_eq!(
            convert_units(Some("userSpaceOnUse"), Units::ObjectBoundingBox),
            Units::UserSpaceOnUse
        );
        assert_eq!(
            convert_units(Some("objectBoundingBox"), Units::UserSpaceOnUse),
            Units::ObjectBoundingBox
        );
    }

    #[test]
    fn convert_units_falls_back_to_the_default_for_every_other_spelling() {
        // `usvg` parses these type-wise and does not trim or fold case, so each of
        // these is invalid and the attribute's default applies (round-data/R43).
        for value in [
            Some(" userSpaceOnUse "),
            Some("UserSpaceOnUse"),
            Some("userspaceonuse"),
            Some(" objectBoundingBox "),
            Some(""),
            Some("foo"),
            None,
        ] {
            assert_eq!(
                convert_units(value, Units::ObjectBoundingBox),
                Units::ObjectBoundingBox,
                "{value:?} is not a valid unit and must fall back to the default"
            );
            assert_eq!(
                convert_units(value, Units::UserSpaceOnUse),
                Units::UserSpaceOnUse,
                "{value:?} is not a valid unit and must fall back to the default"
            );
        }
    }

    #[test]
    fn marker_units_is_a_literal_comparison() {
        let mut element = crate::dom::parse(
            r##"<svg xmlns="http://www.w3.org/2000/svg"><marker id="k" markerUnits="userSpaceOnUse"/></svg>"##,
        )
        .unwrap();
        let marker = match &mut element.children[0] {
            crate::dom::Node::Element(child) => child,
            _ => unreachable!(),
        };
        assert!(marker_units_are_user_space(marker));
        marker.set_attr("markerUnits", " userSpaceOnUse");
        assert!(
            !marker_units_are_user_space(marker),
            "a spaced value is invalid and means the strokeWidth default"
        );
        marker.remove_attr("markerUnits");
        assert!(!marker_units_are_user_space(marker));
    }
}
