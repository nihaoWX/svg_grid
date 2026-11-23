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

use super::lengths::{length_scale_factor, map_point, LengthAxis};
use super::units::{convert_units, Units};
use super::ScalePolicy;
use crate::dom::{Element, Node};
use crate::error::Result;
use crate::geom::Rect;

/// `patternUnits` defaults to `objectBoundingBox` (`usvg-0.47.0/src/parser/
/// paint_server.rs:168`).
const PATTERN_UNITS_DEFAULT: Units = Units::ObjectBoundingBox;
/// `patternContentUnits` defaults to `userSpaceOnUse` (`paint_server.rs:169`).
const PATTERN_CONTENT_UNITS_DEFAULT: Units = Units::UserSpaceOnUse;

/// The attributes that decide how a `<pattern>` body and tile are read, read off
/// one `<pattern>`.
struct PatternSource {
    href: Option<String>,
    content_units: Option<String>,
    view_box: Option<String>,
    pattern_units: Option<String>,
    x: Option<String>,
    y: Option<String>,
    width: Option<String>,
    height: Option<String>,
}

impl PatternSource {
    fn new(element: &Element) -> Self {
        Self {
            href: link_of(element).map(str::to_string),
            content_units: element.attr("patternContentUnits").map(str::to_string),
            view_box: element.attr("viewBox").map(str::to_string),
            pattern_units: element.attr("patternUnits").map(str::to_string),
            x: element.attr("x").map(str::to_string),
            y: element.attr("y").map(str::to_string),
            width: element.attr("width").map(str::to_string),
            height: element.attr("height").map(str::to_string),
        }
    }

    /// The tile and unit attributes this pattern declares itself.
    fn attr(&self, name: &str) -> Option<&String> {
        match name {
            "x" => self.x.as_ref(),
            "y" => self.y.as_ref(),
            "width" => self.width.as_ref(),
            "height" => self.height.as_ref(),
            "patternContentUnits" => self.content_units.as_ref(),
            "patternUnits" => self.pattern_units.as_ref(),
            "viewBox" => self.view_box.as_ref(),
            _ => None,
        }
    }
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
    /// Duplicate ids keep the **last** occurrence, like the renderer's id map
    /// (`usvg-0.47.0/src/parser/svgtree/parse.rs:117-121`).
    pub(super) fn collect(root: &Element) -> Self {
        let mut index = Self::default();
        index.walk(root);
        index
    }

    fn walk(&mut self, element: &Element) {
        if element.name == "pattern" {
            if let Some(id) = element.attr("id") {
                self.patterns
                    .insert(id.to_string(), PatternSource::new(element));
            }
        }
        for child in &element.children {
            if let Node::Element(child) = child {
                self.walk(child);
            }
        }
    }
}

/// The first explicit value of each inherited attribute along the `href` chain,
/// with the unit attributes already parsed by [`convert_units`].
#[derive(Default)]
struct Resolved {
    content_units: Option<Units>,
    view_box: Option<String>,
    pattern_units: Option<Units>,
}

impl Resolved {
    fn of(element: &Element) -> Self {
        let mut resolved = Self::default();
        resolved.absorb_owner(element);
        resolved
    }

    /// Record what `element` itself declares.
    ///
    /// A declared-but-invalid unit value still shadows the inherited one:
    /// `resolve_pattern_attr` returns the first link that merely *has* the
    /// attribute, and only then does `convert_units` parse it — an unparseable
    /// value falls back to the default instead of continuing down the chain.
    fn absorb_owner(&mut self, element: &Element) {
        if self.content_units.is_none() {
            if let Some(value) = element.attr("patternContentUnits") {
                self.content_units =
                    Some(convert_units(Some(value), PATTERN_CONTENT_UNITS_DEFAULT));
            }
        }
        if self.view_box.is_none() {
            self.view_box = element.attr("viewBox").map(str::to_string);
        }
        if self.pattern_units.is_none() {
            if let Some(value) = element.attr("patternUnits") {
                self.pattern_units = Some(convert_units(Some(value), PATTERN_UNITS_DEFAULT));
            }
        }
    }

    fn absorb_source(&mut self, source: &PatternSource) {
        if self.content_units.is_none() {
            if let Some(value) = source.content_units.as_deref() {
                self.content_units =
                    Some(convert_units(Some(value), PATTERN_CONTENT_UNITS_DEFAULT));
            }
        }
        if self.view_box.is_none() {
            self.view_box.clone_from(&source.view_box);
        }
        if self.pattern_units.is_none() {
            if let Some(value) = source.pattern_units.as_deref() {
                self.pattern_units = Some(convert_units(Some(value), PATTERN_UNITS_DEFAULT));
            }
        }
    }

    fn complete(&self) -> bool {
        // `viewBox` is optional by design: its absence is itself an answer, so it
        // must be looked for along the whole chain before the body is judged.
        self.content_units.is_some() && self.pattern_units.is_some()
    }

    fn content_units(&self) -> Units {
        self.content_units.unwrap_or(PATTERN_CONTENT_UNITS_DEFAULT)
    }

    fn pattern_units(&self) -> Units {
        self.pattern_units.unwrap_or(PATTERN_UNITS_DEFAULT)
    }
}

fn resolve(element: &Element, index: &PatternIndex) -> Resolved {
    let mut resolved = Resolved::of(element);
    let mut link = link_of(element);
    let mut visited: HashSet<String> = HashSet::new();

    while !resolved.complete() || resolved.view_box.is_none() {
        let Some(href) = link else { break };
        // Not an id of this document (another file, a bare URL, garbage, or an
        // IRI the grammar rejects): `HrefIter` cannot follow it, so inheritance
        // stops and the defaults win.
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
        resolved.absorb_source(source);
        link = source.href.as_deref();
    }

    resolved
}

/// Whether the body of `element` (a `<pattern>`) must keep its numbers verbatim:
/// bbox fractions or viewBox units, as opposed to user units of the tile.
pub(super) fn pattern_body_is_verbatim(element: &Element, index: &PatternIndex) -> bool {
    let resolved = resolve(element, index);
    resolved.content_units() == Units::ObjectBoundingBox
        || resolved.view_box.as_deref().is_some_and(valid_view_box)
}

/// Whether the tile rectangle of `element` is expressed in user units, i.e. it
/// follows the panel and must be rewritten. `patternUnits` defaults to
/// `objectBoundingBox`, whose fractions are relative to the (already scaled) box.
pub(super) fn pattern_units_is_user_space(element: &Element, index: &PatternIndex) -> bool {
    resolve(element, index).pattern_units() == Units::UserSpaceOnUse
}

/// Rewrite the tile of a `<pattern>` whose **resolved** `patternUnits` is user
/// space, so every number of the tile follows the panel.
///
/// Unlike a `<rect>`, a missing `x`/`y`/`width`/`height` here is not a zero: the
/// renderer resolves each one along the `href` chain (`usvg` calls
/// `resolve_number` for every attribute), so the value that has to be scaled may
/// live on another element — and the units that apply to it are the *resolved*
/// units, no matter which element spelled the number. The scaled value is written
/// onto this pattern, which shadows the inherited one with exactly the same
/// meaning (round-data/R43; R41 fixed the older bug where the `0` defaults were
/// written out and erased an inherited tile).
pub(super) fn rewrite_pattern_tile(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    patterns: &PatternIndex,
) -> Result<()> {
    // Resolve first, mutate after: the values may live on another pattern.
    let x = tile_number(element, patterns, "x");
    let y = tile_number(element, patterns, "y");
    let width = tile_number(element, patterns, "width");
    let height = tile_number(element, patterns, "height");

    // `x` and `y` move as a **point** (the tile is placed in the cell), unlike a
    // plain length, so they are mapped together. `usvg` defaults a missing tile
    // coordinate to zero, and zero maps to the panel offset under `Xy`; writing the
    // mapped value is therefore equivalent to the default, not a change of meaning.
    if x.is_some() || y.is_some() {
        let (mapped_x, mapped_y) =
            map_point(x.unwrap_or(0.0), y.unwrap_or(0.0), policy, source, target);
        element.set_attr("x", crate::format_number(mapped_x));
        element.set_attr("y", crate::format_number(mapped_y));
    }
    for (attr, value, axis) in [
        ("width", width, LengthAxis::X),
        ("height", height, LengthAxis::Y),
    ] {
        let Some(value) = value else { continue };
        if !value.is_finite() {
            continue;
        }
        if let Some(factor) = length_scale_factor(policy, source, target, axis) {
            element.set_attr(attr, crate::format_number(value * factor));
        }
    }
    Ok(())
}

/// A tile coordinate resolved along the `href` chain, if it is a plain number.
fn tile_number(element: &Element, patterns: &PatternIndex, name: &str) -> Option<f32> {
    resolved_attr(element, patterns, name).and_then(|raw| raw.parse::<f32>().ok())
}

/// The value the renderer reads for `name` on this `<pattern>`: the element's own
/// attribute first, then the first `href`-chain link that declares it.
fn resolved_attr(element: &Element, index: &PatternIndex, name: &str) -> Option<String> {
    if let Some(value) = element.attr(name) {
        return Some(value.to_string());
    }
    let mut link = link_of(element).map(str::to_string);
    let mut visited: HashSet<String> = HashSet::new();
    while let Some(href) = link {
        let id = internal_target(&href)?;
        if !visited.insert(id.to_string()) {
            return None;
        }
        let source = index.patterns.get(id)?;
        if let Some(value) = source.attr(name) {
            return Some(value.clone());
        }
        link = source.href.clone();
    }
    None
}

/// The raw `href` of a `<pattern>`, in either spelling.
fn link_of(element: &Element) -> Option<&str> {
    element.attr("href").or_else(|| element.attr("xlink:href"))
}

/// The `#id` a link points at, if the link is an internal IRI that the renderer
/// can follow, parsed like `svgtypes::Stream::parse_iri`:
///
/// * leading and trailing **spaces** are skipped (`' '` only: a tab belongs to
///   the id, so `"#\tbase"` names a different, unresolvable element);
/// * the fragment runs to the first literal space;
/// * whatever follows it must be spaces only.
///
/// A URI with a scheme or a path (`other.svg#q`, `http://example.com/#p`) is not
/// an internal target, and neither is an empty fragment (round-data/R43).
fn internal_target(href: &str) -> Option<&str> {
    let fragment = href.trim_start_matches(' ').strip_prefix('#')?;
    let end = fragment.find(' ').unwrap_or(fragment.len());
    let (id, tail) = fragment.split_at(end);
    if id.is_empty() || !tail.bytes().all(|byte| byte == b' ') {
        return None;
    }
    Some(id)
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
