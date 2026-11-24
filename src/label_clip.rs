//! Does a `<symbol>` instance draw anything inside its `viewBox`?
//!
//! A `<symbol>` carrying a `viewBox` clips its content to that rectangle: with
//! resvg 0.47 a `<symbol viewBox="0 0 6 6">` holding a text at (4, 20) leaves ink
//! 0, while the same text without a `viewBox` leaves ink 23. The letter check
//! must not count a glyph the clip removes (R56 P3e).
//!
//! Only the whole-instance question is answered here: when the union of the
//! instance's ink boxes misses the `viewBox` rectangle nothing is drawn, and when
//! it intersects, every letter is kept. Partial clipping and the viewport scaling
//! a `<use>` `width`/`height` applies are not modelled — both can only remove
//! ink, so keeping the letter is the fail-open direction (registered in R56).

use crate::dom::{Element, Node};
use crate::ink::ink_bbox;

/// The `viewBox` rectangle as `(min_x, min_y, width, height)`.
///
/// A malformed or empty `viewBox` (not exactly four numbers, or a non-positive
/// size) is ignored, as the renderers do.
pub(crate) fn view_box(element: &Element) -> Option<(f64, f64, f64, f64)> {
    let mut values = [0.0_f64; 4];
    let mut count = 0;
    for part in element
        .attr("viewBox")?
        .split([' ', ',', '\t', '\n', '\r'])
        .filter(|part| !part.is_empty())
    {
        if count == values.len() {
            return None;
        }
        values[count] = part.parse().ok()?;
        count += 1;
    }
    if count != values.len() || values[2] <= 0.0 || values[3] <= 0.0 {
        return None;
    }
    Some((values[0], values[1], values[2], values[3]))
}

/// Does the instance of `symbol` leave any ink inside its `viewBox`?
///
/// `true` when the symbol has no usable `viewBox` (nothing is clipped).
pub(crate) fn instance_draws(symbol: &Element) -> bool {
    let Some((min_x, min_y, width, height)) = view_box(symbol) else {
        return true;
    };
    let clip = (min_x, min_y, min_x + width, min_y + height);
    symbol
        .children
        .iter()
        .filter_map(|child| match child {
            Node::Element(child) => ink_bbox(child),
            Node::Text(_) => None,
        })
        .any(|ink| intersects(ink, clip))
}

/// Do two `(min_x, min_y, max_x, max_y)` boxes overlap? Touching edges do not
/// count: a box that only reaches the clip border leaves no pixel inside.
fn intersects(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(markup: &str) -> Element {
        crate::dom::parse(markup).expect("test markup parses")
    }

    #[test]
    fn a_view_box_is_four_numbers() {
        let element = symbol("<symbol viewBox=\"0 0 6 6\"/>");
        assert_eq!(view_box(&element), Some((0.0, 0.0, 6.0, 6.0)));
        let commas = symbol("<symbol viewBox=\"10,10,6,6\"/>");
        assert_eq!(view_box(&commas), Some((10.0, 10.0, 6.0, 6.0)));
        for bad in [
            "<symbol/>",
            "<symbol viewBox=\"0 0 6\"/>",
            "<symbol viewBox=\"0 0 6 6 6\"/>",
            "<symbol viewBox=\"0 0 0 6\"/>",
            "<symbol viewBox=\"0 0 6 -1\"/>",
            "<symbol viewBox=\"a b c d\"/>",
        ] {
            assert_eq!(view_box(&symbol(bad)), None, "{bad}");
        }
    }

    #[test]
    fn a_clipped_away_text_leaves_no_ink() {
        // Measured with resvg 0.47: ink 0 for the 6x6 box, ink 91 for the 20x20
        // one (the text's estimated box is x 4..13, y 11..23).
        let clipped = symbol(
            "<symbol viewBox=\"0 0 6 6\">\
             <text x=\"4\" y=\"20\" font-size=\"12\" font-family=\"Arial\">A</text></symbol>",
        );
        assert!(!instance_draws(&clipped));
        let visible = symbol(
            "<symbol viewBox=\"0 0 20 20\">\
             <text x=\"4\" y=\"20\" font-size=\"12\" font-family=\"Arial\">A</text></symbol>",
        );
        assert!(instance_draws(&visible));
        let unclipped = symbol(
            "<symbol>\
             <text x=\"4\" y=\"20\" font-size=\"12\" font-family=\"Arial\">A</text></symbol>",
        );
        assert!(instance_draws(&unclipped));
    }

    #[test]
    fn a_view_box_origin_shifts_the_clip() {
        // resvg clips to the rectangle at (min_x, min_y): a rect at (10, 10) is
        // drawn inside `viewBox="10 10 6 6"`, one at the origin is not.
        let inside = symbol(
            "<symbol viewBox=\"10 10 6 6\">\
             <rect x=\"10\" y=\"10\" width=\"6\" height=\"6\" fill=\"black\"/></symbol>",
        );
        assert!(instance_draws(&inside));
        let outside = symbol(
            "<symbol viewBox=\"10 10 6 6\">\
             <rect x=\"0\" y=\"0\" width=\"6\" height=\"6\" fill=\"black\"/></symbol>",
        );
        assert!(!instance_draws(&outside));
    }
}
