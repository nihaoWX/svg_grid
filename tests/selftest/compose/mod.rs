//! Composition themes: the `plot-grid` pipeline from input panel to output file.
//!
//! * [`plot_grid`] — writing files, nesting, columns/widths, namespacing.
//! * [`normalize`] — `--normalize-input-max-side`: geometry and style lengths.
//! * [`path`] — `<path>` rewriting (one geometry written three ways, arcs).
//! * [`lenient`] — `<path>` / non-finite numbers are W7, not fail-closed.
//! * [`raster`] — PNG and AVIF previews of a composed SVG.

mod lenient;
mod normalize;
mod path;
mod plot_grid;
mod raster;
