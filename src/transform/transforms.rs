//! `patternTransform` / `gradientTransform`: a paint server's transform is written in
//! the **user space of the panel**, so it has to be carried through the panel scale
//! like every other coordinate (round-data/R55).
//!
//! The rewritten geometry already lives in the cell's user space: the tile box, the
//! pattern body and the gradient's `x1/y1/x2/y2`/`cx/cy/fx/fy/r/fr` are all mapped by
//! `S = diag(sx, sy)`. A transform `T` inside that space therefore has to become
//!
//! ```text
//! T' = S · T · S⁻¹
//! ```
//!
//! (the conjugation that makes `T' · (S·p) == S · (T · p)`), *not* a replacement and
//! **not** an accumulation: every definition is conjugated with the scale of the
//! panel that references it, so a pattern nested inside another pattern is scaled
//! once for each level, exactly like the numbers around it (measured: inner
//! `translate(2 2)` + outer `translate(4 4)` at s=2 must become `4 4` / `8 8`,
//! xor 0; accumulating would scale the inner one twice).
//!
//! * Isotropic `s`: the conjugation keeps the linear part and multiplies every
//!   translation (including a `rotate` centre) by `s`, so the keyword form is kept.
//! * Anisotropic: a pure translation still becomes `translate(sx·tx, sy·ty)`, but a
//!   rotation/shear is no longer expressible with the keywords and is written as
//!   `matrix(a b c d e f)`.
//! * A transform the renderer leaves unchanged (a diagonal `scale()`, `skewX` under
//!   an isotropic scale — anything commuting with `S`) is **not rewritten at all**:
//!   the attribute keeps its exact bytes, so the corpus without a paint-server
//!   transform is byte-identical.
//!
//! A value this module cannot parse is kept verbatim with a W7 passthrough, like a
//! malformed `d`: the renderer still knows what to do with it, and failing closed
//! would be a new hard error the old version did not have.

use crate::dom::Element;
use crate::format_number;
use crate::geom::Rect;
use crate::warnings::Passthroughs;

use super::lengths::{length_scale_factor, LengthAxis};
use super::ScalePolicy;

/// Rewrite `attr` (`patternTransform` / `gradientTransform`) on `element` through the
/// panel scale. A missing attribute is never added.
pub(super) fn rewrite_paint_transform(
    element: &mut Element,
    attr: &str,
    policy: ScalePolicy,
    source: Rect,
    target: Rect,
    index: usize,
    passthroughs: &mut Passthroughs,
) {
    let sx = length_scale_factor(policy, source, target, LengthAxis::X).unwrap_or(1.0);
    let sy = length_scale_factor(policy, source, target, LengthAxis::Y).unwrap_or(1.0);
    // A degenerate cell (zero width/height) has no meaningful conjugation, and a
    // missing scale policy means the transform is already correct.
    if !sx.is_finite() || !sy.is_finite() || sx.abs() < f32::EPSILON || sy.abs() < f32::EPSILON {
        return;
    }
    if (sx - 1.0).abs() < 1e-6 && (sy - 1.0).abs() < 1e-6 {
        return;
    }

    let Some(raw) = element.attr(attr).map(str::to_string) else {
        return;
    };
    let segments = match parse(&raw) {
        Ok(segments) => segments,
        Err(reason) => {
            passthroughs.record(
                index,
                &element.name,
                attr,
                format!("unparsable transform {raw:?}: {reason}"),
            );
            return;
        }
    };

    let Some(rewritten) = rewrite(&segments, sx, sy) else {
        return;
    };
    element.set_attr(attr, rewritten);
}

/// The conjugated transform, or `None` when the value must stay exactly as written.
fn rewrite(segments: &[Segment], sx: f32, sy: f32) -> Option<String> {
    if approx_equal(sx, sy) {
        // Isotropic: conjugate segment by segment, which is exact and keeps the
        // keyword spelling. Any segment whose numbers do not move keeps the
        // attribute verbatim.
        let mut changed = false;
        let mut parts = Vec::with_capacity(segments.len());
        for segment in segments {
            let (text, moved) = segment.conjugate_isotropic(sx);
            changed |= moved;
            parts.push(text);
        }
        return changed.then(|| parts.join(" "));
    }

    let total = segments.iter().fold(Matrix::IDENTITY, |acc, segment| {
        acc.multiply(&segment.matrix())
    });
    let conjugated = Matrix::diagonal(sx, sy)
        .multiply(&total)
        .multiply(&Matrix::diagonal(1.0 / sx, 1.0 / sy));
    if conjugated.approx_equals(&total) {
        return None;
    }
    if conjugated.is_translation() {
        return Some(format!(
            "translate({}, {})",
            format_number(conjugated.e),
            format_number(conjugated.f)
        ));
    }
    Some(format!(
        "matrix({} {} {} {} {} {})",
        format_number(conjugated.a),
        format_number(conjugated.b),
        format_number(conjugated.c),
        format_number(conjugated.d),
        format_number(conjugated.e),
        format_number(conjugated.f)
    ))
}

fn approx_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= 1e-6 * left.abs().max(right.abs()).max(1.0)
}

/// One `name(...)` component of a transform list.
#[derive(Debug, Clone, PartialEq)]
struct Segment {
    kind: Kind,
    values: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Matrix,
    Translate,
    Scale,
    Rotate,
    SkewX,
    SkewY,
}

impl Segment {
    fn matrix(&self) -> Matrix {
        match self.kind {
            Kind::Matrix => Matrix {
                a: self.values[0],
                b: self.values[1],
                c: self.values[2],
                d: self.values[3],
                e: self.values[4],
                f: self.values[5],
            },
            Kind::Translate => Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: self.values[0],
                f: self.values.get(1).copied().unwrap_or(0.0),
            },
            Kind::Scale => {
                let sx = self.values[0];
                let sy = self.values.get(1).copied().unwrap_or(sx);
                Matrix {
                    a: sx,
                    b: 0.0,
                    c: 0.0,
                    d: sy,
                    e: 0.0,
                    f: 0.0,
                }
            }
            Kind::Rotate => {
                let angle = self.values[0].to_radians();
                let (sin, cos) = angle.sin_cos();
                let rotation = Matrix {
                    a: cos,
                    b: sin,
                    c: -sin,
                    d: cos,
                    e: 0.0,
                    f: 0.0,
                };
                match self.values.get(1..3) {
                    Some([cx, cy]) => Matrix {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: *cx,
                        f: *cy,
                    }
                    .multiply(&rotation)
                    .multiply(&Matrix {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: -cx,
                        f: -cy,
                    }),
                    _ => rotation,
                }
            }
            Kind::SkewX => Matrix {
                a: 1.0,
                b: 0.0,
                c: self.values[0].to_radians().tan(),
                d: 1.0,
                e: 0.0,
                f: 0.0,
            },
            Kind::SkewY => Matrix {
                a: 1.0,
                b: self.values[0].to_radians().tan(),
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            },
        }
    }

    /// The isotropic conjugation of this one component: linear parts are invariant
    /// (any rotation/scale/shear commutes with a *uniform* scale), translation
    /// vectors and rotation centres multiply by `s`. Returns the text and whether
    /// any number moved.
    fn conjugate_isotropic(&self, s: f32) -> (String, bool) {
        let mut values = self.values.clone();
        match self.kind {
            Kind::Matrix => {
                values[4] *= s;
                values[5] *= s;
            }
            Kind::Translate => {
                for value in &mut values {
                    *value *= s;
                }
            }
            Kind::Rotate if values.len() == 3 => {
                values[1] *= s;
                values[2] *= s;
            }
            Kind::Scale | Kind::Rotate | Kind::SkewX | Kind::SkewY => {}
        }
        let moved = values
            .iter()
            .zip(&self.values)
            .any(|(new, old)| !approx_equal(*new, *old));
        let separator = if self.kind == Kind::Matrix { " " } else { ", " };
        let numbers = values
            .iter()
            .map(|value| format_number(clean(*value)))
            .collect::<Vec<_>>()
            .join(separator);
        let name = match self.kind {
            Kind::Matrix => "matrix",
            Kind::Translate => "translate",
            Kind::Scale => "scale",
            Kind::Rotate => "rotate",
            Kind::SkewX => "skewX",
            Kind::SkewY => "skewY",
        };
        (format!("{name}({numbers})"), moved)
    }
}

/// `format_number` prints `-0.000` for a tiny negative; a transform coefficient is
/// never meaningfully negative zero.
fn clean(value: f32) -> f32 {
    if value.abs() < 0.0005 {
        0.0
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy)]
struct Matrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Matrix {
    const IDENTITY: Matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn diagonal(x: f32, y: f32) -> Matrix {
        Matrix {
            a: x,
            b: 0.0,
            c: 0.0,
            d: y,
            e: 0.0,
            f: 0.0,
        }
    }

    /// `self · other`, with SVG's `matrix(a b c d e f) = [a c e; b d f; 0 0 1]`.
    fn multiply(&self, other: &Matrix) -> Matrix {
        Matrix {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    fn is_translation(&self) -> bool {
        approx_equal(self.a, 1.0)
            && approx_equal(self.b, 0.0)
            && approx_equal(self.c, 0.0)
            && approx_equal(self.d, 1.0)
    }

    fn approx_equals(&self, other: &Matrix) -> bool {
        [
            (self.a, other.a),
            (self.b, other.b),
            (self.c, other.c),
            (self.d, other.d),
            (self.e, other.e),
            (self.f, other.f),
        ]
        .iter()
        .all(|(left, right)| (left - right).abs() <= 1e-5 * left.abs().max(right.abs()).max(1.0))
    }
}

/// Parse the six SVG transform functions, comma or whitespace separated, with
/// chained segments and no-space negatives (`translate(3-4)`). Anything else is an
/// error so the caller can keep the attribute verbatim.
fn parse(raw: &str) -> Result<Vec<Segment>, String> {
    let bytes = raw.as_bytes();
    let mut position = 0usize;
    let mut segments = Vec::new();
    loop {
        skip_separators(bytes, &mut position);
        if position == bytes.len() {
            break;
        }
        let name_start = position;
        while position < bytes.len() && bytes[position].is_ascii_alphabetic() {
            position += 1;
        }
        let name = &raw[name_start..position];
        let kind = match name {
            "matrix" => Kind::Matrix,
            "translate" => Kind::Translate,
            "scale" => Kind::Scale,
            "rotate" => Kind::Rotate,
            "skewX" => Kind::SkewX,
            "skewY" => Kind::SkewY,
            _ => return Err(format!("unknown function {name:?}")),
        };
        skip_separators(bytes, &mut position);
        if position == bytes.len() || bytes[position] != b'(' {
            return Err(format!("expected `(` after {name:?}"));
        }
        position += 1;
        let mut values = Vec::new();
        loop {
            skip_separators(bytes, &mut position);
            if position == bytes.len() {
                return Err(format!("unterminated {name:?}"));
            }
            if bytes[position] == b')' {
                position += 1;
                break;
            }
            let Some(value) = parse_number(raw, &mut position) else {
                return Err(format!("bad number in {name:?}"));
            };
            values.push(value);
        }
        let expected: &[usize] = match kind {
            Kind::Matrix => &[6],
            Kind::Translate | Kind::Scale => &[1, 2],
            Kind::Rotate => &[1, 3],
            Kind::SkewX | Kind::SkewY => &[1],
        };
        if !expected.contains(&values.len()) {
            return Err(format!(
                "{name} takes {:?} arguments, got {}",
                expected,
                values.len()
            ));
        }
        segments.push(Segment { kind, values });
    }
    if segments.is_empty() {
        return Err("empty transform".to_string());
    }
    Ok(segments)
}

fn skip_separators(bytes: &[u8], position: &mut usize) {
    while *position < bytes.len()
        && (bytes[*position].is_ascii_whitespace() || bytes[*position] == b',')
    {
        *position += 1;
    }
}

fn parse_number(raw: &str, position: &mut usize) -> Option<f32> {
    let bytes = raw.as_bytes();
    let start = *position;
    if *position < bytes.len() && (bytes[*position] == b'+' || bytes[*position] == b'-') {
        *position += 1;
    }
    let mut digits = false;
    while *position < bytes.len() && bytes[*position].is_ascii_digit() {
        digits = true;
        *position += 1;
    }
    if *position < bytes.len() && bytes[*position] == b'.' {
        *position += 1;
        while *position < bytes.len() && bytes[*position].is_ascii_digit() {
            digits = true;
            *position += 1;
        }
    }
    if !digits {
        *position = start;
        return None;
    }
    if *position < bytes.len() && (bytes[*position] == b'e' || bytes[*position] == b'E') {
        let exponent = *position;
        *position += 1;
        if *position < bytes.len() && (bytes[*position] == b'+' || bytes[*position] == b'-') {
            *position += 1;
        }
        let mut exponent_digits = false;
        while *position < bytes.len() && bytes[*position].is_ascii_digit() {
            exponent_digits = true;
            *position += 1;
        }
        if !exponent_digits {
            *position = exponent;
        }
    }
    raw[start..*position].parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(raw: &str, sx: f32, sy: f32) -> Option<String> {
        rewrite(&parse(raw).unwrap(), sx, sy)
    }

    #[test]
    fn isotropic_conjugation_multiplies_translation_and_rotation_centre() {
        assert_eq!(out("translate(3 4)", 2.0, 2.0).unwrap(), "translate(6, 8)");
        assert_eq!(out("rotate(30 3 4)", 2.0, 2.0).unwrap(), "rotate(30, 6, 8)");
        assert_eq!(
            out("matrix(1 0 0 1 3 4)", 2.0, 2.0).unwrap(),
            "matrix(1 0 0 1 6 8)"
        );
        assert_eq!(
            out("translate(3 4) rotate(30) translate(1 2)", 2.0, 2.0).unwrap(),
            "translate(6, 8) rotate(30) translate(2, 4)"
        );
    }

    #[test]
    fn a_transform_that_commutes_with_the_scale_is_kept() {
        assert_eq!(out("scale(2 3)", 2.0, 2.0), None);
        assert_eq!(out("skewX(20)", 2.0, 2.0), None);
        assert_eq!(out("scale(2 3)", 4.0, 5.0), None);
        assert_eq!(out("translate(0 0)", 2.0, 2.0), None);
    }

    #[test]
    fn anisotropic_conjugation_uses_a_matrix_when_keywords_cannot() {
        assert_eq!(
            out("translate(3 4)", 4.0, 5.0).unwrap(),
            "translate(12, 20)"
        );
        assert_eq!(
            out("rotate(30)", 4.0, 5.0).unwrap(),
            "matrix(0.866 0.625 -0.400 0.866 0 0)"
        );
        assert_eq!(
            out("skewX(20)", 4.0, 5.0).unwrap(),
            "matrix(1 0 0.291 1 0 0)"
        );
    }

    #[test]
    fn parsing_handles_commas_negatives_exponents_and_chains() {
        assert_eq!(out("translate(3-4)", 2.0, 2.0).unwrap(), "translate(6, -8)");
        assert_eq!(
            out("translate(1e1, 2.5e0)", 2.0, 2.0).unwrap(),
            "translate(20, 5)"
        );
        assert_eq!(
            out("rotate(30)translate(1 2)", 2.0, 2.0).unwrap(),
            "rotate(30) translate(2, 4)"
        );
        assert!(parse("translate(3").is_err());
        assert!(parse("wobble(3)").is_err());
        assert!(parse("matrix(1 0 0 1 3)").is_err());
    }
}
