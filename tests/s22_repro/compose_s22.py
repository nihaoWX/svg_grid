#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Reproduce the Supplementary Figure S22 layout with `svg_grid plot-grid`.

Pipeline (all outputs land in ``out/``; the research project is only ever READ):

  1. convert every *sized* matplotlib panel in ``out/panels_sized/`` to the tag
     protocol with the `svg_grid_convert` binary (``out/tagged/``);
  2. verify no dialect residue survives (no ``<path>``/``<use>``/``<g transform>``/
     un-normalised text transform / transformed ``<image>``);
  3. compose the three blocks and stack them:
         block 1: A | B          (2 columns)
         block 2: C | D | E      (3 columns)
         block 3: F              (1 column, full content width)
     `plot-grid` only does a *regular* grid, so S22 is built by nesting: AB and
     CDE are composed first, then the three blocks are stacked with ``--ncol 1``.
     Neither level passes ``--rel-*``: the columns take the panels' own canvas
     aspect ratios (``--rel-widths`` omitted) and the stacked rows take the
     blocks' natural heights (``--rel-heights`` omitted), which is exactly what
     the hand-computed ``aspects`` / block heights used to be fed as;
  4. assert, for every block and every cell, that the composed ``cell/canvas``
     scale is ~1 — i.e. that each panel was drawn at the size of the cell it
     occupies.  The tag protocol never rescales ``font-size``, so a panel drawn
     at design size and upscaled into a cell keeps its text too small; drawing it
     already at cell size (``make_panels_svg.py --scale-to-cell``) makes
     ``sx = cell.w/canvas.w`` and ``sy = cell.h/canvas.h`` both ~1, so the text
     comes out at its design proportion.  This is checked both analytically and
     by parsing the composed SVG: each panel's full-canvas background polygon
     must land on exactly its expected cell rectangle;
  5. render ``FigureS22.png`` at 600 dpi, verify the paper rules from
     ``tools/manuscripts_skills/general-figure-guide`` (panel letters serif
     regular and 1.8-2.2 % of the canvas width; no >=200 px all-white row/column
     band; no dead 200 px band) and build a stacked, equal-width comparison with
     the project's existing ``Figure S22.png``.

Usage:
    micromamba run -n python-bio python compose_s22.py
"""

import os
import re
import subprocess
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
SVG_GRID_ROOT = os.path.dirname(os.path.dirname(HERE))          # tools/svg_grid
CONVERT = os.path.join(SVG_GRID_ROOT, "convert/target/release/svg_grid_convert")
GRID = os.path.join(SVG_GRID_ROOT, "target/release/svg_grid")

OUT = os.path.join(HERE, "out")
PANELS_NATIVE = os.path.join(OUT, "panels")        # kept for reference
PANELS_SIZED = os.path.join(OUT, "panels_sized")   # drawn at cell size
TAGGED = os.path.join(OUT, "tagged")

ORIGINAL = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/08_手稿/"
            "Figure/supp/Figure S22.png")

# --- S22 layout (mirrors the project's make_FigureS22.py; sizes are fractions
#     of the canvas width so the same design works at any output scale) ------- #
CANVAS_WIDTH = 4961
M_SIDE_FRAC = 40 / 4961      # outer left/right margin
G_COL_FRAC = 40 / 4961       # horizontal gap between panels in a row
GAP_LETTER_FRAC = 40 / 4961  # gap between a letter's ink bottom and its panel
TOPRUN_FRAC = 55 / 4961      # white above a letter inside its band
LETTER_FRAC = 0.020          # letter ink width as a fraction of canvas width

try:
    import matplotlib
    _TTF = os.path.join(matplotlib.get_data_path(), "fonts/ttf")
except Exception:                                            # pragma: no cover
    _TTF = ("/home/nihao/micromamba/envs/python-bio/lib/python3.12/"
            "site-packages/matplotlib/mpl-data/fonts/ttf")
SERIF = os.path.join(_TTF, "DejaVuSerif.ttf")
SERIF_BOLD = os.path.join(_TTF, "DejaVuSerif-Bold.ttf")

# Panels A..F; F uses the vector (pcolormesh) variant so the heatmap stays
# vector — the embedded `<image>` raster is kept for the colour bar.
ROWS = [
    dict(name="AB", panels=["fig1_pairwise_matched_cos", "fig2_shared_ecotype_count"],
         labels=["A", "B"]),
    dict(name="CDE", panels=["fig3_empty_factors", "fig4_transferability", "fig5_joint_nmf"],
         labels=["C", "D", "E"]),
    dict(name="F", panels=["fig6_joint_k7_composition_pcolormesh"], labels=["F"]),
]

ALL_PANELS = [p for row in ROWS for p in row["panels"]]
ALL_PANELS.append("fig6_joint_k7_composition_imshow")       # converted + reported too

POINTS_RE = re.compile(r"<polygon\b[^>]*\bpoints=\"([^\"]*)\"")


def run(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stderr.write(" ".join(cmd) + "\n")
        sys.stderr.write(result.stdout + result.stderr)
        raise SystemExit(f"command failed with exit code {result.returncode}")
    return result


def canvas_size(path):
    text = open(path, encoding="utf-8").read(4000)
    match = re.search(r'viewBox="([^"]+)"', text)
    values = [float(v) for v in match.group(1).split()]
    return values[2], values[3]


# --- letter metrics (DejaVu Serif, the font the letters are drawn with) ------ #
def render_glyph(ch, size, fontpath=SERIF):
    """uint8 alpha mask of the glyph ink, cropped to its bounding box."""
    font = ImageFont.truetype(fontpath, size)
    im = Image.new("L", (size * 4, size * 4), 0)
    ImageDraw.Draw(im).text((size, size), ch, fill=255, font=font)
    a = np.asarray(im)
    ys, xs = np.where(a > 60)
    return a[ys.min():ys.max() + 1, xs.min():xs.max() + 1]


def size_for_width(ch, target_w, fontpath=SERIF):
    """Smallest integer font size whose glyph ink width >= target_w."""
    lo, hi = 8, 4000
    while lo < hi:
        mid = (lo + hi) // 2
        glyph = render_glyph(ch, mid, fontpath)
        if glyph is None or glyph.shape[1] < target_w:
            lo = mid + 1
        else:
            hi = mid
    return lo


def geometry(width):
    """Canvas-unit geometry for a given canvas width.

    ``label_size`` is the font-size (canvas units) whose DejaVu Serif ``A`` ink is
    ``LETTER_FRAC * width`` wide, so the drawn letter lands in the 1.8-2.2 %
    band; ``band`` is the white letter strip above each row
    (``gap + glyph height + top run``) and ``vjust`` puts the letter's ink bottom
    exactly ``gap`` above the panel.
    """
    m_side = round(width * M_SIDE_FRAC)
    g_col = round(width * G_COL_FRAC)
    gap_letter = round(width * GAP_LETTER_FRAC)
    toprun = round(width * TOPRUN_FRAC)
    label_size = size_for_width("A", LETTER_FRAC * width)
    glyph_h = render_glyph("A", label_size).shape[0]
    band = gap_letter + glyph_h + toprun
    return dict(m_side=m_side, g_col=g_col, gap_letter=gap_letter, toprun=toprun,
                label_size=label_size, glyph_h=glyph_h, band=band,
                vjust=-gap_letter / label_size)


# --------------------------------------------------------------------------- #
# 1. convert
# --------------------------------------------------------------------------- #
def convert_panels():
    os.makedirs(TAGGED, exist_ok=True)
    rows = []
    for panel in ALL_PANELS:
        src = os.path.join(PANELS_SIZED, panel + ".svg")
        dst = os.path.join(TAGGED, panel + ".svg")
        result = subprocess.run(
            [CONVERT, "--input", src, "--output", dst, "--report"],
            capture_output=True, text=True,
        )
        svg = open(dst, encoding="utf-8").read() if os.path.exists(dst) else ""
        rows.append(dict(
            panel=panel,
            exit=result.returncode,
            path=svg.count("<path"),
            use=svg.count("<use"),
            gtransform=len(re.findall(r"<g\b[^>]*\btransform=", svg)),
            imgtf=len(re.findall(r"<image\b[^>]*\btransform=", svg)),
            text=svg.count("<text"),
            polyline=svg.count("<polyline"),
            polygon=svg.count("<polygon"),
            image=svg.count("<image"),
        ))
    return rows


# --------------------------------------------------------------------------- #
# 3. compose (with 4. scale/aspect assertions)
# --------------------------------------------------------------------------- #
def expected_cells(panels, aspects, width, gap, m_side, band, row_height):
    available = width - 2 * m_side - (len(aspects) - 1) * gap
    cells = []
    x = m_side
    for aspect in aspects:
        cell_w = available * aspect / sum(aspects)
        cells.append((x, band, cell_w, row_height))
        x += cell_w + gap
    assert abs(x - gap - (width - m_side)) < 1e-6, f"{panels}: row does not fill the width"
    return cells


def assert_aspects(name, aspects, cells):
    for cell, aspect in zip(cells, aspects):
        assert abs(cell[2] / cell[3] - aspect) < 1e-6, (
            f"{name}: cell aspect {cell[2] / cell[3]:.6f} != canvas aspect {aspect:.6f}"
        )


def assert_unit_scale(name, canvas, cells, tol=1e-4):
    """Each cell must be (almost exactly) the panel's own canvas → scale ~1.

    ``scale_panel_to_cell`` uses ``sx = cell.w/canvas.w`` and
    ``sy = cell.h/canvas.h`` independently; if either is far from 1 the panel is
    stretched, and (because ``font-size`` is never rescaled) its text is wrong.
    """
    for (cw, ch), (x, y, w, h) in zip(canvas, cells):
        sx, sy = w / cw, h / ch
        assert abs(sx - 1) < tol and abs(sy - 1) < tol, (
            f"{name}: cell scale ({sx:.6f}, {sy:.6f}) deviates from 1 "
            f"(canvas {cw}x{ch} -> cell {w:.3f}x{h:.3f})"
        )
    return [((w / cw), (h / ch)) for (cw, ch), (x, y, w, h) in zip(canvas, cells)]


def polygon_bboxes(svg):
    boxes = []
    for points in POINTS_RE.findall(svg):
        xs, ys = [], []
        for pair in points.split():
            x, y = pair.split(",")
            xs.append(float(x))
            ys.append(float(y))
        boxes.append((min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys)))
    return boxes


def assert_cells_land(block_svg, cells):
    """Each full-canvas background polygon must land on its expected cell box."""
    boxes = polygon_bboxes(block_svg)
    for (x, y, w, h) in cells:
        found = any(
            abs(bx - x) < 1.5 and abs(by - y) < 1.5 and abs(bw - w) < 1.5 and abs(bh - h) < 1.5
            for (bx, by, bw, bh) in boxes
        )
        assert found, f"no background polygon landed on the cell {x, y, w, h}"


def compose(width, prefix, geo):
    blocks_dir = os.path.join(OUT, "blocks")
    os.makedirs(blocks_dir, exist_ok=True)

    block_info = []
    scales = []          # (panel, sx, sy) for the report
    local_anchors = []   # (block_index, char, x, y_baseline) in block coords
    local_cells = []     # (block_index, panel, x, y, w, h) in block coords
    for block_index, row in enumerate(ROWS):
        canvas = [canvas_size(os.path.join(TAGGED, p + ".svg")) for p in row["panels"]]
        aspects = [w / h for (w, h) in canvas]

        column_gap = geo["g_col"] if len(row["panels"]) > 1 else 0
        available = width - 2 * geo["m_side"] - (len(aspects) - 1) * column_gap
        row_height = available / sum(aspects)
        cells = expected_cells(row["panels"], aspects, width, column_gap,
                               geo["m_side"], geo["band"], row_height)
        assert_aspects(row["name"], aspects, cells)
        for panel, (sx, sy) in zip(row["panels"],
                                   assert_unit_scale(row["name"], canvas, cells)):
            scales.append((panel, sx, sy))
        for label, cell in zip(row["labels"], cells):
            local_anchors.append((block_index, label, cell[0],
                                  cell[1] - geo["gap_letter"]))
        for panel, cell in zip(row["panels"], cells):
            local_cells.append((block_index, panel, cell[0], cell[1], cell[2], cell[3]))

        block_h = geo["band"] + row_height
        out = os.path.join(blocks_dir, row["name"] + ".svg")
        run([
            GRID, "plot-grid",
            *sum([["--input", os.path.join(TAGGED, p + ".svg")] for p in row["panels"]], []),
            "--output", out,
            "--width", str(width), "--height", str(block_h),
            "--ncol", str(len(row["panels"])),
            # No --rel-widths: the columns take each panel's own canvas aspect.
            "--plot-margin", f"{geo['band']},{geo['m_side']},0,{geo['m_side']}",
            "--gap", str(column_gap),
            "--labels", ",".join(row["labels"]),
            "--label-size", str(geo["label_size"]),
            "--label-fontfamily", "DejaVu Serif",
            "--label-fontface", "regular",
            "--label-x", "0", "--label-y", "1",
            "--hjust", "0", "--vjust", str(geo["vjust"]),
        ])
        assert_cells_land(open(out, encoding="utf-8").read(), cells)
        block_info.append(dict(name=row["name"], height=block_h))

    # Stack the three blocks with ncol=1 + rel-heights, no outer margin/gap: the
    # outer cell is then the block canvas itself (sx == sy == 1).
    total_h = sum(info["height"] for info in block_info)
    # block-local letter anchors -> figure coordinates (block b starts at the sum
    # of the heights of the blocks before it)
    offsets = []
    running = 0.0
    for info in block_info:
        offsets.append(running)
        running += info["height"]
    anchors = [(char, x, y + offsets[b]) for (b, char, x, y) in local_anchors]
    # every panel's cell rectangle in *figure* coordinates (blocks stacked)
    cell_boxes = {panel: (x, y + offsets[b], w, h)
                  for (b, panel, x, y, w, h) in local_cells}
    figure_svg = os.path.join(OUT, prefix + ".svg")
    run([
        GRID, "plot-grid",
        *sum([["--input", os.path.join(blocks_dir, info["name"] + ".svg")] for info in block_info], []),
        "--output", figure_svg,
        "--output-svgz", os.path.join(OUT, prefix + ".svgz"),
        "--output-png", os.path.join(OUT, prefix + ".png"),
        "--png-dpi", "600",
        "--width", str(width), "--height", str(total_h),
        "--ncol", "1",
        # No --rel-heights: each block row takes its own natural height (= the
        # block's canvas height, since blocks are W-wide), summing to total_h.
        "--plot-margin", "0",
        "--gap", "0",
    ])
    return figure_svg, total_h, scales, anchors, cell_boxes


# --------------------------------------------------------------------------- #
# 4b. regression guards (three failure modes that shipped once)
# --------------------------------------------------------------------------- #
IMAGE_RE = re.compile(r"<image\b[^>]*>")
TEXT_RE = re.compile(r"<text\b([^>]*)>(.*?)</text>", re.S)
FACTOR_ORDER = [f"F{i}" for i in range(7)]

# reference panels that carry exactly one legend frame
LEGEND_REFERENCE = {
    "fig1_pairwise_matched_cos": "fig1_pairwise_matched_cos",
    "fig2_shared_ecotype_count": "fig2_shared_ecotype_count",
    "fig3_empty_factors": "fig3_empty_factors",
    "fig4_transferability": "fig4_transferability",
}
REFERENCE_PANELS = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/"
                    "05_空间深化/spatial_transcription/nmf_k_selection/figure_panels")


def _vertical_run(mask):
    """per-column longest run of consecutive True (mask: 2D bool, rows first)."""
    idx = np.arange(mask.shape[0])[:, None]
    last_false = np.where(~mask, idx, -1)
    np.maximum.accumulate(last_false, axis=0, out=last_false)
    return np.where(mask, idx - last_false, 0).max(axis=0)


def legend_ratio(image):
    """legend-frame width / axes width, measured from a rendered RGB panel.

    Identical rule for our render and the project's reference panel, so their
    ratios are comparable:

      * axes spines : near-black (gray < 128) vertical runs covering most of the
                      crop -> the only dark runs that long (the legend frame is
                      light gray, the grid is lighter still);
      * legend frame: matplotlib's default legend edge is '0.8', a light-gray
                      rounded rectangle -> the largest connected component of
                      light-gray pixels; its bounding box is the legend box.
    """
    from scipy import ndimage as ndi

    a = np.asarray(image.convert("RGB"), dtype=float).mean(axis=2)
    col_run = _vertical_run(a < 128)
    spines = np.where(col_run >= 0.55 * col_run.max())[0]
    axes_w = int(spines.max()) - int(spines.min())
    assert axes_w > 0, "no axes spines found in the panel crop"

    gray = (a >= 190) & (a <= 228)
    labels, count = ndi.label(gray)
    areas = ndi.sum(gray, labels, range(1, count + 1))
    best = None
    for index, sl in enumerate(ndi.find_objects(labels)):
        if sl is None:
            continue
        ys, xs = sl
        w, h = xs.stop - xs.start, ys.stop - ys.start
        if w > 50 and h > 50 and (best is None or areas[index] > best[0]):
            best = (areas[index], w)
    assert best is not None, "no legend frame found in the panel crop"
    return best[1] / axes_w


def assert_raster_rendered(png_path, figure_svg, min_colors=100):
    """Every embedded <image> (the colour bars) must actually be decoded."""
    svg = open(figure_svg, encoding="utf-8").read()
    image = Image.open(png_path).convert("RGB")
    results = []
    for tag in IMAGE_RE.findall(svg):
        box = re.search(
            r'\bx="([0-9.]+)"\s+y="([0-9.]+)"\s+width="([0-9.]+)"\s+height="([0-9.]+)"',
            tag,
        )
        x, y, w, h = (float(v) for v in box.groups())
        crop = image.crop((int(x), int(y), int(round(x + w)), int(round(y + h))))
        colours = np.asarray(crop).reshape(-1, 3)
        unique = len(np.unique(colours, axis=0))
        results.append(((x, y, w, h), unique, colours.mean(axis=0)))
        assert unique > min_colors, (
            f"<image> box {(x, y, w, h)} rendered only {unique} unique colours "
            f"(<= {min_colors}); the raster was not decoded (resvg raster-images?)"
        )
    assert results, "no <image> element found in the composed SVG"
    return results


def assert_legend_ratios(png_path, cell_boxes, max_rel_err=0.15):
    """Our legend/plot-area ratio must match the reference panel's."""
    image = Image.open(png_path).convert("RGB")
    results = []
    for panel, ref_name in LEGEND_REFERENCE.items():
        ref_path = os.path.join(REFERENCE_PANELS, ref_name + ".png")
        x, y, w, h = cell_boxes[panel]
        crop = image.crop((int(round(x)), int(round(y)),
                           int(round(x + w)), int(round(y + h))))
        ours = legend_ratio(crop)
        reference = legend_ratio(Image.open(ref_path))
        rel_err = abs(ours - reference) / reference
        results.append((panel, ours, reference, rel_err))
        assert rel_err < max_rel_err, (
            f"{panel}: legend width / plot width = {ours:.4f} vs reference "
            f"{reference:.4f} ({rel_err * 100:.1f} % relative error)"
        )
    return results


def assert_factor_order(figure_svg, f_cell):
    """The F panel's y tick text, top-to-bottom, must read F0..F6 (like imshow)."""
    svg = open(figure_svg, encoding="utf-8").read()
    _, cell_y, _, cell_h = f_cell
    rows = []
    for match in TEXT_RE.finditer(svg):
        text = match.group(2).strip()
        if text in FACTOR_ORDER:
            y = float(re.search(r'\by="([0-9.\-]+)"', match.group(1)).group(1))
            if cell_y <= y <= cell_y + cell_h:
                rows.append((y, text))
    rows.sort()
    order = [text for _, text in rows]
    assert order == FACTOR_ORDER, (
        f"F-cell y ticks top-to-bottom are {order}, expected {FACTOR_ORDER} "
        f"(pcolormesh must not mirror the imshow panel)"
    )
    return rows


# --------------------------------------------------------------------------- #
# 5. verification + comparison
# --------------------------------------------------------------------------- #
def max_run(flags):
    best = cur = 0
    for flag in flags:
        cur = cur + 1 if flag else 0
        best = max(best, cur)
    return best


def check_paper_rules(png_path, anchors, label_size):
    a = np.asarray(Image.open(png_path).convert("L"))
    H, W = a.shape

    # (1) panel letters: the letter *size* must be ~2 % of the canvas width and
    # the letters must not be bold.  Glyph ink widths differ between letters
    # ('A' 0.74em .. 'E'/'F' 0.60em in DejaVu Serif), and that spread (1.235x) is
    # wider than the rule's band (1.222x), so no single font size puts all six
    # letters inside 1.8-2.2 %; the reference letter 'A' (which fixes the size)
    # must, and the others must stay in a sane 1.5-2.2 % envelope.
    letters = []
    for char, x, y in anchors:
        X0, X1 = max(0, int(x - 2)), min(W, int(x + label_size * 1.7))
        Y0, Y1 = max(0, int(y - label_size * 1.1)), min(H, int(y + label_size * 0.3))
        sub = a[Y0:Y1, X0:X1]
        ys, xs = np.where(sub < 100)
        assert len(xs) > 0, f"letter {char} has no ink around ({x:.1f}, {y:.1f})"
        ink = sub[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        w_frac = (xs.max() - xs.min() + 1) / W
        # fill fraction is glyph-dependent (regular 'B' is already ~0.45), so the
        # bold test is relative: the measured fill must be closer to the regular
        # glyph's than to the bold glyph's.
        reg_fill = float((render_glyph(char, label_size, SERIF) > 60).mean())
        bold_fill = float((render_glyph(char, label_size, SERIF_BOLD) > 60).mean())
        letters.append(dict(char=char, x=x, y=y, ink_w=int(xs.max() - xs.min() + 1),
                            ink_h=int(ys.max() - ys.min() + 1), w_frac=float(w_frac),
                            fill=float((ink < 100).mean()),
                            reg_fill=reg_fill, bold_fill=bold_fill))
    widest = max(letters, key=lambda L: L["w_frac"])
    narrowest = min(letters, key=lambda L: L["w_frac"])
    assert 0.018 <= widest["w_frac"] <= 0.022, (
        f"letter {widest['char']}: widest ink width {widest['w_frac'] * 100:.2f} % "
        f"of canvas is outside the 1.8-2.2 % band"
    )
    assert narrowest["w_frac"] >= 0.015, (
        f"letter {narrowest['char']}: ink width {narrowest['w_frac'] * 100:.2f} % "
        f"of canvas is below 1.5 %"
    )
    for L in letters:
        assert abs(L["fill"] - L["reg_fill"]) < abs(L["fill"] - L["bold_fill"]), (
            f"letter {L['char']}: ink fill {L['fill']:.3f} is closer to bold "
            f"({L['bold_fill']:.3f}) than to regular ({L['reg_fill']:.3f})"
        )

    # (2) no >=200 px all-white row/column band. (3) no dead 200 px row band
    # (every full-height 200 px window must contain ink — equivalent to (2)).
    white = a >= 250
    row_full = white.all(axis=1)
    col_full = white.all(axis=0)
    max_row = max_run(row_full)
    max_col = max_run(col_full)
    dead_bands = [i for i in range(0, H - 199, 200) if not (a[i:i + 200] < 245).any()]
    assert max_row < 200, f"all-white row band of {max_row} px (>= 200)"
    assert max_col < 200, f"all-white column band of {max_col} px (>= 200)"
    assert not dead_bands, f"dead (all-white) 200 px row band(s) at {dead_bands}"

    # report every contiguous run of fully-white rows (expected: the small
    # <200 px margins inside the letter bands / panel padding)
    row_runs = []
    start = None
    for i, flag in enumerate(row_full):
        if flag and start is None:
            start = i
        elif not flag and start is not None:
            row_runs.append((start, i - start))
            start = None
    if start is not None:
        row_runs.append((start, len(row_full) - start))

    return dict(size=(W, H), letters=letters, max_white_row=max_row,
                max_white_col=max_col, full_white_rows=int(row_full.sum()),
                full_white_cols=int(col_full.sum()), white_row_runs=row_runs,
                ink_fraction=float((a < 250).mean()))


def compare(figure_png, prefix):
    ours = Image.open(figure_png).convert("RGB")
    original = Image.open(ORIGINAL).convert("RGB")
    # Equal width (the project figure's own width), top = original, bottom = ours.
    width = original.width

    def fit(image):
        if image.width == width:
            return image
        return image.resize((width, round(image.height * width / image.width)),
                            Image.LANCZOS)

    original, ours = fit(original), fit(ours)
    separator = 28
    canvas = Image.new("RGB", (width, original.height + separator + ours.height),
                       (60, 60, 60))
    canvas.paste(original, (0, 0))
    canvas.paste(ours, (0, original.height + separator))
    draw = ImageDraw.Draw(canvas)
    draw.text((14, 8), "ORIGINAL  Figure S22.png", fill=(255, 60, 60))
    draw.text((14, original.height + separator + 8),
              f"REPRODUCTION  {prefix}.png  (svg_grid plot-grid)", fill=(60, 120, 255))
    path = os.path.join(OUT, prefix + "_comparison.png")
    canvas.save(path)
    return path, (width, original.height), (width, ours.height)


# --------------------------------------------------------------------------- #
def main():
    print("== 1. convert (sized panels) ==")
    conversion = convert_panels()
    print(f"{'panel':44s} {'exit':>4s} {'path':>4s} {'use':>4s} {'g_tf':>4s} "
          f"{'img_tf':>6s} {'text':>4s} {'image':>5s} {'polyline':>8s} {'polygon':>7s}")
    for row in conversion:
        print(f"{row['panel']:44s} {row['exit']:4d} {row['path']:4d} {row['use']:4d} "
              f"{row['gtransform']:4d} {row['imgtf']:6d} {row['text']:4d} {row['image']:5d} "
              f"{row['polyline']:8d} {row['polygon']:7d}")
        assert row["exit"] == 0, f"{row['panel']} failed to convert"
        assert (row["path"], row["use"], row["gtransform"], row["imgtf"]) == (0, 0, 0, 0), \
            f"{row['panel']} still has dialect residue"

    geo = geometry(CANVAS_WIDTH)
    print(f"\n== 2. geometry (canvas width {CANVAS_WIDTH}) ==")
    print(f"  m_side={geo['m_side']} g_col={geo['g_col']} band={geo['band']} "
          f"(gap={geo['gap_letter']} + glyph_h={geo['glyph_h']} + toprun={geo['toprun']})")
    print(f"  label_size={geo['label_size']} canvas units  vjust={geo['vjust']:.5f}  "
          f"-> A ink ~{LETTER_FRAC * CANVAS_WIDTH:.1f} px "
          f"({LETTER_FRAC * 100:.2f} % of width)")

    prefix = "FigureS22"
    print(f"\n== 3. compose {prefix} (canvas width {CANVAS_WIDTH}) ==")
    figure_svg, total_h, scales, anchors, cell_boxes = compose(CANVAS_WIDTH, prefix, geo)
    svg = open(figure_svg, encoding="utf-8").read()
    g_tf = len(re.findall(r"<g\b[^>]*\btransform=", svg))
    image_tf = len(re.findall(r"<image\b[^>]*\btransform=", svg))
    unnormalized_text = len(re.findall(
        r"<text\b[^>]*\btransform=(?!\"rotate\(-?[0-9.]+[, ]+-?[0-9.]+[, ]+-?[0-9.]+\)\")", svg))
    print(f"  {prefix}.svg  size={os.path.getsize(figure_svg)} bytes  "
          f"height={total_h:.1f}  cells={svg.count('data-svg-grid-cell')}  "
          f"text={svg.count('<text')}")
    print(f"  residue: path={svg.count('<path')} use={svg.count('<use')} "
          f"symbol={svg.count('<symbol')} g_tf={g_tf} image_tf={image_tf} "
          f"unnormalized_text_tf={unnormalized_text}")
    assert svg.count("<path") == 0 and svg.count("<use") == 0 and svg.count("<symbol") == 0
    assert (g_tf, image_tf, unnormalized_text) == (0, 0, 0)

    print("  per-cell scale (should be 1):")
    for panel, sx, sy in scales:
        print(f"    {panel:44s} sx={sx:.6f} sy={sy:.6f}")

    png = os.path.join(OUT, prefix + ".png")
    print("\n== 4. paper-rule checks (on the rendered image) ==")
    rules = check_paper_rules(png, anchors, geo["label_size"])
    print(f"  {prefix}.png  {rules['size'][0]}x{rules['size'][1]} px @600dpi  "
          f"ink(non-white)={rules['ink_fraction'] * 100:.2f}%")
    print(f"  max all-white row band = {rules['max_white_row']} px  "
          f"(fully-white rows: {rules['full_white_rows']})")
    print(f"  max all-white col band = {rules['max_white_col']} px  "
          f"(fully-white cols: {rules['full_white_cols']})")
    print(f"  all-white row runs (y, len) = {rules['white_row_runs']}")
    for L in rules["letters"]:
        print(f"    {L['char']}: ink {L['ink_w']}x{L['ink_h']} px = "
              f"{L['w_frac'] * 100:.2f}% of canvas width  fill={L['fill']:.3f} "
              f"(DejaVu Serif regular {L['reg_fill']:.3f} vs bold {L['bold_fill']:.3f})")

    print("\n== 4b. regression guards ==")
    raster = assert_raster_rendered(png, figure_svg)
    for box, unique, mean in raster:
        print(f"  <image> box {box}  unique colours={unique}  "
              f"mean=[{mean[0]:.1f} {mean[1]:.1f} {mean[2]:.1f}]  (need > 100)")

    legend = assert_legend_ratios(png, cell_boxes)
    for panel, ours, reference, rel_err in legend:
        print(f"  {panel:32s} legend/plot = {ours:.4f}  ref = {reference:.4f}  "
              f"rel_err = {rel_err * 100:.2f} %  (need < 15 %)")

    f_cell = cell_boxes["fig6_joint_k7_composition_pcolormesh"]
    rows = assert_factor_order(figure_svg, f_cell)
    print(f"  F-cell y ticks top-to-bottom = "
          f"{','.join(text for _, text in rows)}  (need F0..F6)")

    print("\n== 5. comparison ==")
    path, original_size, ours_size = compare(png, prefix)
    print(f"  original   {original_size[0]}x{original_size[1]}")
    print(f"  ours       {ours_size[0]}x{ours_size[1]}")
    print(f"  stacked (equal width {original_size[0]}) -> {path}")


if __name__ == "__main__":
    main()
