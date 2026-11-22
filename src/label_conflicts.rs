//! D5 W3: re-lettering a composite is a hard error, not a warning.
//!
//! Only the layer that owns a panel may place its letter; running `--labels`
//! again over an already-lettered input would duplicate the glyph and make the
//! figure ambiguous. Both the render path and `--plan` call this before writing
//! or printing anything.

use crate::plan::PreparedInput;
use crate::{dom, warnings, LabelSpec, Result, SvgGridError};

/// Reject re-lettering a composite: any input that already contains a panel
/// letter may not receive another non-empty one.
pub(crate) fn validate_label_conflicts(
    prepared: &[Option<PreparedInput>],
    labels: &LabelSpec,
) -> Result<()> {
    let offenders: Vec<(usize, String)> = prepared
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let item = item.as_ref()?;
            let label = labels.labels.get(index)?;
            (!label.is_empty() && contains_data_svg_grid_label(&item.root))
                .then(|| (index, label.clone()))
        })
        .collect();
    match warnings::describe_existing_labels(&offenders) {
        Some(message) => Err(SvgGridError::InvalidInput(message)),
        None => Ok(()),
    }
}

/// Whether an input subtree already contains a `data-svg-grid-label` marker,
/// i.e. it is (part of) a figure that a previous `plot_grid` call already lettered.
fn contains_data_svg_grid_label(element: &dom::Element) -> bool {
    element.attr("data-svg-grid-label").is_some()
        || element.children.iter().any(|child| match child {
            dom::Node::Element(child) => contains_data_svg_grid_label(child),
            dom::Node::Text(_) => false,
        })
}
