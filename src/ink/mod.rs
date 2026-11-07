//! Ink bounding-box estimation for an input SVG, feeding the W6 warning.
//!
//! W6 ([`crate::warnings::describe_content_outside_canvas`]) compares each input's
//! *ink* bounding box with that input's own canvas box
//! ([`crate::geom::find_canvas_box`]) and warns when the two disagree by more than
//! a small tolerance. This module owns only the geometry: a pure
//! [`ink_bbox`] returning `(min_x, min_y, max_x, max_y)` in the root's user
//! coordinate system.
//!
//! ## Coverage (deliberately the same element set as `src/transform.rs`)
//!
//! * **Exact** axis-aligned boxes for `circle`, `rect`, `line`, `polyline`,
//!   `polygon`, `image` and `use`, including a half-`stroke-width` outward
//!   expansion when a stroke is actually painted (paint and width are inherited
//!   down the tree; an unpainted `stroke="none"` never expands).
//! * A **flattened** box for `path`: the `d` is flattened to polylines at
//!   [`crate::path::DEFAULT_TOLERANCE`] and the extent of the vertices is used
//!   (not the raw control-point hull). This is the same geometry the composer
//!   rewrites, so a dialect that draws everything as `<path>` is measured.
//! * A `<use>` whose `href` / `xlink:href` resolves to a `<defs>` element by
//!   `id` is measured as that element's box **translated by the `<use>`'s
//!   `x`/`y`**, once per reference (the same content may be pulled in several
//!   times at different offsets). `<defs>` content is otherwise *not* part of
//!   the normal traversal: it is not drawn where it sits, and an unreferenced
//!   definition must not be measured.
//! * A proportional **estimate** for `text` / `tspan`, reusing the panel-letter
//!   constants [`crate::LABEL_CAP_HEIGHT_RATIO`] / [`crate::LABEL_INK_WIDTH_RATIO`]
//!   (the same approach as `label_ink_box` in `lib.rs`) and honouring
//!   `font-size`, `text-anchor` and `dominant-baseline`. The per-character width
//!   ratio is an upper bound, so wide runs may be over-estimated: W6 errs on the
//!   side of speaking up.
//!
//! ## Element `transform`
//!
//! An element's *own* ink is measured in its local coordinate system and then
//! mapped through that element's `transform`: the four corners of the local box
//! are transformed and their axis-aligned bounding box (AABB) is unioned in. A
//! `transform` list is composed left to right, SVG-style. The supported
//! functions are `translate(tx [ty])`, `scale(sx [sy])`, `rotate(a)`,
//! `rotate(a cx cy)` and `matrix(a b c d e f)`. `rotate(-90 …)` — the canonical
//! form a converter leaves on a rotated text run — is therefore handled.
//!
//! Only the *element's own* `transform` is applied: an ancestor `<g transform=…>`
//! is **not** propagated to its descendants (every element is still measured
//! against the root's user coordinate system, exactly as before).
//!
//! A `transform` that cannot be evaluated — `skewX`/`skewY`, a malformed list,
//! or a list mixing a supported function with anything else — makes the
//! element's own geometry **ignored** (it is not measured at its untransformed
//! position), so an unmodelled transform can never invent an overflow.
//! Descendants are still visited and measured through their own transforms.
//!
//! Everything else is **ignored** — the contents of `<symbol>` / `<clipPath>` /
//! `<mask>` / … , `display:none`, gradients, `<title>` / `<desc>`. A `<use>` that
//! references a `<symbol>` or a nested `<svg>` is not measured either: the
//! composer does not carry those, and their viewport semantics are not modelled
//! here, so only its placement box is counted. A `<use>` whose `href` names an id
//! **absent from the document** is dangling and draws nothing, so it contributes
//! no ink at all. When a primitive cannot be evaluated we prefer to *miss* a
//! warning than to raise a false one.
//!
//! ## Layout
//!
//! * [`walk`] — the tree traversal, inherited [`walk::Ctx`], and the shared
//!   attribute helpers (this is where [`ink_bbox`] lives).
//! * [`bbox`] — the running box, the affine `transform`, and its `transform`
//!   attribute parser.
//! * [`shapes`] — the per-element *own* geometry (the W6 judge's "warning face").
//! * [`text`] — the proportional `text` / `tspan` estimate.
//! * [`uses`] — the `<defs>` / `<use>` instantiation.

mod bbox;
mod shapes;
mod text;
mod uses;
mod walk;

pub use walk::ink_bbox;

#[cfg(test)]
mod tests;
