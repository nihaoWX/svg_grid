//! `style="name: value; ..."` declaration-list helpers.
//!
//! The composer used to rewrite only presentation *attributes*, but both dialects
//! we consume put scale-relevant lengths inside the inline `style="..."` attribute:
//! matplotlib writes `font-size`/`stroke-width` there (`style="... stroke-width: 0.8; ..."`)
//! and svglite writes `font-size` there (`style="font-size: 8.8px;"`). These helpers
//! read and rewrite a single numeric *length* declaration in place.
//!
//! The XML parser has already resolved the attribute's own quoting (a source
//! `style='...'` and `style="..."` both arrive here as the same inner string), so
//! only the declaration list itself is handled here.

/// Rewrite the value(s) of the property `prop` inside a CSS declaration list by
/// multiplying every numeric token by `factor`, keeping each token's unit suffix
/// verbatim (`10px` → `20px`, `8.8px` → `17.6px`, `0.8` → `1.6`, `4 2` → `8 4`).
///
/// Properties of the contract:
/// * declarations are separated by `;`, each is `name : value`;
/// * **only** the targeted declaration is touched — every other declaration, the
///   declaration order and all surrounding whitespace are preserved byte-for-byte;
/// * tokens whose leading number cannot be parsed (`none`, `middle`, `url(#grad)`)
///   are left as they are, so an unparseable value is never corrupted;
/// * returns `None` when the property is not declared, or when none of its tokens
///   yielded a parseable number (the input is then left completely unchanged).
pub(crate) fn scale_length(style: &str, prop: &str, factor: f32) -> Option<String> {
    let mut out = String::with_capacity(style.len() + 8);
    let mut changed = false;
    let mut rest = style;

    loop {
        let (declaration, remainder) = match rest.split_once(';') {
            Some((declaration, remainder)) => (declaration, Some(remainder)),
            None => (rest, None),
        };
        match rewrite_declaration(declaration, prop, factor) {
            Some(rewritten) => {
                out.push_str(&rewritten);
                changed = true;
            }
            None => out.push_str(declaration),
        }
        match remainder {
            Some(remainder) => {
                out.push(';');
                rest = remainder;
            }
            None => break,
        }
    }

    if changed {
        Some(out)
    } else {
        None
    }
}

/// Rewrite one `name: value` declaration when `name` (trimmed) equals `prop`.
fn rewrite_declaration(declaration: &str, prop: &str, factor: f32) -> Option<String> {
    let (name, value) = declaration.split_once(':')?;
    if name.trim() != prop {
        return None;
    }
    let value = scale_value(value, factor)?;
    Some(format!("{name}:{value}"))
}

/// Scale every numeric token in a declaration value, preserving the separators
/// (whitespace / commas) verbatim.
fn scale_value(value: &str, factor: f32) -> Option<String> {
    let mut out = String::with_capacity(value.len() + 8);
    let mut changed = false;
    let mut rest = value;

    while !rest.is_empty() {
        let separator_len = rest
            .find(|ch: char| !(ch.is_whitespace() || ch == ','))
            .unwrap_or(rest.len());
        out.push_str(&rest[..separator_len]);
        rest = &rest[separator_len..];
        if rest.is_empty() {
            break;
        }

        let token_len = rest
            .find(|ch: char| ch.is_whitespace() || ch == ',')
            .unwrap_or(rest.len());
        let token = &rest[..token_len];
        match scale_token(token, factor) {
            Some(scaled) => {
                out.push_str(&scaled);
                changed = true;
            }
            None => out.push_str(token),
        }
        rest = &rest[token_len..];
    }

    if changed {
        Some(out)
    } else {
        None
    }
}

/// Multiply the numeric prefix of a length token by `factor`, keeping whatever
/// follows (the unit, or a stray suffix) verbatim. Returns `None` for a token with
/// no parseable numeric prefix.
fn scale_token(token: &str, factor: f32) -> Option<String> {
    // Longest prefix (by character boundary) that parses as a number. `char_indices`
    // never yields `token.len()`, so the full-token case is handled separately.
    let (number_end, value) = match token.parse::<f32>() {
        Ok(value) => (token.len(), value),
        Err(_) => {
            let mut best = None;
            for (index, _) in token.char_indices() {
                if index == 0 {
                    continue;
                }
                if let Ok(value) = token[..index].parse::<f32>() {
                    best = Some((index, value));
                }
            }
            best?
        }
    };
    let scaled = crate::format_number(value * factor);
    Some(format!("{scaled}{}", &token[number_end..]))
}

#[cfg(test)]
mod tests {
    use super::scale_length;

    #[test]
    fn scales_a_single_px_length_keeping_the_unit() {
        assert_eq!(
            scale_length("font-size: 8.8px;", "font-size", 2.0).as_deref(),
            Some("font-size: 17.600px;")
        );
    }

    #[test]
    fn scales_a_unitless_length() {
        assert_eq!(
            scale_length("stroke-width: 0.8", "stroke-width", 3.0).as_deref(),
            Some("stroke-width: 2.400")
        );
    }

    #[test]
    fn keeps_other_declarations_and_order_untouched() {
        let style = "fill: none ; stroke: #b0b0b0; stroke-width: 0.8; stroke-linecap: square";
        assert_eq!(
            scale_length(style, "stroke-width", 2.0).as_deref(),
            Some("fill: none ; stroke: #b0b0b0; stroke-width: 1.600; stroke-linecap: square")
        );
    }

    #[test]
    fn scales_every_token_of_a_dasharray() {
        assert_eq!(
            scale_length("stroke-dasharray: 4, 2 1", "stroke-dasharray", 2.0).as_deref(),
            Some("stroke-dasharray: 8, 4 2")
        );
    }

    #[test]
    fn leaves_unparseable_values_alone() {
        assert_eq!(
            scale_length("stroke-dasharray: none", "stroke-dasharray", 2.0),
            None
        );
        assert_eq!(scale_length("fill: #ffffff", "fill", 4.0), None);
        assert_eq!(scale_length("font-size: 1.5em", "stroke-width", 2.0), None);
    }

    #[test]
    fn only_the_named_property_is_rewritten() {
        // `width` must not be confused with `stroke-width`.
        let style = "width: 5; stroke-width: 2";
        assert_eq!(
            scale_length(style, "stroke-width", 2.0).as_deref(),
            Some("width: 5; stroke-width: 4")
        );
    }

    #[test]
    fn a_missing_property_returns_none() {
        assert_eq!(scale_length("fill: none", "stroke-width", 2.0), None);
    }

    #[test]
    fn empty_style_is_inert() {
        assert_eq!(scale_length("", "font-size", 2.0), None);
    }
}
