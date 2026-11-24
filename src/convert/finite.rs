//! Non-finite numbers: the second half of the fail-closed scan.
//!
//! `parse::<f32>()` accepts `1e999` (which overflows to `inf`) and `NaN`, so a
//! value can be *syntactically* a number and still be one the composer cannot
//! rewrite: it would write `inf`/`NaN` back into the product, and neither is a
//! legal SVG number. R65 closes that hole for every attribute the composer
//! parses, while leaving the R59 policy intact — malformed *text*
//! (`stdDeviation="2px"`, `dx="abc"`) is still a W7 passthrough, not a hard
//! error; only a number that is not finite is fatal here.

use crate::convert::dom::Element;
use crate::convert::path::non_finite_numbers;
use crate::convert::translate::style::parse_style;

/// Why a value is unusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unusable {
    /// Not a number at all (`5em`, `abc`).
    NotANumber,
    /// A number, but `inf`/`NaN` (`1e999`, `NaN`, `-inf`).
    NotFinite,
}

/// One attribute (or one number inside an attribute) the scan rejects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bad {
    pub(crate) attr: String,
    pub(crate) value: String,
    pub(crate) kind: Unusable,
}

/// Attributes the composer parses with `geom::parse_f32` and rewrites. Anything
/// present but not a plain finite number (after `%` resolution) would make the
/// composer bail out or write an illegal number, so we fail closed instead.
fn geometry_attrs(name: &str) -> &'static [&'static str] {
    match name {
        "circle" => &["cx", "cy", "r"],
        "ellipse" => &["cx", "cy", "rx", "ry"],
        "text" => &["x", "y"],
        "use" => &["x", "y", "width", "height"],
        "line" => &["x1", "y1", "x2", "y2"],
        "rect" => &["x", "y", "width", "height"],
        "image" => &["x", "y", "width", "height"],
        _ => &[],
    }
}

/// Lengths the composer rescales but does **not** require to be a bare number:
/// svglite writes `textLength="10.23px"` and `font-size: 8.80px`, so a unit
/// suffix is legal here. Only a non-finite numeric prefix is fatal.
const LENIENT_ATTRS: [&str; 2] = ["font-size", "textLength"];

/// Filter primitive numbers. Malformed text stays a W7 passthrough (R59), but a
/// non-finite number would be kept verbatim in the product, so it is fatal.
const FILTER_ATTRS: [&str; 6] = [
    "stdDeviation",
    "dx",
    "dy",
    "radius",
    "baseFrequency",
    "scale",
];

/// Length declarations the composer rescales inside an inline `style="..."`.
const STYLE_LENGTHS: [&str; 4] = [
    "stroke-width",
    "font-size",
    "stroke-dasharray",
    "stroke-dashoffset",
];

/// Every attribute of `element` whose value the composer cannot rewrite.
pub(crate) fn unusable(element: &Element) -> Vec<Bad> {
    let mut bad = Vec::new();

    for attr in geometry_attrs(&element.name)
        .iter()
        .copied()
        .chain(std::iter::once("stroke-width"))
    {
        if let Some(value) = element.attr(attr) {
            if let Some(kind) = classify_strict(value) {
                bad.push(Bad {
                    attr: attr.to_string(),
                    value: value.trim().to_string(),
                    kind,
                });
            }
        }
    }

    for attr in LENIENT_ATTRS {
        if let Some(value) = element.attr(attr) {
            if let Some(token) = non_finite_prefix(value) {
                bad.push(Bad {
                    attr: attr.to_string(),
                    value: token,
                    kind: Unusable::NotFinite,
                });
            }
        }
    }

    if matches!(element.name.as_str(), "polyline" | "polygon") {
        if let Some(points) = element.attr("points") {
            // A pair the composer cannot parse (`abc,1`) is a hard error, exactly
            // as it is for the other geometry attributes.
            'outer: for pair in points.split_whitespace() {
                if let Some((x, y)) = pair.split_once(',') {
                    for token in [x, y] {
                        if let Some(kind) = classify_strict(token) {
                            bad.push(Bad {
                                attr: "points".to_string(),
                                value: token.trim().to_string(),
                                kind,
                            });
                            break 'outer;
                        }
                    }
                }
            }
            // A non-finite number is illegal however the list is punctuated: the
            // composer keeps an untransformed `points` verbatim, so `1e999` would
            // reach the product even when no pair is ever parsed.
            if let Some(token) = non_finite_tokens(points).into_iter().next() {
                if !bad.iter().any(|bad| bad.attr == "points") {
                    bad.push(Bad {
                        attr: "points".to_string(),
                        value: token,
                        kind: Unusable::NotFinite,
                    });
                }
            }
        }
    }

    // A path's `d` is tokenised by the converter itself, so the check is
    // token-wise: command letters and `Z`/`z` are never numbers, and a
    // non-finite value is legal in any coordinate slot.
    if element.name == "path" {
        if let Some(d) = element.attr("d") {
            if let Some(token) = non_finite_numbers(d).into_iter().next() {
                bad.push(Bad {
                    attr: "d".to_string(),
                    value: token,
                    kind: Unusable::NotFinite,
                });
            }
        }
    }

    // `<stop offset>` is a number or a percentage; both must be finite.
    if element.name == "stop" {
        if let Some(offset) = element.attr("offset") {
            if let Some(token) = non_finite_prefix(offset) {
                bad.push(Bad {
                    attr: "offset".to_string(),
                    value: token,
                    kind: Unusable::NotFinite,
                });
            }
        }
    }

    if element.name.starts_with("fe") {
        for attr in FILTER_ATTRS {
            if let Some(value) = element.attr(attr) {
                if let Some(token) = non_finite_prefix(value) {
                    bad.push(Bad {
                        attr: attr.to_string(),
                        value: token,
                        kind: Unusable::NotFinite,
                    });
                }
            }
        }
    }

    if let Some(style) = element.attr("style") {
        for (name, value) in parse_style(style) {
            if !STYLE_LENGTHS.contains(&name.as_str()) {
                continue;
            }
            if let Some(token) = non_finite_tokens(&value).into_iter().next() {
                bad.push(Bad {
                    attr: name,
                    value: token,
                    kind: Unusable::NotFinite,
                });
            }
        }
    }

    bad
}

/// Classify a value the composer parses strictly: `Ok` only for a finite number.
fn classify_strict(value: &str) -> Option<Unusable> {
    match value.trim().parse::<f32>() {
        Ok(number) if number.is_finite() => None,
        Ok(_) => Some(Unusable::NotFinite),
        Err(_) => Some(Unusable::NotANumber),
    }
}

/// The raw text of `token` when its numeric prefix (a bare number, a `px` length
/// or a percentage) is not finite. Malformed text yields `None`: that is the W7
/// passthrough policy, not this gate's business.
fn non_finite_prefix(token: &str) -> Option<String> {
    let token = token.trim();
    let number = token.strip_suffix("px").unwrap_or(token);
    let number = number.strip_suffix('%').unwrap_or(number).trim();
    match number.parse::<f32>() {
        Ok(value) if !value.is_finite() => Some(number.to_string()),
        _ => None,
    }
}

/// Every non-finite number in a whitespace/comma separated list
/// (`stroke-dasharray: 1,2`, `stdDeviation="2 3"`).
fn non_finite_tokens(value: &str) -> Vec<String> {
    value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(non_finite_prefix)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::dom::parse;

    fn bad(source: &str) -> Vec<Bad> {
        let root = parse(source).unwrap();
        let mut found = Vec::new();
        walk(&root, &mut found);
        found
    }

    fn walk(element: &Element, found: &mut Vec<Bad>) {
        found.extend(unusable(element));
        for child in &element.children {
            if let crate::convert::dom::Node::Element(child) = child {
                walk(child, found);
            }
        }
    }

    fn attrs(source: &str) -> Vec<String> {
        bad(source).into_iter().map(|bad| bad.attr).collect()
    }

    #[test]
    fn an_overflowing_exponent_is_not_finite() {
        let found = bad(
            "<svg width='100' height='80'><rect x='0' y='0' width='10' height='10' \
             stroke-width='1e999'/></svg>",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].attr, "stroke-width");
        assert_eq!(found[0].value, "1e999");
        assert_eq!(found[0].kind, Unusable::NotFinite);
    }

    #[test]
    fn nan_and_infinity_spellings_are_not_finite() {
        for value in ["NaN", "nan", "inf", "-inf", "Infinity", "1e999", "-1e999"] {
            let source =
                format!("<svg width='100' height='80'><circle cx='{value}' cy='0' r='1'/></svg>");
            assert_eq!(attrs(&source), ["cx"], "{value}");
        }
    }

    #[test]
    fn legal_numbers_are_accepted() {
        for value in ["0", "-0", ".5", "1.", "+1", "1E3", "1e-45", "1e38", "-2.5"] {
            let source =
                format!("<svg width='100' height='80'><circle cx='{value}' cy='0' r='1'/></svg>");
            assert!(attrs(&source).is_empty(), "{value}");
        }
    }

    #[test]
    fn a_unit_bearing_length_is_still_a_plain_number_error() {
        let found =
            bad("<svg width='100' height='80'><rect x='0' y='0' width='5em' height='10'/></svg>");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].attr, "width");
        assert_eq!(found[0].kind, Unusable::NotANumber);
    }

    #[test]
    fn a_px_suffixed_text_length_is_legal_but_a_non_finite_one_is_not() {
        assert!(attrs(
            "<svg width='100' height='80'><text x='0' y='0' textLength='10.23px'>a</text></svg>"
        )
        .is_empty());
        assert_eq!(
            attrs("<svg width='100' height='80'><text x='0' y='0' textLength='1e999px'>a</text></svg>"),
            ["textLength"]
        );
    }

    #[test]
    fn a_path_is_checked_token_wise() {
        assert!(attrs(
            "<svg width='100' height='80'><path d='M 0 0 L 10 10 Z' fill='none'/></svg>"
        )
        .is_empty());
        let found =
            bad("<svg width='100' height='80'><path d='M 1e999 0 L 10 10' fill='none'/></svg>");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].attr, "d");
        assert_eq!(found[0].value, "1e999");
    }

    #[test]
    fn points_are_checked_pair_wise() {
        assert!(
            attrs("<svg width='100' height='80'><polygon points='0,0 10,10 20,0'/></svg>")
                .is_empty()
        );
        assert_eq!(
            attrs("<svg width='100' height='80'><polygon points='1e999,10 20,20'/></svg>"),
            ["points"]
        );
    }

    #[test]
    fn a_stop_offset_may_be_a_percentage_but_must_be_finite() {
        assert!(attrs(
            "<svg width='100' height='80'><linearGradient id='g'><stop offset='50%' \
             stop-color='#000'/></linearGradient></svg>"
        )
        .is_empty());
        assert_eq!(
            attrs(
                "<svg width='100' height='80'><linearGradient id='g'><stop offset='1e999' \
                 stop-color='#000'/></linearGradient></svg>"
            ),
            ["offset"]
        );
    }

    #[test]
    fn a_filter_primitive_number_must_be_finite() {
        assert!(attrs(
            "<svg width='100' height='80'><filter id='f'><feGaussianBlur stdDeviation='2 3'/>\
             </filter></svg>"
        )
        .is_empty());
        assert_eq!(
            attrs(
                "<svg width='100' height='80'><filter id='f'><feGaussianBlur \
                 stdDeviation='1e999'/></filter></svg>"
            ),
            ["stdDeviation"]
        );
    }

    #[test]
    fn a_style_declaration_is_checked_too() {
        assert!(attrs(
            "<svg width='100' height='80'><rect x='0' y='0' width='10' height='10' \
             style='stroke-width: 0.77; font-size: 8.80px'/></svg>"
        )
        .is_empty());
        assert_eq!(
            attrs(
                "<svg width='100' height='80'><rect x='0' y='0' width='10' height='10' \
                 style='stroke-width: 1e999'/></svg>"
            ),
            ["stroke-width"]
        );
    }
}
