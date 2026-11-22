//! W8: a requested label font family that the font library does not have.
//!
//! A mistyped family name used to be completely silent: the renderer quietly fell
//! back to its own default, so the only symptom was "the letters are a bit
//! narrower than expected". The library now only holds what `fontlib/` provides,
//! and this warning says which requested name is missing and what it fell back to.

/// CSS generic families: they name no font file, so they are never checked against
/// the library (and never trigger this warning).
pub const GENERIC_FAMILIES: &[&str] = &[
    "sans-serif",
    "serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
];

/// The names of a CSS `font-family` list, in order, with quotes stripped.
pub fn parse_family_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .map(|name| name.trim_matches(['"', '\'']).trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

fn is_generic(name: &str) -> bool {
    GENERIC_FAMILIES
        .iter()
        .any(|generic| generic.eq_ignore_ascii_case(name))
}

fn has(available: &[String], name: &str) -> bool {
    available
        .iter()
        .any(|family| family.eq_ignore_ascii_case(name))
}

/// The hard-error form of W8 (D5): a missing label family
/// is "necessary information", so every output — SVG included — fails with the
/// font-error exit tier instead of silently falling back.
///
/// A CSS `font-family` list is a **fallback chain**, not a single name
/// (`ARCHITECTURE/fonts.md`): the first family that can actually be used wins, so
/// `"Helvetica, Arial, sans-serif"` is satisfied by `Arial` even though
/// `Helvetica` is not in the library. Only a chain with **no** usable name at all
/// is an error, and then the message names the whole chain, the families the
/// library does hold, and the fix. The user's `font-family` string is never
/// rewritten: the renderer performs the same fallback itself.
pub fn describe_missing_font_error(requested: &str, available: &[String]) -> Option<String> {
    let names = parse_family_list(requested);
    if names.is_empty() {
        return None;
    }
    if names
        .iter()
        .any(|name| is_generic(name) || has(available, name))
    {
        return None;
    }
    let chain = format!("{:?}", requested.trim());
    let tails = [
        "no family in the chain is usable. Fix: --list-fonts or fontlib/.",
        "no family is usable. Fix: --list-fonts.",
    ];
    // Richest rendering first: the full chain echo with the longest library list,
    // then shorter lists, then a clamped chain echo. Every candidate keeps the
    // `available:` list and the `--list-fonts` fix so the message stays actionable.
    for tail in tails {
        for shown in [super::MAX_LISTED, 3, 1] {
            let listed = list_families(available, shown);
            let body = format!(
                "--label-fontfamily {chain} is not in the font library (available: {listed}); {tail}"
            );
            if body.chars().count() <= super::MAX_ERROR_BODY {
                return Some(body);
            }
        }
        let listed = list_families(available, 1);
        let fixed = format!(" (available: {listed}); {tail}");
        let budget = super::MAX_ERROR_BODY.saturating_sub(fixed.chars().count() + HEAD_FIXED.len());
        if budget >= 4 {
            let clamped = clamp_quoted(&chain, budget);
            let body = format!("--label-fontfamily {clamped} is not in the font library{fixed}");
            if body.chars().count() <= super::MAX_ERROR_BODY {
                return Some(body);
            }
        }
    }
    // Unreachable in practice (the shortest tail always fits), but never emit a
    // line over the cap: fall back to the plainest actionable form.
    let listed = list_families(available, 1);
    let fixed = format!(" (available: {listed}); Fix: --list-fonts.");
    let budget = super::MAX_ERROR_BODY.saturating_sub(fixed.chars().count() + HEAD_FIXED.len());
    let clamped = clamp_quoted(&chain, budget.max(4));
    Some(format!(
        "--label-fontfamily {clamped} is not in the font library{fixed}"
    ))
}

/// The head with an empty chain echo, used to budget a clamped chain.
const HEAD_FIXED: &str = "--label-fontfamily  is not in the font library";

/// `Arial, Times New Roman, …`: at most `shown` families, with the rest folded.
fn list_families(available: &[String], shown: usize) -> String {
    if available.is_empty() {
        return "none".to_string();
    }
    let mut listed = available
        .iter()
        .take(shown)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if available.len() > shown {
        listed.push_str(", …");
    }
    listed
}

/// Shorten a quoted echo to `budget` characters, keeping both quotes and marking
/// the cut with `…`. The cut prefers a family boundary (`"A, B, …"`) so the echo
/// never ends in the middle of a family name.
fn clamp_quoted(quoted: &str, budget: usize) -> String {
    if quoted.chars().count() <= budget {
        return quoted.to_string();
    }
    if budget < 6 {
        return "\"…\"".to_string();
    }
    let inner = quoted.trim_matches('"');
    let keep = budget - 4; // two quotes, `, ` and the ellipsis
    let mut shortened: String = inner.chars().take(keep).collect();
    if let Some(cut) = shortened.rfind(',') {
        shortened.truncate(cut);
    }
    let shortened = shortened.trim_end_matches([',', ' ']);
    format!("\"{shortened}, …\"")
}
