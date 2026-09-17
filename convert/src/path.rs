//! `<path d="...">` parsing and flattening to straight-line subpaths.
//!
//! The composer only rewrites points (`polyline`/`polygon`), so every path is
//! turned into one element per `M`/`m` subpath: `polyline` when the subpath is
//! stroked-only, `polygon` when it is filled. Straight commands (`M L H V Z`)
//! convert exactly; Bézier curves (`C S Q T`) are flattened by recursive
//! subdivision until the control points are within `tolerance` of the chord;
//! elliptical arcs (`A`) are sampled from the centre parameterisation.

use crate::affine::Affine;

/// Default flattening tolerance, in the *source* user units of the panel.
///
/// The composed reproduction scales a panel by ~10x into the final 600-dpi PNG
/// (a 504-unit canvas maps to ≈4880 px, and the cell keeps that factor on the
/// shortest axis), so an error of 0.02 user units is ≈0.2 px on the final image
/// — below the ~1 px that a reader could notice. The concrete value and its
/// rationale are recorded in the task report.
pub const DEFAULT_TOLERANCE: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct Subpath {
    pub points: Vec<(f32, f32)>,
    pub closed: bool,
}

/// Flatten a `d` attribute into subpaths, mapping every emitted point through
/// `transform`.
pub fn flatten_path(d: &str, tolerance: f32, transform: &Affine) -> Result<Vec<Subpath>, String> {
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

    for (command, args) in segments {
        match command {
            b'M' | b'm' => {
                flush(&mut points, false, &mut subpaths);
                let (dx, dy) = if command == b'm' {
                    (cur.0, cur.1)
                } else {
                    (0.0, 0.0)
                };
                cur = (dx + args[0], dy + args[1]);
                start = cur;
                points.push(cur);
                prev_cubic = None;
                prev_quad = None;
            }
            b'L' | b'l' | b'H' | b'h' | b'V' | b'v' => {
                cur = match command {
                    b'L' => (args[0], args[1]),
                    b'l' => (cur.0 + args[0], cur.1 + args[1]),
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
                let relative = command.is_ascii_lowercase();
                let offset = |x: f32, y: f32, cur: (f32, f32)| {
                    if relative {
                        (cur.0 + x, cur.1 + y)
                    } else {
                        (x, y)
                    }
                };
                let (control1, control2, end) = match command {
                    b'C' | b'c' => (
                        offset(args[0], args[1], cur),
                        offset(args[2], args[3], cur),
                        offset(args[4], args[5], cur),
                    ),
                    b'S' | b's' => {
                        let control1 = match prev_cubic {
                            Some(previous) => (2.0 * cur.0 - previous.0, 2.0 * cur.1 - previous.1),
                            None => cur,
                        };
                        (
                            control1,
                            offset(args[0], args[1], cur),
                            offset(args[2], args[3], cur),
                        )
                    }
                    b'Q' | b'q' => {
                        let control = offset(args[0], args[1], cur);
                        (
                            control,
                            (control.0, control.1),
                            offset(args[2], args[3], cur),
                        )
                    }
                    _ => {
                        // T/t: reflect the previous quadratic control point.
                        let control = match prev_quad {
                            Some(previous) => (2.0 * cur.0 - previous.0, 2.0 * cur.1 - previous.1),
                            None => cur,
                        };
                        (
                            control,
                            (control.0, control.1),
                            offset(args[0], args[1], cur),
                        )
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
                let relative = command == b'a';
                let end = if relative {
                    (cur.0 + args[5], cur.1 + args[6])
                } else {
                    (args[5], args[6])
                };
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

    for subpath in &mut subpaths {
        for point in &mut subpath.points {
            *point = transform.apply(point.0, point.1);
        }
    }

    Ok(subpaths)
}

/// Tokenise `d` into `(command, arguments)` pairs, expanding implicit repeats.
fn tokenize(d: &str) -> Result<Vec<(u8, Vec<f32>)>, String> {
    let bytes = d.as_bytes();
    let mut index = 0usize;
    let mut segments = Vec::new();
    let mut last_command: Option<u8> = None;

    loop {
        skip_separators(bytes, &mut index);
        if index >= bytes.len() {
            break;
        }
        let command = if bytes[index].is_ascii_alphabetic() {
            let command = bytes[index];
            index += 1;
            command
        } else {
            match last_command {
                // Implicit repeat: after a moveto, extra pairs are linetos.
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(command) => command,
                None => return Err("path data must start with a command".to_string()),
            }
        };

        let arity = match command {
            b'Z' | b'z' => 0,
            b'H' | b'h' | b'V' | b'v' => 1,
            b'M' | b'm' | b'L' | b'l' | b'T' | b't' => 2,
            b'S' | b's' | b'Q' | b'q' => 4,
            b'C' | b'c' => 6,
            b'A' | b'a' => 7,
            other => return Err(format!("unsupported path command {:?}", other as char)),
        };

        let mut args = Vec::with_capacity(arity);
        for _ in 0..arity {
            skip_separators(bytes, &mut index);
            let number = read_number(bytes, &mut index).ok_or_else(|| {
                format!("missing argument for path command {:?}", command as char)
            })?;
            args.push(number);
        }
        last_command = Some(command);
        segments.push((command, args));
    }

    Ok(segments)
}

fn skip_separators(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && (bytes[*index].is_ascii_whitespace() || bytes[*index] == b',') {
        *index += 1;
    }
}

fn read_number(bytes: &[u8], index: &mut usize) -> Option<f32> {
    let start = *index;
    if *index < bytes.len() && (bytes[*index] == b'+' || bytes[*index] == b'-') {
        *index += 1;
    }
    let mut has_digit = false;
    while *index < bytes.len() && (bytes[*index].is_ascii_digit() || bytes[*index] == b'.') {
        has_digit = true;
        *index += 1;
    }
    if *index < bytes.len() && (bytes[*index] == b'e' || bytes[*index] == b'E') {
        *index += 1;
        if *index < bytes.len() && (bytes[*index] == b'+' || bytes[*index] == b'-') {
            *index += 1;
        }
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
    }
    if !has_digit {
        *index = start;
        return None;
    }
    std::str::from_utf8(&bytes[start..*index])
        .ok()?
        .parse::<f32>()
        .ok()
}

fn midpoint(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
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
fn flatten_arc(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn points(d: &str, tolerance: f32) -> Vec<Subpath> {
        flatten_path(d, tolerance, &Affine::IDENTITY).unwrap()
    }

    #[test]
    fn straight_lines_are_exact() {
        let subpaths = points("M 0 0 L 10 0 L 10 10 z", DEFAULT_TOLERANCE);
        assert_eq!(subpaths.len(), 1);
        assert!(subpaths[0].closed);
        assert_eq!(
            subpaths[0].points,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)]
        );
    }

    #[test]
    fn relative_and_implicit_commands() {
        // "m 1 2" then implicit linetos "3 4" (relative).
        let subpaths = points("m 1 2 3 4 5 6", 0.02);
        assert_eq!(
            subpaths[0].points,
            vec![(1.0, 2.0), (4.0, 6.0), (9.0, 12.0)]
        );
    }

    #[test]
    fn curve_flattening_respects_tolerance() {
        // A single cubic; a fine tolerance must yield more vertices than a coarse one.
        let d = "M 0 0 C 0 10 10 10 10 0";
        let coarse = points(d, 1.0)[0].points.len();
        let fine = points(d, 0.01)[0].points.len();
        assert!(fine > coarse, "fine={fine} coarse={coarse}");
        // Endpoints are exact.
        let fine_points = points(d, 0.02);
        assert_eq!(fine_points[0].points.first().copied(), Some((0.0, 0.0)));
        assert_eq!(fine_points[0].points.last().copied(), Some((10.0, 0.0)));
    }

    #[test]
    fn quadratic_is_flattened() {
        let subpaths = points("M 0 0 Q 5 10 10 0", 0.02);
        assert!(subpaths[0].points.len() > 2);
        assert_eq!(subpaths[0].points.last().copied(), Some((10.0, 0.0)));
    }

    #[test]
    fn arc_becomes_a_polyline() {
        let subpaths = points("M 0 0 A 5 5 0 0 1 10 0", 0.05);
        assert!(subpaths[0].points.len() > 3);
        let last = subpaths[0].points.last().unwrap();
        assert!((last.0 - 10.0).abs() < 1e-3 && last.1.abs() < 1e-3);
    }

    #[test]
    fn transform_is_applied_to_every_point() {
        let m = Affine::translate(100.0, 200.0);
        let subpaths = flatten_path("M 0 0 L 1 1", 0.02, &m).unwrap();
        assert_eq!(subpaths[0].points, vec![(100.0, 200.0), (101.0, 201.0)]);
    }

    #[test]
    fn unsupported_command_is_an_error() {
        assert!(flatten_path("M 0 0 X 1 1", 0.02, &Affine::IDENTITY).is_err());
    }
}
