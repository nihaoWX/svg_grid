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
pub fn describe_missing_font_error(requested: &str, available: &[String]) -> Option<String> {
    let names = parse_family_list(requested);
    let position = names.iter().position(|name| !is_generic(name))?;
    let missing = &names[position];
    if has(available, missing) {
        return None;
    }
    let shown: Vec<&str> = available.iter().take(6).map(String::as_str).collect();
    let mut listed = shown.join(", ");
    if available.len() > shown.len() {
        listed.push_str(", …");
    }
    Some(super::fit(&[format!(
        "--label-fontfamily {missing:?} is not in the font library (available: {listed}). \
         Fix: use a family from --list-fonts, or add it to fontlib/ (fontlib/setup.sh \
         copies Arial / Times New Roman)."
    )]))
}
