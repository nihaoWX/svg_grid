//! Errors and findings.
//!
//! `ConvertError` is the crate-level error; `Finding` is a single incompatible
//! node reported *before* anything is written.

use std::fmt;

#[derive(Debug)]
pub enum ConvertError {
    Io(std::io::Error),
    Xml(String),
    /// The input (or its root/canvas) cannot be rewritten by the composer.
    Incompatible(Vec<Finding>),
    /// The input is a half-finished conversion product (a panel box without the
    /// matching `data-scale` grouping) and `--force` was not given.
    Incomplete(String),
    /// A matplotlib-dialect construct could not be translated (fail closed).
    Translate(String),
    /// The post-write self-check found a broken invariant (internal bug).
    SelfCheck(String),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::Io(error) => write!(f, "I/O error: {error}"),
            ConvertError::Xml(error) => write!(f, "XML error: {error}"),
            ConvertError::Incompatible(findings) => {
                writeln!(
                    f,
                    "input is not compatible with `svg_grid plot-grid`: found {} problem(s):",
                    findings.len()
                )?;
                for finding in findings {
                    writeln!(
                        f,
                        "  - <{}> #{} at {}: {}",
                        finding.tag, finding.ordinal, finding.path, finding.reason
                    )?;
                }
                write!(f, "no output file was written")
            }
            ConvertError::Incomplete(detail) => write!(f, "{detail}"),
            ConvertError::Translate(detail) => {
                write!(
                    f,
                    "cannot translate the input SVG to the tag protocol: {detail}"
                )
            }
            ConvertError::SelfCheck(detail) => {
                write!(f, "post-conversion self-check failed: {detail}")
            }
        }
    }
}

impl std::error::Error for ConvertError {}

impl ConvertError {
    pub(crate) fn xml<E: fmt::Display>(error: E) -> Self {
        ConvertError::Xml(error.to_string())
    }
}

impl From<std::io::Error> for ConvertError {
    fn from(error: std::io::Error) -> Self {
        ConvertError::Io(error)
    }
}

/// A single incompatible node, reported before anything is written.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Element name, e.g. `path`, `image`, `g`.
    pub tag: String,
    /// 1-based position among all elements in document order.
    pub ordinal: usize,
    /// Slash-joined breadcrumb from the root, e.g. `svg/g/g/path`.
    pub path: String,
    /// Why the composer cannot handle it.
    pub reason: String,
}

/// A document-level (root / canvas) problem, reported on the `svg` element.
pub(crate) fn svg_finding(reason: String) -> Finding {
    Finding {
        tag: "svg".to_string(),
        ordinal: 1,
        path: "svg".to_string(),
        reason,
    }
}
