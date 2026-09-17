//! Minimal 2-D affine transforms (`matrix(a b c d e f)` convention) used by the
//! matplotlib dialect translator to bake `<g transform>` into leaf geometry.
//!
//! ```text
//! x' = a*x + c*y + e
//! y' = b*x + d*y + f
//! ```
//!
//! A `transform="f1 f2 ..."` list is composed left-to-right: the leftmost
//! function is the outermost, so the accumulated matrix is `f1 * f2 * ...` and a
//! point is mapped by `f1(f2(...))`.

const EPSILON: f32 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn matrix(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Affine { a, b, c, d, e, f }
    }

    pub fn translate(tx: f32, ty: f32) -> Self {
        Affine::matrix(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Affine::matrix(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    pub fn rotate(degrees: f32) -> Self {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        Affine::matrix(cos, sin, -sin, cos, 0.0, 0.0)
    }

    pub fn rotate_around(degrees: f32, cx: f32, cy: f32) -> Self {
        Affine::translate(cx, cy)
            .mul(Affine::rotate(degrees))
            .mul(Affine::translate(-cx, -cy))
    }

    pub fn skew_x(degrees: f32) -> Self {
        Affine::matrix(1.0, 0.0, degrees.to_radians().tan(), 1.0, 0.0, 0.0)
    }

    pub fn skew_y(degrees: f32) -> Self {
        Affine::matrix(1.0, degrees.to_radians().tan(), 0.0, 1.0, 0.0, 0.0)
    }

    /// `self ∘ other` (apply `other` first, then `self`).
    pub fn mul(self, other: Affine) -> Affine {
        Affine {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    pub fn is_identity(&self) -> bool {
        (self.a - 1.0).abs() < EPSILON
            && self.b.abs() < EPSILON
            && self.c.abs() < EPSILON
            && (self.d - 1.0).abs() < EPSILON
            && self.e.abs() < EPSILON
            && self.f.abs() < EPSILON
    }

    /// `(sx, sy, tx, ty)` when the linear part is diagonal (no rotation/skew).
    pub fn as_axis_scale_translate(&self) -> Option<(f32, f32, f32, f32)> {
        if self.b.abs() < EPSILON && self.c.abs() < EPSILON {
            Some((self.a, self.d, self.e, self.f))
        } else {
            None
        }
    }

    /// `(s, tx, ty)` when the linear part is a *uniform* scale (b=c=0, |a|=|d|),
    /// i.e. one that keeps circles circular.
    pub fn as_uniform_scale_translate(&self) -> Option<(f32, f32, f32)> {
        let (sx, sy, tx, ty) = self.as_axis_scale_translate()?;
        if (sx.abs() - sy.abs()).abs() < EPSILON && sx.abs() > EPSILON {
            Some((sx, tx, ty))
        } else {
            None
        }
    }
}

/// Parse a whitespace/comma separated list of numbers (used for the arguments
/// of a transform function).
pub(crate) fn parse_number_list(value: &str) -> Option<Vec<f32>> {
    let mut numbers = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b',') {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let start = index;
        if bytes[index] == b'+' || bytes[index] == b'-' {
            index += 1;
        }
        while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
            index += 1;
        }
        if index < bytes.len() && (bytes[index] == b'e' || bytes[index] == b'E') {
            index += 1;
            if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-') {
                index += 1;
            }
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        let number = std::str::from_utf8(&bytes[start..index])
            .ok()?
            .parse::<f32>()
            .ok()?;
        numbers.push(number);
    }
    Some(numbers)
}

/// Parse a full `transform="..."` attribute. Returns `None` on anything that is
/// not a recognised function, or on a wrong argument count.
pub fn parse_transform(value: &str) -> Option<Affine> {
    let bytes = value.as_bytes();
    let mut index = 0usize;
    let mut transform = Affine::IDENTITY;

    loop {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b',') {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let name_start = index;
        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        let name = std::str::from_utf8(&bytes[name_start..index]).ok()?;

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'(' {
            return None;
        }
        let open = index + 1;
        let close = value[open..].find(')')? + open;
        let args = parse_number_list(&value[open..close])?;

        let function = match (name, args.as_slice()) {
            ("matrix", [a, b, c, d, e, f]) => Affine::matrix(*a, *b, *c, *d, *e, *f),
            ("translate", [tx]) => Affine::translate(*tx, 0.0),
            ("translate", [tx, ty]) => Affine::translate(*tx, *ty),
            ("scale", [s]) => Affine::scale(*s, *s),
            ("scale", [sx, sy]) => Affine::scale(*sx, *sy),
            ("rotate", [angle]) => Affine::rotate(*angle),
            ("rotate", [angle, cx, cy]) => Affine::rotate_around(*angle, *cx, *cy),
            ("skewX", [angle]) => Affine::skew_x(*angle),
            ("skewY", [angle]) => Affine::skew_y(*angle),
            _ => return None,
        };
        transform = transform.mul(function);
        index = close + 1;
    }

    Some(transform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: (f32, f32), expected: (f32, f32)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-3 && (actual.1 - expected.1).abs() < 1e-3,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn identity_and_translate() {
        assert!(Affine::IDENTITY.is_identity());
        let m = parse_transform("translate(10, 20)").unwrap();
        assert_close(m.apply(0.0, 0.0), (10.0, 20.0));
        assert!(m.as_axis_scale_translate().is_some());
    }

    #[test]
    fn matplotlib_image_idiom_is_axis_aligned() {
        // scale(1 -1) translate(0 -195.12)  →  x'=x, y'=-y+195.12
        let m = parse_transform("scale(1 -1) translate(0 -195.12)").unwrap();
        assert_close(m.apply(30.24, -25.92), (30.24, 221.04));
        let (sx, sy, _, _) = m.as_axis_scale_translate().unwrap();
        assert_eq!((sx, sy), (1.0, -1.0));
        assert_eq!(m.as_uniform_scale_translate().map(|v| v.0), Some(1.0));
    }

    #[test]
    fn rotation_is_not_axis_aligned() {
        let m = parse_transform("rotate(30) translate(1 2)").unwrap();
        assert!(m.as_axis_scale_translate().is_none());
    }

    #[test]
    fn malformed_transform_is_none() {
        assert!(parse_transform("translate(1 2").is_none());
        assert!(parse_transform("wobble(1)").is_none());
        assert!(parse_transform("matrix(1 2 3)").is_none());
    }
}
