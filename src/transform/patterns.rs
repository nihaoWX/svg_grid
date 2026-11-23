//! What a `<pattern>` body's numbers mean: `patternContentUnits`, `viewBox` and
//! `patternUnits`, including the ones inherited along the `href` chain
//! (round-data/R40, corrected in round-data/R41).
//!
//! The composer has to know this before it rewrites any child of a `<pattern>`,
//! because the same literal `0.1` can mean three different things:
//!
//! * `0.1` user units of the tile — the `patternContentUnits="userSpaceOnUse"`
//!   default. The tile scales with the panel, so the body scales too.
//! * `0.1` × the **referencing element's box** — `patternContentUnits` set to
//!   `objectBoundingBox`. That box already scales with the panel, so scaling the
//!   fraction again double-scales the motif (measured in R39: ink 4 instead of 16
//!   at s=0.5).
//! * `0.1` **viewBox units** — a `viewBox` on the pattern maps its own coordinate
//!   system onto the tile, and the tile already follows the panel, so the numbers
//!   must stay verbatim as well (measured in R40: `2 2 6 6` became `4 4 12 12`,
//!   xor 1792 against the reference).
//!
//! All three attributes follow the `href`/`xlink:href` chain in usvg
//! (`resolve_pattern_attr`), first explicit occurrence winning. The resolution
//! below mirrors `usvg::parser::paint_server`:
//!
//! * `patternContentUnits` and `patternUnits` are read with `convert_units`:
//!   a typed parse, so only the exact token counts (no trimming, case sensitive);
//!   a value that does not parse falls back to the attribute's **default** —
//!   `userSpaceOnUse` for the content units, `objectBoundingBox` for the units.
//! * `viewBox` counts only when it actually parses (`svgtypes::ViewBox`).
//! * `HrefIter` stops at the first link that does not resolve to an element of
//!   this document (a dangling `#id`, `other.svg#q`, a bare URL), and
//!   `resolve_pattern_attr` stops at the first link that is not a `<pattern>`.
//!   Inheritance then ends and the defaults above apply, so an unresolvable
//!   chain is **user space**, not verbatim: the renderer really does scale such a
//!   body (measured in R41: xor 448 / ink 96 against ink 384 when left verbatim).
//!
//! Duplicate ids resolve to the **last** definition: usvg fills its id map with
//! `links.insert(...)` in document order (`parser/svgtree/parse.rs`).

use std::collections::{HashMap, HashSet};

use crate::dom::{Element, Node};

/// The attributes that decide how a `<pattern>` body and tile are read, read off
/// one `<pattern>`.
struct PatternSource {
    href: Option<String>,
    content_units: Option<String>,
    view_box: Option<String>,
    pattern_units: Option<String>,
}

/// Document-wide `<pattern>` inheritance data, collected once per transform.
///
/// Looking the chain up in a map keeps the walker free of borrow trouble: a
/// `<pattern>` may point at one defined later in the document, which a purely
/// recursive descent could not see.
#[derive(Default)]
pub(super) struct PatternIndex {
    patterns: HashMap<String, PatternSource>,
}

impl PatternIndex {
    /// Index every `<pattern>` reachable through `href`.
    ///
    /// Duplicate ids keep the **last** occurrence, like the renderer's id map.
    pub(super) fn collect(root: &Element) -> Self {
        let mut index = Self::default();
        index.walk(root);
        index
    }

    fn walk(&mut self, element: &Element) {
        if element.name == "pattern" {
            if let Some(id) = element.attr("id") {
                self.patterns.insert(
                    id.to_string(),
                    PatternSource {
                        href: link_of(element).map(str::to_string),
                        content_units: element.attr("patternContentUnits").map(str::to_string),
                        view_box: element.attr("viewBox").map(str::to_string),
                        pattern_units: element.attr("patternUnits").map(str::to_string),
                    },
                );
            }
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                self.walk(child);
            }
        }
    }
}

/// The first explicit value of each inherited attribute along the `href` chain.
#[derive(Default)]
struct Resolved {
    content_units: Option<String>,
    view_box: Option<String>,
    pattern_units: Option<String>,
}

impl Resolved {
    fn complete(&self) -> bool {
        self.content_units.is_some() && self.view_box.is_some() && self.pattern_units.is_some()
    }

    fn absorb(&mut self, source: &PatternSource) {
        if self.content_units.is_none() {
            self.content_units.clone_from(&source.content_units);
        }
        if self.view_box.is_none() {
            self.view_box.clone_from(&source.view_box);
        }
        if self.pattern_units.is_none() {
            self.pattern_units.clone_from(&source.pattern_units);
        }
    }
}

fn resolve(element: &Element, index: &PatternIndex) -> Resolved {
    let mut resolved = Resolved {
        content_units: element.attr("patternContentUnits").map(str::to_string),
        view_box: element.attr("viewBox").map(str::to_string),
        pattern_units: element.attr("patternUnits").map(str::to_string),
    };
    let mut link = link_of(element);
    let mut visited: HashSet<String> = HashSet::new();

    while !resolved.complete() {
        let Some(href) = link else { break };
        // Not an id of this document (another file, a bare URL, garbage): usvg's
        // `HrefIter` cannot follow it, so inheritance stops and the defaults win.
        let Some(id) = internal_target(href) else {
            break;
        };
        if !visited.insert(id.to_string()) {
            // `href` cycle. The renderer does not loop either, so decide with what
            // has been collected so far.
            break;
        }
        // A dangling id, or an id owned by a non-`<pattern>` element: `href_iter`
        // stops at the first link that is not a pattern, so the chain ends here.
        let Some(source) = index.patterns.get(id) else {
            break;
        };
        resolved.absorb(source);
        link = source.href.as_deref();
    }

    resolved
}

/// Whether the body of `element` (a `<pattern>`) must keep its numbers verbatim:
/// bbox fractions or viewBox units, as opposed to user units of the tile.
pub(super) fn pattern_body_is_verbatim(element: &Element, index: &PatternIndex) -> bool {
    let resolved = resolve(element, index);
    resolved.content_units.as_deref() == Some("objectBoundingBox")
        || resolved.view_box.as_deref().is_some_and(valid_view_box)
}

/// Whether the tile rectangle of `element` is expressed in user units, i.e. it
/// follows the panel and must be rewritten. `patternUnits` defaults to
/// `objectBoundingBox`, whose fractions are relative to the (already scaled) box.
pub(super) fn pattern_units_is_user_space(element: &Element, index: &PatternIndex) -> bool {
    resolve(element, index).pattern_units.as_deref() == Some("userSpaceOnUse")
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
