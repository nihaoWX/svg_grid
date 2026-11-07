//! The `--only <glob>` matcher.
//!
//! Fixture ids are `<package>/<name>` (`ggplot2/annotate`), so the useful patterns
//! are short: `ggplot2/*`, `*heatmap`, `matplotlib/bars_*`. The matcher is
//! deliberately tiny — no character classes, no braces, no escaping:
//!
//! * `*` matches any run of characters (`/` included, so `*heatmap` finds
//!   `ggplot2/tile_heatmap`); `**` means the same thing,
//! * `?` matches exactly one character,
//! * everything else is literal.
//!
//! Matching is anchored on both ends: `--only ggplot2` selects nothing while
//! `--only 'ggplot2/*'` selects the package's fixtures, so a typo cannot silently
//! run half the corpus.

/// Whether `text` matches the glob `pattern`.
pub(crate) fn matches(pattern: &str, text: &str) -> bool {
    match_from(pattern.as_bytes(), text.as_bytes())
}

fn match_from(pattern: &[u8], text: &[u8]) -> bool {
    let Some((&head, rest)) = pattern.split_first() else {
        return text.is_empty();
    };
    match head {
        b'*' => {
            // `**` is the same thing: there is no path-segment rule here.
            let mut tail = rest;
            while tail.first() == Some(&b'*') {
                tail = &tail[1..];
            }
            (0..=text.len()).any(|end| match_from(tail, &text[end..]))
        }
        b'?' => match text.split_first() {
            Some((_, tail)) => match_from(rest, tail),
            None => false,
        },
        literal => match text.split_first() {
            Some((&next, tail)) => next == literal && match_from(rest, tail),
            None => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn a_star_crosses_path_segments() {
        assert!(matches("ggplot2/*", "ggplot2/annotate"));
        assert!(matches("ggplot2/*", "ggplot2/nested/annotate"));
        assert!(matches("*", "ggplot2/annotate"));
        assert!(matches("**", "ggplot2/annotate"));
        assert!(matches("**/annotate", "ggplot2/annotate"));
    }

    #[test]
    fn a_prefix_and_suffix_glob_both_work() {
        assert!(matches("*heatmap", "ggplot2/tile_heatmap"));
        assert!(matches("matplotlib/bars_*", "matplotlib/bars_grouped"));
        assert!(matches("matplotlib/bars_*", "matplotlib/bars_stacked"));
        assert!(!matches("matplotlib/bars_*", "ggplot2/bars_dodged"));
    }

    #[test]
    fn a_question_mark_takes_exactly_one_character() {
        assert!(matches("matplotlib/bar?", "matplotlib/barh"));
        assert!(!matches("matplotlib/bar?", "matplotlib/bar"));
        assert!(!matches("matplotlib/bar?", "matplotlib/barhs"));
    }

    #[test]
    fn matching_is_anchored_on_both_ends() {
        assert!(matches("ggplot2/annotate", "ggplot2/annotate"));
        assert!(!matches("ggplot2", "ggplot2/annotate"));
        assert!(!matches("annotate", "ggplot2/annotate"));
        assert!(!matches("ggplot2/annotate/", "ggplot2/annotate"));
    }
}
