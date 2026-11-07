//! The running ink box and the affine `transform` it is mapped through.

use crate::dom::Element;

use super::walk::parse_number;

/// Running union of points, initialised to the empty set.
#[derive(Clone, Copy)]
pub(super) struct Bbox {
    pub(super) min_x: f64,
    pub(super) min_y: f64,
    pub(super) max_x: f64,
    pub(super) max_y: f64,
}

impl Bbox {
    pub(super) const fn empty() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.min_x > self.max_x
    }

    fn add_point(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub(super) fn add_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.add_point(x0.min(x1), y0.min(y1));
        self.add_point(x0.max(x1), y0.max(y1));
    }

    /// Map this axis-aligned box through `transform` and union the tight
    /// axis-aligned bounding box of the four transformed corners into `out`. A
    /// rotation (or any other non-axis-aligned map) thus widens the box to what
    /// the transformed ink actually covers.
    pub(super) fn transform_into(&self, transform: &Transform, out: &mut Bbox) {
        if self.is_empty() {
            return;
        }
        for (x, y) in [
            (self.min_x, self.min_y),
            (self.max_x, self.min_y),
            (self.min_x, self.max_y),
            (self.max_x, self.max_y),
        ] {
            let (x, y) = transform.apply(x, y);
            out.add_point(x, y);
        }
    }

    /// Union this box shifted by `(dx, dy)` into `out`: how a `<use>` places its
    /// referenced definition.
    pub(super) fn translate_into(&self, dx: f64, dy: f64, out: &mut Bbox) {
        if self.is_empty() {
            return;
        }
        out.add_rect(
            self.min_x + dx,
            self.min_y + dy,
            self.max_x + dx,
            self.max_y + dy,
        );
    }
}

/// A 2D affine transform in SVG's `matrix(a b c d e f)` form:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Clone, Copy)]
pub(super) struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Transform {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// Matrix product `self · rhs` (both acting on column vectors `[x, y, 1]ᵀ`),
    /// so the result applies `rhs` first and `self` second — the composition a
    /// left-to-right `transform` list asks for.
    fn mul(self, rhs: Self) -> Self {
        Self {
            a: self.a * rhs.a + self.c * rhs.b,
            b: self.b * rhs.a + self.d * rhs.b,
            c: self.a * rhs.c + self.c * rhs.d,
            d: self.b * rhs.c + self.d * rhs.d,
            e: self.a * rhs.e + self.c * rhs.f + self.e,
            f: self.b * rhs.e + self.d * rhs.f + self.f,
        }
    }
}

/// The element's own `transform`, or `None` when it is present but cannot be
/// evaluated. An absent or empty `transform` is the identity; `None` means "do
/// not measure this element's own geometry" (see the module docs).
pub(super) fn element_transform(element: &Element) -> Option<Transform> {
    match element.attr("transform").map(str::trim) {
        None | Some("") => Some(Transform::IDENTITY),
        Some(value) => parse_transform(value),
    }
}

/// Parse a left-to-right `transform` list into a single matrix, or `None` if any
/// part is unsupported. Only `translate`, `scale`, `rotate` and `matrix` are
/// accepted; `skewX`/`skewY` — or anything else — make the whole list unusable.
fn parse_transform(value: &str) -> Option<Transform> {
    let mut rest = value.trim();
    let mut acc = Transform::IDENTITY;
    while !rest.is_empty() {
        let open = rest.find('(')?;
        let name = rest[..open].trim();
        let close = open + rest[open..].find(')')?;
        let args = parse_number_list(&rest[open + 1..close])?;
        acc = acc.mul(function_matrix(name, &args)?);
        rest = rest[close + 1..].trim_start_matches(|ch: char| ch.is_whitespace() || ch == ',');
    }
    Some(acc)
}

/// Every number in a transform argument list, or `None` when one of them does
/// not parse as a number.
fn parse_number_list(value: &str) -> Option<Vec<f64>> {
    value
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(parse_number)
        .collect()
}

/// The matrix for one supported `transform` function, or `None` for an unknown
/// function or a wrong argument count.
fn function_matrix(name: &str, args: &[f64]) -> Option<Transform> {
    match (name, args) {
        ("translate", [tx]) => Some(Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: *tx,
            f: 0.0,
        }),
        ("translate", [tx, ty]) => Some(Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: *tx,
            f: *ty,
        }),
        ("scale", [sx]) => Some(Transform {
            a: *sx,
            b: 0.0,
            c: 0.0,
            d: *sx,
            e: 0.0,
            f: 0.0,
        }),
        ("scale", [sx, sy]) => Some(Transform {
            a: *sx,
            b: 0.0,
            c: 0.0,
            d: *sy,
            e: 0.0,
            f: 0.0,
        }),
        ("rotate", [angle]) => Some(rotation(*angle, 0.0, 0.0)),
        ("rotate", [angle, cx, cy]) => Some(rotation(*angle, *cx, *cy)),
        ("matrix", [a, b, c, d, e, f]) => Some(Transform {
            a: *a,
            b: *b,
            c: *c,
            d: *d,
            e: *e,
            f: *f,
        }),
        _ => None,
    }
}

/// `rotate(angle cx cy)` as a matrix (`angle` in degrees, positive = clockwise
/// in SVG's y-down system).
fn rotation(angle: f64, cx: f64, cy: f64) -> Transform {
    let (sin, cos) = angle.to_radians().sin_cos();
    Transform {
        a: cos,
        b: sin,
        c: -sin,
        d: cos,
        e: cx - cx * cos + cy * sin,
        f: cy - cx * sin - cy * cos,
    }
}
