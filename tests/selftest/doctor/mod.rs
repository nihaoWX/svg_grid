//! The other two CLI entry points, end to end: `svg_grid doctor`
//! (`ARCHITECTURE/testing.md`) and `svg_grid convert` (`ARCHITECTURE/cli.md`).
//!
//! These run the **real** pipeline over a tiny 3-fixture corpus built in a
//! temporary directory — never the 58-fixture repository corpus (that is
//! `round-data/R18/test.py`'s job, and it takes long enough to deserve its own
//! evidence). Nothing here reads or writes the working directory, nothing touches
//! the network, and the font library is whatever the caller's
//! `$SVG_GRID_FONT_DIR` / data root already holds (a missing one is a SKIP with a
//! printed reason, not a silent pass).
//!
//! The converter is part of the binary under test (a library dependency), so there
//! is no second executable to look up and no SKIP for a missing one.

mod convert_cmd;
mod corpus;
