#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""V1 render-layer confirmation.

Take the *composed* (post-`plot-grid`) SVG, find the first matplotlib mathtext
tick run (the log axis ``10^1`` label, whose glyphs live in absolute
``<tspan x y>`` coordinates), and check that its ink really lands where those
coordinates say — inside the canvas, at the expected place — in the PNG that
`svg_grid plot-grid --output-png` produced.

Criteria (all numeric, no eyeballing):
  * the run's ink is present: > 20 dark pixels in a window around the glyphs;
  * the measured ink bbox matches the bbox predicted from the tspan
    coordinates + font size (each edge within TOL px);
  * a control window of the same size shifted to a blank spot has *no* ink.

Usage:
    python check_render.py COMPOSED.svg COMPOSED.png
"""

import sys
import xml.etree.ElementTree as ET

import numpy as np
from PIL import Image

TOL = 3.0            # px
DARK = 100           # luminance threshold


def local(tag):
    return tag.split("}")[-1] if "}" in tag else tag


def first_text_run(svg_path):
    """(tspans, baseline) for the first <text> in document order."""
    root = ET.parse(svg_path).getroot()

    def find(el):
        if local(el.tag) == "text":
            return el
        for child in el:
            got = find(child)
            if got is not None:
                return got
        return None

    text = find(root)
    tspans = []
    for el in text.iter():
        if local(el.tag) == "tspan":
            fs = 10.0
            style = el.get("style", "")
            for token in style.split(";"):
                if token.strip().startswith("font-size"):
                    fs = float(token.split(":")[1].strip().replace("px", ""))
            tspans.append((float(el.get("x")), float(el.get("y")), fs))
    return tspans


def dark_bbox(mask):
    ys, xs = np.where(mask)
    if len(xs) == 0:
        return None
    return xs.min(), ys.min(), xs.max(), ys.max()


def main():
    svg_path, png_path = sys.argv[1], sys.argv[2]
    tspans = first_text_run(svg_path)
    # rough per-glyph ink: left inset ~0.10 em, cap height ~0.72 em, digits have
    # no descender.
    left = min(x + 0.10 * fs for x, _, fs in tspans)
    right = min(x + 0.60 * fs for x, _, fs in tspans)
    right = max(x + 0.60 * fs for x, _, fs in tspans)
    top = min(y - 0.72 * fs for _, y, fs in tspans)
    base = max(t[1] for t in tspans)
    expected = (left, top, right, base)
    print("tspans (x, baseline, font-size):", tspans)
    print(f"expected ink bbox: x=[{left:.2f},{right:.2f}] y=[{top:.2f},{base:.2f}]")

    img = np.asarray(Image.open(png_path).convert("L"))
    H, W = img.shape
    print(f"PNG {W}x{H}")

    # A tight window: the axis spine (x≈57) and the tick marks (y≤304) sit just
    # outside it, so only the glyphs are measured.
    x0 = int(left) - 2
    x1 = int(right) + 3
    y0 = int(top) - 2
    y1 = int(base) + 3
    window = img[y0:y1, x0:x1]
    mask = window < DARK
    count = int(mask.sum())
    bb = dark_bbox(mask)
    print(f"window x=[{x0},{x1}] y=[{y0},{y1}] dark pixels = {count}")
    assert count > 20, f"too few dark pixels ({count}); the run did not render"
    lx0, ly0, lx1, ly1 = bb[0] + x0, bb[1] + y0, bb[2] + x0, bb[3] + y0
    print(f"measured ink bbox: x=[{lx0},{lx1}] y=[{ly0},{ly1}]")
    edge = {
        "left": abs(lx0 - left),
        "right": abs(lx1 - right),
        "top": abs(ly0 - top),
        "bottom": abs(ly1 - base),
    }
    print("edge deltas (px):", {k: round(v, 2) for k, v in edge.items()})
    for name, d in edge.items():
        assert d <= TOL, f"{name} edge off by {d:.2f} px (> {TOL})"

    # control: same-sized window 40 px to the right (a blank spot between ticks)
    cx0, cx1 = x0 + 40, x1 + 40
    control = img[y0:y1, cx0:cx1]
    dark_control = int((control < DARK).sum())
    print(f"control window x=[{cx0},{cx1}] y=[{y0},{y1}] dark pixels = {dark_control}")
    assert dark_control == 0, f"control window is not blank ({dark_control} dark px)"

    print(f"OK: mathtext tick ink lands on the expected position "
          f"(<= {TOL} px on every edge), inside the {W}x{H} canvas")
    return 0


if __name__ == "__main__":
    sys.exit(main())
