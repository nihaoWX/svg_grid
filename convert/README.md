# svg_grid_convert

Convert an SVG panel exported by R (`svglite` / `ggplot2`) into the **tag
protocol** that `svg_grid plot-grid` consumes.

`svg_grid` is not a general SVG tool: it is a tagged grid composer. Each input
must carry a `<rect data-panel-box="main" .../>` panel box and `data-scale`
groups, otherwise the composer bails out or silently produces a mis-scaled
figure. R output has neither, so this thin adapter adds them — and refuses to
run (writing nothing) on anything the composer cannot rewrite.

## What it does

For every accepted input it:

1. computes the source canvas box exactly like the composer
   (`geom::find_canvas_box`: `viewBox -or- width/height`) and keeps the root
   element's `width`/`height`/`viewBox` untouched;
2. inserts `<rect data-panel-box="main" x y width height visibility="hidden"
   pointer-events="none"/>` as the **first child** of the root;
3. wraps everything else (including `<defs>` / `<clipPath>`) in a single
   `<g data-scale="xy">`, so the clipped geometry and the clipping rectangles
   scale together;
4. wraps every `<text>` and `<circle>` **in place** in
   `<g data-scale="position">`, so font size and point radius stay fixed;
5. resolves percentage geometry (svglite emits `<rect width='100%' .../>`) to
   absolute values, since the composer parses geometry with `parse_f32`.
   Percentage radii follow the SVG spec: a percentage `r` is resolved against the
   normalized diagonal `sqrt((width² + height²) / 2)`.

## Validation (fail closed)

**Validation always runs first — the idempotency check below never short-circuits
it.** Before writing anything the converter checks the root element and the canvas
box, then scans the whole tree:

* the root element must be `<svg>`;
* a `viewBox` attribute that is present must be **four whitespace-separated
  numbers** — a comma-separated `"0,0,288,216"` is a hard error instead of
  silently falling back to `width`/`height` (which would rescale the geometry);
* with no usable `viewBox`, `width`/`height` must be **plain numbers** (the
  composer parses them with `parse_f32`, so `288.00pt` is rejected with a
  "canvas size is not a pure number" message);
* the resulting canvas width and height must be **positive**.

It **does** translate (bake into the protocol) the following:

* `<g transform>` — a pure **translation** is baked into the leaf coordinates
  (including `<text>`/`<tspan>` `x`/`y` and `<clipPath>` rects); a `<g>` carrying
  scale or rotation is still rejected above a `<text>` (see below);
* `<use>` referencing a defined element / defined `<defs>` content — expanded at
  each use site, then removed;
* `<path d>` — converted to `<polyline>`/`<polygon>` (curves flattened within a
  tolerance). A **fill-only** path becomes **one** `<polygon>` covering all of its
  subpaths, each explicitly closed, so the outer contour *and* the holes stay a
  single fill region and the counters — `O`, `0`, `8`, `B` — keep their holes; the
  source `fill-rule` is preserved verbatim (it is **never** rewritten to
  `evenodd`). A **stroke-only** path keeps one `<polyline>` per subpath. A path
  that is both filled **and** stroked is decided by its subpath count: a single
  subpath keeps its shape as before (`<polygon>` when closed, else `<polyline>`),
  while several subpaths are **split** into one fill-only `<polygon>` (all
  subpaths concatenated and closed) plus one stroke-only shape per subpath — so
  the stroke never paints the bridges of the concatenated fill region;
* `<text>` transform — `rotate(angle, x, y)` is rewritten, svglite's
  `translate(x y) rotate(a)` is normalised to `x`/`y` + `rotate(a x y)`, and a
  bare `translate(tx[, ty])` is folded into the text's `x`/`y`;
* **`<tspan>` with absolute `x`/`y`** — accepted: the composer rewrites the
  `<text>`'s own `x`/`y` and carries each `<tspan>` coordinate along the same
  axis (each axis independently; `dx`/`dy` are increments and are left as-is).
  This is what makes matplotlib's **automatic mathtext** (`$\mathdefault{10^{n}}$`
  from log axes / `LogNorm` colour bars) convert — it lands as
  `<g transform="translate(X Y)"><text><tspan x= y=…>`;
* **`<clipPath>` body `<path>`** (the pdftocairo shape) — every axis-aligned
  rectangular subpath is rewritten to a `<rect>`, and every `<rect>` in a
  referenced clip is baked with the same transform as the geometry it clips. A
  non-axis-aligned clip subpath still fails closed;
* **`<image>`** — kept as a raster child: any axis-aligned transform — a pure
  translation, an axis-aligned uniform flip, or an **axis-aligned non-uniform
  scale** (independent x/y, e.g. a `matrix(...)`) — is baked into
  `x/y/width/height` (the raster is re-encoded), and `preserveAspectRatio="none"`
  is written so the bitmap fills its box exactly like a `<rect>` (the main
  library moves it by `x/y/width/height`). Rotation/shear still fails closed.
  **An `<image>`-only wrapper SVG is the supported way to bring a bitmap panel
  (e.g. a ChimeraX render) into a composed figure.**

Then it aborts (exit code 1, no output file) when it finds any of:

* nested `<svg>` (its own viewport/coordinate system would be misplaced);
* `<symbol>` (its own nested coordinate system), or a `<use>` whose `href` does
  not resolve to a defined element — a resolvable `<use>` is expanded, not
  rejected;
* a `<circle>` under a rotation, skew or non-uniform scale (it would become an
  ellipse), or a `<rect>` under a rotation or skew (it would no longer be
  axis-aligned);
* a `<text>` under a scale or rotation inherited from an ancestor `<g>` — only a
  pure translation can be baked into text, so the label would be misplaced;
* an `<image>` under a transform that is not axis-aligned (a rotation or shear);
* a `<path>` inside a `<clipPath>` whose subpath is not an axis-aligned rectangle
  (a clip can only be baked as a union of `<rect>`s);
* an unparseable `transform`, or a `<text>` `transform` that is none of
  `rotate(angle, x, y)`, svglite's `translate(x,y) rotate(angle)`, or a bare
  `translate(tx[, ty])` (see below);
* any geometry attribute the composer parses with `parse_f32` that is not a plain
  number.

It prints an actionable per-node report (tag, document-order index, breadcrumb
path, reason) for each offender.

Non-fatal warnings (stderr, exit code still 0): scale-relevant properties
written inside a CSS `style="..."` attribute (e.g. `stroke-width`, `font-size`)
and the presence of a `<style>` CSS block — the composer rewrites neither.

## Idempotency and half-finished products

The idempotency shortcut is only taken once the input has **passed full
validation** and is a **complete** conversion product — i.e. it carries *exactly
one* `data-panel-box="main"` rect **and** a `data-scale` grouping:

* **complete product** → exit 0. If `--output <path>` was given (and differs from
  the input) the input is **copied verbatim** to that path, so a batch pipeline
  `convert --input a.svg --output b.svg` always leaves `b.svg` on disk for the
  next stage. `--force` re-does the conversion from scratch (byte-stable).
* **half-finished product** — a panel box but **no** `data-scale` grouping — is a
  hard error (exit 1) telling you to use `--force`, so a broken intermediate can
  never be silently accepted.
* `--check` always prints an explicit verdict on **stdout**, including for
  already-converted input.

## 缩放后果与配准建议

`data-scale="position"` only translates — it **never scales**. Therefore, inside
a panel, `font-size`, point radius and `stroke-width` are *not* touched by the
composer. If you assemble a source panel into a grid cell whose size differs from
the source canvas, the geometry scales but the text/point marks do not, so their
relative proportions change.

Quantitative example (real `fig3_p1` panel): a title with `font-size: 9.84px` has
an ink height of ~7 px. Grouped into a cell of roughly the source size
(scale ≈ 1), that is `7 / 648 ≈ 0.0108` of the panel width. Grouped into a cell
twice as wide (scale ≈ 2) the mark is still 7 px while the panel is 1296 px wide,
i.e. `7 / 1296 ≈ 0.0054` — **the panel doubled and the text halved in relative
size**.

Practical rules:

* assemble panels into cells **close to the original canvas size** (scale ≈ 1);
* to make a figure larger, re-export it from R at the desired size (and font
  size) instead of upscaling the cell;
* `--check` / `--report` print the source canvas size plus this hint, so you can
  pick a matching downstream cell size.

## Build (offline)

Only depends on `quick-xml 0.39`, which is already in the shared cargo registry
cache, so no network is needed:

```bash
cd tools/svg_grid/convert
cargo build --offline --release     # produces target/release/svg_grid_convert
cargo test --offline                # 67 unit + 11 CLI tests, hand-made fixtures only
cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings
```

## CLI

```text
svg_grid_convert --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]
```

| flag | meaning |
|---|---|
| `--input <path>` | source SVG produced by R (required) |
| `--output <path>` | destination tagged SVG (required unless `--check`) |
| `--check` | only run the compatibility check and print a report; write nothing |
| `--report` | print the per-node / conversion report to stdout |
| `--force` | re-run conversion even if the input already looks converted (also completes a half-finished product) |

Exit codes: `0` success/compatible, `1` incompatible input or error. Errors and
warnings go to stderr; reports go to stdout.

```bash
# dry run: is this panel composable at all?
svg_grid_convert --input fig3_p1.svg --check --report

# convert
svg_grid_convert --input fig3_p1.svg --output fig3_p1.tagged.svg --report
```

## Two-step pipeline (convert → plot-grid)

`geom::scale_panel_to_cell` scales the two axes **independently**
(`sx = cell.width / canvas.width`, `sy = cell.height / canvas.height`), so the
grid cell's aspect ratio **must equal the source canvas's**. For a single panel
that means choosing `--width/--height/--plot-margin` (and `--gap`, for multi-panel
grids) such that the cell matches the canvas ratio:

```text
available_w = W − margin.left  − margin.right  − gap·(ncol − 1)
available_h = H − margin.top   − margin.bottom − gap·(nrow − 1)

cell_w = available_w · relw_i / Σ relw
cell_h = available_h · relh_j / Σ relh

undistorted  ⇔  cell_w / cell_h == canvas.width / canvas.height
```

(The default `margin = 24`, `gap = 0`; `relw`/`relh` default to `1` for every
cell.)

For the real `648 x 280.80` panel (ratio `2.3077`) assembled at scale ≈ 1, use a
cell of `648 x 280.80`, i.e. margin `24` → `--width 696 --height 328.8`. (A `2×`
cell such as `1296 x 561.6` keeps the aspect ratio and the geometry, but it halves
the *relative* size of the text — see “缩放后果与配准建议” above.)

```bash
SVG=tools/svg_grid/convert/target/release/svg_grid_convert
GRID=tools/svg_grid/target/release/svg_grid

"$SVG"  --input  fig3_p1.svg  --output fig3_p1.tagged.svg

"$GRID" plot-grid \
  --input fig3_p1.tagged.svg \
  --output fig3.svg \
  --output-png fig3.png \
  --width 696 --height 328.8 --plot-margin 24
```

The composed SVG keeps text as `<text>` (the composer never rasterises text) and the panel
geometry is scaled to the cell while `font-size`, point radius and `stroke-width`
(presentation attributes) stay fixed. **Raster panels pass through inline as base64 `<image>`**
(a bitmap-only wrapper, or matplotlib's `imshow`/colour bar) — expected in a mixed figure.

## Rerun / reproduce

Both crates build offline. To reproduce the checks from a clean tree:

```bash
cd tools/svg_grid/convert && cargo build --offline --release && cargo test --offline
cd ../../.. && git status --short tools/svg_grid   # only convert/ + this README appear
```

The `convert/target/` directory is gitignored (see the repository root
`.gitignore`).
