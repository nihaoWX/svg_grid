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
    // then shorter lists, then a clamped chain echo, then a library list whose own
    // family names are shortened too. Every candidate keeps the `available:` list
    // and the `--list-fonts` fix so the message stays actionable; the ladder only
    // gives those up when the input itself is pathologically long.
    for tail in tails {
        for shown in [super::MAX_LISTED, 3, 1] {
            let listed = list_families(available, shown);
            let body = format!(
                "--label-fontfamily {chain} is not in the font library (available: {listed}); {tail}"
            );
            if fits(&body) {
                return Some(body);
            }
        }
        // Shortest library list first: the chain the user typed is worth more than
        // the tail of the library list, so give the echo the most room.
        for shown in [1, 3, super::MAX_LISTED] {
            let fixed = format!(" (available: {}); {tail}", list_families(available, shown));
            if let Some(body) = with_clamped_chain(&chain, &fixed) {
                return Some(body);
            }
        }
        // The library itself may hold very long family names: shorten them, but
        // keep enough room for the chain echo so the message still names what the
        // user typed.
        let suffix = format!("); {tail}");
        let overhead = PREFIX.len() + MIDDLE.len() + "(available: ".len() + suffix.chars().count();
        let chain_room = chain.chars().count().min(CHAIN_ECHO);
        let room = super::MAX_ERROR_BODY.saturating_sub(overhead + chain_room);
        if let Some(listed) = list_families_within(available, super::MAX_LISTED, room) {
            let fixed = format!(" (available: {listed}); {tail}");
            if let Some(body) = with_clamped_chain(&chain, &fixed) {
                return Some(body);
            }
        }
    }
    // Never emit a line over the cap: the plainest actionable form, with the chain
    // echo shortened to whatever room is left. `with_clamped_chain` always fits
    // here because the fixed part is short.
    with_clamped_chain(&chain, "; Fix: --list-fonts.")
        .or_else(|| Some(format!("{PREFIX}\"…\"{MIDDLE}; Fix: --list-fonts.")))
}

/// The body around the chain echo: `--label-fontfamily <echo> is not in the font
/// library<fixed>`.
const PREFIX: &str = "--label-fontfamily ";
const MIDDLE: &str = " is not in the font library";

/// The shortest chain echo, `"…"`, used to budget the rest of the line.
const MIN_ECHO: usize = 3;

/// How much of the chain echo to keep when the library list has to be shortened:
/// enough to recognise the requested family, never more than the chain itself.
const CHAIN_ECHO: usize = 24;

fn fits(body: &str) -> bool {
    body.chars().count() <= super::MAX_ERROR_BODY
}

/// Build the message with the chain echo shortened to the room `fixed` leaves.
/// Returns `None` when not even `"…"` fits, so the caller can try a shorter form.
fn with_clamped_chain(chain: &str, fixed: &str) -> Option<String> {
    let budget =
        super::MAX_ERROR_BODY.checked_sub(PREFIX.len() + MIDDLE.len() + fixed.chars().count())?;
    if budget < MIN_ECHO {
        return None;
    }
    let clamped = clamp_quoted(chain, budget);
    let body = format!("{PREFIX}{clamped}{MIDDLE}{fixed}");
    fits(&body).then_some(body)
}

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

/// [`list_families`] that also shortens individual family names so the whole list
/// fits `budget` characters: a name that does not fit keeps a recognisable prefix
/// and an ellipsis. Returns `None` when not even one shortened name fits.
fn list_families_within(available: &[String], shown: usize, budget: usize) -> Option<String> {
    if available.is_empty() {
        return (budget >= 4).then(|| "none".to_string());
    }
    let mut listed = String::new();
    for (index, family) in available.iter().take(shown).enumerate() {
        let separator = if index == 0 { 0 } else { 2 };
        let more = available.len() > index + 1;
        let reserve = if more { 3 } else { 0 };
        let room = budget.checked_sub(listed.chars().count() + separator + reserve)?;
        if room < 2 {
            break;
        }
        let truncated = family.chars().count() > room;
        let name: String = if truncated {
            let mut cut: String = family.chars().take(room - 1).collect();
            cut.push('…');
            cut
        } else {
            family.clone()
        };
        if index > 0 {
            listed.push_str(", ");
        }
        listed.push_str(&name);
        if truncated {
            break;
        }
    }
    if listed.is_empty() {
        return None;
    }
    if available.len() > shown {
        listed.push_str(", …");
    }
    (listed.chars().count() <= budget).then_some(listed)
}

/// Shorten a quoted echo to `budget` characters, keeping both quotes and marking
/// the cut with `…`. The cut prefers a family boundary (`"A, B, …"`) so the echo
/// never ends in the middle of a family name. The result is guaranteed to fit:
/// the wrapper adds five characters (two quotes, `, ` and the ellipsis).
fn clamp_quoted(quoted: &str, budget: usize) -> String {
    if quoted.chars().count() <= budget {
        return quoted.to_string();
    }
    if budget < 6 {
        return "\"…\"".to_string();
    }
    let inner = quoted.trim_matches('"');
    let keep = budget - 5;
    let mut shortened: String = inner.chars().take(keep).collect();
    if let Some(cut) = shortened.rfind(',') {
        shortened.truncate(cut);
    }
    let shortened = shortened.trim_end_matches([',', ' ']);
    if shortened.is_empty() {
        return "\"…\"".to_string();
    }
    let clamped = format!("\"{shortened}, …\"");
    if clamped.chars().count() <= budget {
        clamped
    } else {
        "\"…\"".to_string()
    }
}
