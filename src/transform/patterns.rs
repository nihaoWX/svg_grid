//! What a `<pattern>` body's numbers mean: `patternContentUnits` and `viewBox`,
//! including the ones inherited along the `href` chain (round-data/R40).
//!
//! The composer has to know this before it rewrites any child of a `<pattern>`,
//! because the same literal `0.1` means three different things:
//!
//! * `0.1` user units of the tile — the `patternContentUnits="userSpaceOnUse"`
//!   default. The tile scales with the panel, so the body scales too
//!   ([`PatternBody::UserSpace`]).
//! * `0.1` × the **referencing element's box** — `patternContentUnits` set to
//!   `objectBoundingBox`. That box already scales with the panel, so scaling the
//!   fraction again double-scales the motif (measured in R39: ink 4 instead of 16
//!   at s=0.5).
//! * `0.1` **viewBox units** — a `viewBox` on the pattern maps its own coordinate
//!   system onto the tile, and the tile already follows the panel, so the numbers
//!   must stay verbatim as well (measured in R40: `2 2 6 6` became `4 4 12 12`,
//!   xor 1792 against the reference).
//!
//! Both attributes follow the `href`/`xlink:href` chain in usvg
//! (`resolve_pattern_attr`), with the first explicit occurrence winning. The
//! resolution below mirrors that rule, including its strictly typed reading of the
//! values: only the exact token `objectBoundingBox` and a `viewBox` that actually
//! parses select [`PatternBody::Verbatim`]. Anything else — a missing attribute, a
//! different case, surrounding whitespace, an unknown token or an empty string —
//! falls back to user space, exactly like `usvg::parser::paint_server::convert_units`
//! (`attribute(...).unwrap_or(default)`, no trimming, case sensitive).
//!
//! A chain the composer cannot resolve (a dangling `#id`, a link to another
//! document, a cycle) is treated as [`PatternBody::Verbatim`]: leaving the body
//! alone can never *add* a scaling the renderer did not ask for, while guessing
//! user space could double-scale an inherited bbox body.

use std::collections::{HashMap, HashSet};

use crate::dom::{Element, Node};

/// How the renderer interprets the numbers inside a `<pattern>` body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternBody {
    /// Relative to a box that already follows the panel (bbox fractions or viewBox
    /// units): the body keeps every number.
    Verbatim,
    /// User units of the tile: scale them about the tile origin, never translate.
    UserSpace,
}

/// The attributes that decide [`PatternBody`], read off one `<pattern>`.
struct PatternSource {
    href: Option<String>,
    content_units: Option<String>,
    view_box: Option<String>,
}

/// Document-wide `<pattern>` inheritance data, collected once per transform.
///
/// Looking the chain up in a map keeps the walker free of borrow trouble: a
/// `<pattern>` may point at one defined later in the document, which a purely
/// recursive descent could not see.
#[derive(Default)]
pub(super) struct PatternIndex {
    patterns: HashMap<String, PatternSource>,
    /// Every id in the document, so a link to a non-`<pattern>` can be told apart
    /// from a dangling one.
    ids: HashSet<String>,
}

impl PatternIndex {
    /// Index every element id plus the pattern attributes reachable through `href`.
    ///
    /// Duplicate ids keep the first occurrence, like the renderer's id map.
    pub(super) fn collect(root: &Element) -> Self {
        let mut index = Self::default();
        index.walk(root);
        index
    }

    fn walk(&mut self, element: &Element) {
        if element.name == "pattern" {
            if let Some(id) = element.attr("id") {
                if self.ids.insert(id.to_string()) {
                    self.patterns.insert(
                        id.to_string(),
                        PatternSource {
                            href: link_of(element).map(str::to_string),
                            content_units: element.attr("patternContentUnits").map(str::to_string),
                            view_box: element.attr("viewBox").map(str::to_string),
                        },
                    );
                }
            }
        } else if let Some(id) = element.attr("id") {
            self.ids.insert(id.to_string());
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                self.walk(child);
            }
        }
    }
}

/// Whether the body of `element` (a `<pattern>`) must keep its numbers verbatim.
pub(super) fn pattern_body_is_verbatim(element: &Element, index: &PatternIndex) -> bool {
    matches!(pattern_body(element, index), PatternBody::Verbatim)
}

fn pattern_body(element: &Element, index: &PatternIndex) -> PatternBody {
    let mut content_units = element.attr("patternContentUnits");
    let mut view_box = element.attr("viewBox");
    let mut link = link_of(element);
    let mut visited: HashSet<&str> = HashSet::new();

    loop {
        if content_units.is_some() && view_box.is_some() {
            break;
        }
        let Some(href) = link else { break };
        let Some(id) = internal_target(href) else {
            // Not an id in this document (another file, a bare URL, garbage): the
            // chain cannot be resolved here, so play safe and add no scaling.
            return PatternBody::Verbatim;
        };
        if !visited.insert(id) {
            // `href` cycle. The renderer does not loop either, so stop here and
            // decide with what has been collected so far.
            break;
        }
        let Some(source) = index.patterns.get(id) else {
            if index.ids.contains(id) {
                // The link points at some other element: `href_iter` stops at the
                // first non-`<pattern>`, so the chain ends here as well.
                break;
            }
            return PatternBody::Verbatim;
        };
        if content_units.is_none() {
            content_units = source.content_units.as_deref();
        }
        if view_box.is_none() {
            view_box = source.view_box.as_deref();
        }
        link = source.href.as_deref();
    }

    if content_units == Some("objectBoundingBox") || view_box.is_some_and(valid_view_box) {
        PatternBody::Verbatim
    } else {
        PatternBody::UserSpace
    }
}

/// The raw `href` of a `<pattern>`, in either spelling.
fn link_of(element: &Element) -> Option<&str> {
    element.attr("href").or_else(|| element.attr("xlink:href"))
}

/// The `#id` a link points at, if it stays inside this document.
fn internal_target(href: &str) -> Option<&str> {
    href.trim().strip_prefix('#').filter(|id| !id.is_empty())
}

/// Whether `value` parses as the `viewBox` attribute (`svgtypes::ViewBox`): four
/// numbers `x y width height` separated by whitespace and/or a single comma each,
/// with a positive, finite size. Trailing garbage after the fourth number is
/// ignored, exactly like `svgtypes`.
fn valid_view_box(value: &str) -> bool {
    let mut rest = value;
    let mut numbers = [0.0_f32; 4];
    for slot in &mut numbers {
        let Some((number, tail)) = next_number(rest) else {
            return false;
        };
        *slot = number;
        rest = tail;
    }
    numbers.iter().all(|number| number.is_finite()) && numbers[2] > 0.0 && numbers[3] > 0.0
}

/// Read one SVG number from the front of `input` (the `svgtypes` grammar) and
/// return it with the remainder after its separator.
fn next_number(input: &str) -> Option<(f32, &str)> {
    let rest = input.trim_start_matches(is_space);
    let bytes = rest.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        i += 1;
    }
    let integer_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i > integer_start;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let fraction_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        digits |= i > fraction_start;
    }
    if !digits {
        return None;
    }
    // An exponent is optional, but `e`/`E` followed by `m`/`x` starts a unit
    // instead (`1em`, `1ex`); an exponent marker with no digits after it makes the
    // whole number invalid, like `svgtypes`.
    if i < bytes.len()
        && matches!(bytes[i], b'e' | b'E')
        && !matches!(bytes.get(i + 1), Some(b'm') | Some(b'x'))
    {
        let mut j = i + 1;
        if matches!(bytes.get(j), Some(b'+') | Some(b'-')) {
            j += 1;
        }
        let exponent_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j == exponent_start {
            return None;
        }
        i = j;
    }
    let number = rest[..i].parse::<f32>().ok()?;
    let tail = rest[i..].trim_start_matches(is_space);
    let tail = tail.strip_prefix(',').unwrap_or(tail);
    Some((number, tail))
}

fn is_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r')
}

#[cfg(test)]
mod tests {
    use super::valid_view_box;

    #[test]
    fn view_box_accepts_svgtypes_separators() {
        for value in [
            "0 0 20 20",
            "-20 30 100 500",
            "0,0,20,20",
            "0 0,20,20",
            "10-20 30 40",
            "0 0 20 20 trailing junk",
            "0 0 20 20e0",
        ] {
            assert!(valid_view_box(value), "{value:?} should parse");
        }
    }

    #[test]
    fn view_box_rejects_invalid_geometry() {
        for value in [
            "",
            "0 0 20",
            "0 0 0 20",
            "0 0 20 -1",
            "0 0 20 20e",
            "a b c d",
            "0 0 20px 20",
            "0 0 inf 20",
            "0 0 . 20",
        ] {
            assert!(!valid_view_box(value), "{value:?} must not parse");
        }
    }
}
