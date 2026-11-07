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

/// The family the renderer substitutes for the generic `serif` at the end of every
/// requested list (`fontdb`'s default; see `fontdb::Database::new`). This is where
/// a missing name ends up when the library holds that family.
const SERIF_FALLBACK: &str = "Times New Roman";

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

/// Build the W8 warning when the first **non-generic** family in `requested` is not
/// in `available` (the families the font library actually holds).
///
/// Returns `None` when the request is satisfied, when every name in it is generic
/// (then the library's own aliases decide and nothing was asked for by name), and
/// when the caller had no library to compare against.
///
/// The line names the missing family, where the letters actually land and the
/// fix; *why* a fallback is not cosmetic is the shared note
/// [`super::NOTE_FONT_FALLBACK`], printed once per run.
pub fn describe_missing_font(requested: &str, available: &[String]) -> Option<String> {
    let names = parse_family_list(requested);
    let position = names.iter().position(|name| !is_generic(name))?;
    let missing = &names[position];
    if has(available, missing) {
        return None;
    }

    // Where the text actually lands: the next requested name the library has,
    // otherwise the renderer's trailing `serif` fallback.
    let requested_fallback = names[position + 1..]
        .iter()
        .find(|name| !is_generic(name) && has(available, name));
    let lands_on = match requested_fallback {
        Some(name) => format!("{name:?}"),
        None if has(available, SERIF_FALLBACK) => format!("{SERIF_FALLBACK:?}"),
        None => "whatever face it finds first".to_string(),
    };

    Some(super::fit(&[
        format!(
            "--label-fontfamily {missing:?} is not in the font library: the letters fall back to \
             {lands_on}. Fix: use a family from --list-fonts, or add it to fontlib/ \
             (fontlib/setup.sh copies Arial / Times New Roman)."
        ),
        format!(
            "--label-fontfamily {missing:?} is not in the font library: the letters fall back to \
             {lands_on}. Fix: use a family from --list-fonts or add it to fontlib/."
        ),
        format!(
            "--label-fontfamily {missing:?} is not in the font library: the letters fall back to \
             {lands_on}. Fix: see --list-fonts."
        ),
    ]))
}
