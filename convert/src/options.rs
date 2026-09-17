//! Options and result types for one conversion run.

use crate::error::Finding;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Only run the compatibility check and print a report; write nothing.
    pub check: bool,
    /// Print the (post-conversion) report to stdout.
    pub report: bool,
    /// Re-run conversion even if the input already looks converted.
    pub force: bool,
}

#[derive(Debug)]
pub enum Conversion {
    /// Input already is a *complete* conversion product → nothing to do.
    AlreadyConverted { canvas: String },
    /// Input contains elements the composer cannot rewrite.
    Incompatible(Vec<Finding>),
    /// Successfully converted.
    Converted(Box<Converted>),
}

#[derive(Debug)]
pub struct Converted {
    pub svg: String,
    pub report: Report,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub canvas: String,
    pub panel_box: String,
    pub texts: usize,
    pub circles: usize,
    pub primitives: Vec<(String, usize)>,
}
