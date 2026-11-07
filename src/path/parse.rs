//! Tokenising a `<path d>` into `(command, arguments)` pairs.
//!
//! The command table ([`arity`]) and SVG number syntax live here; [`rewrite`]
//! and [`flatten`] consume the segments without re-parsing.
//!
//! [`rewrite`]: super::rewrite
//! [`flatten`]: super::flatten

/// An absolute source-space point for an argument pair: relative (`m`/`l`/…)
/// deltas are added to the current point, absolute ones stand alone.
pub(super) fn resolve(x: f32, y: f32, relative: bool, cur: (f32, f32)) -> (f32, f32) {
    if relative {
        (cur.0 + x, cur.1 + y)
    } else {
        (x, y)
    }
}

/// One `d` argument: the parsed value plus the exact source text, so the `A`
/// flags and rotation can be re-emitted untouched.
pub(super) struct Arg {
    pub(super) value: f32,
    pub(super) raw: String,
}

pub(super) struct Segment {
    pub(super) command: u8,
    pub(super) args: Vec<Arg>,
}

/// Tokenise `d` into `(command, arguments)` pairs, expanding implicit repeats.
pub(super) fn tokenize(d: &str) -> Result<Vec<Segment>, String> {
    let bytes = d.as_bytes();
    let mut index = 0usize;
    let mut segments = Vec::new();
    let mut last_command: Option<u8> = None;

    loop {
        skip_separators(bytes, &mut index);
        if index >= bytes.len() {
            break;
        }
        let command = if bytes[index].is_ascii_alphabetic() {
            let command = bytes[index];
            index += 1;
            command
        } else {
            match last_command {
                // Implicit repeat: after a moveto, extra pairs are linetos.
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(command) => command,
                None => return Err("path data must start with a command".to_string()),
            }
        };

        let arity = arity(command)?;
        let mut args = Vec::with_capacity(arity);
        for _ in 0..arity {
            skip_separators(bytes, &mut index);
            let start = index;
            let value = read_number(bytes, &mut index).ok_or_else(|| {
                format!("missing argument for path command {:?}", command as char)
            })?;
            let raw = std::str::from_utf8(&bytes[start..index])
                .map_err(|_| "path number is not valid UTF-8".to_string())?
                .to_string();
            args.push(Arg { value, raw });
        }
        last_command = Some(command);
        segments.push(Segment { command, args });
    }

    Ok(segments)
}

fn arity(command: u8) -> Result<usize, String> {
    match command {
        b'Z' | b'z' => Ok(0),
        b'H' | b'h' | b'V' | b'v' => Ok(1),
        b'M' | b'm' | b'L' | b'l' | b'T' | b't' => Ok(2),
        b'S' | b's' | b'Q' | b'q' => Ok(4),
        b'C' | b'c' => Ok(6),
        b'A' | b'a' => Ok(7),
        other => Err(format!("unsupported path command {:?}", other as char)),
    }
}

fn skip_separators(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && (bytes[*index].is_ascii_whitespace() || bytes[*index] == b',') {
        *index += 1;
    }
}

fn read_number(bytes: &[u8], index: &mut usize) -> Option<f32> {
    let start = *index;
    if *index < bytes.len() && (bytes[*index] == b'+' || bytes[*index] == b'-') {
        *index += 1;
    }
    let mut has_digit = false;
    while *index < bytes.len() && (bytes[*index].is_ascii_digit() || bytes[*index] == b'.') {
        has_digit = true;
        *index += 1;
    }
    if *index < bytes.len() && (bytes[*index] == b'e' || bytes[*index] == b'E') {
        *index += 1;
        if *index < bytes.len() && (bytes[*index] == b'+' || bytes[*index] == b'-') {
            *index += 1;
        }
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
    }
    if !has_digit {
        *index = start;
        return None;
    }
    std::str::from_utf8(&bytes[start..*index])
        .ok()?
        .parse::<f32>()
        .ok()
}
