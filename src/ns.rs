//! Id namespacing: give every element and every internal reference a per-cell prefix.
//!
//! The composer renames ids (`id="base"` -> `id="svg-grid-cell0-base"`) so several
//! panels can share one output document; every reference has to be renamed with them.

use crate::dom;

/// Namespace one internal IRI, keeping its spelling.
///
/// The renderer accepts `" #base"` and `"#base "` — `svgtypes::Stream::parse_iri`
/// skips literal spaces around the fragment, and a tab is *not* a space, so
/// `"#\tbase"` names a different id — so those forms have to be rewritten too, or
/// the reference dangles as soon as the ids carry the cell prefix (round-data
/// R43; measured ink 2160 -> 0 before this).
///
/// Anything that is not an internal IRI (another document, a bare `url(…)` with a
/// path, garbage) is left exactly as written.
fn namespace_iri(value: &str, prefix: &str) -> Option<String> {
    let rest = value.trim_start_matches(' ');
    let fragment = rest.strip_prefix('#')?;
    let end = fragment.find(' ').unwrap_or(fragment.len());
    let (id, tail) = fragment.split_at(end);
    if id.is_empty() || !tail.bytes().all(|byte| byte == b' ') {
        return None;
    }
    let leading = &value[..value.len() - rest.len()];
    Some(format!("{leading}#{prefix}{id}{tail}"))
}

/// Namespace every internal `url(#id)` reference inside an attribute value.
fn namespace_iri_references(value: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("url(") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        let Some(close) = after.find(')') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..close];
        out.push_str("url(");
        match namespace_iri(inner, prefix) {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(inner),
        }
        out.push(')');
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

pub(crate) fn namespace_ids(element: &mut dom::Element, prefix: &str) {
    for (name, value) in &mut element.attrs {
        if name == "id" {
            *value = format!("{prefix}{value}");
        } else if name == "href" || name == "xlink:href" {
            if let Some(rewritten) = namespace_iri(value, prefix) {
                *value = rewritten;
            }
        } else if value.contains("url(") {
            *value = namespace_iri_references(value, prefix);
        }
    }
    for child in &mut element.children {
        if let dom::Node::Element(child) = child {
            namespace_ids(child, prefix);
        }
    }
}
