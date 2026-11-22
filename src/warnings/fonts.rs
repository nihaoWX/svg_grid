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
        // reserve the chain echo first so the message still names what the user
        // typed. The echo gets `min(chain, CHAIN_ECHO)` characters, and only when
        // that leaves no room for the list does it fall back to `MIN_ECHO`.
        //
        // Every budget below is measured from the string that is actually
        // formatted (`fixed.chars().count()`), never from hand-counted constants:
        // an earlier version counted `"(available: "` as 12 characters instead of
        // 13, which silently dropped the whole `available:` list.
        let suffix = format!("); {tail}");
        for echo_room in [chain.chars().count().min(CHAIN_ECHO), MIN_ECHO] {
            let head = PREFIX.len() + MIDDLE.len() + echo_room;
            let room = super::MAX_ERROR_BODY
                .saturating_sub(head + AVAILABLE_OPEN.len() + suffix.chars().count());
            if let Some(listed) = list_families_within(available, super::MAX_LISTED, room) {
                let fixed = format!("{AVAILABLE_OPEN}{listed}{suffix}");
                if let Some(body) = with_clamped_chain(&chain, &fixed) {
                    return Some(body);
                }
            }
        }
    }
    // Last resort: the shortest fix, but `available:` is still kept — it is the
    // actionable half of the message, so it is only given up when the library
    // itself cannot be named in any form. `truncate_body` is the unconditional
    // guarantee that no input can push the line over the cap.
    let suffix = "; Fix: --list-fonts.";
    let head = PREFIX.len() + MIDDLE.len() + MIN_ECHO;
    let room = super::MAX_ERROR_BODY.saturating_sub(head + AVAILABLE_OPEN.len() + suffix.len());
    let listed = list_families_within(available, 1, room).unwrap_or_else(|| "…".to_string());
    let fixed = format!("{AVAILABLE_OPEN}{listed}{suffix}");
    let body = with_clamped_chain(&chain, &fixed)
        .unwrap_or_else(|| format!("{PREFIX}\"…\"{MIDDLE}{fixed}"));
    Some(truncate_body(&body))
}

/// The body around the chain echo: `--label-fontfamily <echo> is not in the font
/// library<fixed>`.
const PREFIX: &str = "--label-fontfamily ";
const MIDDLE: &str = " is not in the font library";

/// The opening of the library list inside `fixed`. It is a named constant, and
/// every budget is measured from the formatted string, precisely because the
/// leading space is easy to forget — forgetting it once dropped the whole
/// `available:` list from the message.
const AVAILABLE_OPEN: &str = " (available: ";

/// The shortest chain echo, `"…"`, used to budget the rest of the line.
const MIN_ECHO: usize = 3;

/// How much of the chain echo to keep when the library list has to be shortened:
/// enough to recognise the requested family, never more than the chain itself.
const CHAIN_ECHO: usize = 24;

fn fits(body: &str) -> bool {
    body.chars().count() <= super::MAX_ERROR_BODY
}

/// Build the message with the chain echo shortened to the room `fixed` leaves.
/// Returns `None` when the result would not fit, so the caller can try a shorter
/// form — this check is the one guarantee that the ladder never emits an over-long
/// line, even if the echo arithmetic is edited later.
pub(super) fn with_clamped_chain(chain: &str, fixed: &str) -> Option<String> {
    let budget =
        super::MAX_ERROR_BODY.saturating_sub(PREFIX.len() + MIDDLE.len() + fixed.chars().count());
    let clamped = clamp_quoted(chain, budget);
    let body = format!("{PREFIX}{clamped}{MIDDLE}{fixed}");
    fits(&body).then_some(body)
}

/// The unconditional backstop: no input may push the line over the cap, even if a
/// future edit to the ladder above gets the arithmetic wrong. It is applied to the
/// last-resort form, whose budget is computed rather than hand-counted, so it
/// normally changes nothing; it exists so the cap holds by construction.
pub(super) fn truncate_body(body: &str) -> String {
    if fits(body) {
        return body.to_string();
    }
    let mut cut: String = body.chars().take(super::MAX_ERROR_BODY - 1).collect();
    cut.push('…');
    cut
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
pub(super) fn list_families_within(
    available: &[String],
    shown: usize,
    budget: usize,
) -> Option<String> {
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

/// Shorten a quoted echo to at most `budget` characters, keeping both quotes and
/// marking the cut with `…`. The cut prefers a family boundary (`"A, B, …"`) so the
/// echo never ends in the middle of a family name.
///
/// The wrapper adds exactly five characters (two quotes, `, ` and the ellipsis), so
/// keeping `budget - 5` inner characters makes the result exactly `budget` long —
/// that arithmetic is pinned by `every_clamped_echo_fits_its_budget`. When less
/// than five characters are available there is nothing to echo, and the caller's
/// `fits` check is what rejects the three-character `"…"` if even that is too wide.
pub(super) fn clamp_quoted(quoted: &str, budget: usize) -> String {
    if quoted.chars().count() <= budget {
        return quoted.to_string();
    }
    let inner = quoted.trim_matches('"');
    let keep = budget.saturating_sub(5);
    if keep == 0 {
        return "\"…\"".to_string();
    }
    let mut shortened: String = inner.chars().take(keep).collect();
    if let Some(cut) = shortened.rfind(',') {
        shortened.truncate(cut);
    }
    let shortened = shortened.trim_end_matches([',', ' ']);
    if shortened.is_empty() {
        return "\"…\"".to_string();
    }
    let clamped = format!("\"{shortened}, …\"");
    debug_assert!(
        clamped.chars().count() <= budget,
        "the echo must fit the budget it was given: {clamped:?} > {budget}"
    );
    clamped
}
