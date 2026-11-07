//! Axis-wise coordinate rewriting of a `<path d>` under the `data-scale` policy.
//!
//! [`AxisMap`] is the mapping ([`rewrite`] is its application); the contract is
//! the R5 one spelled out in the module docs.

use crate::format_number;

use super::flatten::flatten_arc;
use super::parse::{resolve, tokenize};
use super::DEFAULT_TOLERANCE;

/// The per-axis mapping a `data-scale` policy applies to a `<path d>`.
///
/// Absolute coordinates map as `x' = x_scale * x + x_offset` (and likewise for
/// `y`); relative deltas are vectors and scale by `x_scale` / `y_scale` with
/// **no** offset; `A` radii use the unsigned size factors `x_length` / `y_length`
/// — the same "size" axis rule `<rect width/height>` get, so the `position`
/// policy leaves sizes untouched while absolute coordinates still remap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMap {
    pub x_scale: f32,
    pub y_scale: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub x_length: f32,
    pub y_length: f32,
}

impl AxisMap {
    fn map_point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.x_scale * x + self.x_offset,
            self.y_scale * y + self.y_offset,
        )
    }

    /// A relative delta is a vector: it scales by axis and is *not* translated.
    fn map_delta(&self, dx: f32, dy: f32) -> (f32, f32) {
        (self.x_scale * dx, self.y_scale * dy)
    }

    /// Whether the coordinate mapping is non-uniform: the geometric half of the
    /// flatten-vs-keep decision for a rotated arc.
    fn non_uniform(&self) -> bool {
        (self.x_scale.abs() - self.y_scale.abs()).abs() > 1e-6
    }
}

/// Rewrite every coordinate of `d` through `map`, preserving the command case
/// and the `A` rotation/flags byte-for-byte (see the module docs).
pub fn rewrite(d: &str, map: &AxisMap) -> Result<String, String> {
    let segments = tokenize(d)?;
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = (0.0f32, 0.0f32);
    let mut start = (0.0f32, 0.0f32);

    for segment in &segments {
        let command = segment.command;
        let args = &segment.args;
        // Absolute commands carry absolute coordinates; relative commands carry
        // vectors that scale without translation.
        let point = |x: f32, y: f32| {
            if command.is_ascii_lowercase() {
                map.map_delta(x, y)
            } else {
                map.map_point(x, y)
            }
        };
        tokens.push((command as char).to_string());

        match command.to_ascii_uppercase() {
            b'M' | b'L' => {
                let (dx, dy) = (args[0].value, args[1].value);
                cur = resolve(dx, dy, command.is_ascii_lowercase(), cur);
                if matches!(command, b'M' | b'm') {
                    start = cur;
                }
                if command.is_ascii_uppercase() {
                    // The token above already announced the command; emit the
                    // mapped absolute point.
                    push_point(&mut tokens, map.map_point(cur.0, cur.1));
                } else {
                    push_point(&mut tokens, point(dx, dy));
                }
            }
            b'H' => {
                let x = args[0].value;
                if command == b'h' {
                    cur = (cur.0 + x, cur.1);
                    tokens.push(format_number(map.x_scale * x));
                } else {
                    cur = (x, cur.1);
                    tokens.push(format_number(map.x_scale * x + map.x_offset));
                }
            }
            b'V' => {
                let y = args[0].value;
                if command == b'v' {
                    cur = (cur.0, cur.1 + y);
                    tokens.push(format_number(map.y_scale * y));
                } else {
                    cur = (cur.0, y);
                    tokens.push(format_number(map.y_scale * y + map.y_offset));
                }
            }
            b'C' => {
                if command == b'c' {
                    let (c1, c2, e) = (
                        (args[0].value, args[1].value),
                        (args[2].value, args[3].value),
                        (args[4].value, args[5].value),
                    );
                    cur = (cur.0 + e.0, cur.1 + e.1);
                    push_point(&mut tokens, map.map_delta(c1.0, c1.1));
                    push_point(&mut tokens, map.map_delta(c2.0, c2.1));
                    push_point(&mut tokens, map.map_delta(e.0, e.1));
                } else {
                    cur = (args[4].value, args[5].value);
                    push_point(&mut tokens, map.map_point(args[0].value, args[1].value));
                    push_point(&mut tokens, map.map_point(args[2].value, args[3].value));
                    push_point(&mut tokens, map.map_point(cur.0, cur.1));
                }
            }
            b'S' | b'Q' => {
                if command.is_ascii_lowercase() {
                    let (c, e) = (
                        (args[0].value, args[1].value),
                        (args[2].value, args[3].value),
                    );
                    cur = (cur.0 + e.0, cur.1 + e.1);
                    push_point(&mut tokens, map.map_delta(c.0, c.1));
                    push_point(&mut tokens, map.map_delta(e.0, e.1));
                } else {
                    cur = (args[2].value, args[3].value);
                    push_point(&mut tokens, map.map_point(args[0].value, args[1].value));
                    push_point(&mut tokens, map.map_point(cur.0, cur.1));
                }
            }
            b'T' => {
                let (dx, dy) = (args[0].value, args[1].value);
                cur = resolve(dx, dy, command == b't', cur);
                push_point(&mut tokens, point(dx, dy));
            }
            b'A' => {
                let arc_start = cur;
                let (ex, ey) = (args[5].value, args[6].value);
                let relative = command == b'a';
                cur = resolve(ex, ey, relative, cur);
                let rotation = args[2].value;
                if rotation != 0.0 && map.non_uniform() {
                    // A rotated arc sheared by a non-uniform scale is no longer
                    // "an ellipse arc rotated by `rotation`": sample the source
                    // arc and map each vertex, emitting absolute straight
                    // segments (the `A` token above is replaced by them).
                    tokens.pop();
                    let mut points = Vec::new();
                    flatten_arc(
                        arc_start,
                        args[0].value,
                        args[1].value,
                        rotation,
                        args[3].value != 0.0,
                        args[4].value != 0.0,
                        cur,
                        DEFAULT_TOLERANCE,
                        &mut points,
                    );
                    for point in &points {
                        tokens.push("L".to_string());
                        push_point(&mut tokens, map.map_point(point.0, point.1));
                    }
                    // Sampling stops within the tolerance of the end point; pin
                    // the subpath exactly onto it so later commands (which use
                    // `end` as the current point) stay consistent.
                    if points.last() != Some(&cur) {
                        tokens.push("L".to_string());
                        push_point(&mut tokens, map.map_point(cur.0, cur.1));
                    }
                } else {
                    tokens.push(format_number(args[0].value * map.x_length));
                    tokens.push(format_number(args[1].value * map.y_length));
                    // Rotation and both flags are copied verbatim.
                    tokens.push(args[2].raw.clone());
                    tokens.push(args[3].raw.clone());
                    tokens.push(args[4].raw.clone());
                    push_point(&mut tokens, point(ex, ey));
                }
            }
            b'Z' => {
                cur = start;
            }
            _ => unreachable!("tokenize only yields known commands"),
        }
    }

    Ok(tokens.join(" "))
}

fn push_point(tokens: &mut Vec<String>, point: (f32, f32)) {
    tokens.push(format_number(point.0));
    tokens.push(format_number(point.1));
}
