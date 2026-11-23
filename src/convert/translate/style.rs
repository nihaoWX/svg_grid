//! `style="name: value; ..."` declaration-list helpers.
//!
//! Kept in one place because three stages read the same inline style: the
//! `<use>` merge (`uses`), the `<path>` rewrite (`shapes`) and the `<use>`
//! inheritance logic.

/// Read one property from a `style="a: b; c: d"` declaration list. When a
/// property is declared more than once, the last declaration wins (CSS order).
pub(crate) fn style_value(style: &str, name: &str) -> Option<String> {
    parse_style(style)
        .into_iter()
        .rev()
        .find(|(property, _)| property == name)
        .map(|(_, value)| value)
}

/// Parse a `style="a: b; c: d"` declaration list into ordered pairs.
pub(crate) fn parse_style(style: &str) -> Vec<(String, String)> {
    style
        .split(';')
        .filter_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some((name.to_string(), value.trim().to_string()))
        })
        .collect()
}

pub(super) fn serialize_style(properties: &[(String, String)]) -> String {
    properties
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) fn style_without(style: &str, drop: &str) -> Option<String> {
    let parsed = parse_style(style);
    if parsed.is_empty() {
        return None;
    }
    let kept: Vec<(String, String)> = parsed
        .into_iter()
        .filter(|(name, _)| name != drop)
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(serialize_style(&kept))
    }
}
