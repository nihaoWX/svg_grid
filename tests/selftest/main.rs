//! The single self-check test target (`ARCHITECTURE/testing.md` §两层自检).
//!
//! Self-checking is **one** thing — "does this tool still work?" — and this is its
//! source-level entry: `cargo test` is the only test target, `svg_grid doctor` is
//! the same check at run time over the real corpus. There is deliberately no
//! `compose_tests` / `doctor_tests` split: directory layout here is by **theme**,
//! never by "which command it tests" (user ruling, 2026-10-06 R19).
//!
//! Every theme is a `mod` declaration — never `include!`, because rustfmt follows
//! `mod` and does not follow `include!` (round-data/R8: that made fmt report a
//! clean tree while a file was permanently unformatted). The price is that libtest
//! prefixes each test name with its module path (`compose::plot_grid::…`).
//!
//! Shared helpers live in [`common`]: `common` itself holds the hand-written
//! protocol panels and the `plot-grid` CLI wrappers, `common::harness` holds the
//! subprocess harness (`svg_grid doctor` / `svg_grid convert` runs).

mod common;
mod compose;
mod doctor;
mod fonts;
mod labels;
mod layout;
mod refs;
mod transform;
mod warnings;
