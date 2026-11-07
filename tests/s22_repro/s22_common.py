#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""compose_s22 的共享配置与基础工具（面板规格 / 版式几何 / 子进程封装）。

这里搬的是原始 `compose_s22.py` 顶部的常量与"面板字母几何"那一节；入口
`compose_s22.py` 与其余辅助模块（`s22_convert` / `s22_compose` / `s22_guards` /
`s22_verify`）都从这里取，避免各自再复制一份常量。
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
