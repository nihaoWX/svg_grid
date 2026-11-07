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
//! | yes | no | any | one `<polygon>`: every subpath concatenated and explicitly closed |
//! | yes | yes | 1 | a `<polygon>` if the subpath is closed, else a `<polyline>` (unchanged from before) |
//! | yes | yes | >1 | one **fill-only** `<polygon>` (concatenated + closed) plus one **stroke-only** shape per subpath |
//! | no | any | any | one `<polyline>` per subpath |
//!
//! A **filled** path is a single fill region, so all its subpaths (a glyph's outer
//! contour *and* its holes) belong to one `<polygon>` — one polygon per subpath
//! would fill every counter (`O`, `0`, `8`, `B`) solid. Concatenating them,
//! however, introduces *bridges* between the contours, so each concatenated
//! subpath is **explicitly closed** (see below) and a stroked path is **split**:
//! a `<polygon>` that kept the source `stroke` would paint those bridges.
//!
//! ## Why each concatenated subpath is explicitly closed
//!
//! A `<polygon>` implicitly draws an edge from its last point back to its first,
//! and the concatenated list also has an edge from one subpath's end to the next
//! subpath's start. For an *open* subpath (`M` without `Z` — exactly the
//! pdftocairo glyph shape) those bridge edges are real edges that change the
//! winding number and can add or erase fill. Appending the first point to every
//! open subpath makes each outbound bridge and its return the same segment walked
//! in opposite directions, so their net winding contribution is zero: the fill
//! region then matches the source's "all subpaths share one `fill-rule`"
//! semantics, and nested rings keep their holes.
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
