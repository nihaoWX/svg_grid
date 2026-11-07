//! The `--help` text (`ARCHITECTURE/cli.md` §`--help` 契约).
//!
//! It lives in its own module so `main.rs` stays a one-line entry point, and so the
//! parser's unit tests can assert that **every** flag the parser knows appears here.

/// Everything `--help` prints, on **stdout**, with exit code 0.
pub(crate) const HELP: &str = "\
svg_grid — compose SVG panels into one grid figure.

Each panel must carry the tag protocol: a `<rect data-panel-box=\"main\" .../>` outlining
the panel and `data-scale` groups marking how geometry follows the cell (panel geometry
scales, text size / marker size / stroke width do not).

usage:
  svg_grid plot-grid --input A.svg [--input B.svg ...] --output OUT.svg [options]
  svg_grid plot-grid --input A.svg [--input B.svg ...] --output-png OUT.png [options]
  svg_grid plot-grid --input A.svg [--input B.svg ...] --plan [options]
  svg_grid plot-grid --list-fonts [--font-dir <dir>]
  svg_grid convert --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]
  svg_grid doctor [--fixtures <dir>] [--only <glob>] [--jobs <n>] [--json] [--keep <dir>]
  svg_grid --help | -h
  svg_grid --version

inputs and outputs:
  --input <path>          one panel SVG; repeat once per panel. `--input none` /
                          `--input null` is a blank cell (cowplot's NULL).
  --spacer                a blank grid cell; same as `--input none`.
  --output <path>         the composed SVG (kept on disk).
  --output-svgz <path>    also write a gzipped .svgz.
  --output-png <path>     also rasterise to PNG (needs the font library).
  --output-avif <path>    also rasterise to AVIF (needs the font library).
  --avif-quality <1..100> AVIF quality; default 70.
  --png-dpi <dpi>         physical resolution written into the PNG (pHYs chunk); default 600.
  (with no --output)      a temporary SVG is derived from the raster path and removed.

canvas and grid:
  --width <pt>            canvas width; default 1200.
  --height <pt>|auto      canvas height; omitted or `auto` ⇒ derived from the panels'
                          natural heights, and that value IS the output canvas height.
  --nrow <n>              number of rows (≥1); the grid never has an empty row/column.
  --ncol <n>              number of columns (≥1). Both omitted ⇒ one row.
  --rel-widths <list>     per-column multipliers on each column's natural width
                          (default all 1; values may be 0 to collapse a column).
  --rel-heights <list>    per-row multipliers on each row's natural height
                          (default all 1). A length that does not match the grid is an error.
  --gap <pt>              gap between cells; default 0.
  --plot-margin <all>|<top,right,bottom,left>
                          outer margins; default 24.
  --normalize-input-max-side <pt>
                          rescale every input so its longer side is at most this.
  --plan                  print the layout and exit 0: the canvas size, then one line
                          per input, in input order —
                          `#<i> <path>  canvas <w>x<h>  cell <w>x<h>  s=<sx>x<sy>  Δ<percent>`
                          (Δ = (sx/sy − 1): 0 % means no distortion) — then one
                          `note:` line per input saying whether it has to be re-emitted
                          at its cell size first, with the exact command when it does.
                          Writes no file and rasterises nothing, so it cannot be
                          combined with any output flag (that is a usage error, exit 2).

panel letters:
  --labels [A,B,...]      `--labels` alone = AUTO (A, B, C, …), `--labels A,B` = explicit
                          letters, `--labels none` = no letters (the default).
  --label-size <auto>|<pt>
                          letter size; `auto` (default) ≈ 2.7% of the canvas width.
  --label-x <list>        horizontal anchor inside the cell; default 0.
  --label-y <list>        vertical anchor inside the cell; default 1.
  --hjust <list>          horizontal justification; default 0.
  --vjust <list>          vertical justification; default -0.25.
  --label-fontfamily <list>
                          CSS family list, default Arial; a fallback chain is allowed,
                          e.g. --label-fontfamily '\"Arial\", \"Liberation Sans\", sans-serif'.
  --label-fontface <face> font face; default bold.
  --label-colour <colour> letter colour; default #000000.

fonts (panel letters and every PNG/AVIF):
  --font-dir <dir>        font directory. Resolution order: this flag, then
                          $SVG_GRID_FONT_DIR, then the data root <home>/svg_grid/fontlib
                          (HOME; %USERPROFILE% on Windows). Only that directory is read —
                          the host's fonts are never used.
  --list-fonts            print the families the font library holds (sorted,
                          deduplicated) and exit; no figure is composed.
  A family the library does not have is non-fatal: stderr names it and says how to add
  it. A missing or empty library is fatal whenever the run needs glyphs (PNG/AVIF output
  or --labels) — that is a configuration error, not a rendering quirk.

convert (graphics dialect -> tag protocol; the same code the standalone
`svg_grid_convert` binary runs, now built into this one):
  --input <path>          the SVG a plotting tool produced (svglite/ggplot2/
                          matplotlib); required. `-i` is the short form.
  --output <path>         where to write the tagged SVG; required unless --check.
                          `-o` is the short form.
  --check                 only run the compatibility check and print a report;
                          write nothing.
  --report                print the per-node / conversion report to stdout.
  --force                 re-run conversion even if the input already looks
                          converted (also completes a half-finished product: a
                          panel box without the matching data-scale grouping).
                          `-f` is the short form.
  An input that already is a complete conversion product exits 0; with --output
  it is copied verbatim, so `convert --input a.svg --output b.svg` always
  produces b.svg. Its diagnostics are prefixed `svg_grid convert:` (the standalone
  `svg_grid_convert` binary prefixes its own name) and its exit codes are
  unchanged: 0 = success/compatible, 1 = incompatible input or error.

doctor (self-check: run the whole fixture corpus end to end):
  --fixtures <dir>        corpus to run. Resolution order: this flag, then
                          $SVG_GRID_FIXTURES, then the data root <home>/svg_grid/fixtures
                          (install.sh fills it; on Windows copy tests\\fixtures there, see
                          docs/install-windows.md), then the
                          repository's <exe dir>/../../tests/fixtures. An explicit
                          flag or environment value is used as given; of the last two
                          the first that exists wins.
                          Each fixture is composed at its own natural size, through
                          the real two stages: the converter (called in process, no
                          second binary is looked up), then plot-grid
                          (--png-dpi 300, canvas = panel + the default --plot-margin).
  --only <glob>           run only the fixture ids matching this glob: `*` is any
                          run of characters, `?` is one, matching is anchored at
                          both ends; e.g. --only 'ggplot2/*' or --only '*heatmap'.
  --jobs <n>              run up to n fixtures at once; default 1 (the report stays
                          in fixture order either way).
  --json                  print the machine-readable report on stdout instead of the
                          one-line-per-fixture one.
  --keep <dir>            keep the intermediate tagged/composed files in <dir>.
  Each fixture prints one line: PASS|FAIL|SKIP  <id>  <step>  <ms>  warnings=N,
  then `summary: N pass, M fail, K skip`. A FAIL (non-zero stage, empty output, a
  PNG with no ink, or W7/W9 on stderr) lists its criteria and stderr lines and
  makes doctor exit 1. Fixtures the manifest marks unsupported are SKIPped and
  their `why` is printed. `doctor --help` prints this text.

exit codes:
  0  success (non-fatal warnings may be printed to stderr; they never change the code)
  1  runtime failure: I/O, XML parsing, rasterisation; `doctor` found a FAIL; or
     `convert` could not convert its input
  2  invalid usage or input: unknown flag, bad flag value, malformed input SVG
  3  configuration error: the font library is missing or empty, or `doctor` cannot
     find the fixture corpus where it looks for it

  -h, --help              print this text on stdout and exit 0.
  --version               print the version and exit 0.
";
