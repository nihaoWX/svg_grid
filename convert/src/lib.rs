//! `svg_grid_convert` — turn an R (`svglite` / `ggplot2`) SVG panel into the
//! "tag protocol" that `svg_grid plot-grid` consumes.
//!
//! The composer (`tools/svg_grid/src/transform.rs`) rewrites coordinates
//! primitive-by-primitive; it never offsets an outer `transform`. So an
//! external SVG has to be pre-tagged with:
//!
//! * a panel box: `<rect data-panel-box="main" x y width height/>` (anywhere in
//!   the tree), otherwise `geom::find_panel_box` makes the composer bail out;
//! * `data-scale` groups: the composer rewrites each primitive using the
//!   *nearest* ancestor policy (or the primitive's own), defaulting to `none`
//!   (translate only, no scaling).
//!
//! This crate guarantees three properties:
//!
//! 1. **fail closed, validation first** — the root element, the canvas box and
//!    the whole tree are validated *before* anything else happens. If the
//!    document cannot be safely rewritten (nested `<svg>`, `<symbol>`, a `<use>`
//!    that does not resolve, a transform the
//!    translator cannot bake, an `<image>` with rotation/skew,
//!    a `viewBox` that is not four whitespace-separated numbers, or a
//!    non-positive / non-numeric canvas), the conversion aborts with a per-node
//!    report and writes nothing. Validation is never short-circuited by the
//!    idempotency check.
//!
//!    Between the scan and the tag protocol runs a *translation* stage
//!    (`translate`) that rewrites the matplotlib dialect into the composer's
//!    vocabulary: `<g transform>`/affine transforms are baked into leaf
//!    coordinates, `<path>` becomes `<polyline>`/`<polygon>`, `<use>` is expanded
//!    (and unreferenced `<defs>` content dropped), and `<image>` transforms are
//!    baked into the bitmap.
//! 2. **geometry that scales, marks that do not** — panel/axis geometry (and the
//!    `<clipPath>` rectangles) get `data-scale="xy"`; every `<text>` and
//!    `<circle>` is wrapped *in place* in `data-scale="position"` so font size
//!    and point radius stay fixed.
//! 3. **idempotent, but only for a *complete* product** — an input that already
//!    carries exactly one `data-panel-box="main"` rect **and** a `data-scale`
//!    group and no incompatible node is reported as already converted (exit 0);
//!    if `--output` was given it is copied verbatim so a batch pipeline always
//!    produces the file the next stage expects. An input with a panel box but
//!    *no* `data-scale` group is a half-finished product: it is a hard error
//!    (exit 1) unless `--force` is given.
//!
//! Target format details live in `ARCHITECTURE.md` and
//! `docs/single_cell/svg_grid_agent_usage.md`.

mod affine;
mod base64;
pub mod cli;
mod dom;
mod error;
mod geom;
mod options;
mod path;
mod pipeline;
mod scan;
mod self_check;
mod stroke;
mod tag;
mod transform;
mod translate;
mod warnings;

#[cfg(test)]
mod test_support;

pub use cli::run;
pub use dom::{Element, Node};
pub use error::{ConvertError, Finding};
pub use options::{Conversion, Converted, Options, Report};
pub use pipeline::convert;
pub use self_check::self_check;
