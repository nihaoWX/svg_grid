//! W1 (an input rescaled by the layout) and W2 (a cell stretched away from its
//! panel's aspect ratio).
//!
//! Both warnings only report *numbers*: why a rescale or a stretch matters (the
//! tag protocol never rescales `font-size`) is a cause they share, so it lives in
//! [`NOTE_FONT_SIZE`] and is printed once per run.

use super::{assemble, counted, ASPECT_TOLERANCE, SCALE_TOLERANCE};

/// One input's resolved layout scale: `sx = cell.width / canvas.width`,
/// `sy = cell.height / canvas.height`.
#[derive(Debug, Clone, Copy)]
pub struct InputScale {
    pub index: usize,
    pub sx: f32,
    pub sy: f32,
}

/// One drawn cell's aspect ratio compared with its panel's canvas aspect ratio.
#[derive(Debug, Clone, Copy)]
pub struct CellStretch {
    pub index: usize,
    pub cell_aspect: f32,
    pub panel_aspect: f32,
}

/// Warn when the layout rescales an input away from 1.
///
/// All offending inputs are collapsed into a single line; [`NOTE_FONT_SIZE`] is
/// what explains *why* a rescale matters.
pub fn describe_scale(scales: &[InputScale]) -> Option<String> {
    let mut with_percent = Vec::new();
    let mut detailed = Vec::new();
    let mut plain = Vec::new();
    for scale in scales {
        if (scale.sx - 1.0).abs() <= SCALE_TOLERANCE && (scale.sy - 1.0).abs() <= SCALE_TOLERANCE {
            continue;
        }
        let deviation = (scale.sx - 1.0).abs().max((scale.sy - 1.0).abs());
        // One shared scale prints once: `sx = sy = 10` only needs one number.
        if (scale.sx - scale.sy).abs() < 1e-5 {
            with_percent.push(format!(
                "#{} sx={:.5} (off by {:.2}%)",
                scale.index,
                scale.sx,
                deviation * 100.0
            ));
            detailed.push(format!("#{} sx={:.5}", scale.index, scale.sx));
        } else {
            with_percent.push(format!(
                "#{} sx={:.5} sy={:.5} (off by {:.2}%)",
                scale.index,
                scale.sx,
                scale.sy,
                deviation * 100.0
            ));
            detailed.push(format!(
                "#{} sx={:.5} sy={:.5}",
                scale.index, scale.sx, scale.sy
            ));
        }
        plain.push(format!("#{} sx={:.5}", scale.index, scale.sx));
    }
    if detailed.is_empty() {
        return None;
    }
    let head = format!(
        "{} scaled by the layout: ",
        counted(detailed.len(), "input is", "inputs are")
    );
    let tails = [
        ". Fix: draw it at its cell size (--normalize-input-max-side) or use --plot-margin 0.",
        ". Fix: draw it at its cell size or use --plot-margin 0.",
    ];
    // The derived percentage is the first thing to go, then the second scale
    // factor: the `Fix:` never is (`ARCHITECTURE/warnings.md` §1 rule 1).
    for items in [&with_percent, &detailed, &plain] {
        let list = super::fold(items, items.len().min(super::MAX_LISTED));
        for tail in &tails {
            let body = format!("{head}{list}{tail}");
            if body.chars().count() <= super::MAX_BODY {
                return Some(body);
            }
        }
    }
    Some(assemble(&head, &plain, &plain, &tails[1..]))
}

/// Warn when a cell's aspect ratio does not match its panel's, i.e. the panel is
/// stretched.
pub fn describe_stretch(cells: &[CellStretch]) -> Option<String> {
    let mut detailed = Vec::new();
    let mut plain = Vec::new();
    for cell in cells {
        if cell.panel_aspect <= 0.0 || cell.cell_aspect <= 0.0 {
            continue;
        }
        let stretch = cell.cell_aspect / cell.panel_aspect - 1.0;
        if stretch.abs() <= ASPECT_TOLERANCE {
            continue;
        }
        detailed.push(format!(
            "#{} aspect {:.3} vs panel {:.3} (stretched {:+.1}%)",
            cell.index,
            cell.cell_aspect,
            cell.panel_aspect,
            stretch * 100.0
        ));
        plain.push(format!(
            "#{} (stretched {:+.1}%)",
            cell.index,
            stretch * 100.0
        ));
    }
    if detailed.is_empty() {
        return None;
    }
    let head = format!(
        "{} stretched: ",
        counted(detailed.len(), "cell is", "cells are")
    );
    Some(assemble(
        &head,
        &detailed,
        &plain,
        &[
            ". Fix: omit --rel-* so the cell matches the panel.",
            ". Fix: omit --rel-* or pass matching --rel-*.",
        ],
    ))
}
