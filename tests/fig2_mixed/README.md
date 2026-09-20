# `fig2_mixed` — Figure 2 assembled from a **vector + bitmap** panel mix

Test asset for `tools/svg_grid`: does `plot-grid` compose the project's main
**Figure 2** when one band is a re-emitted **vector** panel and two bands are
**bitmap** panels?  The supplementary figures (S2–S4) are out of scope.

Everything here is built from the project's own panels; the research tree
(`research/白藜芦醇胰腺癌/…`) is **read only**.

## Panel inventory (the existing `Figure 2.png`, from `make_Figure2_v6.py`)

`Figure 2.png` (4961 × 7016 @600 dpi, written 2026-09-12 19:13) is produced by
`steps/02_分子对接/figure_scripts/make_Figure2_v6.py`. **Every band is raster in
the existing product — there is no vector panel in it yet.** Three full-width
bands:

| band | letter | content | source (script line) | native px | placed px | kind |
|---|---|---|---|---|---|---|
| A | `A` | 9×4 docking-score heatmap | `render_heatmap()` `make_Figure2_v6.py:158-186`, matplotlib `imshow` → PNG | 4661 × 809 | 4661 × 809 | **raster** (can be vector) |
| B | `B` | 4 ligand 2D structures, 1×4 | `render_ligands()` `:189-209`, RDKit `MolDraw2DCairo(1100,700)` → PNG | 1100 × 700 each | contain-fit 1084 × 560 each | **raster** |
| C | `C` | 9 binding-pocket close-ups, 3×3 | `closeup_all.cxc` → ChimeraX PNG (`figure_panels/closeup_*.png`) | 2400 × 2400 each | contain-fit 1541 × 1541 each | **raster** (renderer output — no vector source) |

Evidence for the sizes: `make_Figure2_v6.py:96-115` (band geometry:
`C_GAP=19`, `C_CELL=1541`, `C_HDR=56`, `B_GAP=39`, `B_CELL=1136`, `B_PAD_T=16`,
`B_IMG_H=560`, `H_A=809`, `H_B=760`, `H_C=4829`) and the real files under
`steps/02_分子对接/figure_panels/` (`closeup_*.png` = 2400 × 2400,
`diagram_*.png` = 1800 × 1200, `overview_*.png` = 2400 × 2400).

## What this test builds

```
band A   matplotlib SVG, svg.fonttype='none', pcolormesh body   -> VECTOR   (1 panel)
band B   4 × bitmap wrapper SVG (RDKit PNG in <image>)          -> BITMAP   (4 panels)
band C   9 × bitmap wrapper SVG (ChimeraX PNG in <image>)       -> BITMAP   (9 panels)
```

* **vector band** — `build_panel_a()` in `make_panels.py`: the heatmap is
  re-drawn at `figsize = (4661/72, 809/72)` inches so its `viewBox` is exactly
  the grid cell (4661 × 809 canvas units), with every point-valued size and
  rcParam multiplied by `k = 600/72`. `pcolormesh` (not `imshow`) keeps the
  heatmap body as vector polygons; the colour bar is still an embedded
  `<image>` (matplotlib always rasterises a colour bar).
* **bitmap bands** — one wrapper SVG per raster panel:
  `<image x y width height preserveAspectRatio="none" href="data:image/png;base64,…">`
  filling its box. `preserveAspectRatio="none"` is what makes the bitmap **fill**
  the box instead of being letterboxed by the SVG default `xMidYMid meet`.
  The wrapper canvas is the grid cell itself (so `s = cell/canvas = 1`) and the
  PNG it embeds is the same `contain()`-fitted raster the project script pastes,
  so the comparison against `Figure 2.png` is apples to apples.
  Panels keep the labels the project script puts in them: the ligand chip (B) and
  the target name above each close-up (C), both as vector `<text>`.

## Run

```bash
micromamba run -n python-bio python make_panels.py     # panels -> out/panels, rasters -> out/rasters
micromamba run -n python-bio python compose_fig2.py    # convert + compose + verify -> out/
```

Products (`out/`, gitignored): `Figure2.svg`, `Figure2.png` (600 dpi),
`Figure2_comparison.png` (original above, ours below, equal width),
plus `out/tagged/`, `out/blocks/`, `out/verify.log`.

## Measured result (2026-09-17)

Layout = the existing figure's own geometry, stacked as three nested
`plot-grid` calls (one per band, where the letters are added) + one
`--ncol 1 --plot-margin 0` stack. Canvas width **4961** (the existing product's
width); height is derived → **4961 × 7016**, i.e. A4 portrait
(height/width = 1.41423 vs A4 1.41421).

| check | result |
|---|---|
| `sx`/`sy` per cell (all 14 panels) | **1.00000** (from the `data-scale` element's `width`/`height` ratio and, where non-degenerate, from coordinates) |
| panel-box → cell placement | every panel box lands exactly on its analytic cell (`<0.05` px) |
| cell aspect vs canvas aspect | identical to 5 decimals everywhere (no distortion) |
| `plot-grid` stderr | **0 bytes on all four calls** (no W1–W5 warning) |
| `svg_grid_convert` stderr | only the known benign `<style>` block note on the matplotlib panel |
| dialect residue in `Figure2.svg` | `path=0 use=0 symbol=0 g_transform=0 image_transform=0` |
| `Figure2.png` | 4961 × 7016, pHYs 600 dpi, ink (non-white) **33.69 %** |
| longest all-white row band | **100 px** (< 200) |
| longest all-white column band | **149 px** (< 200) |
| dead (all-white) 200 px row bands | none |
| letter ink width / canvas width | `A` 99 px = **2.00 %**; `B` 82 px = 1.65 %; `C` 86 px = 1.73 % (glyph-dependent, see `01_canvas_layout.md`) |
| letter font weight | all three closer to DejaVu **Serif regular** fill than to bold |
| embedded rasters decoded | all 14 `<image>` boxes render > 100 unique colours (256 … 63 739) |

### bitmap magnification (native px → px inside the grid cell)

| panel | native px | box in the cell | factor |
|---|---|---|---|
| A colour bar (raster `<image>` inside the vector panel) | 136 × 915 | 93.6 × 658.8 | 0.688 × / 0.720 × |
| B ligand A / B / C / D | 1100 × 700 (ink ≈ 987 × 510 after the project's `contain()` trim) | 1084 × 560 | **1.098 ×** / **1.079 ×** / 0.868 × / 0.960 × |
| C close-up (×9) | 2400 × 2400 | 1541 × 1541 | 0.642 × … 0.787 × |

Only ligands A and B are magnified (**1.10 × / 1.08 ×** — a mild, sub-pixel-level
upsample). That is **inherited from the project script**, which applies the same
`contain()` rule; the existing `Figure 2.png` is equally soft there. Nothing in
this pipeline magnifies a bitmap.

### difference from the existing `Figure 2.png` (RMS / share of pixels differing by > 30)

| region | RMS | > 30 | note |
|---|---|---|---|
| band A heatmap body | 11.93 | 1.71 % | `pcolormesh` (flat cells) vs `imshow` (interpolated) — our choice to keep the body vector |
| band A colour bar | 13.88 | 3.34 % | same 136 × 915 bitmap, resampled by resvg instead of by Agg |
| band A axis / tick text | 18.12 | 2.70 % | matplotlib vs resvg glyph rasterisation, sub-pixel phase |
| band A `kcal/mol` label | 42.86 | 3.89 % | **present in ours, clipped in the original** (see below) |
| band B | 6.90 | 0.12 % | identical apart from letter size |
| band C | 8.98 | 0.17 % | identical apart from target-name text |

Expected differences: the vector heatmap body, the sharper/larger letters
(the original's letters are `A` 1.79 %, `B` 1.49 %, `C` 1.55 % — *below* the
guide's 1.8–2.2 % band; ours land at 2.00 / 1.65 / 1.73 %), and the now-visible
`kcal/mol` colour-bar label.

Unexpected difference: none found — every band's raster content reproduces the
original to RMS ≤ 9 except band A, which differs only where the rendering route
changed (documented above).

## The `<image>` verdict

`PROGRESS.md` used to carry the open item “*转换器对 `<image>` 的放行条件未重评
（主库已支持）*”. **It is already resolved in the current tree: `convert/` accepts
`<image>` and no relaxation was needed.** Evidence in this run:

* a pure `<image>`-only wrapper (`viewBox` = the PNG's own pixel size, e.g.
  `0 0 2400 2400`) converts with exit 0 and report `primitives: rect=1`;
* **14 / 14** panels — including the 13 bitmap wrappers — convert with exit 0;
* `convert/src/scan.rs:150-166` accepts an `<image>` whose accumulated transform
  is axis-aligned — any x/y scale (uniform **or non-uniform**), an optional flip,
  or a pure translation; only a rotation or shear fails closed — and
  `convert/src/translate/image.rs` bakes that transform into `x/y/width/height`
  and writes `preserveAspectRatio="none"`;
* the matplotlib panel with `imshow` (2 embedded rasters, non-trivial transforms)
  also converts with exit 0.

**No file under `tools/svg_grid/convert/` was changed by this test.** The only
mismatch found was documentation: `convert/README.md` then said the converter
“aborts … on `<image>` (raster / base64 content)”, which contradicted
`scan.rs` / `translate/image.rs`. That README has since been corrected (it now
documents `<image>` as a supported raster child), so this is no longer an open
mismatch.

## Things the manual does not say (learned by doing)

1. **`convert` accepts `<image>`** (see above) — the manual/`PROGRESS.md` implied
   the opposite; verified by running it, 14/14 panels.
2. **Inlining loses per-panel clipping.** The composer inlines each panel's
   children into a `<g data-svg-grid-cell=…>` inside the output SVG; there is no
   nested `<svg>` viewport, so content that overflows the panel canvas is *no
   longer clipped at the panel edge*. Panel A's `kcal/mol` colour-bar label
   (local x ≈ 4749 > the 4661-wide panel canvas) is clipped away in the existing
   PNG but renders here, ~130 px into the right margin. Harmless in this figure
   (still inside the canvas) but it can bleed into a neighbour in a tighter grid.
3. **A bitmap wrapper's canvas choice decides `s`.** Making the wrapper canvas
   equal to the grid cell gets `s = 1` with no `--normalize-input-max-side` and no
   W1 warning; using `viewBox` = the *native* PNG pixel size and a smaller cell
   raises W1 (`sx=sy=0.62208`, off by 37.8 %) — measured on the probe wrapper.
   Both compose correctly; only the second shifts text/point proportions.
4. **`preserveAspectRatio="none"` is load-bearing.** Without it the SVG default
   `xMidYMid meet` letterboxes the bitmap inside its box; the composer stretches
   the box, not the bitmap. `convert` writes it automatically for inputs *it*
   produces, but a hand-written wrapper must set it itself.
5. **`--label-size` is in canvas units, not points**, and `--label-fontfamily`
   must name an installed family (`"DejaVu Serif"`) — the resvg text backend
   resolves it; the default label family/weight (sans-serif / bold) is the
   opposite of the guide.
6. **The letter band has to be reserved by hand when you pin `--plot-margin`.**
   With an explicit margin the automatic letter band is off, and the only warning
   is “letter would be clipped by the canvas” — a letter that merely overflows
   *the band* (but not the canvas) is silently cropped by the band's own canvas
   edge. Sizing the band as `gap + cap-height(size)` avoids it.
7. **Glyph ink width is font- and letter-dependent.** `--label-size 134` gives
   `A` 2.00 % of 4961 but `B` 1.65 % / `C` 1.73 %; no single size puts all three
   inside 1.8–2.2 %. Only the reference letter (`A`) is asserted into the band.
8. **Panel A needs a `k = 600/72` rescale to be "drawn at cell size".** A panel
   re-emitted at its design point size has a `viewBox` in points
   (`figsize × 72`), so its canvas is `600/72` times smaller than the cell in
   pixels; drawn that way, `s = 8.33` and the (never-rescaled) text would land at
   ~1 pt on paper.
9. **matplotlib `<text>` carries `style="font-size: …px"`,** and the composer
   reads lengths out of `style` (for `stroke-width`), but `font-size` is never
   rescaled — so a matplotlib panel is only text-correct when `s ≈ 1`.
10. **`pcolormesh` needs `invert_yaxis()`** to match `imshow`'s `origin='upper'`
    row order (`CYP1B1` on top); otherwise the heatmap is vertically mirrored.

## Uncertainties / not verified

* Only the band-A heatmap was re-emitted as vector. Panels **B (RDKit)** and **C
  (ChimeraX)** have no vector source here: B *could* be vector
  (`rdkit.Chem.Draw.MolDraw2DSVG` emits pure `<path>` + `<rect>`), but
  `svg_grid_convert` fails its post-conversion self-check on that dialect
  (“output still contains 37 unsupported node(s)”), so it was left as a bitmap
  wrapper. B/C panels were not attempted as vectors.
* `Figure2.png` is RGBA (resvg output); the project's own figures are RGB. Not
  exercised: AVIF/SVGZ output, `--output-avif`, and a canvas narrower than
  4961 (I kept the existing product's width).
* The per-band difference numbers are for this exact canvas width only.
