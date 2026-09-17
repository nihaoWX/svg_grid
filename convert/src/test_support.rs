//! Shared fixtures and helpers for the crate's unit tests.
//!
//! Declared `#[cfg(test)]` in `lib.rs`, so it is never part of a release build.

use crate::error::Finding;
use crate::options::{Conversion, Converted};
use crate::pipeline::convert;

/// Minimal "svglite-shaped" fixture: root with pt units + viewBox, a
/// `<style>` block, a `<clipPath>` with a rect, a 100%-sized background
/// rect, circles and texts, and svglite's rotated-axis-label idiom.
pub(crate) const SVGLITE: &str = r#"<?xml version='1.0' encoding='UTF-8' ?>
<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='288.00pt' height='216.00pt' viewBox='0 0 288.00 216.00'>
<g class='svglite'>
<defs>
  <style type='text/css'><![CDATA[
    .svglite circle { fill: none; stroke: #000000; }
  ]]></style>
</defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<defs>
  <clipPath id='cp1'>
    <rect x='17.92' y='24.17' width='258.74' height='238.72' />
  </clipPath>
</defs>
<g clip-path='url(#cp1)'>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; stroke: none; fill: #CFA15A;' />
<circle cx='63.98' cy='145.95' r='0.65' style='stroke-width: 0.77; stroke: none; fill: #E0907E;' />
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
<text transform='translate(12.72,143.53) rotate(-90)' text-anchor='middle' style='font-size: 8.80px;'>ylabel</text>
</g>
</g>
</svg>"#;

/// Minimal "matplotlib-shaped" fixture: geometry in `<path d>`, an embedded
/// `<image>`, and a non-rotate `<g transform>`.
pub(crate) const MATPLOTLIB: &str = r#"<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='640' height='480' viewBox='0 0 640 480'>
<g transform='translate(80.0,48.0) scale(1,-1)'>
<path d='M 0 0 L 10 10 L 20 5' fill='none' stroke='#000000'/>
</g>
<image x='0' y='0' width='10' height='10' xlink:href='data:image/png;base64,iVBORw0KGgo='/>
</svg>"#;

pub(crate) fn converted(source: &str) -> Converted {
    convert_with(source, false)
}

pub(crate) fn convert_with(source: &str, force: bool) -> Converted {
    match convert(source, force).expect("convert should not error") {
        Conversion::Converted(done) => *done,
        other => panic!("expected Converted, got {other:?}"),
    }
}

pub(crate) fn incompatible(source: &str) -> Vec<Finding> {
    match convert(source, false).expect("convert should not error") {
        Conversion::Incompatible(findings) => findings,
        other => panic!("expected Incompatible, got {other:?}"),
    }
}
