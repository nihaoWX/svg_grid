//! `<path d="...">` geometry for the composer: axis-wise coordinate rewriting,
//! flattening to polyline subpaths, and an axis-aligned bounding box.
//!
//! ## Twin implementation (deliberate duplication)
//!
//! This module (`src/path/`) is the **twin** of `convert/src/path.rs`: the
//! converter keeps its own copy of the path parser/flattener *on purpose* — the
//! user decided against a shared crate, so both crates own one. The two must stay
//! semantically identical: **when you change path semantics here, read the other
//! side too**, and keep the flattening tolerance ([`DEFAULT_TOLERANCE`], measured
//! in *source* user units) exactly the same value in both files.
//!
//! ## The R5 rewrite contract (see `ARCHITECTURE.md` §已定型的决定 / `ARCHITECTURE/tag-protocol.md`)
//!
//! [`rewrite`] re-expresses a `d` under the composer's `data-scale` policy:
//!
//! * `M`/`L`/`T`/`C`/`S`/`Q`: both `x` and `y` are coordinates, mapped per axis.
//! * `H` moves only `x`, `V` only `y`.
//! * `A`: `rx` on the **X** axis, `ry` on the **Y** axis, the end point is a
//!   coordinate pair — and the **`x-axis-rotation` and both flags (`large-arc` /
//!   `sweep`) are copied byte-for-byte**, never reformatted.
//! * Lower-case commands are relative: their deltas are *vectors*, so `dx`/`dy`
//!   scale by the per-axis factor (no translation); `A`'s `rx`/`ry` scale the
//!   same way and its rotation/flags still do not move.
//! * Implicit repeats are supported: `M0,0 1,1 2,2` is a moveto followed by
//!   linetos (and likewise for the repeated argument groups of `L`/`C`/`S`/`Q`/`T`).
//! * SVG number syntax is accepted: exponents (`1e-5`), a sign glued to the
//!   previous number (`10-5` is `10, -5`), and any mix of commas and whitespace.
//!
//! **Fidelity boundary.** A rotated elliptical arc (`rotation != 0`) under a
//! **non-uniform** coordinate scale (`|sx| != |sy|`) is **flattened into a
//! polyline**: a non-uniform scale turns that arc into an ellipse arc whose axes
//! are no longer rotated by `rotation`, so keeping it as a literal `A` would draw
//! the *wrong shape*. Every other case is rewritten faithfully (`A` stays `A`).
//!
//! ## Layout
//!
//! * [`parse`] — tokenising `d` into `(command, arguments)` pairs.
//! * [`rewrite`] — the axis-wise coordinate rewrite above.
//! * [`flatten`] — flattening curves/arcs to polyline subpaths and the flattened
//!   bounding box.

mod flatten;
mod parse;
mod rewrite;

pub use flatten::bbox;
// `flatten`/`Subpath` keep their original `crate::path::` spelling. They are not
// referenced outside this module (only `bbox`/`rewrite`/`AxisMap` are), but the
// re-export is kept so the module's surface matches the pre-split `src/path.rs`.
#[allow(unused_imports)]
pub use flatten::{flatten, Subpath};
pub use rewrite::{rewrite, AxisMap};

/// Default flattening tolerance, in the *source* user units of the panel.
///
/// The composed reproduction scales a panel by ~10x into the final 600-dpi PNG
/// (a 504-unit canvas maps to ≈4880 px), so an error of 0.02 user units is
/// ≈0.2 px on the final image — below the ~1 px a reader could notice.
/// **Same value and rationale as `convert/src/path.rs`; the two must not drift.**
pub const DEFAULT_TOLERANCE: f32 = 0.02;

#[cfg(test)]
mod tests;
