//! R65's non-finite gate: the geometry table, the lenient lengths, the path /
//! points / stop-offset token rules and the inline-style declarations.
//!
//! Split out of `finite.rs` in R67 so the module stays under the 20000 B limit
//! after the class table was added; the assertions are unchanged.

use super::dom::{parse, Element};
use super::finite::{resolved_units_user_space, unusable, Bad, IdIndex, Unusable};
use std::collections::HashMap;

fn bad(source: &str) -> Vec<Bad> {
    let root = parse(source).unwrap();
    let ids = index(&root);
    let mut found = Vec::new();
    walk(&root, &ids, true, &mut found);
    found
}

fn index(root: &Element) -> IdIndex<'_> {
    let mut ids = HashMap::new();
    collect(root, &mut ids);
    ids
}

fn collect<'a>(element: &'a Element, ids: &mut IdIndex<'a>) {
    if let Some(id) = element.attr("id") {
        ids.insert(id.to_string(), element);
    }
    for child in &element.children {
        if let crate::convert::dom::Node::Element(child) = child {
            collect(child, ids);
        }
    }
}

fn walk(element: &Element, ids: &IdIndex<'_>, primitive_user_space: bool, found: &mut Vec<Bad>) {
    found.extend(unusable(element, ids, primitive_user_space));
    let child_primitive = if element.name == "filter" {
        resolved_units_user_space(element, ids, "primitiveUnits", true)
    } else {
        primitive_user_space
    };
    for child in &element.children {
        if let crate::convert::dom::Node::Element(child) = child {
            walk(child, ids, child_primitive, found);
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
    assert!(
        attrs("<svg width='100' height='80'><path d='M 0 0 L 10 10 Z' fill='none'/></svg>")
            .is_empty()
    );
    let found = bad("<svg width='100' height='80'><path d='M 1e999 0 L 10 10' fill='none'/></svg>");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].attr, "d");
    assert_eq!(found[0].value, "1e999");
}

#[test]
fn points_are_checked_pair_wise() {
    assert!(
        attrs("<svg width='100' height='80'><polygon points='0,0 10,10 20,0'/></svg>").is_empty()
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
