#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_s22` 第 5 节：paper-rule 核验 + 与项目既有 Figure S22.png 的对比。

死白带那一条**不是本地副本**：它 `import check_deliverable` 后调
`cd.check_dead_band`（判据的唯一来源是 `tools/svg_grid/scripts/check_deliverable.py`）。
"""

import os
import sys

import numpy as np
from PIL import Image, ImageDraw

from s22_common import OUT, ORIGINAL, SERIF, SERIF_BOLD, SVG_GRID_ROOT, render_glyph

# The deliverable checks (dead white band / outer 2 px ink)
# live in ONE place now: tools/svg_grid/scripts/check_deliverable.py.  This test
# imports its functions instead of keeping a copy that can drift (see the guide's
# 05_checklist.md C section).
SCRIPTS = os.path.join(SVG_GRID_ROOT, "scripts")
if SCRIPTS not in sys.path:
    sys.path.insert(0, SCRIPTS)
import check_deliverable as cd                                  # noqa: E402


def max_run(flags):
    best = cur = 0
    for flag in flags:
        cur = cur + 1 if flag else 0
        best = max(best, cur)
    return best


def check_paper_rules(png_path, anchors, label_size, margin=24, label_band=0):
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

    # (2) dead white band — criterion owned by check_deliverable.check_dead_band,
    # no longer duplicated here (the old copy mis-fired on a full-width panel's
    # right page margin, and on letters sitting inside a row gap).  S22 stacks
    # three row blocks (each carrying its own letter band) with normal inter-row
    # gaps, so we deduct the side margin + the top letter band and allow gaps.
    dead = cd.check_dead_band(a, margin=margin, band=200, ink_threshold=245,
                              label_band=label_band, allow_gap=True)
    assert dead["status"] == "pass", (
        f"dead-band check failed ({dead['status']}): {dead.get('reason', '')}")

    white = a >= 250
    row_full = white.all(axis=1)
    col_full = white.all(axis=0)
    max_row = max_run(row_full)
    max_col = max_run(col_full)

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
