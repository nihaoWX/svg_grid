//! The `data-scale` protocol executor: rewrite every coordinate of an input panel so
//! it lands in its grid cell (`ARCHITECTURE/tag-protocol.md`).
//!
//! Split by responsibility (R14, `ARCHITECTURE.md` §跨领域约定 体积口径):
//! this module walks the tree and dispatches, [`lengths`] owns the scale factors and
//! the lengths they apply to, [`primitives`] rewrites the plain shapes, [`paths`]
//! rewrites `<path d>`, [`patterns`] decides what a `<pattern>` body's numbers mean
//! (its `patternContentUnits`/`viewBox`, inherited along `href`), [`units`]
//! parses the `*Units` attributes the way the renderer does, [`defs`] decides which
//! policy a definition is evaluated under (and copies it when several disagree),
//! [`hoist`] lifts a nested definition that is also referenced from outside, and
//! [`links`] resolves what a paint server inherits from its `href` target,
//! [`filters`] rewrites the user lengths inside a `<filter>`'s primitives under
//! `primitiveUnits` (R58).

mod defs;
mod dispatch;
mod domains;
mod filters;
mod hoist;
mod lengths;
mod lights;
mod links;
mod paths;
mod patterns;
mod primitives;
mod transforms;
mod units;
mod walk;

use crate::dom::Element;

pub(crate) use lengths::{note_overflow, scaled_finite};
pub(crate) use units::{marker_units_are_user_space, units_of, Units};
pub(crate) use walk::{parse_policy, transform_svg};

/// The `*Units` facts `crate::normalize` needs, without exposing the whole walk.
///
/// The normaliser rewrites the *input* before any panel geometry exists, so it
/// cannot ask "which policy references this definition"; what it does need is the
/// units a definition attribute is written in, resolved along `href` exactly like
/// the renderer (`links::resolved_units`). Kept as one opaque bundle so the index
/// is built once per document instead of once per element.
pub(crate) struct Definitions(links::IdIndex);

impl Definitions {
    pub(crate) fn collect(root: &Element) -> Self {
        Self(links::IdIndex::collect(root))
    }

    /// The units of `attr` on `element`, following the `href` chain.
    pub(crate) fn units(&self, element: &Element, attr: &str, default: Units) -> Units {
        links::resolved_units(element, &self.0, attr, default)
    }
}

/// Whether `value` is a `viewBox` the renderer honours (`patterns::valid_view_box`).
pub(crate) fn is_view_box(value: &str) -> bool {
    patterns::valid_view_box(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum ScalePolicy {
    Xy,
    X,
    Y,
    Position,
    None,
    /// Scale the numbers but never translate them. Only the composer produces it,
    /// and only for a `<pattern>`/`<marker>` body: those coordinates are *tile
    /// local* (the renderer places them relative to the tile origin or the marker
    /// vertex), so the box may move to its cell while the body must stay where it
    /// is — moving both would push the body out of its own window and the fill
    /// would vanish silently (round-data/R38). `data-scale` never yields this
    /// variant.
    Scale,
    /// The same "scale but never translate", restricted to one axis: a definition
    /// referenced from a `data-scale="x"` group must scale its x numbers only, or a
    /// square motif becomes an oblong one (round-data/R46).
    ScaleX,
    /// The `y` half of [`ScalePolicy::ScaleX`].
    ScaleY,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_r58;
#[cfg(test)]
mod tests_r59;
#[cfg(test)]
mod tests_r60;
#[cfg(test)]
mod tests_r61;
#[cfg(test)]
mod tests_r62;
#[cfg(test)]
mod tests_r62b;
#[cfg(test)]
mod tests_r62c;
#[cfg(test)]
mod tests_r64;
#[cfg(test)]
mod tests_r67;
