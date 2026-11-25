//! R67: the fail-closed scan must cover every attribute the composer really
//! rewrites, not just the geometry table R65 started from.
//!
//! The criterion is the **composer's** behaviour, not the SVG spec: a class is
//! in scope when a `rewrite_*` call site parses the attribute, and the gate is
//! the resolved `*Units` (so a legal `objectBoundingBox` fraction such as
//! `r="50%"` keeps passing). Each test names the attribute and the value in the
//! finding, so a regression cannot pass by rejecting the wrong thing.

use super::test_support::{converted, incompatible};

const HEAD: &str = "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='80' \
                    viewBox='0 0 100 80'>";

fn doc(body: &str) -> String {
    format!("{HEAD}{body}</svg>")
}

/// The finding must name `attr="value"` (the scan's own wording).
fn rejects(source: &str, attr: &str, value: &str) {
    let findings = incompatible(source);
    let needle = format!("{attr}={value:?}");
    assert!(
        findings
            .iter()
            .any(|finding| finding.reason.contains(&needle)),
        "{needle} not named in {findings:?}"
    );
}

fn accepts(source: &str) {
    let done = converted(source);
    assert!(
        !done.svg.is_empty(),
        "a legal control must convert to an SVG"
    );
}

// ---- the 13 classes R65 found leaking ------------------------------------

#[test]
fn tspan_x_is_in_class() {
    rejects(
        &doc("<text x='10' y='20' font-size='10'>a<tspan x='1e999' y='30'>b</tspan></text>"),
        "x",
        "1e999",
    );
}

#[test]
fn tspan_y_is_in_class() {
    rejects(
        &doc("<text x='10' y='20' font-size='10'>a<tspan y='1e999'>b</tspan></text>"),
        "y",
        "1e999",
    );
}

#[test]
fn a_user_space_linear_gradient_coordinate_is_in_class() {
    rejects(
        &doc(
            "<defs><linearGradient id='g' gradientUnits='userSpaceOnUse' x1='1e999'>\
             <stop offset='0' stop-color='#000'/></linearGradient></defs>\
             <rect width='10' height='10' fill='url(#g)'/>",
        ),
        "x1",
        "1e999",
    );
}

#[test]
fn a_user_space_radial_gradient_centre_is_in_class() {
    rejects(
        &doc(
            "<defs><radialGradient id='g' gradientUnits='userSpaceOnUse' cx='1e999'>\
             <stop offset='0' stop-color='#000'/></radialGradient></defs>\
             <rect width='10' height='10' fill='url(#g)'/>",
        ),
        "cx",
        "1e999",
    );
}

#[test]
fn a_user_space_radial_gradient_radius_is_in_class() {
    rejects(
        &doc(
            "<defs><radialGradient id='g' gradientUnits='userSpaceOnUse' r='1e999'>\
             <stop offset='0' stop-color='#000'/></radialGradient></defs>\
             <rect width='10' height='10' fill='url(#g)'/>",
        ),
        "r",
        "1e999",
    );
}

#[test]
fn a_user_space_radial_gradient_focal_radius_is_in_class() {
    rejects(
        &doc(
            "<defs><radialGradient id='g' gradientUnits='userSpaceOnUse' fr='1e999'>\
             <stop offset='0' stop-color='#000'/></radialGradient></defs>\
             <rect width='10' height='10' fill='url(#g)'/>",
        ),
        "fr",
        "1e999",
    );
}

#[test]
fn a_user_space_pattern_tile_origin_is_in_class() {
    rejects(
        &doc(
            "<defs><pattern id='p' patternUnits='userSpaceOnUse' x='1e999' width='8' \
             height='8'><rect width='4' height='4' fill='#000'/></pattern></defs>\
             <rect width='10' height='10' fill='url(#p)'/>",
        ),
        "x",
        "1e999",
    );
}

#[test]
fn a_user_space_mask_region_is_in_class() {
    rejects(
        &doc("<defs><mask id='m' maskUnits='userSpaceOnUse' x='1e999'>\
             <rect width='10' height='10' fill='#fff'/></mask></defs>\
             <rect width='10' height='10' mask='url(#m)'/>"),
        "x",
        "1e999",
    );
}

#[test]
fn a_user_space_filter_region_is_in_class() {
    rejects(
        &doc(
            "<defs><filter id='f' filterUnits='userSpaceOnUse' x='1e999'>\
             <feGaussianBlur stdDeviation='1'/></filter></defs>\
             <rect width='10' height='10' filter='url(#f)'/>",
        ),
        "x",
        "1e999",
    );
}

#[test]
fn a_user_space_marker_width_is_in_class() {
    rejects(
        &doc(
            "<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='1e999'>\
             <rect width='2' height='2' fill='#000'/></marker></defs>\
             <path d='M 0 0 L 10 10' marker-start='url(#m)' stroke='#000'/>",
        ),
        "markerWidth",
        "1e999",
    );
}

#[test]
fn a_user_space_marker_reference_point_is_in_class() {
    rejects(
        &doc(
            "<defs><marker id='m' markerUnits='userSpaceOnUse' refX='1e999'>\
             <rect width='2' height='2' fill='#000'/></marker></defs>\
             <path d='M 0 0 L 10 10' marker-start='url(#m)' stroke='#000'/>",
        ),
        "refX",
        "1e999",
    );
}

#[test]
fn a_filter_primitive_subregion_is_in_class() {
    rejects(
        &doc("<defs><filter id='f'><feTile x='1e999'/></filter></defs>\
             <rect width='10' height='10' filter='url(#f)'/>"),
        "x",
        "1e999",
    );
}

#[test]
fn a_filter_primitive_subregion_width_is_in_class() {
    rejects(
        &doc(
            "<defs><filter id='f'><feTile width='1e999'/></filter></defs>\
             <rect width='10' height='10' filter='url(#f)'/>",
        ),
        "width",
        "1e999",
    );
}

// ---- the units gate: legal fractions must keep passing --------------------

#[test]
fn an_object_bounding_box_gradient_percentage_is_accepted() {
    accepts(&doc(
        "<defs><radialGradient id='g' r='50%'><stop offset='0' stop-color='#000'/>\
         </radialGradient></defs><rect width='10' height='10' fill='url(#g)'/>",
    ));
}

#[test]
fn an_object_bounding_box_pattern_percentage_is_accepted() {
    accepts(&doc("<defs><pattern id='p' x='50%' width='8' height='8'>\
         <rect width='4' height='4' fill='#000'/></pattern></defs>\
         <rect width='10' height='10' fill='url(#p)'/>"));
}

#[test]
fn a_user_space_pattern_percentage_tile_is_accepted() {
    // `tile_number` parses with `parse::<f32>().ok()`, so `50%` is kept verbatim
    // rather than rewritten: only a non-finite number is fatal here.
    accepts(&doc(
        "<defs><pattern id='p' patternUnits='userSpaceOnUse' width='50%' height='8'>\
         <rect width='4' height='4' fill='#000'/></pattern></defs>\
         <rect width='10' height='10' fill='url(#p)'/>",
    ));
}

#[test]
fn a_marker_under_the_default_stroke_width_units_is_not_in_class() {
    accepts(&doc(
        "<defs><marker id='m' markerWidth='1e999'><rect width='2' height='2' fill='#000'/>\
         </marker></defs><path d='M 0 0 L 10 10' marker-start='url(#m)' stroke='#000'/>",
    ));
}

#[test]
fn a_filter_primitive_under_object_bounding_box_is_not_in_class() {
    accepts(&doc(
        "<defs><filter id='f' primitiveUnits='objectBoundingBox'><feTile x='1e999'/>\
         </filter></defs><rect width='10' height='10' filter='url(#f)'/>",
    ));
}

// ---- href inheritance: the composer resolves units along the chain --------

#[test]
fn an_inherited_user_space_gradient_units_is_in_class() {
    rejects(
        &doc(
            "<defs><linearGradient id='base' gradientUnits='userSpaceOnUse'/>\
             <linearGradient id='g' href='#base' x1='1e999'>\
             <stop offset='0' stop-color='#000'/></linearGradient></defs>\
             <rect width='10' height='10' fill='url(#g)'/>",
        ),
        "x1",
        "1e999",
    );
}

#[test]
fn an_inherited_user_space_pattern_units_is_in_class() {
    rejects(
        &doc(
            "<defs><pattern id='base' patternUnits='userSpaceOnUse' width='8' height='8'/>\
             <pattern id='p' href='#base' x='1e999'><rect width='4' height='4' fill='#000'/>\
             </pattern></defs><rect width='10' height='10' fill='url(#p)'/>",
        ),
        "x",
        "1e999",
    );
}

#[test]
fn an_inherited_user_space_filter_units_is_in_class() {
    rejects(
        &doc("<defs><filter id='base' filterUnits='userSpaceOnUse'/>\
             <filter id='f' href='#base' x='1e999'><feGaussianBlur stdDeviation='1'/>\
             </filter></defs><rect width='10' height='10' filter='url(#f)'/>"),
        "x",
        "1e999",
    );
}
