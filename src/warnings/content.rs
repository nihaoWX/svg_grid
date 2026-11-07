//! W6: an input whose ink crosses its own canvas box.

use crate::geom::Rect;

use super::{fit, CONTENT_OUTSIDE_DUST, CONTENT_OUTSIDE_TOLERANCE, DUST_HINT};

/// Build the content-outside-canvas warning (W6) for one input, given its ink
/// bounding box `(min_x, min_y, max_x, max_y)` and its own canvas box.
///
/// Reported whenever the ink crosses a canvas edge by more than
/// [`CONTENT_OUTSIDE_TOLERANCE`] — deliberately *not* filtered by whether the
/// overflow would survive composition, because the silent failure (a raster
/// export quietly dropping glyphs) is exactly what must be surfaced.
///
/// The line names the side and the amount plus the ink coordinate that left the
/// canvas, and gives the fix; *what those consequences are* is the shared cause
/// note [`super::NOTE_CANVAS_INK`], printed once per run. The "dust" caveat is
/// [`super::NOTE_INK_DUST`]; the line itself only flags it with
/// [`super::DUST_HINT`].
pub fn describe_content_outside_canvas(
    index: usize,
    ink: (f64, f64, f64, f64),
    canvas: Rect,
) -> Option<String> {
    let (min_x, min_y, max_x, max_y) = ink;
    let canvas_x0 = canvas.x as f64;
    let canvas_y0 = canvas.y as f64;
    let canvas_x1 = canvas.x as f64 + canvas.width as f64;
    let canvas_y1 = canvas.y as f64 + canvas.height as f64;

    // (side, amount past that edge, the ink coordinate that left the canvas)
    let mut overflows: Vec<(&str, f64, String)> = Vec::new();
    let mut max_overflow = 0.0_f64;
    for (side, amount, reach) in [
        ("left", canvas_x0 - min_x, format!("ink x={min_x:.3}")),
        ("top", canvas_y0 - min_y, format!("ink y={min_y:.3}")),
        ("right", max_x - canvas_x1, format!("ink x={max_x:.3}")),
        ("bottom", max_y - canvas_y1, format!("ink y={max_y:.3}")),
    ] {
        if amount > CONTENT_OUTSIDE_TOLERANCE {
            max_overflow = max_overflow.max(amount);
            overflows.push((side, amount, reach));
        }
    }
    if overflows.is_empty() {
        return None;
    }

    // Self-priority hint: if every overflow is mere "dust" (below
    // `CONTENT_OUTSIDE_DUST`) say so — the evidence is real but usually absorbed
    // by the margin. A single overflow of 2 px or more suppresses the hint.
    let dust = if max_overflow < CONTENT_OUTSIDE_DUST {
        DUST_HINT
    } else {
        ""
    };

    let amounts: Vec<String> = overflows
        .iter()
        .map(|(side, amount, _)| format!("{side} by {amount:.3} px"))
        .collect();
    let reaches: Vec<&str> = overflows
        .iter()
        .map(|(_, _, reach)| reach.as_str())
        .collect();
    let head = format!("input #{index} draws ink outside its own canvas box (overflows ");
    let fix = " Fix: more margin at the plotting end, or less on that side.";
    // Every overflowing side with its ink coordinate, then every side without it,
    // then the worst side plus a count. A canvas with several overflowing sides
    // still names the one that matters most (`ARCHITECTURE/warnings.md` §1 rule 1).
    let worst = overflows
        .iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .expect("non-empty");
    let rest = overflows.len() - 1;
    let folded = if rest == 0 {
        format!("{} by {:.3} px", worst.0, worst.1)
    } else {
        format!(
            "{} by {:.3} px (worst of {rest} more sides)",
            worst.0, worst.1
        )
    };
    Some(fit(&[
        format!(
            "{head}{}; {}).{fix}{dust}",
            amounts.join(", "),
            reaches.join(", ")
        ),
        format!("{head}{}).{fix}{dust}", amounts.join(", ")),
        format!("{head}{folded}).{fix}{dust}"),
    ]))
}
