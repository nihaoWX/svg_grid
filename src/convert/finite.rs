//! Non-finite numbers: the second half of the fail-closed scan.
//!
//! `parse::<f32>()` accepts `1e999` (which overflows to `inf`) and `NaN`, so a
//! value can be *syntactically* a number and still be one the composer cannot
//! rewrite: it would write `inf`/`NaN` back into the product, and neither is a
//! legal SVG number. R65 closes that hole for every attribute the composer
//! parses, while leaving the R59 policy intact — malformed *text*
//! (`stdDeviation="2px"`, `dx="abc"`) is still a W7 passthrough, not a hard
//! error; only a number that is not finite is fatal here.

use std::collections::{HashMap, HashSet};

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

/// Every element with an `id`, so a definition's units can be resolved along its
/// `href` chain exactly as the composer does (`transform::links`).
pub(crate) type IdIndex<'a> = HashMap<String, &'a Element>;

/// How the composer's rewrite of a class is gated.
#[derive(Clone, Copy)]
enum Gate {
    /// Rewritten whatever the units (`<tspan>` coordinates).
    Always,
    /// Rewritten only when the resolved units attribute is `userSpaceOnUse`
    /// (the default is `objectBoundingBox` for every class here).
    UserSpace(&'static str),
}

/// One table row: the attributes of `tag` the composer parses and rewrites.
struct Class {
    tag: &'static str,
    attrs: &'static [&'static str],
    gate: Gate,
    /// `true` when the composer parses the attribute with `parse_f32`, so a
    /// non-number is fatal too; `false` when it silently keeps a non-number
    /// (the pattern tile numbers, which may legally be percentages).
    strict: bool,
}

/// The classes R65 found missing. Each row is derived from a `rewrite_*` call
/// site, not from the SVG spec: `tspan` x/y (`dispatch.rs`), the gradient
/// coordinates (`dispatch.rs`, gated by `gradientUnits`), the pattern tile
/// (`patterns.rs`, `patternUnits`), the mask/filter regions (`dispatch.rs`,
/// `maskUnits`/`filterUnits`) and the marker box (`primitives.rs`,
/// `markerUnits`). A legal `objectBoundingBox` fraction (`r="50%"`) is not a
/// number the composer parses, so it stays accepted; only a value the composer
/// would actually rewrite is checked.
const CLASSES: &[Class] = &[
    Class {
        tag: "tspan",
        attrs: &["x", "y"],
        gate: Gate::Always,
        strict: true,
    },
    Class {
        tag: "linearGradient",
        attrs: &["x1", "y1", "x2", "y2"],
        gate: Gate::UserSpace("gradientUnits"),
        strict: true,
    },
    Class {
        tag: "radialGradient",
        attrs: &["cx", "cy", "r", "fr", "fx", "fy"],
        gate: Gate::UserSpace("gradientUnits"),
        strict: true,
    },
    Class {
        tag: "pattern",
        attrs: &["x", "y", "width", "height"],
        gate: Gate::UserSpace("patternUnits"),
        strict: false,
    },
    Class {
        tag: "mask",
        attrs: &["x", "y", "width", "height"],
        gate: Gate::UserSpace("maskUnits"),
        strict: true,
    },
    Class {
        tag: "filter",
        attrs: &["x", "y", "width", "height"],
        gate: Gate::UserSpace("filterUnits"),
        strict: true,
    },
    Class {
        tag: "marker",
        attrs: &["markerWidth", "markerHeight", "refX", "refY"],
        gate: Gate::UserSpace("markerUnits"),
        strict: true,
    },
];

/// The subregion every filter primitive carries; rewritten only when the
/// enclosing `<filter>`'s `primitiveUnits` is `userSpaceOnUse` (the default).
const FE_SUBREGION: [&str; 4] = ["x", "y", "width", "height"];

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
pub(crate) fn unusable(
    element: &Element,
    ids: &IdIndex<'_>,
    primitive_user_space: bool,
) -> Vec<Bad> {
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

    // The definition/primitive classes the composer rewrites (R67). The gate is
    // the resolved units, so a legal `objectBoundingBox` fraction is untouched.
    for class in CLASSES {
        if element.name != class.tag || !gate_open(element, ids, class.gate) {
            continue;
        }
        for attr in class.attrs {
            let Some(value) = element.attr(attr) else {
                continue;
            };
            let found = if class.strict {
                classify_strict(value).map(|kind| (value.trim().to_string(), kind))
            } else {
                non_finite_prefix(value).map(|token| (token, Unusable::NotFinite))
            };
            if let Some((value, kind)) = found {
                bad.push(Bad {
                    attr: attr.to_string(),
                    value,
                    kind,
                });
            }
        }
    }

    // A filter primitive's own subregion is rewritten only when the enclosing
    // `<filter>` resolves `primitiveUnits` to `userSpaceOnUse` (the default).
    if element.name.starts_with("fe") && primitive_user_space {
        for attr in FE_SUBREGION {
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

fn gate_open(element: &Element, ids: &IdIndex<'_>, gate: Gate) -> bool {
    match gate {
        Gate::Always => true,
        Gate::UserSpace(attr) => resolved_units_user_space(element, ids, attr, false),
    }
}

/// The composer resolves a units attribute along the `href` chain
/// (`transform::links::resolved_units`); mirror that so an inherited
/// `gradientUnits="userSpaceOnUse"` is not missed.
pub(crate) fn resolved_units_user_space(
    element: &Element,
    ids: &IdIndex<'_>,
    attr: &str,
    default: bool,
) -> bool {
    if let Some(value) = element.attr(attr) {
        return value == "userSpaceOnUse";
    }
    let mut link = element.attr("href").or_else(|| element.attr("xlink:href"));
    let mut seen = HashSet::new();
    while let Some(href) = link {
        let Some(id) = internal_target(href) else {
            break;
        };
        if !seen.insert(id.to_string()) {
            break;
        }
        let Some(target) = ids.get(id) else {
            break;
        };
        if !follows(element.name.as_str(), target.name.as_str(), attr) {
            break;
        }
        if let Some(value) = target.attr(attr) {
            return value == "userSpaceOnUse";
        }
        link = target.attr("href").or_else(|| target.attr("xlink:href"));
    }
    default
}

/// Which `href` edges carry `attr` (`transform::links::follows`).
fn follows(owner: &str, target: &str, attr: &str) -> bool {
    match attr {
        "gradientUnits" => {
            matches!(owner, "linearGradient" | "radialGradient")
                && matches!(target, "linearGradient" | "radialGradient")
        }
        "patternUnits" => owner == "pattern" && target == "pattern",
        "filterUnits" | "primitiveUnits" => owner == "filter" && target == "filter",
        _ => false,
    }
}

/// The fragment of an internal `href` (`transform::links::internal_target`).
fn internal_target(href: &str) -> Option<&str> {
    let fragment = href.trim_start_matches(' ').strip_prefix('#')?;
    let end = fragment.find(' ').unwrap_or(fragment.len());
    let (id, tail) = fragment.split_at(end);
    if id.is_empty() || !tail.bytes().all(|byte| byte == b' ') {
        return None;
    }
    if id.chars().any(|c| matches!(c, '\t' | '\n' | '\r')) {
        return None;
    }
    Some(id)
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
