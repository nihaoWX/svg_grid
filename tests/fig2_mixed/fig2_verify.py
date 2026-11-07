#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_fig2` 的核验与对比：letter/纸上规则、内嵌栅格、与既有 Figure 2.png 对比。

死白带（dead-band）那一条**不是本地副本**：它 `import check_deliverable` 后调
`cd.check_dead_band`（判据的唯一来源是 `tools/svg_grid/scripts/check_deliverable.py`）。
"""

import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from fig2_common import (
    LETTER_H,
    M_B,
    M_L,
    M_R,
    M_T,
    ORIGINAL,
    OUT,
    ROOT,
    SERIF,
    SERIF_BOLD,
)
from fig2_compose import images

# The deliverable checks (dead white band / outer 2 px ink)
# live in ONE place now: tools/svg_grid/scripts/check_deliverable.py.  This test
# imports its functions instead of keeping a copy that can drift (see the guide's
# 05_checklist.md C section).
SCRIPTS = os.path.join(ROOT, "scripts")
if SCRIPTS not in sys.path:
    sys.path.insert(0, SCRIPTS)
import check_deliverable as cd                                  # noqa: E402


def render_glyph(ch, size, fontpath=SERIF):
    font = ImageFont.truetype(fontpath, size)
    image = Image.new("L", (size * 4, size * 4), 0)
    ImageDraw.Draw(image).text((size, size), ch, fill=255, font=font)
    a = np.asarray(image)
    ys, xs = np.where(a > 60)
    return a[ys.min():ys.max() + 1, xs.min():xs.max() + 1]


def max_run(flags):
    best = cur = 0
    for flag in flags:
        cur = cur + 1 if flag else 0
        best = max(best, cur)
    return best


def paper_rules(png_path, label_size, anchors):
    """``anchors``: (char, panel_top_y) — the letter sits just above the panel,
    so its ink lives in [panel_top - 1.3 * size, panel_top - 2]."""
    a = np.asarray(Image.open(png_path).convert("L"))
    height, width = a.shape
    letters = []
    for char, top in anchors:
        y0, y1 = max(0, int(top - 1.35 * label_size)), max(1, int(top - 2))
        x0, x1 = max(0, M_L - 6), min(width, int(M_L + 1.35 * label_size))
        sub = a[y0:y1, x0:x1]
        ys, xs = np.where(sub < 100)
        assert len(xs), f"letter {char} has no ink in {x0, y0, x1, y1}"
        ink = sub[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        letters.append(dict(
            char=char, ink_w=int(xs.max() - xs.min() + 1), ink_h=int(ys.max() - ys.min() + 1),
            w_frac=float((xs.max() - xs.min() + 1) / width),
            fill=float((ink < 100).mean()),
            reg_fill=float((render_glyph(char, label_size, SERIF) > 60).mean()),
            bold_fill=float((render_glyph(char, label_size, SERIF_BOLD) > 60).mean()),
            top=int(y0 + ys.min()), left=int(x0 + xs.min())))

    # Dead white band — criterion owned by check_deliverable.check_dead_band, no
    # longer duplicated here (the old copy mis-fired on a full-width panel's right
    # page margin, and on letters sitting inside a row gap).  Figure 2 stacks three
    # bands (each carrying its own letter band) with normal inter-row gaps, so we
    # deduct the figure's margin + the top letter band and allow interior gaps.
    dead_band = cd.check_dead_band(a, margin=(M_T, M_R, M_B, M_L), band=200,
                                   ink_threshold=245, label_band=LETTER_H,
                                   allow_gap=True)
    assert dead_band["status"] == "pass", (
        f"dead-band check failed ({dead_band['status']}): {dead_band.get('reason', '')}")

    white = a >= 250
    row_full, col_full = white.all(axis=1), white.all(axis=0)
    return dict(size=(width, height), letters=letters,
                max_white_row=max_run(row_full), max_white_col=max_run(col_full),
                dead_bands=[], ink_fraction=float((a < 250).mean()),
                full_white_rows=int(row_full.sum()), full_white_cols=int(col_full.sum()))


def assert_raster_rendered(png_path, svg_path):
    svg = open(svg_path, encoding="utf-8").read()
    image = Image.open(png_path).convert("RGB")
    boxes = images(svg)
    results = []
    for x, y, w, h in boxes:
        crop = image.crop((int(round(x)), int(round(y)),
                           int(round(x + w)), int(round(y + h))))
        unique = len(np.unique(np.asarray(crop).reshape(-1, 3), axis=0))
        results.append(((x, y, w, h), unique))
    return results


def compare(ours_png):
    ours = Image.open(ours_png).convert("RGB")
    original = Image.open(ORIGINAL).convert("RGB")
    width = original.width

    def fit(image):
        if image.width == width:
            return image
        return image.resize((width, round(image.height * width / image.width)), Image.LANCZOS)

    original, ours = fit(original), fit(ours)
    separator = 28
    canvas = Image.new("RGB", (width, original.height + separator + ours.height), (60, 60, 60))
    canvas.paste(original, (0, 0))
    canvas.paste(ours, (0, original.height + separator))
    draw = ImageDraw.Draw(canvas)
    draw.text((14, 8), "ORIGINAL  Figure 2.png  (make_Figure2_v6.py, all raster)",
              fill=(255, 60, 60))
    draw.text((14, original.height + separator + 8),
              "MIXED  Figure2.png  (svg_grid plot-grid: vector panel A + bitmap panels B/C)",
              fill=(60, 120, 255))
    path = os.path.join(OUT, "Figure2_comparison.png")
    canvas.save(path)
    return path, ours.size
