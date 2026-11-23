//! `<path>` → `<polyline>`/`<polygon>` rewriting, plus `<clipPath>` rectification.
//!
//! Straight commands are exact; curves are flattened to within
//! [`DEFAULT_TOLERANCE`] source units, after which the (already baked) transform
//! is applied to every point.
//!
//! # Draw form: fill, stroke and closure decide the elements
//!
//! One `<path>` may carry both a fill and a stroke, and the two are painted
//! separately, so the emitted shape depends on whether the path is *filled*,
//! whether it is *stroked* (`stroke::is_stroked`) and how many subpaths it has:
//!
//! | filled | stroked | subpaths | output |
//! |---|---|---|---|
//! | yes | no | 1–2 | one `<polygon>`: the subpaths concatenated and explicitly closed |
//! | yes | no | ≥3 | one flattened `<path>` (`M`/`L`/`Z` per subpath) |
//! | yes | yes | 1 | a `<polygon>` if the subpath is closed, else a `<polyline>` (unchanged from before) |
//! | yes | yes | 2 | one **fill-only** `<polygon>` (concatenated + closed) plus one **stroke-only** shape per subpath |
//! | yes | yes | ≥3 | one **fill-only** flattened `<path>` plus one **stroke-only** shape per subpath |
//! | no | any | any | one `<polyline>` per subpath |
//!
//! A **filled** path is a single fill region, so all its subpaths (a glyph's outer
//! contour *and* its holes) belong to one shape — one shape per subpath would fill
//! every counter (`O`, `0`, `8`, `B`) solid. A `<path>` carrying both paints is
//! **split**, because a shape that kept the source `stroke` would paint the
//! bridges between contours.
//!
//! ## Why three or more subpaths cannot be one `<polygon>`
//!
//! Concatenating the contours into a `<polygon>` introduces a bridge from one
//! contour's end to the next contour's start, and the polygon's **implicit
//! closing edge** (last point → first point) adds one more. With **two**
//! subpaths those two bridges are the same segment walked in opposite directions,
//! so they enclose zero area and the winding number is unchanged — that is the
//! only case the old "the bridges cancel" argument covered, and the old tests
//! (`tests.rs` N=2 cases) exercised exactly that. With **three or more**
//! subpaths the bridges plus the implicit edge form a real ring whose interior
//! gets winding ±1, so every hole inside it is filled solid.
//!
//! Therefore a fill region with three or more subpaths is emitted as a flattened
//! `<path>` with one explicit `M … L … Z` contour per subpath: there is no
//! implicit edge between contours, each contour is closed on its own, and the
//! source `fill-rule` (`nonzero` or `evenodd`) applies to the whole region with
//! exactly the source semantics. `resvg` supports `<path>` fully; every point is
//! already flattened and transformed, so no curves or transforms survive.
//!
//! Because a `<polygon>`/`<polyline>` cannot paint the fill along the concatenated
//! outline and the stroke along each subpath at once, the several-subpath stroked
//! case is split into two kinds of element, each with the *other* paint removed
//! thoroughly (an inline `style` beats a presentation attribute, and the other
//! paint could also be inherited — so both a `style` edit and a
//! `stroke="none"`/`fill="none"` attribute are needed).
//!
//! The stage is split by responsibility: this facade re-exports the two entry
//! points under their original `shapes::` paths, so no caller changes.

mod clip;
mod path_form;

pub(super) use clip::rectify_clip_paths;
pub(super) use path_form::path_shapes;

#[cfg(test)]
mod tests;
