//! `<path d>`: the axis map a policy imposes on path data, and the verbatim
//! pass-through (W7) when the data cannot be parsed (see `super`).

use crate::dom::Element;
use crate::error::Result;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

use super::lengths::{length_scale_factor, LengthAxis};
use super::ScalePolicy;

pub(super) fn rewrite_path(
    element: &mut Element,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) -> Result<()> {
    let Some(d) = element.attr("d").map(str::to_string) else {
        return Ok(());
    };
    let map = path_axis_map(policy, source, target);
    match crate::path::rewrite(&d, &map) {
        Ok(rewritten) => element.set_attr("d", rewritten),
        // A `d` the parser cannot understand (malformed structure, a missing
        // argument, a `nan`/`inf` number, …) is no longer fail-closed: the path is
        // passed through byte-for-byte and the run keeps its exit code and output
        // bytes. A browser is lenient about such data, and rejecting the whole
        // figure for it would narrow what used to work. Structural garbage in
        // *other* attributes (`rect x="abc"`) still fails closed.
        Err(error) => passthroughs.record(
            index,
            &element.name,
            "d",
            format!("unparseable path data {:?} ({error})", excerpt(&d, 40)),
        ),
    }
    Ok(())
}

/// A short, single-line excerpt of an attribute value for a warning, so two
/// *different* offenders of the same kind stay distinguishable (identical ones
/// still collapse in [`Passthroughs`]).
pub(super) fn excerpt(value: &str, max: usize) -> String {
    let mut out: String = value.chars().take(max).collect();
    if value.chars().count() > max {
        out.push('…');
    }
    out
}

/// The axis mapping a `data-scale` policy imposes on a `<path d>`, derived from
/// the same source/target boxes as [`map_point`]: absolute coordinates follow
/// `map_point`, while the `A` radii use the *length* factors
/// ([`length_scale_factor`]) so a `position` policy leaves sizes untouched —
/// exactly the rule `<rect width/height>` follow.
pub(super) fn path_axis_map(
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
) -> crate::path::AxisMap {
    let sx = if source.width.abs() < f32::EPSILON {
        1.0
    } else {
        target.width / source.width
    };
    let sy = if source.height.abs() < f32::EPSILON {
        1.0
    } else {
        target.height / source.height
    };

    let (x_scale, y_scale, x_offset, y_offset) = match policy {
        ScalePolicy::Xy | ScalePolicy::Position => {
            (sx, sy, target.x - source.x * sx, target.y - source.y * sy)
        }
        ScalePolicy::X => (sx, 1.0, target.x - source.x * sx, target.y - source.y),
        ScalePolicy::Y => (1.0, sy, target.x - source.x, target.y - source.y * sy),
        ScalePolicy::None => (1.0, 1.0, target.x - source.x, target.y - source.y),
    };

    crate::path::AxisMap {
        x_scale,
        y_scale,
        x_offset,
        y_offset,
        x_length: length_scale_factor(policy, source, target, LengthAxis::X).unwrap_or(1.0),
        y_length: length_scale_factor(policy, source, target, LengthAxis::Y).unwrap_or(1.0),
    }
}
