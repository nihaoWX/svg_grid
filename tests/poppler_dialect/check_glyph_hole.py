#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""V2 objective hole-survival check.

Given the **converted** glyph SVG and its rendered PNG (produced by
`svg_grid plot-grid --output-png`, i.e. resvg), walk the vertical centre line of
the glyph's bounding box and verify the fill pattern is *stroke / hole / stroke*:
the glyph's counter must be background, its ring must be the fill colour.

It also prints the exact RGB sampled at the hole centre and on the ring, so the
verdict is a pair of pixel values, not an eyeball.

Usage:
    python check_glyph_hole.py TAGGED.svg PNG R G B
where R G B is the expected fill colour (0-255).
"""

import re
import sys

import numpy as np
from PIL import Image

FILL_TOL = 40          # per-channel tolerance for "is the fill colour"
BG_MIN = 235           # a channel-mean above this is "background (white)"


def polygon_bbox(svg_path):
    text = open(svg_path, encoding="utf-8").read()
    best = None
    for points in re.findall(r'<polygon\b[^>]*\bpoints="([^"]*)"', text):
        xs, ys = [], []
        for pair in points.split():
            x, y = pair.split(",")
            xs.append(float(x))
            ys.append(float(y))
        box = (min(xs), min(ys), max(xs), max(ys))
        area = (box[2] - box[0]) * (box[3] - box[1])
        if best is None or area > best[0]:
            best = (area, box)
    assert best is not None, "no <polygon> in the converted glyph SVG"
    return best[1]


def classify(rgb, fill):
    arr = np.array(rgb, dtype=float)
    d = np.abs(arr - np.array(fill, dtype=float)).max()
    if d <= FILL_TOL:
        return "F"
    if arr.mean() >= BG_MIN:
        return "B"
    return "?"


def main():
    svg_path, png_path = sys.argv[1], sys.argv[2]
    fill = tuple(int(v) for v in sys.argv[3:6])
    x0, y0, x1, y1 = polygon_bbox(svg_path)
    print(f"glyph polygon bbox: x=[{x0:.2f},{x1:.2f}] y=[{y0:.2f},{y1:.2f}] "
          f"(centre column x={((x0 + x1) / 2):.2f})")

    img = np.asarray(Image.open(png_path).convert("RGB"))
    H, W = img.shape[:2]
    print(f"PNG {W}x{H}  expected fill rgb{fill}")

    cx = int(round((x0 + x1) / 2))
    # trim one pixel each end: the bbox edge itself is background.
    ya, yb = int(round(y0)) + 1, int(round(y1)) - 1
    column = [classify(img[y, cx], fill) for y in range(ya, yb + 1)]
    pattern = "".join(column)
    print(f"centre column y=[{ya},{yb}] classes: {pattern}")

    # collapse into runs
    runs = []
    for ch in pattern:
        if runs and runs[-1][0] == ch:
            runs[-1][1] += 1
        else:
            runs.append([ch, 1])
    print("runs:", [(c, n) for c, n in runs])

    # stroke / hole / stroke: there must be a background run (the counter) that
    # has fill *both above and below* it. Antialiased '?' pixels at the stroke
    # edges are ignored.
    assert any(c == "F" for c, _ in runs), f"no fill found: {pattern}"
    hole_index = None
    for i, (c, n) in enumerate(runs):
        if c == "B" and n >= 3:
            before = any(rc == "F" for rc, _ in runs[:i])
            after = any(rc == "F" for rc, _ in runs[i + 1:])
            if before and after:
                hole_index = i
                break
    assert hole_index is not None, f"NO HOLE — the counter was filled solid: {pattern}"

    hole_run = runs[hole_index]
    first_fill = next(n for c, n in runs if c == "F")
    # y of the middle of the hole run
    hole_y = ya + sum(n for _, n in runs[:hole_index]) + hole_run[1] // 2
    ring_y = ya + first_fill // 2
    print(f"hole  centre pixel ({cx},{hole_y}) RGB = {tuple(int(v) for v in img[hole_y, cx])}")
    print(f"ring  pixel       ({cx},{ring_y}) RGB = {tuple(int(v) for v in img[ring_y, cx])}")

    # hard RGB assertions: the hole must be (near) white, the ring must be the fill
    hole_rgb = tuple(int(v) for v in img[hole_y, cx])
    ring_rgb = tuple(int(v) for v in img[ring_y, cx])
    assert min(hole_rgb) >= 245, f"hole pixel {hole_rgb} is not background white"
    assert max(abs(a - b) for a, b in zip(ring_rgb, fill)) <= FILL_TOL, \
        f"ring pixel {ring_rgb} is not the fill rgb{fill}"
    print(f"OK: glyph hole survives ({hole_run[1]} px background), ring is filled")
    return 0


if __name__ == "__main__":
    sys.exit(main())
