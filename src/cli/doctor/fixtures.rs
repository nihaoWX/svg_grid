//! Corpus discovery plus the minimal `MANIFEST.toml` reader.
//!
//! The manifest is the registry the corpus is defined by
//! (`ARCHITECTURE/testing.md` §语料集): every `<id>.svg` has one `[[fixture]]` block
//! carrying `id`, `expect` and — for `expect = "unsupported"` — `why`.
//!
//! `doctor` only needs those three keys, so it reads them with a small line
//! scanner instead of pulling a TOML parser into the binary. The scanner handles
//! what the manifest actually uses: `[[fixture]]` tables, `key = "value"` /
//! `key = 'value'` scalars with `\"` escapes, multi-line arrays (skipped, they are
//! evidence, not verdicts) and comment lines (which double as the fallback `why`,
//! because the corpus documents its known defects in comments above the id).

use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::CliError;

/// One registered fixture.
#[derive(Debug, Clone)]
pub(crate) struct Fixture {
    /// `<package>/<basename>`, as the manifest and the report spell it.
    pub id: String,
    /// `tests/fixtures/<id>.svg`.
    pub path: PathBuf,
    /// `pass` (default) or `unsupported`.
    pub expect: String,
    /// Why an `unsupported` fixture is skipped (the manifest's `why`, or the
    /// comments of its block when `why` is not given).
    pub why: Option<String>,
}

impl Fixture {
    /// Whether the manifest declares this fixture as *unsupported* (skip, do not
    /// fail). Anything that is not `pass` skips: an unknown value is a manifest
    /// bug, and failing every run over it would hide the corpus.
    pub(crate) fn unsupported(&self) -> bool {
        self.expect != "pass"
    }

    /// The reason printed for a skipped fixture. Never empty: skipping silently is
    /// exactly what `ARCHITECTURE/testing.md` forbids.
    pub(crate) fn skip_reason(&self) -> String {
        match self.why.as_deref().map(str::trim) {
            Some(why) if !why.is_empty() => why.to_string(),
            _ => format!(
                "MANIFEST.toml registers this fixture with expect = {:?} but gives no `why`",
                self.expect
            ),
        }
    }
}

/// One `[[fixture]]` block, as the scanner sees it.
#[derive(Debug, Default, Clone)]
struct Entry {
    id: Option<String>,
    expect: Option<String>,
    why: Option<String>,
    comments: Vec<String>,
}

/// Load every fixture under `dir`, in manifest order, then any unregistered
/// `*.svg` sorted by path (a corpus file that is not in the manifest is still
/// judged — it must not become invisible).
pub(crate) fn load(dir: &Path) -> Result<Vec<Fixture>, CliError> {
    let manifest_path = dir.join("MANIFEST.toml");
    let entries = match fs::read_to_string(&manifest_path) {
        Ok(text) => parse_manifest(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "doctor: note: no MANIFEST.toml in {} — every *.svg is expected to pass",
                dir.display()
            );
            Vec::new()
        }
        Err(error) => {
            return Err(CliError::from(svg_grid::SvgGridError::at(
                &manifest_path,
                std::io::Error::new(
                    error.kind(),
                    format!("cannot read the corpus manifest: {error}"),
                ),
            )))
        }
    };

    let mut fixtures = Vec::new();
    for entry in entries {
        let Some(id) = entry.id else { continue };
        let why = entry.why.filter(|why| !why.trim().is_empty()).or_else(|| {
            let comments = entry.comments.join(" ");
            (!comments.trim().is_empty()).then_some(comments)
        });
        fixtures.push(Fixture {
            path: dir.join(format!("{id}.svg")),
            id,
            expect: entry.expect.unwrap_or_else(|| "pass".to_string()),
            why,
        });
    }

    // Files the manifest does not list: run them last, as `pass`.
    let mut extra: Vec<PathBuf> = Vec::new();
    collect_svgs(dir, &mut extra)?;
    extra.sort();
    for path in extra {
        let id = path
            .strip_prefix(dir)
            .unwrap_or(&path)
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        if fixtures.iter().any(|fixture| fixture.id == id) {
            continue;
        }
        fixtures.push(Fixture {
            path,
            id,
            expect: "pass".to_string(),
            why: None,
        });
    }

    Ok(fixtures)
}

/// Every `*.svg` under `dir`, recursively (the corpus is `package/file.svg`).
fn collect_svgs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let entries = fs::read_dir(dir).map_err(|error| svg_grid::SvgGridError::at(dir, error))?;
    for entry in entries {
        let entry = entry.map_err(svg_grid::SvgGridError::from)?;
        let path = entry.path();
        if path.is_dir() {
            collect_svgs(&path, found)?;
        } else if path.extension().is_some_and(|ext| ext == "svg") {
            found.push(path);
        }
    }
    Ok(())
}

/// The `[[fixture]]` blocks of a manifest.
fn parse_manifest(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut current: Option<Entry> = None;
    let mut in_array = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_array = false;
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            if trimmed.starts_with("[[fixture]]") {
                current = Some(Entry::default());
            }
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue; // the file's preamble
        };
        if trimmed.is_empty() {
            in_array = false;
            continue;
        }
        if trimmed.starts_with('#') {
            entry
                .comments
                .push(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }
        if in_array {
            // Inside a multi-line array (`covers = [...]`) until the closing `]`.
            in_array = !trimmed.contains(']');
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.starts_with('[') {
            in_array = !value.contains(']');
            continue;
        }
        let Some(value) = string_value(value) else {
            continue;
        };
        match key {
            "id" => entry.id = Some(value),
            "expect" => entry.expect = Some(value),
            "why" => entry.why = Some(value),
            _ => {}
        }
    }
    if let Some(entry) = current {
        entries.push(entry);
    }
    entries
}

/// A TOML basic/literal string value (`"…"` or `'…'`), with `\"`/`\\` unescaped.
fn string_value(value: &str) -> Option<String> {
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &value[quote.len_utf8()..];
    let mut out = String::new();
    let mut characters = rest.chars();
    while let Some(character) = characters.next() {
        if character == quote {
            return Some(out);
        }
        if character == '\\' && quote == '"' {
            match characters.next() {
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return None,
            }
            continue;
        }
        out.push(character);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{parse_manifest, string_value};

    const MANIFEST: &str = "\
# Corpus.
[[fixture]]
id = \"ggplot2/annotate\"
package = \"ggplot2\"
covers = [\"<path>\", \"<rect>\"]
expect = \"pass\"

[[fixture]]
# KNOWN DEFECT: the paint server is dropped.
id = 'matplotlib/bars_grouped'
covers = [
  \"<pattern>\",
  \"<use>\",
]
expect = \"pass\"

[[fixture]]
id = \"matplotlib/mathtext\"
expect = \"unsupported\"
why = \"<text> under a rotated ancestor <g transform=\\\"translate(...) rotate(-90)\\\">\"
";

    #[test]
    fn every_block_keeps_its_keys() {
        let entries = parse_manifest(MANIFEST);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id.as_deref(), Some("ggplot2/annotate"));
        assert_eq!(entries[0].expect.as_deref(), Some("pass"));
        assert_eq!(entries[0].why, None);
        // Single-quoted id, a comment inside the block, a multi-line array.
        assert_eq!(entries[1].id.as_deref(), Some("matplotlib/bars_grouped"));
        assert_eq!(entries[1].expect.as_deref(), Some("pass"));
        assert_eq!(
            entries[1].comments,
            vec!["KNOWN DEFECT: the paint server is dropped.".to_string()],
            "the comments of a block are its fallback `why`"
        );
        assert_eq!(entries[2].expect.as_deref(), Some("unsupported"));
        assert!(entries[2]
            .why
            .as_deref()
            .unwrap()
            .contains("<g transform=\"translate(...) rotate(-90)\">"));
    }

    #[test]
    fn array_lines_do_not_leak_into_the_next_block() {
        let entries = parse_manifest(MANIFEST);
        // The multi-line `covers` array ends the second block; the third block's
        // `expect` must still belong to the third block.
        assert_eq!(entries[1].id.as_deref(), Some("matplotlib/bars_grouped"));
        assert_eq!(entries[2].id.as_deref(), Some("matplotlib/mathtext"));
    }

    #[test]
    fn string_values_handle_both_quote_styles() {
        assert_eq!(string_value("\"a\""), Some("a".to_string()));
        assert_eq!(string_value("'a'"), Some("a".to_string()));
        assert_eq!(string_value("\"a\\\"b\""), Some("a\"b".to_string()));
        assert_eq!(string_value("42"), None);
        assert_eq!(string_value("\"unterminated"), None);
    }
}
