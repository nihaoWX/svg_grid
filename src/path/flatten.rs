//! Flattening a `<path d>` into straight-line subpaths and its bounding box.
//!
//! Bézier curves are flattened by recursive subdivision; elliptical arcs are
//! sampled from the centre parameterisation (see [`flatten`]).

use super::parse::{resolve, tokenize};
use super::DEFAULT_TOLERANCE;

/// A flattened path subpath: the polyline vertices and whether a `Z` closed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Subpath {
    pub points: Vec<(f32, f32)>,
    pub closed: bool,
}

/// Flatten a `d` attribute into straight-line subpaths.
///
/// Straight commands (`M L H V Z`) are exact; Bézier curves (`C S Q T`) are
/// flattened by recursive subdivision until the control points are within
/// `tolerance` of the chord; elliptical arcs (`A`) are sampled from the centre
/// parameterisation.
pub fn flatten(d: &str, tolerance: f32) -> Result<Vec<Subpath>, String> {
    let segments = tokenize(d)?;
    let mut subpaths = Vec::new();
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut cur = (0.0f32, 0.0f32);
    let mut start = (0.0f32, 0.0f32);
    let mut prev_cubic: Option<(f32, f32)> = None;
    let mut prev_quad: Option<(f32, f32)> = None;

    let flush = |points: &mut Vec<(f32, f32)>, closed: bool, out: &mut Vec<Subpath>| {
        if points.len() >= 2 {
            out.push(Subpath {
                points: std::mem::take(points),
                closed,
            });
        } else {
            points.clear();
        }
    };

    for segment in &segments {
        let command = segment.command;
        let args: Vec<f32> = segment.args.iter().map(|arg| arg.value).collect();
        let relative = command.is_ascii_lowercase();
        match command {
            b'M' | b'm' => {
                flush(&mut points, false, &mut subpaths);
                cur = resolve(args[0], args[1], relative, cur);
                start = cur;
                points.push(cur);
                prev_cubic = None;
                prev_quad = None;
            }
            b'L' | b'l' | b'H' | b'h' | b'V' | b'v' => {
                cur = match command {
                    b'L' | b'l' => resolve(args[0], args[1], relative, cur),
                    b'H' => (args[0], cur.1),
                    b'h' => (cur.0 + args[0], cur.1),
                    b'V' => (cur.0, args[0]),
                    _ => (cur.0, cur.1 + args[0]), // b'v'
                };
                points.push(cur);
                prev_cubic = None;
                prev_quad = None;
            }
            b'C' | b'c' | b'S' | b's' | b'Q' | b'q' | b'T' | b't' => {
                let offset = |x: f32, y: f32| resolve(x, y, relative, cur);
                let (control1, control2, end) = match command {
                    b'C' | b'c' => (
                        offset(args[0], args[1]),
                        offset(args[2], args[3]),
                        offset(args[4], args[5]),
                    ),
                    b'S' | b's' => {
                        let control1 = match prev_cubic {
                            Some(previous) => (2.0 * cur.0 - previous.0, 2.0 * cur.1 - previous.1),
                            None => cur,
                        };
                        (control1, offset(args[0], args[1]), offset(args[2], args[3]))
                    }
                    b'Q' | b'q' => {
                        let control = offset(args[0], args[1]);
                        (control, control, offset(args[2], args[3]))
                    }
                    _ => {
                        // T/t: reflect the previous quadratic control point.
                        let control = match prev_quad {
                            Some(previous) => (2.0 * cur.0 - previous.0, 2.0 * cur.1 - previous.1),
                            None => cur,
                        };
                        (control, control, offset(args[0], args[1]))
                    }
                };
                let p0 = cur;
                match command {
                    b'C' | b'c' | b'S' | b's' => {
                        flatten_cubic(p0, control1, control2, end, tolerance, 0, &mut points);
                        prev_cubic = Some(control2);
                        prev_quad = None;
                    }
                    _ => {
                        // Quadratic (or reflected): turn it into an equivalent cubic.
                        let c1 = (
                            p0.0 + 2.0 / 3.0 * (control1.0 - p0.0),
                            p0.1 + 2.0 / 3.0 * (control1.1 - p0.1),
                        );
                        let c2 = (
                            end.0 + 2.0 / 3.0 * (control1.0 - end.0),
                            end.1 + 2.0 / 3.0 * (control1.1 - end.1),
                        );
                        flatten_cubic(p0, c1, c2, end, tolerance, 0, &mut points);
                        prev_quad = Some(control1);
                        prev_cubic = None;
                    }
                }
                cur = end;
            }
            b'A' | b'a' => {
                let end = resolve(args[5], args[6], relative, cur);
                flatten_arc(
                    cur,
                    args[0],
                    args[1],
                    args[2],
                    args[3] != 0.0,
                    args[4] != 0.0,
                    end,
                    tolerance,
                    &mut points,
                );
                cur = end;
                prev_cubic = None;
                prev_quad = None;
            }
            b'Z' | b'z' => {
                if !points.is_empty() {
                    if points.last() != Some(&start) {
                        points.push(start);
                    }
                    flush(&mut points, true, &mut subpaths);
                }
                cur = start;
                prev_cubic = None;
                prev_quad = None;
            }
            _ => return Err(format!("unsupported path command {:?}", command as char)),
        }
    }
    flush(&mut points, false, &mut subpaths);

    Ok(subpaths)
}

/// Axis-aligned bounding box `(min_x, min_y, max_x, max_y)` of the flattened
/// path, or `None` when the path draws nothing. Uses [`DEFAULT_TOLERANCE`].
///
/// An unparseable `d` yields `None`: like the rest of the ink estimate, the W6
/// judge prefers to *miss* a warning than to raise a false one.
pub fn bbox(d: &str) -> Option<(f32, f32, f32, f32)> {
    let subpaths = flatten(d, DEFAULT_TOLERANCE).ok()?;
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for (x, y) in subpaths.iter().flat_map(|subpath| &subpath.points) {
        bounds = Some(match bounds {
            None => (*x, *y, *x, *y),
            Some((min_x, min_y, max_x, max_y)) => {
                (min_x.min(*x), min_y.min(*y), max_x.max(*x), max_y.max(*y))
            }
        });
    }
    bounds
}

pub(super) fn midpoint(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5)
}

fn distance_to_line(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1e-9 {
        return ((point.0 - a.0).powi(2) + (point.1 - a.1).powi(2)).sqrt();
    }
    ((point.0 - a.0) * dy - (point.1 - a.1) * dx).abs() / length
}

fn flatten_cubic(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    tolerance: f32,
    depth: u32,
    out: &mut Vec<(f32, f32)>,
) {
    let flat = distance_to_line(p1, p0, p3).max(distance_to_line(p2, p0, p3)) <= tolerance;
    if flat || depth >= 18 {
        out.push(p3);
        return;
    }
    let p01 = midpoint(p0, p1);
    let p12 = midpoint(p1, p2);
    let p23 = midpoint(p2, p3);
    let p012 = midpoint(p01, p12);
    let p123 = midpoint(p12, p23);
    let p0123 = midpoint(p012, p123);
    flatten_cubic(p0, p01, p012, p0123, tolerance, depth + 1, out);
    flatten_cubic(p0123, p123, p23, p3, tolerance, depth + 1, out);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn flatten_arc(
    start: (f32, f32),
    rx: f32,
    ry: f32,
    x_rotation: f32,
    large_arc: bool,
    sweep: bool,
    end: (f32, f32),
    tolerance: f32,
    out: &mut Vec<(f32, f32)>,
) {
    if start == end {
        return;
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-9 || ry < 1e-9 {
        out.push(end);
        return;
    }

    let phi = x_rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx2 = (start.0 - end.0) * 0.5;
    let dy2 = (start.1 - end.1) * 0.5;
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    // Scale the radii up if they are too small to reach the endpoint.
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let numerator = (rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p).max(0.0);
    let denominator = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coefficient = if denominator > 0.0 {
        (numerator / denominator).sqrt()
    } else {
        0.0
    };
    if large_arc == sweep {
        coefficient = -coefficient;
    }
    let cxp = coefficient * rx * y1p / ry;
    let cyp = -coefficient * ry * x1p / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (start.0 + end.0) * 0.5;
    let cy = sin_phi * cxp + cos_phi * cyp + (start.1 + end.1) * 0.5;

    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let length = ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt()).max(1e-12);
        let mut value = (dot / length).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            value = -value;
        }
        value
    };
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta1 = angle(1.0, 0.0, ux, uy);
    let mut delta = angle(ux, uy, vx, vy);
    if !sweep && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }

    let max_radius = rx.max(ry);
    let step = if tolerance >= max_radius {
        std::f32::consts::TAU
    } else {
        2.0 * (1.0 - tolerance / max_radius).clamp(-1.0, 1.0).acos()
    };
    let step = if step <= 0.0 {
        std::f32::consts::TAU
    } else {
        step
    };
    let segments = ((delta.abs() / step).ceil() as usize).max(1);

    for i in 1..=segments {
        let theta = theta1 + delta * (i as f32) / (segments as f32);
        let (sin_theta, cos_theta) = theta.sin_cos();
        out.push((
            cx + rx * cos_theta * cos_phi - ry * sin_theta * sin_phi,
            cy + rx * cos_theta * sin_phi + ry * sin_theta * cos_phi,
        ));
    }
}
