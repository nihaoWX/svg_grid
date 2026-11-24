//! Panel letters must be unique in the finished figure.
//!
//! Two different accidents break that rule, and both are hard errors:
//!
//! - **W3 (D5)**: *re-lettering* an input that already carries letters — the
//!   classic nesting mistake (`--labels` over a composite). Only the layer that
//!   owns a panel may place its letter.
//! - **R44**: assembling composites that were lettered *separately*, with no
//!   `--labels` this time. Each layer keeps its own letters, so the output can
//!   show two panels answering to "A" with `rc=0` and an empty stderr. The
//!   letters that survive into the output are the inputs' own
//!   `data-svg-grid-label` markers plus any letter this call assigns, so that
//!   multiset is what has to be collision-free.
//!
//! Duplicates among *explicit* `--labels` entries stay a warning (W4b): there the
//! user wrote the collision on purpose, while an inherited collision is always an
//! accident across layers (`INSTRUCTIONS.md` lists "重复 labels" under 必要信息).
//!
//! Both the render path and `--plan` call this before writing or printing
//! anything, so a rejected call leaves no output file and no plan behind.

use std::collections::HashMap;

use crate::dom;
use crate::label_letters::collect_letters;
use crate::plan::PreparedInput;
use crate::warnings::LetterSite;
use crate::{warnings, LabelSpec, Result, SvgGridError};

/// Reject a figure whose panel letters do not name exactly one panel each.
pub(crate) fn validate_label_conflicts(
    prepared: &[Option<PreparedInput>],
    labels: Option<&LabelSpec>,
) -> Result<()> {
    if let Some(labels) = labels {
        if let Some(message) = warnings::describe_existing_labels(&relettered(prepared, labels)) {
            return Err(SvgGridError::InvalidInput(message));
        }
    }
    if let Some(message) = warnings::describe_duplicate_panel_letters(&collisions(prepared, labels))
    {
        return Err(SvgGridError::InvalidInput(message));
    }
    Ok(())
}

/// W3: the inputs this call would letter *again*.
fn relettered(prepared: &[Option<PreparedInput>], labels: &LabelSpec) -> Vec<(usize, String)> {
    prepared
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let item = item.as_ref()?;
            let label = labels.labels.get(index)?;
            (!label.is_empty() && contains_data_svg_grid_label(&item.root))
                .then(|| (index, label.clone()))
        })
        .collect()
}

/// R44: letters that end up in the output more than once, when at least one of
/// those places is an input's own letter (a `--labels A,A` collision is W4b's
/// job, not ours).
fn collisions(
    prepared: &[Option<PreparedInput>],
    labels: Option<&LabelSpec>,
) -> Vec<(String, Vec<LetterSite>)> {
    let mut sites: Vec<(String, LetterSite)> = Vec::new();
    for (index, item) in prepared.iter().enumerate() {
        let Some(item) = item else {
            // A spacer draws no letter of its own; an explicit `--labels` entry
            // for the blank cell is still drawn, and is collected below.
            continue;
        };
        for marker in collect_letters(&item.root) {
            sites.push((
                marker.letter,
                LetterSite {
                    input: index,
                    // The marker value is the cell index *inside* this input, so
                    // a nested composite reports the real cell instead of the
                    // input's own index (R48).
                    cell: marker.marker.parse::<usize>().ok(),
                    inner: Some(marker.marker),
                },
            ));
        }
    }
    if let Some(labels) = labels {
        for (index, label) in labels.labels.iter().enumerate().take(prepared.len()) {
            let letter = crate::normalize_label(label);
            if letter.is_empty() {
                continue;
            }
            sites.push((
                letter,
                LetterSite {
                    input: index,
                    cell: Some(index),
                    inner: None,
                },
            ));
        }
    }

    // First-seen order keeps the message deterministic (input order, then the
    // order the letters appear in the file). The map makes grouping O(n): the
    // old linear `find` was O(n²) and took 1.6 s for 40000 distinct letters.
    let mut groups: Vec<(String, Vec<LetterSite>)> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (letter, site) in &sites {
        match index.get(letter.as_str()) {
            Some(&group) => groups[group].1.push(site.clone()),
            None => {
                index.insert(letter.as_str(), groups.len());
                groups.push((letter.clone(), vec![site.clone()]));
            }
        }
    }
    groups
        .into_iter()
        .filter(|(_, places)| places.len() > 1 && places.iter().any(LetterSite::inherited))
        .collect()
}

/// Walk one input and collect its rendered letters, in document order. The
/// renderer semantics (nested markers, `<use>` instancing, paint) live in
/// [`crate::label_letters`].
/// Whether an input subtree already contains a `data-svg-grid-label` marker.
fn contains_data_svg_grid_label(element: &dom::Element) -> bool {
    element.attr("data-svg-grid-label").is_some()
        || element.children.iter().any(|child| match child {
            dom::Node::Element(child) => contains_data_svg_grid_label(child),
            dom::Node::Text(_) => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;

    fn prepared(inputs: &[&str]) -> Vec<Option<PreparedInput>> {
        inputs
            .iter()
            .map(|markup| {
                Some(PreparedInput {
                    root: dom::parse(markup).expect("test markup parses"),
                    panel: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 80.0,
                    },
                    canvas: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 80.0,
                    },
                    auto_converted: None,
                })
            })
            .collect()
    }

    fn spec(labels: &[&str]) -> LabelSpec {
        LabelSpec {
            labels: labels.iter().map(|label| label.to_string()).collect(),
            x: Vec::new(),
            y: Vec::new(),
            hjust: Vec::new(),
            vjust: Vec::new(),
            size: crate::LabelSize::Auto,
            font_family: "serif".to_string(),
            font_face: String::new(),
            colour: "#000000".to_string(),
        }
    }

    fn lettered(letter: &str) -> String {
        format!("<g><text data-svg-grid-label=\"0\">{letter}</text></g>")
    }

    fn plain() -> &'static str {
        "<g><rect width=\"100\" height=\"80\"/></g>"
    }

    #[test]
    fn separately_lettered_composites_are_rejected() {
        let err = validate_label_conflicts(&prepared(&[&lettered("A"), &lettered("A")]), None)
            .expect_err("two inputs both carrying A must be rejected");
        let message = err.to_string();
        assert!(
            message.contains("\"A\"") && message.contains("#0") && message.contains("#1"),
            "message must name the letter and both inputs: {message}"
        );
        assert!(
            message.chars().count() <= 200,
            "one stderr line, prefix included: {message}"
        );
    }

    #[test]
    fn unique_letters_across_inputs_are_accepted() {
        validate_label_conflicts(&prepared(&[&lettered("A"), &lettered("B")]), None)
            .expect("A + B is fine");
    }

    #[test]
    fn a_lettered_input_next_to_an_unlettered_one_is_accepted() {
        validate_label_conflicts(&prepared(&[&lettered("A"), plain()]), None)
            .expect("one composite + one plain panel is fine");
    }

    #[test]
    fn a_single_lettered_composite_is_accepted() {
        validate_label_conflicts(&prepared(&[&lettered("A")]), None)
            .expect("a lone composite is fine");
    }

    #[test]
    fn duplicates_inside_one_input_are_rejected() {
        let both = "<g><text data-svg-grid-label=\"0\">A</text>\
                    <text data-svg-grid-label=\"1\">A</text></g>";
        let err = validate_label_conflicts(&prepared(&[both]), None)
            .expect_err("one input carrying A twice must be rejected");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn explicit_duplicate_labels_stay_a_warning() {
        // W4b's case: the user wrote `--labels A,A` on two plain inputs.
        validate_label_conflicts(&prepared(&[plain(), plain()]), Some(&spec(&["A", "A"])))
            .expect("--labels A,A is the user's own choice (W4b warns, rc=0)");
    }

    #[test]
    fn tspan_wrapped_letters_are_collected_recursively() {
        let tspan = "<g><text data-svg-grid-label=\"0\"><tspan>A</tspan></text></g>";
        let err = validate_label_conflicts(&prepared(&[tspan, tspan]), None)
            .expect_err("matplotlib/ggplot2 split the letter into a tspan");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn a_group_marker_collects_the_text_below_it() {
        let group = "<g data-svg-grid-label=\"0\"><text>A</text></g>";
        let err = validate_label_conflicts(&prepared(&[group, group]), None)
            .expect_err("a marker on a <g> must read the text inside it");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn text_is_concatenated_in_document_order() {
        let inline = "<g><text data-svg-grid-label=\"0\">A<tspan>B</tspan></text></g>";
        let nested =
            "<g><text data-svg-grid-label=\"0\"><tspan>A</tspan><tspan>B</tspan></text></g>";
        let err = validate_label_conflicts(&prepared(&[inline, nested]), None)
            .expect_err("both spell AB");
        assert!(err.to_string().contains("\"AB\""), "{err}");
    }

    #[test]
    fn surrounding_whitespace_is_normalized_away() {
        let padded = "<g><text data-svg-grid-label=\"0\"> A </text></g>";
        let err = validate_label_conflicts(&prepared(&[&lettered("A"), padded]), None)
            .expect_err("A and ' A ' render identically");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn an_empty_marker_is_not_a_letter() {
        let empty = "<g><text data-svg-grid-label=\"0\">   </text></g>";
        validate_label_conflicts(&prepared(&[empty, &lettered("A")]), None)
            .expect("a marker that draws nothing must not collide");
    }

    #[test]
    fn markers_in_non_rendered_subtrees_are_ignored() {
        for wrapper in [
            "<defs><text data-svg-grid-label=\"0\">A</text></defs>",
            "<symbol><text data-svg-grid-label=\"0\">A</text></symbol>",
            "<g display=\"none\"><text data-svg-grid-label=\"0\">A</text></g>",
            "<g visibility=\"hidden\"><text data-svg-grid-label=\"0\">A</text></g>",
            "<g opacity=\"0\"><text data-svg-grid-label=\"0\">A</text></g>",
        ] {
            validate_label_conflicts(&prepared(&[wrapper, &lettered("A")]), None)
                .unwrap_or_else(|err| panic!("{wrapper} is not rendered: {err}"));
        }
    }

    #[test]
    fn visibility_can_be_restored_by_a_descendant() {
        let restored = "<g visibility=\"hidden\"><text data-svg-grid-label=\"0\" \
                        visibility=\"visible\">A</text></g>";
        let err = validate_label_conflicts(&prepared(&[restored, &lettered("A")]), None)
            .expect_err("visibility=visible on the text makes it render");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn display_none_cannot_be_restored_by_a_descendant() {
        let hidden = "<g display=\"none\"><text data-svg-grid-label=\"0\" \
                      display=\"inline\">A</text></g>";
        validate_label_conflicts(&prepared(&[hidden, &lettered("A")]), None)
            .expect("display:none removes the subtree, whatever the child says");
    }

    #[test]
    fn the_reported_cell_is_the_marker_value_not_the_input_index() {
        let nested = "<g><text data-svg-grid-label=\"1\">A</text></g>";
        let err = validate_label_conflicts(&prepared(&[nested, &lettered("A")]), None)
            .expect_err("A at cell 1 and A at cell 0 collide");
        let message = err.to_string();
        assert!(message.contains("#0 (cell 1)"), "{message}");
        assert!(message.contains("#1 (cell 0)"), "{message}");
    }

    #[test]
    fn many_colliding_sites_keep_an_explicit_remainder() {
        let inputs: Vec<String> = (0..6).map(|_| lettered("A")).collect();
        let refs: Vec<&str> = inputs.iter().map(String::as_str).collect();
        let err = validate_label_conflicts(&prepared(&refs), None).expect_err("six A's collide");
        let message = err.to_string();
        assert!(message.chars().count() <= 200, "{message}");
        assert!(
            message.contains("+3 more") || message.contains("(6 places)"),
            "the omitted sites must be counted, not dropped: {message}"
        );
    }

    #[test]
    fn explicit_labels_are_normalized_like_inherited_ones() {
        let err = validate_label_conflicts(
            &prepared(&[&lettered("A"), plain()]),
            Some(&spec(&["", " A "])),
        )
        .expect_err("' A ' is the same letter as A");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn an_explicit_letter_on_a_spacer_still_collides() {
        // A spacer draws no letter of its own, but an explicit --labels entry
        // for that cell is drawn (R48 verified with d1render: 25 ink pixels).
        let mut inputs = prepared(&[&lettered("A")]);
        inputs.push(None);
        let err = validate_label_conflicts(&inputs, Some(&spec(&["", "A"])))
            .expect_err("the spacer's A renders, so it collides with the inherited A");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }

    #[test]
    fn an_explicit_letter_that_collides_with_an_inherited_one_is_rejected() {
        // W3 does not fire: input #1 is plain, so only the R44 check can see it.
        let err = validate_label_conflicts(
            &prepared(&[&lettered("A"), plain()]),
            Some(&spec(&["", "A"])),
        )
        .expect_err("an assigned A colliding with an inherited A is still a collision");
        assert!(err.to_string().contains("\"A\""), "{err}");
    }
}
